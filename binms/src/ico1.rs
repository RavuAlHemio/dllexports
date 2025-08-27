//! The original Windows 1.0 icon format.

use std::fmt;


#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Error {
    UnknownIndicator(u16),
    TooShort,
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            Self::UnknownIndicator(i)
                => write!(f, "unknown bitmap indicator {:#06X}", i),
            Self::TooShort
                => write!(f, "icon data too short"),
        }
    }
}
impl std::error::Error for Error {
}


/// A Windows 1.0 icon.
#[derive(Clone, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Icon1 {
    // indicator: u16, // 0x0001=device-independent, 0x0101=device-dependent, 0x0201=both
    pub device_independent: Option<IconData>,
    pub device_dependent: Option<IconData>,
}
impl Icon1 {
    pub fn take_from_bytes(bytes: &[u8]) -> Result<(&[u8], Self), Error> {
        let rest = bytes;

        if rest.len() < 2 {
            return Err(Error::TooShort);
        }
        let indicator = u16::from_le_bytes(rest[0..2].try_into().unwrap());
        let rest = &rest[2..];

        match indicator {
            0x0001 => {
                // device-independent format only
                let (rest, di) = IconData::take_from_bytes(rest)?;
                let icon = Self {
                    device_independent: Some(di),
                    device_dependent: None,
                };
                Ok((rest, icon))
            },
            0x0101 => {
                // device-dependent format only
                let (rest, dd) = IconData::take_from_bytes(rest)?;
                let icon = Self {
                    device_independent: None,
                    device_dependent: Some(dd),
                };
                Ok((rest, icon))
            },
            0x0201 => {
                // both
                let (rest, di) = IconData::take_from_bytes(rest)?;
                let (rest, dd) = IconData::take_from_bytes(rest)?;
                let icon = Self {
                    device_independent: Some(di),
                    device_dependent: Some(dd),
                };
                Ok((rest, icon))
            },
            other => Err(Error::UnknownIndicator(other)),
        }
    }
}

/// The icon data.
#[derive(Clone, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct IconData {
    pub cursor_hotspot_x: u16,
    pub cursor_hotspot_y: u16,
    pub width_pixels: u16,
    pub height_pixels: u16,
    pub width_bytes: u16, // AKA "stride"
    pub cursor_color: u16,
    pub and_bytes: Vec<u8>,
    pub xor_bytes: Vec<u8>,
}
impl IconData {
    pub fn take_from_bytes(bytes: &[u8]) -> Result<(&[u8], Self), Error> {
        let rest = bytes;

        if rest.len() < 12 {
            return Err(Error::TooShort);
        }
        let cursor_hotspot_x = u16::from_le_bytes(rest[0..2].try_into().unwrap());
        let cursor_hotspot_y = u16::from_le_bytes(rest[2..4].try_into().unwrap());
        let width_pixels = u16::from_le_bytes(rest[4..6].try_into().unwrap());
        let height_pixels = u16::from_le_bytes(rest[6..8].try_into().unwrap());
        let width_bytes = u16::from_le_bytes(rest[8..10].try_into().unwrap());
        let cursor_color = u16::from_le_bytes(rest[10..12].try_into().unwrap());
        let rest = &rest[12..];

        let pixel_byte_count = usize::try_from(width_bytes).unwrap() * usize::try_from(height_pixels).unwrap();

        if rest.len() < pixel_byte_count {
            return Err(Error::TooShort);
        }
        let and_bytes = rest[..pixel_byte_count].to_vec();
        let rest = &rest[pixel_byte_count..];

        if rest.len() < pixel_byte_count {
            return Err(Error::TooShort);
        }
        let xor_bytes = rest[..pixel_byte_count].to_vec();
        let rest = &rest[pixel_byte_count..];

        let data = IconData {
            cursor_hotspot_x,
            cursor_hotspot_y,
            width_pixels,
            height_pixels,
            width_bytes,
            cursor_color,
            and_bytes,
            xor_bytes,
        };
        Ok((rest, data))
    }

    fn bytes_as_sixels(&self, bytes: &[u8]) -> String {
        let mut ret = String::new();

        // enter sixel mode
        ret.push_str("\u{1B}Pq");

        // take six rows at a time
        let width_bytes = usize::try_from(self.width_bytes).unwrap();
        for chunk in bytes.chunks(6 * width_bytes) {
            for x in 0..width_bytes {
                // bits in the bitmap are packed MSB-to-LSB
                for bit in (0..8).rev() {
                    let mut sixel_byte = 0u8;
                    for y in 0..6 {
                        let chunk_byte_index = y * width_bytes + x;
                        let byte = if chunk_byte_index >= chunk.len() {
                            0x00
                        } else {
                            chunk[chunk_byte_index]
                        };
                        if byte & (1 << bit) != 0 {
                            sixel_byte |= 1 << y;
                        }
                    }
                    let sixel_char = char::from_u32(u32::from(sixel_byte + 63)).unwrap();
                    ret.push(sixel_char);
                }
            }

            // CRLF
            ret.push_str("$-");
        }

        // exit sixel mode
        ret.push_str("\u{1B}\\");

        ret
    }

    pub fn and_bytes_as_sixels(&self) -> String {
        self.bytes_as_sixels(&self.and_bytes)
    }

    pub fn xor_bytes_as_sixels(&self) -> String {
        self.bytes_as_sixels(&self.xor_bytes)
    }
}
