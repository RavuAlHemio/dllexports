//! Decompression logic for the DEFLATE algorithm (RFC1951).


use std::borrow::Cow;
use std::fmt;
use std::io::{self, Read, Write};
use std::sync::LazyLock;

use display_bytes::HexBytesSlice;
use tracing::debug;

use crate::huff::{HuffmanCanonicalizable, HuffmanTree};
use crate::io_util::BitReader;
use crate::ring_buffer::RingBuffer;


const LENGTH_VALUES: [BaseCountAndExtraBits; 29] = [
    BaseCountAndExtraBits::new(3, 0),
    BaseCountAndExtraBits::new(4, 0),
    BaseCountAndExtraBits::new(5, 0),
    BaseCountAndExtraBits::new(6, 0),
    BaseCountAndExtraBits::new(7, 0),
    BaseCountAndExtraBits::new(8, 0),
    BaseCountAndExtraBits::new(9, 0),
    BaseCountAndExtraBits::new(10, 0),
    BaseCountAndExtraBits::new(11, 1),
    BaseCountAndExtraBits::new(13, 1),
    BaseCountAndExtraBits::new(15, 1),
    BaseCountAndExtraBits::new(17, 1),
    BaseCountAndExtraBits::new(19, 2),
    BaseCountAndExtraBits::new(23, 2),
    BaseCountAndExtraBits::new(27, 2),
    BaseCountAndExtraBits::new(31, 2),
    BaseCountAndExtraBits::new(35, 3),
    BaseCountAndExtraBits::new(43, 3),
    BaseCountAndExtraBits::new(51, 3),
    BaseCountAndExtraBits::new(59, 3),
    BaseCountAndExtraBits::new(67, 4),
    BaseCountAndExtraBits::new(83, 4),
    BaseCountAndExtraBits::new(99, 4),
    BaseCountAndExtraBits::new(115, 4),
    BaseCountAndExtraBits::new(131, 5),
    BaseCountAndExtraBits::new(163, 5),
    BaseCountAndExtraBits::new(195, 5),
    BaseCountAndExtraBits::new(227, 5),
    BaseCountAndExtraBits::new(258, 0),
];
const DISTANCE_VALUES: [BaseCountAndExtraBits; 30] = [
    BaseCountAndExtraBits::new(1, 0),
    BaseCountAndExtraBits::new(2, 0),
    BaseCountAndExtraBits::new(3, 0),
    BaseCountAndExtraBits::new(4, 0),
    BaseCountAndExtraBits::new(5, 1),
    BaseCountAndExtraBits::new(7, 1),
    BaseCountAndExtraBits::new(9, 2),
    BaseCountAndExtraBits::new(13, 2),
    BaseCountAndExtraBits::new(17, 3),
    BaseCountAndExtraBits::new(25, 3),
    BaseCountAndExtraBits::new(33, 4),
    BaseCountAndExtraBits::new(49, 4),
    BaseCountAndExtraBits::new(65, 5),
    BaseCountAndExtraBits::new(97, 5),
    BaseCountAndExtraBits::new(129, 6),
    BaseCountAndExtraBits::new(193, 6),
    BaseCountAndExtraBits::new(257, 7),
    BaseCountAndExtraBits::new(385, 7),
    BaseCountAndExtraBits::new(513, 8),
    BaseCountAndExtraBits::new(769, 8),
    BaseCountAndExtraBits::new(1025, 9),
    BaseCountAndExtraBits::new(1537, 9),
    BaseCountAndExtraBits::new(2049, 10),
    BaseCountAndExtraBits::new(3073, 10),
    BaseCountAndExtraBits::new(4097, 11),
    BaseCountAndExtraBits::new(6145, 11),
    BaseCountAndExtraBits::new(8193, 12),
    BaseCountAndExtraBits::new(12289, 12),
    BaseCountAndExtraBits::new(16385, 13),
    BaseCountAndExtraBits::new(24577, 13),
];
const MAX_LOOKBACK_DISTANCE: usize = 32768;


