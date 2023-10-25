//! Implement num-traits traits.

use dashu_base::{DivEuclid, ParseError, RemEuclid};

use crate::{ibig::IBig, ops::Abs, ubig::UBig, Sign};

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

impl num_traits::ToPrimitive for UBig {
    #[inline]
    fn to_u64(&self) -> Option<u64> {
        self.try_into().ok()
    }
    #[inline]
    fn to_i64(&self) -> Option<i64> {
        self.try_into().ok()
    }
    #[inline]
    fn to_u128(&self) -> Option<u128> {
        self.try_into().ok()
    }
    #[inline]
    fn to_i128(&self) -> Option<i128> {
        self.try_into().ok()
    }
    #[inline]
    fn to_f64(&self) -> Option<f64> {
        Some(self.to_f64().value())
    }
}

impl num_traits::ToPrimitive for IBig {
    #[inline]
    fn to_u64(&self) -> Option<u64> {
        self.try_into().ok()
    }
    #[inline]
    fn to_i64(&self) -> Option<i64> {
        self.try_into().ok()
    }
    #[inline]
    fn to_u128(&self) -> Option<u128> {
        self.try_into().ok()
    }
    #[inline]
    fn to_i128(&self) -> Option<i128> {
        self.try_into().ok()
    }
    #[inline]
    fn to_f64(&self) -> Option<f64> {
        Some(self.to_f64().value())
    }
}

impl num_traits::FromPrimitive for UBig {
    #[inline]
    fn from_u64(n: u64) -> Option<Self> {
        n.try_into().ok()
    }
    #[inline]
    fn from_i64(n: i64) -> Option<Self> {
        n.try_into().ok()
    }
    #[inline]
    fn from_i128(n: i128) -> Option<Self> {
        n.try_into().ok()
    }
    #[inline]
    fn from_u128(n: u128) -> Option<Self> {
        n.try_into().ok()
    }
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
    #[inline]
    fn from_u64(n: u64) -> Option<Self> {
        n.try_into().ok()
    }
    #[inline]
    fn from_i64(n: i64) -> Option<Self> {
        n.try_into().ok()
    }
    #[inline]
    fn from_i128(n: i128) -> Option<Self> {
        n.try_into().ok()
    }
    #[inline]
    fn from_u128(n: u128) -> Option<Self> {
        n.try_into().ok()
    }
    #[inline]
    fn from_f32(n: f32) -> Option<Self> {
        n.try_into().ok()
    }
    #[inline]
    fn from_f64(n: f64) -> Option<Self> {
        n.try_into().ok()
    }
}
