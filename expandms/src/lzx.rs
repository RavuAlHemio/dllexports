//! The Microsoft LZX format.
//!
//! [Microsoft's documentation][mscabdoc] describes most of the format, but the `mspack`
//! implementation found [a few inconsistencies][mspacklzx].
//!
//! [mscabdoc]: https://learn.microsoft.com/en-us/previous-versions/bb417343(v=msdn.10)
//! [mspacklzx]: https://github.com/kyz/libmspack/blob/305907723a4e7ab2018e58040059ffb5e77db837/libmspack/mspack/lzxd.c#L18

use std::fmt;
use std::io::{self, Read, Write};

use tracing::{debug, error};

use crate::huff::{HuffmanCanonicalizable, HuffmanTree};
use crate::io_util::BitReader;
use crate::ring_buffer::RingBuffer;


/// The exponent of the power of two representing the smallest allowed window size.
///
/// The exponent must be greater than or equal to this value.
pub const MIN_WINDOW_SIZE_EXPONENT: usize = 15;

/// The exponent of the power of two representing the largest allowed window size.
///
/// The exponent must be less than or equal to this value.
pub const MAX_WINDOW_SIZE_EXPONENT: usize = 21;

const MAX_LOOKBACK_DISTANCE: usize = 2*1024*1024;

const LENGTH_TREE_ENTRIES: usize = 249;
const ALIGNED_OFFSET_TREE_ENTRIES: usize = 8;


const fn extra_bits(i: usize) -> usize {
    if i < 4 {
        0
    } else if i < 36 {
        // i is guaranteed to be in 4..=35
        // => i / 2 is at least 2
        // => subtraction (worst case: 2 - 1) will never underflow
        // compiler can't reason that out => use wrapping_sub
        (i / 2).wrapping_sub(1)
    } else {
        17
    }
}

const POSITION_BASE: [usize; 291] = {
    let mut pb = [0; 291];
    let mut i = 1;
    while i < pb.len() {
        pb[i] = pb[i-1] + (1 << extra_bits(i-1));
        i += 1;
    }
    pb
};

const WINDOW_SIZE_EXPONENT_TO_POSITION_SLOTS: [usize; 26] = {
    // the index of the smallest position base that can fit the given power of 2
    let mut ps = [0; 26];
    let mut i = 0;
    while i < ps.len() {
        let two_power = 1 << i;
        let mut j = 0;
        while j < POSITION_BASE.len() {
            if two_power <= POSITION_BASE[j] {
                ps[i] = j;
                break;
            }
            j += 1;
        }
        i += 1;
    }
    ps
};

// the main tree contains 256 + 8*WINDOW_SIZE_EXPONENT_TO_POSITION_SLOTS[x] elements
// where x is taken from: window_size = 2**x
// x must be in 15..=21 (so window size ranges from 32K to 2M)
// (the window size is specified out-of-band)

// the length tree always contains 249 elements

// the aligned offset tree always contains 8 elements

// each pre-tree always contains 20 elements

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
enum Offset {
    MostRecent,
    SecondMostRecent,
    ThirdMostRecent,
    Absolute(u32), // max: window_size - 3 (absolute max: 2_097_149)
}

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct RecentLookback {
    r0: u32,
    r1: u32,
    r2: u32,
}
impl RecentLookback {
    pub fn new() -> Self {
        Self {
            r0: 0,
            r1: 0,
            r2: 0,
        }
    }

