mod error;
pub mod fat;
mod huff;
mod io_util;
mod kwaj;
mod ring_buffer;


use std::array::TryFromSliceError;
use std::fmt;
use std::io::{Read, Write};
use std::ops::{Index, IndexMut};

use crate::error::DecompressionError;


#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DisplayBytes<const SIZE: usize>([u8; SIZE]);
impl<const SIZE: usize> Default for DisplayBytes<SIZE> {
    fn default() -> Self {
        let buf = [0u8; SIZE];
        Self(buf)
    }
}
impl<const SIZE: usize> fmt::Debug for DisplayBytes<SIZE> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DisplayBytes({})", self)
    }
}
impl<const SIZE: usize> fmt::Display for DisplayBytes<SIZE> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "b\"")?;
        for &b in &self.0 {
            match b {
                0x00 => write!(f, "\\0")?,
                0x09 => write!(f, "\\t")?,
                0x0A => write!(f, "\\n")?,
                0x0D => write!(f, "\\r")?,
                0x22 => write!(f, "\\\"")?,
                // no need to escape 0x27
                0x5C => write!(f, "\\\\")?,
                0x20..=0x7E => write!(f, "{}", char::from_u32(b.into()).unwrap())?,
                other => write!(f, "\\x{:02X}", other)?,
            }
        }
        write!(f, "\"")
    }
}
impl<const SIZE: usize> From<[u8; SIZE]> for DisplayBytes<SIZE> {
    fn from(value: [u8; SIZE]) -> Self {
        Self(value)
    }
}
impl<const SIZE: usize> From<DisplayBytes<SIZE>> for [u8; SIZE] {
    fn from(value: DisplayBytes<SIZE>) -> Self {
        value.0
    }
}
impl<const SIZE: usize> TryFrom<&[u8]> for DisplayBytes<SIZE> {
    type Error = TryFromSliceError;
    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let buf: [u8; SIZE] = value.try_into()?;
        Ok(Self(buf))
    }
}
impl<const SIZE: usize> AsRef<[u8]> for DisplayBytes<SIZE> {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}
impl<const SIZE: usize> Index<usize> for DisplayBytes<SIZE> {
    type Output = u8;
    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}
impl<const SIZE: usize> IndexMut<usize> for DisplayBytes<SIZE> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.0[index]
    }
}

pub fn decompress<R: Read, W: Write>(
    compressed_reader: &mut R,
    decompressed_writer: &mut W,
) -> Result<(), DecompressionError> {
    let mut magic_buf = [0u8; 8];
    compressed_reader.read_exact(&mut magic_buf)?;
    if &magic_buf == b"KWAJ\x88\xF0\x27\xD1" {
        crate::kwaj::decompress(compressed_reader, decompressed_writer)
    } else {
        Err(DecompressionError::UnknownCompressionMethod)
    }
}
