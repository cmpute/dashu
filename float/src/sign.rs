use crate::{
    fbig::FBig,
    repr::{Context, Repr, Word},
    round::Round,
};
use core::ops::{Mul, MulAssign, Neg};
use dashu_base::{Abs, Sign, Signed};
use dashu_int::IBig;

impl<R: Round, const B: Word> FBig<R, B> {
    /// Get the sign of the number. Positive zero has a positive sign, negative zero has a
    /// negative sign.
    ///
    /// # Examples
    ///
    /// ```
    /// # use core::str::FromStr;
    /// # use dashu_base::{ParseError, Sign};
    /// # use dashu_float::DBig;
    /// assert_eq!(DBig::ZERO.sign(), Sign::Positive);
    /// assert_eq!(DBig::from_str("-1.234")?.sign(), Sign::Negative);
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
    /// # use core::str::FromStr;
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// assert_eq!(DBig::from_str("2.01")?.signum(), DBig::ONE);
    /// assert_eq!(DBig::from_str("-1.234")?.signum(), DBig::NEG_ONE);
    /// # Ok::<(), ParseError>(())
    /// ```
    pub const fn signum(&self) -> Self {
        let significand = if self.repr.significand.is_zero() {
            // distinguish infinities from signed zero; signum(±0) = +0
            match self.repr.exponent {
                isize::MAX => IBig::ONE,
                isize::MIN => IBig::NEG_ONE,
                _ => IBig::ZERO,
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
    fn neg(self) -> Self::Output {
        Repr::neg(self)
    }
}

impl<R: Round, const B: Word> Neg for FBig<R, B> {
    type Output = Self;
    #[inline]
    fn neg(mut self) -> Self::Output {
        self.repr = self.repr.neg();
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
        // flip -0 -> +0 and -inf -> +inf by toggling the special-value exponent;
        // finite values take the absolute value of their significand.
        if self.repr.significand.is_zero() {
            if self.repr.exponent == -1 {
                self.repr.exponent = 0;
            } else if self.repr.exponent == isize::MIN {
                self.repr.exponent = isize::MAX;
            }
        } else {
            self.repr.significand = self.repr.significand.abs();
        }
        self
    }
}

impl<R: Round, const B: Word> Mul<FBig<R, B>> for Sign {
    type Output = FBig<R, B>;
    #[inline]
    fn mul(self, rhs: FBig<R, B>) -> Self::Output {
        match self {
            Sign::Positive => rhs,
            Sign::Negative => -rhs,
        }
    }
}

impl<R: Round, const B: Word> Mul<Sign> for FBig<R, B> {
    type Output = FBig<R, B>;
    #[inline]
    fn mul(self, rhs: Sign) -> Self::Output {
        match rhs {
            Sign::Positive => self,
            Sign::Negative => -self,
        }
    }
}

impl<R: Round, const B: Word> MulAssign<Sign> for FBig<R, B> {
    #[inline]
    fn mul_assign(&mut self, rhs: Sign) {
        if rhs == Sign::Negative {
            self.repr = self.repr.clone().neg();
        }
    }
}

impl<R: Round, const B: Word> Signed for FBig<R, B> {
    #[inline]
    fn sign(&self) -> Sign {
        self.repr.sign()
    }
}
