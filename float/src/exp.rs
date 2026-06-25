use core::convert::TryInto;

use crate::{
    error::{assert_finite, assert_limited_precision, FpError, FpResult},
    fbig::FBig,
    math::cache::{reborrow_cache, ConstCache},
    repr::{Context, Repr, Word},
    round::Round,
    utils::ceil_usize,
};
use dashu_base::{AbsOrd, Approximation::*, BitTest, DivRemEuclid, EstimatedLog2, Sign};
use dashu_int::IBig;

impl<R: Round, const B: Word> FBig<R, B> {
    /// Raise the floating point number to an integer power.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// let a = DBig::from_str_native("-1.234")?;
    /// assert_eq!(a.powi(10.into()), DBig::from_str_native("8.188")?);
    /// # Ok::<(), ParseError>(())
    /// ```
    #[inline]
    pub fn powi(&self, exp: IBig) -> FBig<R, B> {
        self.context.unwrap_fp(self.context.powi(&self.repr, exp))
    }

    /// Raise the floating point number to an floating point power.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// let x = DBig::from_str_native("1.23")?;
    /// let y = DBig::from_str_native("-4.56")?;
    /// assert_eq!(x.powf(&y), DBig::from_str_native("0.389")?);
    /// # Ok::<(), ParseError>(())
    /// ```
    #[inline]
    pub fn powf(&self, exp: &Self) -> Self {
        let context = Context::max(self.context, exp.context);
        context.unwrap_fp(context.powf(&self.repr, &exp.repr, None))
    }

    /// Calculate the exponential function (`eˣ`) on the floating point number.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// let a = DBig::from_str_native("-1.234")?;
    /// assert_eq!(a.exp(), DBig::from_str_native("0.2911")?);
    /// # Ok::<(), ParseError>(())
    /// ```
    #[inline]
    pub fn exp(&self) -> FBig<R, B> {
        self.context.unwrap_fp(self.context.exp(&self.repr, None))
    }

    /// Calculate the exponential minus one function (`eˣ-1`) on the floating point number.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// let a = DBig::from_str_native("-0.1234")?;
    /// assert_eq!(a.exp_m1(), DBig::from_str_native("-0.11609")?);
    /// # Ok::<(), ParseError>(())
    /// ```
    #[inline]
    pub fn exp_m1(&self) -> FBig<R, B> {
        self.context.unwrap_fp(self.context.exp_m1(&self.repr, None))
    }
}

// TODO: give the exact formulation of required guard bits

