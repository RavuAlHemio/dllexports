use std::array::TryFromSliceError;
use std::fmt;
use std::ops::{Index, IndexMut};


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


#[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DisplayBytesVec(Vec<u8>);
impl Default for DisplayBytesVec {
    fn default() -> Self {
        Self(Vec::new())
    }
}
impl fmt::Debug for DisplayBytesVec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DisplayBytesVec({})", self)
    }
}
impl fmt::Display for DisplayBytesVec {
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
impl From<Vec<u8>> for DisplayBytesVec {
    fn from(value: Vec<u8>) -> Self {
        Self(value)
    }
}
impl From<DisplayBytesVec> for Vec<u8> {
    fn from(value: DisplayBytesVec) -> Self {
        value.0
    }
}
impl From<&[u8]> for DisplayBytesVec {
    fn from(value: &[u8]) -> Self {
        let buf: Vec<u8> = value.into();
        Self(buf)
    }
}
impl AsRef<Vec<u8>> for DisplayBytesVec {
    fn as_ref(&self) -> &Vec<u8> {
        &self.0
    }
}
impl AsRef<[u8]> for DisplayBytesVec {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}
impl Index<usize> for DisplayBytesVec {
    type Output = u8;
    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}
impl IndexMut<usize> for DisplayBytesVec {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.0[index]
    }
}



#[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DisplayBytesSlice<'a>(&'a [u8]);
impl<'a> Default for DisplayBytesSlice<'a> {
    fn default() -> Self {
        Self(&[])
    }
}
impl<'a> fmt::Debug for DisplayBytesSlice<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DisplayBytesSlice({})", self)
    }
}
impl<'a> fmt::Display for DisplayBytesSlice<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "b\"")?;
        for &b in self.0 {
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
impl<'a> From<DisplayBytesSlice<'a>> for &'a [u8] {
    fn from(value: DisplayBytesSlice<'a>) -> Self {
        value.0
    }
}
impl<'a> From<&'a [u8]> for DisplayBytesSlice<'a> {
    fn from(value: &'a [u8]) -> Self {
        Self(value)
    }
}
impl<'a> AsRef<[u8]> for DisplayBytesSlice<'a> {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}
impl<'a> Index<usize> for DisplayBytesSlice<'a> {
    type Output = u8;
    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}
