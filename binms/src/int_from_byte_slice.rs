pub trait IntFromByteSlice {
    fn size() -> usize;
    fn from_be_byte_slice(bytes: &[u8]) -> Self;
    fn from_le_byte_slice(bytes: &[u8]) -> Self;
    fn from_ne_byte_slice(bytes: &[u8]) -> Self;
}

macro_rules! impl_ifbs {
    ($type:ty) => {
        impl IntFromByteSlice for $type {
            fn size() -> usize {
                core::mem::size_of::<$type>()
            }

            fn from_be_byte_slice(bytes: &[u8]) -> Self {
                Self::from_be_bytes(bytes.try_into().unwrap())
            }

            fn from_le_byte_slice(bytes: &[u8]) -> Self {
                Self::from_le_bytes(bytes.try_into().unwrap())
            }

            fn from_ne_byte_slice(bytes: &[u8]) -> Self {
                Self::from_ne_bytes(bytes.try_into().unwrap())
            }
        }
    };
}
impl_ifbs!(u8);
impl_ifbs!(u16);
impl_ifbs!(u32);
impl_ifbs!(u64);
impl_ifbs!(u128);
impl_ifbs!(usize);
impl_ifbs!(i8);
impl_ifbs!(i16);
impl_ifbs!(i32);
impl_ifbs!(i64);
impl_ifbs!(i128);
impl_ifbs!(isize);
