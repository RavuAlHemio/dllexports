//! Numeric and type leaves in the CodeView debugging format.


use std::io::{self, Read, Seek, SeekFrom};

use bitflags::bitflags;
use display_bytes::DisplayBytesVec;
use from_to_repr::{from_to_other, FromToRepr};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use tracing::{debug, error, instrument};

use crate::bit_pattern_float::{
    BitPatternF32, BitPatternF64, ComplexBitPatternF32, ComplexBitPatternF64,
};
use crate::code_view::SymbolEntry;
use crate::int_from_byte_slice::IntFromByteSlice;
use crate::read_pascal_byte_string;


#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub enum NumericLeaf {
    Immediate(u16), // < 0x8000
    SignedChar(i8), // 0x8000
    SignedShort(i16), // 0x8001
    UnsignedShort(u16), // 0x8002
    SignedLong(i32), // 0x8003
    UnsignedLong(u32), // 0x8004
    Float32(BitPatternF32), // 0x8005
    Float64(BitPatternF64), // 0x8006
    Float80([u8; 10]), // 0x8007
    Float128([u8; 16]), // 0x8008
    SignedQuadWord(i64), // 0x8009
    UnsignedQuadWord(u64), // 0x800A
    Float48([u8; 6]), // 0x800B
    Complex32(ComplexBitPatternF32), // 0x800C
    Complex64(ComplexBitPatternF64), // 0x800D
    Complex80(Complex<10>), // 0x800E
    Complex128(Complex<16>), // 0x800F
    String(DisplayBytesVec), // 0x8010
}
impl NumericLeaf {
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, io::Error> {
        let mut buf = [0u8; 2];
        reader.read_exact(&mut buf)?;
        let value = u16::from_le_bytes(buf);

        match value {
            0x0000..=0x7FFF => Ok(Self::Immediate(value)),
            0x8000 => {
                let mut value_buf = [0u8];
                reader.read_exact(&mut value_buf)?;
                // no byte order on 8-bit integers, but be consistent
                Ok(Self::SignedChar(i8::from_le_bytes(value_buf)))
            },
            0x8001 => {
                let mut value_buf = [0u8; 2];
                reader.read_exact(&mut value_buf)?;
                Ok(Self::SignedShort(i16::from_le_bytes(value_buf)))
            },
            0x8002 => {
                let mut value_buf = [0u8; 2];
                reader.read_exact(&mut value_buf)?;
                Ok(Self::UnsignedShort(u16::from_le_bytes(value_buf)))
            },
            0x8003 => {
                let mut value_buf = [0u8; 4];
                reader.read_exact(&mut value_buf)?;
                Ok(Self::SignedLong(i32::from_le_bytes(value_buf)))
            },
            0x8004 => {
                let mut value_buf = [0u8; 4];
                reader.read_exact(&mut value_buf)?;
                Ok(Self::UnsignedLong(u32::from_le_bytes(value_buf)))
            },
            0x8005 => {
                let mut value_buf = [0u8; 4];
                reader.read_exact(&mut value_buf)?;
                Ok(Self::Float32(BitPatternF32::from(f32::from_le_bytes(value_buf))))
            },
            0x8006 => {
                let mut value_buf = [0u8; 8];
                reader.read_exact(&mut value_buf)?;
                Ok(Self::Float64(BitPatternF64::from(f64::from_le_bytes(value_buf))))
            },
            0x8007 => {
                let mut value_buf = [0u8; 10];
                reader.read_exact(&mut value_buf)?;
                Ok(Self::Float80(value_buf))
            },
            0x8008 => {
                let mut value_buf = [0u8; 16];
                reader.read_exact(&mut value_buf)?;
                Ok(Self::Float128(value_buf))
            },
            0x8009 => {
                let mut value_buf = [0u8; 8];
                reader.read_exact(&mut value_buf)?;
                Ok(Self::SignedQuadWord(i64::from_le_bytes(value_buf)))
            },
            0x800A => {
                let mut value_buf = [0u8; 8];
                reader.read_exact(&mut value_buf)?;
                Ok(Self::UnsignedQuadWord(u64::from_le_bytes(value_buf)))
            },
            0x800B => {
                let mut value_buf = [0u8; 6];
                reader.read_exact(&mut value_buf)?;
                Ok(Self::Float48(value_buf))
            },
            0x800C => {
                let mut value_buf = [0u8; 8];
                reader.read_exact(&mut value_buf)?;
                let real = BitPatternF32::from(f32::from_le_bytes(value_buf[0..4].try_into().unwrap()));
                let imag = BitPatternF32::from(f32::from_le_bytes(value_buf[4..8].try_into().unwrap()));
                Ok(Self::Complex32(ComplexBitPatternF32 { real, imag }))
            },
            0x800D => {
                let mut value_buf = [0u8; 16];
                reader.read_exact(&mut value_buf)?;
                let real = BitPatternF64::from(f64::from_le_bytes(value_buf[0..8].try_into().unwrap()));
                let imag = BitPatternF64::from(f64::from_le_bytes(value_buf[8..16].try_into().unwrap()));
                Ok(Self::Complex64(ComplexBitPatternF64 { real, imag }))
            },
            0x800E => {
                let mut value_buf = [0u8; 20];
                reader.read_exact(&mut value_buf)?;
                let real = value_buf[0..10].try_into().unwrap();
                let imag = value_buf[10..20].try_into().unwrap();
                Ok(Self::Complex80(Complex { real, imag }))
            },
            0x800F => {
                let mut value_buf = [0u8; 32];
                reader.read_exact(&mut value_buf)?;
                let real = value_buf[0..16].try_into().unwrap();
                let imag = value_buf[16..32].try_into().unwrap();
                Ok(Self::Complex128(Complex { real, imag }))
            },
            0x8010 => {
                let mut length_buf = [0u8; 2];
                reader.read_exact(&mut length_buf)?;
                let length = usize::from(u16::from_le_bytes(length_buf));
                let mut string_buf = vec![0u8; length];
                reader.read_exact(&mut string_buf)?;
                Ok(Self::String(DisplayBytesVec::from(string_buf)))
            },
            other => {
                error!("unknown numeric leaf type {:#06X}", other);
                Err(io::ErrorKind::InvalidData.into())
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct Complex<const BYTES: usize> {
    #[serde(with = "serde_complex")]
    pub real: [u8; BYTES],

    #[serde(with = "serde_complex")]
    pub imag: [u8; BYTES],
}
#[cfg(feature = "serde")]
mod serde_complex {
    pub fn serialize<S: serde::Serializer, const BYTES: usize>(bytes: &[u8; BYTES], serializer: S) -> Result<S::Ok, S::Error> {
        use serde::Serialize as _;

        let bytes_vec = bytes.to_vec();
        bytes_vec.serialize(serializer)
    }

    pub fn deserialize<'d, D: serde::Deserializer<'d>, const BYTES: usize>(deserializer: D) -> Result<[u8; BYTES], D::Error> {
        use serde::Deserialize as _;
        use serde::de::Error as _;

        let bytes_vec: Vec<u8> = Vec::deserialize(deserializer)?;
        if bytes_vec.len() == BYTES {
            let mut ret = [0u8; BYTES];
            ret.copy_from_slice(&bytes_vec);
            Ok(ret)
        } else {
            Err(D::Error::custom("wrong length"))
        }
    }
}

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
#[from_to_other(base_type = u16, derive_compare = "as_int")]
pub enum TypeLeafIndex {
    // 0x0000..=0x01FF: type records that can be referenced from symbols
    Modifier = 0x0001,
    Pointer = 0x0002,
    Array = 0x0003,
    Class = 0x0004,
    Structure = 0x0005,
    Union = 0x0006,
    Enum = 0x0007,
    Procedure = 0x0008,
    MemberFunction = 0x0009,
    VirtualFunctionTableShape = 0x000A,
    Cobol0 = 0x000B,
    Cobol1 = 0x000C,
    BasicArray = 0x000D,
    Label = 0x000E,
    Null = 0x000F,
    NotTranslated = 0x0010,
    MultiDimensionalArray = 0x0011,
    VirtualFunctionTablePath = 0x0012,
    PrecompiledTypeReference = 0x0013,
    PrecompiledTypesEnd = 0x0014,
    OemGenericType = 0x0015,

    // 0x0200..=0x03FF: type records that can be referenced from other type records
    Skip = 0x0200,
    ArgumentList = 0x0201,
    DefaultArgument = 0x0202,
    List = 0x0203,
    FieldList = 0x0204,
    DerivedClasses = 0x0205,
    BitFields = 0x0206,
    MethodList = 0x0207,
    DimensionedArrayDefaultLowerConstantUpper = 0x0208,
    DimensionedArrayConstantLowerConstantUpper = 0x0209,
    DimensionedArrayDefaultLowerVariableUpper = 0x020A,
    DimensionedArrayVariableLowerVariableUpper = 0x020B,
    ReferencedSymbol = 0x020C,

    // 0x0400..=0x05FF: type records for fields of complex lists
    RealBaseClass = 0x0400,
    DirectVirtualBaseClass = 0x0401,
    IndirectVirtualBaseClass = 0x0402,
    EnumerationNameAndValue = 0x0403,
    FriendFunction = 0x0404,
    IndexToAnotherTypeRecord = 0x0405,
    DataMember = 0x0406,
    StaticDataMember = 0x0407,
    Method = 0x0408,
    NestedTypeDefinition = 0x0409,
    VirtualFunctionTablePointer = 0x040A,
    FriendClass = 0x040B,
    OneMethod = 0x040C,
    VirtualFunctionOffset = 0x040D,

    Other(u16),
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub enum TypeLeaf {
    Modifier(ModifierTypeLeaf),
    Pointer(PointerTypeLeaf),
    Array(ArrayTypeLeaf),
    Class(StructureTypeLeaf),
    Structure(StructureTypeLeaf),
    Union(UnionTypeLeaf),
    Enum(EnumTypeLeaf),
    Procedure(ProcedureTypeLeaf),
    MemberFunction(MemberFunctionTypeLeaf),
    VirtualFunctionTableShape(VirtualFunctionTableShapeTypeLeaf),
    /*
    Cobol0(Cobol0TypeLeaf),
    Cobol1(Cobol1TypeLeaf),
    BasicArray(BasicArrayTypeLeaf),
    Label(LabelTypeLeaf),
    Null(NullTypeLeaf),
    NotTranslated(NotTranslatedTypeLeaf),
    MultiDimensionalArray(MultiDimensionalArrayTypeLeaf),
    VirtualFunctionTablePath(VirtualFunctionTablePathTypeLeaf),
    PrecompiledTypeReference(PrecompiledTypeReferenceTypeLeaf),
    PrecompiledTypesEnd(PrecompiledTypesEndTypeLeaf),
    OemGenericType(OemGenericTypeTypeLeaf),

    Skip(SkipTypeLeaf),
    */
    ArgumentList(ArgumentListTypeLeaf),
    /*
    DefaultArgument(DefaultArgumentTypeLeaf),
    List(ListTypeLeaf),
    */
    FieldList(FieldListTypeLeaf),
    DerivedClasses(DerivedClassesTypeLeaf),
    BitFields(BitFieldsTypeLeaf),
    MethodList(MethodListTypeLeaf),
    /*
    DimensionedArrayDefaultLowerConstantUpper(DimensionedArrayDefaultLowerConstantUpperTypeLeaf),
    DimensionedArrayConstantLowerConstantUpper(DimensionedArrayConstantLowerConstantUpperTypeLeaf),
    DimensionedArrayDefaultLowerVariableUpper(DimensionedArrayDefaultLowerVariableUpperTypeLeaf),
    DimensionedArrayVariableLowerVariableUpper(DimensionedArrayVariableLowerVariableUpperTypeLeaf),
    ReferencedSymbol(ReferencedSymbolTypeLeaf),
    */

    RealBaseClass(RealBaseClassTypeLeaf),
    /*
    DirectVirtualBaseClass(DirectVirtualBaseClassTypeLeaf),
    IndirectVirtualBaseClass(IndirectVirtualBaseClassTypeLeaf),
    */
    EnumerationNameAndValue(EnumerationNameAndValueTypeLeaf),
    /*
    FriendFunction(FriendFunctionTypeLeaf),
    IndexToAnotherTypeRecord(IndexToAnotherTypeRecordTypeLeaf),
    */
    DataMember(DataMemberTypeLeaf),
    StaticDataMember(StaticDataMemberTypeLeaf),
    Method(MethodTypeLeaf),
    NestedTypeDefinition(NestedTypeDefinitionTypeLeaf),
    VirtualFunctionTablePointer(VirtualFunctionTablePointerTypeLeaf),
    /*
    FriendClass(FriendClassTypeLeaf),
    */
    OneMethod(OneMethodTypeLeaf),
    /*
    VirtualFunctionOffset(VirtualFunctionOffsetTypeLeaf),
    */

    Other { index: u16, data: DisplayBytesVec },
}
impl TypeLeaf {
    #[instrument(skip_all)]
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, io::Error> {
        let mut index_buf = [0u8; 2];
        reader.read_exact(&mut index_buf)?;
        let index_u16 = u16::from_le_bytes(index_buf);
        let index = TypeLeafIndex::from_base_type(index_u16);

        debug!("type leaf index: {:?}", index);

        match index {
            TypeLeafIndex::Modifier => {
                let content = ModifierTypeLeaf::read(reader)?;
                Ok(Self::Modifier(content))
            },
            TypeLeafIndex::Pointer => {
                let content = PointerTypeLeaf::read(reader)?;
                Ok(Self::Pointer(content))
            },
            TypeLeafIndex::Array => {
                let content = ArrayTypeLeaf::read(reader)?;
                Ok(Self::Array(content))
            },
            TypeLeafIndex::Class|TypeLeafIndex::Structure => {
                let content = StructureTypeLeaf::read(reader)?;
                match index {
                    TypeLeafIndex::Class => Ok(Self::Class(content)),
                    TypeLeafIndex::Structure => Ok(Self::Structure(content)),
                    _ => unreachable!(),
                }
            },
            TypeLeafIndex::Union => {
                let content = UnionTypeLeaf::read(reader)?;
                Ok(Self::Union(content))
            },
            TypeLeafIndex::Enum => {
                let content = EnumTypeLeaf::read(reader)?;
                Ok(Self::Enum(content))
            },
            TypeLeafIndex::Procedure => {
                let content = ProcedureTypeLeaf::read(reader)?;
                Ok(Self::Procedure(content))
            },
            TypeLeafIndex::MemberFunction => {
                let content = MemberFunctionTypeLeaf::read(reader)?;
                Ok(Self::MemberFunction(content))
            },
            TypeLeafIndex::VirtualFunctionTableShape => {
                let content = VirtualFunctionTableShapeTypeLeaf::read(reader)?;
                Ok(Self::VirtualFunctionTableShape(content))
            },
            TypeLeafIndex::ArgumentList => {
                let content = ArgumentListTypeLeaf::read(reader)?;
                Ok(Self::ArgumentList(content))
            },
            TypeLeafIndex::FieldList => {
                let content = FieldListTypeLeaf::read(reader)?;
                Ok(Self::FieldList(content))
            },
            TypeLeafIndex::DerivedClasses => {
                let content = DerivedClassesTypeLeaf::read(reader)?;
                Ok(Self::DerivedClasses(content))
            },
            TypeLeafIndex::BitFields => {
                let content = BitFieldsTypeLeaf::read(reader)?;
                Ok(Self::BitFields(content))
            },
            TypeLeafIndex::MethodList => {
                let content = MethodListTypeLeaf::read(reader)?;
                Ok(Self::MethodList(content))
            },
            TypeLeafIndex::RealBaseClass => {
                let content = RealBaseClassTypeLeaf::read(reader)?;
                Ok(Self::RealBaseClass(content))
            },
            TypeLeafIndex::EnumerationNameAndValue => {
                let content = EnumerationNameAndValueTypeLeaf::read(reader)?;
                Ok(Self::EnumerationNameAndValue(content))
            },
            TypeLeafIndex::DataMember => {
                let content = DataMemberTypeLeaf::read(reader)?;
                Ok(Self::DataMember(content))
            },
            TypeLeafIndex::StaticDataMember => {
                let content = StaticDataMemberTypeLeaf::read(reader)?;
                Ok(Self::StaticDataMember(content))
            },
            TypeLeafIndex::Method => {
                let content = MethodTypeLeaf::read(reader)?;
                Ok(Self::Method(content))
            },
            TypeLeafIndex::NestedTypeDefinition => {
                let content = NestedTypeDefinitionTypeLeaf::read(reader)?;
                Ok(Self::NestedTypeDefinition(content))
            },
            TypeLeafIndex::VirtualFunctionTablePointer => {
                let content = VirtualFunctionTablePointerTypeLeaf::read(reader)?;
                Ok(Self::VirtualFunctionTablePointer(content))
            },
            TypeLeafIndex::OneMethod => {
                let content = OneMethodTypeLeaf::read(reader)?;
                Ok(Self::OneMethod(content))
            },
            other => {
                let other_u16 = other.to_base_type();
                let mut data_vec = Vec::new();
                reader.read_to_end(&mut data_vec)?;
                let data = DisplayBytesVec::from(data_vec);
                Ok(Self::Other { index: other_u16, data })
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct MemberAttributes {
    // begin 16-bit bitfield
    pub access: MemberAccess, // u2
    pub method_property: MethodProperty, // u3
    pub pseudo: bool, // u1
    pub no_inherit: bool, // u1
    pub no_construct: bool, // u1
    pub reserved: u8, // u8
    // end bitfield
}
impl MemberAttributes {
    pub fn from_u16(value: u16) -> Self {
        let access_u8 = u8::try_from((value >> 0) & 0b11).unwrap();
        let method_property_u8 = u8::try_from((value >> 2) & 0b111).unwrap();
        let pseudo = (value & (1 << 5)) != 0;
        let no_inherit = (value & (1 << 6)) != 0;
        let no_construct = (value & (1 << 7)) != 0;
        let reserved = u8::try_from((value >> 8) & 0xFF).unwrap();

        let access = MemberAccess::try_from_repr(access_u8).unwrap();
        let method_property = MethodProperty::try_from_repr(method_property_u8).unwrap();

        Self {
            access,
            method_property,
            pseudo,
            no_inherit,
            no_construct,
            reserved,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, FromToRepr, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
#[repr(u8)] // technically u2
pub enum MemberAccess {
    NoProtection = 0,
    Private = 1,
    Protected = 2,
    Public = 3,
}

#[derive(Clone, Copy, Debug, Eq, FromToRepr, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
#[repr(u8)] // technically u3
pub enum MethodProperty {
    Vanilla = 0,
    Virtual = 1,
    Static = 2,
    Friend = 3,
    IntroducingVirtual = 4,
    PureVirtual = 5,
    PureIntroducingVirtual = 6,
    Reserved = 7,
}
impl MethodProperty {
    pub fn is_virtual(&self) -> bool {
        match self {
            Self::Vanilla|Self::Static|Self::Friend|Self::Reserved
                => false,
            Self::Virtual|Self::IntroducingVirtual|Self::PureVirtual|Self::PureIntroducingVirtual
                => true,
        }
    }

    pub fn is_introducing_virtual(&self) -> bool {
        match self {
            Self::Vanilla|Self::Static|Self::Friend|Self::Reserved|Self::Virtual|Self::PureVirtual
                => false,
            Self::IntroducingVirtual|Self::PureIntroducingVirtual
                => true,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct ModifierTypeLeaf {
    pub attributes: ModifierTypeAttributes, // u16
    pub base_type_index: u16,
}
impl ModifierTypeLeaf {
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, io::Error> {
        let mut header_buf = [0u8; 4];
        reader.read_exact(&mut header_buf)?;

        let attributes_u16 = u16::from_le_byte_slice(&header_buf[0..2]);
        let base_type_index = u16::from_le_byte_slice(&header_buf[0..2]);

        let attributes = ModifierTypeAttributes::from_bits_retain(attributes_u16);

        Ok(Self {
            attributes,
            base_type_index,
        })
    }
}

bitflags! {
    #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    #[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
    pub struct ModifierTypeAttributes : u16 {
        const CONST = 0x0000_0001;
        const VOLATILE = 0x0000_0002;
        const UNALIGNED = 0x0000_0004;
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct PointerTypeLeaf {
    pub attribute: PointerTypeAttributes, // u16
    pub pointee_type_index: u16,
    pub variant: PointerTypeVariant,
}
impl PointerTypeLeaf {
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, io::Error> {
        let mut header_buf = [0u8; 4];
        reader.read_exact(&mut header_buf)?;

        let attribute_u16 = u16::from_le_byte_slice(&header_buf[0..2]);
        let pointee_type_index = u16::from_le_byte_slice(&header_buf[2..4]);
        let attribute = PointerTypeAttributes::from_u16(attribute_u16);

        let variant = match attribute.mode {
            PointerMode::Pointer|PointerMode::Reference => {
                match attribute.kind {
                    PointerKind::BasedOnSegment => {
                        let mut segment_buf = [0u8; 2];
                        reader.read_exact(&mut segment_buf)?;
                        let segment = u16::from_le_bytes(segment_buf);
                        PointerTypeVariant::BaseSegment(segment)
                    },
                    PointerKind::BasedOnType => {
                        let mut index_buf = [0u8; 2];
                        reader.read_exact(&mut index_buf)?;
                        let index = u16::from_le_bytes(index_buf);
                        let name = read_pascal_byte_string(reader)?;

                        PointerTypeVariant::BaseType {
                            base_type_index: index,
                            base_type_name: DisplayBytesVec::from(name),
                        }
                    },
                    PointerKind::BasedOnSymbolAddress|PointerKind::BasedOnSymbolAddressSegment => {
                        let symbol = SymbolEntry::read(reader)?;
                        PointerTypeVariant::BaseSymbol(symbol)
                    },
                    _ => PointerTypeVariant::Empty,
                }
            },
            PointerMode::DataMemberPointer|PointerMode::MethodPointer => {
                let mut variant_buf = [0u8; 4];
                reader.read_exact(&mut variant_buf)?;

                let class_type_index = u16::from_le_byte_slice(&variant_buf[0..2]);
                let format_u16 = u16::from_le_byte_slice(&variant_buf[2..4]);

                let format = PointerToMemberFormat::from_base_type(format_u16);

                PointerTypeVariant::DataMember {
                    class_type_index,
                    format,
                }
            },
            _ => PointerTypeVariant::Empty,
        };

        Ok(Self {
            attribute,
            pointee_type_index,
            variant,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct PointerTypeAttributes {
    // begin 16-bit bitfield
    pub kind: PointerKind, // u5
    pub mode: PointerMode, // u3
    pub is_flat_32: bool, // u1
    pub is_volatile: bool, // u1
    pub is_const: bool, // u1
    pub is_unaligned: bool, // u1
    pub unused: u8, // u4
    // end bitfield
}
impl PointerTypeAttributes {
    pub fn from_u16(value: u16) -> Self {
        let kind_u8 = u8::try_from((value >> 0) & 0b11111).unwrap();
        let mode_u8 = u8::try_from((value >> 5) & 0b111).unwrap();
        let is_flat_32 = (value & (1 << 8)) != 0;
        let is_volatile = (value & (1 << 9)) != 0;
        let is_const = (value & (1 << 10)) != 0;
        let is_unaligned = (value & (1 << 11)) != 0;
        let unused = u8::try_from((value >> 12) & 0b1111).unwrap();

        let kind = PointerKind::from_base_type(kind_u8);
        let mode = PointerMode::from_base_type(mode_u8);

        Self {
            kind,
            mode,
            is_flat_32,
            is_volatile,
            is_const,
            is_unaligned,
            unused,
        }
    }
}

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
#[from_to_other(base_type = u8, derive_compare = "as_int")] // technically u5
pub enum PointerKind {
    Near = 0,
    Far = 1,
    Huge = 2,
    BasedOnSegment = 3,
    BasedOnValue = 4,
    BasedOnValueSegment = 5,
    BasedOnSymbolAddress = 6,
    BasedOnSymbolAddressSegment = 7,
    BasedOnType = 8,
    BasedOnSelf = 9,
    Near32 = 10,
    Far32 = 11,
    Other(u8),
}

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
#[from_to_other(base_type = u8, derive_compare = "as_int")] // technically u3
pub enum PointerMode {
    Pointer = 0,
    Reference = 1,
    DataMemberPointer = 2,
    MethodPointer = 3, 
    Other(u8),
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub enum PointerTypeVariant {
    Empty,
    DataMember {
        class_type_index: u16,
        format: PointerToMemberFormat, // u16,
    },
    BaseSegment(u16),
    BaseSymbol(SymbolEntry),
    BaseType {
        base_type_index: u16,
        base_type_name: DisplayBytesVec,
    },
}

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
#[from_to_other(base_type = u16, derive_compare = "as_int")]
pub enum PointerToMemberFormat {
    Data16_16VirtualNone = 0,
    Data16_16VirtualFunctions = 1,
    Data16_16VirtualBases = 2,
    Data16_32VirtualNoneOrFunctions = 3,
    Data16_32VirtualBases = 4,
    NearMethod16_16SingleAddressPoint = 5,
    NearMethod16_16MultipleAddressPoints = 6,
    NearMethod16_16VirtualBases = 7,
    FarMethod16_16SingleAddressPoint = 8,
    FarMethod16_16MultipleAddressPoints = 9,
    FarMethod16_16VirtualBases = 10,
    Method16_32SingleAddressPoint = 11,
    Method16_32MultipleAddressPoints = 12,
    Method16_32VirtualBases = 13,
    Other(u16),
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct ArrayTypeLeaf {
    pub element_type_index: u16,
    pub index_type_index: u16,
    pub length: NumericLeaf,
    pub name: DisplayBytesVec, // PascalString
}
impl ArrayTypeLeaf {
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, io::Error> {
        let mut header_buf = [0u8; 4];
        reader.read_exact(&mut header_buf)?;

        let element_type_index = u16::from_le_byte_slice(&header_buf[0..2]);
        let index_type_index = u16::from_le_byte_slice(&header_buf[2..4]);
        let length = NumericLeaf::read(reader)?;
        let name_vec = read_pascal_byte_string(reader)?;

        let name = DisplayBytesVec::from(name_vec);

        Ok(Self {
            element_type_index,
            index_type_index,
            length,
            name,
        })
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct StructureTypeLeaf {
    pub element_count: u16,
    pub field_list_type_index: u16,
    pub properties: StructureProperties,
    pub derivation_list_type_index: u16,
    pub virtual_function_table_shape_descriptor_type_index: u16,
    pub length: NumericLeaf,
    pub name: DisplayBytesVec, // PascalString
}
impl StructureTypeLeaf {
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, io::Error> {
        let mut header_buf = [0u8; 10];
        reader.read_exact(&mut header_buf)?;

        let element_count = u16::from_le_byte_slice(&header_buf[0..2]);
        let field_list_type_index = u16::from_le_byte_slice(&header_buf[2..4]);
        let properties_u16 = u16::from_le_byte_slice(&header_buf[4..6]);
        let derivation_list_type_index = u16::from_le_byte_slice(&header_buf[6..8]);
        let virtual_function_table_shape_descriptor_type_index = u16::from_le_byte_slice(&header_buf[8..10]);

        let properties = StructureProperties::from_bits_retain(properties_u16);

        let length = NumericLeaf::read(reader)?;
        let name_vec = read_pascal_byte_string(reader)?;

        let name = DisplayBytesVec::from(name_vec);

        Ok(Self {
            element_count,
            field_list_type_index,
            properties,
            derivation_list_type_index,
            virtual_function_table_shape_descriptor_type_index,
            length,
            name,
        })
    }
}

bitflags! {
    #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    #[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
    pub struct StructureProperties : u16 {
        const PACKED = 0x0001;
        const CONSTRUCTOR_DESTRUCTOR = 0x0002;
        const OVERLOADED_OPERATORS = 0x0004;
        const NESTED = 0x0008;
        const CONTAINS_NESTED = 0x0010;
        const OVERLOADED_ASSIGNMENT = 0x0020;
        const CASTING_METHODS = 0x0040;
        const FORWARD_REFERENCE = 0x0080;
        const SCOPED = 0x0100;
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct UnionTypeLeaf {
    pub field_count: u16,
    pub field_list_type_index: u16,
    pub properties: StructureProperties,
    pub length: NumericLeaf,
    pub name: DisplayBytesVec, // PascalString
}
impl UnionTypeLeaf {
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, io::Error> {
        let mut header_buf = [0u8; 6];
        reader.read_exact(&mut header_buf)?;

        let field_count = u16::from_le_byte_slice(&header_buf[0..2]);
        let field_list_type_index = u16::from_le_byte_slice(&header_buf[2..4]);
        let properties_u16 = u16::from_le_byte_slice(&header_buf[4..6]);

        let properties = StructureProperties::from_bits_retain(properties_u16);

        let length = NumericLeaf::read(reader)?;
        let name_vec = read_pascal_byte_string(reader)?;

        let name = DisplayBytesVec::from(name_vec);

        Ok(Self {
            field_count,
            field_list_type_index,
            properties,
            length,
            name,
        })
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct EnumTypeLeaf {
    pub option_count: u16,
    pub underlying_type_index: u16,
    pub field_list_type_index: u16,
    pub member_attributes: MemberAttributes,
    pub name: DisplayBytesVec, // PascalString
}
impl EnumTypeLeaf {
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, io::Error> {
        let mut buf = [0u8; 8];
        reader.read_exact(&mut buf)?;

        let option_count = u16::from_le_byte_slice(&buf[0..2]);
        let underlying_type_index = u16::from_le_byte_slice(&buf[2..4]);
        let field_list_type_index = u16::from_le_byte_slice(&buf[4..6]);
        let member_attributes_u16 = u16::from_le_byte_slice(&buf[6..8]);

        let member_attributes = MemberAttributes::from_u16(member_attributes_u16);

        let name_vec = read_pascal_byte_string(reader)?;

        let name = DisplayBytesVec::from(name_vec);

        Ok(Self {
            option_count,
            underlying_type_index,
            field_list_type_index,
            member_attributes,
            name,
        })
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct ProcedureTypeLeaf {
    pub return_value_type_index: u16,
    pub calling_convention: CallingConvention, // u8
    pub reserved: u8,
    pub parameter_count: u16,
    pub argument_list_type_index: u16,
}
impl ProcedureTypeLeaf {
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, io::Error> {
        let mut buf = [0u8; 8];
        reader.read_exact(&mut buf)?;

        let return_value_type_index = u16::from_le_byte_slice(&buf[0..2]);
        let calling_convention_u8 = buf[2];
        let reserved = buf[3];
        let parameter_count = u16::from_le_byte_slice(&buf[4..6]);
        let argument_list_type_index = u16::from_le_byte_slice(&buf[6..8]);

        let calling_convention = CallingConvention::from_base_type(calling_convention_u8);

        Ok(Self {
            return_value_type_index,
            calling_convention,
            reserved,
            parameter_count,
            argument_list_type_index,
        })
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct MemberFunctionTypeLeaf {
    pub return_value_type_index: u16,
    pub class_type_index: u16,
    pub this_arg_type_index: u16,
    pub calling_convention: CallingConvention, // u8
    pub reserved: u8,
    pub parameter_count: u16,
    pub argument_list_type_index: u16,
    pub this_adjuster: u32,
}
impl MemberFunctionTypeLeaf {
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, io::Error> {
        let mut buf = [0u8; 16];
        reader.read_exact(&mut buf)?;

        let return_value_type_index = u16::from_le_byte_slice(&buf[0..2]);
        let class_type_index = u16::from_le_byte_slice(&buf[2..4]);
        let this_arg_type_index = u16::from_le_byte_slice(&buf[4..6]);
        let calling_convention_u8 = buf[6];
        let reserved = buf[7];
        let parameter_count = u16::from_le_byte_slice(&buf[8..10]);
        let argument_list_type_index = u16::from_le_byte_slice(&buf[10..12]);
        let this_adjuster = u32::from_le_byte_slice(&buf[12..16]);

        let calling_convention = CallingConvention::from_base_type(calling_convention_u8);

        Ok(Self {
            return_value_type_index,
            class_type_index,
            this_arg_type_index,
            calling_convention,
            reserved,
            parameter_count,
            argument_list_type_index,
            this_adjuster,
        })
    }
}

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
#[from_to_other(base_type = u8, derive_compare = "as_int")]
pub enum CallingConvention {
    NearC = 0,
    FarC = 1,
    NearPascal = 2,
    FarPascal = 3,
    NearFastcall = 4,
    FarFastcall = 5,
    // 6 is reserved
    NearStdcall = 7,
    FarStdcall = 8,
    NearSyscall = 9,
    FarSyscakk = 10,
    ThisCall = 11,
    MipsCall = 12,
    Generic = 13,
    Other(u8),
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct VirtualFunctionTableShapeTypeLeaf {
    pub descriptor_count: u16,
    pub descriptors: Vec<VirtualFunctionTableShapeDescriptor>, // [VirtualFunctionTableShapeDescriptor; ceil(descriptor_count / 2)]
}
impl VirtualFunctionTableShapeTypeLeaf {
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, io::Error> {
        let mut count_buf = [0u8; 2];
        reader.read_exact(&mut count_buf)?;
        let descriptor_count = u16::from_le_bytes(count_buf);
        let descriptor_count_usize = usize::from(descriptor_count);

        // two descriptors per byte, round up
        let descriptor_byte_count = (descriptor_count_usize + (2 - 1)) / 2;
        let mut descriptor_buf = vec![0u8; descriptor_byte_count];
        reader.read_exact(&mut descriptor_buf)?;

        let mut descriptors = Vec::with_capacity(descriptor_count_usize);
        for b in descriptor_buf {
            let first_descriptor_u4 = (b >> 4) & 0b1111;
            let first_descriptor = VirtualFunctionTableShapeDescriptor::from_base_type(first_descriptor_u4);
            descriptors.push(first_descriptor);

            if descriptors.len() < descriptor_count_usize {
                let second_descriptor_u4 = (b >> 0) & 0b1111;
                let second_descriptor = VirtualFunctionTableShapeDescriptor::from_base_type(second_descriptor_u4);
                descriptors.push(second_descriptor);
            }
        }

        Ok(Self {
            descriptor_count,
            descriptors,
        })
    }
}

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
#[from_to_other(base_type = u8, derive_compare = "as_int")] // actually u4
pub enum VirtualFunctionTableShapeDescriptor {
    Near = 0,
    Far = 1,
    Thin = 2,
    AddressPointDisplacementToOutermostClass = 3,
    FarPointerToMetaclassDescriptor = 4,
    Near32 = 5,
    Far32 = 6,
    Other(u8),
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct ArgumentListTypeLeaf {
    pub argument_count: u16,
    pub argument_type_indexes: Vec<u16>, // [u16; argument_count]
}
impl ArgumentListTypeLeaf {
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, io::Error> {
        let mut count_buf = [0u8; 2];
        reader.read_exact(&mut count_buf)?;
        let argument_count = u16::from_le_bytes(count_buf);
        let argument_count_usize = usize::from(argument_count);

        let mut argument_type_indexes_buf = vec![0u8; 2*argument_count_usize];
        reader.read_exact(&mut argument_type_indexes_buf)?;
        let argument_type_indexes: Vec<u16> = argument_type_indexes_buf
            .chunks(2)
            .map(|chunk| u16::from_le_byte_slice(chunk))
            .collect();

        Ok(Self {
            argument_count,
            argument_type_indexes,
        })
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct FieldListTypeLeaf {
    pub fields: Vec<TypeLeaf>,
}
impl FieldListTypeLeaf {
    #[instrument(skip_all)]
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, io::Error> {
        let mut fields = Vec::new();

        loop {
            // is there a field?
            let mut before_field_buf = [0u8];
            match reader.read(&mut before_field_buf)? {
                0 => {
                    // nope
                    break;
                },
                1 => {
                    // yep; take a step back and read it
                    reader.seek(SeekFrom::Current(-1))?;
                },
                other  => unreachable!("read {} bytes into a buffer 1 byte long", other),
            }

            // read a field
            let field = TypeLeaf::read(reader)?;
            fields.push(field);

            // is it followed by padding?
            let mut possible_padding_buf = [0u8; 1];
            match reader.read(&mut possible_padding_buf)? {
                0 => {
                    // EOF, even better
                    break;
                },
                1 => {
                    // is it padding?
                    if possible_padding_buf[0] > 0xF0 {
                        // yes; munch n-1 bytes (since we already ate the first one)
                        let padding_bytes = usize::from(possible_padding_buf[0] & 0x0F) - 1;
                        if padding_bytes > 0 {
                            let mut padding_buf = vec![0u8; padding_bytes];
                            reader.read_exact(&mut padding_buf)?;
                        }
                    } else {
                        // it's another field; go back and read from there
                        reader.seek(SeekFrom::Current(-1))?;
                    }
                },
                other => unreachable!("read {} bytes into a buffer 1 byte long", other),
            }
        }

        Ok(Self {
            fields,
        })
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct DerivedClassesTypeLeaf {
    pub derived_class_count: u16,
    pub derived_class_type_record_indices: Vec<u16>, // [u16; derived_class_count]
}
impl DerivedClassesTypeLeaf {
    #[instrument(skip_all)]
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, io::Error> {
        let mut header_buf = [0u8; 2];
        reader.read_exact(&mut header_buf)?;
        let derived_class_count = u16::from_le_bytes(header_buf);

        let derived_class_count_usize = usize::from(derived_class_count);
        let mut derived_class_buf = vec![0u8; 2*derived_class_count_usize];
        reader.read_exact(&mut derived_class_buf)?;
        let derived_class_type_record_indices: Vec<u16> = derived_class_buf
            .chunks(2)
            .map(|chunk| u16::from_le_byte_slice(chunk))
            .collect();

        Ok(Self {
            derived_class_count,
            derived_class_type_record_indices,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct BitFieldsTypeLeaf {
    pub bit_count: u8,
    pub position: u8,
    pub type_record_index: u16,
}
impl BitFieldsTypeLeaf {
    #[instrument(skip_all)]
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, io::Error> {
        let mut buf = [0u8; 4];
        reader.read_exact(&mut buf)?;
        let bit_count = buf[0];
        let position = buf[1];
        let type_record_index = u16::from_le_byte_slice(&buf[2..4]);

        Ok(Self {
            bit_count,
            position,
            type_record_index,
        })
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct MethodListTypeLeaf {
    pub methods: Vec<MethodListEntry>, // repeated until buffer is exhausted
}
impl MethodListTypeLeaf {
    #[instrument(skip_all)]
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, io::Error> {
        let mut methods = Vec::new();
        loop {
            // any more entries?
            let mut test_buf = [0u8];
            match reader.read(&mut test_buf)? {
                0 => {
                    // no
                    break;
                },
                1 => {
                    // yes
                    // keep going
                },
                other => unreachable!("read {} bytes into 1-byte buffer?!", other),
            }

            // rewind
            reader.seek(SeekFrom::Current(-1))?;

            // read
            let method = MethodListEntry::read(reader)?;
            methods.push(method);
        }

        Ok(Self {
            methods,
        })
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct MethodListEntry {
    pub member_attributes: MemberAttributes, // u16
    pub type_record_index: u16,
    pub virtual_function_table_offset: Option<u32>,
}
impl MethodListEntry {
    #[instrument(skip_all)]
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, io::Error> {
        let mut header_buf = [0u8; 4];
        reader.read_exact(&mut header_buf)?;
        let member_attributes_u16 = u16::from_le_byte_slice(&header_buf[0..2]);
        let type_record_index = u16::from_le_byte_slice(&header_buf[2..4]);

        let member_attributes = MemberAttributes::from_u16(member_attributes_u16);

        let virtual_function_table_offset = if member_attributes.method_property.is_introducing_virtual() {
            let mut vfto_buf = [0u8; 4];
            reader.read_exact(&mut vfto_buf)?;
            Some(u32::from_le_bytes(vfto_buf))
        } else {
            None
        };

        Ok(Self {
            member_attributes,
            type_record_index,
            virtual_function_table_offset,
        })
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct RealBaseClassTypeLeaf {
    pub type_record_index: u16,
    pub member_attributes: MemberAttributes, // u16
    pub offset: NumericLeaf,
}
impl RealBaseClassTypeLeaf {
    #[instrument(skip_all)]
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, io::Error> {
        let mut header_buf = [0u8; 4];
        reader.read_exact(&mut header_buf)?;
        let type_record_index = u16::from_le_byte_slice(&header_buf[0..2]);
        let member_attributes_u16 = u16::from_le_byte_slice(&header_buf[2..4]);

        let member_attributes = MemberAttributes::from_u16(member_attributes_u16);
        debug!("tri: {}, ma: {:?}", type_record_index, member_attributes);

        let offset = NumericLeaf::read(reader)?;
        debug!("data member offset: {:?}", offset);

        Ok(Self {
            type_record_index,
            member_attributes,
            offset,
        })
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct EnumerationNameAndValueTypeLeaf {
    pub member_attributes: MemberAttributes, // u16
    pub value: NumericLeaf,
    pub name: DisplayBytesVec, // PascalString
}
impl EnumerationNameAndValueTypeLeaf {
    #[instrument(skip_all)]
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, io::Error> {
        let mut attributes_buf = [0u8; 2];
        reader.read_exact(&mut attributes_buf)?;
        let member_attributes_u16 = u16::from_le_bytes(attributes_buf);

        let member_attributes = MemberAttributes::from_u16(member_attributes_u16);

        let value = NumericLeaf::read(reader)?;
        let name_vec = read_pascal_byte_string(reader)?;

        let name = DisplayBytesVec::from(name_vec);

        Ok(Self {
            member_attributes,
            value,
            name,
        })
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct DataMemberTypeLeaf {
    pub type_record_index: u16,
    pub member_attributes: MemberAttributes, // u16
    pub offset: NumericLeaf,
    pub name: DisplayBytesVec, // PascalString
}
impl DataMemberTypeLeaf {
    #[instrument(skip_all)]
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, io::Error> {
        let mut header_buf = [0u8; 4];
        reader.read_exact(&mut header_buf)?;
        let type_record_index = u16::from_le_byte_slice(&header_buf[0..2]);
        let member_attributes_u16 = u16::from_le_byte_slice(&header_buf[2..4]);

        let member_attributes = MemberAttributes::from_u16(member_attributes_u16);

        let offset = NumericLeaf::read(reader)?;
        debug!("data member offset: {:?}", offset);
        let name_vec = read_pascal_byte_string(reader)?;

        let name = DisplayBytesVec::from(name_vec);
        debug!("data member name: {}", name);

        Ok(Self {
            type_record_index,
            member_attributes,
            offset,
            name,
        })
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct StaticDataMemberTypeLeaf {
    pub type_record_index: u16,
    pub member_attributes: MemberAttributes, // u16
    pub name: DisplayBytesVec, // PascalString
}
impl StaticDataMemberTypeLeaf {
    #[instrument(skip_all)]
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, io::Error> {
        let mut header_buf = [0u8; 4];
        reader.read_exact(&mut header_buf)?;
        let type_record_index = u16::from_le_byte_slice(&header_buf[0..2]);
        let member_attributes_u16 = u16::from_le_byte_slice(&header_buf[2..4]);

        let member_attributes = MemberAttributes::from_u16(member_attributes_u16);

        let name_vec = read_pascal_byte_string(reader)?;

        let name = DisplayBytesVec::from(name_vec);
        debug!("data member name: {}", name);

        Ok(Self {
            type_record_index,
            member_attributes,
            name,
        })
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct MethodTypeLeaf {
    pub overload_count: u16, // I think?
    pub method_list_type_index: u16,
    pub name: DisplayBytesVec, // PascalString
}
impl MethodTypeLeaf {
    #[instrument(skip_all)]
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, io::Error> {
        let mut header_buf = [0u8; 4];
        reader.read_exact(&mut header_buf)?;
        let overload_count = u16::from_le_byte_slice(&header_buf[0..2]);
        let method_list_type_index = u16::from_le_byte_slice(&header_buf[2..4]);

        let name_vec = read_pascal_byte_string(reader)?;

        let name = DisplayBytesVec::from(name_vec);

        Ok(Self {
            overload_count,
            method_list_type_index,
            name,
        })
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct NestedTypeDefinitionTypeLeaf {
    pub nested_type_record_index: u16,
    pub name: DisplayBytesVec, // PascalString
}
impl NestedTypeDefinitionTypeLeaf {
    #[instrument(skip_all)]
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, io::Error> {
        let mut buf = [0u8; 2];
        reader.read_exact(&mut buf)?;
        let nested_type_record_index = u16::from_le_bytes(buf);

        let name_vec = read_pascal_byte_string(reader)?;

        let name = DisplayBytesVec::from(name_vec);

        Ok(Self {
            nested_type_record_index,
            name,
        })
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct VirtualFunctionTablePointerTypeLeaf {
    pub pointer_type_record_index: u16,
}
impl VirtualFunctionTablePointerTypeLeaf {
    #[instrument(skip_all)]
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, io::Error> {
        let mut buf = [0u8; 2];
        reader.read_exact(&mut buf)?;
        let pointer_type_record_index = u16::from_le_bytes(buf);

        Ok(Self {
            pointer_type_record_index,
        })
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct OneMethodTypeLeaf {
    pub member_attributes: MemberAttributes,
    pub type_record_index: u16,
    pub virtual_function_table_offset: Option<u32>,
    pub name: DisplayBytesVec, // PascalString
}
impl OneMethodTypeLeaf {
    #[instrument(skip_all)]
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, io::Error> {
        let mut header_buf = [0u8; 4];
        reader.read_exact(&mut header_buf)?;
        let member_attributes_u16 = u16::from_le_byte_slice(&header_buf[0..2]);
        let type_record_index = u16::from_le_byte_slice(&header_buf[2..4]);

        let member_attributes = MemberAttributes::from_u16(member_attributes_u16);
        debug!("one-method member attributes: {:?}", member_attributes);

        // documentation says this field is present if the method "is virtual"
        // apparently, this field is only present if the method "is introducing virtual"
        let virtual_function_table_offset = if member_attributes.method_property.is_introducing_virtual() {
            let mut vfto_buf = [0u8; 4];
            reader.read_exact(&mut vfto_buf)?;
            Some(u32::from_le_bytes(vfto_buf))
        } else {
            None
        };

        let name_vec = read_pascal_byte_string(reader)?;

        let name = DisplayBytesVec::from(name_vec);
        debug!("data member name: {}", name);

        Ok(Self {
            type_record_index,
            member_attributes,
            virtual_function_table_offset,
            name,
        })
    }
}
