//! MZ (Mark Zbikowski) executable format.
//!
//! MZ executables are the native DOS executable format.
//!
//! Every PE and NE executable is simultaneously an MZ executable, although the executable data
//! interpreted from the MZ structures often only prints an error message and terminates.


use std::io::{self, Read, Seek, SeekFrom};


pub const BYTES_PER_PARAGRAPH: usize = 16;
pub const BYTES_PER_PAGE: usize = 512;


#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Executable {
    // signature: b"MZ",
    pub last_page_bytes: u16,
    pub pages: u16,
    // relocation_items: u16,
    pub header_size_paragraphs: u16,
    pub required_allocation_paragraphs: u16,
    pub requested_allocation_paragraphs: u16,
    pub initial_ss: u16,
    pub initial_sp: u16,
    pub checksum: u16,
    pub initial_ip: u16,
    pub initial_cs: u16,
    pub relocation_table_offset: u16,
    pub overlay: u16,
    pub relocation_entries: Vec<RelocationEntry>, // [RelocationEntry; relocation_items]
}
impl Executable {
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, io::Error> {
        let mut signature = [0u8; 2];
        reader.read_exact(&mut signature)?;
        if &signature != b"MZ" {
            return Err(io::ErrorKind::InvalidData.into());
        }

        let mut header_buf = [0u8; 26];
        reader.read_exact(&mut header_buf)?;

        let last_page_bytes = u16::from_le_bytes(header_buf[0..2].try_into().unwrap());
        let pages = u16::from_le_bytes(header_buf[2..4].try_into().unwrap());
        let relocation_items = u16::from_le_bytes(header_buf[4..6].try_into().unwrap());
        let header_size_paragraphs = u16::from_le_bytes(header_buf[6..8].try_into().unwrap());
        let required_allocation_paragraphs = u16::from_le_bytes(header_buf[8..10].try_into().unwrap());
        let requested_allocation_paragraphs = u16::from_le_bytes(header_buf[10..12].try_into().unwrap());
        let initial_ss = u16::from_le_bytes(header_buf[12..14].try_into().unwrap());
        let initial_sp = u16::from_le_bytes(header_buf[14..16].try_into().unwrap());
        let checksum = u16::from_le_bytes(header_buf[16..18].try_into().unwrap());
        let initial_ip = u16::from_le_bytes(header_buf[18..20].try_into().unwrap());
        let initial_cs = u16::from_le_bytes(header_buf[20..22].try_into().unwrap());
        let relocation_table_offset = u16::from_le_bytes(header_buf[22..24].try_into().unwrap());
        let overlay = u16::from_le_bytes(header_buf[24..26].try_into().unwrap());

        // seek to relocation table
        reader.seek(SeekFrom::Start(relocation_table_offset.into()))?;

        let mut relocation_entries = Vec::with_capacity(relocation_items.into());
        for _ in 0..relocation_items {
            let mut buf = [0u8; 4];
            reader.read_exact(&mut buf)?;

            let offset = u16::from_le_bytes(buf[0..2].try_into().unwrap());
            let segment = u16::from_le_bytes(buf[2..4].try_into().unwrap());
            relocation_entries.push(RelocationEntry {
                offset,
                segment,
            });
        }

        Ok(Self {
            last_page_bytes,
            pages,
            header_size_paragraphs,
            required_allocation_paragraphs,
            requested_allocation_paragraphs,
            initial_ss,
            initial_sp,
            checksum,
            initial_ip,
            initial_cs,
            relocation_table_offset,
            overlay,
            relocation_entries,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RelocationEntry {
    pub offset: u16,
    pub segment: u16,
}
