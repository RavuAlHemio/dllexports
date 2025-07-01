use std::io::{self, Read};

use display_bytes::DisplayBytes;
use tracing::debug;


macro_rules! impl_read_n_bits {
    ($name:ident, $bit_count:expr, $ret_type:ty) => {
        #[must_use]
        pub fn $name(&mut self) -> Result<$ret_type, io::Error> {
            let mut ret = 0;
            for i in 0..$bit_count {
                let bit = self.read_bit_strict()?;
                if MSB_TO_LSB {
                    ret <<= 1;
                    if bit {
                        ret |= 1;
                    }
                } else {
                    if bit {
                        ret |= (1 << i);
                    }
                }
            }
            debug!("reading {} bits gave {:#X}", $bit_count, ret);
            Ok(ret)
        }
    };
}
macro_rules! impl_read_n_bytes {
    ($name:ident, $read_byte_count:expr, $decode_byte_count:expr, $ret_type:ty, $convert_func:ident) => {
        #[must_use]
        pub fn $name(&mut self) -> Result<$ret_type, io::Error> {
            let mut buf = [0u8; $decode_byte_count];
            for b in &mut buf[..$read_byte_count] {
                *b = self.read_u8()?;
            }
            Ok(<$ret_type>::$convert_func(buf))
        }
    };
    ($name:ident, $byte_count:expr, $ret_type:ty, $convert_func:ident) => {
        impl_read_n_bytes!($name, $byte_count, $byte_count, $ret_type, $convert_func);
    };
}
macro_rules! impl_bit_reader {
    ($name:ident, $unit_type:ty, $bits_per_unit:expr, $bytes_to_unit_func:ident) => {
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub(crate) struct $name<R: Read, const MSB_TO_LSB: bool> {
            byte_reader: R,
            unit_picked_apart: Option<$unit_type>,
            bit_index: u8,
            total_bits_read: u64,
        }
        impl<R: Read, const MSB_TO_LSB: bool> $name<R, MSB_TO_LSB> {
            pub fn new(byte_reader: R) -> Self {
                Self {
                    byte_reader,
                    unit_picked_apart: None,
                    bit_index: 0,
                    total_bits_read: 0,
                }
            }

            pub fn read_bit(&mut self) -> Result<Option<bool>, io::Error> {
                if self.bit_index == 0 {
                    // pull in new unit
                    // bits to bytes conversion rounding up:
                    let mut buf = [0u8; ($bits_per_unit + (8 - 1)) / 8];
                    let mut total_bytes_read = 0;
                    while total_bytes_read < buf.len() {
                        let now_bytes_read = self.byte_reader.read(&mut buf[total_bytes_read..])?;
                        if now_bytes_read == 0 {
                            // EOF
                            return if total_bytes_read == 0 {
                                // EOF at start; that's okay
                                Ok(None)
                            } else {
                                // EOF midway; that isn't
                                Err(io::ErrorKind::UnexpectedEof.into())
                            };
                        }
                        total_bytes_read += now_bytes_read;
                    }
                    self.unit_picked_apart = Some(<$unit_type>::$bytes_to_unit_func(buf));
                }

                // if bit_index > 0, we have already stored a unit

                let byte_picked_apart = self.unit_picked_apart.unwrap();
                let actual_bit_index = if MSB_TO_LSB {
                    ($bits_per_unit - 1) - self.bit_index
                } else {
                    self.bit_index
                };
                let bit_is_set = (byte_picked_apart & (1 << actual_bit_index)) != 0;

                debug!("bit read: {}", if bit_is_set { "1" } else { "0" });

                self.bit_index += 1;
                if self.bit_index == $bits_per_unit {
                    // prepare for next byte
                    self.drop_rest_of_unit();
                }

                self.total_bits_read += 1;

                Ok(Some(bit_is_set))
            }

            pub fn read_bit_strict(&mut self) -> Result<bool, io::Error> {
                match self.read_bit() {
                    Ok(Some(b)) => Ok(b),
                    Ok(None) => Err(io::ErrorKind::UnexpectedEof.into()),
                    Err(e) => Err(e),
                }
            }

            pub fn drop_rest_of_unit(&mut self) {
                if self.bit_index > 0 {
                    self.total_bits_read += u64::from($bits_per_unit - self.bit_index);
                }
                self.bit_index = 0;
                self.unit_picked_apart = None;
            }

            pub fn total_bits_read(&self) -> u64 { self.total_bits_read }
        }
        impl<R: Read> $name<R, true> {
            pub fn new_msb_to_lsb(byte_reader: R) -> Self {
                Self::new(byte_reader)
            }
        }
        impl<R: Read> $name<R, false> {
            pub fn new_lsb_to_msb(byte_reader: R) -> Self {
                Self::new(byte_reader)
            }
        }
    };
}
impl_bit_reader!(BitReader, u8, 8, from_ne_bytes);
impl_bit_reader!(BitReader16Le, u16, 16, from_le_bytes);


