use core::convert::TryInto;

use crate::{
    error::{assert_finite, assert_limited_precision, FpError, FpResult},
    fbig::FBig,
    math::cache::{reborrow_cache, ConstCache},
    repr::{Context, Repr, Word},
    round::{ErrorBounds, Round},
    utils::ceil_usize,
};
use dashu_base::{Abs, AbsOrd, Approximation::*, BitTest, DivRemEuclid, EstimatedLog2, Sign};
use dashu_int::IBig;

impl<R: Round, const B: Word> FBig<R, B> {
    /// Raise the floating point number to an integer power.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// # use core::str::FromStr;
    /// let a = DBig::from_str("-1.234")?;
    /// assert_eq!(a.powi(10.into()), DBig::from_str("8.188")?);
    /// # Ok::<(), ParseError>(())
    /// ```
    #[inline]
    pub fn powi(&self, exp: IBig) -> FBig<R, B> {
        self.context.unwrap_fp(self.context.powi(&self.repr, exp))
    }
}

// `powf`/`exp`/`exp_m1` route through the Ziv-backed Context methods, which require `R: ErrorBounds`
// for their correctness guarantee.
impl<R: ErrorBounds, const B: Word> FBig<R, B> {
    /// Raise the floating point number to an floating point power.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// # use core::str::FromStr;
    /// let x = DBig::from_str("1.23")?;
    /// let y = DBig::from_str("-4.56")?;
    /// assert_eq!(x.powf(&y), DBig::from_str("0.389")?);
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
    /// # use core::str::FromStr;
    /// let a = DBig::from_str("-1.234")?;
    /// assert_eq!(a.exp(), DBig::from_str("0.2911")?);
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
    /// # use core::str::FromStr;
    /// let a = DBig::from_str("-0.1234")?;
    /// assert_eq!(a.exp_m1(), DBig::from_str("-0.11609")?);
    /// # Ok::<(), ParseError>(())
    /// ```
    #[inline]
    pub fn exp_m1(&self) -> FBig<R, B> {
        self.context
            .unwrap_fp(self.context.exp_m1(&self.repr, None))
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
    /// # use core::str::FromStr;
    /// use dashu_base::Approximation::*;
    /// use dashu_float::{Context, round::{mode::HalfAway, Rounding::*}};
    ///
    /// let context = Context::<HalfAway>::new(2);
    /// let a = DBig::from_str("-1.234")?;
    /// assert_eq!(context.powi(&a.repr(), 10.into()), Ok(Inexact(DBig::from_str("8.2")?, AddOne)));
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
                Err(FpError::Overflow(if base.sign() == Sign::Negative {
                    Sign::Negative
                } else {
                    Sign::Positive
                }))
            } else {
                // |base| < 1 and exponent huge → underflow to signed zero
                let underflow_sign = if base.sign() == Sign::Negative && exp.bit(0) {
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

    /// Near-correct exp core: evaluate `exp(x)` (or `exp_m1(x)` when `minus_one`) at
    /// `work_precision`, returning `(value, error_radius)`.
    ///
    /// Shared by the Ziv-backed `exp`/`exp_m1` (which retry it) and usable directly where only a
    /// near-correct value is needed. The caller must have pre-checked that the reduction quotient
    /// `s = floor(x/ln B)` fits `isize` (astronomical `|x|` overflows and is handled before the
    /// Ziv loop, since this closure can't return `Err`). `n` (the reduction power, `≈ √p`) is
    /// derived from the *target* precision and is constant across retries.
    pub(crate) fn exp_compute<const B: Word>(
        &self,
        x: &Repr<B>,
        work_precision: usize,
        minus_one: bool,
        n: usize,
        mut cache: Option<&mut ConstCache>,
    ) -> (FBig<R, B>, FBig<R, B>) {
        // exp(x) = B^s · exp(r)^(Bⁿ), with r = x − s·ln(B) reduced so |r| < B⁻ⁿ.
        let context = Context::<R>::new(work_precision);
        let x = FBig::new(context.repr_round_ref(x).value(), context);

        // When minus_one is true and |x| < 1/B, evaluate the Maclaurin series without scaling
        // (no Bⁿ reduction, no powering — n_eff = 0).
        let no_scaling = minus_one && x.log2_est() < -B.log2_est();

        let (s, r, n_eff) = if no_scaling {
            (0isize, x, 0usize)
        } else {
            let logb = context.ln_base::<B>(reborrow_cache(&mut cache));
            let (s_big, r) = x.div_rem_euclid(logb);
            let s: isize = s_big
                .try_into()
                .expect("exp reduction quotient fits isize (overflow pre-checked)");
            (s, r, n)
        };
        let r = r >> n_eff as isize;

        // Maclaurin series: exp(r) = 1 + Σ rⁱ/i!
        let mut factorial = IBig::ONE;
        let mut pow = r.clone();
        let mut sum = if no_scaling {
            r.clone()
        } else {
            FBig::ONE + &r
        };
        let mut k = 2u32;
        let mut terms: usize = 1;
        loop {
            factorial *= k;
            pow *= &r;

            let increase = &pow / &factorial;
            if increase.abs_cmp(&sum.sub_ulp()).is_le() {
                break;
            }
            sum += increase;
            k += 1;
            terms += 1;
        }

        // The radius is computed at *unlimited* precision so the bound arithmetic is exact — a
        // work-precision product would drop digits and could under-estimate (a soundness hole).
        let ulp_w = || sum.ulp().with_precision(0).value();

        if no_scaling {
            // exp_m1(x) = sum directly; error is the series truncation + rounding.
            let radius = ulp_w() * (4 * terms + 8) + ulp_w();
            (sum, radius)
        } else {
            // Powering amplifies the series' relative error by Bⁿ. With |v|/|sum| < e < 3 (both
            // near 1, since |r| < B⁻ⁿ), |v − true| ≤ 3·Bⁿ·(4K+8)·ulp(sum) + ulp(v). The B^s shift
            // is exact, so the bound shifts with the value.
            let pow_ctx = Context::<R>::new(work_precision);
            let v = pow_ctx.unwrap_fp(pow_ctx.powi(sum.repr(), Repr::<B>::BASE.pow(n).into()));
            let v_shifted = v.clone() << s;
            let e_v = (ulp_w() << n as isize) * (4 * terms + 8) * 3u32
                + v.ulp().with_precision(0).value();
            let radius = if minus_one {
                // result = v_shifted − 1; the subtraction adds one result-ULP of rounding.
                let result = &v_shifted - FBig::ONE;
                let radius = (e_v << s) + result.ulp().with_precision(0).value();
                return (result, radius);
            } else {
                e_v << s
            };
            (v_shifted, radius)
        }
    }
}

/// Hoisted `exp` overflow probe for the Ziv closures (which can't return `Err`). Returns `true`
/// when `exp(x)` is outside the finite exponent range — astronomically large `|x|` (the reduction
/// quotient `s = x/ln B` overflows `isize`). True for both signs of huge `x` (the quotient
/// *magnitude* overflows `isize`). Shared by `exp_internal`, `powf`, and the hyperbolic functions.
pub(crate) fn exp_overflows<R: Round, const B: Word>(
    ctx: &Context<R>,
    x: &Repr<B>,
    cache: &mut Option<&mut ConstCache>,
) -> bool {
    if x.log2_est().abs() <= 61.0 {
        return false;
    }
    let probe = Context::<R>::new(ctx.precision + 64);
    let logb = probe.ln_base::<B>(reborrow_cache(cache));
    let x_probe = FBig::new(probe.repr_round_ref(x).value(), probe);
    let s_probe = x_probe.div_rem_euclid(logb).0;
    <isize as core::convert::TryFrom<IBig>>::try_from(s_probe).is_err()
}

// `powf` (non-integer exponent), `exp`, and `exp_m1` are correctly rounded via the Ziv loop, so
// they require `R: ErrorBounds`. `powf` with an integer-valued exponent delegates to `powi`
// (`R: Round`, near-correct within 1 ulp).
impl<R: ErrorBounds> Context<R> {
    /// Raise the floating point number to an floating point power under this context.
    ///
    /// A non-integer exponent is correctly rounded via a Ziv loop. An integer-valued exponent
    /// delegates to [`powi`](Context::powi) (binary exponentiation), which also accepts a negative
    /// base — its sign is fixed by the exponent's parity — so `pow(-x, n)` is in domain here for
    /// integer `n`. The integer-exponent path is within 1 ulp (near-correct), matching `powi`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// # use core::str::FromStr;
    /// use dashu_base::Approximation::*;
    /// use dashu_float::{Context, round::{mode::HalfAway, Rounding::*}};
    ///
    /// let context = Context::<HalfAway>::new(2);
    /// let x = DBig::from_str("1.23")?;
    /// let y = DBig::from_str("-4.56")?;
    /// assert_eq!(context.powf(&x.repr(), &y.repr(), None), Ok(Inexact(DBig::from_str("0.39")?, AddOne)));
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
        assert_limited_precision(self.precision);

        // shortcuts
        if exp.is_pos_zero() || exp.is_neg_zero() {
            // pow(x, ±0) = 1 for any base (IEEE 754 §9.2.1); `-0` is numerically zero, so it
            // must take the same shortcut as `+0` (otherwise a negative base falls through to
            // the OutOfDomain path below).
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
        if base.is_one() {
            // pow(1, y) = 1 for any finite y (exp is finite here — infinities were rejected above).
            return Ok(Exact(FBig::ONE));
        }

        // Integer-valued exponent: delegate to the integer-power kernel (binary exponentiation).
        // This sidesteps the `exp(y·ln x)` amplification entirely, and lets a negative base through
        // — `powi` fixes the sign from the exponent's parity. `powi` is near-correct (≤ 1 ulp).
        // Gated on `is_int` (a cheap exponent check) so the non-integer common case skips `to_int`.
        if exp.is_int() {
            return self.powi(base, exp.to_int().value());
        }

        if base.sign() == Sign::Negative {
            // A non-integer exponent on a negative base has no real value.
            return Err(FpError::OutOfDomain);
        }

        // x^y = exp(y·ln x), correctly rounded via the Ziv loop. `ln` and `exp` are themselves
        // Ziv-correct at the working precision, so the radius comes only from the rounding of the
        // `ln`/`mul`/`exp` chain — but `exp` AMPLIFIES the absolute error of its argument `y·ln x`
        // by the result magnitude, i.e. by a relative factor of `|y·ln x|`. The radius is
        // `result.ulp() · (|y·ln x| + 1) · (B + 8)` where `result.ulp()` is taken at the *working*
        // precision, so it shrinks as `B^{-guard}` and the containment test converges. (A radius
        // computed at unlimited precision would be constant across retries and never converge for
        // a value near a rounding boundary.) The `B + 8` scale covers the `ulp`-vs-`value·B^{1-P}`
        // gap plus a safety margin for the chained roundings.
        //
        // The overflow case is hoisted out of the Ziv closure (which can't return `Err`): if
        // `exp(y·ln x)` falls outside the finite exponent range, short-circuit before the loop.
        let probe = Context::<R>::new(self.precision + 32);
        let ln_x_probe = probe.ln(base, reborrow_cache(&mut cache))?.value();
        let arg_probe = probe.mul(ln_x_probe.repr(), exp)?.value();
        if exp_overflows::<R, B>(&probe, arg_probe.repr(), &mut cache) {
            return Err(if arg_probe.sign() == Sign::Positive {
                FpError::Overflow(Sign::Positive)
            } else {
                FpError::Underflow(Sign::Positive)
            });
        }

        let initial_guard = ceil_usize(self.precision.log2_est() / B.log2_est()) + 10;
        Ok(self.ziv(initial_guard, |guard| {
            let work = Context::<R>::new(self.precision + guard);
            let ln_x = work.ln(base, reborrow_cache(&mut cache)).unwrap().value();
            let arg = work.mul(ln_x.repr(), exp).unwrap().value();
            let result = work
                .exp(arg.repr(), reborrow_cache(&mut cache))
                .unwrap()
                .value();

            // Radius at unlimited precision (exact arithmetic), but built from the *work-precision*
            // `result.ulp()` so it carries the `B^{-(p+guard)}` scale and shrinks across retries.
            let ulp_w = result.ulp().with_precision(0).value();
            let arg_abs = arg.abs().with_precision(0).value();
            let scale = (B as i32) + 8;
            let radius = (ulp_w * (arg_abs + FBig::<R, B>::ONE)) * scale;
            (result, radius)
        }))
    }

    /// Calculate the exponential function (`eˣ`) on the floating point number under this context.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// # use core::str::FromStr;
    /// use dashu_base::Approximation::*;
    /// use dashu_float::{Context, round::{mode::HalfAway, Rounding::*}};
    ///
    /// let context = Context::<HalfAway>::new(2);
    /// let a = DBig::from_str("-1.234")?;
    /// assert_eq!(context.exp(&a.repr(), None), Ok(Inexact(DBig::from_str("0.29")?, NoOp)));
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
    /// # use core::str::FromStr;
    /// use dashu_base::Approximation::*;
    /// use dashu_float::{Context, round::{mode::HalfAway, Rounding::*}};
    ///
    /// let context = Context::<HalfAway>::new(2);
    /// let a = DBig::from_str("-0.1234")?;
    /// assert_eq!(context.exp_m1(&a.repr(), None), Ok(Inexact(DBig::from_str("-0.12")?, SubOne)));
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
            // exp(±0) = 1; exp_m1(±0) = ±0 (IEEE 754 §9.2.1 preserves the sign of zero)
            return match minus_one {
                false => Ok(Exact(FBig::ONE)),
                true => {
                    let zero = if input_sign == Sign::Negative {
                        FBig::new(Repr::neg_zero(), Context::new(0))
                    } else {
                        FBig::ZERO
                    };
                    Ok(Exact(zero))
                }
            };
        }

        // Hoisted overflow check: the reduction quotient s = floor(x/ln B) overflows isize only
        // for astronomically large |x| (|x| ≳ 2^61). The Ziv closure below can't return Err, so
        // detect that case here and short-circuit to overflow/underflow (matching IEEE limits).
        if x.log2_est().abs() > 61.0 {
            let probe = Context::<R>::new(self.precision + 64);
            let logb = probe.ln_base::<B>(reborrow_cache(&mut cache));
            let x_probe = FBig::new(probe.repr_round_ref(x).value(), probe);
            let s_probe = x_probe.div_rem_euclid(logb).0;
            if <isize as core::convert::TryFrom<IBig>>::try_from(s_probe).is_err() {
                return if input_sign == Sign::Positive {
                    Err(FpError::Overflow(Sign::Positive))
                } else if minus_one {
                    Ok(Exact(-FBig::ONE)) // exp_m1(−∞) = −1 (finite)
                } else {
                    Err(FpError::Underflow(Sign::Positive)) // exp(−∞) = +0
                };
            }
        }

        // Correct rounding via the Ziv loop. Guards: log_B(p) for the series summation/squaring
        // rounding, plus `n` for the Bⁿ powering amplification — halved from the pre-Ziv `2n`,
        // since Ziv (not the guard count) now certifies correctness. `n ≈ √p` is derived from the
        // target precision and is constant across retries.
        let series_guard = ceil_usize(self.precision.log2_est() / B.log2_est());
        let n = 1usize << (self.precision.bit_len() / 2);
        Ok(self.ziv(series_guard + n, |guard| {
            self.exp_compute::<B>(
                x,
                self.precision + guard,
                minus_one,
                n,
                reborrow_cache(&mut cache),
            )
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::round::mode;

    #[test]
    fn test_exp_overflow_is_infinity() {
        let ctx = Context::<mode::HalfEven>::new(53);
        // exp(huge) overflows the isize exponent range -> Overflow at Context level.
        // Need x large enough that floor(x/ln2) > isize::MAX, i.e. x > ~2^62.5.
        let huge = Repr::new(IBig::from(1) << 63, 0);
        assert_eq!(ctx.exp::<2>(&huge, None), Err(FpError::Overflow(Sign::Positive)));

        // exp(huge negative) underflows to +0
        let neg = Repr::new(-(IBig::from(1) << 63), 0);
        assert_eq!(ctx.exp::<2>(&neg, None), Err(FpError::Underflow(Sign::Positive)));

        // exp_m1(huge negative) -> -1 (a finite value, not an error)
        let m1 = ctx.exp_m1::<2>(&neg, None).unwrap().value();
        assert_eq!(m1, -FBig::<mode::HalfEven>::ONE);
    }

    #[test]
    fn test_powf_zero_base() {
        use crate::DBig;
        // powf with a float exponent returns the *positive* result on a zero base
        // (matching the common float-pow convention); use powi for the signed result.
        let ctx = Context::<mode::HalfEven>::new(53);
        // powf(-0, 2.0) = +0 (NOT -0)
        let r = ctx
            .powf::<2>(&Repr::<2>::neg_zero(), &Repr::new(2.into(), 0), None)
            .unwrap()
            .value();
        assert!(r.repr().is_pos_zero(), "expected +0");
        assert!(!r.repr().is_neg_zero(), "powf(-0, x) should be +0, not -0");
        // powf(0, -1) = +inf
        let r = ctx
            .powf::<2>(&Repr::<2>::zero(), &Repr::new((-1i32).into(), 0), None)
            .unwrap()
            .value();
        assert!(r.repr().is_infinite());
        assert_eq!(r.repr().sign(), Sign::Positive);
        // powi(-0, 3) = -0 (the sign-correct, integer-exponent variant)
        let r = ctx
            .powi::<2>(&Repr::<2>::neg_zero(), 3.into())
            .unwrap()
            .value();
        assert!(r.repr().is_neg_zero());
        let _ = DBig::ZERO;
    }

    #[test]
    fn test_powf_integer_exponent() {
        use crate::DBig;
        let ctx = Context::<mode::HalfEven>::new(53);
        // integer-valued float exponent delegates to powi and supports a negative base (its sign
        // is fixed by the exponent's parity): (-5)^3 = -125.
        let neg_base = &Repr::<2>::new((-5).into(), 0);
        let exp3 = &Repr::<2>::new(3.into(), 0);
        let via_powf = ctx.powf::<2>(neg_base, exp3, None).unwrap().value();
        let via_powi = ctx.powi::<2>(neg_base, 3.into()).unwrap().value();
        assert_eq!(via_powf.repr(), via_powi.repr());
        assert_eq!(via_powf.repr().sign(), Sign::Negative);

        // a non-integer exponent on a negative base is out of domain (no real value)
        let exp_half = &Repr::<2>::new(5.into(), -1); // 2.5
        assert_eq!(ctx.powf::<2>(neg_base, exp_half, None), Err(FpError::OutOfDomain));

        // positive base, integer exponent: also routes through powi
        let pos_base = &Repr::<2>::new(3.into(), 0);
        let exp4 = &Repr::<2>::new(4.into(), 0);
        let r = ctx.powf::<2>(pos_base, exp4, None).unwrap().value();
        assert_eq!(r.repr(), ctx.powi::<2>(pos_base, 4.into()).unwrap().value().repr());
        let _ = DBig::ZERO;
    }
}
