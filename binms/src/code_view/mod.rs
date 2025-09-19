//! Debug information in the CodeView format.
//!
//! Most of the structures have been derived from the contents of
//! https://www.os2site.com/sw/dev/openwatcom/docs/codeview.pdf.


pub mod leaves;
pub mod symbol_entries;


use std::io::{self, Cursor, Read, Seek, SeekFrom};

use display_bytes::{DisplayBytesSlice, DisplayBytesVec};
use from_to_repr::from_to_other;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use tracing::{debug, error, instrument};

use crate::read_pascal_byte_string;
use crate::code_view::leaves::TypeLeaf;
use crate::code_view::symbol_entries::SymbolEntry;
use crate::int_from_byte_slice::IntFromByteSlice;


#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct DebugInfo {
    pub signature: [u8; 4],
    pub directory_offset: u32,
    pub subsection_directory_header: SubsectionDirectoryHeader,
    pub subsection_directory_entries: Vec<SubsectionDirectoryEntry>, // [SubsectionDirectoryEntry; subsection_directory_header.entry_count]
}
impl DebugInfo {
    #[instrument(skip_all)]
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, io::Error> {
        let mut header_buf = [0u8; 8];
        reader.read_exact(&mut header_buf)?;

        let signature: [u8; 4] = header_buf[0..4].try_into().unwrap();
        if signature[0] != b'N' || signature[1] != b'B' {
            error!("debug info signature {:?} does not start with b\"NB\"", signature);
            return Err(io::ErrorKind::InvalidData.into());
        }

        let directory_offset = u32::from_le_bytes(header_buf[4..8].try_into().unwrap());

        reader.seek(SeekFrom::Start(directory_offset.into()))?;

        let subsection_directory_header = SubsectionDirectoryHeader::read(reader)?;
        let mut subsection_directory_metadata = Vec::with_capacity(subsection_directory_header.entry_count.try_into().unwrap());
        for _ in 0..subsection_directory_header.entry_count {
            let entry = SubsectionDirectoryEntryMetadata::read(reader)?;
            subsection_directory_metadata.push(entry);
        }

        let mut subsection_directory_entries = Vec::with_capacity(subsection_directory_metadata.len());
        for metadata in &subsection_directory_metadata {
            let mut data = vec![0u8; metadata.size_bytes.try_into().unwrap()];
            debug!("about to decode {:?}", metadata);
            debug!("seeking to {} to read {} bytes", metadata.offset, metadata.size_bytes);
            reader.seek(SeekFrom::Start(metadata.offset.into()))?;
            reader.read_exact(&mut data)?;
            let mut data_reader = Cursor::new(&data);

            debug!("decoding {:?}", metadata.subsection_type);
            let subsection_data = match metadata.subsection_type {
                SubsectionType::Module => {
                    let content = ModuleSubsection::read(&mut data_reader)?;
                    SubsectionData::Module(content)
                },
                /*
                SubsectionType::Types => SubsectionData::Other(),
                SubsectionType::PublicSymbolsLegacy => todo!(),
                SubsectionType::PublicSymbols => todo!(),
                */
                SubsectionType::Symbols|SubsectionType::AlignSymbols => {
                    let content = SymbolsSubsection::read(&mut data_reader)?;
                    SubsectionData::Symbols(content)
                },
                /*
                SubsectionType::SourceLineSegment => todo!(),
                */
                SubsectionType::SourceLineModule => {
                    let content = SourceLineModuleSubsection::read(&mut data_reader)?;
                    SubsectionData::SourceLineModule(content)
                },
                SubsectionType::Libraries => {
                    let content = LibrariesSubsection::read(&mut data_reader)?;
                    SubsectionData::Libraries(content)
                },
                SubsectionType::GlobalSymbols => {
                    let content = GlobalSymbolsSubsection::read(&mut data_reader)?;
                    SubsectionData::GlobalSymbols(content)
                },
                SubsectionType::GlobalPublicSymbols => {
                    let content = GlobalSymbolsSubsection::read(&mut data_reader)?;
                    SubsectionData::GlobalPublicSymbols(content)
                },
                SubsectionType::GlobalTypes => {
                    let content = GlobalTypesSubsection::read(&mut data_reader)?;
                    SubsectionData::GlobalTypes(content)
                },
                /*
                SubsectionType::MakePCode => todo!(),
                SubsectionType::SegmentMap => todo!(),
                SubsectionType::SegmentName => todo!(),
                SubsectionType::PreCompile => todo!(),
                SubsectionType::FileIndex => todo!(),
                */
                SubsectionType::StaticSymbols => {
                    // very much not global symbols, but the same structure
                    let content = GlobalSymbolsSubsection::read(&mut data_reader)?;
                    SubsectionData::GlobalPublicSymbols(content)
                },
                _ => SubsectionData::Other(DisplayBytesVec::from(data)),
            };

            let entry = SubsectionDirectoryEntry {
                metadata: metadata.clone(),
                data: subsection_data,
            };
            subsection_directory_entries.push(entry);
        }

        Ok(Self {
            signature,
            directory_offset,
            subsection_directory_header,
            subsection_directory_entries,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct SubsectionDirectoryHeader {
    pub header_length: u16,
    pub entry_length: u16,
    pub entry_count: u32,
    pub next_directory_offset: u32,
    pub flags: u32,
}
impl SubsectionDirectoryHeader {
    #[instrument(skip_all)]
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, io::Error> {
        let mut header_buf = [0u8; 16];
        reader.read_exact(&mut header_buf)?;

        let header_length = u16::from_le_bytes(header_buf[0..2].try_into().unwrap());
        if usize::from(header_length) != header_buf.len() {
            error!("subsection directory header is announced to be {} bytes long, expected {}", header_length, header_buf.len());
            return Err(io::ErrorKind::InvalidData.into());
        }

        let entry_length = u16::from_le_bytes(header_buf[2..4].try_into().unwrap());
        if entry_length != 12 {
            error!("subsection directory entries are announced to be {} bytes long each, expected {}", entry_length, 12);
            return Err(io::ErrorKind::InvalidData.into());
        }

        let entry_count = u32::from_le_bytes(header_buf[4..8].try_into().unwrap());
        let next_directory_offset = u32::from_le_bytes(header_buf[4..8].try_into().unwrap());
        let flags = u32::from_le_bytes(header_buf[4..8].try_into().unwrap());

        Ok(Self {
            header_length,
            entry_length,
            entry_count,
            next_directory_offset,
            flags,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct SubsectionDirectoryEntryMetadata {
    pub subsection_type: SubsectionType, // u16
    pub module_index: u16,
    pub offset: u32,
    pub size_bytes: u32,
}
impl SubsectionDirectoryEntryMetadata {
    #[instrument(skip_all)]
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, io::Error> {
        let mut header_buf = [0u8; 12];
        reader.read_exact(&mut header_buf)?;

        let subsection_type_u16 = u16::from_le_bytes(header_buf[0..2].try_into().unwrap());
        let subsection_type = SubsectionType::from_base_type(subsection_type_u16);
        let module_index = u16::from_le_bytes(header_buf[2..4].try_into().unwrap());
        let offset = u32::from_le_bytes(header_buf[4..8].try_into().unwrap());
        let size_bytes = u32::from_le_bytes(header_buf[8..12].try_into().unwrap());

        Ok(Self {
            subsection_type,
            module_index,
            offset,
            size_bytes,
        })
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct SubsectionDirectoryEntry {
    pub metadata: SubsectionDirectoryEntryMetadata,
    pub data: SubsectionData,
}

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
#[from_to_other(base_type = u16, derive_compare = "as_int")]
pub enum SubsectionType {
    Module = 0x120,
    Types = 0x121,
    PublicSymbolsLegacy = 0x122,
    PublicSymbols = 0x123,
    Symbols = 0x124,
    AlignSymbols = 0x125,
    SourceLineSegment = 0x126,
    SourceLineModule = 0x127,
    Libraries = 0x128,
    GlobalSymbols = 0x129,
    GlobalPublicSymbols = 0x12A,
    GlobalTypes = 0x12B,
    MakePCode = 0x12C,
    SegmentMap = 0x12D,
    SegmentName = 0x12E,
    PreCompile = 0x12F,
    FileIndex = 0x133,
    StaticSymbols = 0x134,
    Other(u16),
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub enum SubsectionData {
    Module(ModuleSubsection),
    //Types(TypesSubsection),
    Symbols(SymbolsSubsection),
    /*
    PublicSymbolsLegacy(PublicSymbolsLegacySubsection),
    PublicSymbols(PublicSymbolsSubsection),
    SourceLineSegment(SourceLineSegmentSubsection),
    */
    SourceLineModule(SourceLineModuleSubsection),
    Libraries(LibrariesSubsection),
    GlobalSymbols(GlobalSymbolsSubsection),
    GlobalPublicSymbols(GlobalSymbolsSubsection),
    GlobalTypes(GlobalTypesSubsection),
    /*
    MakePCode(MakePCodeSubsection),
    SegmentMap(SegmentMapSubsection),
    SegmentName(SegmentNameSubsection),
    PreCompile(PreCompileSubsection),
    FileIndex(FileIndexSubsection),
    StaticSymbols(StaticSymbolsSubsection),
    */
    Other(DisplayBytesVec),
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct ModuleSubsection {
    pub overlay_number: u16,
    pub library_index: u16,
    pub code_segment_count: u16,
    pub debugging_style: u16,
    pub segment_info: Vec<ModuleSegmentInfo>, // [ModuleSegmentInfo; code_segment_count]
    pub name: DisplayBytesVec, // PascalString
}
impl ModuleSubsection {
    #[instrument(skip_all)]
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, io::Error> {
        let mut header_buf = [0u8; 8];
        reader.read_exact(&mut header_buf)?;

        let overlay_number = u16::from_le_bytes(header_buf[0..2].try_into().unwrap());
        let library_index = u16::from_le_bytes(header_buf[2..4].try_into().unwrap());
        let code_segment_count = u16::from_le_bytes(header_buf[4..6].try_into().unwrap());
        let debugging_style = u16::from_le_bytes(header_buf[6..8].try_into().unwrap());

        let mut segment_info = Vec::with_capacity(usize::from(code_segment_count));
        for _ in 0..code_segment_count {
            let seg = ModuleSegmentInfo::read(reader)?;
            segment_info.push(seg);
        }
        let name_vec = read_pascal_byte_string(reader)?;
        let name = DisplayBytesVec::from(name_vec);

        Ok(Self {
            overlay_number,
            library_index,
            code_segment_count,
            debugging_style,
            segment_info,
            name,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct ModuleSegmentInfo {
    pub segment: u16,
    pub padding: u16,
    pub code_offset: u32,
    pub code_size: u32,
}
impl ModuleSegmentInfo {
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, io::Error> {
        let mut header_buf = [0u8; 12];
        reader.read_exact(&mut header_buf)?;

        let segment = u16::from_le_bytes(header_buf[0..2].try_into().unwrap());
        let padding = u16::from_le_bytes(header_buf[2..4].try_into().unwrap());
        let code_offset = u32::from_le_bytes(header_buf[4..8].try_into().unwrap());
        let code_size = u32::from_le_bytes(header_buf[8..12].try_into().unwrap());

        Ok(Self {
            segment,
            padding,
            code_offset,
            code_size,
        })
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct TypesSubsection {
    // TODO: suss out structure
    pub data: DisplayBytesVec,
}
impl TypesSubsection {
    #[instrument(skip_all)]
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, io::Error> {
        let mut data = Vec::new();
        reader.read_to_end(&mut data)?;
        Ok(Self {
            data: DisplayBytesVec::from(data),
        })
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct SymbolsSubsection {
    pub signature: u32, // 0x00000001
    pub symbols: Vec<SymbolEntry>,
}
impl SymbolsSubsection {
    #[instrument(skip_all)]
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, io::Error> {
        let mut signature_buf = [0u8; 4];
        reader.read_exact(&mut signature_buf)?;
        let signature = u32::from_le_bytes(signature_buf);
        if signature != 0x0000_0001 {
            error!("symbols subsection signature is {:#010X}, expected 0x00000001", signature);
            return Err(io::ErrorKind::InvalidData.into());
        }

        let mut symbols = Vec::new();
        loop {
            // try reading a length byte
            let mut buf = [0u8];
            match reader.read(&mut buf) {
                Ok(0) => {
                    // end of data, no more entries
                    break;
                },
                Ok(1) => {
                    // that works, keep going
                },
                Ok(n) => {
                    unreachable!("read() read {} bytes even though the buffer only has space for {}?!", n, buf.len());
                },
                Err(e) => return Err(e),
            }

            // read another length byte and fail if that doesn't work
            let mut buf2 = [0u8];
            reader.read_exact(&mut buf2)?;

            // read as much data as the length indicates
            let length_u16 =
                (u16::from(buf[0]) << 0)
                | (u16::from(buf2[0]) << 8)
                ;
            let length = usize::from(length_u16);
            let mut data = vec![0u8; length+2];
            data[0] = buf[0];
            data[1] = buf2[0];
            reader.read_exact(&mut data[2..])?;
            let mut data_cursor = Cursor::new(&data);

            // parse it as a symbol
            let symbol = SymbolEntry::read(&mut data_cursor)?;
            symbols.push(symbol);
        }

        Ok(Self {
            signature,
            symbols,
        })
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct SourceLineModuleSubsection {
    pub source_file_count: u16,
    pub segment_count: u16,
    pub source_file_offsets: Vec<u32>, // [u32; source_file_count]
    pub segment_starts_ends: Vec<(u32, u32)>, // [(u32, u32); segment_count]
    pub segment_indices: Vec<u16>, // [u16; segment_count]
    pub padding: Option<u16>, // if segment_count % 2 == 1
    pub source_files: Vec<SourceLineFile>, // [SourceLineFile; source_file_count]
}
impl SourceLineModuleSubsection {
    #[instrument(skip_all)]
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, io::Error> {
        let mut header_buf = [0u8; 4];
        reader.read_exact(&mut header_buf)?;

        let source_file_count = u16::from_le_byte_slice(&header_buf[0..2]);
        let segment_count = u16::from_le_byte_slice(&header_buf[2..4]);

        let source_file_count_usize = usize::from(source_file_count);
        let segment_count_usize = usize::from(segment_count);

        let mut source_file_offsets_buf = vec![0u8; 4*source_file_count_usize];
        reader.read_exact(&mut source_file_offsets_buf)?;
        let source_file_offsets: Vec<u32> = source_file_offsets_buf
            .chunks(4)
            .map(|chunk| u32::from_le_byte_slice(chunk))
            .collect();

        let mut segment_starts_ends_buf = vec![0u8; 8*segment_count_usize];
        reader.read_exact(&mut segment_starts_ends_buf)?;
        let segment_starts_ends: Vec<(u32, u32)> = segment_starts_ends_buf
            .chunks(8)
            .map(|chunk| (
                u32::from_le_byte_slice(&chunk[0..4]),
                u32::from_le_byte_slice(&chunk[4..8]),
            ))
            .collect();

        let mut segment_indices_buf = vec![0u8; 2*segment_count_usize];
        reader.read_exact(&mut segment_indices_buf)?;
        let segment_indices: Vec<u16> = segment_indices_buf
            .chunks(2)
            .map(|chunk| u16::from_le_byte_slice(chunk))
            .collect();

        let padding = if segment_count % 2 == 0 {
            None
        } else {
            let mut padding_buf = [0u8; 2];
            reader.read_exact(&mut padding_buf)?;
            Some(u16::from_le_bytes(padding_buf))
        };

        let mut source_files = Vec::with_capacity(source_file_count_usize);
        for _ in 0..source_file_count {
            let source_file = SourceLineFile::read(reader)?;
            source_files.push(source_file);
        }

        Ok(Self {
            source_file_count,
            segment_count,
            source_file_offsets,
            segment_starts_ends,
            segment_indices,
            padding,
            source_files,
        })
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct SourceLineFile {
    pub segment_count: u16,
    pub padding: u16,
    pub source_line_offsets: Vec<u32>, // [u32; segment_count]
    pub segment_starts_ends: Vec<(u32, u32)>, // [(u32, u32); segment_count]
    pub name: DisplayBytesVec, // PascalString
    pub padding2: [Option<u8>; 3], // pad to the next full u32
    pub segments: Vec<SourceLineSegment>, // [SourceLineSegment; segment_count]
}
impl SourceLineFile {
    #[instrument(skip_all)]
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, io::Error> {
        let mut header_buf = [0u8; 4];
        reader.read_exact(&mut header_buf)?;

        let segment_count = u16::from_le_byte_slice(&header_buf[0..2]);
        let padding = u16::from_le_byte_slice(&header_buf[2..4]);

        let segment_count_usize = usize::from(segment_count);

        let mut source_line_offsets_buf = vec![0u8; 4*segment_count_usize];
        reader.read_exact(&mut source_line_offsets_buf)?;
        let source_line_offsets: Vec<u32> = source_line_offsets_buf
            .chunks(4)
            .map(|chunk| u32::from_le_byte_slice(chunk))
            .collect();

        let mut segment_starts_ends_buf = vec![0u8; 8*segment_count_usize];
        reader.read_exact(&mut segment_starts_ends_buf)?;
        let segment_starts_ends: Vec<(u32, u32)> = segment_starts_ends_buf
            .chunks(8)
            .map(|chunk| (
                u32::from_le_byte_slice(&chunk[0..4]),
                u32::from_le_byte_slice(&chunk[4..8]),
            ))
            .collect();

        let name_vec = read_pascal_byte_string(reader)?;
        let name = DisplayBytesVec::from(name_vec);

        let name_slice: &[u8] = name.as_ref();
        let padding2 = match (name_slice.len() + 1) % 4 {
            0 => [None, None, None],
            1 => {
                let mut padding2_buf = [0u8; 3];
                reader.read_exact(&mut padding2_buf)?;
                [Some(padding2_buf[0]), Some(padding2_buf[1]), Some(padding2_buf[2])]
            },
            2 => {
                let mut padding2_buf = [0u8; 2];
                reader.read_exact(&mut padding2_buf)?;
                [Some(padding2_buf[0]), Some(padding2_buf[1]), None]
            },
            3 => {
                let mut padding2_buf = [0u8; 1];
                reader.read_exact(&mut padding2_buf)?;
                [Some(padding2_buf[0]), None, None]
            },
            _ => unreachable!(),
        };

        let mut segments = Vec::with_capacity(segment_count_usize);
        for _ in 0..segment_count_usize {
            let segment = SourceLineSegment::read(reader)?;
            segments.push(segment);
        }

        Ok(Self {
            segment_count,
            padding,
            source_line_offsets,
            segment_starts_ends,
            name,
            padding2,
            segments,
        })
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct SourceLineSegment {
    pub segment_index: u16,
    pub line_pair_count: u16,
    pub line_offsets: Vec<u32>, // [u32; line_pair_count]
    pub line_numbers: Vec<u16>, // [u16; line_pair_count]
    pub padding: Option<u16>, // if line_pair_count % 2 == 1
}
impl SourceLineSegment {
    #[instrument(skip_all)]
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, io::Error> {
        let mut header_buf = [0u8; 4];
        reader.read_exact(&mut header_buf)?;

        let segment_index = u16::from_le_byte_slice(&header_buf[0..2]);
        let line_pair_count = u16::from_le_byte_slice(&header_buf[2..4]);
        let line_pair_count_usize = usize::from(line_pair_count);

        let mut line_offsets_buf = vec![0u8; 4*line_pair_count_usize];
        reader.read_exact(&mut line_offsets_buf)?;
        let line_offsets: Vec<u32> = line_offsets_buf
            .chunks(4)
            .map(|chunk| u32::from_le_byte_slice(chunk))
            .collect();

        let mut line_numbers_buf = vec![0u8; 2*line_pair_count_usize];
        reader.read_exact(&mut line_numbers_buf)?;
        let line_numbers: Vec<u16> = line_numbers_buf
            .chunks(2)
            .map(|chunk| u16::from_le_byte_slice(chunk))
            .collect();

        let padding = if line_pair_count % 2 == 0 {
            None
        } else {
            let mut padding_buf = [0u8; 2];
            reader.read_exact(&mut padding_buf)?;
            Some(u16::from_le_bytes(padding_buf))
        };

        Ok(Self {
            segment_index,
            line_pair_count,
            line_offsets,
            line_numbers,
            padding,
        })
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct LibrariesSubsection {
    pub libraries: Vec<DisplayBytesVec>,
}
impl LibrariesSubsection {
    #[instrument(skip_all)]
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, io::Error> {
        let mut libraries = Vec::new();

        // read Pascal strings until we run out of bytes
        loop {
            let mut length_buf = [0u8];
            let bytes_read = reader.read(&mut length_buf)?;
            match bytes_read {
                0 => {
                    // EOF, break out
                    break;
                },
                1 => {
                    // length byte read, keep going
                },
                other => unreachable!("unexpectedly read {} bytes into a buffer of 1", other),
            }

            let mut library_vec = vec![0u8; usize::from(length_buf[0])];
            reader.read_exact(&mut library_vec)?;
            libraries.push(DisplayBytesVec::from(library_vec));
        }

        Ok(Self {
            libraries,
        })
    }
}


#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct GlobalSymbolsSubsection {
    pub symbol_hash_function_index: u16,
    pub address_hash_function_index: u16,
    pub symbols_length: u32,
    pub symbol_hash_table_length: u32,
    pub address_hash_table_length: u32,
    pub symbols: Vec<SymbolEntry>, // until symbols_length bytes have been read
    pub symbol_hash_table: DisplayBytesVec, // [u8; symbol_hash_table_length]
    pub address_hash_table: DisplayBytesVec, // [u8; address_hash_table_length]
}
impl GlobalSymbolsSubsection {
    #[instrument(skip_all)]
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, io::Error> {
        let mut header_buf = [0u8; 16];
        reader.read_exact(&mut header_buf)?;

        let symbol_hash_function_index = u16::from_le_byte_slice(&header_buf[0..2]);
        let address_hash_function_index = u16::from_le_byte_slice(&header_buf[2..4]);
        let symbols_length = u32::from_le_byte_slice(&header_buf[4..8]);
        let symbol_hash_table_length = u32::from_le_byte_slice(&header_buf[8..12]);
        let address_hash_table_length = u32::from_le_byte_slice(&header_buf[12..16]);

        let mut symbol_bytes = vec![0u8; symbols_length.try_into().unwrap()];
        reader.read_exact(&mut symbol_bytes)?;
        let mut symbol_hash_table_vec = vec![0u8; symbol_hash_table_length.try_into().unwrap()];
        reader.read_exact(&mut symbol_hash_table_vec)?;
        let mut address_hash_table_vec = vec![0u8; address_hash_table_length.try_into().unwrap()];
        reader.read_exact(&mut address_hash_table_vec)?;

        let symbol_hash_table = DisplayBytesVec::from(symbol_hash_table_vec);
        let address_hash_table = DisplayBytesVec::from(address_hash_table_vec);

        let mut symbol_reader = Cursor::new(&symbol_bytes);

        let mut symbols = Vec::new();
        loop {
            // try reading a length byte
            let mut buf = [0u8];
            match symbol_reader.read(&mut buf) {
                Ok(0) => {
                    // end of data, no more entries
                    break;
                },
                Ok(1) => {
                    // that works, keep going
                },
                Ok(n) => {
                    unreachable!("read() read {} bytes even though the buffer only has space for {}?!", n, buf.len());
                },
                Err(e) => return Err(e),
            }

            // read another length byte and fail if that doesn't work
            let mut buf2 = [0u8];
            symbol_reader.read_exact(&mut buf2)?;

            // read as much data as the length indicates
            let length_u16 =
                (u16::from(buf[0]) << 0)
                | (u16::from(buf2[0]) << 8)
                ;
            let length = usize::from(length_u16);
            let mut data = vec![0u8; length+2];
            data[0] = buf[0];
            data[1] = buf2[0];
            symbol_reader.read_exact(&mut data[2..])?;
            let mut data_cursor = Cursor::new(&data);

            // parse it as a symbol
            let symbol = SymbolEntry::read(&mut data_cursor)?;
            symbols.push(symbol);
        }

        Ok(Self {
            symbol_hash_function_index,
            address_hash_function_index,
            symbols_length,
            symbol_hash_table_length,
            address_hash_table_length,
            symbols,
            symbol_hash_table,
            address_hash_table,
        })
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct GlobalTypesSubsection {
    pub flags: u32,
    pub type_count: u32,
    pub type_offsets: Vec<u32>, // [u32; type_count]
    pub type_leaves: Vec<TypeLeaf>, // [TypeLeaf; type_count], each starting at corresponding entry of type_offsets
}
impl GlobalTypesSubsection {
    #[instrument(skip_all)]
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, io::Error> {
        let mut header_buf = [0u8; 8];
        reader.read_exact(&mut header_buf)?;

        let flags = u32::from_le_byte_slice(&header_buf[0..4]);
        let type_count = u32::from_le_byte_slice(&header_buf[4..8]);
        let type_count_usize: usize = type_count.try_into().unwrap();

        let mut type_offsets_buf = vec![0u8; 4*type_count_usize];
        reader.read_exact(&mut type_offsets_buf)?;
        let type_offsets: Vec<u32> = type_offsets_buf
            .chunks(4)
            .map(|chunk| u32::from_le_byte_slice(chunk))
            .collect();

        // TODO: NB07+NB08: offsets are from beginning of subsection table
        // NB09: offsets are from first type
        let first_type_pos = reader.seek(SeekFrom::Current(0))?;

        let mut type_leaves = Vec::with_capacity(type_count_usize);
        for type_offset in &type_offsets {
            reader.seek(SeekFrom::Start(first_type_pos + u64::from(*type_offset)))?;
            let mut length_buf = [0u8; 2];
            reader.read_exact(&mut length_buf)?;
            let length_u16 = u16::from_le_bytes(length_buf);
            let length = usize::from(length_u16);

            let mut type_leaf_buf = vec![0u8; length];
            reader.read_exact(&mut type_leaf_buf)?;

            debug!("type leaf data: {}", DisplayBytesSlice::from(type_leaf_buf.as_slice()));
            let mut type_leaf_reader = Cursor::new(&type_leaf_buf);
            let type_leaf = TypeLeaf::read(&mut type_leaf_reader)?;
            type_leaves.push(type_leaf);
        }

        Ok(Self {
            flags,
            type_count,
            type_offsets,
            type_leaves,
        })
    }
}
