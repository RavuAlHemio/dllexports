//! Decoding CD-ROM file systems.


use std::io::{self, Read};

use bitflags::bitflags;
use from_to_repr::from_to_other;

use crate::io_util::{ByteBufReadable, ReadEndian};


/// The number of bytes per logical sector.
///
/// According to ISO9660 and High Sierra (both § 6.1.2), each logical sector contains 2**n bytes but
/// at least 2048 bytes. In practice, almost all CD-ROM file systems encode their data in logical
/// sectors 2048 bytes in size.
const BYTES_PER_LOGICAL_SECTOR: u64 = 2048;

/// The offset of the area containing the CD's data.
///
/// Logical sectors 0 to 15 contain the System Area in both ISO9660 (§ 6.2.1) and High Sierra
/// (§ 6.3) CDs. The Data Area therefore starts with logical sector 16.
const DATA_AREA_OFFSET: u64 = 16 * BYTES_PER_LOGICAL_SECTOR;

/// The byte offset within a logical sector of the High Sierra identifier.
///
/// See High Sierra § 11.1.
const HIGH_SIERRA_IDENTIFIER_OFFSET: u64 = 9;

/// The value of the High Sierra identifier.
///
/// See High Sierra § 11.1.3.
const HIGH_SIERRA_IDENTIFIER_VALUE: [u8; 5] = *b"CDROM";

/// The byte offset within a logical sector of the ISO9660 identifier.
///
/// See ISO9660 § 8.1.
const ISO9660_IDENTIFIER_OFFSET: u64 = 1;

/// The value of the ISO9660 identifier.
///
/// See ISO9660 § 8.1.2.
const ISO9660_IDENTIFIER_VALUE: [u8; 5] = *b"CD001";


/// String of all characters from allowed in the set of "d-characters".
///
/// The "d" in "d-characters" probably stems from "descriptor".
///
/// See ISO9660 § 7.4.1 and High Sierra § 10.4.1.
const D_CHARACTERS_SORTED: &str = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ_";


/// String of all characters from allowed in the set of "a-characters".
///
/// The "a" in "a-characters" probably stems from "ASCII".
///
/// See ISO9660 § 7.4.1 and High Sierra § 10.4.2.
const A_CHARACTERS_SORTED: &str = " !\"%&'()*+,-./0123456789:;<=>?ABCDEFGHIJKLMNOPQRSTUVWXYZ_";


/// The type of an ISO9660 descriptor.
///
/// High Sierra reserves the same values for mostly semantically equivalent descriptors but calls
/// them differently.
#[derive(Clone, Copy, Debug)]
#[from_to_other(base_type = u8, derive_compare = "as_int")]
pub enum DescriptorType {
    BootRecord = 0x00,
    PrimaryVolumeDescriptor = 0x01,
    SupplementaryOrEnhancedVolumeDescriptor = 0x02,
    VolumePartitionDescriptor = 0x03,
    SetTerminator = 0xFF,
    Other(u8),
}
impl Default for DescriptorType {
    fn default() -> Self { Self::BootRecord }
}


/// The same value, encoded first as little endian and then as big endian.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct EndianPair<T> {
    pub little_endian: T,
    pub big_endian: T,
}
impl<T: ReadEndian> ByteBufReadable for EndianPair<T> {
    fn read(buf: &[u8], pos: &mut usize) -> Self {
        let little_endian = <T as ReadEndian>::read_le(buf, pos);
        let big_endian = <T as ReadEndian>::read_le(buf, pos);
        Self {
            little_endian,
            big_endian,
        }
    }
}


