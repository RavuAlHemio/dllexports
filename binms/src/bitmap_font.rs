use std::fmt;

use bitflags::bitflags;

use crate::part_int::{U3, U4};
use crate::define_part_int_enum;


#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Error {
    TooShort,
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooShort
                => write!(f, "not enough bytes"),
        }
    }
}
impl std::error::Error for Error {
}


#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Font {
    pub version: u16,
    pub size: u32,
    pub copyright: Vec<u8>, // [u8; 60]
    pub font_type: TypeFlags, // u16
    pub point_size: u16,
    pub vertical_dpi: u16,
    pub horizontal_dpi: u16,
    pub ascent: u16,
    pub internal_leading: u16,
    pub external_leading: u16,
    pub italic: Italic, // u8
    pub underline: Underline, // u8
    pub strike_out: StrikeOut, // u8
    pub weight: u16,
    pub char_set: u8,
    pub pixel_width: u16,
    pub pixel_height: u16,
    pub pitch_and_family: PitchAndFamily, // u8
    pub average_width: u16,
    pub max_width: u16,
    pub first_char: u8,
    pub last_char: u8,
    pub default_char: u8,
    pub break_char_relative: u8, // relative to first_char
    pub bytes_per_row: u16,
    pub device_name_offset: u32,
    pub name_offset: u32,
    pub bits_pointer: u32,
    pub bits_offset: u32,
    pub widths_offset: u32,
    pub device_name: Vec<u8>, // NUL-terminated string at at device_name_offset
    pub name: Vec<u8>, // NUL-terminated string at at name_offset
    pub bitmap: Vec<u16>, // pixel_height * bytes_per_row / 2
    pub widths: Vec<u16>, // [u16; (dfLastChar - dfFirstChar) + 2]
}
impl Font {
    pub fn take_from_bytes(bytes: &[u8]) -> Result<(&[u8], Self), Error> {
        if bytes.len() < 121 {
            return Err(Error::TooShort);
        }

        todo!();
    }
}

macro_rules! bottom_bit {
    ($struct_name:ident, $flag_name:ident) => {
        bitflags! {
            #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
            #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
            pub struct $struct_name : u8 {
                const $flag_name = 0x0001;
            }
        }
    };
}

bitflags! {
    #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct TypeFlags : u16 {
        const VECTOR = 0x0001;
        const AT_OFFSET = 0x0004;
        const REALIZED_BY_DEVICE = 0x0080;
    }
}

bottom_bit!(Italic, ITALIC);
bottom_bit!(Underline, UNDERLINE);
bottom_bit!(StrikeOut, STRIKE_OUT);


#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct PitchAndFamily {
    pub variable_pitch: bool,
    pub more_bits: U3,
    pub family: Family,
}
impl From<u8> for PitchAndFamily {
    fn from(value: u8) -> Self {
        let variable_pitch = ((value >> 0) & 0b0001) == 0;
        let more_bits = U3::from_base_type((value >> 1) & 0b0111).unwrap();
        let family = Family::from(U4::from_base_type((value >> 4) & 0b1111).unwrap());
        Self {
            variable_pitch,
            more_bits,
            family,
        }
    }
}
impl From<PitchAndFamily> for u8 {
    fn from(value: PitchAndFamily) -> Self {
        let b = if value.variable_pitch { 0b0001 } else { 0b0000 }
            | (value.more_bits.as_base_type() << 1)
            | (U4::from(value.family).as_base_type() << 4);
        b
    }
}

define_part_int_enum!(
    Family, U4,
    0 => "DontCare",
    1 => "Roman",
    2 => "Swiss",
    3 => "Modern",
    4 => "Script",
    5 => "Decorative",
);
