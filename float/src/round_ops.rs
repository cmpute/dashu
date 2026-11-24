use crate::{
    error::assert_finite,
    fbig::FBig,
    repr::{Context, Repr},
    round::{mode, Round, Rounded},
    utils::{shr_digits, split_digits_ref},
};
use dashu_base::{Approximation::*, Sign};
use dashu_int::{IBig, Word};

impl<R: Round, const B: Word> FBig<R, B> {
    /// Build an integer-valued `FBig` (exponent 0) from `value` under `context`, attaching
    /// `self`'s sign when `value` is zero. This is how truncation/rounding preserve IEEE 754
    /// signed zero: a negative value whose integer part is zero yields `-0`. Pass
    /// `Context::new(0)` for a precision-0 result matching `FBig::ZERO`.
    fn sign_kept_int(&self, value: IBig, context: Context<R>) -> Self {
        let repr = if value.is_zero() && self.repr.sign() == Sign::Negative {
            Repr::neg_zero()
        } else {
            Repr::new(value, 0)
        };
        FBig::new(repr, context)
    }

    /// Get the integral part of the float
    ///
    /// See [FBig::round] for how the output precision is determined.
    ///
    /// # Examples
    ///
    /// ```
    /// # use core::str::FromStr;
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// let a = DBig::from_str("1.234")?;
    /// assert_eq!(a.trunc(), DBig::from_str("1")?);
    /// // the actual precision of the integral part is 1 digit
    /// assert_eq!(a.trunc().precision(), 1);
    /// # Ok::<(), ParseError>(())
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the number is infinte
    #[inline]
    pub fn trunc(&self) -> Self {
        assert_finite(&self.repr);

        if self.repr.significand.is_zero() || self.repr.exponent >= 0 {
            return self.clone();
        } else if self.repr.smaller_than_one() {
            return self.sign_kept_int(IBig::ZERO, Context::new(0));
        }

        let shift = self.repr.exponent.unsigned_abs();
        let signif = shr_digits::<B>(&self.repr.significand, shift);
        if signif.is_zero() {
            // |self| < 1 after all (`smaller_than_one` is only an estimate): same result as
            // the shortcut above
            return self.sign_kept_int(IBig::ZERO, Context::new(0));
        }
        let context = Context::new(self.context.precision.saturating_sub(shift));
        self.sign_kept_int(signif, context)
    }

    // Split the float number at the radix point, assuming it exists (the number is not a integer).
    // The method returns (integral part, fractional part, fractional scale).
    //
    // Different from the public `split_at_point()` API, this method doesn't take the ownership of
    // this number.
    pub(crate) fn split_at_point_internal(&self) -> (IBig, IBig, usize) {
        debug_assert!(self.repr.exponent < 0);
        let shift = self.repr.exponent.unsigned_abs();
        if self.repr.smaller_than_one() {
            // For numbers smaller than 1, the integral part is zero and the stored
            // significand is the whole fractional payload.
            //
            // The third return value is the fractional scale, i.e. the number of
            // radix digits after the point. It must be -exponent, because callers
            // such as round_fract use it as the denominator exponent B^scale.
            //
            // This is intentionally not self.context.precision: context precision is
            // the significant-digit precision of the float, while this value describes
            // the positional scale of the fractional part.
            return (IBig::ZERO, self.repr.significand.clone(), shift);
        }

        let (hi, lo) = split_digits_ref::<B>(&self.repr.significand, shift);
        (hi, lo, shift)
    }