/// An ISO9660 volume descriptor.
///
/// Volume descriptors come in three flavors:
///
/// | ISO9660 flavor | High Sierra flavor | `vd_type` | `version` | High Sierra | ISO9660 | A charset     | D charset     |
/// | -------------- | ------------------ | ---------:| ---------:| ----------- | ------- | ------------- | ------------- |
/// | Primary        | SFS                |      0x01 |      0x01 | § 11.4      | § 8.4   | a-characters  | d-characters  |
/// | Supplementary  | CCSFS              |      0x02 |      0x01 | § 11.5      | § 8.5   | a1-characters | d1-characters |
/// | Enhanced       | n/a                |      0x02 |      0x02 | n/a         | § 8.5   | by agreement  | by agreement  |
///
/// * SFS: Standard File Structure
/// * CCSFS: Coded Character Set File Structure
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct VolumeDescriptor {
    /// Volume descriptor logical block number.
    ///
    /// High Sierra only.
    pub vd_lbn: Option<EndianPair<u32>>, // 9660: (), HS: [u32; 2]

    /// Volume descriptor type.
    ///
    /// See the [`VolumeDescriptor`] documentation to see which volume descriptor types can be
    /// represented by it.
    pub vd_type: DescriptorType, // u8

    /// Volume descriptor standard identifier.
    ///
    /// Equals [`ISO9660_IDENTIFIER_VALUE`] on ISO9660 volumes and [`HIGH_SIERRA_IDENTIFIER_VALUE`]
    /// on High Sierra volumes.
    pub standard_identifier: [u8; 5],

    /// Volume descriptor version.
    ///
    /// Depends on the flavor (see [`VolumeDescriptor`] documentation).
    pub version: u8,

    /// Volume flags.
    ///
    /// Always zero on Primary/SFS volume descriptors.
    pub flags: VolumeFlags, // u8

    /// System identifier.
    ///
    /// May only contain characters from the A charset, which depends on the flavor (see
    /// [`VolumeDescriptor`] documentation).
    pub system_identifier: [u8; 32],

    /// Volume identifier.
    ///
    /// May only contain characters from the D charset, which depends on the flavor (see
    /// [`VolumeDescriptor`] documentation).
    pub volume_identifier: [u8; 32],

    /// Reserved field.
    pub reserved0: [u8; 8],

    /// Volume space size.
    pub volume_space_size: EndianPair<u32>,

    /// Escape sequences for the a1 and d1 character sets.
    ///
    /// Always zeroes on Primary/SFS volume descriptors.
    pub escape_sequences: [u8; 32],

    /// Number of volumes in this volume's volume set.
    pub volume_set_size: EndianPair<u16>, // [u16; 2]

    /// Sequence number of this volume in its volume set.
    pub volume_sequence_number: EndianPair<u16>, // [u16; 2]

    /// Size, in bytes, of a logical block.
    pub logical_block_size: EndianPair<u16>, // [u16; 2]

    /// Size of the path table.
    pub path_table_size: EndianPair<u32>, // [u32; 2]

    /// Location of the little-endian path table.
    ///
    /// This value is encoded in little-endian only.
    pub le_path_table_location: u32,

    /// Location of the backup copy of the little-endian path table.
    ///
    /// Zero indicates that no backup copy is stored.
    ///
    /// This value is encoded in little-endian only.
    pub le_path_table_backup_location: u32,

    /// Location of the second backup copy of the little-endian path table.
    ///
    /// Zero indicates that no second backup copy is stored.
    ///
    /// This field only exists on High Sierra volumes and value is encoded in little-endian only.
    pub le_path_table_backup_location_2: Option<u32>, // 9660: (), HS: u32

    /// Location of the third backup copy of the little-endian path table.
    ///
    /// Zero indicates that no third backup copy is stored.
    ///
    /// This field only exists on High Sierra volumes and value is encoded in little-endian only.
    pub le_path_table_backup_location_3: Option<u32>, // 9660: (), HS: u32

    /// Location of the big-endian path table.
    ///
    /// This value is encoded in big-endian only.
    pub be_path_table_location: u32,

    /// Location of the backup copy of the big-endian path table.
    ///
    /// Zero indicates that no backup copy is stored.
    ///
    /// This value is encoded in big-endian only.
    pub be_path_table_backup_location: u32,

    /// Location of the second backup copy of the big-endian path table.
    ///
    /// Zero indicates that no second backup copy is stored.
    ///
    /// This field only exists on High Sierra volumes and value is encoded in big-endian only.
    pub be_path_table_backup_location_2: Option<u32>, // 9660: (), HS: u32

    /// Location of the third backup copy of the big-endian path table.
    ///
    /// Zero indicates that no third backup copy is stored.
    ///
    /// This field only exists on High Sierra volumes and value is encoded in big-endian only.
    pub be_path_table_backup_location_3: Option<u32>, // 9660: (), HS: u32

    /// Directory record for the root directory.
    pub root_directory_record: DirectoryRecord, // [u8; 34]

    /// Volume set identifier.
    ///
    /// May only contain characters from the D charset, which depends on the flavor (see
    /// [`VolumeDescriptor`] documentation).
    pub volume_set_identifier: [u8; 128],

    /// Publisher identifier.
    ///
    /// May only contain characters from the A charset, which depends on the flavor (see
    /// [`VolumeDescriptor`] documentation).
    pub publisher_identifier: [u8; 128],

    /// Data preparer identifier.
    ///
    /// May only contain characters from the A charset, which depends on the flavor (see
    /// [`VolumeDescriptor`] documentation).
    pub data_preparer_identifier: [u8; 128],

    /// Application identifier.
    ///
    /// May only contain characters from the A charset, which depends on the flavor (see
    /// [`VolumeDescriptor`] documentation).
    pub application_identifier: [u8; 128],

    /// Copyright file identifier.
    ///
    /// May only contain:
    ///
    /// * characters from the D charset, which depends on the flavor (see [`VolumeDescriptor`]
    ///   documentation)
    /// * SEPARATOR 1 (`.`, U+002E)
    /// * on ISO9660 but not High Sierra: SEPARATOR 2 (`;`, U+003B)
    ///
    /// and must abide by the encoding of file identifiers (ISO9660 § 7.5, High Sierra § 10.5).
    ///
    /// 37 bytes long on ISO9660, 32 bytes long (right-padded with 0x00 on read) on High Sierra.
    pub copyright_file_identifier: [u8; 37], // 9660: [u8; 37], HS: [u8; 32]

    /// Abstract file identifier.
    ///
    /// May only contain:
    ///
    /// * characters from the D charset, which depends on the flavor (see [`VolumeDescriptor`]
    ///   documentation)
    /// * SEPARATOR 1 (`.`, U+002E)
    /// * on ISO9660 but not High Sierra: SEPARATOR 2 (`;`, U+003B)
    ///
    /// and must abide by the encoding of file identifiers (ISO9660 § 7.5, High Sierra § 10.5).
    ///
    /// 37 bytes long on ISO9660, 32 bytes long (right-padded with 0x00 on read) on High Sierra.
    pub abstract_file_identifier: [u8; 37], // 9660: [u8; 37], HS: [u8; 32]

    /// Bibliographic file identifier.
    ///
    /// May only contain:
    ///
    /// * characters from the D charset, which depends on the flavor (see [`VolumeDescriptor`]
    ///   documentation)
    /// * SEPARATOR 1 (`.`, U+002E)
    /// * SEPARATOR 2 (`;`, U+003B)
    ///
    /// and must abide by the encoding of file identifiers (ISO9660 § 7.5).
    ///
    /// This field only exists on ISO9660 volumes.
    pub bibliographic_file_identifier: Option<[u8; 37]>, // 9660: [u8; 37], HS: ()

    /// Volume creation date and time.
    pub volume_creation_timestamp: DigitTimestamp, // 9660: [u8; 17], HS: [u8; 16]

    /// Volume modification date and time.
    pub volume_modification_timestamp: DigitTimestamp, // 9660: [u8; 17], HS: [u8; 16]

    /// Volume expiration date and time.
    pub volume_expiration_timestamp: DigitTimestamp, // 9660: [u8; 17], HS: [u8; 16]

    /// Volume effective date and time.
    pub volume_effective_timestamp: DigitTimestamp, // 9660: [u8; 17], HS: [u8; 16]

    /// Version of the file structure.
    ///
    /// Generally equals the value of `version`.
    pub file_structure_version: u8,

    /// Reserved field.
    pub reserved1: u8,

    /// Reserved for application use.
    pub app_use: [u8; 512],

    /// Reserved field.
    ///
    /// 680 bytes on High Sierra, 653 bytes on ISO9660 volumes (right-padded with 0x00 on read).
    pub reserved2: [u8; 680], // 9660: [u8; 653], HS: [u8; 680]
}
impl VolumeDescriptor {
    pub fn read<R: Read>(reader: &mut R, is_high_sierra: bool) -> Result<Self, io::Error> {
        let mut buf = [0u8; 2048];
        reader.read_exact(&mut buf)?;
        let mut pos = 0;

        let vd_lbn = if is_high_sierra {
            Some(EndianPair::read(&buf, &mut pos))
        } else {
            None
        };
        let vd_type = DescriptorType::from_base_type(u8::read(&buf, &mut pos));
        let standard_identifier = ByteBufReadable::read(&buf, &mut pos);
        let version = u8::read(&buf, &mut pos);
        let flags = VolumeFlags::from_bits_retain(u8::read(&buf, &mut pos));
        let system_identifier = ByteBufReadable::read(&buf, &mut pos);
        let volume_identifier = ByteBufReadable::read(&buf, &mut pos);
        let reserved0 = ByteBufReadable::read(&buf, &mut pos);
        let volume_space_size = EndianPair::read(&buf, &mut pos);
        let escape_sequences = ByteBufReadable::read(&buf, &mut pos);
        let volume_set_size = EndianPair::read(&buf, &mut pos);
        let volume_sequence_number = EndianPair::read(&buf, &mut pos);
        let logical_block_size = EndianPair::read(&buf, &mut pos);
        let path_table_size = EndianPair::read(&buf, &mut pos);
        let le_path_table_location = u32::read_le(&buf, &mut pos);
        let le_path_table_backup_location = u32::read_le(&buf, &mut pos);
        let le_path_table_backup_location_2 = if is_high_sierra {
            Some(u32::read_le(&buf, &mut pos))
        } else {
            None
        };
        let le_path_table_backup_location_3 = if is_high_sierra {
            Some(u32::read_le(&buf, &mut pos))
        } else {
            None
        };
        let be_path_table_location = u32::read_be(&buf, &mut pos);
        let be_path_table_backup_location = u32::read_be(&buf, &mut pos);
        let be_path_table_backup_location_2 = if is_high_sierra {
            Some(u32::read_be(&buf, &mut pos))
        } else {
            None
        };
        let be_path_table_backup_location_3 = if is_high_sierra {
            Some(u32::read_be(&buf, &mut pos))
        } else {
            None
        };
        let root_directory_record = DirectoryRecord::read_from_volume_descriptor(&buf, &mut pos, is_high_sierra)?;
        let volume_set_identifier = ByteBufReadable::read(&buf, &mut pos);
        let publisher_identifier = ByteBufReadable::read(&buf, &mut pos);
        let data_preparer_identifier = ByteBufReadable::read(&buf, &mut pos);
        let application_identifier = ByteBufReadable::read(&buf, &mut pos);
        let copyright_file_identifier = if is_high_sierra {
            let hs_ident: [u8; 32] = ByteBufReadable::read(&buf, &mut pos);
            let mut padded_ident = [0u8; 37];
            padded_ident[..32].copy_from_slice(&hs_ident);
            padded_ident
        } else {
            // full 37 bytes
            ByteBufReadable::read(&buf, &mut pos)
        };
        let abstract_file_identifier = if is_high_sierra {
            let hs_ident: [u8; 32] = ByteBufReadable::read(&buf, &mut pos);
            let mut padded_ident = [0u8; 37];
            padded_ident[..32].copy_from_slice(&hs_ident);
            padded_ident
        } else {
            // full 37 bytes
            ByteBufReadable::read(&buf, &mut pos)
        };
        let bibliographic_file_identifier = if is_high_sierra {
            None
        } else {
            Some(ByteBufReadable::read(&buf, &mut pos))
        };
        let volume_creation_timestamp = DigitTimestamp::read(&buf, &mut pos, is_high_sierra);
        let volume_modification_timestamp = DigitTimestamp::read(&buf, &mut pos, is_high_sierra);
        let volume_expiration_timestamp = DigitTimestamp::read(&buf, &mut pos, is_high_sierra);
        let volume_effective_timestamp = DigitTimestamp::read(&buf, &mut pos, is_high_sierra);
        let file_structure_version = ByteBufReadable::read(&buf, &mut pos);
        let reserved1 = ByteBufReadable::read(&buf, &mut pos);
        let app_use = ByteBufReadable::read(&buf, &mut pos);
        let reserved2 = if is_high_sierra {
            ByteBufReadable::read(&buf, &mut pos)
        } else {
            let iso_ident: [u8; 653] = ByteBufReadable::read(&buf, &mut pos);
            let mut padded_ident = [0u8; 680];
            padded_ident[..653].copy_from_slice(&iso_ident);
            padded_ident
        };
        Ok(Self {
            vd_lbn,
            vd_type,
            standard_identifier,
            version,
            flags,
            system_identifier,
            volume_identifier,
            reserved0,
            volume_space_size,
            escape_sequences,
            volume_set_size,
            volume_sequence_number,
            logical_block_size,
            path_table_size,
            le_path_table_location,
            le_path_table_backup_location,
            le_path_table_backup_location_2,
            le_path_table_backup_location_3,
            be_path_table_location,
            be_path_table_backup_location,
            be_path_table_backup_location_2,
            be_path_table_backup_location_3,
            root_directory_record,
            volume_set_identifier,
            publisher_identifier,
            data_preparer_identifier,
            application_identifier,
            copyright_file_identifier,
            abstract_file_identifier,
            bibliographic_file_identifier,
            volume_creation_timestamp,
            volume_modification_timestamp,
            volume_expiration_timestamp,
            volume_effective_timestamp,
            file_structure_version,
            reserved1,
            app_use,
            reserved2,
        })
    }
}