impl<R: Round> Context<R> {
    /// Raise the floating point number to an integer power under this context.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// use dashu_base::Approximation::*;
    /// use dashu_float::{Context, round::{mode::HalfAway, Rounding::*}};
    ///
    /// let context = Context::<HalfAway>::new(2);
    /// let a = DBig::from_str_native("-1.234")?;
    /// assert_eq!(context.powi(&a.repr(), 10.into()), Ok(Inexact(DBig::from_str_native("8.2")?, AddOne)));
    /// # Ok::<(), ParseError>(())
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the precision is unlimited and the exponent is negative. In this case, the exact
    /// result is likely to have infinite digits.
    pub fn powi<const B: Word>(&self, base: &Repr<B>, exp: IBig) -> FpResult<FBig<R, B>> {
        if base.is_infinite() {
            return Err(FpError::InfiniteInput);
        }

        let (exp_sign, exp) = exp.into_parts();
        if exp_sign == Sign::Negative {
            // if the exponent is negative, then negate the exponent
            // note that do the inverse at last requires less guard bits
            assert_limited_precision(self.precision); // TODO: we can allow this if the inverse is exact (only when significand is one?)

            let guard_bits = self.precision.bit_len() * 2; // heuristic
            let rev_context = Context::<R::Reverse>::new(self.precision + guard_bits);
            let pow = rev_context.unwrap_fp(rev_context.powi(base, exp.into()));
            let inv = rev_context.unwrap_fp_repr(rev_context.repr_div(Repr::one(), pow.repr));
            let repr = self.repr_round(inv);
            return Ok(repr.map(|v| FBig::new(v, *self)));
        }
        if exp.is_zero() {
            return Ok(Exact(FBig::ONE));
        } else if exp.is_one() {
            let repr = self.repr_round_ref(base);
            return Ok(repr.map(|v| FBig::new(v, *self)));
        }

        // Guard against exponent overflow for astronomically large results: the result
        // magnitude has log2 ≈ exp·log2(base); if that exceeds the isize exponent range,
        // return ±inf (|base| > 1) or 0 (|base| < 1) instead of overflowing mid-computation.
        let base_log2 = base.log2_est() as f64;
        let threshold = (isize::MAX as f64) * (B.log2_est() as f64);
        let exp_f64 = i64::try_from(&exp).ok().map(|e| e as f64);
        let overflows = match exp_f64 {
            Some(e) => e * base_log2 > threshold,
            None => base_log2 != 0.0, // exp doesn't fit i64: overflows unless |base| == 1
        };
        if overflows {
            return if base_log2 > 0.0 {
                Err(FpError::Overflow(
                    if base.sign() == Sign::Negative {
                        Sign::Negative
                    } else {
                        Sign::Positive
                    },
                ))
            } else {
                // |base| < 1 and exponent huge → underflow to signed zero
                let underflow_sign = if base.sign() == Sign::Negative
                    && exp.bit(0)
                {
                    Sign::Negative
                } else {
                    Sign::Positive
                };
                Err(FpError::Underflow(underflow_sign))
            };
        }

        let work_context = if self.is_limited() {
            // increase working precision when the exponent is large
            let guard_digits = exp.bit_len() + self.precision.bit_len(); // heuristic
            Context::<R>::new(self.precision + guard_digits)
        } else {
            Context::<R>::new(0)
        };

        // binary exponentiation from left to right
        let mut p = exp.bit_len() - 2;
        let mut res = work_context.unwrap_fp(work_context.sqr(base));
        loop {
            if exp.bit(p) {
                res = work_context.unwrap_fp(work_context.mul(res.repr(), base));
            }
            if p == 0 {
                break;
            }
            p -= 1;
            res = work_context.unwrap_fp(work_context.sqr(res.repr()));
        }

        Ok(res.with_precision(self.precision))
    }

    /// Raise the floating point number to an floating point power under this context.
    ///
    /// Note that this method will not rely on [FBig::powi] even if the `exp` is actually an integer.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// use dashu_base::Approximation::*;
    /// use dashu_float::{Context, round::{mode::HalfAway, Rounding::*}};
    ///
    /// let context = Context::<HalfAway>::new(2);
    /// let x = DBig::from_str_native("1.23")?;
    /// let y = DBig::from_str_native("-4.56")?;
    /// assert_eq!(context.powf(&x.repr(), &y.repr(), None), Ok(Inexact(DBig::from_str_native("0.39")?, AddOne)));
    /// # Ok::<(), ParseError>(())
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the precision is unlimited.
    pub fn powf<const B: Word>(
        &self,
        base: &Repr<B>,
        exp: &Repr<B>,
        mut cache: Option<&mut ConstCache>,
    ) -> FpResult<FBig<R, B>> {
        if base.is_infinite() || exp.is_infinite() {
            return Err(FpError::InfiniteInput);
        }
        assert_limited_precision(self.precision); // TODO: we can allow it if exp is integer

        // shortcuts
        if exp.is_zero() {
            return Ok(Exact(FBig::ONE));
        } else if exp.is_one() {
            let repr = self.repr_round_ref(base);
            return Ok(repr.map(|v| FBig::new(v, *self)));
        } else if base.significand.is_zero() {
            // With a *float* exponent the result on a zero base is the positive one — this
            // matches the common float-pow convention (e.g. CPython: `(-0.0) ** y == 0.0`),
            // which doesn't track the parity of the exponent:
            //   pow(±0, y > 0) = +0,    pow(±0, y < 0) = +inf.
            // For the sign-correct result (e.g. `pow(-0, odd) = -0`), use the integer-exponent
            // [`powi`](Context::powi). Short-circuiting here also avoids the negative-base path.
            return Ok(Exact(if exp.sign() == Sign::Negative {
                FBig::new(Repr::infinity(), *self)
            } else {
                FBig::ZERO
            }));
        }
        if base.sign() == Sign::Negative {
            // TODO: we should allow negative base when exp is an integer
            return Err(FpError::OutOfDomain);
        }

        // x^y = exp(y*ln(x)), use a simple rule for guard bits
        let guard_digits = 10 + ceil_usize(self.precision.log2_est());
        let work_context = Context::<R>::new(self.precision + guard_digits);

        // ln and exp each consult/extend the shared cache; reborrows are sequential.
        let ln_val = work_context.unwrap_fp(work_context.ln(base, reborrow_cache(&mut cache)));
        let mul_val = work_context.unwrap_fp(work_context.mul(ln_val.repr(), exp));
        let exp_val =
            work_context.unwrap_fp(work_context.exp(mul_val.repr(), reborrow_cache(&mut cache)));
        Ok(exp_val.with_precision(self.precision))
    }