    /// Split the rational number into integral and fractional parts (split at the radix point)
    ///
    /// It's equivalent to `(self.trunc(), self.fract())`, including the precisions and the
    /// signs of zero parts (both parts carry the sign of `self`).
    ///
    /// # Examples
    ///
    /// ```
    /// # use core::str::FromStr;
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// let a = DBig::from_str("1.234")?;
    /// let (trunc, fract) = a.split_at_point();
    /// assert_eq!(trunc, DBig::from_str("1.0")?);
    /// assert_eq!(fract, DBig::from_str("0.234")?);
    /// // the actual precision of the fractional part is 3 digits
    /// assert_eq!(trunc.precision(), 1);
    /// assert_eq!(fract.precision(), 3);
    /// # Ok::<(), ParseError>(())
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the number is infinte
    pub fn split_at_point(self) -> (Self, Self) {
        assert_finite(&self.repr);

        // trivial cases: an integer (including ±0) has a zero fractional part, and a number
        // below one in magnitude a zero integral part; the zeros keep the sign of `self`
        if self.repr.significand.is_zero() || self.repr.exponent >= 0 {
            let fract = self.sign_kept_int(IBig::ZERO, Context::new(0));
            return (self, fract);
        } else if self.repr.smaller_than_one() {
            let trunc = self.sign_kept_int(IBig::ZERO, Context::new(0));
            return (trunc, self);
        }

        let shift = self.repr.exponent.unsigned_abs();
        let (hi, lo) = split_digits_ref::<B>(&self.repr.significand, shift);
        if hi.is_zero() {
            // |self| < 1 after all (`smaller_than_one` is only an estimate)
            let trunc = self.sign_kept_int(IBig::ZERO, Context::new(0));
            return (trunc, self);
        }
        let hi_ctxt = Context::new(self.context.precision.saturating_sub(shift));
        let lo_ctxt = Context::new(shift);
        (
            self.sign_kept_int(hi, hi_ctxt),
            FBig::new(Repr::new(lo, self.repr.exponent), lo_ctxt),
        )
    }

    /// Get the fractional part of the float
    ///
    /// **Note**: this function will adjust the precision accordingly! The fractional part
    /// of a number with a nonzero integral part gets the number of its fractional digits as
    /// precision; a number below one in magnitude is its own fractional part and is returned
    /// unchanged (precision included); the (zero) fractional part of an integer has unlimited
    /// precision and the sign of the number.
    ///
    /// # Examples
    ///
    /// ```
    /// # use core::str::FromStr;
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// let a = DBig::from_str("1.234")?;
    /// assert_eq!(a.fract(), DBig::from_str("0.234")?);
    /// // the actual precision of the fractional part is 3 digits
    /// assert_eq!(a.fract().precision(), 3);
    /// # Ok::<(), ParseError>(())
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the number is infinte
    #[inline]
    pub fn fract(&self) -> Self {
        assert_finite(&self.repr);
        if self.repr.significand.is_zero() || self.repr.exponent >= 0 {
            return self.sign_kept_int(IBig::ZERO, Context::new(0));
        } else if self.repr.smaller_than_one() {
            return self.clone();
        }

        let (hi, lo, precision) = self.split_at_point_internal();
        if hi.is_zero() {
            // |self| < 1 after all (`smaller_than_one` is only an estimate): the number is
            // its own fractional part, with its own precision
            return self.clone();
        }
        let context = Context::new(precision);
        FBig::new(Repr::new(lo, self.repr.exponent), context)
    }

    /// Returns the smallest integer greater than or equal to self.
    ///
    /// See [FBig::round] for how the output precision is determined.
    ///
    /// # Examples
    ///
    /// ```
    /// # use core::str::FromStr;
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// let a = DBig::from_str("1.234")?;
    /// assert_eq!(a.ceil(), DBig::from_str("2")?);
    ///
    /// // works for very large exponent
    /// let b = DBig::from_str("1.234e10000")?;
    /// assert_eq!(b.ceil(), b);
    /// # Ok::<(), ParseError>(())
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the number is infinte
    #[inline]
    pub fn ceil(&self) -> Self {
        assert_finite(&self.repr);
        if self.repr.significand.is_zero() || self.repr.exponent >= 0 {
            return self.clone();
        } else if self.repr.smaller_than_one() {
            return match self.repr.sign() {
                Sign::Positive => Self::ONE,
                // a negative number above -1 rounds up to -0: rounding to an integer keeps
                // the sign of the operand (IEEE 754 roundToIntegralTowardPositive)
                Sign::Negative => self.sign_kept_int(IBig::ZERO, Context::new(0)),
            };
        }

        let (hi, lo, precision) = self.split_at_point_internal();
        let rounding = mode::Up::round_fract::<B>(&hi, lo, precision);
        let context = Context::new(self.context.precision.saturating_sub(precision));
        self.sign_kept_int(hi + rounding, context)
    }

