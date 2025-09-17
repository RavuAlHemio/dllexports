//! Floating-point values that are ordered, compared and hashed by their bit patterns.


use core::cmp::Ordering;
use core::hash::{Hash, Hasher};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};


macro_rules! impl_bpf {
    ($name:ident, $fty:ty, $complex_name:ident) => {
        #[derive(Clone, Copy, Debug, Default)]
        #[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
        pub struct $name($fty);
        impl From<$fty> for $name {
            fn from(value: $fty) -> Self {
                Self(value)
            }
        }
        impl From<$name> for $fty {
            fn from(value: $name) -> Self {
                value.0
            }
        }
        impl PartialEq for $name {
            fn eq(&self, other: &Self) -> bool {
                self.0.to_bits() == other.0.to_bits()
            }
        }
        impl Eq for $name {}
        impl Ord for $name {
            fn cmp(&self, other: &Self) -> Ordering {
                self.0.to_bits().cmp(&other.0.to_bits())
            }
        }
        impl PartialOrd for $name {
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                Some(self.cmp(other))
            }
        }
        impl Hash for $name {
            fn hash<H: Hasher>(&self, hasher: &mut H) {
                self.0.to_bits().hash(hasher)
            }
        }

        #[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
        #[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
        pub struct $complex_name {
            pub real: $name,
            pub imag: $name,
        }
    };
}

impl_bpf!(BitPatternF32, f32, ComplexBitPatternF32);
impl_bpf!(BitPatternF64, f64, ComplexBitPatternF64);