    /// Calculate the exponential function (`eˣ`) on the floating point number under this context.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// use dashu_base::Approximation::*;
    /// use dashu_float::{Context, round::{mode::HalfAway, Rounding::*}};
    ///
    /// let context = Context::<HalfAway>::new(2);
    /// let a = DBig::from_str_native("-1.234")?;
    /// assert_eq!(context.exp(&a.repr(), None), Ok(Inexact(DBig::from_str_native("0.29")?, NoOp)));
    /// # Ok::<(), ParseError>(())
    /// ```
    #[inline]
    pub fn exp<const B: Word>(
        &self,
        x: &Repr<B>,
        cache: Option<&mut ConstCache>,
    ) -> FpResult<FBig<R, B>> {
        if x.is_infinite() {
            return Ok(Exact(FBig::new(
                match x.sign() {
                    Sign::Positive => Repr::infinity(),
                    Sign::Negative => Repr::zero(),
                },
                *self,
            )));
        }
        self.exp_internal(x, false, cache)
    }

    /// Calculate the exponential minus one function (`eˣ-1`) on the floating point number under this context.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// use dashu_base::Approximation::*;
    /// use dashu_float::{Context, round::{mode::HalfAway, Rounding::*}};
    ///
    /// let context = Context::<HalfAway>::new(2);
    /// let a = DBig::from_str_native("-0.1234")?;
    /// assert_eq!(context.exp_m1(&a.repr(), None), Ok(Inexact(DBig::from_str_native("-0.12")?, SubOne)));
    /// # Ok::<(), ParseError>(())
    /// ```
    #[inline]
    pub fn exp_m1<const B: Word>(
        &self,
        x: &Repr<B>,
        cache: Option<&mut ConstCache>,
    ) -> FpResult<FBig<R, B>> {
        if x.is_infinite() {
            return match x.sign() {
                Sign::Positive => Ok(Exact(FBig::new(Repr::infinity(), *self))),
                Sign::Negative => Ok(Exact(-FBig::ONE)), // exp_m1(−∞) = −1
            };
        }
        self.exp_internal(x, true, cache)
    }

    // TODO: change reduction to (x - s log2) / 2ⁿ, so that the final powering is always base 2, and doesn't depends on powi.
    //       the powering exp(r)^(2ⁿ) could be optimized by noticing (1+x)^2 - 1 = x^2 + 2x
    //       consider this change after having a benchmark

