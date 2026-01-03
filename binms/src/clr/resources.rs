//! Collects and decodes CLR resources from a PE file.
//!
//! The [CLR header](crate::clr::header::ClrHeader) points at a sequence of the following:
//!
//! ```
//! struct WrappedResourceContainer {
//!     pub length: u32,
//!     pub data: [u8; length],
//!     pub padding: [u8; _], // to 8 bytes
//! }
//! ```
//!
//! Each `data` item generally has the structure [`ClrResourceContainer`].


use display_bytes::DisplayBytesVec;
use tracing::{debug, error};

use crate::clr::Error;
use crate::int_from_byte_slice::IntFromByteSlice;


#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u64)]
pub enum ResourceType {
    Null = 0,
    String = 1,
    Boolean = 2,
    Char = 3,
    Byte = 4,
    SignedByte = 5,
    Int16 = 6,
    UInt16 = 7,
    Int32 = 8,
    UInt32 = 9,
    Int64 = 10,
    UInt64 = 11,
    Single = 12,
    Double = 13,
    Decimal = 14,
    DateTime = 15,
    TimeSpan = 16,

    ByteArray = 32,
    Stream = 33,

    // >= 64
    Custom(u64),
}
impl TryFrom<u64> for ResourceType {
    type Error = ();

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Null),
            1 => Ok(Self::String),
            2 => Ok(Self::Boolean),
            3 => Ok(Self::Char),
            4 => Ok(Self::Byte),
            5 => Ok(Self::SignedByte),
            6 => Ok(Self::Int16),
            7 => Ok(Self::UInt16),
            8 => Ok(Self::Int32),
            9 => Ok(Self::UInt32),
            10 => Ok(Self::Int64),
            11 => Ok(Self::UInt64),
            12 => Ok(Self::Single),
            13 => Ok(Self::Double),
            14 => Ok(Self::Decimal),
            15 => Ok(Self::DateTime),
            16 => Ok(Self::TimeSpan),

            32 => Ok(Self::ByteArray),
            33 => Ok(Self::Stream),

            custom_value if custom_value >= 64 => {
                let custom_index = custom_value - 64;
                Ok(Self::Custom(custom_index))
            },
            _ => Err(()),
        }
    }
}
impl From<ResourceType> for u64 {
    fn from(value: ResourceType) -> Self {
        match value {
            ResourceType::Null => 0,
            ResourceType::String => 1,
            ResourceType::Boolean => 2,
            ResourceType::Char => 3,
            ResourceType::Byte => 4,
            ResourceType::SignedByte => 5,
            ResourceType::Int16 => 6,
            ResourceType::UInt16 => 7,
            ResourceType::Int32 => 8,
            ResourceType::UInt32 => 9,
            ResourceType::Int64 => 10,
            ResourceType::UInt64 => 11,
            ResourceType::Single => 12,
            ResourceType::Double => 13,
            ResourceType::Decimal => 14,
            ResourceType::DateTime => 15,
            ResourceType::TimeSpan => 16,

            ResourceType::ByteArray => 32,
            ResourceType::Stream => 33,

            ResourceType::Custom(custom_index) => custom_index + 64,
        }
    }
}