impl<R: Read, const MSB_TO_LSB: bool> BitReader<R, MSB_TO_LSB> {
    impl_read_n_bits!(read_u1, 1, u8);
    impl_read_n_bits!(read_u2, 2, u8);
    impl_read_n_bits!(read_u3, 3, u8);
    impl_read_n_bits!(read_u4, 4, u8);
    impl_read_n_bits!(read_u5, 5, u8);
    impl_read_n_bits!(read_u6, 6, u8);
    impl_read_n_bits!(read_u7, 7, u8);
    impl_read_n_bits!(read_u8_bitwise, 8, u8);

    #[must_use]
    pub fn read_u8(&mut self) -> Result<u8, io::Error> {
        // optimization: are we at a byte boundary?
        if self.bit_index == 0 {
            // yes; just read the next byte from the underlying reader
            let mut buf = [0u8];
            self.byte_reader.read_exact(&mut buf)?;
            self.total_bits_read += 8;
            debug!("reading 8 bits (speedy) gave {:#X}", buf[0]);
            Ok(buf[0])
        } else {
            self.read_u8_bitwise()
        }
    }

    impl_read_n_bytes!(read_u16_le, 2, u16, from_le_bytes);
    impl_read_n_bytes!(read_u16_be, 2, u16, from_be_bytes);

    impl_read_n_bytes!(read_u24_le, 3, 4, u32, from_le_bytes);

    #[must_use]
    pub fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), io::Error> {
        for b in buf {
            *b = self.read_u8()?;
        }
        Ok(())
    }
}

impl<R: Read, const MSB_TO_LSB: bool> BitReader16Le<R, MSB_TO_LSB> {
    impl_read_n_bits!(read_u1, 1, u8);
    impl_read_n_bits!(read_u2, 2, u8);
    impl_read_n_bits!(read_u3, 3, u8);
    impl_read_n_bits!(read_u4, 4, u8);
    impl_read_n_bits!(read_u5, 5, u8);
    impl_read_n_bits!(read_u6, 6, u8);
    impl_read_n_bits!(read_u7, 7, u8);
    impl_read_n_bits!(read_u8, 8, u8);
    impl_read_n_bits!(read_u9, 9, u16);
    impl_read_n_bits!(read_u10, 10, u16);
    impl_read_n_bits!(read_u11, 11, u16);
    impl_read_n_bits!(read_u12, 12, u16);
    impl_read_n_bits!(read_u13, 13, u16);
    impl_read_n_bits!(read_u14, 14, u16);
    impl_read_n_bits!(read_u15, 15, u16);
    impl_read_n_bits!(read_u16_bitwise, 16, u16);

    #[must_use]
    pub fn read_u16(&mut self) -> Result<u16, io::Error> {
        // optimization: are we at a unit boundary?
        if self.bit_index == 0 {
            // yes; just read the next unit from the underlying reader
            let mut buf = [0u8; 2];
            self.byte_reader.read_exact(&mut buf)?;
            self.total_bits_read += 16;
            debug!("reading 16 bits (speedy) gave {:#X}", u16::from_le_bytes(buf));
            Ok(u16::from_le_bytes(buf))
        } else {
            self.read_u16_bitwise()
        }
    }

