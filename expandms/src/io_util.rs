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
            self.bit_index = 0;
            self.byte_picked_apart = None;
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
}

macro_rules! impl_read_n_bits {
    ($name:ident, $bit_count:expr, $ret_type:ty) => {
        pub fn $name(&mut self) -> Result<$ret_type, io::Error> {
            let mut ret = 0;
            for i in 0..$bit_count {
                let bit = self.read_bit_strict()?;
                if MSB_TO_LSB {
                    if bit {
                        ret |= 1;
                    }
                    ret <<= 1;
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