bitflags! {
    #[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
    pub struct VolumeFlags : u8 {
        /// If this bit is set, the `escape_sequences` field contains at least one escape sequence
        /// that is not registered according to ISO2375.
        const CONTAINS_NON_ISO_2375_ESCAPE_SEQUENCE = 0x01;
    }
}


/// A partially-textual representation of a timestamp.
///
/// Apart from a valid date and time, a special zero value may be encoded by setting all digits to
/// b'0' (and, on ISO9660 volumes, the GMT offset to 0). This is the only situation where `year`,
/// `month` or `day` may contain zero values, as years, days and months are customarily numbered
/// starting with 1.
///
/// 17 bytes on ISO9660 (§ 8.4.26.1), 16 bytes on High Sierra (§ 11.4.30.1).
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DigitTimestamp {
    /// The year, in ASCII digits from b"0001" to b"9999", or b"0000" if encoding the zero value.
    pub year: [u8; 4],

    /// The month, in ASCII digits from b"01" to b"12", or b"00" if encoding the zero value.
    pub month: [u8; 2],

    /// The day, in ASCII digits from b"01" to b"31", or b"00" if encoding the zero value.
    pub day: [u8; 2],

    /// The hour, in ASCII digits from b"00" to b"23".
    pub hour: [u8; 2],

    /// The minute, in ASCII digits from b"00" to b"59".
    pub minute: [u8; 2],

    /// The second, in ASCII digits from b"00" to b"59".
    pub second: [u8; 2],

    /// Hundredths of a second, in ASCII digits from b"00" to b"99".
    pub centisecond: [u8; 2],

    /// Offset from GMT in units of 15min, from -48 to 52.
    ///
    /// ISO9660 volumes only.
    pub gmt_offset_15min: Option<i8>,
}
impl DigitTimestamp {
    pub fn read(buf: &[u8], pos: &mut usize, is_high_sierra: bool) -> Self {
        let year = ByteBufReadable::read(buf, pos);
        let month = ByteBufReadable::read(buf, pos);
        let day = ByteBufReadable::read(buf, pos);
        let hour = ByteBufReadable::read(buf, pos);
        let minute = ByteBufReadable::read(buf, pos);
        let second = ByteBufReadable::read(buf, pos);
        let centisecond = ByteBufReadable::read(buf, pos);
        let gmt_offset_15min = if is_high_sierra {
            None
        } else {
            Some(ByteBufReadable::read(buf, pos))
        };
        Self {
            year,
            month,
            day,
            hour,
            minute,
            second,
            centisecond,
            gmt_offset_15min,
        }
    }
}

