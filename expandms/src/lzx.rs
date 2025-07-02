//! The Microsoft LZX format.
//!
//! [Microsoft's documentation][mscabdoc] describes most of the format, but the `mspack`
//! implementation found [a few inconsistencies][mspacklzx].
//!
//! [mscabdoc]: https://learn.microsoft.com/en-us/previous-versions/bb417343(v=msdn.10)
//! [mspacklzx]: https://github.com/kyz/libmspack/blob/305907723a4e7ab2018e58040059ffb5e77db837/libmspack/mspack/lzxd.c#L18

use std::fmt;
use std::io::{self, Read};

use display_bytes::{DisplayBytesSlice, HexBytesSlice};
use tracing::{debug, error};

use crate::huff::{HuffmanCanonicalizable, HuffmanTree};
use crate::io_util::BitReader16Le;
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


macro_rules! make_unsigned_const_conversion_function {
    ($name:ident, $from_type:ty, $to_type:ty) => {
        const fn $name(val: $from_type) -> $to_type {
            const FROM_SIZE: usize = std::mem::size_of::<$from_type>();
            const TO_SIZE: usize = std::mem::size_of::<$to_type>();
            if FROM_SIZE <= TO_SIZE {
                val as $to_type
            } else {
                // FROM_SIZE > TO_SIZE
                if val <= (<$to_type>::MAX as $from_type) {
                    val as $to_type
                } else {
                    panic!("value too large");
                }
            }
        }
    };
}


const fn extra_bits(position_slot_number: u32) -> u32 {
    if position_slot_number < 4 {
        0
    } else if position_slot_number < 36 {
        // i is guaranteed to be in 4..=35
        // => i / 2 is at least 2
        // => subtraction (worst case: 2 - 1) will never underflow
        // compiler can't reason that out => use wrapping_sub
        (position_slot_number / 2).wrapping_sub(1)
    } else {
        17
    }
}

make_unsigned_const_conversion_function!(usize_to_u32, usize, u32);
make_unsigned_const_conversion_function!(u32_to_usize, u32, usize);

