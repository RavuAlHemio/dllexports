//! Support for some formats of GDI bitmaps.
//!
//! Windows 3.0 encodes its icon resources as bitmaps based on the BITMAPINFOHEADER structure,
//! which is implemented in this file. This is sometimes known as the Windows v3 bitmap format.
//!
//! Support for the following formats is currently not implemented:
//! * BMP files (`BITMAPFILEHEADER`)
//! * Windows 1.0 icons/cursors (see [`ico1`])
//! * OS/2 v1/Windows v2 bitmaps (`BITMAPCOREHEADER`)
//! * OS/2 v2 bitmaps (`BITMAPCOREHEADER2`)
//! * Windows v4 bitmaps (`BITMAPV4HEADER`)
//! * Windows v5 bitmaps (`BITMAPV5HEADER`)
//! * other uncommon Windows bitmaps (`BITMAPV2HEADER`, `BITMAPV3HEADER`)
//!
//! There is a "bit" of a caveat for icons, though: the height in the header is twice the actual
//! size of the icon, since a second image representing the transparency bitmap is included. That,
//! in itself, might be considered a clever hack, but things get worse: independent of the bit
//! depth specified in the header, the bit depth of the transparency bitmap is always 1bpp. For
//! most images, that means the bit depth changes halfway through the image.


use std::fmt;

use from_to_repr::from_to_other;


#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ReadStage {
    HeaderLength,
    Header,
    Palette,
    Data,
}


#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Error {
    TooShort { stage: ReadStage },
    WrongHeaderSize { expected: u32, obtained: u32 },
    WrongPlaneCount { expected: u16, obtained: u16 },
    InvalidBitDepthForCompression { bit_depth: u16, compression: Compression },
    OverlyLargePalette { bit_depth: u16, color_count: u32, max_color_count: u32 },
    UnsupportedBitDepth { bit_depth: u16 },
    UnsupportedCompression { compression: Compression },
    OddHeightIcon { height: i32 },
    NonPositiveWidth { width: i32 },
    ZeroHeight,
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            Self::TooShort { stage }
                => write!(f, "data too short in read stage {:?}", stage),
            Self::WrongHeaderSize { expected, obtained }
                => write!(f, "wrong header size (expected {}, obtained {})", expected, obtained),
            Self::WrongPlaneCount { expected, obtained }
                => write!(f, "wrong plane count (expected {}, obtained {})", expected, obtained),
            Self::UnsupportedBitDepth { bit_depth }
                => write!(f, "unsupported bit depth {}", bit_depth),
            Self::InvalidBitDepthForCompression { bit_depth, compression }
                => write!(f, "bit depth {} is invalid for compression {:?}", bit_depth, compression),
            Self::OverlyLargePalette { bit_depth, color_count, max_color_count }
                => write!(f, "palette too large for bit depth {} ({} given, {} maximum)", bit_depth, color_count, max_color_count),
            Self::UnsupportedCompression { compression }
                => write!(f, "{:?} compression not supported", compression),
            Self::OddHeightIcon { height }
                => write!(f, "the image is an icon and the height ({}) is odd", height),
            Self::NonPositiveWidth { width }
                => write!(f, "the image has a width <= 0 ({})", width),
            Self::ZeroHeight
                => write!(f, "the image has a zero height"),
        }
    }
}


#[derive(Clone, Copy, Debug)]
#[from_to_other(base_type = u32, derive_compare = "as_int")]
pub enum Compression {
    Rgb = 0,
    Rle8 = 1,
    Rle4 = 2,
    BitFields = 3,
    Jpeg = 4,
    Png = 5,
    AlphaBitFields = 6,
    Other(u32),
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
enum Palette {
    /// The pixel format does not use a palette.
    ///
    /// The palette structure after the header can be assumed to have zero entries.
    #[default] No,

    /// The pixel format has a palette, but this palette only has an advisory function.
    ///
    /// Devices which support a limited number of simultaneous colors should use the palette to
    /// choose which colors to display, but the color values of the image's pixels are encoded
    /// directly. The palette has `color_count` entries.
    Advisory { color_count: usize },

