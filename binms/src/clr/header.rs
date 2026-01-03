use bitflags::bitflags;

use crate::clr::{AddressAndLength32, Error};
use crate::int_from_byte_slice::IntFromByteSlice;


bitflags! {
    #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    pub struct ClrFlags : u32 {
        const CLR_ONLY = 0b0000_0001;
    }
}


#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ClrHeader {
    // size: u32,
    pub runtime_version_major: u16,
    pub runtime_version_minor: u16,
    pub metadata_range: AddressAndLength32, // 64
    pub flags: ClrFlags,
    pub entry_point_token: u32,
    pub resources_range: AddressAndLength32, // 64
    pub strong_name_signature_range: AddressAndLength32, // 64
    pub code_manager_table_range: AddressAndLength32, // 64
    pub v_table_fixups_range: AddressAndLength32, // 64
    pub export_address_table_jumps_range: AddressAndLength32, // 64
    pub managed_native_header_range: AddressAndLength32, // 64
}
impl ClrHeader {
    pub fn take_from_bytes(bytes: &[u8]) -> Result<(&[u8], Self), Error> {
        let rest = bytes;

        if rest.len() < 4 {
            return Err(Error::TooShort);
        }
        let header_size = u32::from_le_bytes(rest[0..4].try_into().unwrap());
        let rest = &rest[4..];

        if header_size < 24 {
            return Err(Error::Size { expected_at_least: 48, obtained: header_size });
        }

        let rest_at_least = usize::try_from(header_size - 4).unwrap();
        if rest.len() < rest_at_least {
            return Err(Error::TooShort);
        }

        let runtime_version_major = u16::from_le_byte_slice(&rest[0..2]);
        let runtime_version_minor = u16::from_le_byte_slice(&rest[2..4]);
        let (_, metadata_range) = AddressAndLength32::take_from_bytes(&rest[4..12])?;
        let flags = ClrFlags::from_bits_retain(u32::from_le_byte_slice(&rest[12..16]));
        let entry_point_token = u32::from_le_byte_slice(&rest[16..20]);
        let rest = &rest[20..];

        let (rest, resources_range) = if rest.len() >= 8 {
            AddressAndLength32::take_from_bytes(&rest[0..8])?
        } else {
            (rest, AddressAndLength32::default())
        };
        let (rest, strong_name_signature_range) = if rest.len() >= 8 {
            AddressAndLength32::take_from_bytes(&rest[0..8])?
        } else {
            (rest, AddressAndLength32::default())
        };
        let (rest, code_manager_table_range) = if rest.len() >= 8 {
            AddressAndLength32::take_from_bytes(&rest[0..8])?
        } else {
            (rest, AddressAndLength32::default())
        };
        let (rest, v_table_fixups_range) = if rest.len() >= 8 {
            AddressAndLength32::take_from_bytes(&rest[0..8])?
        } else {
            (rest, AddressAndLength32::default())
        };
        let (rest, export_address_table_jumps_range) = if rest.len() >= 8 {
            AddressAndLength32::take_from_bytes(&rest[0..8])?
        } else {
            (rest, AddressAndLength32::default())
        };
        let (rest, managed_native_header_range) = if rest.len() >= 8 {
            AddressAndLength32::take_from_bytes(&rest[0..8])?
        } else {
            (rest, AddressAndLength32::default())
        };

        let header = Self {
            runtime_version_major,
            runtime_version_minor,
            metadata_range,
            flags,
            entry_point_token,
            resources_range,
            strong_name_signature_range,
            code_manager_table_range,
            v_table_fixups_range,
            export_address_table_jumps_range,
            managed_native_header_range,
        };
        Ok((rest, header))
    }
}
