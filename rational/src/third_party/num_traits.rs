//! Implement num-traits traits.

use crate::rbig::{RBig, Relaxed};
use dashu_base::{Abs, DivEuclid, ParseError, RemEuclid, Sign};
use num_traits_v02 as num_traits;

macro_rules! impl_num_traits {
    ($t:ty) => {
        impl num_traits::Zero for $t {
            #[inline]
            fn zero() -> Self {
                <$t>::ZERO
            }
            #[inline]
            fn is_zero(&self) -> bool {
                <$t>::is_zero(self)
            }
        }

        impl num_traits::One for $t {
            #[inline]
            fn one() -> Self {
                <$t>::ONE
            }
            #[inline]
            fn is_one(&self) -> bool {
                <$t>::is_one(self)
            }
        }

        impl num_traits::Num for $t {
            type FromStrRadixErr = ParseError;
            #[inline]
            fn from_str_radix(src: &str, radix: u32) -> Result<Self, Self::FromStrRadixErr> {
                <$t>::from_str_radix(src, radix)
            }
        }

        impl num_traits::Signed for $t {
            #[inline]
            fn abs(&self) -> Self {
                Abs::abs(self.clone())
            }

            #[inline]
            fn abs_sub(&self, other: &Self) -> Self {
                Abs::abs(self - other)
            }

            #[inline]
            fn signum(&self) -> Self {
                <$t>::signum(self)
            }

            #[inline]
            fn is_positive(&self) -> bool {
                !self.is_zero() && self.sign() == Sign::Positive
            }

            #[inline]
            fn is_negative(&self) -> bool {
                self.sign() == Sign::Negative
            }
        }

        impl num_traits::Euclid for $t {
            #[inline]
            fn div_euclid(&self, v: &Self) -> Self {
                DivEuclid::div_euclid(self, v).into()
            }
            #[inline]
            fn rem_euclid(&self, v: &Self) -> Self {
                RemEuclid::rem_euclid(self, v)
            }
        }

        impl num_traits::Pow<usize> for $t {
            type Output = $t;
            #[inline]
            fn pow(self, rhs: usize) -> $t {
                <$t>::pow(&self, rhs)
            }
        }
        impl num_traits::Pow<usize> for &$t {
            type Output = $t;
            #[inline]
            fn pow(self, rhs: usize) -> $t {
                <$t>::pow(self, rhs)
            }
        }
    };
}
impl_num_traits!(RBig);
impl_num_traits!(Relaxed);

macro_rules! impl_from_primitive_int {
    ($t:ty, $method:ident) => {
        #[inline]
        fn $method(n: $t) -> Option<Self> {
            Some(Self::from(n))
        }
    };
}

impl num_traits::FromPrimitive for RBig {
    impl_from_primitive_int!(i8, from_i8);
    impl_from_primitive_int!(i16, from_i16);
    impl_from_primitive_int!(i32, from_i32);
    impl_from_primitive_int!(i64, from_i64);
    impl_from_primitive_int!(i128, from_i128);
    impl_from_primitive_int!(isize, from_isize);
    impl_from_primitive_int!(u8, from_u8);
    impl_from_primitive_int!(u16, from_u16);
    impl_from_primitive_int!(u32, from_u32);
    impl_from_primitive_int!(u64, from_u64);
    impl_from_primitive_int!(u128, from_u128);
    impl_from_primitive_int!(usize, from_usize);

    #[inline]
    fn from_f32(f: f32) -> Option<Self> {
        Self::try_from(f).ok()
    }
    #[inline]
    fn from_f64(f: f64) -> Option<Self> {
        Self::try_from(f).ok()
    }
}

impl num_traits::FromPrimitive for Relaxed {
    impl_from_primitive_int!(i8, from_i8);
    impl_from_primitive_int!(i16, from_i16);
    impl_from_primitive_int!(i32, from_i32);
    impl_from_primitive_int!(i64, from_i64);
    impl_from_primitive_int!(i128, from_i128);
    impl_from_primitive_int!(isize, from_isize);
    impl_from_primitive_int!(u8, from_u8);
    impl_from_primitive_int!(u16, from_u16);
    impl_from_primitive_int!(u32, from_u32);
    impl_from_primitive_int!(u64, from_u64);
    impl_from_primitive_int!(u128, from_u128);
    impl_from_primitive_int!(usize, from_usize);

    #[inline]
    fn from_f32(f: f32) -> Option<Self> {
        Self::try_from(f).ok()
    }
    #[inline]
    fn from_f64(f: f64) -> Option<Self> {
        Self::try_from(f).ok()
    }
}

macro_rules! impl_to_primitive_int {
    ($t:ty, $method:ident) => {
        #[inline]
        fn $method(&self) -> Option<$t> {
            num_traits::ToPrimitive::$method(&self.to_int().value())
        }
    };
}

impl num_traits::ToPrimitive for RBig {
    impl_to_primitive_int!(i8, to_i8);
    impl_to_primitive_int!(i16, to_i16);
    impl_to_primitive_int!(i32, to_i32);
    impl_to_primitive_int!(i64, to_i64);
    impl_to_primitive_int!(i128, to_i128);
    impl_to_primitive_int!(isize, to_isize);
    impl_to_primitive_int!(u8, to_u8);
    impl_to_primitive_int!(u16, to_u16);
    impl_to_primitive_int!(u32, to_u32);
    impl_to_primitive_int!(u64, to_u64);
    impl_to_primitive_int!(u128, to_u128);
    impl_to_primitive_int!(usize, to_usize);

    #[inline]
    fn to_f32(&self) -> Option<f32> {
        Some(self.to_f32().value())
    }
    #[inline]
    fn to_f64(&self) -> Option<f64> {
        Some(self.to_f64().value())
    }
}

impl num_traits::ToPrimitive for Relaxed {
    impl_to_primitive_int!(i8, to_i8);
    impl_to_primitive_int!(i16, to_i16);
    impl_to_primitive_int!(i32, to_i32);
    impl_to_primitive_int!(i64, to_i64);
    impl_to_primitive_int!(i128, to_i128);
    impl_to_primitive_int!(isize, to_isize);
    impl_to_primitive_int!(u8, to_u8);
    impl_to_primitive_int!(u16, to_u16);
    impl_to_primitive_int!(u32, to_u32);
    impl_to_primitive_int!(u64, to_u64);
    impl_to_primitive_int!(u128, to_u128);
    impl_to_primitive_int!(usize, to_usize);

    #[inline]
    fn to_f32(&self) -> Option<f32> {
        Some(self.to_f32().value())
    }
    #[inline]
    fn to_f64(&self) -> Option<f64> {
        Some(self.to_f64().value())
    }
}
