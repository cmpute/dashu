//! Implement num-traits traits.

use crate::{ibig::IBig, ops::Abs, ubig::UBig, Sign};
use dashu_base::{DivEuclid, ParseError, RemEuclid};
use num_traits_v02 as num_traits;

impl num_traits::Zero for UBig {
    #[inline]
    fn zero() -> Self {
        UBig::ZERO
    }
    #[inline]
    fn is_zero(&self) -> bool {
        UBig::is_zero(self)
    }
}

impl num_traits::Zero for IBig {
    #[inline]
    fn zero() -> Self {
        IBig::ZERO
    }
    #[inline]
    fn is_zero(&self) -> bool {
        IBig::is_zero(self)
    }
}

impl num_traits::One for UBig {
    #[inline]
    fn one() -> Self {
        UBig::ONE
    }
    #[inline]
    fn is_one(&self) -> bool {
        UBig::is_one(self)
    }
}

impl num_traits::One for IBig {
    #[inline]
    fn one() -> Self {
        IBig::ONE
    }
    #[inline]
    fn is_one(&self) -> bool {
        IBig::is_one(self)
    }
}

impl num_traits::Pow<usize> for UBig {
    type Output = UBig;

    #[inline]
    #[allow(clippy::needless_borrow)]
    fn pow(self, rhs: usize) -> UBig {
        (&self).pow(rhs)
    }
}

impl num_traits::Pow<usize> for &UBig {
    type Output = UBig;

    #[inline]
    fn pow(self, rhs: usize) -> UBig {
        self.pow(rhs)
    }
}

impl num_traits::Pow<usize> for IBig {
    type Output = IBig;

    #[inline]
    #[allow(clippy::needless_borrow)]
    fn pow(self, rhs: usize) -> IBig {
        (&self).pow(rhs)
    }
}

impl num_traits::Pow<usize> for &IBig {
    type Output = IBig;

    #[inline]
    fn pow(self, rhs: usize) -> IBig {
        self.pow(rhs)
    }
}

impl num_traits::Unsigned for UBig {}

impl num_traits::Signed for IBig {
    #[inline]
    fn abs(&self) -> Self {
        Abs::abs(self)
    }

    #[inline]
    fn abs_sub(&self, other: &Self) -> Self {
        Abs::abs(self - other)
    }

    #[inline]
    fn signum(&self) -> Self {
        self.signum()
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

impl num_traits::Num for UBig {
    type FromStrRadixErr = ParseError;

    fn from_str_radix(s: &str, radix: u32) -> Result<Self, ParseError> {
        Self::from_str_radix(s, radix)
    }
}

impl num_traits::Num for IBig {
    type FromStrRadixErr = ParseError;

    fn from_str_radix(s: &str, radix: u32) -> Result<Self, ParseError> {
        Self::from_str_radix(s, radix)
    }
}

impl num_traits::Euclid for UBig {
    #[inline]
    fn div_euclid(&self, v: &Self) -> Self {
        DivEuclid::div_euclid(self, v)
    }
    #[inline]
    fn rem_euclid(&self, v: &Self) -> Self {
        RemEuclid::rem_euclid(self, v)
    }
}

impl num_traits::Euclid for IBig {
    #[inline]
    fn div_euclid(&self, v: &Self) -> Self {
        DivEuclid::div_euclid(self, v)
    }
    #[inline]
    fn rem_euclid(&self, v: &Self) -> Self {
        RemEuclid::rem_euclid(self, v).into()
    }
}

macro_rules! impl_to_primitive_int {
    ($t:ty, $method:ident) => {
        #[inline]
        fn $method(&self) -> Option<$t> {
            self.try_into().ok()
        }
    };
}

impl num_traits::ToPrimitive for UBig {
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

impl num_traits::ToPrimitive for IBig {
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

macro_rules! impl_from_primitive_int {
    ($t:ty, $method:ident) => {
        #[inline]
        fn $method(n: $t) -> Option<Self> {
            Some(n.into())
        }
    };
}

macro_rules! impl_unsigned_from_primitive_int {
    ($t:ty, $method:ident) => {
        #[inline]
        fn $method(n: $t) -> Option<Self> {
            n.try_into().ok()
        }
    };
}

impl num_traits::FromPrimitive for UBig {
    impl_unsigned_from_primitive_int!(i8, from_i8);
    impl_unsigned_from_primitive_int!(i16, from_i16);
    impl_unsigned_from_primitive_int!(i32, from_i32);
    impl_unsigned_from_primitive_int!(i64, from_i64);
    impl_unsigned_from_primitive_int!(i128, from_i128);
    impl_unsigned_from_primitive_int!(isize, from_isize);
    impl_from_primitive_int!(u8, from_u8);
    impl_from_primitive_int!(u16, from_u16);
    impl_from_primitive_int!(u32, from_u32);
    impl_from_primitive_int!(u64, from_u64);
    impl_from_primitive_int!(u128, from_u128);
    impl_from_primitive_int!(usize, from_usize);

    #[inline]
    fn from_f32(n: f32) -> Option<Self> {
        n.try_into().ok()
    }
    #[inline]
    fn from_f64(n: f64) -> Option<Self> {
        n.try_into().ok()
    }
}

impl num_traits::FromPrimitive for IBig {
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
    fn from_f32(n: f32) -> Option<Self> {
        n.try_into().ok()
    }
    #[inline]
    fn from_f64(n: f64) -> Option<Self> {
        n.try_into().ok()
    }
}
