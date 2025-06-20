//! Reading the Microsoft Cabinet format.
//!
//! Conceptually, cabinet files are subdivided into folders (which are unrelated to file system
//! directories) which are further subdivided into data blocks. A folder, in its uncompressed state,
//! is a concatenation of all files within it, which is then subdivided into data blocks, which are
//! then optionally compressed. There tends to be some carry-over in the compression mechanism
//! between data blocks within a folder; for example, with MSZIP (DEFLATE) compression, the lookback
//! buffer (or dictionary, in zlib parlance) is preserved between data blocks of the same folder.
//! Thus, decisions about compression are made on the folder level, not on the data block level.
//!
//! Files within a cabinet file are a reference to a folder, an uncompressed byte position and an
//! uncompressed byte length. It might therefore be necessary to decompress the whole folder until
//! the requested file is reached.
//!
//! The payload may also be subdivided into multiple cabinet files; a file may span multiple cabinet
//! files (but only one folder per cabinet file). `expandms` probably won't support extracting
//! spanned files anytime soon, however.


use std::io::{self, Read, Seek};

use bitflags::bitflags;
use from_to_repr::from_to_other;

use crate::io_util::{ByteBufReadable, ReadEndian};


#[derive(Clone, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CabHeader {
    pub signature: [u8; 4],
    pub reserved1: u32,
    pub total_size_bytes: u32,
    pub reserved2: u32,
    pub first_file_offset: u32,
    pub reserved3: u32,
    pub minor_version: u8,
    pub major_version: u8,
    pub folder_count: u16,
    pub file_count: u16,
    pub flags: CabFlags, // u16
    pub set_id: u16,
    pub cabinet_index_in_set: u16,
    // header_reserved_length: Option<u16>, // if flags.contains(CabFlags::RESERVE_PRESENT)
    pub folder_reserved_length: Option<u8>,
    pub data_reserved_length: Option<u8>,
    pub reserved_data: Vec<u8>, // [u8; header_reserved_length.unwrap_or(0)]
    pub previous_cabinet_name: Option<Vec<u8>>, // only if PREV_CABINET flag set; 0x00-terminated
    pub previous_disk_name: Option<Vec<u8>>, // only if PREV_CABINET flag set; 0x00-terminated
    pub next_cabinet_name: Option<Vec<u8>>, // only if NEXT_CABINET flag set; 0x00-terminated
    pub next_disk_name: Option<Vec<u8>>, // only if NEXT_CABINET flag set; 0x00-terminated
}
impl CabHeader {
    pub fn read<R: Read>(reader: &mut R) -> Result<Self, io::Error> {
        let mut signature = [0u8; 4];
        reader.read_exact(&mut signature)?;
        if &signature != b"MSCF" {
            return Err(io::ErrorKind::InvalidData.into());
        }

        let mut fixed_part_buf = [0u8; 32];
        reader.read_exact(&mut fixed_part_buf)?;
        let mut pos = 0;
        let reserved1 = ReadEndian::read_le(&fixed_part_buf, &mut pos);
        let total_size_bytes = ReadEndian::read_le(&fixed_part_buf, &mut pos);
        let reserved2 = ReadEndian::read_le(&fixed_part_buf, &mut pos);
        let first_file_offset = ReadEndian::read_le(&fixed_part_buf, &mut pos);
        let reserved3 = ReadEndian::read_le(&fixed_part_buf, &mut pos);
        let minor_version = ByteBufReadable::read(&fixed_part_buf, &mut pos);
        let major_version = ByteBufReadable::read(&fixed_part_buf, &mut pos);
        let folder_count = ReadEndian::read_le(&fixed_part_buf, &mut pos);
        let file_count = ReadEndian::read_le(&fixed_part_buf, &mut pos);
        let flags = CabFlags::from_bits_retain(ReadEndian::read_le(&fixed_part_buf, &mut pos));
        let set_id = ReadEndian::read_le(&fixed_part_buf, &mut pos);
        let cabinet_index_in_set = ReadEndian::read_le(&fixed_part_buf, &mut pos);

        let (
            header_reserved_length,
            folder_reserved_length,
            data_reserved_length,
        ) = if flags.contains(CabFlags::RESERVE_PRESENT) {
            let mut reserved_lengths_buf = [0u8; 4];
            reader.read_exact(&mut reserved_lengths_buf)?;
            let h = u16::from_le_bytes(reserved_lengths_buf[0..2].try_into().unwrap());
            let f = reserved_lengths_buf[2];
            let d = reserved_lengths_buf[3];
            (Some(h), Some(f), Some(d))
        } else {
            (None, None, None)
        };

        let mut reserved_data = vec![0u8; header_reserved_length.unwrap_or(0).into()];
        reader.read_exact(&mut reserved_data)?;

        let (previous_cabinet_name, previous_disk_name) = if flags.contains(CabFlags::PREV_CABINET) {
            let pcn = read_until_0_and_return(reader)?;
            let pdn = read_until_0_and_return(reader)?;
            (Some(pcn), Some(pdn))
        } else {
            (None, None)
        };

        let (next_cabinet_name, next_disk_name) = if flags.contains(CabFlags::NEXT_CABINET) {
            let ncn = read_until_0_and_return(reader)?;
            let ndn = read_until_0_and_return(reader)?;
            (Some(ncn), Some(ndn))
        } else {
            (None, None)
        };

        Ok(Self {
            signature,
            reserved1,
            total_size_bytes,
            reserved2,
            first_file_offset,
            reserved3,
            minor_version,
            major_version,
            folder_count,
            file_count,
            flags,
            set_id,
            cabinet_index_in_set,
            folder_reserved_length,
            data_reserved_length,
            reserved_data,
            previous_cabinet_name,
            previous_disk_name,
            next_cabinet_name,
            next_disk_name,
        })
    }
}


