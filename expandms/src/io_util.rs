use std::io::{self, Read};


#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) struct BitReader<R: Read, const MSB_TO_LSB: bool> {
    byte_reader: R,
    byte_picked_apart: Option<u8>,
    bit_index: u8,
}
impl<R: Read, const MSB_TO_LSB: bool> BitReader<R, MSB_TO_LSB> {
    pub fn new(byte_reader: R) -> Self {
        Self {
            byte_reader,
            byte_picked_apart: None,
            bit_index: 0,
        }
    }

    pub fn read_bit(&mut self) -> Result<Option<bool>, io::Error> {
        if self.bit_index == 0 {
            // pull in new byte
            let mut buf = [0u8];
            let bytes_read = self.byte_reader.read(&mut buf)?;
            if bytes_read == 0 {
                // EOF
                return Ok(None);
            }
            self.byte_picked_apart = Some(buf[0]);
        }

        // if bit_index > 0, we have already stored a byte

        let byte_picked_apart = self.byte_picked_apart.unwrap();
        let actual_bit_index = if MSB_TO_LSB {
            7 - self.bit_index
        } else {
            self.bit_index
        };
        let bit_is_set = (byte_picked_apart & (1 << actual_bit_index)) != 0;

        self.bit_index += 1;
        if self.bit_index == 8 {
            // prepare for next byte
            self.drop_rest_of_byte();
        }

        Ok(Some(bit_is_set))
    }

    pub fn read_bit_strict(&mut self) -> Result<bool, io::Error> {
        match self.read_bit() {
            Ok(Some(b)) => Ok(b),
            Ok(None) => Err(io::ErrorKind::UnexpectedEof.into()),
            Err(e) => Err(e),
        }
    }

    pub fn drop_rest_of_byte(&mut self) {
        self.bit_index = 0;
        self.byte_picked_apart = None;
    }
}

macro_rules! impl_read_n_bits {
    ($name:ident, $bit_count:expr, $ret_type:ty) => {
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
            Ok(ret)
        }
    };
}

impl<R: Read, const MSB_TO_LSB: bool> BitReader<R, MSB_TO_LSB> {
    impl_read_n_bits!(read_u2, 2, u8);
    impl_read_n_bits!(read_u3, 3, u8);
    impl_read_n_bits!(read_u4, 4, u8);
    impl_read_n_bits!(read_u5, 5, u8);
    impl_read_n_bits!(read_u6, 6, u8);
    impl_read_n_bits!(read_u7, 7, u8);
    impl_read_n_bits!(read_u8, 8, u8);
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

pub(crate) trait ReadEndian {
    fn read_be(buf: &[u8], pos: &mut usize) -> Self;
    fn read_le(buf: &[u8], pos: &mut usize) -> Self;
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


fn read_byte(buf: &[u8], pos: &mut usize) -> u8 {
    let ret = buf[*pos];
    *pos += 1;
    ret
}
fn read_bytes<const N: usize>(buf: &[u8], pos: &mut usize) -> [u8; N] {
    let ret = buf[*pos..*pos+N].try_into().unwrap();
    *pos += N;
    ret
}
