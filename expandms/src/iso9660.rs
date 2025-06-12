//! Decoding CD-ROM file systems.


use bitflags::bitflags;
use from_to_repr::from_to_other;


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


/// The same value, encoded once as big and once as little endian.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct EndianPair<T> {
    pub little_endian: T,
    pub big_endian: T,
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
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
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
/// b'0' and the GMT offset to 0. This is the only situation where `year`, `month` or `day` may
/// contain zero values, as years, days and months are customarily numbered starting with 1.
///
/// 17 bytes on ISO9660 (§ 8.4.26.1), 16 bytes on High Sierra (§ 11.4.30.1).
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DigitTimestamp {
    /// The year, in ASCII digits from b"0001" to b"9999", or b"0000" if encoding the zero value.
    pub year: [u8; 4],

    /// The month, in ASCII digits from b"01" to b"12", or b"00" if encoding the zero value.
    pub month: [u8; 2],

    /// The month, in ASCII digits from b"01" to b"31", or b"00" if encoding the zero value.
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
