use core::hash::Hash;

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

macro_rules! impl_num_cmp_with {
    ($t:ty) => {
        impl num_order::NumOrd<$t> for UBig {
            fn num_partial_cmp(&self, other: &$t) -> Option<core::cmp::Ordering> {
                self.partial_cmp(other)
            }
        }
        impl num_order::NumOrd<UBig> for $t {
            fn num_partial_cmp(&self, other: &UBig) -> Option<core::cmp::Ordering> {
                self.partial_cmp(other)
            }
        }
        impl num_order::NumOrd<$t> for IBig {
            fn num_partial_cmp(&self, other: &$t) -> Option<core::cmp::Ordering> {
                self.partial_cmp(other)
            }
        }
        impl num_order::NumOrd<IBig> for $t {
            fn num_partial_cmp(&self, other: &IBig) -> Option<core::cmp::Ordering> {
                self.partial_cmp(other)
            }
        }
    };
}

impl_num_cmp_with!(u8);
impl_num_cmp_with!(u16);
impl_num_cmp_with!(u32);
impl_num_cmp_with!(u64);
impl_num_cmp_with!(u128);
impl_num_cmp_with!(usize);
impl_num_cmp_with!(i8);
impl_num_cmp_with!(i16);
impl_num_cmp_with!(i32);
impl_num_cmp_with!(i64);
impl_num_cmp_with!(i128);
impl_num_cmp_with!(isize);

// TODO(next): implement partial_cmp with f32/f64