#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ClrResourceContainer {
    // magic: u32, // 0xBEEFCACE
    pub reader_count: u32,
    // reader_assembly_and_type_size: u32,
    pub reader_assembly: String, // length-prefixed US-ASCII; min. u8
    pub reader_type: String, // length-prefixed US-ASCII; min. u8
    pub version: u32,
    pub custom_resource_types: Vec<String>,
    pub resources: Vec<ClrResource>,
}
impl ClrResourceContainer {
    pub fn take_from_bytes(bytes: &[u8]) -> Result<(&[u8], Self), Error> {
        let rest = bytes;
        if rest.len() < 14 {
            error!("not enough bytes ({}) for fixed part of header (obtained {})", 14, rest.len());
            return Err(Error::TooShort);
        }

        let magic = u32::from_le_byte_slice(&rest[0..4]);
        if magic != 0xBEEFCACE {
            return Err(Error::WrongMagic { expected: 0xBEEFCACE, obtained: magic });
        }
        let rest = &rest[4..];

        let reader_count = u32::from_le_byte_slice(&rest[0..4]);
        let rest = &rest[4..];

        let reader_assembly_and_type_size_u32 = u32::from_le_byte_slice(&rest[0..4]);
        let rest = &rest[4..];
        let reader_assembly_and_type_size: usize = reader_assembly_and_type_size_u32.try_into().unwrap();
        if rest.len() < reader_assembly_and_type_size {
            error!("size of reader assembly and type info is {} but only {} bytes remain", reader_assembly_and_type_size, rest.len());
            return Err(Error::TooShort);
        }

        let (raats_rest, rest) = rest.split_at(reader_assembly_and_type_size);
        let (raats_rest, reader_assembly_bytes) = take_length_prefixed_bytes(raats_rest)?;
        let (_raats_rest, reader_type_bytes) = take_length_prefixed_bytes(raats_rest)?;
        let reader_assembly = String::from_utf8(reader_assembly_bytes.to_vec())
            .map_err(|_| Error::InvalidText)?;
        let reader_type = String::from_utf8(reader_type_bytes.to_vec())
            .map_err(|_| Error::InvalidText)?;
        debug!("reader assembly: {:?}", reader_assembly);
        debug!("reader type: {:?}", reader_type);

        if rest.len() < 12 {
            error!("not enough bytes ({}) for second fixed part of header (obtained {})", 12, rest.len());
            return Err(Error::TooShort);
        }
        let version = u32::from_le_byte_slice(&rest[0..4]);
        let resource_count_u32 = u32::from_le_byte_slice(&rest[4..8]);
        let custom_resource_type_count = u32::from_le_byte_slice(&rest[8..12]);
        let mut rest = &rest[12..];

        let resource_count: usize = resource_count_u32.try_into().unwrap();
        debug!("version {:?}, rescount {}, typecount {}", version, resource_count_u32, custom_resource_type_count);

        let mut custom_resource_types = Vec::with_capacity(custom_resource_type_count.try_into().unwrap());
        for i in 0..custom_resource_type_count {
            let (new_rest, custom_resource_type_bytes) = take_length_prefixed_bytes(rest)?;
            rest = new_rest;

            let custom_resource_type = String::from_utf8(custom_resource_type_bytes.to_vec())
                .inspect_err(|_| error!("resource index {} type is an invalid UTF-8 string", i))
                .map_err(|_| Error::InvalidText)?;
            debug!("custom resource type {}: {:?}", i, custom_resource_type);
            custom_resource_types.push(custom_resource_type);
        }

        // realign ourselves to 64 bits (8 bytes)
        let hitherto_bytes_read = bytes.len() - rest.len();
        let align_skip = (8 - (hitherto_bytes_read % 8)) % 8;
        debug!("align skip: {}", align_skip);
        if rest.len() < align_skip {
            error!("not enough bytes ({}) to realign ourselves after resource types ({} remain)", align_skip, rest.len());
            return Err(Error::TooShort);
        }
        let mut rest = &rest[align_skip..];

        // now, hashes of the resource names
        let mut name_hashes = Vec::with_capacity(resource_count);
        for i in 0..resource_count {
            if rest.len() < 4 {
                error!("not enough bytes ({}) for name hash of resource with index {} ({} bytes remain)", 4, i, rest.len());
                return Err(Error::TooShort);
            }
            let name_hash = u32::from_le_byte_slice(&rest[0..4]);
            debug!("resource with index {} has name hash {:#010X}", i, name_hash);
            name_hashes.push(name_hash);
            rest = &rest[4..];
        }

        // now, the same structure for the offsets to each resource's name and offset within the data section
        let mut name_offsets = Vec::with_capacity(resource_count);
        for i in 0..resource_count {
            if rest.len() < 4 {
                error!("not enough bytes ({}) for name of resource with index {} ({} bytes remain)", 4, i, rest.len());
                return Err(Error::TooShort);
            }
            let name_offset = u32::from_le_byte_slice(&rest[0..4]);
            debug!("resource with index {} has name at offset {:#010X}", i, name_offset);
            name_offsets.push(u32::from_le_byte_slice(&rest[0..4]));
            rest = &rest[4..];
        }

        // data section offset
        if rest.len() < 4 {
            error!("not enough bytes ({}) for data section offset ({} bytes remain)", 4, rest.len());
            return Err(Error::TooShort);
        }
        let data_section_offset_u32 = u32::from_le_byte_slice(&rest[0..4]);
        let data_section_offset: usize = data_section_offset_u32.try_into().unwrap();
        let names_and_offsets = &rest[4..];
        debug!("data section starts at {:#010X}", data_section_offset);

        // next, the names, hashes and offsets
        let mut resource_entries = Vec::with_capacity(resource_count);
        for (i, offset_u32) in name_offsets.iter().copied().enumerate() {
            let offset: usize = offset_u32.try_into().unwrap();
            let resource_slice = &names_and_offsets[offset..];
            let (res_rest, name_bytes) = take_length_prefixed_bytes(resource_slice)
                .inspect_err(|_| error!("resource with index {} has invalid length-prefixed name bytes", i))?;
            let name = utf16_le_bytes_to_string(name_bytes)
                .inspect_err(|_| error!("resource with index {} has invalid name", i))?;
            if res_rest.len() < 4 {
                error!("not enough bytes ({}) for data offset for resource with index {} ({} bytes remain)", 4, i, res_rest.len());
                return Err(Error::TooShort);
            }
            let data_offset = u32::from_le_byte_slice(&res_rest[0..4]);
            debug!("resource {:?} starts at {:#010X} within data section", name, data_offset);
            resource_entries.push((name, name_hashes[i], data_offset));
        }

        // now we can get at the resource data! (probably)

        // slice from the beginning of the data section offset
        if data_section_offset >= bytes.len() {
            error!("data section offset ({}) greater than than resources section length ({})", data_section_offset, bytes.len());
            return Err(Error::TooShort);
        }
        let data_section = &bytes[data_section_offset..];

        // find the lengths of the resources

        // 1. sort by offset
        resource_entries.sort_unstable_by_key(|(_name, _name_hash, offset)| *offset);

        // 2. take the next entry's offset as indicative of the previous entry's length
        let mut resource_name_hash_offset_length = Vec::with_capacity(resource_entries.len());
        for window in resource_entries.windows(2) {
            let (name, name_hash, offset_u32) = &window[0];
            let offset: usize = (*offset_u32).try_into().unwrap();
            let (_, _, next_offset_u32) = &window[1];
            let length = usize::try_from(next_offset_u32 - offset_u32).unwrap();
            resource_name_hash_offset_length.push((name.clone(), *name_hash, offset, length));
        }

        // 3. append the length of the last resource (assume it extends to the end of the data section)
        if let Some((last_name, last_name_hash, last_offset_u32)) = resource_entries.last() {
            let last_offset = usize::try_from(*last_offset_u32).unwrap();
            let total_data_length = data_section.len();
            let last_length = total_data_length - last_offset;
            resource_name_hash_offset_length.push((last_name.clone(), *last_name_hash, last_offset, last_length));
        }

        let mut resources: Vec<ClrResource> = Vec::with_capacity(resource_entries.len());
        for (name, name_hash, offset, length) in resource_name_hash_offset_length {
            if offset >= data_section.len() {
                error!("resource {:?} data section offset ({}) beyond resource data section (length: {})", name, offset, data_section.len());
                return Err(Error::TooShort);
            }
            if offset + length > data_section.len() {
                error!(
                    "resource {:?} data section offset ({}) + length ({}; total {}) beyond resource data section (length: {})",
                    name, offset, length, offset + length, data_section.len(),
                );
                return Err(Error::TooShort);
            }
            let resource_data = &data_section[offset..offset+length];

            // we start with a variable-length type index
            let (actual_data, type_index) = take_variable_length_integer(resource_data)
                .inspect_err(|_| error!("invalid type index for resource {:?}", name))?;

            let resource_type = ResourceType::try_from(type_index)
                .map_err(|_| Error::InvalidTypeIndex { obtained: type_index })
                .inspect_err(|_| error!("unknown type index value {} for resource {:?}", type_index, name))?;
            resources.push(ClrResource {
                name,
                name_hash,
                resource_type,
                data: actual_data.to_vec().into(),
            });
        }

        let ret = Self {
            reader_count,
            reader_assembly,
            reader_type,
            version,
            custom_resource_types,
            resources,
        };
        Ok((&[], ret))
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ClrResource {
    pub name: String,
    pub name_hash: u32,
    pub resource_type: ResourceType,
    pub data: DisplayBytesVec,
}

fn utf16_le_bytes_to_string(slice: &[u8]) -> Result<String, Error> {
    if slice.len() % 2 != 0 {
        // invalid UTF-16
        return Err(Error::InvalidText);
    }
    let words: Vec<u16> = slice
        .chunks(2)
        .map(|ch| u16::from_le_byte_slice(ch))
        .collect();
    String::from_utf16(&words)
        .map_err(|_| Error::InvalidText)
}


fn take_variable_length_integer(slice: &[u8]) -> Result<(&[u8], u64), Error> {
    // each byte is 7 bits of length + top bit stating whether length continues into next byte;
    // this procedure is used to encode 32-bit integers;
    // the length can therefore only occupy up to 5 bytes (encoding a 35-bit integer)
    let mut rest = slice;
    if rest.len() < 1 {
        return Err(Error::TooShort);
    }

    let mut length_u64: u64 = 0;
    for i in 0..=5 {
        if i == 5 {
            // max. 5 length bytes (i in 0..=4) are allowed
            return Err(Error::VariableLengthIntegerLength { max_size: 5 });
        }
        if rest.len() < 1 {
            return Err(Error::TooShort);
        }
        let b = rest[0];
        rest = &rest[1..];
        if i > 0 && b == 0b0000_0000 {
            // last length byte, not the first length byte, but all bits are 0
            // => "encode length using the shortest representation" rule violated
            return Err(Error::VariableLengthIntegerNotMinimal);
        }
        // shift it into the length;
        // the length is represented in little-endian
        // (the first byte always contains bits 0-7)
        // so we can't do the traditional Horner Scheme (len <<= 7; len |= b;)
        length_u64 |= u64::from(b & 0b0111_1111) << (7 * i);
        if b & 0b1000_0000 == 0 {
            // that was the last length byte
            break;
        }
    }

    Ok((rest, length_u64))
}


fn take_length_prefixed_bytes(slice: &[u8]) -> Result<(&[u8], &[u8]), Error> {
    let (rest, length_u64) = take_variable_length_integer(slice)?;
    let length: usize = length_u64.try_into().unwrap();
    if rest.len() < length {
        return Err(Error::TooShort);
    }
    let (slice, new_rest) = rest.split_at(length);
    Ok((new_rest, slice))
}


pub fn collect_wrapped_resource_containers(slice: &[u8]) -> Vec<Vec<u8>> {
    let mut rest = slice;
    let mut containers = Vec::new();
    while rest.len() >= 4 {
        let length_u32 = u32::from_le_bytes(rest[0..4].try_into().unwrap());
        rest = &rest[4..];

        let length: usize = length_u32.try_into().unwrap();
        if rest.len() < length {
            break;
        }
        containers.push(rest[..length].to_vec());
        rest = &rest[length..];

        // padding to u32
        let padding = (4 - (length % 4)) % 4;
        rest = &rest[padding..];
    }
    containers
}
