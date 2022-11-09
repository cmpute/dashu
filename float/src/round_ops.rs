use crate::{
    error::check_inf,
    fbig::FBig,
    repr::{Context, Repr},
    round::{
        mode,
        Round,
    },
    utils::{shr_digits, split_digits_ref, split_digits},
};
use dashu_int::{IBig, Word};

impl<R: Round, const B: Word> FBig<R, B> {
    /// Get the integral part of the float
    ///
    /// **Note**: this function will adjust the precision accordingly.
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
        check_inf(&self.repr);

        let exponent = self.repr.exponent;
        if exponent >= 0 {
            return self.clone();
        } else if exponent + (self.repr.digits_ub() as isize) < 0 {
            return Self::ZERO;
        }

        let shift = (-exponent) as usize;
        let signif = shr_digits::<B>(&self.repr.significand, shift);
        let context = Context::new(self.precision() - shift);
        FBig::new(Repr::new(signif, 0), context)
    }

    // Split the float number at the radix point, assuming it exists (the number is not a integer).
    // The method returns (integral part, fractional part, fraction precision).
    //
    // Different from the public `split_at_point()` API, this method doesn't take the ownership of
    // this number.
    pub(crate) fn split_at_point_internal(&self) -> (IBig, IBig, usize) {
        debug_assert!(self.repr.exponent < 0);

        let exponent = self.repr.exponent;
        if exponent + (self.repr.digits_ub() as isize) < 0 {
            return (IBig::ZERO, self.repr.significand.clone(), self.context.precision);
        }

        let shift = (-exponent) as usize;
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
        }

        let exponent = self.repr.exponent;
        if exponent + (self.repr.digits_ub() as isize) < 0 {
            return (Self::ZERO, self);
        }

        let shift = (-exponent) as usize;
        let (hi, lo) = split_digits::<B>(self.repr.significand, shift);
        let hi_ctxt = Context::new(self.context.precision - shift);
        let lo_ctxt = Context::new(shift);
        (
            FBig::new(Repr::new(hi, 0), hi_ctxt),
            FBig::new(Repr::new(lo, self.repr.exponent), lo_ctxt)
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
        check_inf(&self.repr);
        if self.repr.exponent >= 0 {
            return Self::ZERO;
        }

        let (_, lo, precision) = self.split_at_point_internal();
        let context = Context::new(precision);
        FBig::new(Repr::new(lo, self.repr.exponent), context)
    }

    /// Returns the smallest integer greater than or equal to self.
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
        check_inf(&self.repr);
        if self.repr.exponent >= 0 {
            return self.clone();
        }

        let (hi, lo, precision) = self.split_at_point_internal();
        let rounding = mode::Up::round_fract::<B>(&hi, lo, precision);
        let context = Context::new(self.precision() - precision);
        FBig::new(Repr::new(hi + rounding, 0), context)
    }

    /// Returns the largest integer less than or equal to self.
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
        check_inf(&self.repr);
        if self.repr.exponent >= 0 {
            return self.clone();
        }

        let (hi, lo, precision) = self.split_at_point_internal();
        let rounding = mode::Down::round_fract::<B>(&hi, lo, precision);
        let context = Context::new(self.precision() - precision);
        FBig::new(Repr::new(hi + rounding, 0), context)
    }
}