    /// The pixel format uses a palette.
    ///
    /// The color values of the image's pixels are indexes into the palette, which has
    /// `color_count` entries.
    Indexed { color_count: usize },
}
impl Palette {
    pub fn color_count(&self) -> usize {
        match self {
            Self::No => 0,
            Self::Advisory { color_count } => *color_count,
            Self::Indexed { color_count } => *color_count,
        }
    }
}


#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct BitmapInfoHeader {
    pub header_size: u32, // always 40
    pub width: i32,
    pub height: i32,
    pub planes: u16, // always 1 (packed pixels)
    pub bit_count: u16,
    pub compression: Compression,
    pub size_image: u32,
    pub x_pixels_per_meter: i32,
    pub y_pixels_per_meter: i32,
    pub colors_used: u32,
    pub important_colors: u32,
}
impl BitmapInfoHeader {
    pub fn take_from_bytes(bytes: &[u8]) -> Result<(&[u8], Self), Error> {
        let rest = bytes;

        if rest.len() < 4 {
            return Err(Error::TooShort { stage: ReadStage::HeaderLength });
        }
        let header_size = u32::from_le_bytes(rest[0..4].try_into().unwrap());
        let rest = &rest[4..];

        if header_size != 40 {
            return Err(Error::WrongHeaderSize { expected: 40, obtained: header_size });
        }

        if rest.len() < 36 {
            return Err(Error::TooShort { stage: ReadStage::Header });
        }

        let width = i32::from_le_bytes(rest[0..4].try_into().unwrap());
        let height = i32::from_le_bytes(rest[4..8].try_into().unwrap());
        let planes = u16::from_le_bytes(rest[8..10].try_into().unwrap());
        let bit_count = u16::from_le_bytes(rest[10..12].try_into().unwrap());
        let compression_u32 = u32::from_le_bytes(rest[12..16].try_into().unwrap());
        let size_image = u32::from_le_bytes(rest[16..20].try_into().unwrap());
        let x_pixels_per_meter = i32::from_le_bytes(rest[20..24].try_into().unwrap());
        let y_pixels_per_meter = i32::from_le_bytes(rest[24..28].try_into().unwrap());
        let colors_used = u32::from_le_bytes(rest[28..32].try_into().unwrap());
        let important_colors = u32::from_le_bytes(rest[32..36].try_into().unwrap());
        let rest = &rest[36..];

        if planes != 1 {
            return Err(Error::WrongPlaneCount { expected: 1, obtained: planes });
        }
        if width <= 0 {
            return Err(Error::NonPositiveWidth { width });
        }
        if height == 0 {
            return Err(Error::ZeroHeight);
        }

        let compression = Compression::from_base_type(compression_u32);
        let header = Self {
            header_size,
            width,
            height,
            planes,
            bit_count,
            compression,
            size_image,
            x_pixels_per_meter,
            y_pixels_per_meter,
            colors_used,
            important_colors,
        };
        Ok((rest, header))
    }
}