    pub fn lookup(&mut self, offset: Offset) -> u32 {
        match offset {
            Offset::MostRecent => {
                // theoretically: swap R0 with R0
            },
            Offset::SecondMostRecent => {
                // swap R0 with R1
                std::mem::swap(&mut self.r0, &mut self.r1);
            },
            Offset::ThirdMostRecent => {
                // swap R0 with R2
                std::mem::swap(&mut self.r0, &mut self.r2);
            },
            Offset::Absolute(abs) => {
                // shift the absolute value in
                self.r2 = self.r1;
                self.r1 = self.r0;
                self.r0 = abs;
            },
        }

        // return newest R0
        self.r0
    }
}
impl Default for RecentLookback {
    fn default() -> Self {
        Self { r0: 1, r1: 1, r2: 1 }
    }
}

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    InvalidWindowSizeExponent(usize),
    UnknownBlockType(u8),
    ConstructingPreTree,
    InvalidSecondPreTreeValue(&'static str),
    ConstructingMainTree,
    ConstructingLengthTree,
    ConstructingAlignedOffsetTree,

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
impl Error {
    pub fn new_eof() -> Self {
        Self::Io(io::ErrorKind::UnexpectedEof.into())
    }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e)
                => write!(f, "I/O error: {}", e),
            Self::InvalidWindowSizeExponent(window_size_exponent)
                => write!(
                    f,
                    "invalid window size exponent, expected at least {} and at most {}, obtained {}",
                    MIN_WINDOW_SIZE_EXPONENT,
                    MAX_WINDOW_SIZE_EXPONENT,
                    window_size_exponent,
                ),
            Self::UnknownBlockType(t)
                => write!(f, "unknown block type {:#04X}", t),
            Self::ConstructingPreTree
                => write!(f, "error constructing pre-tree"),
            Self::InvalidSecondPreTreeValue(value_description)
                => write!(f, "invalid second pre-tree value: expected LengthDelta(_), obtained {}", value_description),
            Self::ConstructingMainTree
                => write!(f, "error constructing main tree"),
            Self::ConstructingLengthTree
                => write!(f, "error constructing length tree"),
            Self::ConstructingAlignedOffsetTree
                => write!(f, "error constructing aligned offset tree"),
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
            Self::InvalidWindowSizeExponent(_) => None,
            Self::UnknownBlockType(_) => None,
            Self::ConstructingPreTree => None,
            Self::InvalidSecondPreTreeValue(_) => None,
            Self::ConstructingMainTree => None,
            Self::ConstructingLengthTree => None,
            Self::ConstructingAlignedOffsetTree => None,
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
enum PreTreeCode {
    LengthDelta(u8), // n in 0..=16 => length = (previous_length + n) % 17
    ZeroesShort, // bits = read_u4(); length = 0 for the next (4 + bits) elements
    ZeroesLong, // bits = read_u5(); length = 0 for the next (20 + bits) elements
    Repeat, // bits = read_u1(); new_code = read_next_pre_tree_code(); length = (previous_length + new_code) % 17 for the next (4 + bits) elements
}
impl HuffmanCanonicalizable for PreTreeCode {
    fn first_value() -> Self {
        Self::LengthDelta(0)
    }

    fn incremented(&self) -> Self {
        match self {
            Self::LengthDelta(n) => {
                if *n == 16 {
                    Self::ZeroesShort
                } else {
                    Self::LengthDelta(*n + 1)
                }
            },
            Self::ZeroesShort => Self::ZeroesLong,
            Self::ZeroesLong => Self::Repeat,
            Self::Repeat => panic!("cannot increment further"),
        }
    }
}

pub struct LzxDecompressor<'r, R: Read> {
    reader: BitReader<&'r mut R, false>,
    window_size_exponent: usize,
    lookback: Box<RingBuffer<u8, MAX_LOOKBACK_DISTANCE>>,
    recent_lookback: RecentLookback,
    jump_translation: Option<u32>,

    last_main_256_lengths: Box<[usize; 256]>,
    last_main_rest_lengths: Vec<usize>,
    last_length_lengths: Box<[usize; LENGTH_TREE_ENTRIES]>,
}
impl<'r, R: Read> LzxDecompressor<'r, R> {
    pub fn new(reader: &'r mut R, window_size_exponent: usize) -> Result<Self, Error> {
        let mut reader = BitReader::new(reader);

        if window_size_exponent < MIN_WINDOW_SIZE_EXPONENT || window_size_exponent > MAX_WINDOW_SIZE_EXPONENT {
            return Err(Error::InvalidWindowSizeExponent(window_size_exponent));
        }

        let has_jump_translation = reader.read_bit_strict()?;
        let jump_translation = if has_jump_translation {
            // basically stored as middle-endian
            // (22 33 00 11)
            let top_half = u32::from(reader.read_u16_le()?);
            let bottom_half = u32::from(reader.read_u16_le()?);
            let full = (top_half << 16) | bottom_half;
            Some(full)
        } else {
            None
        };

        // how long are our length lists going to be?
        // main tree is 256 byte values plus NUM_POSITION_SLOTS*8
        // => after 256 values, main tree rest is NUM_POSITION_SLOTS*8
        let num_position_slots = WINDOW_SIZE_EXPONENT_TO_POSITION_SLOTS[window_size_exponent];
        let main_tree_rest_entries = 8*num_position_slots;
        // length tree is always 249 values

        Ok(Self {
            reader,
            window_size_exponent,
            lookback: Box::new(RingBuffer::new(0x00)),
            recent_lookback: RecentLookback::new(),
            jump_translation,

            last_main_256_lengths: Box::new([0; 256]),
            last_main_rest_lengths: vec![0; main_tree_rest_entries],
            last_length_lengths: Box::new([0; LENGTH_TREE_ENTRIES]),
        })
    }

