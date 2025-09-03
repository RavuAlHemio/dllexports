use std::fmt;

use bitflags::bitflags;
use display_bytes::DisplayBytesVec;

use crate::{collect_nul_terminated_ascii_string, define_part_int_enum};
use crate::part_int::{U3, U4};


#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum OffsetType {
    DeviceName,
    Name,
    Bits,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum StringType {
    DeviceName,
    Name,
}


#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Error {
    TooShort,
    UnsupportedVersion { obtained: u16 },
    SizeTooSmall { obtained: u32, minimum: u32 },
    OffsetBeyondEnd { offset_type: OffsetType, obtained: usize, font_size: usize },
    InvalidUtf8String { string_type: StringType },
    LastCharBeforeFirstChar { last_char: u8, first_char: u8 },
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooShort
                => write!(f, "not enough bytes"),
            Self::UnsupportedVersion { obtained }
                => write!(f, "unsupported font format version {:#06X}", obtained),
            Self::SizeTooSmall { obtained, minimum }
                => write!(f, "size in header is too small (obtained {}, expected at least {})", obtained, minimum),
            Self::OffsetBeyondEnd { offset_type, obtained, font_size }
                => write!(f, "{:?} offset beyond end (obtained {}, expected less than {})", offset_type, obtained, font_size),
            Self::InvalidUtf8String { string_type }
                => write!(f, "{:?} string is invalid UTF-8", string_type),
            Self::LastCharBeforeFirstChar { last_char, first_char }
                => write!(f, "last character ({:#04X}) is before first character ({:#04X})", last_char, first_char),
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
    pub copyright: DisplayBytesVec, // [u8; 60]
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
    pub version_specific: VersionSpecific,

    pub device_name: String, // NUL-terminated string at at device_name_offset
    pub name: String, // NUL-terminated string at at name_offset

    // version 1: [u8; pixel_height * bytes_per_row]
    // versions 2 and 3: [u8; char_table.map(|c| c.width_rounded_up_to_full_bytes()).sum() * pixel_height]
    // bits are always packed MSB-first
    pub bitmap: DisplayBytesVec,
}
impl Font {
    pub fn take_from_bytes(bytes: &[u8]) -> Result<(&[u8], Self), Error> {
        if bytes.len() < 117 {
            return Err(Error::TooShort);
        }

        let version = u16::from_le_bytes(bytes[0..2].try_into().unwrap());
        let size = u32::from_le_bytes(bytes[2..6].try_into().unwrap());
        let copyright = DisplayBytesVec::from(bytes[6..66].to_vec());
        let font_type = TypeFlags::from_bits_retain(u16::from_le_bytes(bytes[66..68].try_into().unwrap()));
        let point_size = u16::from_le_bytes(bytes[68..70].try_into().unwrap());
        let vertical_dpi = u16::from_le_bytes(bytes[70..72].try_into().unwrap());
        let horizontal_dpi = u16::from_le_bytes(bytes[72..74].try_into().unwrap());
        let ascent = u16::from_le_bytes(bytes[74..76].try_into().unwrap());
        let internal_leading = u16::from_le_bytes(bytes[76..78].try_into().unwrap());
        let external_leading = u16::from_le_bytes(bytes[78..80].try_into().unwrap());
        let italic = Italic::from_bits_retain(bytes[80]);
        let underline = Underline::from_bits_retain(bytes[81]);
        let strike_out = StrikeOut::from_bits_retain(bytes[82]);
        let weight = u16::from_le_bytes(bytes[83..85].try_into().unwrap());
        let char_set = bytes[85];
        let pixel_width = u16::from_le_bytes(bytes[86..88].try_into().unwrap());
        let pixel_height = u16::from_le_bytes(bytes[88..90].try_into().unwrap());
        let pitch_and_family = PitchAndFamily::from(bytes[90]);
        let average_width = u16::from_le_bytes(bytes[91..93].try_into().unwrap());
        let max_width = u16::from_le_bytes(bytes[93..95].try_into().unwrap());
        let first_char = bytes[95];
        let last_char = bytes[96];
        let default_char = bytes[97];
        let break_char_relative = bytes[98];
        let bytes_per_row = u16::from_le_bytes(bytes[99..101].try_into().unwrap());
        let device_name_offset = u32::from_le_bytes(bytes[101..105].try_into().unwrap());
        let name_offset = u32::from_le_bytes(bytes[105..109].try_into().unwrap());
        let bits_pointer = u32::from_le_bytes(bytes[109..113].try_into().unwrap());
        let bits_offset = u32::from_le_bytes(bytes[113..117].try_into().unwrap());

        // check for a plausible length
        let min_header_length: u32 = match version {
            0x0100 => 117,
            0x0200 => 118,
            0x0300 => 148,
            other => return Err(Error::UnsupportedVersion { obtained: other }),
        };
        if bytes.len() < usize::try_from(min_header_length).unwrap() {
            return Err(Error::TooShort);
        }
        if size < min_header_length {
            return Err(Error::SizeTooSmall { obtained: size, minimum: min_header_length });
        }

        let size_usize: usize = size.try_into().unwrap();
        if bytes.len() < size_usize {
            return Err(Error::TooShort);
        }

        let (font_bytes, rest) = bytes.split_at(size_usize);

        // more plausibility checks

        if last_char < first_char {
            return Err(Error::LastCharBeforeFirstChar { last_char, first_char });
        }

        let device_name_offset_usize: usize = device_name_offset.try_into().unwrap();
        let name_offset_usize: usize = name_offset.try_into().unwrap();
        let bits_offset_usize: usize = bits_offset.try_into().unwrap();

        let pairs = [
            (OffsetType::DeviceName, device_name_offset_usize),
            (OffsetType::Name, name_offset_usize),
            (OffsetType::Bits, bits_offset_usize),
        ];
        for (offset_type, offset) in pairs {
            if offset >= font_bytes.len() {
                return Err(Error::OffsetBeyondEnd {
                    offset_type,
                    obtained: offset,
                    font_size: size_usize,
                });
            }
        }

        let device_name = if device_name_offset_usize == 0 {
            String::with_capacity(0)
        } else {
            collect_nul_terminated_ascii_string(&font_bytes[device_name_offset_usize..])
                .ok_or(Error::InvalidUtf8String { string_type: StringType::DeviceName })?
        };
        let name = if name_offset_usize == 0 {
            String::with_capacity(0)
        } else {
            collect_nul_terminated_ascii_string(&font_bytes[name_offset_usize..])
                .ok_or(Error::InvalidUtf8String { string_type: StringType::Name })?
        };

        // we can calculate this for all versions
        let char_entry_count = usize::from(last_char - first_char) + 2;

        // collect the version-specific data
        let (bitmap_byte_count, version_specific) = match version {
            0x0100 => {
                // collect the bit offsets array
                const BIT_OFFSETS_OFFSET: usize = 117;
                let mut bit_offsets = Vec::with_capacity(char_entry_count);
                let offset_byte_count = char_entry_count * 2;
                for offset_chunk in font_bytes[BIT_OFFSETS_OFFSET..BIT_OFFSETS_OFFSET+offset_byte_count].chunks(2) {
                    let offset_word = u16::from_le_bytes(offset_chunk.try_into().unwrap());
                    bit_offsets.push(offset_word);
                }

                (
                    usize::from(pixel_height) * usize::from(bytes_per_row),
                    VersionSpecific::V1 { bit_offsets },
                )
            },
            0x0200|0x0300 => {
                // collect the extended header
                let reserved0 = font_bytes[117];

                // collect the character table
                match version {
                    0x0200 => {
                        const CHAR_TABLE_OFFSET: usize = 118;
                        let char_table_byte_count = char_entry_count * 4;
                        let mut char_table = Vec::with_capacity(char_entry_count);
                        for char_table_chunk in font_bytes[CHAR_TABLE_OFFSET..CHAR_TABLE_OFFSET+char_table_byte_count].chunks(4) {
                            let width = u16::from_le_bytes(char_table_chunk[0..2].try_into().unwrap());
                            let offset = u16::from_le_bytes(char_table_chunk[2..4].try_into().unwrap());
                            char_table.push(WidthOffset16 {
                                width,
                                offset,
                            });
                        }

                        let total_width_bytes: usize = char_table
                            .iter()
                            .map(|wo| {
                                let width_bits = wo.width;
                                let width_ceil_bytes = (width_bits + (8 - 1)) / 8;
                                usize::from(width_ceil_bytes)
                            })
                            .sum();

                        (
                            usize::from(pixel_height) * total_width_bytes,
                            VersionSpecific::V2 {
                                reserved0,
                                char_table,
                            },
                        )
                    },
                    0x0300 => {
                        // collect the extended header
                        let flags = Flags::from_bits_retain(u32::from_le_bytes(font_bytes[118..122].try_into().unwrap()));
                        let a_space = u16::from_le_bytes(font_bytes[122..124].try_into().unwrap());
                        let b_space = u16::from_le_bytes(font_bytes[124..126].try_into().unwrap());
                        let c_space = u16::from_le_bytes(font_bytes[126..128].try_into().unwrap());
                        let color_pointer = u32::from_le_bytes(font_bytes[128..132].try_into().unwrap());
                        let reserved1: [u8; 16] = font_bytes[132..148].try_into().unwrap();
                        let ext_header = V3ExtHeader {
                            reserved0,
                            flags,
                            a_space,
                            b_space,
                            c_space,
                            color_pointer,
                            reserved1,
                        };

                        const CHAR_TABLE_OFFSET: usize = 148;
                        let char_table_byte_count = char_entry_count * 6;
                        let mut char_table = Vec::with_capacity(char_entry_count);
                        for char_table_chunk in font_bytes[CHAR_TABLE_OFFSET..CHAR_TABLE_OFFSET+char_table_byte_count].chunks(6) {
                            let width = u16::from_le_bytes(char_table_chunk[0..2].try_into().unwrap());
                            let offset = u32::from_le_bytes(char_table_chunk[2..6].try_into().unwrap());
                            char_table.push(WidthOffset32 {
                                width,
                                offset,
                            });
                        }

                        let total_width_bytes: usize = char_table
                            .iter()
                            .map(|wo| {
                                let width_bits = wo.width;
                                let width_ceil_bytes = (width_bits + (8 - 1)) / 8;
                                usize::from(width_ceil_bytes)
                            })
                            .sum();

                        (
                            usize::from(pixel_height) * total_width_bytes,
                            VersionSpecific::V3 {
                                ext_header,
                                char_table,
                            },
                        )
                    },
                    _ => unreachable!(),
                }
            },
            _ => unreachable!(),
        };

        let bitmap: DisplayBytesVec = font_bytes[bits_offset_usize..bits_offset_usize+bitmap_byte_count]
            .to_owned()
            .into();

        let font = Self {
            version,
            size,
            copyright,
            font_type,
            point_size,
            vertical_dpi,
            horizontal_dpi,
            ascent,
            internal_leading,
            external_leading,
            italic,
            underline,
            strike_out,
            weight,
            char_set,
            pixel_width,
            pixel_height,
            pitch_and_family,
            average_width,
            max_width,
            first_char,
            last_char,
            default_char,
            break_char_relative,
            bytes_per_row,
            device_name_offset,
            name_offset,
            bits_pointer,
            bits_offset,
            version_specific,
            device_name,
            name,
            bitmap,
        };

        Ok((rest, font))
    }