    /// Returns the largest integer less than or equal to self.
    ///
    /// See [FBig::round] for how the output precision is determined.
    ///
    /// # Examples
    ///
    /// ```
    /// # use core::str::FromStr;
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// let a = DBig::from_str("1.234")?;
    /// assert_eq!(a.floor(), DBig::from_str("1")?);
    ///
    /// // works for very large exponent
    /// let b = DBig::from_str("1.234e10000")?;
    /// assert_eq!(b.floor(), b);
    /// # Ok::<(), ParseError>(())
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the number is infinte
    #[inline]
    pub fn floor(&self) -> Self {
        assert_finite(&self.repr);
        if self.repr.significand.is_zero() || self.repr.exponent >= 0 {
            return self.clone();
        } else if self.repr.smaller_than_one() {
            return match self.repr.sign() {
                Sign::Positive => Self::ZERO,
                Sign::Negative => Self::NEG_ONE,
            };
        }

        let (hi, lo, precision) = self.split_at_point_internal();
        let rounding = mode::Down::round_fract::<B>(&hi, lo, precision);
        let context = Context::new(self.context.precision.saturating_sub(precision));
        FBig::new(Repr::new(hi + rounding, 0), context)
    }

    /// Returns the integer nearest to self.
    ///
    /// If there are two integers equally close, then the one farther from zero is chosen.
    ///
    /// **Note**: like [rounding on the primitive floats](f64::round), this method always
    /// applies that "nearest, ties away from zero" rule — the rounding mode attached to
    /// the type does not participate. To round to an integer with the type's own rounding
    /// mode, use [`to_int()`](Self::to_int) (yields an [IBig]) or [`quantize(0)`](Self::quantize)
    /// (yields an integer-valued float); more generally, [`with_precision()`](Self::with_precision)
    /// re-rounds to any precision with the type's rounding mode.
    ///
    /// # Examples
    ///
    /// ```
    /// # use core::str::FromStr;
    /// # use dashu_base::ParseError;
    /// # use dashu_float::{DBig, FBig, round::mode};
    /// let a = DBig::from_str("1.234")?;
    /// assert_eq!(a.round(), DBig::from_str("1")?);
    ///
    /// // works for very large exponent
    /// let b = DBig::from_str("1.234e10000")?;
    /// assert_eq!(b.round(), b);
    ///
    /// // the rounding mode attached to the type is ignored: 1.5 still rounds
    /// // away from zero on a type that otherwise rounds toward zero
    /// let c = FBig::<mode::Zero, 10>::from_str("1.5")?;
    /// assert_eq!(c.round(), FBig::<mode::Zero, 10>::from_str("2")?);
    /// assert_eq!(c.to_int().value(), 1.into());
    /// # Ok::<(), ParseError>(())
    /// ```
    ///
    /// # Precision
    ///
    /// If `self` is an integer, the result will have the same precision as `self`.
    /// If `self` has fractional part, then the precision will be subtracted by the digits
    /// in the fractional part. Examples:
    /// * `1.00e100` (precision = 3) rounds to `1.00e100` (precision = 3)
    /// * `1.234` (precision = 4) rounds to `1.` (precision = 1)
    /// * `1.234e-10` (precision = 4) rounds to `0.` (precision = 0, i.e arbitrary precision)
    ///
    /// # Panics
    ///
    /// Panics if the number is infinte
    pub fn round(&self) -> Self {
        assert_finite(&self.repr);
        if self.repr.significand.is_zero() || self.repr.exponent >= 0 {
            return self.clone();
        } else if self.repr.exponent + (self.repr.digits_ub() as isize) < -2 {
            // to determine if the number rounds to zero, we need to make sure |self| < 0.5
            // which is stricter than `self.repr.smaller_than_one()`
            return self.sign_kept_int(IBig::ZERO, Context::new(0));
        }

        let (hi, lo, precision) = self.split_at_point_internal();
        let rounding = mode::HalfAway::round_fract::<B>(&hi, lo, precision);
        let context = Context::new(self.context.precision.saturating_sub(precision));
        self.sign_kept_int(hi + rounding, context)
    }

