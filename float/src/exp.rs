use core::convert::TryInto;

use crate::{
    error::{assert_finite, assert_limited_precision, FpError, FpResult},
    fbig::FBig,
    math::cache::{reborrow_cache, ConstCache},
    repr::{Context, Repr, Word},
    round::{ErrorBounds, Round},
};
use dashu_base::{Abs, AbsOrd, Approximation::*, BitTest, DivRemEuclid, EstimatedLog2, Sign};
use dashu_int::{IBig, UBig};

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
    /// Left-to-right binary exponentiation of `start` to the power `n` (`n ≥ 2`) at this context's
    /// precision — the shared squaring kernel.
    ///
    /// Each `sqr`/`mul` is correctly rounded, but their rounding flags are folded away (`.value()`)
    /// and no containment test is applied, so the result is only *near*-correct: repeated squaring
    /// compounds the relative error (it roughly doubles per step), so after `n.bit_len()` squarings
    /// the error is on the order of `2^nlen · ulp`. The public [`powi`](Context::powi) retries this
    /// kernel inside a Ziv loop to certify the rounding; `exp_compute` also uses it for its internal
    /// `Bⁿ` powering (where the outer `exp` Ziv loop absorbs the error).
    ///
    /// Returns the value together with an `exact` flag that is `true` only when **every** squaring
    /// and multiplication rounded `Exact` (so the returned value is the mathematically exact
    /// `startⁿ`). The Ziv caller uses this to report a zero radius for exact results — under
    /// directed rounding modes an exactly-representable result sits on a one-sided rounding
    /// boundary, which a nonzero radius can never certify.
    pub(crate) fn powi_chain<const B: Word>(
        &self,
        start: &Repr<B>,
        n: &UBig,
    ) -> (FBig<R, B>, bool) {
        let nlen = n.bit_len();
        debug_assert!(nlen >= 2, "powi_chain requires n >= 2");
        let mut p = nlen - 2;
        let first = self.sqr(start);
        let mut exact = matches!(first, Ok(Exact(_)));
        let mut res = self.unwrap_fp(first);
        loop {
            if n.bit(p) {
                let m = self.mul(res.repr(), start);
                exact = exact && matches!(m, Ok(Exact(_)));
                res = self.unwrap_fp(m);
            }
            if p == 0 {
                break;
            }
            p -= 1;
            let s = self.sqr(res.repr());
            exact = exact && matches!(s, Ok(Exact(_)));
            res = self.unwrap_fp(s);
        }
        (res, exact)
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
            //
            // The squaring chain compounds the relative error (it doubles per step), so it is run
            // at an inflated precision and rounded back to `work_precision` — near-correct, which
            // is what the `+ ulp(v)` term above accounts for.
            let bn: UBig = Repr::<B>::BASE.pow(n);
            let chain_ctx =
                Context::<R>::new(work_precision + bn.bit_len() + work_precision.bit_len());
            let (v_pow, _) = chain_ctx.powi_chain(sum.repr(), &bn);
            let v = v_pow.with_precision(work_precision).value();
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
        // TODO: range handling has three known limitations at the exponent extremes:
        // (1) the overflow guard below estimates the result magnitude with an f64 and misclassifies
        // representable boundaries near 2^63; (2) genuine Overflow/Underflow is unwrapped mode-blindly
        // to ±inf / signed zero; (3) the negative-exponent reciprocal path can panic. None affects
        // ordinary inputs; fixing requires mode-aware range saturation.
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
        let base_log2 = base.log2_est() as f64;
        let threshold = (isize::MAX as f64) * (B.log2_est() as f64);
        let result_log2 = match i64::try_from(&n).ok() {
            Some(e) => {
                let signed = if negative { -(e as f64) } else { e as f64 };
                signed * base_log2
            }
            None => {
                // |n| doesn't fit i64: the magnitude is unbounded. |base| ≈ 1 (estimate 0) gives
                // exactly ±1 regardless of the huge exponent; otherwise it over- or underflows.
                if base_log2 == 0.0 {
                    let repr = if base.sign() == Sign::Negative && odd {
                        Repr::<B>::neg_one()
                    } else {
                        Repr::<B>::one()
                    };
                    return Ok(Exact(FBig::new(repr, *self)));
                }
                let over = (!negative && base_log2 > 0.0) || (negative && base_log2 < 0.0);
                let sign = if base.sign() == Sign::Negative && odd {
                    Sign::Negative
                } else {
                    Sign::Positive
                };
                return Err(if over {
                    FpError::Overflow(sign)
                } else {
                    FpError::Underflow(sign)
                });
            }
        };
        if result_log2 > threshold || result_log2 < -threshold {
            let sign = if base.sign() == Sign::Negative && odd {
                Sign::Negative
            } else {
                Sign::Positive
            };
            return Err(if result_log2 > threshold {
                FpError::Overflow(sign)
            } else {
                FpError::Underflow(sign)
            });
        }

        let nlen = n.bit_len();
        let initial_guard = nlen + self.base_guard_digits::<B>() + 2;
        Ok(self.ziv(initial_guard, |guard| {
            let pw = self.precision + guard;
            let work = Context::<R>::new(pw);
            // start from base (positive exponent, always exact) or its working-precision
            // reciprocal (negative exponent, exact only when 1/base is exactly representable).
            let (start, start_exact) = if negative {
                let d = work.div(&Repr::one(), base);
                let exact = matches!(d, Ok(Exact(_)));
                (work.unwrap_fp(d).repr().clone(), exact)
            } else {
                (base.clone(), true)
            };
            let (res, chain_exact) = work.powi_chain(&start, &n);
            // When the whole computation is exact (start exact + no squaring rounded), `res` is the
            // exact value and the true error is 0 — report a zero radius. This is required under
            // directed rounding modes, where an exactly-representable result lies on a one-sided
            // rounding boundary that no nonzero radius can fit inside (the Ziv loop would retry
            // forever). Otherwise the squaring compounds the error ~`2^nlen · ulp_w`.
            let radius = if pw == 0 || (start_exact && chain_exact) {
                FBig::ZERO
            } else {
                res.ulp().with_precision(0).value() << (nlen as isize + 1)
            };
            (res, radius)
        }))
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

        let initial_guard = self.base_guard_digits::<B>() + 10;
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
        let series_guard = self.base_guard_digits::<B>();
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
}
