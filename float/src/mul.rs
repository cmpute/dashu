use dashu_base::Sign;
use dashu_int::{IBig, UBig};

use crate::{
    error::{assert_finite_operands, FpError, FpResult},
    fbig::FBig,
    helper_macros,
    repr::{Context, Repr, Word},
    round::Round,
};
use core::ops::{Mul, MulAssign};

/// Raw product of two finite reprs, attaching the XOR sign of the operands to a zero product
/// (the significand product alone is `+0`, losing the sign).
///
/// Returns an error when the result exponent overflows or underflows `isize`.
fn mul_finite_reprs<const B: Word>(lhs: &Repr<B>, rhs: &Repr<B>) -> Result<Repr<B>, FpError> {
    let significand = &lhs.significand * &rhs.significand;
    if significand.is_zero() {
        return Ok(if lhs.sign() != rhs.sign() {
            Repr::neg_zero()
        } else {
            Repr::zero()
        });
    }
    let sign = if lhs.sign() != rhs.sign() {
        Sign::Negative
    } else {
        Sign::Positive
    };
    let exponent = lhs.exponent.checked_add(rhs.exponent).ok_or_else(|| {
        debug_assert!(
            lhs.exponent.is_positive() == rhs.exponent.is_positive(),
            "checked_add overflow with mixed-sign exponents is impossible"
        );
        if lhs.exponent > 0 {
            FpError::Overflow(sign)
        } else {
            FpError::Underflow(sign)
        }
    })?;
    Repr::new(significand, exponent).check_finite_exponent()
}

macro_rules! unwrap_mul_repr {
    ($result:expr, $context:expr) => {
        match $result {
            Ok(r) => r,
            Err(FpError::Overflow(sign)) => {
                return FBig::new(Repr::infinity_with_sign(sign), $context);
            }
            Err(FpError::Underflow(sign)) => {
                return FBig::new(Repr::zero_with_sign(sign), $context);
            }
            Err(_) => unreachable!(),
        }
    };
}

impl<R: Round, const B: Word> Mul<&FBig<R, B>> for &FBig<R, B> {
    type Output = FBig<R, B>;

    #[inline]
    fn mul(self, rhs: &FBig<R, B>) -> Self::Output {
        assert_finite_operands(&self.repr, &rhs.repr);

        let context = Context::max(self.context, rhs.context);
        let repr = unwrap_mul_repr!(mul_finite_reprs(&self.repr, &rhs.repr), context);
        FBig::new(context.repr_round(repr).value(), context)
    }
}

impl<R: Round, const B: Word> Mul<&FBig<R, B>> for FBig<R, B> {
    type Output = FBig<R, B>;

    #[inline]
    fn mul(self, rhs: &FBig<R, B>) -> Self::Output {
        assert_finite_operands(&self.repr, &rhs.repr);

        let context = Context::max(self.context, rhs.context);
        let repr = unwrap_mul_repr!(mul_finite_reprs(&self.repr, &rhs.repr), context);
        FBig::new(context.repr_round(repr).value(), context)
    }
}

impl<R: Round, const B: Word> Mul<FBig<R, B>> for &FBig<R, B> {
    type Output = FBig<R, B>;

    #[inline]
    fn mul(self, rhs: FBig<R, B>) -> Self::Output {
        assert_finite_operands(&self.repr, &rhs.repr);

        let context = Context::max(self.context, rhs.context);
        let repr = unwrap_mul_repr!(mul_finite_reprs(&self.repr, &rhs.repr), context);
        FBig::new(context.repr_round(repr).value(), context)
    }
}

impl<R: Round, const B: Word> Mul<FBig<R, B>> for FBig<R, B> {
    type Output = FBig<R, B>;

    #[inline]
    fn mul(self, rhs: FBig<R, B>) -> Self::Output {
        assert_finite_operands(&self.repr, &rhs.repr);

        let context = Context::max(self.context, rhs.context);
        let repr = unwrap_mul_repr!(mul_finite_reprs(&self.repr, &rhs.repr), context);
        FBig::new(context.repr_round(repr).value(), context)
    }
}

helper_macros::impl_binop_assign_by_taking!(impl MulAssign<Self>, mul_assign, mul);

macro_rules! impl_mul_primitive_with_fbig {
    ($($t:ty)*) => {$(
        helper_macros::impl_binop_with_primitive!(impl Mul<$t>, mul);
        helper_macros::impl_binop_assign_with_primitive!(impl MulAssign<$t>, mul_assign);
    )*};
}
impl_mul_primitive_with_fbig!(u8 u16 u32 u64 u128 usize UBig i8 i16 i32 i64 i128 isize IBig);

impl<R: Round, const B: Word> FBig<R, B> {
    /// Compute the square of this number (`self * self`)
    ///
    /// # Examples
    ///
    /// ```
    /// # use core::str::FromStr;
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// let a = DBig::from_str("-1.234")?;
    /// assert_eq!(a.sqr(), DBig::from_str("1.523")?);
    /// # Ok::<(), ParseError>(())
    /// ```
    #[inline]
    pub fn sqr(&self) -> Self {
        self.context.unwrap_fp(self.context.sqr(&self.repr))
    }

    /// Compute the cubic of this number (`self * self * self`)
    ///
    /// # Examples
    ///
    /// ```
    /// # use core::str::FromStr;
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// let a = DBig::from_str("-1.234")?;
    /// assert_eq!(a.cubic(), DBig::from_str("-1.879")?);
    /// # Ok::<(), ParseError>(())
    /// ```
    #[inline]
    pub fn cubic(&self) -> Self {
        self.context.unwrap_fp(self.context.cubic(&self.repr))
    }
}

