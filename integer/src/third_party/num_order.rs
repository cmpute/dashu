use core::{hash::Hash, cmp::Ordering};

use crate::{ibig::IBig, ubig::UBig};

impl num_order::NumHash for UBig {
    fn num_hash<H: core::hash::Hasher>(&self, state: &mut H) {
        let m = self % (i128::MAX as u128);
        (m as i128).hash(state)
    }
}
impl num_order::NumHash for IBig {
    fn num_hash<H: core::hash::Hasher>(&self, state: &mut H) {
        (self % i128::MAX).hash(state)
    }
}

// TODO(next): implement partial_cmp with f32/f64

macro_rules! impl_num_cmp_ubig_with_unsigned {
    ($($t:ty)*) => {$(
        impl num_order::NumOrd<$t> for UBig {
            #[inline]
            fn num_partial_cmp(&self, other: &$t) -> Option<Ordering> {
                self.partial_cmp(&UBig::from_unsigned(*other))
            }
        }
        impl num_order::NumOrd<UBig> for $t {
            #[inline]
            fn num_partial_cmp(&self, other: &UBig) -> Option<Ordering> {
                UBig::from_unsigned(*self).partial_cmp(other)
            }
        }
    )*};
}
impl_num_cmp_ubig_with_unsigned!(u8 u16 u32 u64 u128 usize);

macro_rules! impl_num_cmp_ubig_with_signed {
    ($($t:ty)*) => {$(
        impl num_order::NumOrd<$t> for UBig {
            #[inline]
            fn num_partial_cmp(&self, other: &$t) -> Option<Ordering> {
                self.partial_cmp(&IBig::from_signed(*other))
            }
        }
        impl num_order::NumOrd<UBig> for $t {
            #[inline]
            fn num_partial_cmp(&self, other: &UBig) -> Option<Ordering> {
                IBig::from_signed(*self).partial_cmp(other)
            }
        }
    )*};
}
impl_num_cmp_ubig_with_signed!(i8 i16 i32 i64 i128 isize);

macro_rules! impl_num_cmp_ibig_with_unsigned {
    ($($t:ty)*) => {$(
        impl num_order::NumOrd<$t> for IBig {
            #[inline]
            fn num_partial_cmp(&self, other: &$t) -> Option<Ordering> {
                self.partial_cmp(&IBig::from_unsigned(*other))
            }
        }
        impl num_order::NumOrd<IBig> for $t {
            #[inline]
            fn num_partial_cmp(&self, other: &IBig) -> Option<Ordering> {
                IBig::from_unsigned(*self).partial_cmp(other)
            }
        }
    )*};
}
impl_num_cmp_ibig_with_unsigned!(u8 u16 u32 u64 u128 usize);

macro_rules! impl_num_cmp_ibig_with_signed {
    ($($t:ty)*) => {$(
        impl num_order::NumOrd<$t> for IBig {
            #[inline]
            fn num_partial_cmp(&self, other: &$t) -> Option<Ordering> {
                self.partial_cmp(&IBig::from_signed(*other))
            }
        }
        impl num_order::NumOrd<IBig> for $t {
            #[inline]
            fn num_partial_cmp(&self, other: &IBig) -> Option<Ordering> {
                IBig::from_signed(*self).partial_cmp(other)
            }
        }
    )*};
}
impl_num_cmp_ibig_with_signed!(i8 i16 i32 i64 i128 isize);