static PREDEFINED_VALUE_TREE: LazyLock<HuffmanTree<InflateValue>> = LazyLock::new(|| {
    let mut symbol_lengths = Vec::with_capacity(288);
    for value in 0..288 {
        if value <= 143 {
            symbol_lengths.push(8);
        } else if value <= 255 {
            symbol_lengths.push(9);
        } else if value <= 279 {
            symbol_lengths.push(7);
        } else {
            symbol_lengths.push(8);
        }
    }

    HuffmanTree::new_canonical(&symbol_lengths)
        .expect("failed to construct predefined length tree")
});
static PREDEFINED_DISTANCE_TREE: LazyLock<HuffmanTree<usize>> = LazyLock::new(|| {
    // distance "tree" is simply 5 bits of index into DISTANCE_VALUES
    let symbol_lengths = [5; 32];
    HuffmanTree::new_canonical(&symbol_lengths)
        .expect("failed to construct predefined distance tree")
});
const DEFINITION_CODE_LENGTH_ORDER: [usize; 19] = [
    16, // DefinitionValue::CopyPreviousCodeLength
    17, // DefinitionValue::ShortZeroes,
    18, // DefinitionValue::LongZeroes,
    0, 8, 7, 9, 6, 10, 5, 11,
    4, 12, 3, 13, 2, 14, 1, 15,
];

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    BuildingDefinitionTree,
    DecodingDefinitionValue,
    NoPreviousCodeLength,
    BuildingValueTree,
    BuildingDistanceTree,
    ReadingValue,
    ReadingDistance,
    InvalidDefinitionValue,
    InvalidValue,
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e)
                => write!(f, "I/O error: {}", e),
            Self::BuildingDefinitionTree
                => write!(f, "error building definition tree"),
            Self::DecodingDefinitionValue
                => write!(f, "error decoding definition value"),
            Self::NoPreviousCodeLength
                => write!(f, "referring to yet-unset previous code length"),
            Self::BuildingValueTree
                => write!(f, "error building value tree"),
            Self::BuildingDistanceTree
                => write!(f, "error building distance tree"),
            Self::ReadingValue
                => write!(f, "error reading value"),
            Self::ReadingDistance
                => write!(f, "error reading distance"),
            Self::InvalidDefinitionValue
                => write!(f, "invalid definition value returned from tree"),
            Self::InvalidValue
                => write!(f, "invalid value returned from tree"),
        }
    }
}
impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::BuildingDefinitionTree => None,
            Self::DecodingDefinitionValue => None,
            Self::NoPreviousCodeLength => None,
            Self::BuildingValueTree => None,
            Self::BuildingDistanceTree => None,
            Self::ReadingValue => None,
            Self::ReadingDistance => None,
            Self::InvalidDefinitionValue => None,
            Self::InvalidValue => None,
        }
    }
}
impl From<io::Error> for Error {
    fn from(value: io::Error) -> Self { Self::Io(value) }
}


#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
enum InflateValue {
    Literal(u8), // 0..=255
    EndOfBlock, // 256
    Lookback(usize), // 257..=285, index into LENGTH_VALUES
    Invalid(usize),
}
impl HuffmanCanonicalizable for InflateValue {
    fn first_value() -> Self {
        Self::Literal(0)
    }

