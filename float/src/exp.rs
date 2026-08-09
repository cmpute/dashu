use core::cmp::Ordering;
use core::convert::TryInto;

use crate::{
    ball::Ball,
    error::{assert_finite, assert_limited_precision, FpError, FpResult},
    fbig::FBig,
    math::cache::{reborrow_cache, ConstCache},
    repr::{Context, Repr, Word},
    round::{mode, ErrorBounds, Round, Rounded, Rounding::*},
};
use dashu_base::{Abs, AbsOrd, Approximation::*, BitTest, DivRemEuclid, EstimatedLog2, Sign};
use dashu_int::{IBig, UBig};

// `|x|` (in log2) above which exp's reduction quotient `s = floor(x/ln B)` might overflow `isize`,
// so the hoisted overflow probe must run instead of the fast skip. `s` overflows when
// `|x| > isize::MAX · ln B`, i.e. `log2|x| > log2(isize::MAX) + log2(ln B)`; the minimum (over
// `B ≥ 2`) is `~isize::BITS − 1.5`, and the `−3` margin stays below it. The literal was `61` (the
// 64-bit value); on 32-bit `isize` it must be ~29 or `exp_compute`'s `s.try_into().expect()` panics
// for inputs like `exp(-2⁵⁰)` (whose `s ≈ -1.8e15` overflows 32-bit `isize`).
const EXP_OVERFLOW_PROBE_LOG2: f32 = (isize::BITS - 3) as f32;

// Maximum bit length of a `powi` exponent for which the binary squaring chain is feasible. The
// chain runs `n.bit_len() - 1` correctly-rounded squarings at a working precision that grows with
// `n.bit_len()`, and compounds relative error ~`2^nlen · ulp`; for `nlen` in the millions/billions
// (an `IBig` exponent far past `i64`) a single attempt exhausts memory. Exponents beyond `i64`
// (`nlen > 64`) only ever produce a finite result when `|base| ≈ 1` (otherwise the magnitude
// overflows the finite range), so they go to the `exp(y·ln x)` fallback instead of the chain.
const MAX_POWI_CHAIN_BITS: usize = 64;

// Margin certifying a `powi` result is outside the finite range: the magnitude guard estimates the
// result's log2 as `e · log2(base)` in f64, where `e` is up to `i64::MAX` and `isize::MAX as f64`
// is itself rounded — together ~2^17 of f64 error near the boundary. A result whose log2 is within
// this margin of the threshold is deferred to the squaring chain (whose `checked_mul` catches a
// genuine overflow) rather than risk a false-positive overflow returning ±∞.
const POWI_RANGE_MARGIN: f64 = (1 << 20) as f64;

// `powi` (integer power), `powf`/`exp`/`exp_m1` route through Ziv-backed Context methods, which
// require `R: ErrorBounds` for their correctness guarantee.
impl<R: ErrorBounds, const B: Word> FBig<R, B> {
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

impl<R: Round> Context<R> {
    /// Near-correct exp core: evaluate `exp(x)` (or `exp_m1(x)` when `minus_one`) at
    /// `work_precision`, returning a [`Ball`] whose radius is derived mechanically.
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
    ) -> Result<Ball<B>, FpError> {
        // exp(x) = B^s · exp(r)^(Bⁿ), with r = x − s·ln(B) reduced so |r| < B⁻ⁿ.
        let context = Context::<mode::HalfEven>::new(work_precision);
        let x_ball = Ball::from_rounded(context.repr_round_ref(x).map(|r| FBig::new(r, context)));

        // When minus_one is true and |x| < 1/B, evaluate the Maclaurin series without scaling
        // (no Bⁿ reduction, no powering — n_eff = 0).
        let no_scaling = minus_one && x_ball.mid.log2_est() < -B.log2_est();
        let (s, r_ball, n_eff) = if no_scaling {
            (0isize, x_ball, 0usize)
        } else {
            // The reduction quotient `s = floor(x / ln B)` amplifies ln(B)'s rounding error by
            // |x|: a 1-ulp error in ln(B) shifts `s` (and thus the result exponent) by ~|x|/ln B.
            // For large |x| the work precision is far too low to pin `s` — exp(5.7e14) at p=24 was
            // certified with the exponent off by ~1000 — so compute ln(B) with `⌈log_B|x|⌉ + 2`
            // extra digits, enough that the reduction contributes well under one work-ulp and the
            // series/powering radius bounds the total error. (The bounds, not the point estimate,
            // guard the inflation magnitude.)
            let x_log2_ub = x_ball.mid.log2_bounds().1;
            let extra = if x_log2_ub > 0.0 {
                (x_log2_ub / B.log2_est()) as usize + 2
            } else {
                2
            };
            // ln(B) as a ball: the cached bases (2, 10, powers of 2) are correctly rounded (the
            // fixed 8-ulp bound in `ln_base_ball` is sound there), while generic bases carry
            // `ln_compute`'s mechanical radius (their atanh series error can be far larger than 8).
            let logb_ball = Context::<mode::HalfEven>::new(work_precision + extra)
                .ln_base_ball::<B>(reborrow_cache(&mut cache));
            let x_sign = x_ball.mid.repr().sign();
            let (s_big, _) = x_ball.mid.clone().div_rem_euclid(logb_ball.mid.clone());
            let s: isize = match s_big.try_into() {
                Ok(s) => s,
                Err(_) => {
                    // |x| is astronomical — the reduction quotient overflows isize. The magnitude
                    // gate in `exp_internal` is meant to catch this first; reaching here is a
                    // gray-zone miss, so surface the range error it would have returned.
                    return Err(if x_sign == Sign::Positive {
                        FpError::Overflow(Sign::Positive)
                    } else {
                        FpError::Underflow(Sign::Positive)
                    });
                }
            };
            // r = x − s·ln(B), as a ball: the cancellation and ln(B)'s error are tracked by the
            // Ball propagation.
            let r_ball = x_ball.sub(&logb_ball.scale_int(&IBig::from(s)));
            (s, r_ball, n)
        };
        let r_ball = r_ball.shift(n_eff as isize);

        // Maclaurin series: exp(r) = 1 + Σ rⁱ/i! (exp_m1(x) = Σ xⁱ/i! when no_scaling).
        let one = Ball::exact_int(r_ball.mid.precision(), IBig::ONE);
        let mut factorial = IBig::ONE;
        let mut pow = r_ball.clone();
        let mut sum = if no_scaling {
            r_ball.clone()
        } else {
            one.add(&r_ball)
        };
        let mut k = 2u32;
        loop {
            factorial *= k;
            pow = pow.mul(&r_ball);

            let increase = pow.div_exact(&factorial);
            if increase.mid.abs_cmp(&sum.mid.ulp_lb()).is_le() {
                break;
            }
            sum = sum.add(&increase);
            k += 1;
        }

        // Omitted series tail: the exp terms shrink by r/(i+1) < 1/4 (|r| < B⁻ⁿ), so the tail is
        // < 2 ulps of sum.
        sum.inflate(&IBig::from(2));

        if no_scaling {
            // exp_m1(x) = sum directly.
            return Ok(sum);
        }
        // Powering: exp(r)^(Bⁿ) = sum^Bⁿ, via binary exponentiation (the compounding rounding is
        // tracked by the Ball multiplications). The chain is exact only when the series sum is
        // exact (impossible here — sum ≈ exp(r) with |r| < B⁻ⁿ), so the exact flag is ignored.
        let bn = Repr::<B>::BASE.pow(n);
        let (v_ball, _) = sum.pow_exact(&bn)?;

        // B^s is an exact power-of-base shift: `exp(x) = B^s · exp(r)^(Bⁿ)`.
        let v_shifted = v_ball.shift(-s);

        if minus_one {
            // exp_m1(x) = exp(x) − 1; the subtraction folds one rounding ulp.
            let one = Ball::exact_int(v_shifted.mid.precision(), IBig::ONE);
            Ok(v_shifted.sub(&one))
        } else {
            Ok(v_shifted)
        }
    }