    fn exp_internal<const B: Word>(
        &self,
        x: &Repr<B>,
        minus_one: bool,
        mut cache: Option<&mut ConstCache>,
    ) -> FpResult<FBig<R, B>> {
        assert_finite(x);
        assert_limited_precision(self.precision);
        let input_sign = x.sign();

        if x.significand.is_zero() {
            // exp(±0) = 1, exp_m1(±0) = +0
            return match minus_one {
                false => Ok(Exact(FBig::ONE)),
                true => Ok(Exact(FBig::ZERO)),
            };
        }

        // A simple algorithm:
        // - let r = (x - s logB) / Bⁿ, where s = floor(x / logB), such that r < B⁻ⁿ.
        // - if the target precision is p digits, then there're only about p/m terms in Tyler series
        // - finally, exp(x) = Bˢ * exp(r)^(Bⁿ)
        // - the optimal n is √p as given by MPFR

        // Maclaurin series: exp(r) = 1 + Σ(rⁱ/i!)
        // There will be about p/log_B(r) summations when calculating the series, to prevent
        // loss of significance, we need about log_B(p) guard digits.
        let series_guard_digits = ceil_usize(self.precision.log2_est() / B.log2_est()) + 2;

        // Reduction power: the series value is later raised to Bⁿ, which amplifies its
        // relative error by a factor of Bⁿ. So the series (and the squarings) must carry
        // about n extra base-B digits for the result to come out correct to p digits. We
        // use 2n for safety — this mirrors MPFR's working precision q = precy + 2·K + …
        // (K ≈ √precy is MPFR's squaring count, the analogue of our n). The log_B(p)
        // summation/squaring rounding terms are already covered by series_guard_digits.
        let n = 1usize << (self.precision.bit_len() / 2);
        let pow_guard_digits = 2 * n;
        let work_precision;

        // When minus_one is true and |x| < 1/B, the input is fed into the Maclaurin series without scaling
        let no_scaling = minus_one && x.log2_est() < -B.log2_est();
        let (s, n, r) = if no_scaling {
            // if minus_one is true and x is already small (x < 1/B),
            // then directly evaluate the Maclaurin series without scaling
            if x.sign() == Sign::Negative {
                // extra digits are required to prevent cancellation during the summation
                work_precision = self.precision + 2 * series_guard_digits;
            } else {
                work_precision = self.precision + series_guard_digits;
            }
            let context = Context::<R>::new(work_precision);
            (0, 0, FBig::new(context.repr_round_ref(x).value(), context))
        } else {
            work_precision = self.precision + series_guard_digits + pow_guard_digits;
            let context = Context::<R>::new(work_precision);
            let x = FBig::new(context.repr_round_ref(x).value(), context);
            let logb = context.ln_base::<B>(reborrow_cache(&mut cache));
            let (s, r) = x.div_rem_euclid(logb);

            let s: isize = match s.try_into() {
                Ok(v) => v,
                Err(_) => {
                    // |floor(x / ln B)| overflows isize — x is astronomically large, so the
                    // result is an infinity (x → +∞) or underflows to the limit (x → −∞).
                    return if input_sign == Sign::Positive {
                        Err(FpError::Overflow(Sign::Positive))
                    } else if minus_one {
                        Ok(Exact(-FBig::ONE)) // exp_m1(−∞) = −1 (finite)
                    } else {
                        Err(FpError::Underflow(Sign::Positive)) // exp(−∞) = +0
                    };
                }
            };
            (s, n, r)
        };

        let r = r >> n as isize;
        let mut factorial = IBig::ONE;
        let mut pow = r.clone();
        let mut sum = if no_scaling {
            r.clone()
        } else {
            FBig::ONE + &r
        };

        let mut k = 2;
        loop {
            factorial *= k;
            pow *= &r;

            let increase = &pow / &factorial;
            if increase.abs_cmp(&sum.sub_ulp()).is_le() {
                break;
            }
            sum += increase;
            k += 1;
        }

        if no_scaling {
            Ok(sum.with_precision(self.precision))
        } else if minus_one {
            // Power at the series' working precision (it already carries the 2n guard
            // digits that the Bⁿ powering amplifies away). The final "−1" can cancel at
            // most ~1 leading digit here (the |x| < 1/B case is handled by no_scaling),
            // which the same guard digits comfortably absorb.
            let pow_ctx = Context::<R>::new(work_precision);
            let v = pow_ctx.unwrap_fp(pow_ctx.powi(sum.repr(), Repr::<B>::BASE.pow(n).into()));
            Ok(((v << s) - FBig::ONE).with_precision(self.precision))
        } else {
            let pow_ctx = Context::<R>::new(work_precision);
            let v = pow_ctx.unwrap_fp(pow_ctx.powi(sum.repr(), Repr::<B>::BASE.pow(n).into()));
            Ok((v << s).with_precision(self.precision))
        }
    }
}
