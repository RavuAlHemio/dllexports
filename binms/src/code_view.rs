//! Debug information in the CodeView format.
//!
//! Most of the structures have been derived from the contents of
//! https://www.os2site.com/sw/dev/openwatcom/docs/codeview.pdf.


use std::io::{self, Cursor, Read, Seek, SeekFrom};

use display_bytes::DisplayBytesVec;
use from_to_repr::from_to_other;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use tracing::{debug, error};

use crate::read_pascal_byte_string;


#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct DebugInfo {
    pub signature: [u8; 4],
    pub directory_offset: u32,
    pub subsection_directory_header: SubsectionDirectoryHeader,
    pub subsection_directory_entries: Vec<SubsectionDirectoryEntry>, // [SubsectionDirectoryEntry; subsection_directory_header.entry_count]
}
impl DebugInfo {
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
                SubsectionType::SourceLineModule => todo!(),
                SubsectionType::Libraries => todo!(),
                SubsectionType::GlobalSymbols => todo!(),
                SubsectionType::GlobalPublicSymbols => todo!(),
                SubsectionType::GlobalTypes => todo!(),
                SubsectionType::MakePCode => todo!(),
                SubsectionType::SegmentMap => todo!(),
                SubsectionType::SegmentName => todo!(),
                SubsectionType::PreCompile => todo!(),
                SubsectionType::FileIndex => todo!(),
                SubsectionType::StaticSymbols => todo!(),
                SubsectionType::Other(_) => todo!(),
                */
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
    SourceLineModule(SourceLineModuleSubsection),
    Libraries(LibrariesSubsection),
    GlobalSymbols(GlobalSymbolsSubsection),
    GlobalPublicSymbols(GlobalPublicSymbolsSubsection),
    GlobalTypes(GlobalTypesSubsection),
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
pub struct SymbolEntry {
    pub length: u16, // length of kind + data!
    pub kind: SymbolEntryType, // u16,
    pub data: DisplayBytesVec, // [u8; length - size_of(kind)] = [u8; length - 2]
}
impl SymbolEntry {
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, io::Error> {
        let mut header_buf = [0u8; 4];
        reader.read_exact(&mut header_buf)?;

        let length = u16::from_le_bytes(header_buf[0..2].try_into().unwrap());
        if length < 2 {
            error!("symbol entry has length {} which leaves no space for \"kind\" field (u16)", length);
            return Err(io::ErrorKind::InvalidData.into());
        }
        let kind_u16 = u16::from_le_bytes(header_buf[2..4].try_into().unwrap());
        let kind = SymbolEntryType::from_base_type(kind_u16);

        let data_length: usize = (length - 2).try_into().unwrap();
        let mut data_vec = vec![0u8; data_length];
        reader.read_exact(&mut data_vec)?;
        let data = DisplayBytesVec::from(data_vec);

        Ok(Self {
            length,
            kind,
            data,
        })
    }
}

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
#[from_to_other(base_type = u16, derive_compare = "as_int")]
pub enum SymbolEntryType {
    CompileFlags = 0x0001,
    RegisterVariable = 0x0002,
    Constant = 0x0003,
    UserDefinedType = 0x0004,
    StartSearch = 0x0005,
    End = 0x0006,
    Skip = 0x0007,
    CodeViewReserve = 0x0008,
    ObjectName = 0x0009,
    EndArguments = 0x000A,
    MicrofocusCobolUserDefinedType = 0x000B,
    ManyRegister = 0x000C,
    ReturnDescription = 0x000D,
    EntryThisPointer = 0x000E,

    BpRelative16_16 = 0x0100,
    LocalData16_16 = 0x0101,
    GlobalData16_16 = 0x0102,
    PublicSymbol16_16 = 0x0103,
    LocalProcedure16_16 = 0x0104,
    GlobalProcedure16_16 = 0x0105,
    Thunk16_16 = 0x0106,
    Block16_16 = 0x0107,
    With16_16 = 0x0108,
    Label16_16 = 0x0109,
    ChangeExecutionModel16_16 = 0x010A,
    VirtualFunctionTablePath16_16 = 0x010B,
    RegisterRelativeOffset16_16 = 0x010C,

    BpRelative16_32 = 0x0200,
    LocalData16_32 = 0x0201,
    GlobalData16_32 = 0x0202,
    PublicSymbol16_32 = 0x0203,
    LocalProcedure16_32 = 0x0204,
    GlobalProcedure16_32 = 0x0205,
    Thunk16_32 = 0x0206,
    Block16_32 = 0x0207,
    VirtualFunctionTablePath16_32 = 0x020B,
    RegisterRelativeOffset16_32 = 0x020C,
    LocalThreadData16_32 = 0x020D,
    GlobalThreadData16_32 = 0x020E,

    LocalProcedureMips = 0x0300,
    GlobalProcedureMips = 0x0301,

    ProcedureReference = 0x0400,
    DataReference = 0x0401,
    PageAlignment = 0x0402,

    Other(u16),
}