bitflags! {
    #[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
    pub struct CabFlags : u16 {
        const PREV_CABINET = 0x0001;
        const NEXT_CABINET = 0x0002;
        const RESERVE_PRESENT = 0x0004;
    }
}


#[derive(Clone, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CabFolder {
    pub start_offset: u32,
    pub data_count: u16,
    pub compression_type: CompressionType, // u16
    pub reserved_data: Vec<u8>, // [u8; header.folder_reserved_length.unwrap_or(0)]
}
impl CabFolder {
    pub fn read<R: Read>(reader: &mut R, header: &CabHeader) -> Result<Self, io::Error> {
        let mut fixed_part_buf = [0u8; 8];
        reader.read_exact(&mut fixed_part_buf)?;
        let mut pos = 0;
        let start_offset = ReadEndian::read_le(&fixed_part_buf, &mut pos);
        let data_count = ReadEndian::read_le(&fixed_part_buf, &mut pos);
        let compression_type = CompressionType::from_base_type(ReadEndian::read_le(&fixed_part_buf, &mut pos));

        let mut reserved_data = vec![0u8; header.folder_reserved_length.unwrap_or(0).into()];
        reader.read_exact(&mut reserved_data)?;

        Ok(Self {
            start_offset,
            data_count,
            compression_type,
            reserved_data,
        })
    }
}


#[derive(Clone, Copy, Debug)]
#[from_to_other(base_type = u16, derive_compare = "as_int")]
pub enum CompressionType {
    NoCompression = 0x0000,
    MsZip = 0x0001,
    Quantum = 0x0002,
    Lzx = 0x0003,
    Other(u16),
}
impl Default for CompressionType {
    fn default() -> Self { Self::NoCompression }
}


#[derive(Clone, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct FileInCab {
    pub uncompressed_size_bytes: u32,
    pub uncompressed_offset_in_folder: u32,
    pub folder_index: FolderIndex, // u16
    pub date: CabDate, // u16
    pub time: CabTime, // u16
    pub attributes: FileInCabAttributes, // u16
    pub name: Vec<u8>, // 0x00-terminated
}
impl FileInCab {
    pub fn read<R: Read>(reader: &mut R) -> Result<Self, io::Error> {
        let mut fixed_part_buf = [0u8; 16];
        reader.read_exact(&mut fixed_part_buf)?;
        let mut pos = 0;
        let uncompressed_size_bytes = ReadEndian::read_le(&fixed_part_buf, &mut pos);
        let uncompressed_offset_in_folder = ReadEndian::read_le(&fixed_part_buf, &mut pos);
        let folder_index = FolderIndex::from_base_type(ReadEndian::read_le(&fixed_part_buf, &mut pos));
        let date = CabDate(ReadEndian::read_le(&fixed_part_buf, &mut pos));
        let time = CabTime(ReadEndian::read_le(&fixed_part_buf, &mut pos));
        let attributes = FileInCabAttributes::from_bits_retain(ReadEndian::read_le(&fixed_part_buf, &mut pos));

        let name = read_until_0_and_return(reader)?;

        Ok(Self {
            uncompressed_size_bytes,
            uncompressed_offset_in_folder,
            folder_index,
            date,
            time,
            attributes,
            name,
        })
    }
}