    impl_read_n_bytes!(read_u16_le, 2, u16, from_le_bytes);
    impl_read_n_bytes!(read_u16_be, 2, u16, from_be_bytes);

    impl_read_n_bytes!(read_u24_le, 3, 4, u32, from_le_bytes);

    #[must_use]
    pub fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), io::Error> {
        for b in buf {
            *b = self.read_u8()?;
        }
        Ok(())
    }
}


pub trait AnyBitReader {
    fn read_bit(&mut self) -> Result<Option<bool>, io::Error>;
}
impl<R: Read, const MSB_TO_LSB: bool> AnyBitReader for BitReader<R, MSB_TO_LSB> {
    fn read_bit(&mut self) -> Result<Option<bool>, io::Error> {
        BitReader::read_bit(self)
    }
}
impl<R: Read, const MSB_TO_LSB: bool> AnyBitReader for BitReader16Le<R, MSB_TO_LSB> {
    fn read_bit(&mut self) -> Result<Option<bool>, io::Error> {
        BitReader16Le::read_bit(self)
    }
}


pub(crate) trait ByteBufReadable {
    fn read(buf: &[u8], pos: &mut usize) -> Self;
}
impl ByteBufReadable for u8 {
    fn read(buf: &[u8], pos: &mut usize) -> Self {
        let ret = buf[*pos];
        *pos += 1;
        ret
    }
}
impl ByteBufReadable for i8 {
    fn read(buf: &[u8], pos: &mut usize) -> Self {
        let byte_buf = [buf[*pos]];
        *pos += 1;
        // byte order doesn't matter for single-byte values
        i8::from_ne_bytes(byte_buf)
    }
}
impl<const N: usize> ByteBufReadable for [u8; N] {
    fn read(buf: &[u8], pos: &mut usize) -> Self {
        let ret = buf[*pos..*pos+N].try_into().unwrap();
        *pos += N;
        ret
    }
}
impl<const N: usize> ByteBufReadable for DisplayBytes<N> {
    fn read(buf: &[u8], pos: &mut usize) -> Self {
        let ret: [u8; N] = buf[*pos..*pos+N].try_into().unwrap();
        *pos += N;
        ret.into()
    }
}

pub(crate) trait ReadEndian {
    fn read_be(buf: &[u8], pos: &mut usize) -> Self;
    fn read_le(buf: &[u8], pos: &mut usize) -> Self;
    fn read(buf: &[u8], pos: &mut usize, is_big_endian: bool) -> Self where Self : Sized {
        if is_big_endian {
            Self::read_be(buf, pos)
        } else {
            Self::read_le(buf, pos)
        }
    }
}
macro_rules! impl_read_endian {
    ($type:ty) => {
        impl ReadEndian for $type {
            fn read_be(buf: &[u8], pos: &mut usize) -> Self {
                let size = ::std::mem::size_of::<$type>();
                let val = <$type>::from_be_bytes(buf[*pos..*pos+size].try_into().unwrap());
                *pos += size;
                val
            }
            fn read_le(buf: &[u8], pos: &mut usize) -> Self {
                let size = ::std::mem::size_of::<$type>();
                let val = <$type>::from_le_bytes(buf[*pos..*pos+size].try_into().unwrap());
                *pos += size;
                val
            }
        }
    };
}
impl_read_endian!(u16);
impl_read_endian!(u32);


pub(crate) fn read_bytes<const N: usize>(buf: &[u8], pos: &mut usize) -> [u8; N] {
    let ret = buf[*pos..*pos+N].try_into().unwrap();
    *pos += N;
    ret
}
pub(crate) fn read_bytes_variable(buf: &[u8], pos: &mut usize, byte_count: usize) -> Vec<u8> {
    let ret = buf[*pos..*pos+byte_count].to_vec();
    *pos += byte_count;
    ret
}