#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Bitmap {
    pub header: BitmapInfoHeader,
    pub palette: Option<Vec<u8>>,
    pub data: Vec<u8>,
    pub transparency: Option<Vec<u8>>,
}
impl Bitmap {
    pub fn take_from_bytes(bytes: &[u8], is_icon: bool) -> Result<(&[u8], Self), Error> {
        let (rest, header) = BitmapInfoHeader::take_from_bytes(bytes)?;

        if header.compression != Compression::Rgb {
            return Err(Error::UnsupportedCompression { compression: header.compression });
        }

        let palette_type = match header.bit_count {
            0 => {
                // bit depth is defined by embedded JPEG or PNG file
                if header.compression != Compression::Jpeg && header.compression != Compression::Png {
                    return Err(Error::InvalidBitDepthForCompression {
                        bit_depth: header.bit_count,
                        compression: header.compression,
                    });
                }

                Palette::No
            },
            1 => {
                if header.compression == Compression::BitFields {
                    return Err(Error::InvalidBitDepthForCompression {
                        bit_depth: header.bit_count,
                        compression: header.compression,
                    });
                }

                // 1-bit image with 2 palette colors
                if header.colors_used == 0 {
                    Palette::Indexed { color_count: 2 }
                } else if header.colors_used > 2 {
                    return Err(Error::OverlyLargePalette {
                        bit_depth: header.bit_count,
                        color_count: header.colors_used,
                        max_color_count: 2,
                    });
                } else {
                    Palette::Indexed { color_count: header.colors_used.try_into().unwrap() }
                }
            },
            4 => {
                if header.compression == Compression::BitFields {
                    return Err(Error::InvalidBitDepthForCompression {
                        bit_depth: header.bit_count,
                        compression: header.compression,
                    });
                }

                // 4-bit image with up to 16 palette colors
                if header.colors_used == 0 {
                    Palette::Indexed { color_count: 16 }
                } else if header.colors_used > 16 {
                    return Err(Error::OverlyLargePalette {
                        bit_depth: header.bit_count,
                        color_count: header.colors_used,
                        max_color_count: 16,
                    });
                } else {
                    Palette::Indexed { color_count: header.colors_used.try_into().unwrap() }
                }
            },
            8 => {
                if header.compression == Compression::BitFields {
                    return Err(Error::InvalidBitDepthForCompression {
                        bit_depth: header.bit_count,
                        compression: header.compression,
                    });
                }

                // 8-bit image with up to 256 palette colors
                if header.colors_used == 0 {
                    Palette::Indexed { color_count: 256 }
                } else if header.colors_used > 256 {
                    return Err(Error::OverlyLargePalette {
                        bit_depth: header.bit_count,
                        color_count: header.colors_used,
                        max_color_count: 256,
                    });
                } else {
                    Palette::Indexed { color_count: header.colors_used.try_into().unwrap() }
                }
            },
            16 => {
                if header.compression == Compression::BitFields {
                    // valid but we don't support it
                    return Err(Error::UnsupportedCompression { compression: header.compression });
                }

                // 16-bit RGB555 image: xrrrrrgg_gggbbbbb
                // (x = don't care)
                // palette is advisory-only
                if header.colors_used > 0x1_0000 {
                    return Err(Error::OverlyLargePalette {
                        bit_depth: header.bit_count,
                        color_count: header.colors_used,
                        max_color_count: 0x1_0000,
                    });
                }

                Palette::Advisory { color_count: header.colors_used.try_into().unwrap() }
            },
            24 => {
                if header.compression == Compression::BitFields {
                    return Err(Error::InvalidBitDepthForCompression {
                        bit_depth: header.bit_count,
                        compression: header.compression,
                    });
                }

                // 24-bit image: bbbbbbbb gggggggg rrrrrrrr
                // (x = don't care)
                // palette is advisory-only

                Palette::Advisory { color_count: header.colors_used.try_into().unwrap() }
            },
            32 => {
                if header.compression == Compression::BitFields {
                    // valid but we don't support it
                    return Err(Error::UnsupportedCompression {
                        compression: header.compression,
                    });
                }

                // 32-bit image: 00000000_bbbbbbbb_gggggggg_rrrrrrrr
                // (x = don't care)
                // palette is advisory-only

                Palette::Advisory { color_count: header.colors_used.try_into().unwrap() }
            },
            other => return Err(Error::UnsupportedBitDepth {
                bit_depth: other,
            }),
        };

        // read the palette
        let (rest, palette) = match palette_type {
            Palette::No => {
                // well that's boring
                (rest, None)
            },
            Palette::Advisory { color_count } => {
                // just gloss over it
                let palette_byte_count = usize::try_from(color_count).unwrap() * 4;
                if rest.len() < palette_byte_count {
                    return Err(Error::TooShort { stage: ReadStage::Palette });
                }
                (&rest[palette_byte_count..], None)
            },
            Palette::Indexed { color_count } => {
                // assemble the palette in PNG order
                let color_count_usize = usize::try_from(color_count).unwrap();
                let palette_byte_count = color_count_usize * 4;
                if rest.len() < palette_byte_count {
                    return Err(Error::TooShort { stage: ReadStage::Palette });
                }

                let mut palette = Vec::with_capacity(color_count_usize * 3);
                for color_bytes in rest[..palette_byte_count].chunks(4) {
                    // color is in 0RGB order
                    let color = u32::from_le_bytes(color_bytes.try_into().unwrap());
                    palette.push(u8::try_from((color >> 16) & 0xFF).unwrap());
                    palette.push(u8::try_from((color >>  8) & 0xFF).unwrap());
                    palette.push(u8::try_from((color >>  0) & 0xFF).unwrap());
                }

                (&rest[palette_byte_count..], Some(palette))
            },
        };

        // good, now let's calculate the stride
        let width_usize: usize = header.width.try_into().unwrap();
        let bit_count_usize: usize = header.bit_count.try_into().unwrap();
        let bits_per_row = width_usize * bit_count_usize;
        let stride_bits = round_up_usize(bits_per_row, 32);
        let stride_bytes = stride_bits / 8;

        // check how many bytes we actually need per row
        let min_bits_per_row = round_up_usize(bits_per_row, 8);
        let min_bytes_per_row = min_bits_per_row / 8;

        // finally, normalize the height
        let (raw_height_usize, flip): (usize, bool) = if header.height < 0 {
            // top-to-bottom image
            ((-header.height).try_into().unwrap(), false)
        } else {
            // bottom-to-top image
            (header.height.try_into().unwrap(), true)
        };

        let (height_usize, data_byte_count, alpha_byte_count) = if is_icon {
            // great, we get to jump through some fun hoops

            // the icon is only half the height
            if raw_height_usize % 2 == 1 {
                return Err(Error::OddHeightIcon { height: header.height });
            }
            let height_usize = raw_height_usize / 2;
            let data_byte_count = stride_bytes * height_usize;

            // and followed by a 1bpp transparency bitmap
            let alpha_bits_per_row = width_usize * 1;
            let alpha_stride_bits = round_up_usize(alpha_bits_per_row, 32);
            let alpha_stride_bytes = alpha_stride_bits / 8;

            let alpha_byte_count = alpha_stride_bytes * height_usize;

            // enough bytes?
            if rest.len() < data_byte_count + alpha_byte_count {
                return Err(Error::TooShort { stage: ReadStage::Data });
            }

            (height_usize, data_byte_count, Some(alpha_byte_count))
        } else {
            let height_usize = raw_height_usize / 2;
            let data_byte_count = stride_bytes * height_usize;

            // enough bytes?
            if rest.len() < data_byte_count {
                return Err(Error::TooShort { stage: ReadStage::Data });
            }

            (height_usize, data_byte_count, None)
        };

        let (data_bytes, rest) = rest.split_at(data_byte_count);

        // go for it
        let mut rows: Vec<Vec<u8>> = Vec::with_capacity(height_usize);
        for in_row in data_bytes.chunks(stride_bytes) {
            let mut out_row: Vec<u8> = Vec::with_capacity(min_bytes_per_row);
            match header.bit_count {
                1|4|8 => {
                    // we can copy the bytes verbatim, they are MSB-first palette indexes
                    out_row.extend(&in_row[..min_bytes_per_row]);
                },
                16 => {
                    // take two bytes at a time and expand 555 to 888
                    // (PNG doesn't natively support 555)
                    for word_bytes in in_row.chunks(2) {
                        let word = u16::from_le_bytes(word_bytes.try_into().unwrap());
                        let r5: u8 = ((word >> 10) & 0b1_1111).try_into().unwrap();
                        let g5: u8 = ((word >>  5) & 0b1_1111).try_into().unwrap();
                        let b5: u8 = ((word >>  0) & 0b1_1111).try_into().unwrap();
                        let r8 = scale_u5_to_u8(r5);
                        let g8 = scale_u5_to_u8(g5);
                        let b8 = scale_u5_to_u8(b5);
                        out_row.push(r8);
                        out_row.push(g8);
                        out_row.push(b8);
                    }
                },
                24 => {
                    // take three bytes at a time and reverse them from BGR to RGB
                    for bytes in in_row.chunks(3) {
                        out_row.push(bytes[2]);
                        out_row.push(bytes[1]);
                        out_row.push(bytes[0]);
                    }
                },
                32 => {
                    // take four bytes at a time and pick them apart
                    for dword_bytes in in_row.chunks(4) {
                        // 0BGR
                        let dword = u32::from_le_bytes(dword_bytes.try_into().unwrap());
                        let r: u8 = ((dword >>  0) & 0b1111_1111).try_into().unwrap();
                        let g: u8 = ((dword >>  8) & 0b1111_1111).try_into().unwrap();
                        let b: u8 = ((dword >> 16) & 0b1111_1111).try_into().unwrap();
                        out_row.push(r);
                        out_row.push(g);
                        out_row.push(b);
                    }
                },
                _ => unreachable!(),
            }
            rows.push(out_row);
        }

        if flip {
            rows.reverse();
        }

        let final_byte_count: usize = rows.iter().map(|r| r.len()).sum();
        let mut final_bytes = Vec::with_capacity(final_byte_count);
        for row in &mut rows {
            final_bytes.append(row);
        }

        let (rest, transparency) = if let Some(abc) = alpha_byte_count {
            // okay, the alpha mask too then
            let (alpha_bytes, rest) = rest.split_at(abc);

            // we will round the bits per row to a full byte, not to a full DWORD
            let alpha_bits_per_row = width_usize * 1;
            let min_alpha_bytes_per_row = round_up_usize(alpha_bits_per_row, 8) / 8;
            let alpha_stride = round_up_usize(alpha_bits_per_row, 32) / 8;
            debug_assert!(min_alpha_bytes_per_row <= alpha_stride);

            let mut t_rows = Vec::with_capacity(height_usize);
            for row in alpha_bytes.chunks(alpha_stride) {
                t_rows.push(row[..min_alpha_bytes_per_row].to_vec());
            }

            if flip {
                // oh flipping heck
                t_rows.reverse();
            }

            let mut transparency = Vec::with_capacity(t_rows.iter().map(|r| r.len()).sum());
            for t_row in &mut t_rows {
                transparency.append(t_row);
            }

            (rest, Some(transparency))
        } else {
            (rest, None)
        };

        let image = Bitmap {
            header,
            palette,
            data: final_bytes,
            transparency,
        };
        Ok((rest, image))
    }

