use std::fmt;


#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Error {
    TooShort,
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            Self::TooShort
                => write!(f, "icon list data too short"),
        }
    }
}
impl std::error::Error for Error {
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct IconGroup {
    pub reserved: u16, // 0x0000
    pub group_type: u16, // 0x0001
    // count: u16,
    pub icons: Vec<GroupIcon>, // [GroupIcon; count]
}
impl IconGroup {
    pub fn take_from_bytes(bytes: &[u8]) -> Result<(&[u8], Self), Error> {
        if bytes.len() < 6 {
            return Err(Error::TooShort);
        }

        let rest = bytes;

        let reserved = u16::from_le_bytes(rest[0..2].try_into().unwrap());
        let group_type = u16::from_le_bytes(rest[2..4].try_into().unwrap());
        let count = u16::from_le_bytes(rest[4..6].try_into().unwrap());

        let mut rest = &rest[6..];

        let count_usize: usize = count.try_into().unwrap();
        if rest.len() < count_usize * 14 {
            return Err(Error::TooShort);
        }

        let mut icons = Vec::with_capacity(count_usize);
        for i in 0..count_usize {
            let (new_rest, icon) = GroupIcon::take_from_bytes(rest)?;
            rest = new_rest;
            icons.push(icon);
        }

        let icon_group = Self {
            reserved,
            group_type,
            icons,
        };
        Ok((rest, icon_group))
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct GroupIcon {
    pub width: u8,
    pub height: u8,
    pub color_count: u8,
    pub reserved: u8,
    pub planes: u16,
    pub bit_count: u16,
    pub byte_count: u32,
    pub id: u16,
}
impl GroupIcon {
    pub fn take_from_bytes(bytes: &[u8]) -> Result<(&[u8], Self), Error> {
        if bytes.len() < 14 {
            return Err(Error::TooShort);
        }

        let width = bytes[0];
        let height = bytes[1];
        let color_count = bytes[2];
        let reserved = bytes[3];
        let planes = u16::from_le_bytes(bytes[4..6].try_into().unwrap());
        let bit_count = u16::from_le_bytes(bytes[6..8].try_into().unwrap());
        let byte_count = u32::from_le_bytes(bytes[8..12].try_into().unwrap());
        let id = u16::from_le_bytes(bytes[12..14].try_into().unwrap());

        let icon = GroupIcon {
            width,
            height,
            color_count,
            reserved,
            planes,
            bit_count,
            byte_count,
            id,
        };
        Ok((&bytes[14..], icon))
    }
}