    fn transpose_bytes(bytes: &[u8], width_bytes: usize, pixel_height: usize) -> Vec<u8> {
        // multi-byte characters must be transposed
        // because V2/V3 encodes characters as such:
        // first, all rows of the first 8-pixel column are output
        // then the rows of the next 8-pixel column
        // etc.
        let mut transposed = vec![0u8; bytes.len()];

        for column_index in 0..width_bytes {
            for row_index in 0..pixel_height {
                let source_index = column_index * usize::from(pixel_height) + row_index;
                let target_index = row_index * width_bytes + column_index;
                transposed[target_index] = bytes[source_index];
            }
        }

        transposed
    }

    pub fn to_bdf(&self) -> String {
        use std::fmt::Write as _;

        let mut ret = String::new();

        let char_count = (self.last_char - self.first_char) + 1;

        let max_width = (0..usize::from(char_count))
            .map(|i| self.version_specific.char_width_at(i))
            .max()
            .unwrap_or(0);

        writeln!(ret, "STARTFONT 2.1").unwrap();
        writeln!(ret, "FONT {}", self.name).unwrap();
        writeln!(ret, "SIZE {} {} {}", self.point_size, self.pixel_width, self.pixel_height).unwrap();
        writeln!(ret, "FONTBOUNDINGBOX {} {} 0 0", max_width, self.pixel_height).unwrap();
        writeln!(ret, "STARTPROPERTIES 2").unwrap();
        writeln!(ret, "FONT_ASCENT {}", self.ascent).unwrap();
        writeln!(ret, "FONT_DESCENT {}", self.pixel_height - self.ascent).unwrap();
        writeln!(ret, "ENDPROPERTIES").unwrap();

        writeln!(ret, "CHARS {}", char_count).unwrap();

        for char_index in 0..usize::from(char_count) {
            let char_code_point = usize::from(self.first_char) + char_index;
            writeln!(ret, "STARTCHAR U+{:04X}", char_code_point).unwrap();
            writeln!(ret, "ENCODING {}", char_code_point).unwrap();

            // pixels = (afm_width / 1000) * (resolution / 72)
            // pixels = (afm_width * resolution) / 72000
            // 72000 * pixels = afm_width * resolution
            // (72000 * pixels) / resolution = afm_width
            let char_pixel_width = match &self.version_specific {
                VersionSpecific::V1 { bit_offsets, .. } => {
                    bit_offsets[char_index + 1] - bit_offsets[char_index]
                },
                VersionSpecific::V2 { char_table, .. } => {
                    char_table[char_index].width
                },
                VersionSpecific::V3 { char_table, .. } => {
                    char_table[char_index].width
                },
            };
            let denominator = if self.pixel_width == 0 { 72 } else { u64::from(self.pixel_width) };
            let char_afm_width = 72000 * u64::from(char_pixel_width) / denominator;
            writeln!(ret, "SWIDTH {} 0", char_afm_width).unwrap();
            writeln!(ret, "DWIDTH {} 0", char_pixel_width).unwrap();
            writeln!(ret, "BBX {} {} 0 0", char_pixel_width, self.pixel_height).unwrap();
            writeln!(ret, "BITMAP").unwrap();

            // read the character
            match &self.version_specific {
                VersionSpecific::V1 { bit_offsets, .. } => {
                    let row_length_bytes = usize::from(self.bytes_per_row);
                    let bit_offset = usize::from(bit_offsets[char_index]);
                    let width = usize::from(bit_offsets[char_index+1]) - bit_offset;

                    let bitmap: &[u8] = self.bitmap.as_ref();
                    for row in bitmap.chunks(row_length_bytes) {
                        let char_bytes = row
                            .iter()
                            .copied()
                            .bytes_to_bits()
                            .skip(bit_offset)
                            .take(width)
                            .bits_to_bytes();
                        for byte in char_bytes {
                            write!(ret, "{:02X}", byte).unwrap();
                        }
                        writeln!(ret).unwrap();
                    }
                },
                VersionSpecific::V2 { char_table, .. } => {
                    let width_bytes = (usize::from(char_table[char_index].width) + (8 - 1)) / 8;
                    let bitmap_offset = usize::from(char_table[char_index].offset) - usize::try_from(self.bits_offset).unwrap();
                    let total_bytes = width_bytes * usize::from(self.pixel_height);

                    let bitmap: &[u8] = self.bitmap.as_ref();
                    let slice = &bitmap[bitmap_offset..bitmap_offset+total_bytes];
                    let transposed = Self::transpose_bytes(slice, width_bytes, self.pixel_height.into());

                    for row in transposed.chunks(width_bytes) {
                        for byte in row.iter().copied() {
                            write!(ret, "{:02X}", byte).unwrap();
                        }
                        writeln!(ret).unwrap();
                    }
                },
                VersionSpecific::V3 { char_table, .. } => {
                    let width_bytes = (usize::from(char_table[char_index].width) + (8 - 1)) / 8;
                    let bitmap_offset = usize::try_from(char_table[char_index].offset).unwrap() - usize::try_from(self.bits_offset).unwrap();
                    let total_bytes = width_bytes * usize::from(self.pixel_height);

                    let bitmap: &[u8] = self.bitmap.as_ref();
                    let slice = &bitmap[bitmap_offset..bitmap_offset+total_bytes];
                    let transposed = Self::transpose_bytes(slice, width_bytes, self.pixel_height.into());

                    for row in transposed.chunks(width_bytes) {
                        for byte in row.iter().copied() {
                            write!(ret, "{:02X}", byte).unwrap();
                        }
                        writeln!(ret).unwrap();
                    }
                },
            };

            writeln!(ret, "ENDCHAR").unwrap();
        }

        writeln!(ret, "ENDFONT").unwrap();
        ret
    }