/// An ISO9660 volume partition descriptor.
///
/// See ISO9660 § 8.6.
///
/// Can also house a High Sierra unspecified structure volume descriptor (§ 11.6).
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PartitionDescriptor {
    /// Volume descriptor logical block number.
    ///
    /// High Sierra only.
    pub vd_lbn: Option<EndianPair<u32>>, // 9660: (), HS: [u32; 2]

    /// Volume descriptor type.
    ///
    /// For partition descriptors, this value is always 0x03.
    pub vd_type: DescriptorType, // u8

    /// Volume descriptor standard identifier.
    ///
    /// Equals [`ISO9660_IDENTIFIER_VALUE`] on ISO9660 volumes and [`HIGH_SIERRA_IDENTIFIER_VALUE`]
    /// on High Sierra volumes.
    pub standard_identifier: [u8; 5],

    /// Volume descriptor version.
    ///
    /// For High Sierra and all hitherto published ISO9660 revisions, this value is always 0x01.
    pub version: u8,

    /// Reserved field.
    pub reserved0: u8,

    /// System identifier.
    ///
    /// May only contain a-characters.
    pub system_identifier: [u8; 32],

    /// Volume partition identifier.
    ///
    /// May only contain d-characters.
    pub partition_identifier: [u8; 32],

    /// Location of this volume partition.
    pub partition_location: EndianPair<u32>,

    /// Size of this volume partition.
    pub partition_size: EndianPair<u32>,

    /// Reserved field.
    ///
    /// 1960 bytes on ISO9660, 1952 bytes on High Sierra volumes (right-padded on read with 0x00).
    pub reserved1: [u8; 1960], // 9660: [u8; 1960], HS: [u8; 1952]
}
impl PartitionDescriptor {
    pub fn read<R: Read>(reader: &mut R, is_high_sierra: bool) -> Result<Self, io::Error> {
        let mut buf = [0u8; 2048];
        reader.read_exact(&mut buf)?;
        let mut pos = 0;

        let vd_lbn = if is_high_sierra {
            Some(EndianPair::read(&buf, &mut pos))
        } else {
            None
        };
        let vd_type = DescriptorType::from_base_type(u8::read(&buf, &mut pos));
        let standard_identifier = ByteBufReadable::read(&buf, &mut pos);
        let version = u8::read(&buf, &mut pos);
        let reserved0 = ByteBufReadable::read(&buf, &mut pos);
        let system_identifier = ByteBufReadable::read(&buf, &mut pos);
        let partition_identifier = ByteBufReadable::read(&buf, &mut pos);
        let partition_location = ByteBufReadable::read(&buf, &mut pos);
        let partition_size = ByteBufReadable::read(&buf, &mut pos);
        let reserved1 = if is_high_sierra {
            let hs_ident: [u8; 1952] = ByteBufReadable::read(&buf, &mut pos);
            let mut padded_ident = [0u8; 1960];
            padded_ident[..1952].copy_from_slice(&hs_ident);
            padded_ident
        } else {
            ByteBufReadable::read(&buf, &mut pos)
        };
        Ok(Self {
            vd_lbn,
            vd_type,
            standard_identifier,
            version,
            reserved0,
            system_identifier,
            partition_identifier,
            partition_location,
            partition_size,
            reserved1,
        })
    }
}