    /// `exp` of a *ball* input. [`exp_compute`](Self::exp_compute) evaluates on `x.mid`; the input
    /// ball's own error `|θ| ≤ x.n·ulp(x)` then contributes `exp(x)·|θ|` to the result (the
    /// exponential's derivative is itself), folded into the radius.
    pub(crate) fn exp_ball<const B: Word>(
        &self,
        x: &Ball<B>,
        mut cache: Option<&mut ConstCache>,
    ) -> Result<Ball<B>, FpError> {
        let n = 1usize << (self.precision.bit_len() / 2);
        let mut result = self.exp_compute::<B>(
            x.mid.repr(),
            x.mid.precision(),
            false,
            n,
            reborrow_cache(&mut cache),
        )?;
        if !x.n.is_zero() {
            // `e_r` is the raw significand exponent (`mid_r = sig_r·B^(e_r)`), `lead_*` is the
            // leading position (`lead_exp`), so `ulp_x = B^(lead_x − p_x)` and
            // `ulp_r = B^(lead_r − p_r)`. The exponential's derivative is itself, so the input
            // error propagates as `n_x·ulp_x·|exp|/ulp_r = n_x·sig_r·B^(lead_x − p_x + e_r − lead_r + p_r)`,
            // the exact derivative bound (no small-constant factor needed, unlike ln's 1/(1+x)).
            // The `sig_r = |mid_r|` factor is essential: it scales the input ulp up to the
            // result's magnitude. Omitting it under-bounds the radius by `sig_r` (≈ B^(p−1)) — the
            // Ziv containment test then certifies an interval that does not contain the true value
            // (e.g. `powf` with a large |y·ln x|).
            let sig_r = result.mid.repr().significand.clone().abs();
            let e_r = result.mid.repr().exponent;
            let lead_r = Ball::lead_exp(&result.mid);
            let p_r = result.mid.precision();
            let lead_x = Ball::lead_exp(&x.mid);
            let p_x = x.mid.precision();
            let shift = lead_x - p_x as isize + e_r - lead_r + p_r as isize;
            result.inflate(&crate::ball::ceil_shift::<B>(x.n.clone() * sig_r, shift));
        }
        Ok(result)
    }

    /// `exp` of a *ball* input. [`exp_compute`](Self::exp_compute) evaluates on `x.mid`; the input
    /// ball's own error `|θ| ≤ x.n·ulp(x)` then contributes `exp(x)·|θ|` to the result (the
    /// exponential's derivative is itself), folded into the radius.
    /// Directed saturation endpoint for an FBig result that has underflowed below the smallest
    /// representable magnitude (its exponent would fall below `isize::MIN`). Outward modes round
    /// the magnitude up to the smallest `B^{isize::MIN}` of the result's sign; toward-zero, the
    /// opposite direction, and nearest round to signed zero. This mirrors the f32/f64 directed
    /// underflow and is the shared endpoint used by `exp_extreme_negative`, `powi`, and `powf`, so
    /// a directed `pow` (e.g. `pow(10, y)` ≈ `exp(y·ln 10)`) saturates to the same value `exp` does
    /// — keeping `Up ≥ Down` consistent across them. The endpoint carries the input context, so a
    /// downstream op keeps a limited precision.
    pub(crate) fn underflow_repr_endpoint<const B: Word>(&self, sign: Sign) -> Rounded<FBig<R, B>> {
        let adj = if sign == Sign::Positive {
            R::round_low_part(&IBig::ZERO, Sign::Positive, || Ordering::Less)
        } else {
            R::round_low_part(&IBig::ZERO, Sign::Negative, || Ordering::Less)
        };
        match adj {
            AddOne => Inexact(FBig::new(Repr::new(IBig::ONE, isize::MIN), *self), AddOne),
            SubOne => Inexact(
                FBig::new(
                    Repr::new(IBig::from_parts(Sign::Negative, UBig::ONE), isize::MIN),
                    *self,
                ),
                SubOne,
            ),
            _ => Inexact(FBig::new(Repr::<B>::zero_with_sign(sign), *self), NoOp),
        }
    }

    /// Directed saturation endpoint for an FBig result that has overflowed above the largest
    /// representable magnitude (its exponent would exceed `isize::MAX`). Outward modes (Up/Away for
    /// positive, Down/Away for negative) and nearest reach `±∞`; inward modes (toward-zero, and the
    /// opposite-infinity direction) saturate to the largest finite `(Bᵖ−1) × B^{isize::MAX}` — the
    /// all-`(B−1)` significand at the max exponent, mirroring MPFR's `mpfr_setmax` (which fills the
    /// significand with all 1-bits *at the output precision* — the significand is `p` digits, not
    /// the value's magnitude). The largest finite is ill-defined at unlimited precision, so this
    /// panics when `precision == 0`.
    pub(crate) fn overflow_repr_endpoint<const B: Word>(&self, sign: Sign) -> Rounded<FBig<R, B>> {
        assert_limited_precision(self.precision);
        let adj = if sign == Sign::Positive {
            R::round_low_part(&IBig::ONE, Sign::Positive, || Ordering::Greater)
        } else {
            R::round_low_part(&IBig::NEG_ONE, Sign::Negative, || Ordering::Greater)
        };
        match adj {
            AddOne => Inexact(FBig::new(Repr::infinity_with_sign(Sign::Positive), *self), AddOne),
            SubOne => Inexact(FBig::new(Repr::infinity_with_sign(Sign::Negative), *self), SubOne),
            _ => {
                // Largest finite at this precision: (B^p − 1) × B^{isize::MAX}.
                let max_mag = Repr::<B>::BASE.pow(self.precision) - UBig::ONE;
                Inexact(
                    FBig::new(Repr::new(IBig::from_parts(sign, max_mag), isize::MAX), *self),
                    NoOp,
                )
            }
        }
    }
}

