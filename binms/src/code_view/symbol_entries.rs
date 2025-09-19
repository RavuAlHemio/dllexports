//! Entries in symbol tables in the CodeView debugging format.


use std::io::{self, Cursor, Read, Seek};

use bitflags::bitflags;
use display_bytes::DisplayBytesVec;
use from_to_repr::from_to_other;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use tracing::error;

use crate::read_pascal_byte_string;
use crate::code_view::leaves::NumericLeaf;
use crate::int_from_byte_slice::IntFromByteSlice;


#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct SymbolEntry {
    pub length: u16, // length of kind + data!
    pub kind: SymbolEntryType, // u16,
    pub data: SymbolEntryData, // [u8; length - size_of(kind)] = [u8; length - 2]
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
        let mut data_cursor = Cursor::new(&data_vec);

        let data = SymbolEntryData::read(&mut data_cursor, kind)?;

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
    CodeViewReserved = 0x0008,
    ObjectName = 0x0009,
    EndArguments = 0x000A,
    MicrofocusCobolUserDefinedType = 0x000B,
    ManyRegisters = 0x000C,
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
    With16_32 = 0x0208,
    Label16_32 = 0x0209,
    ChangeExecutionModel16_32 = 0x020A,
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

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub enum SymbolEntryData {
    CompileFlags(CompileFlags),
    RegisterVariable(RegisterVariable),
    Constant(Constant),
    UserDefinedType(UserDefinedType),
    StartSearch(StartSearch),
    End, // no data
    Skip(Skip),
    CodeViewReserved(DisplayBytesVec),
    ObjectName(ObjectName),
    EndArguments, // no data
    MicrofocusCobolUserDefinedType(UserDefinedType),
    ManyRegisters(ManyRegisters),
    ReturnDescription(ReturnDescription),
    EntryThisPointer(EntryThisPointer),

    BpRelative16_16(BpRelative16<u16>),
    LocalData16_16(Data16<u16>),
    GlobalData16_16(Data16<u16>),
    PublicSymbol16_16(Data16<u16>),
    LocalProcedure16_16(ProcedureStart16<u16>),
    GlobalProcedure16_16(ProcedureStart16<u16>),
    Thunk16_16(Thunk16<u16>),
    Block16_16(BlockStart16<u16>),
    With16_16(BlockStart16<u16>),
    Label16_16(Label16<u16>),
    ChangeExecutionModel16_16(ChangeExecutionModel16<u16>),
    VirtualFunctionTablePath16_16(VirtualFunctionTablePath16<u16>),
    RegisterRelativeOffset16_16(RegisterRelativeOffset16<u16>),

    BpRelative16_32(BpRelative16<u32>),
    LocalData16_32(Data16<u32>),
    GlobalData16_32(Data16<u32>),
    PublicSymbol16_32(Data16<u32>),
    LocalProcedure16_32(ProcedureStart16<u32>),
    GlobalProcedure16_32(ProcedureStart16<u32>),
    Thunk16_32(Thunk16<u32>),
    Block16_32(BlockStart16<u32>),
    With16_32(BlockStart16<u32>),
    Label16_32(Label16<u32>),
    ChangeExecutionModel16_32(ChangeExecutionModel16<u32>),
    VirtualFunctionTablePath16_32(VirtualFunctionTablePath16<u32>),
    RegisterRelativeOffset16_32(RegisterRelativeOffset16<u32>),
    LocalThreadData16_32(ThreadData16_32),
    GlobalThreadData16_32(ThreadData16_32),

    LocalProcedureMips(ProcedureMips),
    GlobalProcedureMips(ProcedureMips),

    ProcedureReference(CodeViewPackReference),
    DataReference(CodeViewPackReference),
    PageAlignment(PageAlignment),

    Other(DisplayBytesVec),
}
impl SymbolEntryData {
    pub fn read<R: Read + Seek>(reader: &mut R, kind: SymbolEntryType) -> Result<Self, io::Error> {
        match kind {
            SymbolEntryType::CompileFlags => {
                let data = CompileFlags::read(reader)?;
                Ok(Self::CompileFlags(data))
            },
            SymbolEntryType::RegisterVariable => {
                let data = RegisterVariable::read(reader)?;
                Ok(Self::RegisterVariable(data))
            },
            SymbolEntryType::Constant => {
                let data = Constant::read(reader)?;
                Ok(Self::Constant(data))
            },
            SymbolEntryType::UserDefinedType => {
                let data = UserDefinedType::read(reader)?;
                Ok(Self::UserDefinedType(data))
            },
            SymbolEntryType::StartSearch => {
                let data = StartSearch::read(reader)?;
                Ok(Self::StartSearch(data))
            },
            SymbolEntryType::End => Ok(Self::End),
            SymbolEntryType::Skip => {
                let data = Skip::read(reader)?;
                Ok(Self::Skip(data))
            },
            SymbolEntryType::CodeViewReserved => {
                let mut buf_vec = Vec::new();
                reader.read_to_end(&mut buf_vec)?;
                let buf = DisplayBytesVec::from(buf_vec);
                Ok(Self::CodeViewReserved(buf))
            },
            SymbolEntryType::ObjectName => {
                let data = ObjectName::read(reader)?;
                Ok(Self::ObjectName(data))
            },
            SymbolEntryType::EndArguments => Ok(Self::EndArguments),
            SymbolEntryType::MicrofocusCobolUserDefinedType => {
                let data = UserDefinedType::read(reader)?;
                Ok(Self::MicrofocusCobolUserDefinedType(data))
            },
            SymbolEntryType::ManyRegisters => {
                let data = ManyRegisters::read(reader)?;
                Ok(Self::ManyRegisters(data))
            },
            SymbolEntryType::ReturnDescription => {
                let data = ReturnDescription::read(reader)?;
                Ok(Self::ReturnDescription(data))
            },
            SymbolEntryType::EntryThisPointer => {
                let data = EntryThisPointer::read(reader)?;
                Ok(Self::EntryThisPointer(data))
            },
            SymbolEntryType::BpRelative16_16 => {
                let data = BpRelative16::read(reader)?;
                Ok(Self::BpRelative16_16(data))
            },
            SymbolEntryType::LocalData16_16 => {
                let data = Data16::read(reader)?;
                Ok(Self::LocalData16_16(data))
            },
            SymbolEntryType::GlobalData16_16 => {
                let data = Data16::read(reader)?;
                Ok(Self::GlobalData16_16(data))
            },
            SymbolEntryType::PublicSymbol16_16 => {
                let data = Data16::read(reader)?;
                Ok(Self::PublicSymbol16_16(data))
            },
            SymbolEntryType::LocalProcedure16_16 => {
                let data = ProcedureStart16::read(reader)?;
                Ok(Self::LocalProcedure16_16(data))
            },
            SymbolEntryType::GlobalProcedure16_16 => {
                let data = ProcedureStart16::read(reader)?;
                Ok(Self::GlobalProcedure16_16(data))
            },
            SymbolEntryType::Thunk16_16 => {
                let data = Thunk16::read(reader)?;
                Ok(Self::Thunk16_16(data))
            },
            SymbolEntryType::Block16_16 => {
                let data = BlockStart16::read(reader)?;
                Ok(Self::Block16_16(data))
            },
            SymbolEntryType::With16_16 => {
                let data = BlockStart16::read(reader)?;
                Ok(Self::With16_16(data))
            },
            SymbolEntryType::Label16_16 => {
                let data = Label16::read(reader)?;
                Ok(Self::Label16_16(data))
            },
            SymbolEntryType::ChangeExecutionModel16_16 => {
                let data = ChangeExecutionModel16::read(reader)?;
                Ok(Self::ChangeExecutionModel16_16(data))
            },
            SymbolEntryType::VirtualFunctionTablePath16_16 => {
                let data = VirtualFunctionTablePath16::read(reader)?;
                Ok(Self::VirtualFunctionTablePath16_16(data))
            },
            SymbolEntryType::RegisterRelativeOffset16_16 => {
                let data = RegisterRelativeOffset16::read(reader)?;
                Ok(Self::RegisterRelativeOffset16_16(data))
            },
            SymbolEntryType::BpRelative16_32 => {
                let data = BpRelative16::read(reader)?;
                Ok(Self::BpRelative16_32(data))
            },
            SymbolEntryType::LocalData16_32 => {
                let data = Data16::read(reader)?;
                Ok(Self::LocalData16_32(data))
            },
            SymbolEntryType::GlobalData16_32 => {
                let data = Data16::read(reader)?;
                Ok(Self::GlobalData16_32(data))
            },
            SymbolEntryType::PublicSymbol16_32 => {
                let data = Data16::read(reader)?;
                Ok(Self::PublicSymbol16_32(data))
            },
            SymbolEntryType::LocalProcedure16_32 => {
                let data = ProcedureStart16::read(reader)?;
                Ok(Self::LocalProcedure16_32(data))
            },
            SymbolEntryType::GlobalProcedure16_32 => {
                let data = ProcedureStart16::read(reader)?;
                Ok(Self::GlobalProcedure16_32(data))
            },
            SymbolEntryType::Thunk16_32 => {
                let data = Thunk16::read(reader)?;
                Ok(Self::Thunk16_32(data))
            },
            SymbolEntryType::Block16_32 => {
                let data = BlockStart16::read(reader)?;
                Ok(Self::Block16_32(data))
            },
            SymbolEntryType::With16_32 => {
                let data = BlockStart16::read(reader)?;
                Ok(Self::With16_32(data))
            },
            SymbolEntryType::Label16_32 => {
                let data = Label16::read(reader)?;
                Ok(Self::Label16_32(data))
            },
            SymbolEntryType::ChangeExecutionModel16_32 => {
                let data = ChangeExecutionModel16::read(reader)?;
                Ok(Self::ChangeExecutionModel16_32(data))
            },
            SymbolEntryType::VirtualFunctionTablePath16_32 => {
                let data = VirtualFunctionTablePath16::read(reader)?;
                Ok(Self::VirtualFunctionTablePath16_32(data))
            },
            SymbolEntryType::RegisterRelativeOffset16_32 => {
                let data = RegisterRelativeOffset16::read(reader)?;
                Ok(Self::RegisterRelativeOffset16_32(data))
            },
            SymbolEntryType::LocalThreadData16_32 => {
                let data = ThreadData16_32::read(reader)?;
                Ok(Self::LocalThreadData16_32(data))
            },
            SymbolEntryType::GlobalThreadData16_32 => {
                let data = ThreadData16_32::read(reader)?;
                Ok(Self::GlobalThreadData16_32(data))
            },
            SymbolEntryType::LocalProcedureMips => {
                let data = ProcedureMips::read(reader)?;
                Ok(Self::LocalProcedureMips(data))
            },
            SymbolEntryType::GlobalProcedureMips => {
                let data = ProcedureMips::read(reader)?;
                Ok(Self::GlobalProcedureMips(data))
            },
            SymbolEntryType::ProcedureReference => {
                let data = CodeViewPackReference::read(reader)?;
                Ok(Self::ProcedureReference(data))
            },
            SymbolEntryType::DataReference => {
                let data = CodeViewPackReference::read(reader)?;
                Ok(Self::DataReference(data))
            },
            SymbolEntryType::PageAlignment => {
                let data = PageAlignment::read(reader)?;
                Ok(Self::PageAlignment(data))
            },
            SymbolEntryType::Other(_) => {
                let mut buf_vec = Vec::new();
                reader.read_to_end(&mut buf_vec)?;
                let buf = DisplayBytesVec::from(buf_vec);
                Ok(Self::Other(buf))
            },
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct CompileFlags {
    pub machine: Machine, // u8

    // begin bitfield of 3 bytes
    pub language: Language, // u8
    pub p_code_present: bool, // u1
    pub float_precision: FloatPrecision, // u2
    pub float_package: FloatPackage, // u2
    pub ambient_data: AmbientMemoryModel, // u3
    pub ambient_code: AmbientMemoryModel, // u3
    pub mode_32: bool, // u1
    pub reserved: u8, // u4
    // end bitfield

    pub version: DisplayBytesVec,
}
impl CompileFlags {
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, io::Error> {
        let mut buf = [0u8; 4];
        reader.read_exact(&mut buf)?;

        let machine_u8 = buf[0];
        let machine = Machine::from_base_type(machine_u8);

        // cl.exe allocates bits from LSB to MSB
        // rrrr 3ccc dddf fFFp llll llll
        let flags_u24: u32 =
            (u32::from(buf[1]) <<  0)
            | (u32::from(buf[2]) <<  8)
            | (u32::from(buf[3]) << 16)
        ;
        let language_u8 = u8::try_from((flags_u24 >> 0) & 0xFF).unwrap();
        let p_code_present = (flags_u24 & (1 << 8)) != 0;
        let float_precision_u8 = u8::try_from((flags_u24 >> 9) & 0b11).unwrap();
        let float_package_u8 = u8::try_from((flags_u24 >> 11) & 0b11).unwrap();
        let ambient_data_u8 = u8::try_from((flags_u24 >> 13) & 0b111).unwrap();
        let ambient_code_u8 = u8::try_from((flags_u24 >> 16) & 0b111).unwrap();
        let mode_32 = (flags_u24 & (1 << 19)) != 0;
        let reserved = u8::try_from((flags_u24 >> 20) & 0b1111).unwrap();

        let language = Language::from_base_type(language_u8);
        let float_precision = FloatPrecision::from_base_type(float_precision_u8);
        let float_package = FloatPackage::from_base_type(float_package_u8);
        let ambient_data = AmbientMemoryModel::from_base_type(ambient_data_u8);
        let ambient_code = AmbientMemoryModel::from_base_type(ambient_code_u8);

        let version_vec = read_pascal_byte_string(reader)?;
        let version = DisplayBytesVec::from(version_vec);

        Ok(Self {
            machine,
            language,
            p_code_present,
            float_precision,
            float_package,
            ambient_data,
            ambient_code,
            mode_32,
            reserved,
            version,
        })
    }
}

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
#[from_to_other(base_type = u8, derive_compare = "as_int")]
pub enum Machine {
    Intel8080 = 0x00,
    Intel8086 = 0x01,
    Intel80286 = 0x02,
    Intel80386 = 0x03,
    Intel80486 = 0x04,
    IntelPentium = 0x05,
    MipsR4000 = 0x10,
    MipsFuture1 = 0x11,
    MipsFuture2 = 0x12,
    Mc68000 = 0x20,
    Mc68010 = 0x21,
    Mc68020 = 0x22,
    Mc68030 = 0x23,
    Mc68040 = 0x24,
    DecAlpha = 0x30,
    Other(u8),
}

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
#[from_to_other(base_type = u8, derive_compare = "as_int")]
pub enum Language {
    C = 0,
    CPlusPlus = 1,
    Fortran = 2,
    Masm = 3,
    Pascal = 4,
    Basic = 5,
    Cobol = 6,
    Link = 7,
    CvtRes = 8,
    CvtPgd = 9,
    CSharp = 10,
    VisualBasic = 11,
    ILAsm = 12,
    Java = 13,
    JScript = 14,
    Msil = 15,
    Hlsl = 16,
    D = b'D',
    Other(u8),
}

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
#[from_to_other(base_type = u8, derive_compare = "as_int")] // technically u2
pub enum FloatPrecision {
    AnsiC = 1,
    Other(u8),
}

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
#[from_to_other(base_type = u8, derive_compare = "as_int")] // technically u2
pub enum FloatPackage {
    Hardware = 0,
    Emulator = 1,
    Altmath = 2,
    Other(u8),
}

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
#[from_to_other(base_type = u8, derive_compare = "as_int")] // technically u3
pub enum AmbientMemoryModel {
    Near = 0,
    Far = 1,
    // Wherever = 2,
    // You = 3,
    // Are = 4,
    Huge = 2,
    Other(u8),
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct RegisterVariable {
    pub value_type: u16,
    pub register: u16,
    pub name: DisplayBytesVec,
    pub tracking: DisplayBytesVec,
}
impl RegisterVariable {
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, io::Error> {
        let mut buf = [0u8; 4];
        reader.read_exact(&mut buf)?;

        let value_type = u16::from_le_byte_slice(&buf[0..2]);
        let register = u16::from_le_byte_slice(&buf[2..4]);

        let name_vec = read_pascal_byte_string(reader)?;
        let mut tracking_vec = Vec::new();
        reader.read_to_end(&mut tracking_vec)?;

        let name = DisplayBytesVec::from(name_vec);
        let tracking = DisplayBytesVec::from(tracking_vec);

        Ok(Self {
            value_type,
            register,
            name,
            tracking,
        })
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct Constant {
    pub value_type: u16,
    pub value: NumericLeaf,
    pub name: DisplayBytesVec,
}
impl Constant {
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, io::Error> {
        let mut buf = [0u8; 2];
        reader.read_exact(&mut buf)?;

        let value_type = u16::from_le_byte_slice(&buf[0..2]);

        let value = NumericLeaf::read(reader)?;
        let name_vec = read_pascal_byte_string(reader)?;
        let name = DisplayBytesVec::from(name_vec);

        Ok(Self {
            value_type,
            value,
            name,
        })
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct UserDefinedType {
    pub value_type: u16,
    pub name: DisplayBytesVec,
}
impl UserDefinedType {
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, io::Error> {
        let mut buf = [0u8; 2];
        reader.read_exact(&mut buf)?;

        let value_type = u16::from_le_byte_slice(&buf[0..2]);

        let name_vec = read_pascal_byte_string(reader)?;
        let name = DisplayBytesVec::from(name_vec);

        Ok(Self {
            value_type,
            name,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct StartSearch {
    pub symbol_offset: u32,
    pub segment: u16,
}
impl StartSearch {
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, io::Error> {
        let mut buf = [0u8; 6];
        reader.read_exact(&mut buf)?;

        let symbol_offset = u32::from_le_byte_slice(&buf[0..4]);
        let segment = u16::from_le_byte_slice(&buf[4..6]);

        Ok(Self {
            symbol_offset,
            segment,
        })
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct Skip {
    pub skip_data: DisplayBytesVec,
}
impl Skip {
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, io::Error> {
        let mut skip_data_vec = Vec::new();
        reader.read_to_end(&mut skip_data_vec)?;
        let skip_data = DisplayBytesVec::from(skip_data_vec);

        Ok(Self {
            skip_data,
        })
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct ObjectName {
    pub signature: u32,
    pub name: DisplayBytesVec,
}
impl ObjectName {
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, io::Error> {
        let mut buf = [0u8; 4];
        reader.read_exact(&mut buf)?;

        let signature = u32::from_le_byte_slice(&buf[0..4]);

        let name_vec = read_pascal_byte_string(reader)?;
        let name = DisplayBytesVec::from(name_vec);

        Ok(Self {
            signature,
            name,
        })
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct ManyRegisters {
    pub value_type: u16,
    // count: u8,
    pub registers: Vec<u8>, // [u8; count]
    pub name: DisplayBytesVec,
}
impl ManyRegisters {
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, io::Error> {
        let mut buf = [0u8; 3];
        reader.read_exact(&mut buf)?;

        let value_type = u16::from_le_byte_slice(&buf[0..2]);

        let count = usize::from(buf[2]);
        let mut registers = vec![0u8; count];
        reader.read_exact(&mut registers)?;

        let name_vec = read_pascal_byte_string(reader)?;
        let name = DisplayBytesVec::from(name_vec);

        Ok(Self {
            value_type,
            registers,
            name,
        })
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct ReturnDescription {
    pub function_flags: FunctionFlags, // u16
    pub return_style: ReturnStyle, // u8
    pub data: DisplayBytesVec,
}
impl ReturnDescription {
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, io::Error> {
        let mut buf = [0u8; 3];
        reader.read_exact(&mut buf)?;

        let function_flags_u16 = u16::from_le_byte_slice(&buf[0..2]);
        let function_flags = FunctionFlags::from_bits_retain(function_flags_u16);

        let return_style_u8 = buf[2];
        let return_style = ReturnStyle::from_base_type(return_style_u8);

        let mut data_vec = Vec::new();
        reader.read_to_end(&mut data_vec)?;
        let data = DisplayBytesVec::from(data_vec);

        Ok(Self {
            function_flags,
            return_style,
            data,
        })
    }
}

bitflags! {
    #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    #[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
    pub struct FunctionFlags : u16 {
        const C_STYLE = 0x0000_0001;
        const CALLEE_STACK_CLEANUP = 0x0000_0002;
    }
}

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
#[from_to_other(base_type = u8, derive_compare = "as_int")]
pub enum ReturnStyle {
    VoidReturn = 0x00,
    RegistersInData = 0x01,
    IndirectCallerAllocatedNear = 0x02,
    IndirectCallerAllocatedFar = 0x03,
    IndirectCalleeAllocatedNear = 0x04,
    IndirectCalleeAllocatedFar = 0x05,
    Other(u8),
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct EntryThisPointer {
    pub symbol: DisplayBytesVec,
}
impl EntryThisPointer {
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, io::Error> {
        let symbol_vec = read_pascal_byte_string(reader)?;
        let symbol = DisplayBytesVec::from(symbol_vec);

        Ok(Self {
            symbol,
        })
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct BpRelative16<T: IntFromByteSlice> {
    pub offset: T,
    pub value_type: u16,
    pub name: DisplayBytesVec,
}
impl<T: IntFromByteSlice> BpRelative16<T> {
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, io::Error> {
        let mut offset_buf = vec![0u8; T::size()];
        reader.read_exact(&mut offset_buf)?;
        let offset = T::from_le_byte_slice(&offset_buf);

        let mut fixed_buf = [0u8; 2];
        reader.read_exact(&mut fixed_buf)?;
        let value_type = u16::from_le_byte_slice(&fixed_buf[0..2]);

        let name_vec = read_pascal_byte_string(reader)?;
        let name = DisplayBytesVec::from(name_vec);

        Ok(Self {
            offset,
            value_type,
            name,
        })
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct Data16<T: IntFromByteSlice> {
    pub offset: T,
    pub segment: u16,
    pub value_type: u16,
    pub name: DisplayBytesVec,
}
impl<T: IntFromByteSlice> Data16<T> {
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, io::Error> {
        let mut offset_buf = vec![0u8; T::size()];
        reader.read_exact(&mut offset_buf)?;
        let offset = T::from_le_byte_slice(&offset_buf);

        let mut fixed_buf = [0u8; 4];
        reader.read_exact(&mut fixed_buf)?;
        let segment = u16::from_le_byte_slice(&fixed_buf[0..2]);
        let value_type = u16::from_le_byte_slice(&fixed_buf[2..4]);

        let name_vec = read_pascal_byte_string(reader)?;
        let name = DisplayBytesVec::from(name_vec);

        Ok(Self {
            offset,
            segment,
            value_type,
            name,
        })
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct ProcedureStart16<T: IntFromByteSlice> {
    pub parent_scope: u32,
    pub scope_end: u32,
    pub next_scope: u32,
    pub proc_length: T,
    pub debug_start: T,
    pub debug_end: T,
    pub offset: T,
    pub segment: u16,
    pub procedure_type: u16,
    pub flags: ProcedureFlags, // u8
    pub name: DisplayBytesVec,
}
impl<T: IntFromByteSlice> ProcedureStart16<T> {
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, io::Error> {
        let mut pre_fixed_buf = [0u8; 12];
        reader.read_exact(&mut pre_fixed_buf)?;
        let parent_scope = u32::from_le_byte_slice(&pre_fixed_buf[0..4]);
        let scope_end = u32::from_le_byte_slice(&pre_fixed_buf[4..8]);
        let next_scope = u32::from_le_byte_slice(&pre_fixed_buf[8..12]);

        let t_size = T::size();
        let mut lengths_buf = vec![0u8; 4*t_size];
        reader.read_exact(&mut lengths_buf)?;
        let proc_length = T::from_le_byte_slice(&lengths_buf[0*t_size..1*t_size]);
        let debug_start = T::from_le_byte_slice(&lengths_buf[1*t_size..2*t_size]);
        let debug_end = T::from_le_byte_slice(&lengths_buf[2*t_size..3*t_size]);
        let offset = T::from_le_byte_slice(&lengths_buf[3*t_size..4*t_size]);

        let mut post_fixed_buf = [0u8; 5];
        reader.read_exact(&mut post_fixed_buf)?;
        let segment = u16::from_le_byte_slice(&post_fixed_buf[0..2]);
        let procedure_type = u16::from_le_byte_slice(&post_fixed_buf[2..4]);
        let flags_u8 = post_fixed_buf[4];
        let flags = ProcedureFlags::from_bits_retain(flags_u8);

        let name_vec = read_pascal_byte_string(reader)?;
        let name = DisplayBytesVec::from(name_vec);

        Ok(Self {
            parent_scope,
            scope_end,
            next_scope,
            proc_length,
            debug_start,
            debug_end,
            offset,
            segment,
            procedure_type,
            flags,
            name,
        })
    }
}

bitflags! {
    #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    #[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
    pub struct ProcedureFlags : u8 {
        const FRAME_POINTER_OMITTED = 0x01;
        const IS_INTERRUPT_ROUTINE = 0x02;
        const PERFORMS_FAR_RETURN = 0x04;
        const NEVER_RETURNS = 0x08;
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct Thunk16<T: IntFromByteSlice> {
    pub parent_scope: u32,
    pub scope_end: u32,
    pub next_scope: u32, // documentation says T, but it's always u32?
    pub offset: T,
    pub segment: u16,
    pub length: u16,
    pub thunk_type: ThunkType, // u8
    pub name: DisplayBytesVec,
    pub variant: DisplayBytesVec,
}
impl<T: IntFromByteSlice> Thunk16<T> {
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, io::Error> {
        let mut pre_fixed_buf = [0u8; 12];
        reader.read_exact(&mut pre_fixed_buf)?;
        let parent_scope = u32::from_le_byte_slice(&pre_fixed_buf[0..4]);
        let scope_end = u32::from_le_byte_slice(&pre_fixed_buf[4..8]);
        let next_scope = u32::from_le_byte_slice(&pre_fixed_buf[8..12]);

        let t_size = T::size();
        let mut lengths_buf = vec![0u8; t_size];
        reader.read_exact(&mut lengths_buf)?;
        let offset = T::from_le_byte_slice(&lengths_buf[0*t_size..1*t_size]);

        let mut post_fixed_buf = [0u8; 5];
        reader.read_exact(&mut post_fixed_buf)?;
        let segment = u16::from_le_byte_slice(&post_fixed_buf[0..2]);
        let length = u16::from_le_byte_slice(&post_fixed_buf[2..4]);
        let thunk_type_u8 = post_fixed_buf[4];
        let thunk_type = ThunkType::from_base_type(thunk_type_u8);

        let name_vec = read_pascal_byte_string(reader)?;
        let name = DisplayBytesVec::from(name_vec);

        let mut variant_vec = Vec::new();
        reader.read_to_end(&mut variant_vec)?;
        let variant = DisplayBytesVec::from(variant_vec);

        Ok(Self {
            parent_scope,
            scope_end,
            next_scope,
            offset,
            segment,
            length,
            thunk_type,
            name,
            variant,
        })
    }
}

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
#[from_to_other(base_type = u8, derive_compare = "as_int")]
pub enum ThunkType {
    NoType = 0x00,
    Adjustor = 0x01,
    VirtualCall = 0x02,
    PCode = 0x03,
    Other(u8),
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct BlockStart16<T: IntFromByteSlice> {
    pub parent_scope: u32,
    pub scope_end: u32,
    pub length: T,
    pub offset: T,
    pub segment: u16,
    pub name: DisplayBytesVec,
}
impl<T: IntFromByteSlice> BlockStart16<T> {
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, io::Error> {
        let mut pre_fixed_buf = [0u8; 8];
        reader.read_exact(&mut pre_fixed_buf)?;
        let parent_scope = u32::from_le_byte_slice(&pre_fixed_buf[0..4]);
        let scope_end = u32::from_le_byte_slice(&pre_fixed_buf[4..8]);

        let t_size = T::size();
        let mut lengths_buf = vec![0u8; 2*t_size];
        reader.read_exact(&mut lengths_buf)?;
        let length = T::from_le_byte_slice(&lengths_buf[0*t_size..1*t_size]);
        let offset = T::from_le_byte_slice(&lengths_buf[1*t_size..2*t_size]);

        let mut post_fixed_buf = [0u8; 2];
        reader.read_exact(&mut post_fixed_buf)?;
        let segment = u16::from_le_byte_slice(&post_fixed_buf[0..2]);

        let name_vec = read_pascal_byte_string(reader)?;
        let name = DisplayBytesVec::from(name_vec);

        Ok(Self {
            parent_scope,
            scope_end,
            length,
            offset,
            segment,
            name,
        })
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct Label16<T: IntFromByteSlice> {
    pub offset: T,
    pub segment: u16,
    pub flags: ProcedureFlags, // u8
    pub name: DisplayBytesVec,
}
impl<T: IntFromByteSlice> Label16<T> {
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, io::Error> {
        let t_size = T::size();
        let mut lengths_buf = vec![0u8; 1*t_size];
        reader.read_exact(&mut lengths_buf)?;
        let offset = T::from_le_byte_slice(&lengths_buf[0*t_size..1*t_size]);

        let mut post_fixed_buf = [0u8; 3];
        reader.read_exact(&mut post_fixed_buf)?;
        let segment = u16::from_le_byte_slice(&post_fixed_buf[0..2]);
        let flags_u8 = post_fixed_buf[2];
        let flags = ProcedureFlags::from_bits_retain(flags_u8);

        let name_vec = read_pascal_byte_string(reader)?;
        let name = DisplayBytesVec::from(name_vec);

        Ok(Self {
            offset,
            segment,
            flags,
            name,
        })
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct ChangeExecutionModel16<T: IntFromByteSlice> {
    pub offset: T,
    pub segment: u16,
    pub new_execution_model: ExecutionModel, // u16
    pub variant: DisplayBytesVec,
}
impl<T: IntFromByteSlice> ChangeExecutionModel16<T> {
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, io::Error> {
        let t_size = T::size();
        let mut lengths_buf = vec![0u8; 1*t_size];
        reader.read_exact(&mut lengths_buf)?;
        let offset = T::from_le_byte_slice(&lengths_buf[0*t_size..1*t_size]);

        let mut post_fixed_buf = [0u8; 4];
        reader.read_exact(&mut post_fixed_buf)?;
        let segment = u16::from_le_byte_slice(&post_fixed_buf[0..2]);
        let new_execution_model_u16 = u16::from_le_byte_slice(&post_fixed_buf[2..4]);
        let new_execution_model = ExecutionModel::from_base_type(new_execution_model_u16);

        let mut variant_vec = Vec::new();
        reader.read_to_end(&mut variant_vec)?;
        let variant = DisplayBytesVec::from(variant_vec);

        Ok(Self {
            offset,
            segment,
            new_execution_model,
            variant,
        })
    }
}

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
#[from_to_other(base_type = u16, derive_compare = "as_int")]
pub enum ExecutionModel {
    NotExecutable = 0x00,
    CompilerGeneratedJumpTable = 0x01,
    DataPadding = 0x02,
    NativeModel = 0x20,
    MicrofocusCobol = 0x21,
    CodePadding = 0x22,
    Code = 0x23,
    PCode = 0x40,
    Other(u16),
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct VirtualFunctionTablePath16<T: IntFromByteSlice> {
    pub offset: T,
    pub segment: u16,
    pub root: u16,
    pub path: u16,
}
impl<T: IntFromByteSlice> VirtualFunctionTablePath16<T> {
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, io::Error> {
        let t_size = T::size();
        let mut lengths_buf = vec![0u8; 1*t_size];
        reader.read_exact(&mut lengths_buf)?;
        let offset = T::from_le_byte_slice(&lengths_buf[0*t_size..1*t_size]);

        let mut post_fixed_buf = [0u8; 6];
        reader.read_exact(&mut post_fixed_buf)?;
        let segment = u16::from_le_byte_slice(&post_fixed_buf[0..2]);
        let root = u16::from_le_byte_slice(&post_fixed_buf[2..4]);
        let path = u16::from_le_byte_slice(&post_fixed_buf[4..6]);

        Ok(Self {
            offset,
            segment,
            root,
            path,
        })
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct RegisterRelativeOffset16<T: IntFromByteSlice> {
    pub offset: T,
    pub register: u16,
    pub value_type: u16,
    pub name: DisplayBytesVec,
}
impl<T: IntFromByteSlice> RegisterRelativeOffset16<T> {
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, io::Error> {
        let t_size = T::size();
        let mut lengths_buf = vec![0u8; 1*t_size];
        reader.read_exact(&mut lengths_buf)?;
        let offset = T::from_le_byte_slice(&lengths_buf[0*t_size..1*t_size]);

        let mut post_fixed_buf = [0u8; 4];
        reader.read_exact(&mut post_fixed_buf)?;
        let register = u16::from_le_byte_slice(&post_fixed_buf[0..2]);
        let value_type = u16::from_le_byte_slice(&post_fixed_buf[2..4]);

        let name_vec = read_pascal_byte_string(reader)?;
        let name = DisplayBytesVec::from(name_vec);

        Ok(Self {
            offset,
            register,
            value_type,
            name,
        })
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct ThreadData16_32 {
    pub offset: u32,
    pub segment: u16,
    pub value_type: u16,
    pub name: DisplayBytesVec,
}
impl ThreadData16_32 {
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, io::Error> {
        let mut buf = [0u8; 8];
        reader.read_exact(&mut buf)?;

        let offset = u32::from_le_byte_slice(&buf[0..4]);
        let segment = u16::from_le_byte_slice(&buf[4..6]);
        let value_type = u16::from_le_byte_slice(&buf[6..8]);

        let name_vec = read_pascal_byte_string(reader)?;
        let name = DisplayBytesVec::from(name_vec);

        Ok(Self {
            offset,
            segment,
            value_type,
            name,
        })
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct ProcedureMips {
    pub parent_scope: u32,
    pub scope_end: u32,
    pub next_scope: u32,
    pub length: u32,
    pub debug_start: u32,
    pub debug_end: u32,
    pub int_save_mask: u32,
    pub float_save_mask: u32,
    pub int_save_offset: u32,
    pub float_save_offset: u32,
    pub offset: u32,
    pub segment: u16,
    pub procedure_type: u16,
    pub return_register: u8,
    pub frame_pointer_register: u8,
    pub name: DisplayBytesVec,
}
impl ProcedureMips {
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, io::Error> {
        let mut buf = [0u8; 50];
        reader.read_exact(&mut buf)?;

        let parent_scope = u32::from_le_byte_slice(&buf[0..4]);
        let scope_end = u32::from_le_byte_slice(&buf[4..8]);
        let next_scope = u32::from_le_byte_slice(&buf[8..12]);
        let length = u32::from_le_byte_slice(&buf[12..16]);
        let debug_start = u32::from_le_byte_slice(&buf[16..20]);
        let debug_end = u32::from_le_byte_slice(&buf[20..24]);
        let int_save_mask = u32::from_le_byte_slice(&buf[24..28]);
        let float_save_mask = u32::from_le_byte_slice(&buf[28..32]);
        let int_save_offset = u32::from_le_byte_slice(&buf[32..36]);
        let float_save_offset = u32::from_le_byte_slice(&buf[36..40]);
        let offset = u32::from_le_byte_slice(&buf[40..44]);
        let segment = u16::from_le_byte_slice(&buf[44..46]);
        let procedure_type = u16::from_le_byte_slice(&buf[46..48]);
        let return_register = buf[48];
        let frame_pointer_register = buf[49];

        let name_vec = read_pascal_byte_string(reader)?;
        let name = DisplayBytesVec::from(name_vec);

        Ok(Self {
            parent_scope,
            scope_end,
            next_scope,
            length,
            debug_start,
            debug_end,
            int_save_mask,
            float_save_mask,
            int_save_offset,
            float_save_offset,
            offset,
            segment,
            procedure_type,
            return_register,
            frame_pointer_register,
            name,
        })
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct CodeViewPackReference {
    pub checksum: u32,
    pub offset: u32,
    pub module: u16,
}
impl CodeViewPackReference {
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, io::Error> {
        let mut buf = [0u8; 10];
        reader.read_exact(&mut buf)?;

        let checksum = u32::from_le_byte_slice(&buf[0..4]);
        let offset = u32::from_le_byte_slice(&buf[4..8]);
        let module = u16::from_le_byte_slice(&buf[8..10]);

        Ok(Self {
            checksum,
            offset,
            module,
        })
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct PageAlignment {
    pub padding: DisplayBytesVec,
}
impl PageAlignment {
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, io::Error> {
        let mut padding_vec = Vec::new();
        reader.read_to_end(&mut padding_vec)?;
        let padding = DisplayBytesVec::from(padding_vec);

        Ok(Self {
            padding,
        })
    }
}
