#[macro_export]
macro_rules! define_part_int_enum {
    (
        $name:ident, $part_int:ty
        $(, $option_value:expr => $option_name:expr)*
        $(,)?
    ) => {
        #[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name($part_int);
        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                match self.0.as_base_type() {
                    $(
                        $option_value => write!(f, $option_name),
                    )*
                    other => write!(f, "Other({})", other),
                }
            }
        }
        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Display::fmt(self, f)
            }
        }
        impl From<$part_int> for $name {
            fn from(value: $part_int) -> Self {
                Self(value)
            }
        }
        impl From<$name> for $part_int {
            fn from(value: $name) -> Self {
                value.0
            }
        }
        #[cfg(feature = "serde")]
        impl serde::Serialize for $name {
            fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                match self.0.as_base_type() {
                    $(
                        $option_value => $option_name.serialize(serializer),
                    )*
                    other => other.to_string().serialize(serializer),
                }
            }
        }
        #[cfg(feature = "serde")]
        impl<'d> serde::Deserialize<'d> for $name {
            fn deserialize<D: serde::Deserializer<'d>>(deserializer: D) -> Result<Self, D::Error> {
                use std::str::FromStr;
                use serde::de::Error as _;

                let stringy: &str = serde::Deserialize::deserialize(deserializer)?;
                let value = match stringy {
                    $(
                        $option_name => Self(<$part_int>::from_base_type($option_value).unwrap()),
                    )*
                    other => {
                        // try parsing as a number
                        let number_base = FromStr::from_str(other)
                            .map_err(|_| D::Error::custom("failed to parse value as constant or number"))?;
                        let number_part = <$part_int>::from_base_type(number_base)
                            .ok_or_else(|| D::Error::custom("numeric value out of range"))?;
                        Self(number_part)
                    },
                };
                Ok(value)
            }
        }
    };
}