    fn read_pre_tree(&mut self) -> Result<HuffmanTree<PreTreeCode>, Error> {
        let mut lengths = [0usize; 20];
        for length in &mut lengths {
            *length = self.reader.read_u4()?.into();
        }
        match HuffmanTree::new_canonical(&lengths) {
            Ok(ht) => Ok(ht),
            Err(e) => {
                debug!("error constructing pre-tree: {}", e);
                Err(Error::ConstructingPreTree)
            }
        }
    }

    fn read_length_delta_tree(&mut self, pre_tree: &HuffmanTree<PreTreeCode>, prev_lengths: &[usize]) -> Result<Vec<usize>, Error> {
        let mut ret = vec![0; prev_lengths.len()];
        let mut i = 0;
        while i < ret.len() {
            let pre_tree_value = *pre_tree.decode_one_from_bit_reader(&mut self.reader)?
                .ok_or_else(|| Error::new_eof())?;
            match pre_tree_value {
                PreTreeCode::LengthDelta(delta) => {
                    ret[i] = (prev_lengths[i] + usize::from(delta)) % 17;
                    i += 1;
                },
                PreTreeCode::ZeroesShort => {
                    let zero_count = self.reader.read_u4()? + 4;
                    for _ in 0..zero_count {
                        ret[i] = 0;
                        i += 1;
                    }
                },
                PreTreeCode::ZeroesLong => {
                    let zero_count = self.reader.read_u5()? + 20;
                    for _ in 0..zero_count {
                        ret[i] = 0;
                        i += 1;
                    }
                },
                PreTreeCode::Repeat => {
                    let repeat_count = 4 + self.reader.read_u1()?;
                    let new_code = *pre_tree.decode_one_from_bit_reader(&mut self.reader)?
                        .ok_or_else(|| Error::new_eof())?;
                    // read another Huffman symbol; this time, it *must* be a length-delta
                    let new_delta = match new_code {
                        PreTreeCode::LengthDelta(ld) => ld,
                        PreTreeCode::ZeroesLong => return Err(Error::InvalidSecondPreTreeValue("ZeroesLong")),
                        PreTreeCode::ZeroesShort => return Err(Error::InvalidSecondPreTreeValue("ZeroesShort")),
                        PreTreeCode::Repeat => return Err(Error::InvalidSecondPreTreeValue("Repeat")),
                    };
                    for _ in 0..repeat_count {
                        ret[i] = (prev_lengths[i] + usize::from(new_delta)) % 17;
                        i += 1;
                    }
                },
            }
        }
        Ok(ret)
    }

