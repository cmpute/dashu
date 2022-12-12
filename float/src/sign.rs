use crate::{
    fbig::FBig,
    repr::{Context, Repr, Word},
    round::Round,
};
use core::ops::{Mul, MulAssign, Neg};
use dashu_base::{Abs, Sign, Signed};
use dashu_int::IBig;

impl<R: Round, const B: Word> FBig<R, B> {
    /// Get the sign of the number. Zero value has a positive sign.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_base::{ParseError, Sign};
    /// # use dashu_float::DBig;
    /// assert_eq!(DBig::ZERO.sign(), Sign::Positive);
    /// assert_eq!(DBig::from_str_native("-1.234")?.sign(), Sign::Negative);
    /// # Ok::<(), ParseError>(())
    /// ```
    #[inline]
    pub const fn sign(&self) -> Sign {
        self.repr.sign()
    }

    /// A number representing the sign of `self`.
    ///
    /// * [FBig::ONE] if the number is positive (including `inf`)
    /// * [FBig::ZERO] if the number is zero
    /// * [FBig::NEG_ONE] if the number is negative (including `-inf`)
    ///
    /// # Examples
    /// ```
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// assert_eq!(DBig::from_str_native("2.01")?.signum(), DBig::ONE);
    /// assert_eq!(DBig::from_str_native("-1.234")?.signum(), DBig::NEG_ONE);
    /// # Ok::<(), ParseError>(())
    /// ```
    pub const fn signum(&self) -> Self {
        let significand = if self.repr.significand.is_zero() && self.repr.exponent != 0 {
            if self.repr.exponent > 0 {
                IBig::ONE
            } else {
                IBig::NEG_ONE
            }
        } else {
            self.repr.significand.signum()
        };
        let repr = Repr {
            significand,
            exponent: 0,
        };
        Self::new(repr, Context::new(1))
    }
}

impl<const B: Word> Neg for Repr<B> {
    type Output = Self;
    #[inline]
    fn neg(mut self) -> Self::Output {
        self.significand = -self.significand;
        self
    }
}

impl<R: Round, const B: Word> Neg for FBig<R, B> {
    type Output = Self;
    #[inline]
    fn neg(mut self) -> Self::Output {
        self.repr.significand = -self.repr.significand;
        self
    }
}

impl<R: Round, const B: Word> Neg for &FBig<R, B> {
    type Output = FBig<R, B>;
    #[inline]
    fn neg(self) -> Self::Output {
        self.clone().neg()
    }
}

impl<R: Round, const B: Word> Abs for FBig<R, B> {
    type Output = Self;
    fn abs(mut self) -> Self::Output {
        self.repr.significand = self.repr.significand.abs();
        self
    }
}

impl<R: Round, const B: Word> Mul<FBig<R, B>> for Sign {
    type Output = FBig<R, B>;
    #[inline]
    fn mul(self, mut rhs: FBig<R, B>) -> Self::Output {
        rhs.repr.significand *= self;
        rhs
    }
}

impl<R: Round, const B: Word> Mul<Sign> for FBig<R, B> {
    type Output = FBig<R, B>;
    #[inline]
    fn mul(mut self, rhs: Sign) -> Self::Output {
        self.repr.significand *= rhs;
        self
    }
}

impl<R: Round, const B: Word> MulAssign<Sign> for FBig<R, B> {
    #[inline]
    fn mul_assign(&mut self, rhs: Sign) {
        self.repr.significand *= rhs;
    }
}

impl<R: Round, const B: Word> Signed for FBig<R, B> {
    #[inline]
    fn sign(&self) -> Sign {
        self.repr.sign()
    }
}