/// An ISO9660 directory record.
///
/// See ISO9660 § 9.1.
///
/// Can also house a High Sierra directory record (§ 13.1).
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DirectoryRecord {
    /// The length of the directory record.
    pub length: u8,

    /// The length of the extended attribute record.
    ///
    /// This is the number of logical blocks preceding the file data that contain the extended
    /// attribute record.
    pub extended_attribute_record_length: u8,

    /// The location of this extent as a logical block number.
    pub extent_location: EndianPair<u32>, // [u32; 2]

    /// The number of bytes contained in this extent.
    ///
    /// This only reflects the length of the data itself, not the length of the extended attribute
    /// record.
    pub data_length: EndianPair<u32>, // [u32; 2]

    /// The date and time at which this file was recorded.
    pub recording_timestamp: BinaryTimestamp, // 9660: [u8; 7], HS: [u8; 6]

    /// Various flags describing the kind of file.
    pub file_flags: FileFlags, // u8

    /// Reserved value.
    ///
    /// High Sierra only. (Removed in ISO9660 because the GMT offset was added to BinaryTimestamp.)
    pub reserved0: Option<u8>, // 9660: (), HS: u8

    /// The unit size if the file is recorded in interleaved mode.
    ///
    /// Zero if the file is recorded contiguously.
    pub interleave_unit_size: u8,

    /// The gap size if the file is recorded in interleaved mode.
    ///
    /// Zero if the file is recorded contiguously.
    pub interleave_gap_size: u8,

    /// Specifies which volume in the volume set contains this file.
    pub volume_sequence_number: EndianPair<u16>,

    // file_identifier_length: u8,

    /// The file identifier.
    ///
    /// If this entry is a directory ([`FileFlags::DIRECTORY`] is set in `file_flags`), this may
    /// only contain:
    /// * d-characters
    /// * on ISO9660 but not High Sierra: d1-characters
    /// * a single byte 0x00 (for a descriptor describing the directory itself)
    /// * a single byte 0x01 (for a descriptor describing the parent directory)
    ///
    /// The root directory is considered its own parent, so both the directory record 0x00 and the
    /// directory record 0x01 in the root directory describe the root directory. The semantics of
    /// 0x00 and 0x01 are found in ISO9660 § 6.8.2.2.
    ///
    /// If this entry is not a directory ([`FileFlags::DIRECTORY`] is not set in `file_flags`), this
    /// may only contain:
    /// * d-characters
    /// * on ISO9660 but not High Sierra: d1-characters
    /// * SEPARATOR 1 (`.`, U+002E)
    /// * SEPARATOR 2 (`;`, U+003B)
    pub file_identifier: Vec<u8>, // [u8; file_identifier_length]

    /// A reserved field to re-align the next one.
    ///
    /// Only present if the length of the file identifier is an even number, since the file
    /// identifier including the length byte then comprise an odd number of bytes.
    pub reserved1: Option<u8>,

    /// Bytes reserved for system use.
    pub system_use_bytes: Vec<u8>, // [u8; length - $directory_record_bytes_read]
}