    fn build_main_tree(&mut self) -> Result<HuffmanTree<_>, Error> {
        // read the pre-tree for the first 256 elements of the main tree
        let pre_tree_main_256 = self.read_pre_tree()?;

        // read the path lengths for the first 256 elements of the main tree using the pre-tree
        let main_256_lengths = self.read_length_delta_tree(&pre_tree_main_256, &self.last_main_256_lengths[..])?;

        // remember the lengths for next time
        self.last_main_256_lengths.copy_from_slice(&main_256_lengths);

        // same two steps for the rest of the main tree
        let pre_tree_main_rest = self.read_pre_tree()?;
        let main_rest_lengths = self.read_length_delta_tree(&pre_tree_main_rest, &self.last_main_rest_lengths)?;
        self.last_main_rest_lengths = main_rest_lengths;

        // build the main tree
        let mut main_all_lengths = Vec::with_capacity(main_256_lengths.len() + main_rest_lengths.len());
        main_all_lengths.extend_from_slice(&main_256_lengths);
        main_all_lengths.extend_from_slice(&main_rest_lengths);
        match HuffmanTree::new_canonical(&main_all_lengths) {
            Ok(mt) => Ok(mt),
            Err(e) => {
                error!("error building main tree: {}", e);
                Err(Error::ConstructingMainTree)
            },
        }
    }

    pub fn decompress_block(&mut self, dest_buffer: &mut Vec<u8>) -> Result<bool, Error> {
        let block_type = self.reader.read_u3()?;
        let num_uncompressed_bytes = self.reader.read_u24_le()?;
        todo!("check endianness of num_uncompressed_bytes");
        match block_type {
            1 => {
                debug!("block type: verbatim");

                // build the main tree
                let main_tree = self.build_main_tree()?;

                // build the length tree
                let pre_tree_length = self.read_pre_tree()?;
                let length_lengths = self.read_length_delta_tree(&pre_tree_length, &self.last_length_lengths[..])?;
                self.last_length_lengths.copy_from_slice(&length_lengths);
                let length_tree = match HuffmanTree::new_canonical(&main_all_lengths) {
                    Ok(mt) => mt,
                    Err(e) => {
                        error!("error building length tree: {}", e);
                        return Err(Error::ConstructingLengthTree);
                    },
                };

                todo!("read compressed literals");
            },
            2 => {
                debug!("block type: aligned offset");

                // build the main tree
                let main_tree = self.build_main_tree()?;

                // build the aligned offset tree
                let aligned_offset_tree_lengths = [0usize; ALIGNED_OFFSET_TREE_ENTRIES];
                for item in &mut aligned_offset_tree_lengths {
                    *item = self.reader.read_u3()?.into();
                }
                let aligned_offset_tree = match HuffmanTree::new_canonical(&aligned_offset_tree_lengths) {
                    Ok(aot) => aot,
                    Err(e) => {
                        error!("error building aligned offset tree: {}", e);
                        return Err(Error::ConstructingAlignedOffsetTree);
                    }
                };

                todo!("read compressed literals");
            },
            3 => {
                // uncompressed block

                // spec erratum: uncompressed blocks also start with the 24 bits of length (read above)

                // padding to next 16-bit boundary, including if we already are at a 16-bit boundary
                let bits_to_drop = 16 - (self.reader.total_bits_read() % 16);
                for _ in 0..bits_to_drop {
                    self.reader.read_bit_strict()?;
                }

                // in little-endian format, new values for the recent-lookback system
                let mut buf = [0u8; 4];
                self.reader.read_exact(&mut buf)?;
                self.recent_lookback.r0 = u32::from_le_bytes(buf);
                self.reader.read_exact(&mut buf)?;
                self.recent_lookback.r1 = u32::from_le_bytes(buf);
                self.reader.read_exact(&mut buf)?;
                self.recent_lookback.r2 = u32::from_le_bytes(buf);

                todo!("read uncompressed bytes");
                todo!("realignment byte if uncompressed size is odd");

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
            other => return Err(Error::UnknownBlockType(other)),
        }
        Ok(is_final)
    }
}
