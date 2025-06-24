//! The Microsoft LZX format.
//!
//! [Microsoft's documentation][mscabdoc] describes most of the format, but the `mspack`
//! implementation found [a few inconsistencies][mspacklzx].
//!
//! [mscabdoc]: https://learn.microsoft.com/en-us/previous-versions/bb417343(v=msdn.10)
//! [mspacklzx]: https://github.com/kyz/libmspack/blob/305907723a4e7ab2018e58040059ffb5e77db837/libmspack/mspack/lzxd.c#L18


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
