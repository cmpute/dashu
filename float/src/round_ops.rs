use crate::{
    error::assert_finite,
    fbig::FBig,
    repr::{Context, Repr},
    round::{mode, Round},
    utils::{shr_digits, split_digits, split_digits_ref},
};
use dashu_base::Sign;
use dashu_int::{IBig, Word};

impl<R: Round, const B: Word> FBig<R, B> {
    /// Get the integral part of the float
    ///
    /// See [FBig::round] for how the output precision is determined.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// let a = DBig::from_str_native("1.234")?;
    /// assert_eq!(a.trunc(), DBig::from_str_native("1")?);
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

        if self.repr.exponent >= 0 {
            return self.clone();
        } else if self.repr.smaller_than_one() {
            return Self::ZERO;
        }

        let shift = (-self.repr.exponent) as usize;
        let signif = shr_digits::<B>(&self.repr.significand, shift);
        let context = Context::new(self.context.precision.saturating_sub(shift));
        FBig::new(Repr::new(signif, 0), context)
    }

    // Split the float number at the radix point, assuming it exists (the number is not a integer).
    // The method returns (integral part, fractional part, fraction precision).
    //
    // Different from the public `split_at_point()` API, this method doesn't take the ownership of
    // this number.
    pub(crate) fn split_at_point_internal(&self) -> (IBig, IBig, usize) {
        debug_assert!(self.repr.exponent < 0);
        if self.repr.smaller_than_one() {
            return (IBig::ZERO, self.repr.significand.clone(), self.context.precision);
        }

        let shift = (-self.repr.exponent) as usize;
        let (hi, lo) = split_digits_ref::<B>(&self.repr.significand, shift);
        (hi, lo, shift)
    }

    /// Split the rational number into integral and fractional parts (split at the radix point)
    ///
    /// It's equivalent to `(self.trunc(), self.fract())`
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// let a = DBig::from_str_native("1.234")?;
    /// let (trunc, fract) = a.split_at_point();
    /// assert_eq!(trunc, DBig::from_str_native("1.0")?);
    /// assert_eq!(fract, DBig::from_str_native("0.234")?);
    /// // the actual precision of the fractional part is 3 digits
    /// assert_eq!(trunc.precision(), 1);
    /// assert_eq!(fract.precision(), 3);
    /// # Ok::<(), ParseError>(())
    /// ```
    pub fn split_at_point(self) -> (Self, Self) {
        // trivial case when the exponent is positive
        if self.repr.exponent >= 0 {
            return (self, Self::ZERO);
        } else if self.repr.smaller_than_one() {
            return (Self::ZERO, self);
        }

        let shift = (-self.repr.exponent) as usize;
        let (hi, lo) = split_digits::<B>(self.repr.significand, shift);
        let hi_ctxt = Context::new(self.context.precision.saturating_sub(shift));
        let lo_ctxt = Context::new(shift);
        (
            FBig::new(Repr::new(hi, 0), hi_ctxt),
            FBig::new(Repr::new(lo, self.repr.exponent), lo_ctxt),
        )
    }

    /// Get the fractional part of the float
    ///
    /// **Note**: this function will adjust the precision accordingly!
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// let a = DBig::from_str_native("1.234")?;
    /// assert_eq!(a.fract(), DBig::from_str_native("0.234")?);
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
        if self.repr.exponent >= 0 {
            return Self::ZERO;
        }

        let (_, lo, precision) = self.split_at_point_internal();
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
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// let a = DBig::from_str_native("1.234")?;
    /// assert_eq!(a.ceil(), DBig::from_str_native("2")?);
    ///
    /// // works for very large exponent
    /// let b = DBig::from_str_native("1.234e10000")?;
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
        if self.repr.is_zero() || self.repr.exponent >= 0 {
            return self.clone();
        } else if self.repr.smaller_than_one() {
            return match self.repr.sign() {
                Sign::Positive => Self::ONE,
                Sign::Negative => Self::ZERO,
            };
        }

        let (hi, lo, precision) = self.split_at_point_internal();
        let rounding = mode::Up::round_fract::<B>(&hi, lo, precision);
        let context = Context::new(self.context.precision.saturating_sub(precision));
        FBig::new(Repr::new(hi + rounding, 0), context)
    }

    /// Returns the largest integer less than or equal to self.
    ///
    /// See [FBig::round] for how the output precision is determined.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// let a = DBig::from_str_native("1.234")?;
    /// assert_eq!(a.floor(), DBig::from_str_native("1")?);
    ///
    /// // works for very large exponent
    /// let b = DBig::from_str_native("1.234e10000")?;
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
        if self.repr.exponent >= 0 {
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
    /// # Examples
    ///
    /// ```
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// let a = DBig::from_str_native("1.234")?;
    /// assert_eq!(a.round(), DBig::from_str_native("1")?);
    ///
    /// // works for very large exponent
    /// let b = DBig::from_str_native("1.234e10000")?;
    /// assert_eq!(b.round(), b);
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
        if self.repr.exponent >= 0 {
            return self.clone();
        } else if self.repr.exponent + (self.repr.digits_ub() as isize) < -2 {
            // to determine if the number rounds to zero, we need to make sure |self| < 0.5
            // which is stricter than `self.repr.smaller_than_one()`
            return Self::ZERO;
        }

        let (hi, lo, precision) = self.split_at_point_internal();
        let rounding = mode::HalfAway::round_fract::<B>(&hi, lo, precision);
        let context = Context::new(self.context.precision.saturating_sub(precision));
        FBig::new(Repr::new(hi + rounding, 0), context)
    }
}