#[derive(Clone, Copy, Debug)]
#[from_to_other(base_type = u16, derive_compare = "as_int")]
pub enum FolderIndex {
    RegularIndex(u16),
    ContinuedFromPrevious = 0xFFFD,
    ContinuedToNext = 0xFFFE,
    ContinuedPreviousAndNext = 0xFFFF,
}
impl Default for FolderIndex {
    fn default() -> Self { Self::RegularIndex(0) }
}


#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CabDate(u16);
impl CabDate {
    // YYYY YYYM MMMD DDDD
    pub fn year(&self) -> u16 {
        1980 + ((self.0 >> 9) & 0b111_1111)
    }

    pub fn month(&self) -> u8 {
        u8::try_from((self.0 >> 5) & 0b1111).unwrap()
    }

    pub fn day(&self) -> u8 {
        u8::try_from((self.0 >> 0) & 0b1_1111).unwrap()
    }
}


#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CabTime(u16);
impl CabTime {
    // HHHH HMMM MMMS SSSS
    pub fn hour(&self) -> u8 {
        u8::try_from((self.0 >> 11) & 0b1_1111).unwrap()
    }

    pub fn minute(&self) -> u8 {
        u8::try_from((self.0 >> 5) & 0b11_1111).unwrap()
    }

    pub fn second(&self) -> u8 {
        2 * u8::try_from((self.0 >> 0) & 0b1_1111).unwrap()
    }
}


bitflags! {
    #[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
    pub struct FileInCabAttributes : u16 {
        const READ_ONLY = 0x0001;
        const HIDDEN = 0x0002;
        const SYSTEM = 0x0004;
        const ARCHIVE = 0x0020;

        const EXECUTE = 0x0040;
        const UTF8_NAME = 0x0080;
    }
}


#[derive(Clone, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CabData {
    pub checksum: u32,
    pub compressed_byte_count: u16,
    pub uncompressed_byte_count: u16,
    pub reserved_data: Vec<u8>, // [u8; header.data_reserved_length.unwrap_or(0)]
    pub data_offset: usize,
}
impl CabData {
    pub fn read<R: Read + Seek>(reader: &mut R, header: &CabHeader) -> Result<Self, io::Error> {
        let mut fixed_part_buf = [0u8; 8];
        reader.read_exact(&mut fixed_part_buf)?;
        let mut pos = 0;
        let checksum = ReadEndian::read_le(&fixed_part_buf, &mut pos);
        let compressed_byte_count = ReadEndian::read_le(&fixed_part_buf, &mut pos);
        let uncompressed_byte_count = ReadEndian::read_le(&fixed_part_buf, &mut pos);

        let mut reserved_data = vec![0u8; header.data_reserved_length.unwrap_or(0).into()];
        reader.read_exact(&mut reserved_data)?;

        let data_offset = reader.seek(io::SeekFrom::Current(0))?.try_into().unwrap();

        Ok(Self {
            checksum,
            compressed_byte_count,
            uncompressed_byte_count,
            reserved_data,
            data_offset,
        })
    }
}


fn read_until_0<R: Read>(reader: &mut R, output: &mut Vec<u8>) -> Result<(), io::Error> {
    let mut buf = [0u8];
    loop {
        reader.read_exact(&mut buf)?;
        if buf[0] == 0x00 {
            break;
        }
        output.push(buf[0]);
    }
    Ok(())
}


fn read_until_0_and_return<R: Read>(reader: &mut R) -> Result<Vec<u8>, io::Error> {
    let mut output = Vec::new();
    read_until_0(reader, &mut output)?;
    Ok(output)
}