    pub fn to_fd(&self) -> String {
        // font format from https://www.chiark.greenend.org.uk/~sgtatham/fonts/
        use std::fmt::Write as _;

        let mut ret = String::new();

        let char_count = (self.last_char - self.first_char) + 1;

        let copyright_string = std::str::from_utf8(self.copyright.as_ref())
            .expect("copyright string not valid")
            .trim_end_matches('\0');

        writeln!(ret, "facename {}", self.name).unwrap();
        writeln!(ret, "copyright {}", copyright_string).unwrap();
        writeln!(ret, "height {}", self.pixel_height).unwrap();
        writeln!(ret, "ascent {}", self.ascent).unwrap();
        writeln!(ret, "pointsize {}", self.point_size).unwrap();
        writeln!(ret, "italic {}", self.italic.as_fd_str()).unwrap();
        writeln!(ret, "underline {}", self.underline.as_fd_str()).unwrap();
        writeln!(ret, "strikeout {}", self.strike_out.as_fd_str()).unwrap();
        writeln!(ret, "weight {}", self.weight).unwrap();
        writeln!(ret, "charset {}", self.char_set).unwrap();

        // output empty characters at the start
        for char_index in 0..self.first_char {
            writeln!(ret).unwrap();
            writeln!(ret, "char {}", char_index).unwrap();
            writeln!(ret, "width 0").unwrap();
        }

        for char_index in 0..usize::from(char_count) {
            writeln!(ret).unwrap();
            writeln!(ret, "char {}", char_index + usize::from(self.first_char)).unwrap();

            let char_pixel_width = match &self.version_specific {
                VersionSpecific::V1 { bit_offsets, .. } => {
                    bit_offsets[char_index + 1] - bit_offsets[char_index]
                },
                VersionSpecific::V2 { char_table, .. } => {
                    char_table[char_index].width
                },
                VersionSpecific::V3 { char_table, .. } => {
                    char_table[char_index].width
                },
            };
            writeln!(ret, "width {}", char_pixel_width).unwrap();

            // read the character
            match &self.version_specific {
                VersionSpecific::V1 { bit_offsets, .. } => {
                    let row_length_bytes = usize::from(self.bytes_per_row);
                    let bit_offset = usize::from(bit_offsets[char_index]);
                    let width = usize::from(bit_offsets[char_index+1]) - bit_offset;

                    let bitmap: &[u8] = self.bitmap.as_ref();
                    for row in bitmap.chunks(row_length_bytes) {
                        let char_bits = row
                            .iter()
                            .copied()
                            .bytes_to_bits()
                            .skip(bit_offset)
                            .take(width);
                        for bit in char_bits {
                            ret.push(if bit { '1' } else { '0' });
                        }
                        writeln!(ret).unwrap();
                    }
                },
                VersionSpecific::V2 { char_table, .. } => {
                    let width_bytes = (usize::from(char_table[char_index].width) + (8 - 1)) / 8;
                    let bitmap_offset = usize::from(char_table[char_index].offset) - usize::try_from(self.bits_offset).unwrap();
                    let total_bytes = width_bytes * usize::from(self.pixel_height);

                    let bitmap: &[u8] = self.bitmap.as_ref();
                    let slice = &bitmap[bitmap_offset..bitmap_offset+total_bytes];
                    let transposed = Self::transpose_bytes(slice, width_bytes, self.pixel_height.into());

                    for row in transposed.chunks(width_bytes) {
                        for bit in row.iter().copied().bytes_to_bits() {
                            ret.push(if bit { '1' } else { '0' });
                        }
                        writeln!(ret).unwrap();
                    }
                },
                VersionSpecific::V3 { char_table, .. } => {
                    let width_bytes = (usize::from(char_table[char_index].width) + (8 - 1)) / 8;
                    let bitmap_offset = usize::try_from(char_table[char_index].offset).unwrap() - usize::try_from(self.bits_offset).unwrap();
                    let total_bytes = width_bytes * usize::from(self.pixel_height);

                    let bitmap: &[u8] = self.bitmap.as_ref();
                    let slice = &bitmap[bitmap_offset..bitmap_offset+total_bytes];
                    let transposed = Self::transpose_bytes(slice, width_bytes, self.pixel_height.into());

                    for row in transposed.chunks(width_bytes) {
                        for byte in row.iter().copied() {
                            write!(ret, "{:02X}", byte).unwrap();
                        }
                        writeln!(ret).unwrap();
                    }
                },
            };
        }

        // output empty characters at the end
        for char_index in self.last_char..=255 {
            writeln!(ret).unwrap();
            writeln!(ret, "char {}", char_index).unwrap();
            writeln!(ret, "width 0").unwrap();
        }

        ret
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum VersionSpecific {
    V1 {
        // characters are arranged horizontally
        // `bit_offsets` enumerates the bit offset of each character across a scanline
        bit_offsets: Vec<u16>, // [u16; (last_char - first_char) + 2]
    },
    V2 {
        // padding
        reserved0: u8,

        // characters are arranged vertically
        // `char_table` enumerates the width and byte offset (within the bitmap) of each character
        char_table: Vec<WidthOffset16>,
    },
    V3 {
        ext_header: V3ExtHeader,

        // same as V2, but offsets are now u32 instead of u16
        char_table: Vec<WidthOffset32>,
    }
}
impl VersionSpecific {
    pub fn char_width_at(&self, index: usize) -> u16 {
        match self {
            VersionSpecific::V1 { bit_offsets, .. } => {
                bit_offsets[index + 1] - bit_offsets[index]
            },
            VersionSpecific::V2 { char_table, .. } => {
                char_table[index].width
            },
            VersionSpecific::V3 { char_table, .. } => {
                char_table[index].width
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct V3ExtHeader {
    pub reserved0: u8,
    pub flags: Flags, // u32
    pub a_space: u16,
    pub b_space: u16,
    pub c_space: u16,
    pub color_pointer: u32,
    pub reserved1: [u8; 16],
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct WidthOffset16 {
    pub width: u16,
    pub offset: u16,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct WidthOffset32 {
    pub width: u16,
    pub offset: u32,
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
        impl $struct_name {
            pub fn as_fd_str(&self) -> &'static str {
                if self.contains(Self::$flag_name) {
                    "yes"
                } else {
                    "no"
                }
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

    #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct Flags : u32 {
        const FIXED = 0x0001;
        const PROPORTIONAL = 0x0002;
        const ABC_FIXED = 0x0004;
        const ABC_PROPORTIONAL = 0x0008;
        const COLORS_1 = 0x0010;
        const COLORS_16 = 0x0020;
        const COLORS_256 = 0x0040;
        const COLORS_RGB = 0x0080;
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
        let variable_pitch = ((value >> 0) & 0b0001) != 0;
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


struct ByteUnpacker<I: Iterator<Item = u8>> {
    iterator: I,
    current_byte: Option<u8>,
    bit_pos: u8,
}
impl<I: Iterator<Item = u8>> ByteUnpacker<I> {
    pub fn new(iterator: I) -> Self {
        Self {
            iterator,
            current_byte: None,
            bit_pos: 0,
        }
    }
}
impl<I: Iterator<Item = u8>> Iterator for ByteUnpacker<I> {
    type Item = bool;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_byte.is_none() {
            let new_byte = self.iterator.next()?;
            self.current_byte = Some(new_byte);
        }

        let shift_count = 7 - self.bit_pos;
        let value = (self.current_byte.unwrap() & (1 << shift_count)) != 0;

        self.bit_pos += 1;
        if self.bit_pos == 8 {
            // advance
            self.current_byte = None;
            self.bit_pos = 0;
        }

        Some(value)
    }
}

struct BytePacker<I: Iterator<Item = bool>> {
    iterator: I,
}
impl<I: Iterator<Item = bool>> BytePacker<I> {
    pub fn new(iterator: I) -> Self {
        Self {
            iterator,
        }
    }
}
impl<I: Iterator<Item = bool>> Iterator for BytePacker<I> {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        // take one bit, failing if that fails
        let first_bit = self.iterator.next()?;

        let mut byte: u8 = if first_bit { 1 << 7 } else { 0 };

        // take seven more bits without failing
        for i in (0..7).rev() {
            let bit = self.iterator.next().unwrap_or(false);
            if bit {
                byte |= 1 << i;
            }
        }

        // spit out the byte
        Some(byte)
    }
}

trait ByteIterExt<I: Iterator<Item = u8>> {
    fn bytes_to_bits(self) -> ByteUnpacker<I>;
}
impl<I: Iterator<Item = u8>> ByteIterExt<I> for I {
    fn bytes_to_bits(self) -> ByteUnpacker<I> {
        ByteUnpacker::new(self)
    }
}

trait BitIterExt<I: Iterator<Item = bool>> {
    fn bits_to_bytes(self) -> BytePacker<I>;
}
impl<I: Iterator<Item = bool>> BitIterExt<I> for I {
    fn bits_to_bytes(self) -> BytePacker<I> {
        BytePacker::new(self)
    }
}