    pub fn actual_width(&self) -> u32 {
        self.header.width.try_into().unwrap()
    }

    pub fn actual_height(&self) -> u32 {
        if self.transparency.is_some() {
            (self.header.height.abs() / 2).try_into().unwrap()
        } else {
            self.header.height.abs().try_into().unwrap()
        }
    }

    pub fn to_rgba8(&self) -> Vec<u8> {
        let width_usize = usize::try_from(self.header.width).unwrap();
        let mut height_usize = usize::try_from(self.header.height.abs()).unwrap();
        if self.transparency.is_some() {
            height_usize /= 2;
        }
        let rgb_byte_count = height_usize * width_usize * 3;
        let mut rgb_bytes = Vec::with_capacity(rgb_byte_count);

        let alpha_byte_count = height_usize * width_usize;
        let mut alpha_bytes = Vec::with_capacity(alpha_byte_count);

        match self.header.bit_count {
            1|4|8 => {
                // palette -- reformat it to fit us
                let palette = self.palette.as_ref().unwrap();
                let mut my_palette: Vec<[u8; 3]> = Vec::with_capacity(palette.len() / 3);
                for color in palette.chunks(3) {
                    my_palette.push(color.try_into().unwrap());
                }

                let bit_depth_usize = usize::try_from(self.header.bit_count).unwrap();
                let bits_per_row = bit_depth_usize * width_usize;
                let bytes_per_row = round_up_usize(bits_per_row, 8) / 8;

                for row in self.data.chunks(bytes_per_row) {
                    let mut row_pixels_written = 0;

                    'bytes_in_row: for byte in row {
                        match self.header.bit_count {
                            1 => {
                                for bit in (0..8).rev() {
                                    if row_pixels_written >= width_usize {
                                        break 'bytes_in_row;
                                    }

                                    if byte & (1 << bit) != 0 {
                                        rgb_bytes.extend(my_palette[1]);
                                    } else {
                                        rgb_bytes.extend(my_palette[0]);
                                    }
                                    row_pixels_written += 1;
                                }
                            },
                            4 => {
                                for index in [(byte >> 4) & 0xF, (byte >> 0) & 0xF] {
                                    if row_pixels_written >= width_usize {
                                        break 'bytes_in_row;
                                    }

                                    let index_usize = usize::from(index);
                                    rgb_bytes.extend(my_palette[index_usize]);
                                    row_pixels_written += 1;
                                }
                            },
                            8 => {
                                if row_pixels_written >= width_usize {
                                    break 'bytes_in_row;
                                }

                                let index_usize = usize::from(*byte);
                                rgb_bytes.extend(my_palette[index_usize]);
                                row_pixels_written += 1;
                            },
                            _ => unreachable!(),
                        }
                    }
                }
            },
            16|24|32 => {
                // already converted to RGB8 when decoding
                rgb_bytes.extend(&self.data);
            },
            _ => panic!("invalid bit depth"),
        }

