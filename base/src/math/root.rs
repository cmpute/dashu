use super::{CubicRoot, SquareRoot};
use crate::{CubicRootRem, NormalizedRootRem, SquareRootRem};

// TODO(next): forward sqrt to f32/f64 if std is enabled and the input is small enough.
//             Implement after we have a benchmark. See https://github.com/Aatch/ramp/blob/master/src/int.rs#L579.

impl SquareRoot for u8 {
    type Output = u8;

    #[inline]
    fn sqrt(&self) -> Self::Output {
        self.sqrt_rem().0
    }
}

impl CubicRoot for u8 {
    type Output = u8;

    #[inline]
    fn cbrt(&self) -> Self::Output {
        self.cbrt_rem().0
    }
}

macro_rules! impl_root_using_rootrem {
    ($t:ty, $half:ty) => {
        impl SquareRoot for $t {
            type Output = $half;

            #[inline]
            fn sqrt(&self) -> $half {
                if *self == 0 {
                    return 0;
                }

                // normalize the input and call the normalized subroutine
                let shift = self.leading_zeros() & !1; // make sure shift is divisible by 2
                let (root, _) = (self << shift).normalized_sqrt_rem();
                root >> (shift / 2)
            }
        }

        impl CubicRoot for $t {
            type Output = $half;

            #[inline]
            fn cbrt(&self) -> $half {
                if *self == 0 {
                    return 0;
                }

                // normalize the input and call the normalized subroutine
                let mut shift = self.leading_zeros();
                shift -= shift % 3; // make sure shift is divisible by 3
                let (root, _) = (self << shift).normalized_cbrt_rem();
                root >> (shift / 3)
            }
        }
    };
}

impl_root_using_rootrem!(u16, u8);
impl_root_using_rootrem!(u32, u16);
impl_root_using_rootrem!(u64, u32);
impl_root_using_rootrem!(u128, u64);
