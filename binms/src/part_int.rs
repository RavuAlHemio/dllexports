use std::fmt;


#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PartIntError<T> {
    value: T,
}
impl<T: fmt::Display> fmt::Display for PartIntError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "failed to convert: value {} too large", self.value)
    }
}
impl<T: fmt::Display + fmt::Debug> std::error::Error for PartIntError<T> {
}

macro_rules! define_part_int {
    ($name:ident, $base_type:ty, $bit_count:expr) => {
        #[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name($base_type);
        impl $name {
            pub const fn from_base_type(value: $base_type) -> Option<Self> {
                if value < (1 << $bit_count) {
                    Some(Self(value))
                } else {
                    None
                }
            }
            pub const fn as_base_type(&self) -> $base_type { self.0 }
        }
        impl TryFrom<$base_type> for $name {
            type Error = PartIntError<$base_type>;
            fn try_from(value: $base_type) -> Result<Self, Self::Error> {
                Self::from_base_type(value)
                    .ok_or(PartIntError { value })
            }
        }
        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        #[cfg(feature = "serde")]
        impl serde::Serialize for $name {
            fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                self.0.serialize(serializer)
            }
        }

        #[cfg(feature = "serde")]
        impl<'d> serde::Deserialize<'d> for $name {
            fn deserialize<D: serde::Deserializer<'d>>(deserializer: D) -> Result<Self, D::Error> {
                use serde::de::Error as _;

                let base_value = <$base_type>::deserialize(deserializer)?;
                Self::from_base_type(base_value)
                    .ok_or_else(|| D::Error::custom("out-of-range value"))
            }
        }
    };
}

define_part_int!(U3, u8, 3);
define_part_int!(U4, u8, 4);