        // note: PNG alpha values are opacity values (0 = transparent, max = opaque)
        // while Windows states 0=opaque 1=transparent
        const PNG_OPAQUE: u8 = 255;
        const PNG_TRANSPARENT: u8 = 0;

        if let Some(transparency) = self.transparency.as_ref() {
            let alpha_bit_depth_usize = 1;
            let alpha_bits_per_row = alpha_bit_depth_usize * width_usize;
            let alpha_bytes_per_row = round_up_usize(alpha_bits_per_row, 8) / 8;

            for alpha_row in transparency.chunks(alpha_bytes_per_row) {
                let mut alpha_row_pixels_written = 0;
                'alpha_bytes_in_row: for byte in alpha_row {
                    for bit in (0..8).rev() {
                        if alpha_row_pixels_written >= width_usize {
                            break 'alpha_bytes_in_row;
                        }

                        let png_alpha_byte = if byte & (1 << bit) != 0 {
                            PNG_TRANSPARENT
                        } else {
                            PNG_OPAQUE
                        };
                        alpha_bytes.push(png_alpha_byte);
                        alpha_row_pixels_written += 1;
                    }
                }
            }
        } else {
            for _ in 0..alpha_byte_count {
                alpha_bytes.push(PNG_OPAQUE);
            }
        }

        assert_eq!(rgb_bytes.len(), 3 * alpha_bytes.len());

        let mut rgba_bytes = Vec::with_capacity(rgb_bytes.len() + alpha_bytes.len());
        for (rgb, a) in rgb_bytes.chunks(3).zip(alpha_bytes.iter()) {
            rgba_bytes.extend(rgb);
            rgba_bytes.push(*a);
        }

        rgba_bytes
    }
}

fn round_up_usize(value: usize, to_multiple_of: usize) -> usize {
    ((value + (to_multiple_of - 1)) / to_multiple_of) * to_multiple_of
}

fn scale_u5_to_u8(u5_value: u8) -> u8 {
    // 0b1_1111 * 0b1111_1111 == 0b0001_1110_1110_0001, which fits into u16
    u8::try_from((u16::from(u5_value) * 0b1111_1111) / 0b1_1111).unwrap()
}