    /// Round the number to the nearest multiple of `BASE^exp`.
    ///
    /// This is the dashu analog of Python's `Decimal.quantize()`. The result's
    /// value is an exact multiple of `BASE^exp`, and its precision is set so that
    /// [`ulp()`][FBig::ulp] equals `BASE^exp`. Because dashu floats are
    /// normalized, trailing zeros are not preserved in storage (the stored
    /// exponent may be coarser than `exp`), but the value and ULP are exact. The
    /// result keeps `self`'s rounding mode.
    ///
    /// The rounding itself is applied with the rounding mode attached to the type;
    /// in particular, `quantize(0)` is the mode-aware counterpart of
    /// [`round()`](Self::round), which always rounds half away from zero regardless
    /// of the type's rounding mode.
    ///
    /// # Examples
    ///
    /// ```
    /// # use core::str::FromStr;
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// use dashu_base::Approximation::*;
    /// use dashu_float::round::Rounding::*;
    ///
    /// let a = DBig::from_str("1.234")?; // precision 4
    ///
    /// // round to 2 fractional digits (exp = -2): 3 significant figures remain
    /// assert_eq!(a.quantize(-2), Inexact(DBig::from_str("1.23")?, NoOp));
    /// assert_eq!(a.quantize(-2).value().precision(), 3);
    ///
    /// // a finer quantum is exact (no rounding) and *increases* the precision
    /// assert_eq!(a.quantize(-10), Exact(DBig::from_str("1.234")?));
    /// assert_eq!(a.quantize(-10).value().precision(), 11);
    ///
    /// // round to integer (exp = 0), or to the nearest 1000 (exp = 3)
    /// assert_eq!(a.quantize(0), Inexact(DBig::from_str("1")?, NoOp));
    /// assert_eq!(DBig::from_str("999")?.quantize(3), Inexact(DBig::from_str("1000")?, AddOne));
    /// # Ok::<(), ParseError>(())
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the number is infinte
    pub fn quantize(&self, exp: isize) -> Rounded<Self> {
        assert_finite(&self.repr);
        if self.repr.is_pos_zero() || self.repr.is_neg_zero() {
            // Zero is an exact multiple of any quantum; preserve the sign of `-0`
            // (IEEE 754 `quantize` is sign-preserving).
            return Exact(self.clone());
        }

        // The exponent differences below are taken as unsigned distances: `exp` and the
        // stored exponent can sit at opposite ends of the `isize` range.
        let self_exp = self.repr.exponent;
        if exp <= self_exp {
            // finer-or-equal quantum: self is already an exact multiple of BASE^exp,
            // so only the precision changes (set so that ulp == BASE^exp).
            let precision = self_exp.abs_diff(exp).saturating_add(self.repr.digits());
            return Exact(FBig::new(self.repr.clone(), Context::new(precision)));
        }

        // coarser quantum: round off (exp - self_exp) low-order digits.
        // Because a normalized significand is never divisible by BASE, this branch
        // is always inexact (its low part is never all-zero).
        let shift = exp.abs_diff(self_exp);
        let (hi, lo) = if shift > self.repr.digits() {
            // the whole significand lies below the quantum (and `shift` may be far too
            // large to build BASE^shift for an actual split)
            (IBig::ZERO, self.repr.significand.clone())
        } else {
            split_digits_ref::<B>(&self.repr.significand, shift)
        };
        let adjust = R::round_fract::<B>(&hi, lo, shift);
        let signif = hi + adjust;
        if signif.is_zero() {
            // A result that rounds to zero keeps the sign of `self` (IEEE 754 `quantize`
            // is sign-preserving, like `round()`); it has no meaningful ulp, so it gets
            // unlimited precision (again like `round()`).
            let repr = Repr::zero_with_sign(self.repr.sign());
            return Inexact(FBig::new(repr, Context::new(0)), adjust);
        }
        let repr = Repr::new(signif, exp);
        // precision is set so that ulp == BASE^exp
        let precision = repr.exponent.abs_diff(exp).saturating_add(repr.digits());
        Inexact(FBig::new(repr, Context::new(precision)), adjust)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shr_assign_shifts_once() {
        // Regression: shr_assign previously subtracted rhs twice.
        let mut x = FBig::<mode::HalfEven>::try_from(8.0f64).unwrap(); // 2^3
        x >>= 1; // 2^2 = 4
        let y = FBig::<mode::HalfEven>::try_from(4.0f64).unwrap();
        assert_eq!(x, y);
    }
}