/// A binary representation of a timestamp.
///
/// Apart from a valid date and time, a special zero value may be encoded by setting all fields to
/// 0. This is the only situation where `month` or `day` may contain zero values, as days and months
/// are customarily numbered starting with 1.
///
/// 7 bytes on ISO9660 (§ 9.1.5), 6 bytes on High Sierra (§ 13.1.5).
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct BinaryTimestamp {
    /// The year since the year 1900.
    ///
    /// For example, a value of 90 designates the year 1990.
    pub year_since_1900: u8,

    /// The month, a value from 1 to 12, or 0 if encoding the zero value.
    pub month: u8,

    /// The day, a value from 1 to 31, or 0 if encoding the zero value.
    pub day: u8,

    /// The hour, a value from 0 to 23.
    pub hour: u8,

    /// The minute, a value from 0 to 59.
    pub minute: u8,

    /// The second, a value from 0 to 59.
    pub second: u8,

    /// Offset from GMT in units of 15min, from -48 to 52.
    ///
    /// ISO9660 volumes only.
    pub gmt_offset_15min: Option<i8>,
}
impl BinaryTimestamp {
    pub fn read(buf: &[u8], pos: &mut usize, is_high_sierra: bool) -> Self {
        let year_since_1900 = ByteBufReadable::read(buf, pos);
        let month = ByteBufReadable::read(buf, pos);
        let day = ByteBufReadable::read(buf, pos);
        let hour = ByteBufReadable::read(buf, pos);
        let minute = ByteBufReadable::read(buf, pos);
        let second = ByteBufReadable::read(buf, pos);
        let gmt_offset_15min = if is_high_sierra {
            None
        } else {
            Some(ByteBufReadable::read(buf, pos))
        };
        Self {
            year_since_1900,
            month,
            day,
            hour,
            minute,
            second,
            gmt_offset_15min,
        }
    }
}