impl<R: Round> Context<R> {
    /// Multiply two floating point numbers under this context.
    ///
    /// # Examples
    ///
    /// ```
    /// # use core::str::FromStr;
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// use dashu_base::Approximation::*;
    /// use dashu_float::{Context, round::{mode::HalfAway, Rounding::*}};
    ///
    /// let context = Context::<HalfAway>::new(2);
    /// let a = DBig::from_str("-1.234")?;
    /// let b = DBig::from_str("6.789")?;
    /// assert_eq!(
    ///     context.mul(&a.repr(), &b.repr()),
    ///     Ok(Inexact(DBig::from_str("-8.4")?, SubOne))
    /// );
    /// # Ok::<(), ParseError>(())
    /// ```
    pub fn mul<const B: Word>(&self, lhs: &Repr<B>, rhs: &Repr<B>) -> FpResult<FBig<R, B>> {
        if lhs.is_infinite() || rhs.is_infinite() {
            return Err(FpError::InfiniteInput);
        }

        // at most double the precision is required to get a correct result
        // shrink the input operands if necessary
        let max_precision = if self.is_limited() {
            self.precision * 2
        } else {
            usize::MAX
        };

        let lhs_shrink;
        let lhs_repr = if lhs.digits() > max_precision {
            lhs_shrink = Context::<R>::new(max_precision).repr_round_ref(lhs).value();
            &lhs_shrink
        } else {
            lhs
        };

        let rhs_shrink;
        let rhs_repr = if rhs.digits() > max_precision {
            rhs_shrink = Context::<R>::new(max_precision).repr_round_ref(rhs).value();
            &rhs_shrink
        } else {
            rhs
        };

        let repr = mul_finite_reprs(lhs_repr, rhs_repr)?;
        Ok(self.repr_round(repr).map(|v| FBig::new(v, *self)))
    }

    /// Calculate the square of the floating point number under this context.
    ///
    /// # Examples
    ///
    /// ```
    /// # use core::str::FromStr;
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// use dashu_base::Approximation::*;
    /// use dashu_float::{Context, round::{mode::HalfAway, Rounding::*}};
    ///
    /// let context = Context::<HalfAway>::new(2);
    /// let a = DBig::from_str("-1.234")?;
    /// assert_eq!(context.sqr(&a.repr()), Ok(Inexact(DBig::from_str("1.5")?, NoOp)));
    /// # Ok::<(), ParseError>(())
    /// ```
    pub fn sqr<const B: Word>(&self, f: &Repr<B>) -> FpResult<FBig<R, B>> {
        if f.is_infinite() {
            return Err(FpError::InfiniteInput);
        }

        // shrink the input operands if necessary
        let max_precision = if self.is_limited() {
            self.precision * 2
        } else {
            usize::MAX
        };

        let f_shrink;
        let f_repr = if f.digits() > max_precision {
            f_shrink = Context::<R>::new(max_precision).repr_round_ref(f).value();
            &f_shrink
        } else {
            f
        };

        let exponent = f_repr.exponent.checked_mul(2).ok_or({
            // sqr always produces a non-negative result
            if f_repr.exponent > 0 {
                FpError::Overflow(Sign::Positive)
            } else {
                FpError::Underflow(Sign::Positive)
            }
        })?;
        let repr = Repr::new(f_repr.significand.sqr().into(), exponent);
        let repr = repr.check_finite_exponent()?;
        Ok(self.repr_round(repr).map(|v| FBig::new(v, *self)))
    }

    /// Calculate the cubic of the floating point number under this context.
    ///
    /// # Examples
    ///
    /// ```
    /// # use core::str::FromStr;
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// use dashu_base::Approximation::*;
    /// use dashu_float::{Context, round::{mode::HalfAway, Rounding::*}};
    ///
    /// let context = Context::<HalfAway>::new(2);
    /// let a = DBig::from_str("-1.234")?;
    /// assert_eq!(context.cubic(&a.repr()), Ok(Inexact(DBig::from_str("-1.9")?, SubOne)));
    /// # Ok::<(), ParseError>(())
    /// ```
    pub fn cubic<const B: Word>(&self, f: &Repr<B>) -> FpResult<FBig<R, B>> {
        if f.is_infinite() {
            return Err(FpError::InfiniteInput);
        }

        // shrink the input operands if necessary
        let max_precision = if self.is_limited() {
            self.precision * 3
        } else {
            usize::MAX
        };

        let f_shrink;
        let f_repr = if f.digits() > max_precision {
            f_shrink = Context::<R>::new(max_precision).repr_round_ref(f).value();
            &f_shrink
        } else {
            f
        };

        let repr = if f_repr.significand.is_zero() {
            // cubic(±0) = ±0 (odd power preserves sign)
            if f_repr.is_neg_zero() {
                Repr::neg_zero()
            } else {
                Repr::zero()
            }
        } else {
            let sign = f_repr.sign();
            let exponent = f_repr.exponent.checked_mul(3).ok_or({
                if f_repr.exponent > 0 {
                    FpError::Overflow(sign)
                } else {
                    FpError::Underflow(sign)
                }
            })?;
            let repr = Repr::new(f_repr.significand.cubic(), exponent);
            repr.check_finite_exponent()?
        };
        Ok(self.repr_round(repr).map(|v| FBig::new(v, *self)))
    }
}
