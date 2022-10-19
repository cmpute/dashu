//! Implement num-traits traits.

use crate::{error::ParseError, ibig::IBig, ops::Abs, ubig::UBig, Sign};

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
