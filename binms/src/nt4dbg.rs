//! Debugging symbols format from the Windows NT 4 days.


use std::io::{self, Read};

use from_to_repr::from_to_other;

use crate::pe::{SectionTable, SectionTableEntry};


#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DbgFile {
    pub header: Header,
    pub section_table: SectionTable, // yup, same structure as PE
    pub exported_names: Vec<Vec<u8>>, // [ByteString; next_nul_terminated_strings_for(header.exported_names_table_size)]
    pub debug_directories: Vec<DebugDirectory>, // [DebugDirectory; header.exported_names_table_size / sizeof(TODO)]
}
impl DbgFile {
    pub fn read<R: Read>(reader: &mut R) -> Result<Self, io::Error> {
        let header = Header::read(reader)?;

        let mut sections = Vec::with_capacity(header.section_count.try_into().unwrap());
        for _ in 0..header.section_count {
            let section = SectionTableEntry::read(reader)?;
            sections.push(section);
        }
        let section_table = SectionTable::from(sections);

        let mut exported_names_buf = vec![0u8; header.exported_names_table_size.try_into().unwrap()];
        reader.read_exact(&mut exported_names_buf)?;
        while let Some(b) = exported_names_buf.last() {
            if *b != 0 {
                break;
            }
            exported_names_buf.pop();
        }
        let exported_names: Vec<Vec<u8>> = exported_names_buf
            .split(|b| *b == 0x00)
            .map(|bs| bs.to_vec())
            .collect();

        let mut debug_directories = Vec::with_capacity(header.debug_directories_size.try_into().unwrap());
        for _ in 0..header.debug_directories_size {
            let debug_directory = DebugDirectory::read(reader)?;
            debug_directories.push(debug_directory);
        }

        Ok(Self {
            header,
            section_table,
            exported_names,
            debug_directories,
        })
    }
}


#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Header {
    pub signature: u16, // 0x4944
    pub flags: u16,
    pub machine: u16,
    pub characteristics: u16,
    pub time_date_stamp: u32,
    pub image_checksum: u32,
    pub image_base: u32,
    pub image_size: u32,
    pub section_count: u32,
    pub exported_names_table_size: u32,
    pub debug_directories_size: u32,
    pub unknown: [u8; 12],
}
impl Header {
    pub fn read<R: Read>(reader: &mut R) -> Result<Self, io::Error> {
        let mut header_buf = [0u8; 48];
        reader.read_exact(&mut header_buf)?;

        let signature = u16::from_le_bytes(header_buf[0..2].try_into().unwrap());
        let flags = u16::from_le_bytes(header_buf[2..4].try_into().unwrap());
        let machine = u16::from_le_bytes(header_buf[4..6].try_into().unwrap());
        let characteristics = u16::from_le_bytes(header_buf[6..8].try_into().unwrap());
        let time_date_stamp = u32::from_le_bytes(header_buf[8..12].try_into().unwrap());
        let image_checksum = u32::from_le_bytes(header_buf[12..16].try_into().unwrap());
        let image_base = u32::from_le_bytes(header_buf[16..20].try_into().unwrap());
        let image_size = u32::from_le_bytes(header_buf[20..24].try_into().unwrap());
        let section_count = u32::from_le_bytes(header_buf[24..28].try_into().unwrap());
        let exported_names_table_size = u32::from_le_bytes(header_buf[28..32].try_into().unwrap());
        let debug_directories_size = u32::from_le_bytes(header_buf[32..36].try_into().unwrap());
        let unknown = header_buf[36..48].try_into().unwrap();

        Ok(Self {
            signature,
            flags,
            machine,
            characteristics,
            time_date_stamp,
            image_checksum,
            image_base,
            image_size,
            section_count,
            exported_names_table_size,
            debug_directories_size,
            unknown,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DebugDirectory {
    pub characteristics: u32,
    pub time_date_stamp: u32,
    pub major_version: u16,
    pub minor_version: u16,
    pub kind: DebugType,
    pub size: u32,
    pub virtual_address: u32, // generally 0
    pub raw_data_pointer: u32,
}
impl DebugDirectory {
    pub fn read<R: Read>(reader: &mut R) -> Result<Self, io::Error> {
        let mut buf = [0u8; 28];
        reader.read_exact(&mut buf)?;

        let characteristics = u32::from_le_bytes(buf[0..4].try_into().unwrap());
        let time_date_stamp = u32::from_le_bytes(buf[4..8].try_into().unwrap());
        let major_version = u16::from_le_bytes(buf[8..10].try_into().unwrap());
        let minor_version = u16::from_le_bytes(buf[10..12].try_into().unwrap());
        let kind_u32 = u32::from_le_bytes(buf[12..16].try_into().unwrap());
        let kind = DebugType::from_base_type(kind_u32);
        let size = u32::from_le_bytes(buf[16..20].try_into().unwrap());
        let virtual_address = u32::from_le_bytes(buf[20..24].try_into().unwrap());
        let raw_data_pointer = u32::from_le_bytes(buf[24..28].try_into().unwrap());

        Ok(Self {
            characteristics,
            time_date_stamp,
            major_version,
            minor_version,
            kind,
            size,
            virtual_address,
            raw_data_pointer,
        })
    }
}

#[derive(Clone, Copy, Debug)]
#[from_to_other(base_type = u32, derive_compare = "as_int")]
pub enum DebugType {
    Unknown = 0,
    Coff = 1,
    CodeView = 2,
    FramePointerOmission = 3,
    DbgFileLocation = 4,
    Exception = 5,
    Fixup = 6,
    OmapToSource = 7,
    OmapFromSource = 8,
    Borland = 9,
    Clsid = 11,
    Reproducibility = 16,
    EmbeddedData = 17,
    SymbolFileHash = 19,
    ExtendedDllCharacteristics = 20,
    Other(u32),
}
