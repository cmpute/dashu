use super::{CubicRoot, SquareRoot};
use crate::{CubicRootRem, NormalizedRootRem, SquareRootRem};

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

// f64::sqrt is reliable for floor(sqrt(x)) when x < (2^26+1)^2.
// Beyond that, IEEE 754 rounding can push the result across an integer
// boundary (e.g. f64::sqrt(67108865^2 - 1) rounds up to 67108865.0).
#[cfg(feature = "std")]
const F64_SQRT_THRESHOLD: u128 = 4503599761588224; // (2^26 + 1)^2 - 1

// When std is available, use native float sqrt for each primitive type.
// For u16, f32 is sufficient (24-bit mantissa covers all u16 values exactly).
// For u32/u64/u128, f64 is used with the [F64_SQRT_THRESHOLD] guard.

#[cfg(feature = "std")]
impl SquareRoot for u16 {
    type Output = u8;

    #[inline]
    fn sqrt(&self) -> u8 {
        if *self == 0 {
            return 0;
        }
        (*self as f32).sqrt().floor() as u8
    }
}

#[cfg(feature = "std")]
impl SquareRoot for u32 {
    type Output = u16;

    #[inline]
    fn sqrt(&self) -> u16 {
        if *self == 0 {
            return 0;
        }
        (*self as f64).sqrt().floor() as u16
    }
}

#[cfg(feature = "std")]
impl SquareRoot for u64 {
    type Output = u32;

    #[inline]
    fn sqrt(&self) -> u32 {
        if *self == 0 {
            return 0;
        }
        // u64 values above the threshold fall back to the bitwise algorithm.
        if (*self as u128) < F64_SQRT_THRESHOLD {
            return (*self as f64).sqrt().floor() as u32;
        }

        let shift = self.leading_zeros() & !1; // make sure shift is divisible by 2
        let (root, _) = (self << shift).normalized_sqrt_rem();
        root >> (shift / 2)
    }
}

#[cfg(feature = "std")]
impl SquareRoot for u128 {
    type Output = u64;

    #[inline]
    fn sqrt(&self) -> u64 {
        if *self == 0 {
            return 0;
        }
        if *self < F64_SQRT_THRESHOLD {
            return (*self as f64).sqrt().floor() as u64;
        }

        let shift = self.leading_zeros() & !1; // make sure shift is divisible by 2
        let (root, _) = (self << shift).normalized_sqrt_rem();
        root >> (shift / 2)
    }
}

// When std is not available, implement SquareRoot through a macro using the
// bitwise normalization approach.
#[cfg(not(feature = "std"))]
macro_rules! impl_sqrt_using_rootrem {
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
    };
}

#[cfg(not(feature = "std"))]
impl_sqrt_using_rootrem!(u16, u8);
#[cfg(not(feature = "std"))]
impl_sqrt_using_rootrem!(u32, u16);
#[cfg(not(feature = "std"))]
impl_sqrt_using_rootrem!(u64, u32);
#[cfg(not(feature = "std"))]
impl_sqrt_using_rootrem!(u128, u64);

// CubicRoot for all types above u8 always uses the bitwise approach.
macro_rules! impl_cbrt_using_cbrtrem {
    ($t:ty, $half:ty) => {
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

impl_cbrt_using_cbrtrem!(u16, u8);
impl_cbrt_using_cbrtrem!(u32, u16);
impl_cbrt_using_cbrtrem!(u64, u32);
impl_cbrt_using_cbrtrem!(u128, u64);