// `powi` (integer power), `powf` (non-integer exponent), `exp`, and `exp_m1` are correctly rounded
// via the Ziv loop, so they require `R: ErrorBounds`. `powf` with an integer-valued exponent
// delegates to `powi`.
impl<R: ErrorBounds> Context<R> {
    /// Raise the floating point number to an integer power under this context, correctly rounded
    /// via a Ziv retry loop.
    ///
    /// `base^n` is computed by left-to-right binary exponentiation (repeated squaring); a negative
    /// exponent computes `(1/base)^|n|`, so the sign-dependent overflow/underflow falls out
    /// naturally. Each squaring compounds the relative error (it roughly doubles per step), so
    /// after `n.bit_len()` squarings the error is bounded by about `2^nlen · ulp` — the Ziv radius
    /// reflects that, and the loop retries with more guard digits until the working-precision
    /// interval unambiguously determines the target rounding.
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
    /// Panics if the precision is unlimited and the exponent is negative (the exact `1/base` is
    /// not finite in general).
    pub fn powi<const B: Word>(&self, base: &Repr<B>, exp: IBig) -> FpResult<FBig<R, B>> {
        if base.is_infinite() {
            return Err(FpError::InfiniteInput);
        }
        let (exp_sign, n) = exp.into_parts();
        let negative = exp_sign == Sign::Negative;
        if negative {
            // a negative exponent needs 1/base, which is not finite at unlimited precision
            assert_limited_precision(self.precision);
        }

        if n.is_zero() {
            return Ok(Exact(FBig::ONE));
        }
        if n.is_one() {
            if negative {
                // base^(-1) = 1/base: a single correctly-rounded division
                return self.div(&Repr::one(), base);
            }
            let repr = self.repr_round_ref(base);
            return Ok(repr.map(|v| FBig::new(v, *self)));
        }

        // Zero base (±0): a positive exponent gives ±0, a negative one ±inf; the sign follows |n|'s
        // parity. Short-circuit before the magnitude pre-check (whose log2 estimate is meaningless
        // for zero) and before the squaring chain (which can't start from zero).
        let odd = n.bit(0);
        if base.significand.is_zero() {
            let neg_sign = base.sign() == Sign::Negative && odd;
            if negative {
                let sign = if neg_sign {
                    Sign::Negative
                } else {
                    Sign::Positive
                };
                return Ok(Exact(FBig::new(Repr::<B>::infinity_with_sign(sign), *self)));
            }
            let repr = if neg_sign {
                Repr::<B>::neg_zero()
            } else {
                Repr::<B>::zero()
            };
            return Ok(Exact(FBig::new(repr, *self)));
        }

        // Magnitude pre-check: the result's log2 is `signed_exp · log2|base|`; outside the finite
        // exponent range it short-circuits to overflow/underflow instead of letting the squaring
        // chain overflow mid-computation (the Ziv closure below can't return `Err`).
        //
        // Use the *bounds* of log2(base), never the point estimate `log2_est`: when base is very
        // close to 1 (a large significand with a large negative exponent), log2(base) is the
        // difference of two large terms and suffers catastrophic cancellation — `log2_est` returns
        // ~1e-4 of f32 noise rather than ~0. Scaled by a large exponent that noise crosses the
        // overflow threshold, which on 32-bit is only isize::MAX·log2(B) ≈ 7e9 (vs ≈3e19 on 64-bit),
        // so the guard fires spuriously and returns ±inf — see issue #95 (it crashed high-precision
        // `FBig::with_base` on wasm32/i686). The bounds are derived from the exact significand bit
        // length, so they don't cancel. Declare an extreme result only when a bound certifies it
        // (no false positives); anything ambiguous is computed.
        let (base_log2_lb, base_log2_ub) = base.log2_bounds();
        let base_log2_lb = base_log2_lb as f64;
        let base_log2_ub = base_log2_ub as f64;
        let threshold = (isize::MAX as f64) * (B.log2_est() as f64);
        let threshold_certain = threshold + POWI_RANGE_MARGIN;
        let exp_f64 = i64::try_from(&n).ok().map(|e| e as f64);
        // `lb_side` certifies |base| > 1 by a wide margin; `ub_side` certifies |base| < 1. (For the
        // None case |exp| is unbounded, so the bound's sign alone decides.) A negative exponent
        // swaps which side over- vs underflows.
        let lb_side = match exp_f64 {
            Some(e) => e * base_log2_lb > threshold_certain,
            None => base_log2_lb > 0.0,
        };
        let ub_side = match exp_f64 {
            Some(e) => e * base_log2_ub < -threshold_certain,
            None => base_log2_ub < 0.0,
        };
        if lb_side || ub_side {
            // |base|>1 (lb_side): positive exp → overflow, negative exp → underflow.
            // |base|<1 (ub_side): positive exp → underflow, negative exp → overflow.
            let overflow = (lb_side && !negative) || (ub_side && negative);
            let sign = if base.sign() == Sign::Negative && odd {
                Sign::Negative
            } else {
                Sign::Positive
            };
            return Err(if overflow {
                FpError::Overflow(sign)
            } else {
                FpError::Underflow(sign)
            });
        }

        // |exp| doesn't fit i64 and the bounds straddle 0, so |base| is within the bounds of 1.
        // If it is *exactly* ±1 the result is ±1 for any exponent (short-circuit so the squaring
        // chain doesn't iterate over exp's enormous bit length); otherwise |base| ≈ 1 but ≠ 1, the
        // huge power is still finite, and we fall through to compute it.
        if exp_f64.is_none() && base.significand.is_one() && base.exponent == 0 {
            let repr = if base.sign() == Sign::Negative && odd {
                Repr::<B>::neg_one()
            } else {
                Repr::<B>::one()
            };
            return Ok(Exact(FBig::new(repr, *self)));
        }

        let nlen = n.bit_len();
        // Exponents past `i64` make the squaring chain infeasible (`nlen - 1` squarings at a
        // working precision that grows with `nlen`) and, when the magnitude also overflows the
        // finite range, can drive it to exhaust memory. The only finite results at that scale have
        // `|base| ≈ 1`, which the `exp(y·ln x)` fallback computes without scaling the working
        // precision with `nlen` — so it cannot allocate unboundedly.
        if nlen > MAX_POWI_CHAIN_BITS {
            let signed_exp = IBig::from_parts(exp_sign, n.clone());
            return self.powi_via_exp_log(base, &signed_exp);
        }
        // The chain runs on the magnitude `start` (base, or its reciprocal for a negative
        // exponent), whose range-error sign is the *intermediate* sign; remap any overflow/underflow
        // to the true result sign (base sign × exponent parity) so the directed endpoint is correct.
        let result_sign = if base.sign() == Sign::Negative && odd {
            Sign::Negative
        } else {
            Sign::Positive
        };
        let initial_guard = nlen + self.base_guard_digits::<B>() + 2;
        self.ziv(initial_guard, |guard| {
            let pw = self.precision + guard;
            let work = Context::<mode::HalfEven>::new(pw);
            // start from base (positive exponent, always exact) or its working-precision reciprocal
            // (negative exponent, exact only when 1/base is exactly representable). A range error
            // here (e.g. 1/base underflows for an extreme base) is remapped to the result sign and
            // propagated — saturating would feed the chain an infinity or zero it can't recover from.
            let start_ball = if negative {
                match work.div(&Repr::one(), base) {
                    Ok(v) => Ball::from_rounded(v),
                    Err(FpError::Overflow(_)) => return Err(FpError::Overflow(result_sign)),
                    Err(FpError::Underflow(_)) => return Err(FpError::Underflow(result_sign)),
                    Err(e) => return Err(e),
                }
            } else {
                Ball::exact(FBig::new(base.clone(), work))
            };
            // The squaring chain compounds the error mechanically; when the whole chain is exact
            // (exact start, no rounding anywhere) the Ball's n is 0, reporting a zero radius — the
            // exactly-representable directed-rounding case that no nonzero radius could certify.
            let (res_ball, _) = start_ball.pow_exact(&n).map_err(|e| match e {
                FpError::Overflow(_) => FpError::Overflow(result_sign),
                FpError::Underflow(_) => FpError::Underflow(result_sign),
                other => other,
            })?;
            Ok(res_ball.to_value_radius::<R>())
        })
    }

    /// Raise the floating point number to an floating point power under this context.
    ///
    /// A non-integer exponent is correctly rounded via a Ziv loop. An integer-valued exponent
    /// delegates to [`powi`](Context::powi) (binary exponentiation), which also accepts a negative
    /// base — its sign is fixed by the exponent's parity — so `pow(-x, n)` is in domain here for
    /// integer `n`. Both paths are correctly rounded.
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
        cache: Option<&mut ConstCache>,
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

        // Integer-valued exponent: delegate to the integer-power kernel (binary exponentiation),
        // itself correctly rounded via its own Ziv loop. This sidesteps the `exp(y·ln x)`
        // amplification entirely, and lets a negative base through — `powi` fixes the sign from
        // the exponent's parity. Gated on `is_int` (a cheap exponent check) so the non-integer
        // common case skips `to_int`.
        if exp.is_int() {
            return self.powi(base, exp.to_int().value());
        }

        if base.sign() == Sign::Negative {
            // A non-integer exponent on a negative base has no real value.
            return Err(FpError::OutOfDomain);
        }