const POSITION_SLOT_NUMBER_TO_POSITION_BASE: [u32; 291] = {
    let mut pb = [0; 291];
    let mut i = 1;
    while i < pb.len() {
        pb[i] = pb[i-1] + (1 << extra_bits(usize_to_u32(i-1)));
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
        while j < POSITION_SLOT_NUMBER_TO_POSITION_BASE.len() {
            if two_power <= POSITION_SLOT_NUMBER_TO_POSITION_BASE[j] {
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

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
enum Offset {
    MostRecent,
    SecondMostRecent,
    ThirdMostRecent,
    Absolute {
        position_slot_number: u32, // max: window_size (absolute max: 2_097_152), with 3 meaning offset 1
    },
}
impl HuffmanCanonicalizable for Offset {
    // the values are not directly placed in a Huffman tree,
    // but this increment logic is useful for later

    fn first_value() -> Self {
        Self::MostRecent
    }

    fn incremented(&self) -> Self {
        match self {
            Self::MostRecent => Self::SecondMostRecent,
            Self::SecondMostRecent => Self::ThirdMostRecent,
            Self::ThirdMostRecent => {
                Self::Absolute {
                    position_slot_number: 3,
                }
            },
            Self::Absolute { position_slot_number } => {
                Self::Absolute {
                    position_slot_number: *position_slot_number + 1,
                }
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct RecentLookback {
    r0: u32,
    r1: u32,
    r2: u32,
}
impl RecentLookback {
    pub const fn new() -> Self {
        Self {
            r0: 1,
            r1: 1,
            r2: 1,
        }
    }

    // the boolean value returns whether it is an absolute offset and a new value must be pushed in
    pub fn lookup(&mut self, offset: Offset) -> (u32, bool) {
        match offset {
            Offset::MostRecent => {
                // "swap r0 and r0" (i.e. do nothing)
                (self.r0, false)
            },
            Offset::SecondMostRecent => {
                // swap r0 and r1
                std::mem::swap(&mut self.r0, &mut self.r1);
                (self.r0, false)
            },
            Offset::ThirdMostRecent => {
                // swap r0 and r2
                std::mem::swap(&mut self.r0, &mut self.r2);
                (self.r0, false)
            },
            Offset::Absolute { position_slot_number } => {
                // this will need adjustment
                (position_slot_number, true)
            },
        }
    }

    pub fn push(&mut self, new_offset: u32) {
        // self.r2 falls out
        self.r2 = self.r1;
        self.r1 = self.r0;
        self.r0 = new_offset;
    }
}
impl Default for RecentLookback {
    fn default() -> Self {
        Self::new()
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

/// A three-bit (0..=7) length indicator in a header.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct LengthHeader(u8);
impl LengthHeader {
    pub const fn new(length_header: u8) -> Option<Self> {
        if length_header < 8 {
            Some(Self(length_header))
        } else {
            None
        }
    }

    pub const fn zero() -> Self {
        Self(0)
    }

    pub const fn as_u8(&self) -> u8 {
        self.0
    }

    /// Whether the length value has its maximum value, which indicates that the actual length is
    /// encoded externally.
    pub const fn is_max(&self) -> bool {
        self.0 == 7
    }
}
impl From<LengthHeader> for u8 {
    fn from(value: LengthHeader) -> Self {
        value.as_u8()
    }
}
impl TryFrom<u8> for LengthHeader {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Self::new(value).ok_or(())
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
enum MainTreeCode {
    LiteralByte(u8), // n in 0..=255
    Lookback {
        offset: Offset, // (n - 256), all other bits
        length_header: LengthHeader, // (n - 256), bottom three bits (0..=7)
    },
}
impl HuffmanCanonicalizable for MainTreeCode {
    fn first_value() -> Self {
        Self::LiteralByte(0)
    }

    fn incremented(&self) -> Self {
        // 0 0000 0000 => literal 0x00
        // 0 0000 0001 => literal 0x01
        // ...
        // 0 1111 1111 => literal 0x0F
        // 1 0000 0000 => recent R0, length header 0
        // 1 0000 0001 => recent R0, length header 1
        // ...
        // 1 0000 0111 => recent R0, length header 7
        // 1 0000 1000 => recent R1, length header 0
        // 1 0000 1001 => recent R1, length header 1
        // ...
        // 1 0000 1111 => recent R1, length header 7
        // 1 0001 0000 => recent R2, length header 0
        // 1 0001 0001 => recent R2, length header 1
        // ...
        // 1 0001 0111 => recent R2, length header 7
        // 1 0001 1000 => actual offset 1, length header 0
        // 1 0001 1001 => actual offset 1, length header 1
        // ...
        // 1 0001 1111 => actual offset 1, length header 7
        // 1 0010 0000 => actual offset 2, length header 0
        // etc.
        match self {
            Self::LiteralByte(n) => {
                if *n == 255 {
                    Self::Lookback {
                        offset: Offset::MostRecent,
                        length_header: LengthHeader::zero(),
                    }
                } else {
                    Self::LiteralByte(*n + 1)
                }
            },
            Self::Lookback { offset, length_header} => {
                if length_header.is_max() {
                    Self::Lookback {
                        offset: offset.incremented(),
                        length_header: LengthHeader::zero(),
                    }
                } else {
                    Self::Lookback {
                        offset: *offset,
                        length_header: LengthHeader::new(length_header.as_u8() + 1).unwrap(),
                    }
                }
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
enum PreviousLengthType<'a> {
    Main256,
    MainRest,
    Length,
    Other(&'a [usize]),
}

pub struct LzxDecompressor<'r, R: Read> {
    reader: BitReader16Le<&'r mut R, true>,
    window_size_exponent: usize,
    lookback: RingBuffer<u8, MAX_LOOKBACK_DISTANCE>,
    recent_lookback: RecentLookback,
    jump_translation: Option<i32>,
    position_for_jump_translation: u32,

    last_main_256_lengths: Box<[usize; 256]>,
    last_main_rest_lengths: Vec<usize>,
    last_length_lengths: Box<[usize; LENGTH_TREE_ENTRIES]>,
}
impl<'r, R: Read> LzxDecompressor<'r, R> {
    pub fn new(reader: &'r mut R, window_size_exponent: usize) -> Result<Self, Error> {
        let mut reader = BitReader16Le::new(reader);

        if window_size_exponent < MIN_WINDOW_SIZE_EXPONENT || window_size_exponent > MAX_WINDOW_SIZE_EXPONENT {
            return Err(Error::InvalidWindowSizeExponent(window_size_exponent));
        }

        let has_jump_translation = reader.read_bit_strict()?;
        let jump_translation = if has_jump_translation {
            debug!("reading jump translation");
            // basically stored as middle-endian
            // (22 33 00 11)
            let top_half = u32::from(reader.read_u16_le()?);
            let bottom_half = u32::from(reader.read_u16_le()?);
            debug!("top half {:#X}, bottom half {:#X}", top_half, bottom_half);
            let full_u32 = (top_half << 16) | bottom_half;
            // bitcast to signed
            let full = full_u32 as i32;
            Some(full)
        } else {
            None
        };
        debug!("jump translation: {:?}", jump_translation);

        // how long are our length lists going to be?
        // main tree is 256 byte values plus NUM_POSITION_SLOTS*8
        // => after 256 values, main tree rest is NUM_POSITION_SLOTS*8
        let num_position_slots = WINDOW_SIZE_EXPONENT_TO_POSITION_SLOTS[window_size_exponent];
        let main_tree_rest_entries = 8*num_position_slots;
        // length tree is always 249 values

        Ok(Self {
            reader,
            window_size_exponent,
            lookback: RingBuffer::new(0x00),
            recent_lookback: RecentLookback::new(),
            jump_translation,
            position_for_jump_translation: 0,

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
        debug!("pre-tree lengths: {:?}", lengths);
        match HuffmanTree::new_canonical(&lengths) {
            Ok(ht) => Ok(ht),
            Err(e) => {
                debug!("error constructing pre-tree: {}", e);
                Err(Error::ConstructingPreTree)
            }
        }
    }

    fn read_length_delta_tree(&mut self, pre_tree: &HuffmanTree<PreTreeCode>, prev_length_type: PreviousLengthType) -> Result<Vec<usize>, Error> {
        let prev_lengths = match prev_length_type {
            PreviousLengthType::Main256 => &self.last_main_256_lengths[..],
            PreviousLengthType::MainRest => &self.last_main_rest_lengths[..],
            PreviousLengthType::Length => &self.last_length_lengths[..],
            PreviousLengthType::Other(items) => items,
        };

        let mut ret = vec![0; prev_lengths.len()];
        let mut i = 0;
        while i < ret.len() {
            let pre_tree_value = *pre_tree.decode_one_from_bit_reader(&mut self.reader)?
                .ok_or_else(|| Error::new_eof())?;
            match pre_tree_value {
                PreTreeCode::LengthDelta(delta) => {
                    let delta_usize = usize::from(delta);
                    ret[i] = if prev_lengths[i] < delta_usize {
                        (prev_lengths[i] + 17) - delta_usize
                    } else {
                        prev_lengths[i] - delta_usize
                    };
                    debug!("building tree: delta length gives {}", ret[i]);
                    i += 1;
                },
                PreTreeCode::ZeroesShort => {
                    let zero_count = self.reader.read_u4()? + 4;
                    debug!("building tree: short zero run of {} items", zero_count);
                    for _ in 0..zero_count {
                        ret[i] = 0;
                        i += 1;
                    }
                },
                PreTreeCode::ZeroesLong => {
                    let zero_count = self.reader.read_u5()? + 20;
                    debug!("building tree: long zero run of {} items", zero_count);
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
                    debug!("building tree: repeat run of {} items with delta {:#04X}", repeat_count, new_delta);
                    for _ in 0..repeat_count {
                        let new_delta_usize = usize::from(new_delta);
                        ret[i] = if prev_lengths[i] < new_delta_usize {
                            (prev_lengths[i] + 17) - new_delta_usize
                        } else {
                            prev_lengths[i] - new_delta_usize
                        };
                        i += 1;
                    }
                },
            }
        }
        debug!("final lengths: {:?}", ret);
        Ok(ret)
    }

    pub fn decompress_block(&mut self, dest_buffer: &mut Vec<u8>) -> Result<(), Error> {
        let original_buf_size = usize_to_u32(dest_buffer.len());
        let block_type = self.reader.read_u3()?;
        let num_uncompressed_bytes = {
            // 24 bits, big endian
            let mut buf = [0u8; 4];
            for b in &mut buf[1..4] {
                *b = self.reader.read_u8()?;
            }
            u32::from_be_bytes(buf)
        };
        match block_type {
            1|2 => {
                // lots of shared code for these two types
                let aligned_offset_tree_opt = if block_type == 1 {
                    debug!("block type: verbatim");
                    None
                } else {
                    debug_assert_eq!(block_type, 2);
                    debug!("block type: aligned offset");

                    // build the aligned offset tree
                    let mut aligned_offset_tree_lengths = [0usize; ALIGNED_OFFSET_TREE_ENTRIES];
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
                    Some(aligned_offset_tree)
                };

                // read the pre-tree for the first 256 elements of the main tree
                let pre_tree_main_256 = self.read_pre_tree()?;

                // read the path lengths for the first 256 elements of the main tree using the pre-tree
                let main_256_lengths = self.read_length_delta_tree(&pre_tree_main_256, PreviousLengthType::Main256)?;

                // remember the lengths for next time
                self.last_main_256_lengths.copy_from_slice(&main_256_lengths);

                // same two steps for the rest of the main tree
                let pre_tree_main_rest = self.read_pre_tree()?;
                let main_rest_lengths = self.read_length_delta_tree(&pre_tree_main_rest, PreviousLengthType::MainRest)?;
                self.last_main_rest_lengths.copy_from_slice(&main_rest_lengths);

                // build the main tree
                let mut main_all_lengths = Vec::with_capacity(main_256_lengths.len() + main_rest_lengths.len());
                main_all_lengths.extend_from_slice(&main_256_lengths);
                main_all_lengths.extend_from_slice(&main_rest_lengths);
                let main_tree = match HuffmanTree::new_canonical(&main_all_lengths) {
                    Ok(mt) => mt,
                    Err(e) => {
                        error!("error building main tree: {}", e);
                        return Err(Error::ConstructingMainTree);
                    },
                };

                // build the length tree
                let pre_tree_length = self.read_pre_tree()?;
                let length_lengths = self.read_length_delta_tree(&pre_tree_length, PreviousLengthType::Length)?;
                self.last_length_lengths.copy_from_slice(&length_lengths);
                let length_tree: HuffmanTree<u32> = match HuffmanTree::new_canonical(&length_lengths) {
                    Ok(mt) => mt,
                    Err(e) => {
                        error!("error building length tree: {}", e);
                        return Err(Error::ConstructingLengthTree);
                    },
                };

                debug!("trees are constructed, let's go!");

                let mut bytes_output = 0;
                while bytes_output < num_uncompressed_bytes {
                    // decode an element from the main tree
                    let main_tree_code = main_tree.decode_one_from_bit_reader(&mut self.reader)?
                        .ok_or_else(|| Error::new_eof())?;
                    match main_tree_code {
                        MainTreeCode::LiteralByte(b) => {
                            // this one's easy to handle
                            self.lookback.push(*b);
                            dest_buffer.push(*b);
                            debug!("outputting literal byte {:#04X}", b);
                            bytes_output += 1;
                            self.position_for_jump_translation = self.position_for_jump_translation.wrapping_add(1);
                        },
                        MainTreeCode::Lookback { offset, length_header } => {
                            // okay, how long is the match?
                            let match_length = if length_header.is_max() {
                                // at least 7 but possibly more; decode using the length table
                                let tree_length = length_tree.decode_one_from_bit_reader(&mut self.reader)?
                                    .ok_or_else(|| Error::new_eof())?;
                                *tree_length + 7 + 2
                            } else {
                                u32::from(length_header.as_u8()) + 2
                            };

                            // how far back is it?
                            debug!("encoded offset is {:?}, recents are {:?}", offset, self.recent_lookback);
                            let (match_offset_value, is_absolute) = self.recent_lookback.lookup(*offset);
                            let match_offset = if is_absolute {
                                let position_slot_number = match_offset_value;

                                // okay, how many extra bits do we have in this position?
                                let extra_bit_count = extra_bits(position_slot_number);

                                let (verbatim_bits, aligned_bits) = if let Some(aligned_offset_tree) = aligned_offset_tree_opt.as_ref() {
                                    // aligned block; some of those extra bits might be aligned bits
                                    if extra_bit_count >= 3 {
                                        // the three bottommost bits are aligned bits, the rest are verbatim bits
                                        debug!("{} verbatim bits, 3 aligned bits", extra_bit_count - 3);

                                        // we have max. 17 extra bits, so they will fit into u32
                                        assert!(extra_bit_count <= 17);
                                        let mut verbatim_bits = 0u32;
                                        for _ in 0..(extra_bit_count-3) {
                                            verbatim_bits <<= 1;
                                            if self.reader.read_bit_strict()? {
                                                verbatim_bits |= 1;
                                            }
                                        }

                                        // move the verbatim bits value by three more bits
                                        verbatim_bits <<= 3;

                                        // obtain the aligned bits from the aligned offset tree
                                        let aligned_bits = *aligned_offset_tree.decode_one_from_bit_reader(&mut self.reader)?
                                            .ok_or_else(|| Error::new_eof())?;

                                        (verbatim_bits, aligned_bits)
                                    } else {
                                        // 0..=2 extra bits => no aligned bits
                                        debug!("{} verbatim bits, 0 aligned bits", extra_bit_count);
                                        assert!(extra_bit_count <= 17);
                                        let mut verbatim_bits = 0u32;
                                        for _ in 0..extra_bit_count {
                                            verbatim_bits <<= 1;
                                            if self.reader.read_bit_strict()? {
                                                verbatim_bits |= 1;
                                            }
                                        }

                                        (verbatim_bits, 0)
                                    }
                                } else {
                                    // verbatim block, no aligned bits
                                    debug!("{} verbatim bits, no aligned bits", extra_bit_count);
                                    assert!(extra_bit_count <= 17);
                                    let mut verbatim_bits = 0u32;
                                    for _ in 0..extra_bit_count {
                                        verbatim_bits <<= 1;
                                        if self.reader.read_bit_strict()? {
                                            verbatim_bits |= 1;
                                        }
                                    }

                                    (verbatim_bits, 0)
                                };

                                debug!(
                                    "lookback offset consists of base {} + verbatim {} + aligned {}",
                                    POSITION_SLOT_NUMBER_TO_POSITION_BASE[u32_to_usize(position_slot_number)],
                                    verbatim_bits,
                                    aligned_bits,
                                );

                                let formatted_offset =
                                    POSITION_SLOT_NUMBER_TO_POSITION_BASE[u32_to_usize(position_slot_number)]
                                    + verbatim_bits
                                    + aligned_bits;
                                let actual_match_offset = formatted_offset - 2;
                                debug!("lookback offset is {}", actual_match_offset);

                                // remember this for next time
                                self.recent_lookback.push(actual_match_offset);

                                actual_match_offset
                            } else {
                                // relative offsets are already complete
                                debug!("lookback offset is {} again", match_offset_value);
                                match_offset_value
                            };

                            // gimme
                            let mut buffer = self.lookback.recall(u32_to_usize(match_offset), u32_to_usize(match_length));
                            bytes_output += match_length;
                            self.position_for_jump_translation = self.position_for_jump_translation.wrapping_add(match_length);
                            debug!("outputting lookback bytes: {}", HexBytesSlice::from(buffer.as_slice()));
                            dest_buffer.append(&mut buffer);
                        },
                    }
                    debug!("{}/{} bytes output ({})", bytes_output, num_uncompressed_bytes, DisplayBytesSlice::from(dest_buffer.as_slice()));
                }

                // realign to next 16 bits
                self.reader.drop_rest_of_unit();
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
                let mut recent_buf = [0u8; 4];
                self.reader.read_exact(&mut recent_buf)?;
                self.recent_lookback.r0 = u32::from_le_bytes(recent_buf);
                self.reader.read_exact(&mut recent_buf)?;
                self.recent_lookback.r1 = u32::from_le_bytes(recent_buf);
                self.reader.read_exact(&mut recent_buf)?;
                self.recent_lookback.r2 = u32::from_le_bytes(recent_buf);

                let mut buf = vec![0u8; u32_to_usize(num_uncompressed_bytes)];
                self.reader.read_exact(&mut buf)?;
                self.position_for_jump_translation = self.position_for_jump_translation.wrapping_add(num_uncompressed_bytes);

                debug!("outputting uncompressed bytes: {}", HexBytesSlice::from(buf.as_slice()));
                dest_buffer.append(&mut buf);

                if num_uncompressed_bytes % 2 == 1 {
                    // read an additional byte to realign to u16
                    let mut byte_buf = [0u8];
                    debug!("reading alignment byte");
                    self.reader.read_exact(&mut byte_buf)?;
                }
            },
            other => return Err(Error::UnknownBlockType(other)),
        }

        debug!("orig_dest_buf: {}", display_bytes::DisplayBytesSlice::from(dest_buffer.as_slice()));

        // jump translation?
        if let Some(jump_translation) = self.jump_translation {
            let new_buf_size = usize_to_u32(dest_buffer.len());
            let chunk_size = new_buf_size - original_buf_size;
            let chunk_offset = self.position_for_jump_translation.wrapping_sub(chunk_size);
            if chunk_offset < 0x4000_0000 && chunk_size > 10 {
                let relative_offset = u32_to_usize(original_buf_size);
                let mut relative_i = 0;
                while relative_i < u32_to_usize(chunk_size)-10 {
                    let i = relative_i + relative_offset;
                    if dest_buffer[i] == 0xE8 {
                        debug!("E8 jump translation");
                        let current_pointer_u32 = chunk_offset + usize_to_u32(i);
                        // bit-cast to signed
                        let current_pointer = current_pointer_u32 as i32;
                        debug!("e8 curptr: {0:11} {0:#010X}", current_pointer);
                        let value_u32 =
                            (u32::from(dest_buffer[i+1]) <<  0) |
                            (u32::from(dest_buffer[i+2]) <<  8) |
                            (u32::from(dest_buffer[i+3]) << 16) |
                            (u32::from(dest_buffer[i+4]) << 24);
                        let value = value_u32 as i32;
                        debug!("e8  value: {0:11} {0:#010X}", value);
                        debug!("e8 -curpt: {0:11} {0:#010X}", -current_pointer);
                        debug!("e8 jmptsl: {0:11} {0:#010X}", jump_translation);
                        debug!("e8     if: {:11} ?>=? {:11} && {:11} ?<? {:11}", value, -current_pointer, value, jump_translation);
                        debug!("e8     if: {:#010X} ?>=? {:#010X} && {:#010X} ?<? {:#010X}", value, -current_pointer, value, jump_translation);
                        debug!(
                            "e8     if: {} && {}",
                            if value >= -current_pointer { "true " } else { "false" },
                            if value < jump_translation { "true " } else { "false" },
                        );
                        if value >= -current_pointer && value < jump_translation {
                            debug!("e8 displace!");
                            let displacement_i32 = if value >= 0 {
                                value.wrapping_sub(current_pointer)
                            } else {
                                value.wrapping_add(jump_translation)
                            };
                            debug!("e8 displc: {0:11} {0:#010X}", displacement_i32);
                            let displacement = displacement_i32 as u32;
                            dest_buffer[i+1] = u8::try_from((displacement >>  0) & 0xFF).unwrap();
                            dest_buffer[i+2] = u8::try_from((displacement >>  8) & 0xFF).unwrap();
                            dest_buffer[i+3] = u8::try_from((displacement >> 16) & 0xFF).unwrap();
                            dest_buffer[i+4] = u8::try_from((displacement >> 24) & 0xFF).unwrap();
                        }
                        relative_i += 4;
                    }
                    relative_i += 1;
                }
            }
        }

        debug!("dest_buf: {}", display_bytes::DisplayBytesSlice::from(dest_buffer.as_slice()));

        Ok(())
    }
}
