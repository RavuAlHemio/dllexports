pub mod header;
pub mod resources;


use std::fmt;


#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Error {
    TooShort,
    Size { expected_at_least: u32, obtained: u32 },
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooShort
                => write!(f, "input buffer is too short to read header"),
            Self::Size { expected_at_least, obtained }
                => write!(f, "CLR header has unexpected size (expected at least {}, obtained {})", expected_at_least, obtained),
        }
    }
}
impl std::error::Error for Error {
}


#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct AddressAndLength32 {
    pub address: u32,
    pub length: u32,
}
impl AddressAndLength32 {
    pub fn take_from_bytes(bytes: &[u8]) -> Result<(&[u8], Self), Error> {
        if bytes.len() < 8 {
            return Err(Error::TooShort);
        }
        let address = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
        let length = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
        Ok((
            &bytes[8..],
            Self {
                address,
                length,
            },
        ))
    }
}
