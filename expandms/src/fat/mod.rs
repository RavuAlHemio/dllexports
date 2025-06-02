//! Routines for reading File Allocation Table file systems.
//!
//! Should support FAT12/FAT16/FAT32 from MS-DOS 2.0 onward.


use std::io::{self, Read, Seek, SeekFrom};

use bitflags::bitflags;

use crate::DisplayBytes;


#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum FatVariant {
    Fat12,
    Fat16,
    Fat32,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RootDirectoryLocation {
    Sector(u32),
    Cluster(u32),
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct FatHeader {
    pub jump: [u8; 3],
    pub oem_name: [u8; 8],
    // here starts the "BIOS Parameter Block"
    // which is, for all intents and purposes, part of the header
    pub bytes_per_sector: u16,
    pub sectors_per_cluster: u8,
    pub reserved_sector_count: u16,
    pub fat_count: u8,
    pub max_root_dir_entries: u16,
    pub total_sector_count: u32,
    pub media_descriptor: u8,
    pub sectors_per_fat: u32,
    pub root_directory_location: RootDirectoryLocation,
}
impl FatHeader {
    pub fn read<R: Read>(reader: &mut R) -> Result<Self, io::Error> {
        let mut header_buf = [0u8; 24];
        reader.read_exact(&mut header_buf)?;

        let jump = header_buf[0..3].try_into().unwrap();
        let oem_name = header_buf[3..11].try_into().unwrap();
        let bytes_per_sector = u16::from_le_bytes(header_buf[11..13].try_into().unwrap());
        let sectors_per_cluster = header_buf[13];
        let reserved_sector_count = u16::from_le_bytes(header_buf[14..16].try_into().unwrap());
        let fat_count = header_buf[16];
        let max_root_dir_entries = u16::from_le_bytes(header_buf[17..19].try_into().unwrap());
        let total_sector_count: u32 = u16::from_le_bytes(header_buf[19..21].try_into().unwrap()).into();
        let media_descriptor = header_buf[21];
        let sectors_per_fat: u32 = u16::from_le_bytes(header_buf[22..24].try_into().unwrap()).into();

        let mut fat_header = Self {
            jump,
            oem_name,
            bytes_per_sector,
            sectors_per_cluster,
            reserved_sector_count,
            fat_count,
            max_root_dir_entries,
            total_sector_count,
            media_descriptor,
            sectors_per_fat,
            root_directory_location: RootDirectoryLocation::Sector(0),
        };

        if fat_header.total_sector_count == 0 || fat_header.sectors_per_fat == 0 {
            // take 32-bit value at 0x0020 or 0x0024, respectively

            // we are currently at 0x0016
            reader.read_exact(&mut header_buf[..18])?;
            if fat_header.total_sector_count == 0 {
                fat_header.total_sector_count = u32::from_le_bytes(header_buf[10..14].try_into().unwrap());
            }
            if fat_header.sectors_per_fat == 0 {
                fat_header.sectors_per_fat = u32::from_le_bytes(header_buf[14..18].try_into().unwrap());
            }
        }

        if fat_header.variant() == FatVariant::Fat32 {
            // root directory is stored in the cluster numbered at 0x002C
            // we are currently at 0x0028
            reader.read_exact(&mut header_buf[0..8])?;
            let root_directory_cluster = u32::from_le_bytes(header_buf[4..8].try_into().unwrap());
            fat_header.root_directory_location = RootDirectoryLocation::Cluster(root_directory_cluster);
        } else {
            // root directory starts after reserved sectors and FATs
            // and is only one sector long
            let sector =
                u32::from(reserved_sector_count)
                + u32::from(fat_count) * fat_header.sectors_per_fat;
            fat_header.root_directory_location = RootDirectoryLocation::Sector(sector);
        }

        Ok(fat_header)
    }

    pub fn total_cluster_count(&self) -> u32 {
        self.total_sector_count / u32::from(self.sectors_per_cluster)
    }

    pub fn variant(&self) -> FatVariant {
        let cluster_count = self.total_cluster_count();
        if cluster_count < 4087 {
            FatVariant::Fat12
        } else if cluster_count < 65526 {
            FatVariant::Fat16
        } else {
            FatVariant::Fat32
        }
    }

    pub fn fat_bytes(&self) -> usize {
        usize::try_from(self.sectors_per_fat).unwrap() * usize::from(self.bytes_per_sector)
    }

    pub fn first_data_sector(&self) -> u32 {
        // 1. reserved sectors
        // 2. sectors with FATs
        // 3. sectors with root directory
        // 4. data sectors

        u32::from(self.reserved_sector_count)
            + u32::from(self.fat_count) * self.sectors_per_fat
            + u32::from(self.max_root_dir_entries) * 32 / u32::from(self.bytes_per_sector)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum FatEntry {
    Empty, // 0...0
    Chain(u32), // any other value
    Bad, // F...F7
    MediaType(u8), // F...Fx where x is the media type
    Sentinel, // F...FF
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct AllocationTable {
    pub entries: Vec<FatEntry>,
}
impl AllocationTable {
    fn check_initial_entry_correctness(entries: &[FatEntry]) -> Result<(), io::Error> {
        if entries.len() > 0 {
            // first value must be a media type entry
            if !matches!(entries[0], FatEntry::MediaType(_)) {
                return Err(io::ErrorKind::InvalidData.into());
            }
        }
        if entries.len() > 1 {
            // second value must be the sentinel value
            if entries[1] != FatEntry::Sentinel {
                return Err(io::ErrorKind::InvalidData.into());
            }
        }
        Ok(())
    }

    pub fn read<R: Read>(reader: &mut R, variant: FatVariant, byte_count: usize) -> Result<Self, io::Error> {
        let mut buf = vec![0u8; byte_count];
        reader.read_exact(&mut buf)?;

        let mut entries = Vec::new();
        match variant {
            FatVariant::Fat12 => {
                // 3 bytes -> 2 entries
                for chunk in buf.chunks_exact(3) {
                    let first =
                        u32::from(chunk[0])
                        | ((u32::from(chunk[1]) & 0x0F) << 8);
                    let second =
                        ((u32::from(chunk[1]) & 0xF0) >> 4)
                        | (u32::from(chunk[2]) << 4);
                    for entry in [first, second] {
                        let typed_entry = match entry {
                            0x000 => FatEntry::Empty,
                            0xFF7 => FatEntry::Bad,
                            0xFFF => FatEntry::Sentinel,
                            0xFF0..=0xFFF => FatEntry::MediaType((entry & 0x0F).try_into().unwrap()),
                            other => FatEntry::Chain(other),
                        };
                        entries.push(typed_entry);
                    }

                    if entries.len() == 2 {
                        Self::check_initial_entry_correctness(&entries)?;
                    }
                }
            },
            FatVariant::Fat16 => {
                for chunk in buf.chunks_exact(2) {
                    let entry: u32 = u16::from_le_bytes(chunk.try_into().unwrap()).into();
                    let typed_entry = match entry {
                        0x0000 => FatEntry::Empty,
                        0xFFF7 => FatEntry::Bad,
                        0xFFFF => FatEntry::Sentinel,
                        0xFFF0..=0xFFFF => FatEntry::MediaType((entry & 0x0F).try_into().unwrap()),
                        other => FatEntry::Chain(other),
                    };
                    entries.push(typed_entry);

                    if entries.len() == 2 {
                        Self::check_initial_entry_correctness(&entries)?;
                    }
                }
            },
            FatVariant::Fat32 => {
                for chunk in buf.chunks_exact(4) {
                    // FAT32 is actually FAT28
                    let entry = u32::from_le_bytes(chunk.try_into().unwrap()) & 0x0FFF_FFFF;
                    let typed_entry = match entry {
                        0x0000_0000 => FatEntry::Empty,
                        0x0FFF_FFF7 => FatEntry::Bad,
                        0x0FFF_FFFF => FatEntry::Sentinel,
                        0x0FFF_FFF0..=0x0FFF_FFFF => FatEntry::MediaType((entry & 0x0F).try_into().unwrap()),
                        other => FatEntry::Chain(other),
                    };
                    entries.push(typed_entry);

                    if entries.len() == 2 {
                        Self::check_initial_entry_correctness(&entries)?;
                    }
                }
            },
        }

        Ok(Self {
            entries,
        })
    }
}

fn read_next_sector_into<R: Read>(reader: &mut R, header: &FatHeader, output: &mut Vec<u8>) -> Result<(), io::Error> {
    let old_len = output.len();
    for _ in 0..header.bytes_per_sector {
        output.push(0x00);
    }
    reader.read_exact(&mut output[old_len..])?;
    Ok(())
}

fn read_next_cluster_into<R: Read>(reader: &mut R, header: &FatHeader, output: &mut Vec<u8>) -> Result<(), io::Error> {
    for _ in 0..header.sectors_per_cluster {
        read_next_sector_into(reader, header, output)?;
    }
    Ok(())
}

fn seek_to_cluster<R: Seek>(reader: &mut R, header: &FatHeader, cluster_index: u32) -> Result<(), io::Error> {
    let cluster_start_sector = u64::from(header.first_data_sector())
        + u64::try_from(cluster_index - 2).unwrap() * u64::from(header.sectors_per_cluster);
    let cluster_start_byte = cluster_start_sector
        * u64::from(header.bytes_per_sector);
    reader.seek(SeekFrom::Start(cluster_start_byte))?;
    Ok(())
}

pub fn read_sector_into<R: Read + Seek>(reader: &mut R, header: &FatHeader, sector_index: u32, output: &mut Vec<u8>) -> Result<(), io::Error> {
    let sector_start_byte = u64::from(header.bytes_per_sector) * u64::from(sector_index);
    reader.seek(SeekFrom::Start(sector_start_byte))?;
    read_next_sector_into(reader, header, output)
}

pub fn read_cluster_chain_into<R: Read + Seek>(reader: &mut R, header: &FatHeader, fat: &AllocationTable, first_cluster_index: u32, output: &mut Vec<u8>) -> Result<(), io::Error> {
    let mut prev_cluster_index = None;
    let mut current_cluster_index = first_cluster_index;

    loop {
        // read the cluster entry from the allocation table
        let current_cluster_entry = fat.entries[usize::try_from(current_cluster_index).unwrap()];
        match current_cluster_entry {
            FatEntry::Empty => return Ok(()),
            FatEntry::Bad => return Err(io::ErrorKind::InvalidData.into()),
            FatEntry::MediaType(_) => return Err(io::ErrorKind::InvalidData.into()),
            FatEntry::Sentinel|FatEntry::Chain(_) => {},
        }

        // read the cluster
        if let Some(pci) = prev_cluster_index {
            if pci + 1 != current_cluster_index {
                // fragmented file; seek
                seek_to_cluster(reader, header, current_cluster_index)?;
            }
            // otherwise, we can just keep reading
        } else {
            // first cluster; seek
            seek_to_cluster(reader, header, current_cluster_index)?;
        }
        read_next_cluster_into(reader, header, output)?;

        // do we continue?
        match current_cluster_entry {
            FatEntry::Sentinel => {
                // we're done
                break;
            },
            FatEntry::Chain(next_cluster_index) => {
                // pointer to the next cluster
                prev_cluster_index = Some(current_cluster_index);
                current_cluster_index = next_cluster_index;
            },
            _ => unreachable!(),
        }
    }

    Ok(())
}


bitflags! {
    #[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
    pub struct Attributes : u8 {
        const READ_ONLY = 0b0000_0001;
        const HIDDEN = 0b0000_0010;
        const SYSTEM = 0b0000_0100;
        const VOLUME_LABEL = 0b0000_1000;
        const SUBDIRECTORY = 0b0001_0000;
        const ARCHIVE = 0b0010_0000;
        const DEVICE = 0b0100_0000;
        const RESERVED = 0b1000_0000;
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DirectoryEntry {
    pub file_name: DisplayBytes<8>,
    pub extension: DisplayBytes<3>,
    pub attributes: Attributes,
    pub reserved: u8,
    pub create_time_10ms: u8,
    pub create_time_h_m_2s: u16,
    pub create_date: u16,
    pub access_date: u16,
    pub reserved2: Option<u16>, // FAT32: top half of first cluster number
    pub modification_time_h_m_2s: u16,
    pub modification_date: u16,
    pub first_cluster_number: u32, // u16 (bottom half on FAT32)
    pub file_size_bytes: u32,
}
impl DirectoryEntry {
    pub fn read<R: Read>(reader: &mut R, variant: FatVariant) -> Result<Self, io::Error> {
        let mut buf = [0u8; 32];
        reader.read_exact(&mut buf)?;

        let file_name = buf[0..8].try_into().unwrap();
        let extension = buf[8..11].try_into().unwrap();
        let attributes = Attributes::from_bits_retain(buf[11]);
        let reserved = buf[12];
        let create_time_10ms = buf[13];
        let create_time_h_m_2s = u16::from_le_bytes(buf[14..16].try_into().unwrap());
        let create_date = u16::from_le_bytes(buf[16..18].try_into().unwrap());
        let access_date = u16::from_le_bytes(buf[18..20].try_into().unwrap());
        let reserved2 = if variant == FatVariant::Fat32 {
            None
        } else {
            Some(u16::from_le_bytes(buf[20..22].try_into().unwrap()))
        };
        let modification_time_h_m_2s = u16::from_le_bytes(buf[22..24].try_into().unwrap());
        let modification_date = u16::from_le_bytes(buf[24..26].try_into().unwrap());
        let first_cluster_number = if variant == FatVariant::Fat32 {
            let bottom_half: u32 = u16::from_le_bytes(buf[26..28].try_into().unwrap()).into();
            let top_half: u32 = u16::from_le_bytes(buf[20..22].try_into().unwrap()).into();
            (top_half << 16) | bottom_half
        } else {
            u16::from_le_bytes(buf[26..28].try_into().unwrap()).into()
        };
        let file_size_bytes = u32::from_le_bytes(buf[28..32].try_into().unwrap());

        Ok(Self {
            file_name,
            extension,
            attributes,
            reserved,
            create_time_10ms,
            create_time_h_m_2s,
            create_date,
            access_date,
            reserved2,
            modification_time_h_m_2s,
            modification_date,
            first_cluster_number,
            file_size_bytes,
        })
    }
}