bitflags! {
    #[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
    pub struct FileFlags : u8 {
        /// Whether the file should be listed when requested by the user.
        const EXISTENCE = (1 << 0);

        /// Whether the entry is a directory, not a file.
        const DIRECTORY = (1 << 1);

        /// Whether this is an associated file, which contains additional metadata pertaining to the
        /// actual file with the same `file_identifier`.
        ///
        /// Neither ISO9660 nor High Sierra define the format of the contents of an Associated File.
        const ASSOCIATED_FILE = (1 << 2);

        /// Whether the structure of the file data is reflected by the Record Format field in the
        /// file's Extended Attribute Record.
        const RECORD = (1 << 3);

        /// Whether access control information should be considered valid when deciding whether a
        /// user may access the file.
        ///
        /// The access control information in question are the Owner Identification, Group
        /// Identification and Permissions fields in the Extended Attribute Record.
        const PROTECTION = (1 << 4);

        /// If this bit is set, more directory entries follow that describe further extents of the
        /// file.
        const MULTI_EXTENT = (1 << 7);
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ExtendedAttributeRecord {
    /// The ID of the owning user of the file.
    pub owner_identification: EndianPair<u16>, // [u16; 2]

    /// The ID of the owning group of the file.
    pub group_identification: EndianPair<u16>, // [u16; 2]

    /// Permissions pertaining to the file.
    ///
    /// Encoded in big-endian only.
    pub permissions: Permissions, // u16

    /// Timestamp at which the file has been created.
    pub file_creation_timestamp: DigitTimestamp, // 9660: [u8; 17], HS: [u8; 16]

    /// Timestamp at which the file has been modified.
    pub file_modification_timestamp: DigitTimestamp, // 9660: [u8; 17], HS: [u8; 16]

    /// Timestamp at which the file will expire.
    pub file_expiration_timestamp: DigitTimestamp, // 9660: [u8; 17], HS: [u8; 16]

    /// Timestamp at which the file becomes effective.
    pub file_effective_timestamp: DigitTimestamp, // 9660: [u8; 17], HS: [u8; 16]

    /// The format of each record in the file.
    pub record_format: u8,

    /// Additional attributes pertaining to the record format.
    pub record_attributes: u8,

    /// The length (or maximum length for variable-lenght records) of each record in the file.
    pub record_length: EndianPair<u16>, // [u16; 2]

    /// The identifier of a system which can use relevant system-use attributes.
    ///
    /// Contains a-characters or a1-characters only.
    pub system_identifier: [u8; 32],

    /// System-specific data.
    pub system_use: [u8; 64],

    /// The version of the extended attribute record.
    ///
    /// Both High Sierra and all hitherto published versions of ISO9660 prescribe the value 1 here,
    /// even though the formats between High Sierra and ISO9660 diverge.
    pub version: u8,

    // pub escape_sequences_length: Option<u8>, // 9660: u8, HS: ()

    /// Reserved for further standardization.
    ///
    /// 64 bytes (right-padded with 0x00 on read) on ISO9660 volumes, 65 bytes on High Sierra
    /// volumes.
    pub reserved0: [u8; 65], // 9660: [u8; 64], HS: [u8; 65]

    /// The number of the path table entry for the file's parent directory.
    ///
    /// High Sierra only.
    pub parent_directory_number: Option<EndianPair<u16>>,  // 9660: (), HS: [u6; 2]

    // pub application_use_length: EndianPair<u16>, // [u16; 2]

    /// A copy of the directory record for this file.
    ///
    /// High Sierra only.
    pub directory_record: Option<DirectoryRecord>, // 9660: (), HS: see first byte

    /// Data for internal application use.
    pub application_use_data: Vec<u8>, // [u8; application_use_length]

    /// Escape sequences used for decoding the file contents.
    ///
    /// ISO9660 only.
    pub escape_sequences: Option<Vec<u8>>, // 9660: [u8; escape_sequences_length], HS: ()
}

bitflags! {
    #[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
    pub struct Permissions : u16 {
        /// Whether the file should not be readable by the System class of users.
        const FORBID_SYSTEM_READ = (1 << 0);

        /// Reserved; should always be set.
        const RESERVED1 = (1 << 1);

        /// Whether the file should not be executable by the System class of users.
        const FORBID_SYSTEM_EXECUTE = (1 << 2);

        /// Reserved; should always be set.
        const RESERVED3 = (1 << 3);

        /// Whether the file should not be readable by the owner.
        const FORBID_OWNER_READ = (1 << 4);

        /// Reserved; should always be set.
        const RESERVED5 = (1 << 5);

        /// Whether the file should not be executable by the owner.
        const FORBID_OWNER_EXECUTE = (1 << 6);

        /// Reserved; should always be set.
        const RESERVED7 = (1 << 7);

        /// Whether the file should not be readable by members of the group.
        ///
        /// If the user in question is both the owner and a member of the group, only
        /// [`Permissions::FORBID_OWNER_READ`] applies.
        const FORBID_GROUP_READ = (1 << 8);

        /// Reserved; should always be set.
        const RESERVED9 = (1 << 9);

        /// Whether the file should not be executable by members of the group.
        ///
        /// If the user in question is both the owner and a member of the group, only
        /// [`Permissions::FORBID_OWNER_EXECUTE`] applies.
        const FORBID_GROUP_EXECUTE = (1 << 10);

        /// Reserved; should always be set.
        const RESERVED11 = (1 << 11);

        /// Whether the file should not be readable by other users.
        ///
        /// A user is considered one of the _other users_ if they are neither the owner of the file
        /// nor a member of the group of the file.
        const FORBID_OTHER_READ = (1 << 12);

        /// Reserved; should always be set.
        const RESERVED13 = (1 << 13);

        /// Whether the file should not be executable by other users.
        ///
        /// A user is considered one of the _other users_ if they are neither the owner of the file
        /// nor a member of the group of the file.
        const FORBID_OTHER_EXECUTE = (1 << 14);

        /// Reserved; should always be set.
        const RESERVED15 = (1 << 15);
    }
}

/// A record in the path table.
///
/// The endianness of some fields depends on the endianness of this particular path table. The
/// primary volume descriptor contains a link to a big-endian path table and a little-endian path
/// table.
#[derive(Clone, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PathTableRecord {
    // pub directory_identifier_length: u8,

    /// The length of the extended attribute record, in bytes.
    pub extended_attribute_record_length: u8,

    /// The location of the first logical block of the directory's extent.
    ///
    /// The endianness depends on the endianness of this particular path table.
    pub extent_location: u32,

    /// The number of the parent directory.
    ///
    /// The endianness depends on the endianness of this particular path table.
    pub parent_directory_number: u16,

    /// The identifier describing this directory.
    ///
    /// Must only contain one of the following:
    /// * a sequence of at least one d-character
    /// * on ISO9660 but not High Sierra volumes: a sequence of at least one d1-character
    /// * the byte 0x00 (to refer to the root directory)
    pub directory_identifier: Vec<u8>, // [u8; directory_identifier_length]

    /// A reserved field to re-align the next path table record.
    ///
    /// Only present if the length of the directory identifier is an odd number.
    pub reserved0: Option<u8>,

    // note that the order of fields is very different in High Sierra:
    // 1. extent_location
    // 2. extended_attribute_record_length
    // 3. directory_identifier_length
    // 4. parent_directory_number
    // 5. directory_identifier
    // 6. reserved0
}