    fn incremented(&self) -> Self {
        match self {
            Self::Literal(n) => {
                if *n < 255 {
                    Self::Literal(*n + 1)
                } else {
                    Self::EndOfBlock
                }
            },
            Self::EndOfBlock => Self::Lookback(0),
            Self::Lookback(n) => {
                if *n < LENGTH_VALUES.len() {
                    Self::Lookback(*n + 1)
                } else {
                    Self::Invalid(0)
                }
            },
            Self::Invalid(n) => Self::Invalid(*n + 1),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct BaseCountAndExtraBits {
    pub base_count: usize,
    pub extra_bits: usize,
}
impl BaseCountAndExtraBits {
    pub const fn new(base_count: usize, extra_bits: usize) -> Self {
        Self {
            base_count,
            extra_bits,
        }
    }

    pub fn obtain_count<R: Read, const MSB_TO_LSB: bool>(&self, reader: &mut BitReader<&mut R, MSB_TO_LSB>) -> Result<usize, io::Error> {
        let mut extra_bits_value = 0;
        for i in 0..self.extra_bits {
            let bit = reader.read_bit_strict()?;
            debug!("extra bit: {}", if bit { "1" } else { "0" });
            if bit {
                extra_bits_value |= 1 << i;
            }
        }
        Ok(self.base_count + extra_bits_value)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
enum DefinitionValue {
    CodeLength(u8), // 0..=15
    CopyPreviousCodeLength, // 16 (read 2 bits and add 3)
    ShortZeroes, // 17 (read 3 bits and add 3)
    LongZeroes, // 18 (read 7 bits and add 11)
    Invalid(usize),
}
impl HuffmanCanonicalizable for DefinitionValue {
    fn first_value() -> Self {
        Self::CodeLength(0)
    }

    fn incremented(&self) -> Self {
        match self {
            Self::CodeLength(n) => {
                if *n < 15 {
                    Self::CodeLength(*n + 1)
                } else {
                    Self::CopyPreviousCodeLength
                }
            },
            Self::CopyPreviousCodeLength => Self::ShortZeroes,
            Self::ShortZeroes => Self::LongZeroes,
            Self::LongZeroes => Self::Invalid(0),
            Self::Invalid(n) => Self::Invalid(*n + 1),
        }
    }
}


pub struct Inflater<'r, R: Read> {
    reader: BitReader<&'r mut R, false>,
    lookback: RingBuffer<u8, MAX_LOOKBACK_DISTANCE>,
}
impl<'r, R: Read> Inflater<'r, R> {
    pub fn new(reader: &'r mut R) -> Self {
        let reader = BitReader::new(reader);
        Self {
            reader,
            lookback: RingBuffer::new(0x00),
        }
    }

    pub fn lookback(&self) -> &RingBuffer<u8, MAX_LOOKBACK_DISTANCE> {
        &self.lookback
    }

    pub fn set_lookback(&mut self, lookback: RingBuffer<u8, MAX_LOOKBACK_DISTANCE>) {
        self.lookback = lookback;
    }

    pub fn inflate_block(&mut self, dest_buffer: &mut Vec<u8>) -> Result<bool, Error> {
        let is_final = self.reader.read_bit_strict()?;
        if is_final {
            debug!("is final");
        } else {
            debug!("is not final");
        }

        let block_type = self.reader.read_u2()?;
        match block_type {
            0 => {
                debug!("block type: no compression");

                // skip remaining bits
                self.reader.drop_rest_of_byte();

                // read LEN and NLEN
                let len = self.reader.read_u16_le()?;
                let _nlen = self.reader.read_u16_le()?;
                // FIXME: check if { len == !_nlen }?

                debug!("reading {} raw bytes", len);

                let mut buf = vec![0u8; len.into()];
                self.reader.read_exact(&mut buf)?;
                dest_buffer.write_all(&buf)?;
                for b in &buf {
                    self.lookback.push(*b);
                }
            },
            1|2 => {
                // Huffman table compression
                let (value_tree, distance_tree) = if block_type == 1 {
                    debug!("block type: fixed Huffman tables");
                    (
                        Cow::Borrowed(&*PREDEFINED_VALUE_TREE),
                        Cow::Borrowed(&*PREDEFINED_DISTANCE_TREE),
                    )
                } else {
                    assert_eq!(block_type, 2);
                    debug!("block type: dynamic Huffman tables");

                    // read the table
                    let value_code_count = u16::from(self.reader.read_u5()?) + 257;
                    let distance_code_count = self.reader.read_u5()? + 1;
                    let length_code_count = self.reader.read_u4()? + 4;

                    let mut definition_code_lengths = [0usize; DEFINITION_CODE_LENGTH_ORDER.len()];
                    for i in 0..usize::from(length_code_count) {
                        let code_length = usize::from(self.reader.read_u3()?);
                        let index = DEFINITION_CODE_LENGTH_ORDER[i];
                        definition_code_lengths[index] = code_length;
                    }
                    debug!("definition code lengths: {:?}", definition_code_lengths);

                    let definition_tree: HuffmanTree<DefinitionValue> = HuffmanTree::new_canonical(&definition_code_lengths)
                        .map_err(|_| Error::BuildingDefinitionTree)?;

                    let total_code_count = usize::from(value_code_count) + usize::from(distance_code_count);
                    debug!("definition: {} values + {} distances = {} codes", value_code_count, distance_code_count, total_code_count);
                    let mut code_lengths = Vec::with_capacity(total_code_count);
                    let mut previous_code_length = None;
                    while code_lengths.len() < total_code_count {
                        let definition_value = definition_tree.decode_one_from_bit_reader(&mut self.reader)
                            .map_err(|_| Error::DecodingDefinitionValue)?
                            .ok_or_else(|| Error::Io(io::ErrorKind::InvalidData.into()))?;
                        match definition_value {
                            DefinitionValue::CodeLength(code_length) => {
                                debug!("definition value: code length {}", code_length);
                                code_lengths.push(usize::from(*code_length));
                                previous_code_length = Some(*code_length);
                            },
                            DefinitionValue::CopyPreviousCodeLength => {
                                if let Some(pcl) = previous_code_length {
                                    // find out how often: read 2 bits and add 3
                                    let copy_count = self.reader.read_u2()? + 3;
                                    debug!("definition value: previous code length {} times", copy_count);
                                    for _ in 0..copy_count {
                                        code_lengths.push(usize::from(pcl));
                                    }
                                } else {
                                    // referring to something unset
                                    return Err(Error::NoPreviousCodeLength);
                                }
                            },
                            DefinitionValue::ShortZeroes => {
                                // append zero
                                // find out how often: read 3 bits and add 3
                                let zero_count = self.reader.read_u3()? + 3;
                                debug!("definition value: {} zeroes (short)", zero_count);
                                for _ in 0..zero_count {
                                    code_lengths.push(0);
                                }
                            },
                            DefinitionValue::LongZeroes => {
                                // append more zero
                                // find out how often: read 7 bits and add 11
                                let zero_count = self.reader.read_u7()? + 11;
                                debug!("definition value: {} zeroes (long)", zero_count);
                                for _ in 0..zero_count {
                                    code_lengths.push(0);
                                }
                            },
                            DefinitionValue::Invalid(_) => return Err(Error::InvalidDefinitionValue),
                        }
                    }

                    // split lengths
                    let (value_lengths, distance_lengths) = code_lengths.split_at(usize::from(value_code_count));

                    // build trees
                    let value_tree: HuffmanTree<InflateValue> = HuffmanTree::new_canonical(value_lengths)
                        .map_err(|_| Error::BuildingValueTree)?;
                    let distance_tree: HuffmanTree<usize> = HuffmanTree::new_canonical(distance_lengths)
                        .map_err(|_| Error::BuildingDistanceTree)?;
                    (Cow::Owned(value_tree), Cow::Owned(distance_tree))
                };

                // now that we have the tree, loop through the data
                loop {
                    // read a value
                    let value = value_tree.decode_one_from_bit_reader(&mut self.reader)
                        .map_err(|_| Error::ReadingValue)?
                        .ok_or_else(|| Error::Io(io::ErrorKind::UnexpectedEof.into()))?;
                    match value {
                        InflateValue::EndOfBlock => {
                            // done
                            debug!("inflate value: end of block");
                            break;
                        },
                        InflateValue::Literal(l) => {
                            let c = if *l >= b' ' && *l <= b'~' {
                                char::from_u32(u32::from(*l)).unwrap()
                            } else {
                                ' '
                            };
                            debug!("inflate value: literal byte {} {:#04X}", c, l);
                            self.lookback.push(*l);
                            dest_buffer.push(*l);
                        },
                        InflateValue::Lookback(length_index) => {
                            let length_value = LENGTH_VALUES[*length_index];
                            let length = length_value.obtain_count(&mut self.reader)?;
                            debug!("inflate value decoding: look back for {} bytes", length);

                            let distance_index = distance_tree.decode_one_from_bit_reader(&mut self.reader)
                                .map_err(|_| Error::ReadingDistance)?
                                .ok_or_else(|| Error::Io(io::ErrorKind::UnexpectedEof.into()))?;
                            debug!("inflate value decoding: look back to distance index {}", distance_index);
                            let distance_value = DISTANCE_VALUES[*distance_index];
                            debug!("inflate value decoding: look back to distance value {:?}", distance_value);
                            let distance = distance_value.obtain_count(&mut self.reader)?;

                            debug!("inflate value: look back {} bytes for {} bytes", distance, length);

                            let mut buf = self.lookback.recall(distance, length);
                            debug!("inflate value addendum: lookback buffer: {}", HexBytesSlice::from(buf.as_slice()));
                            dest_buffer.append(&mut buf);
                        },
                        InflateValue::Invalid(_) => return Err(Error::InvalidValue),
                    }
                }
            },
            3 => {
                // reserved
                return Err(Error::Io(io::ErrorKind::InvalidData.into()));
            },
            _ => unreachable!(),
        }
        Ok(is_final)
    }
}


#[cfg(test)]
mod tests {
    use super::Inflater;
    use std::io::Cursor;
    use tracing_test::traced_test;

    #[test]
    #[traced_test]
    fn test_inflate() {
        // the plaintext should allow for a lot of backreferences
        // deflated variant produced with the official Python 3.12.1 Windows x64 build with zlib.compress(plaintext, wbits=-15)
        let deflated = b"KL\xcaIUHN\x04\x91i`2\x1dL\x16\x83\xc9\x120\x99X\x04\xa6R\xf2\xc1Tj\x1e\x98\xca\xc9\x84\xa8\x83()\x85\x08\x96B\xb4\x95\x81\xe5\x00";
        let plaintext = b"able cable fable gable sable table arable doable enable liable stable unable usable viable";

        let mut deflated_reader = Cursor::new(deflated);
        let mut inflater = Inflater::new(&mut deflated_reader);

        let mut output = Vec::new();
        loop {
            let is_last = inflater.inflate_block(&mut output)
                .expect("failed to inflate block");
            if is_last {
                break;
            }
        }
        assert_eq!(&output, plaintext);
    }
}