        // `base` is positive here (negative non-integer base returned OutOfDomain above).
        self.pow_exp_log(base, exp, cache)
    }

    /// `x^y = exp(y·ln x)` for `pos_base > 0`, correctly rounded via a Ziv loop — the shared core
    /// of [`powf`](Self::powf) (non-integer exponents) and the [`powi`](Self::powi) fallback for
    /// exponents past the squaring chain's feasible range. `ln` and `exp` are themselves Ziv-correct
    /// at the working precision, so the radius comes only from the rounding of the `ln`/`mul`/`exp`
    /// chain — but `exp` AMPLIFIES the absolute error of its argument `y·ln x` by the result
    /// magnitude, i.e. by a relative factor of `|y·ln x|`. The radius is
    /// `result.ulp() · (|y·ln x| + 1) · (B + 8)` where `result.ulp()` is taken at the *working*
    /// precision, so it shrinks as `B^{-guard}` and the containment test converges. (A radius
    /// computed at unlimited precision would be constant across retries and never converge for a
    /// value near a rounding boundary.) The `B + 8` scale covers the `ulp`-vs-`value·B^{1-P}` gap
    /// plus a safety margin for the chained roundings.
    ///
    /// Overflow/underflow of `exp(y·ln x)` is detected inside the Ziv closure by `exp` itself
    /// (which returns `Err(Overflow)` / `Err(Underflow)`) and propagated — the result is positive
    /// (argument to `exp`), so overflow is `+∞` and underflow carries `+` sign; callers that need a
    /// negative result (the `powi` fallback for a negative base) flip the sign of both the value
    /// and the error.
    fn pow_exp_log<const B: Word>(
        &self,
        pos_base: &Repr<B>,
        exp: &Repr<B>,
        mut cache: Option<&mut ConstCache>,
    ) -> FpResult<FBig<R, B>> {
        let initial_guard = self.base_guard_digits::<B>() + 10;
        self.ziv(initial_guard, |guard| {
            let wp = self.precision + guard;
            // exp(y·ln x) as a Ball chain: the ln and the exponent rounding compose mechanically,
            // and the exp input error is folded in via `exp_ball`. The radius shrinks with guard
            // now that `ln_compute`'s s<0 reduction no longer inflates its error count.
            let ln_ball = self.ln_compute::<B>(pos_base, wp, false, reborrow_cache(&mut cache));
            let work = Context::<mode::HalfEven>::new(wp);
            let exp_input =
                Ball::from_rounded(work.repr_round_ref(exp).map(|r| FBig::new(r, work)));
            let arg_ball = ln_ball.mul(&exp_input);
            let result_ball = self.exp_ball::<B>(&arg_ball, reborrow_cache(&mut cache))?;
            Ok(result_ball.to_value_radius::<R>())
        })
    }

    /// `powi` fallback for exponents past the squaring chain's feasible bit length: compute
    /// `base^signed_exp = exp(signed_exp · ln|base|)`. The integer exponent is an exact float
    /// (significand `|signed_exp|`, exponent `0`), so this delegates to [`pow_exp_log`](Self::pow_exp_log)
    /// — reusing its Ziv rounding — and then fixes the result sign from the
    /// exponent's parity for a negative base. The working precision never scales with
    /// `signed_exp.bit_length()`, so (unlike the squaring chain) this cannot allocate unboundedly.
    fn powi_via_exp_log<const B: Word>(
        &self,
        base: &Repr<B>,
        signed_exp: &IBig,
    ) -> FpResult<FBig<R, B>> {
        let neg_base = base.sign() == Sign::Negative;
        // `(-base)^n = (-1)^n · base^n`, so the magnitude is always `|base|^n` and only the sign
        // depends on parity. `pow_exp_log` returns the positive magnitude; flip it (and the
        // overflow/underflow sign) when the base is negative and the exponent is odd.
        let odd = signed_exp.clone().into_parts().1.bit(0);
        let result_sign = if neg_base && odd {
            Sign::Negative
        } else {
            Sign::Positive
        };
        let base_mag = if neg_base {
            let (_, mag) = base.significand().clone().into_parts();
            Repr::<B>::new(IBig::from_parts(Sign::Positive, mag), base.exponent())
        } else {
            base.clone()
        };
        let exp_repr = Repr::new(signed_exp.clone(), 0);
        match self.pow_exp_log(&base_mag, &exp_repr, None) {
            Ok(rounded) => Ok(rounded.map(|v| if result_sign == Sign::Negative { -v } else { v })),
            Err(FpError::Overflow(_)) => Err(FpError::Overflow(result_sign)),
            Err(FpError::Underflow(_)) => Err(FpError::Underflow(result_sign)),
            Err(e) => Err(e),
        }
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

    // The reduction stays in the *base-`B`* logarithm: `exp(x) = B^s · exp(r/Bⁿ)^(Bⁿ)` with
    // `r = x − s·ln B`. A base-2 form (`r = x − s·ln 2`, powering `2ⁿ`) is deliberately **not**
    // used: for a non-power-of-two base the `2^s` scaling is a multi-digit value (≈ s·log₂B⁻¹·log₁₀2
    // digits) that would have to be materialized to multiply in, where the base-`B` form is an
    // exact O(1) exponent shift; and for a power-of-two base the two forms coincide (`B = 2` ⇒
    // `ln B = ln 2`, `B^s = 2^s`, `Bⁿ = 2ⁿ`), so this formulation is already the optimal one.
    // `pow_exact(Bⁿ)` is binary exponentiation (~n·log₂B squarings), tracked exactly by the Ball.

    fn exp_internal<const B: Word>(
        &self,
        x: &Repr<B>,
        minus_one: bool,
        mut cache: Option<&mut ConstCache>,
    ) -> FpResult<FBig<R, B>> {
        assert_finite(x);
        let input_sign = x.sign();

        if x.significand.is_zero() {
            // exp(±0) = 1; exp_m1(±0) = ±0 (IEEE 754 §9.2.1 preserves the sign of zero).
            // These exact results need no rounding, so handle them before the
            // limited-precision assertion: a precision-0 (unlimited) FBig such as the
            // one produced by `try_from(0.0)` must still compute exp/exp_m1 exactly.
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

        assert_limited_precision(self.precision);

        // For sufficiently negative x, exp(x) is below half an ulp of -1, so exp_m1(x) is -1 plus a
        // sub-ulp residual and its rounding is fully determined (Up/Zero -> the next representable
        // above -1; the other modes -> -1). The Ziv loop cannot certify that result: the
        // working-precision value collapses to exactly -1, and a directed rounding preimage is
        // one-sided, so the containment test never resolves and the loop runs to its retry cap.
        // Short-circuit to the same mode-aware endpoint used for the underflowed case. The cutoff is
        // exp(x) < half-ulp(-1): -1 sits on a power-of-B boundary, so the spacing just below it is
        // B^-p and the cutoff is |x| > p·ln(B) + ln 2. Compare the lower bound of log2|x| against an
        // upper bound of log2(threshold) so a borderline input still falls through to Ziv (which
        // converges there) rather than being mis-rounded.
        if minus_one && input_sign == Sign::Negative {
            let thresh = self.precision as f32 * B.log2_est() * core::f32::consts::LN_2
                + core::f32::consts::LN_2;
            if x.log2_bounds().0 > thresh.log2_bounds().1 {
                return Ok(self.exp_extreme_negative::<B>());
            }
        }

        // No-OOM magnitude gate: for an `x` whose exponent is near `isize::MAX`, the reduction
        // quotient `s = floor(x/ln B)` in `exp_compute` would allocate a GB-scale `IBig`, so reject
        // astronomical |x| here (via the cheap `log2_est` fast-skip) before the Ziv loop runs the
        // division. The probe inflates `ln B` with the same `⌈log_B|x|⌉ + 2` extra digits
        // `exp_compute` uses, so its `s` verdict matches the computation's. (`exp_compute` also
        // re-checks `s.try_into()` as a gray-zone backstop, propagating an error if the gate and
        // computation ever disagree — so a miss degrades to the directed endpoint, not a panic.)
        if x.log2_est().abs() > EXP_OVERFLOW_PROBE_LOG2 {
            let x_log2_ub = x.log2_bounds().1;
            let extra = if x_log2_ub > 0.0 {
                (x_log2_ub / B.log2_est()) as usize + 2
            } else {
                2
            };
            let probe = Context::<R>::new(self.precision + 64 + extra);
            let logb = probe.ln_base::<B>(reborrow_cache(&mut cache));
            let x_probe = FBig::new(probe.repr_round_ref(x).value(), probe);
            let s_probe = x_probe.div_rem_euclid(logb).0;
            if <isize as core::convert::TryFrom<IBig>>::try_from(s_probe).is_err() {
                // exp(huge +) overflows to +∞ (the directed endpoint handles every mode); exp(huge −)
                // is a positive value below the smallest representable, so it underflows (the directed
                // endpoint gives +0 / smallest-positive); exp_m1(huge −) ≈ −1 stays a finite value
                // just above −1 (the short-circuit above usually catches this first).
                return if input_sign == Sign::Positive {
                    Err(FpError::Overflow(Sign::Positive))
                } else if minus_one {
                    Ok(self.exp_extreme_negative::<B>())
                } else {
                    Err(FpError::Underflow(Sign::Positive))
                };
            }
        }

        // Correct rounding via the Ziv loop. Guards: log_B(p) for the series summation/squaring
        // rounding, plus `n` for the Bⁿ powering amplification — halved from the pre-Ziv `2n`,
        // since Ziv (not the guard count) now certifies correctness. `n ≈ √p` is derived from the
        // target precision and is constant across retries.
        let series_guard = self.base_guard_digits::<B>();
        let n = 1usize << (self.precision.bit_len() / 2);
        self.ziv(series_guard + n, |guard| {
            Ok(self
                .exp_compute::<B>(
                    x,
                    self.precision + guard,
                    minus_one,
                    n,
                    reborrow_cache(&mut cache),
                )?
                .to_value_radius::<R>())
        })
    }

    /// Directed-rounded `exp_m1(x)` when `x` is so large and negative that `exp(x)` has underflowed
    /// below the smallest representable FBig (the reduction quotient `s = floor(x/ln B)` overflows
    /// `isize`). `exp_m1(x) = exp(x) − 1` then lies in `(−1, −1 + B^{isize::MIN})` — pinned only up
    /// to a sub-representable residual, so the directed rounding mode picks the endpoint of the bin
    /// it falls in: a value just above `−1` rounds to `−1` under `Down`/`Away`/nearest, and to the
    /// next representable above `−1` under `Up`/`Zero` (both round the magnitude down toward 0).
    ///
    /// (`exp` itself of such an `x` is handled earlier — it returns `Err(Underflow)`, whose directed
    /// endpoint is the same `+0` / smallest-positive this used to produce inline.)
    ///
    /// `Round::round_low_part` decides the endpoint: fed `−1` with a positive sub-ulp residual, its
    /// `AddOne`/`NoOp` verdict is exactly the "round up to the next representable / stay" decision.
    /// (The literal significand arithmetic `round_low_part` would do is irrelevant here — only its
    /// directional verdict is used.)
    fn exp_extreme_negative<const B: Word>(&self) -> Rounded<FBig<R, B>> {
        // exp_m1(huge −): −1 + (sub-representable positive) ⇒ just above −1.
        match R::round_low_part(&IBig::NEG_ONE, Sign::Positive, || Ordering::Less) {
            AddOne => {
                // Next representable above −1 at this precision: −(B^p − 1) × B^(−p)
                // (the largest p-digit significand at exponent −p, e.g. p=1,B=2 → −0.5).
                let p = self.precision;
                let next_mag = Repr::<B>::BASE.pow(p) - UBig::ONE;
                let next = Repr::new(IBig::from_parts(Sign::Negative, next_mag), -(p as isize));
                Inexact(FBig::new(next, *self), AddOne)
            }
            // Carry the input context: `−FBig::ONE` is precision 0, which would make a downstream op
            // on the result panic via `assert_limited_precision(0)`.
            _ => Inexact(FBig::new(Repr::<B>::neg_one(), *self), NoOp),
        }
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

        // exp(huge −) is a positive value below the smallest representable -> Underflow at the
        // Context layer; the directed endpoint is +0 under HalfEven (nearest), smallest-positive
        // under Up.
        let neg = Repr::new(-(IBig::from(1) << 63), 0);
        assert_eq!(ctx.exp::<2>(&neg, None), Err(FpError::Underflow(Sign::Positive)));
        assert!(ctx.unwrap_fp(ctx.exp::<2>(&neg, None)).repr().is_pos_zero(), "HalfEven -> +0");
        let up = Context::<mode::Up>::new(53);
        let up_val = up.unwrap_fp(up.exp::<2>(&neg, None));
        assert_eq!(up_val.repr().significand(), &IBig::from(1), "Up -> smallest positive");
        assert_eq!(up_val.repr().exponent(), isize::MIN);

        // exp_m1(huge negative) -> -1 (a finite value, not an error)
        let m1 = ctx.exp_m1::<2>(&neg, None).unwrap().value();
        assert_eq!(m1, -FBig::<mode::HalfEven>::ONE);
    }

    // Directed rounding at the extreme-negative underflow: exp(x) for huge negative x is
    // a positive value below the smallest representable FBig; exp_m1(x) = exp(x) − 1 is just above
    // −1. The blanket +0 / Exact(−1) saturation was mode-blind — it returned +0 under Up and
    // Exact(−1) under Up for exp_m1, violating Up ≥ Down.
    #[test]
    fn test_exp_extreme_negative_directed() {
        // x = -2^63, precision 1 (exactly representable).
        let up = FBig::<mode::Up>::from_parts(-IBig::ONE, 63);
        let down = FBig::<mode::Down>::from_parts(-IBig::ONE, 63);
        assert_eq!(up.precision(), 1);

        // exp(-2^63): Up → smallest positive (1 × 2^{isize::MIN}); Down → +0.
        let up_exp = up.exp();
        let down_exp = down.exp();
        assert_eq!(up_exp.repr().significand(), &IBig::from(1));
        assert_eq!(up_exp.repr().exponent(), isize::MIN);
        assert!(down_exp.repr().is_pos_zero(), "Down exp(huge −) is +0");
        assert!(up_exp > down_exp, "Up(exp) > Down(exp)");

        // exp_m1(-2^63) ∈ (-1, -1/2): at precision 1, Up → -1/2 (next above -1); Down → -1.
        let up_m1 = up
            .context()
            .exp_m1(up.repr(), None)
            .expect("finite exp_m1 input");
        let down_m1 = down
            .context()
            .exp_m1(down.repr(), None)
            .expect("finite exp_m1 input");
        let expected_up = FBig::<mode::Up>::from_parts(-IBig::ONE, -1); // -1/2
        assert!(!matches!(up_m1, Exact(_)), "exp_m1(huge −) is inexact under Up");
        assert!(!matches!(down_m1, Exact(_)), "exp_m1(huge −) is inexact under Down too");
        assert_eq!(up_m1.value().repr(), expected_up.repr());
        assert_eq!(down_m1.value(), -FBig::<mode::Down>::ONE);
    }

    // exp_m1 of a large negative x where the reduction quotient still fits isize (so the overflow
    // short-circuit doesn't fire) but exp(x) is below the result precision. The true value is
    // -1 + (sub-ulp residual), so directed/nearest rounding is fully determined; the directed
    // preimage being one-sided means Ziv cannot certify it, so it is short-circuited to the
    // mode-aware endpoint. Verifies the result matches the directed saturation at several
    // magnitudes/precisions (and that it does not regress to a many-retry Ziv loop).
    #[test]
    fn test_exp_m1_large_negative_directed_saturation() {
        for &(e, p) in &[(50i32, 2usize), (50, 53), (100, 53), (1000, 53), (100, 128)] {
            let up = FBig::<mode::Up, 2>::from_parts(IBig::from(-1), e as isize)
                .with_precision(p)
                .value()
                .exp_m1();
            let down = FBig::<mode::Down, 2>::from_parts(IBig::from(-1), e as isize)
                .with_precision(p)
                .value()
                .exp_m1();
            // Up -> next representable above -1 = -(2^p - 1) * 2^-p; Down -> -1.
            let next_up_mag = (IBig::from(1) << p) - IBig::from(1);
            let expected_up = FBig::<mode::Up, 2>::from_parts(-next_up_mag, -(p as isize));
            assert_eq!(up.repr(), expected_up.repr(), "Up exp_m1(-2^{e}) p={p}");
            assert_eq!(
                down.repr(),
                FBig::<mode::Down, 2>::NEG_ONE.repr(),
                "Down exp_m1(-2^{e}) p={p}"
            );
            // The endpoint must carry the input precision: a precision-0 result would make a
            // downstream op panic via `assert_limited_precision(0)`.
            assert_eq!(up.precision(), p, "Up exp_m1(-2^{e}) p={p} lost precision");
            assert_eq!(down.precision(), p, "Down exp_m1(-2^{e}) p={p} lost precision");
            assert!(up > down, "Up > Down for exp_m1(-2^{e}) p={p}");
        }
    }

    // exp(huge −) saturates to +0 (or the smallest positive under Up/Away). The endpoint must
    // carry the input precision too — the precision-0 `FBig::ZERO` previously returned here
    // tripped `assert_limited_precision(0)` on a downstream op.
    #[test]
    fn test_exp_extreme_negative_endpoint_precision() {
        for &(e, p) in &[(50i32, 53usize), (100, 53), (1000, 53), (100, 128)] {
            let up = FBig::<mode::Up, 2>::from_parts(-IBig::ONE, e as isize)
                .with_precision(p)
                .value()
                .exp();
            let down = FBig::<mode::Down, 2>::from_parts(-IBig::ONE, e as isize)
                .with_precision(p)
                .value()
                .exp();
            assert_eq!(up.precision(), p, "Up exp(-2^{e}) p={p} lost precision");
            assert_eq!(down.precision(), p, "Down exp(-2^{e}) p={p} lost precision");
            // A downstream op must not panic on the saturated endpoint.
            let _ = down.sqrt();
            assert!(up > down, "Up > Down for exp(-2^{e}) p={p}");
        }
    }

    // Directed underflow through `powi`/`powf` must match `exp` (pow(x,y) = exp(y·ln x)) so the
    // `Up ≥ Down` invariant holds across them. Previously `pow` saturated to signed zero under
    // every mode, so `Up(pow(10, huge−))` was `+0` while `Up(exp(huge−·ln 10))` was the smallest
    // positive — the two disagreed on the same mathematical value.
    #[test]
    fn test_pow_directed_underflow() {
        let p = 53;
        // |exp| · log2(10) > isize::MAX ⇒ the result exponent falls below isize::MIN (underflow).
        let huge_neg = IBig::from(-9_000_000_000_000_000_000_i64);

        // powi(10, huge−): positive tiny result. Up → smallest positive, Down/Zero → +0.
        let up = FBig::<mode::Up, 2>::from_parts(IBig::from(10), 0)
            .with_precision(p)
            .value()
            .powi(huge_neg.clone());
        let down = FBig::<mode::Down, 2>::from_parts(IBig::from(10), 0)
            .with_precision(p)
            .value()
            .powi(huge_neg.clone());
        let zero = FBig::<mode::Zero, 2>::from_parts(IBig::from(10), 0)
            .with_precision(p)
            .value()
            .powi(huge_neg.clone());
        assert_eq!(up.repr().significand(), &IBig::from(1));
        assert_eq!(up.repr().exponent(), isize::MIN);
        assert!(down.repr().is_pos_zero());
        assert!(zero.repr().is_pos_zero());
        assert!(up > down);

        // powi(-10, huge odd −): negative tiny result. Up → -0, Down → smallest negative.
        let odd = IBig::from(-9_000_000_000_000_000_001_i64);
        let nup = FBig::<mode::Up, 2>::from_parts(IBig::from(-10), 0)
            .with_precision(p)
            .value()
            .powi(odd.clone());
        let ndown = FBig::<mode::Down, 2>::from_parts(IBig::from(-10), 0)
            .with_precision(p)
            .value()
            .powi(odd.clone());
        assert!(nup.repr().is_neg_zero());
        assert!(ndown.repr().sign() == Sign::Negative && ndown.repr().exponent() == isize::MIN);
        assert!(nup > ndown);

        // powf(2, y) with |y| > isize::MAX underflows and agrees with exp(y·ln 2).
        let ymag = IBig::from(-10_000_000_000_000_000_000_i128);
        let y_up = FBig::<mode::Up, 2>::from_parts(ymag.clone(), 0)
            .with_precision(p)
            .value();
        let y_down = FBig::<mode::Down, 2>::from_parts(ymag.clone(), 0)
            .with_precision(p)
            .value();
        let pf_up = FBig::<mode::Up, 2>::from_parts(IBig::from(2), 0)
            .with_precision(p)
            .value()
            .powf(&y_up);
        let pf_down = FBig::<mode::Down, 2>::from_parts(IBig::from(2), 0)
            .with_precision(p)
            .value()
            .powf(&y_down);
        assert_eq!(pf_up.repr().significand(), &IBig::from(1));
        assert_eq!(pf_up.repr().exponent(), isize::MIN);
        assert!(pf_down.repr().is_pos_zero());
        // Same value as exp(y·ln 2) under the same mode.
        let ln2 = FBig::<mode::Up, 2>::from_parts(IBig::from(2), 0)
            .with_precision(p)
            .value()
            .ln();
        let exp_up = (&y_up * &ln2)
            .with_precision(p)
            .value()
            .with_rounding::<mode::Up>()
            .exp();
        assert_eq!(exp_up.repr(), pf_up.repr(), "powf and exp disagree on directed underflow");
    }

    // Directed overflow through `exp`/`powi`: outward modes reach ±∞, inward modes (toward-zero,
    // opposite-infinity) saturate to the largest finite `(Bᵖ−1) × B^{isize::MAX}` — the all-ones
    // significand at the output precision, mirroring MPFR's `mpfr_setmax`. (Hyperbolic `sinh`/`cosh`
    // overflow under nearest is unchanged — still ±∞.)
    #[test]
    fn test_directed_overflow() {
        let p = 53usize;
        let max_sig = (IBig::ONE << p) - IBig::ONE;

        // exp(2^63): overflows. Up -> +∞, Zero/Down -> largest finite.
        let huge = FBig::<mode::HalfEven, 2>::from_parts(IBig::ONE << 63, 0)
            .with_precision(p)
            .value();
        let up = huge.clone().with_rounding::<mode::Up>().exp();
        let zero = huge.clone().with_rounding::<mode::Zero>().exp();
        let down = huge.clone().with_rounding::<mode::Down>().exp();
        assert!(up.repr().is_infinite() && up.repr().sign() == Sign::Positive, "Up -> +∞");
        assert_eq!(zero.repr().significand(), &max_sig, "Zero -> largest finite significand");
        assert_eq!(zero.repr().exponent(), isize::MAX, "Zero -> largest finite exponent");
        assert_eq!(down.repr().significand(), &max_sig, "Down -> largest finite");
        assert_eq!(down.repr().exponent(), isize::MAX);
        assert!(up > zero, "Up(+∞) > largest finite");

        // powi(-2, huge odd): negative overflow. Down -> -∞, Up -> largest finite negative.
        let odd = IBig::from(10_000_000_000_000_000_001_i128);
        let n_up = FBig::<mode::Up, 2>::from_parts(IBig::from(-2), 0)
            .with_precision(p)
            .value()
            .powi(odd.clone());
        let n_down = FBig::<mode::Down, 2>::from_parts(IBig::from(-2), 0)
            .with_precision(p)
            .value()
            .powi(odd.clone());
        assert!(
            n_down.repr().is_infinite() && n_down.repr().sign() == Sign::Negative,
            "Down -> -∞"
        );
        assert_eq!(
            n_up.repr().significand(),
            &(-max_sig.clone()),
            "Up -> largest finite negative significand"
        );
        assert_eq!(n_up.repr().exponent(), isize::MAX);
        assert_eq!(n_up.repr().sign(), Sign::Negative);
    }

    // Overflow at unlimited precision panics: the largest finite is undefined (no precision cap),
    // so the directed endpoint can't be formed. Reached via `powi` with a positive exponent, which
    // skips the limited-precision assertion and lets the overflow reach `unwrap_fp`.
    #[test]
    #[should_panic(expected = "precision cannot be 0")]
    fn test_overflow_at_unlimited_precision_panics() {
        let base =
            FBig::<mode::Zero, 2>::from_repr(Repr::<2>::new(IBig::from(2), 0), Context::new(0));
        let _ = base.powi(IBig::from(10_000_000_000_000_000_000_i128));
    }

    // Exponents past `i64` (bit length > `MAX_POWI_CHAIN_BITS`) can't use the squaring chain: for a
    // base near 1 the magnitude overflows the finite range mid-computation, and the chain's growing
    // working precision exhausts memory. The `exp(y·ln x)` fallback computes these without scaling
    // the working precision with the exponent's bit length, returning the correct directed endpoint.
    #[test]
    fn test_powi_huge_exponent_fallback() {
        let p = 53usize;
        let max_sig = (IBig::ONE << p) - IBig::ONE;
        // base = 1 + 2^-52 (just above 1); exponent 2^200 has bit length 201.
        let near1 = 0x3ff0_0000_0000_0001u64;
        let huge_pos = IBig::ONE << 200usize;
        let huge_neg = -(IBig::ONE << 200usize);

        // positive base, positive huge exp -> positive overflow. Up -> +∞, Down -> largest finite.
        let up = FBig::<mode::Up, 2>::try_from(f64::from_bits(near1))
            .unwrap()
            .with_precision(p)
            .value()
            .powi(huge_pos.clone());
        let down = FBig::<mode::Down, 2>::try_from(f64::from_bits(near1))
            .unwrap()
            .with_precision(p)
            .value()
            .powi(huge_pos.clone());
        let he = FBig::<mode::HalfEven, 2>::try_from(f64::from_bits(near1))
            .unwrap()
            .with_precision(p)
            .value()
            .powi(huge_pos.clone());
        assert!(up.repr().is_infinite() && up.repr().sign() == Sign::Positive, "Up -> +∞");
        assert_eq!(down.repr().significand(), &max_sig, "Down -> largest finite");
        assert_eq!(down.repr().exponent(), isize::MAX);
        assert!(he.repr().is_infinite() && he.repr().sign() == Sign::Positive, "nearest -> +∞");
        assert!(up > down);

        // positive base, negative huge exp -> underflow. Up -> smallest positive, Down -> +0.
        let u_up = FBig::<mode::Up, 2>::try_from(f64::from_bits(near1))
            .unwrap()
            .with_precision(p)
            .value()
            .powi(huge_neg.clone());
        let u_down = FBig::<mode::Down, 2>::try_from(f64::from_bits(near1))
            .unwrap()
            .with_precision(p)
            .value()
            .powi(huge_neg.clone());
        assert_eq!(u_up.repr().significand(), &IBig::from(1));
        assert_eq!(u_up.repr().exponent(), isize::MIN);
        assert!(u_down.repr().is_pos_zero());
        assert!(u_up > u_down);

        // negative base near -1, huge exponent: sign follows the exponent's parity. The magnitude
        // overflows, so nearest reaches ±∞ with the parity sign.
        let neg_near1 = 0xbff0_0000_0000_0001u64;
        let even = huge_pos.clone();
        let odd = (IBig::ONE << 200usize) + IBig::ONE;
        let he_even = FBig::<mode::HalfEven, 2>::try_from(f64::from_bits(neg_near1))
            .unwrap()
            .with_precision(p)
            .value()
            .powi(even);
        let he_odd = FBig::<mode::HalfEven, 2>::try_from(f64::from_bits(neg_near1))
            .unwrap()
            .with_precision(p)
            .value()
            .powi(odd.clone());
        assert!(
            he_even.repr().is_infinite() && he_even.repr().sign() == Sign::Positive,
            "even exponent -> +∞"
        );
        assert!(
            he_odd.repr().is_infinite() && he_odd.repr().sign() == Sign::Negative,
            "odd exponent -> -∞"
        );
        // Down of a negative overflow rounds toward -∞.
        let down_odd = FBig::<mode::Down, 2>::try_from(f64::from_bits(neg_near1))
            .unwrap()
            .with_precision(p)
            .value()
            .powi(odd);
        assert!(
            down_odd.repr().is_infinite() && down_odd.repr().sign() == Sign::Negative,
            "Down(negative overflow) -> -∞"
        );
    }

    // Directed `powi` on a tiny base with a large negative exponent: base ≈ -3.6e-5, exponent
    // -2^31. The result (≈2^3.17e10) is representable on 64-bit `isize` (MAX ≈ 9.2e18), so this
    // exercises the squaring chain (bit length 32 ≤ MAX_POWI_CHAIN_BITS) and must return a finite
    // value promptly — a regression guard for the chain path. On 32-bit `isize` (MAX ≈ 2.1e9) the
    // same result overflows and is short-circuited by the range guard, so the guard is 64-bit-only.
    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_powi_reproducer_small_base_representable() {
        let base_bits = 0xbf02e3ff24ffff1fu64;
        let exp = IBig::from(-(1i64 << 31));
        for p in [20usize, 50, 100, 500] {
            let v = FBig::<mode::HalfEven, 2>::try_from(f64::from_bits(base_bits))
                .unwrap()
                .with_precision(p)
                .value()
                .powi(exp.clone());
            assert!(!v.repr().is_infinite(), "finite at p={p}");
            assert_eq!(v.repr().sign(), Sign::Positive, "even exponent -> positive at p={p}");
            // magnitude ≈ 2^3.17e10, far from 1 — the chain computed the huge representable value.
            assert!(v.repr().exponent() > 1_000_000_000, "huge magnitude at p={p}");
        }
    }

    // `powi(2, exp)` for `exp` just below `isize::MAX`: the result `2^exp` is representable (its
    // exponent is `exp ≤ isize::MAX`), so the range guard correctly does not short-circuit it and
    // the squaring chain computes it. The Ziv containment test then compares Reprs at that extreme
    // magnitude — `repr_cmp_same_base` used to do `exponent + digits` in plain `isize`, which
    // overflows near the ceiling and aborted (the raw-backend `backend_float_extremes` crash).
    #[test]
    fn test_powi_power_of_two_near_ceiling() {
        for offset in [1isize, 2, 60, 100, 1000] {
            let exp = IBig::from(isize::MAX - offset);
            let up = FBig::<mode::Up, 2>::from_parts(IBig::ONE, 1)
                .with_precision(53)
                .value()
                .powi(exp.clone());
            let down = FBig::<mode::Down, 2>::from_parts(IBig::ONE, 1)
                .with_precision(53)
                .value()
                .powi(exp.clone());
            // 2^exp is an exact power of two: significand 1 at exponent exp, identical under Up/Down.
            assert!(!up.repr().is_infinite(), "finite for offset {offset}");
            assert_eq!(up.repr().significand(), &IBig::from(1), "sig 1 for offset {offset}");
            assert_eq!(up.repr().exponent(), isize::MAX - offset, "exponent for offset {offset}");
            assert_eq!(down.repr().exponent(), isize::MAX - offset);
            assert!(!up.repr().is_infinite() && !down.repr().is_infinite());
        }

        // Symmetric floor: `powi(2, -exp)` for `exp` just below `isize::MAX` gives `2^-exp`, whose
        // exponent sits just above `isize::MIN`. The containment test compares Reprs there too — the
        // saturating `cmp` shortcuts cover the floor as well as the ceiling (this is the negative-
        // exponent half of the range-handling TODO).
        for offset in [1isize, 2, 60, 100, 1000] {
            let exp = IBig::from(-(isize::MAX - offset));
            let up = FBig::<mode::Up, 2>::from_parts(IBig::ONE, 1)
                .with_precision(53)
                .value()
                .powi(exp.clone());
            let down = FBig::<mode::Down, 2>::from_parts(IBig::ONE, 1)
                .with_precision(53)
                .value()
                .powi(exp);
            assert!(!up.repr().is_infinite(), "finite for floor offset {offset}");
            assert_eq!(up.repr().significand(), &IBig::from(1), "sig 1 for floor offset {offset}");
            assert_eq!(up.repr().exponent(), -(isize::MAX - offset));
            assert!(!down.repr().is_infinite());
        }
    }

    // `powi(2, isize::MAX)`: the result `2^isize::MAX` normalizes to significand 1 at the `+inf`
    // sentinel exponent, so it is genuine overflow. This used to panic — the squaring chain absorbed
    // the overflow into an infinity and the Ziv closure then choked on `res.ulp()` of an infinity.
    // Now the chain propagates the range error and `powi` returns the directed endpoint for every
    // mode. `isize::MAX` is the sentinel on both pointer widths, so this test is arch-independent.
    #[test]
    fn test_powi_exact_ceiling_overflow_directed() {
        let max_sig = |p: usize| (IBig::ONE << p) - IBig::ONE;
        let exp = IBig::from(isize::MAX);
        for p in [20usize, 50, 100, 500] {
            // Context layer: genuine overflow, positive sign.
            let ctx = Context::<mode::HalfEven>::new(p);
            let base = Repr::<2>::new(IBig::from(2), 0);
            assert_eq!(
                ctx.powi::<2>(&base, exp.clone()),
                Err(FpError::Overflow(Sign::Positive)),
                "Overflow at p={p}"
            );

            // Convenience layer: directed endpoints (outward → +∞, inward → largest finite).
            let he = FBig::<mode::HalfEven, 2>::from_parts(IBig::ONE, 1)
                .with_precision(p)
                .value()
                .powi(exp.clone());
            let up = FBig::<mode::Up, 2>::from_parts(IBig::ONE, 1)
                .with_precision(p)
                .value()
                .powi(exp.clone());
            let down = FBig::<mode::Down, 2>::from_parts(IBig::ONE, 1)
                .with_precision(p)
                .value()
                .powi(exp.clone());
            let zero = FBig::<mode::Zero, 2>::from_parts(IBig::ONE, 1)
                .with_precision(p)
                .value()
                .powi(exp.clone());
            assert!(
                he.repr().is_infinite() && he.repr().sign() == Sign::Positive,
                "HalfEven -> +∞ at p={p}"
            );
            assert!(
                up.repr().is_infinite() && up.repr().sign() == Sign::Positive,
                "Up -> +∞ at p={p}"
            );
            assert_eq!(down.repr().significand(), &max_sig(p), "Down -> largest finite at p={p}");
            assert_eq!(down.repr().exponent(), isize::MAX, "Down exponent at p={p}");
            assert_eq!(zero.repr().significand(), &max_sig(p), "Zero -> largest finite at p={p}");
            assert!(up > down, "Up >= Down at p={p}");
        }
    }

    // Underflow propagation through the chain: a tiny base `2^(isize::MIN/2)` squared reaches the
    // `-inf` sentinel exponent (`isize::MIN`) on the first squaring, so the chain underflows and
    // `powi` routes it to the directed endpoint instead of panicking. (Note `powi(2, -isize::MAX)` is
    // *not* underflow — `2^-isize::MAX` sits at exponent `isize::MIN+1`, still representable — so this
    // case uses a base whose square genuinely crosses the floor.) `isize::MIN/2` scales with the
    // pointer width, keeping the test arch-independent.
    #[test]
    fn test_powi_chain_underflow_propagates() {
        let half_floor = isize::MIN / 2;
        let ctx = Context::<mode::HalfEven>::new(53);
        let tiny = Repr::<2>::new(IBig::ONE, half_floor);
        assert_eq!(
            ctx.powi::<2>(&tiny, IBig::from(2)),
            Err(FpError::Underflow(Sign::Positive)),
            "base^2 underflows to the floor sentinel"
        );

        // Convenience layer: directed endpoints (outward → smallest positive, inward/nearest → +0).
        let he = FBig::<mode::HalfEven, 2>::from_parts(IBig::ONE, half_floor)
            .with_precision(53)
            .value()
            .powi(IBig::from(2));
        let up = FBig::<mode::Up, 2>::from_parts(IBig::ONE, half_floor)
            .with_precision(53)
            .value()
            .powi(IBig::from(2));
        let down = FBig::<mode::Down, 2>::from_parts(IBig::ONE, half_floor)
            .with_precision(53)
            .value()
            .powi(IBig::from(2));
        assert!(he.repr().is_pos_zero(), "HalfEven -> +0");
        assert_eq!(up.repr().significand(), &IBig::from(1), "Up -> smallest positive");
        assert_eq!(up.repr().exponent(), isize::MIN, "Up exponent");
        assert!(down.repr().is_pos_zero(), "Down -> +0");
        assert!(up > down, "Up >= Down");
    }

    // Significand != 1 and negative-base sign handling at the ceiling: the magnitude overflows just
    // the same and must propagate (not panic), with the overflow sign following base sign × parity.
    #[test]
    fn test_powi_significand_nonunit_ceiling() {
        let ctx = Context::<mode::HalfEven>::new(53);
        // base 3: 3^isize::MAX overflows with a positive sign.
        let base3 = Repr::<2>::new(IBig::from(3), 0);
        assert_eq!(
            ctx.powi::<2>(&base3, IBig::from(isize::MAX)),
            Err(FpError::Overflow(Sign::Positive)),
            "3^MAX overflows positive"
        );
        // base -2, odd exponent isize::MAX: (-2)^odd is negative -> Overflow(Negative).
        let neg2 = Repr::<2>::new(IBig::from(-2), 0);
        assert_eq!(
            ctx.powi::<2>(&neg2, IBig::from(isize::MAX)),
            Err(FpError::Overflow(Sign::Negative)),
            "(-2)^MAX overflows negative"
        );
    }

    // Regression guard: a negative base with an *even* exponent just below the ceiling is
    // representable (positive, magnitude 2^(isize::MAX-1)) and must still compute to a finite value
    // — the overflow propagation must not over-broaden and treat near-ceiling representable results
    // as overflow. `isize::MAX - 1` is even on both 32- and 64-bit.
    #[test]
    fn test_powi_negative_base_even_exp_near_ceiling() {
        let exp = IBig::from(isize::MAX - 1);
        for p in [20usize, 50, 100, 500] {
            let v = FBig::<mode::HalfEven, 2>::try_from(-2.0f64)
                .unwrap()
                .with_precision(p)
                .value()
                .powi(exp.clone());
            assert!(!v.repr().is_infinite(), "finite at p={p}");
            assert_eq!(v.repr().sign(), Sign::Positive, "even exponent -> positive at p={p}");
            assert_eq!(v.repr().significand(), &IBig::from(1), "sig 1 at p={p}");
            assert_eq!(v.repr().exponent(), isize::MAX - 1, "exponent at p={p}");
        }
    }

    // A sharp OOM regression needs an exponent gap large enough that 2^gap exceeds any
    // memory (gap ≳ 1e11), yet with floor(x/ln2) still fitting isize so the overflow
    // branch is not taken. That window only exists where isize is 64-bit: on 32-bit,
    // isize tops out at ~2.1e9 — below any OOM-inducing gap — so the overflow branch
    // always intervenes first. The fix itself (log2_bounds in round_fract) is
    // arch-independent; only this dedicated sharp test is 64-bit-only.
    #[test]
    fn test_exact_results_on_unlimited_precision() {
        // Regression test: values carrying precision 0 (unlimited) — produced by
        // `try_from(0.0)` and the `FBig::ONE`/`ZERO` constants — must still compute
        // their exact-result special cases instead of panicking in
        // assert_limited_precision before reaching the shortcut.
        type F = FBig<mode::HalfEven, 2>;

        let zero = F::try_from(0.0_f64).unwrap();
        assert_eq!(zero.exp(), F::ONE);
        assert_eq!(zero.exp_m1(), F::ZERO);
        assert_eq!(zero.sqrt(), F::ZERO);
        assert_eq!(zero.ln_1p(), F::ZERO);

        // -0.0 preserves its sign through exp_m1 and sqrt.
        let neg_zero = F::try_from(-0.0_f64).unwrap();
        assert!(neg_zero.exp_m1().repr().is_neg_zero());
        assert!(neg_zero.sqrt().repr().is_neg_zero());

        // FBig::ONE carries unlimited precision; ln(1) = 0 is exact.
        assert_eq!(F::ONE.ln(), F::ZERO);
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

    #[test]
    fn exp_ball_bounds_propagated_input_error() {
        // The input ball's error must be amplified by |exp| in the result radius: n·ulp(exp)
        // has to cover n_x·ulp(x)·|exp(x)|. Regression for the missing `sig_r` factor in
        // `exp_ball`'s inflate term, which under-bound the radius by ~sig_r (≈ B^(p−1)) and let
        // Ziv certify an interval that did not contain the true value.
        type F = FBig<mode::HalfEven, 10>;
        let ctx = Context::<mode::HalfEven>::new(10);
        // mid = 0.5 at precision 10 (ulp = 1e-10), n = 10 ⇒ true arg = 0.5000000010.
        let mid = F::from_parts(IBig::from(5000000000i64), -10)
            .with_precision(10)
            .value();
        let x = Ball::<10>::with_error(mid, IBig::from(10));
        let r = ctx.exp_ball::<10>(&x, None).unwrap();
        let true_arg = F::from_parts(IBig::from(5000000010i64), -10)
            .with_precision(0)
            .value();
        let exp_true = true_arg
            .with_precision(60)
            .value()
            .exp()
            .with_precision(0)
            .value();
        let diff = (r.mid.clone().with_precision(0).value() - exp_true).abs();
        let bound = F::from(r.n.clone()) * r.mid.ulp().with_precision(0).value();
        assert!(
            diff <= bound,
            "exp_ball: |mid − true| = {diff} > n·ulp = {bound} (n = {}, missing sig_r?)",
            r.n
        );
    }

    #[test]
    fn exp_generic_base_matches_oracle() {
        // exp at a generic (uncached) base exercises `ln_base_ball`'s mechanical-radius path for
        // ln(B) — the hard-coded `8`-ulp bound would under-bind a generic base's atanh-series
        // error (which is ~series-terms ulps), silently unsounding the reduction.
        type F3 = FBig<mode::HalfEven, 3>;
        for x in [1i64, 2, 3, 5] {
            let x = F3::from_parts(IBig::from(x), 0);
            let ctx = Context::<mode::HalfEven>::new(30);
            let got = ctx.exp::<3>(&x.repr, None).unwrap().value();
            let oracle = Context::<mode::HalfEven>::new(90)
                .exp::<3>(&x.repr, None)
                .unwrap()
                .value();
            let want = ctx.repr_round_ref(&oracle.repr).value();
            assert_eq!(got.repr, want, "exp({x:?}) at base 3 p=30: got {got:?}, want {want:?}");
        }
    }
}
