use dashu_base::{
    utils::{next_down, next_up},
    AbsOrd,
    Approximation::*,
    EstimatedLog2, PowerOfTwo, Sign, UnsignedAbs,
};
use dashu_int::IBig;

use crate::{
    ball::Ball,
    error::{assert_finite, assert_limited_precision, FpError, FpResult},
    fbig::FBig,
    math::cache::{reborrow_cache, ConstCache},
    repr::{Context, Repr, Word},
    round::{mode, ErrorBounds, Round},
};
use core::cmp::Ordering;

impl<const B: Word> EstimatedLog2 for Repr<B> {
    // currently a Word has at most 64 bits, so log2() < f32::MAX
    fn log2_bounds(&self) -> (f32, f32) {
        if self.significand.is_zero() {
            return (f32::NEG_INFINITY, f32::NEG_INFINITY);
        }

        // log(s*B^e) = log(s) + e*log(B)
        let (logs_lb, logs_ub) = self.significand.log2_bounds();
        let (logb_lb, logb_ub) = if B.is_power_of_two() {
            let log = B.trailing_zeros() as f32;
            (log, log)
        } else {
            B.log2_bounds()
        };
        let e = self.exponent as f32;
        let (lb, ub) = if self.exponent >= 0 {
            (logs_lb + e * logb_lb, logs_ub + e * logb_ub)
        } else {
            (logs_lb + e * logb_ub, logs_ub + e * logb_lb)
        };
        (next_down(lb), next_up(ub))
    }

    fn log2_est(&self) -> f32 {
        let logs = self.significand.log2_est();
        let logb = if B.is_power_of_two() {
            B.trailing_zeros() as f32
        } else {
            B.log2_est()
        };
        logs + self.exponent as f32 * logb
    }
}

impl<R: Round, const B: Word> EstimatedLog2 for FBig<R, B> {
    #[inline]
    fn log2_bounds(&self) -> (f32, f32) {
        self.repr.log2_bounds()
    }

    #[inline]
    fn log2_est(&self) -> f32 {
        self.repr.log2_est()
    }
}

impl<R: ErrorBounds, const B: Word> FBig<R, B> {
    /// Calculate the natural logarithm function (`log(x)`) on the float number.
    ///
    /// # Examples
    ///
    /// ```
    /// # use core::str::FromStr;
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// let a = DBig::from_str("1.234")?;
    /// assert_eq!(a.ln(), DBig::from_str("0.2103")?);
    /// # Ok::<(), ParseError>(())
    /// ```
    #[inline]
    pub fn ln(&self) -> Self {
        self.context.unwrap_fp(self.context.ln(&self.repr, None))
    }

    /// Calculate the natural logarithm function (`log(x+1)`) on the float number
    ///
    /// # Examples
    ///
    /// ```
    /// # use core::str::FromStr;
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// let a = DBig::from_str("0.1234")?;
    /// assert_eq!(a.ln_1p(), DBig::from_str("0.11636")?);
    /// # Ok::<(), ParseError>(())
    /// ```
    #[inline]
    pub fn ln_1p(&self) -> Self {
        self.context.unwrap_fp(self.context.ln_1p(&self.repr, None))
    }

    /// Calculate the base-2 logarithm (`log2(x)`) on the float number.
    ///
    /// Correctly rounded to the context's precision under any rounding mode. For an exact power
    /// of two the result is the exact integer `log2(x)`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use core::str::FromStr;
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// let a = DBig::from_str("8")?;
    /// assert_eq!(a.log2(), DBig::from_str("3")?);
    /// # Ok::<(), ParseError>(())
    /// ```
    #[inline]
    pub fn log2(&self) -> Self {
        self.context.unwrap_fp(self.context.log2(&self.repr, None))
    }
}

// `ln2`/`ln10`/`iacoth`/`ln_base`/`ln_compute` are the near-correct logarithm primitives: they
// evaluate the series at a working precision and round once, without a Ziv certification step.
// They live on `R: Round` so that base conversion (`with_base_and_precision`, which only needs a
// near-correct constant `ln(B)`) can use them without inheriting the `ErrorBounds` bound. The
// correctly-rounded public `ln`/`ln_1p` (in the `ErrorBounds` impl below) wrap `ln_compute` in a
// Ziv loop.
impl<R: Round> Context<R> {
    /// Calculate log(2)
    ///
    /// The precision of the output will be larger than self.precision
    #[inline]
    fn ln2<const B: Word>(&self, cache: Option<&mut ConstCache>) -> FBig<R, B> {
        if let Some(c) = cache {
            return c.ln2::<B, R>(self.precision);
        }
        // log(2) = 4L(6) + 2L(99)
        // see formula (24) from Gourdon, Xavier, and Pascal Sebah.
        // "The Logarithmic Constant: Log 2." (2004)
        4 * self.iacoth(6.into()) + 2 * self.iacoth(99.into())
    }

    /// Calculate log(10)
    ///
    /// The precision of the output will be larger than self.precision
    #[inline]
    fn ln10<const B: Word>(&self, cache: Option<&mut ConstCache>) -> FBig<R, B> {
        if let Some(c) = cache {
            return c.ln10::<B, R>(self.precision);
        }
        // log(10) = log(2) + log(5) = 3log(2) + 2L(9)
        3 * self.ln2(None) + 2 * self.iacoth(9.into())
    }

    /// Calculate log(B), for internal use only
    ///
    /// The precision of the output will be larger than self.precision
    #[inline]
    pub(crate) fn ln_base<const B: Word>(&self, cache: Option<&mut ConstCache>) -> FBig<R, B> {
        if let Some(c) = cache {
            return c.ln_base::<B, R>(self.precision);
        }
        match B {
            2 => self.ln2(None),
            10 => self.ln10(None),
            i if i.is_power_of_two() => self.ln2(None) * i.trailing_zeros(),
            _ => {
                // Near-correct ln(B) via the atanh series (no Ziv certification — base conversion
                // only needs a near-correct constant). `ln_compute` is on `R: Round`, so this keeps
                // `ln_base` callable from `R: Round` contexts (base conversion).
                let guard = self.base_guard_digits::<B>() + 2;
                self.ln_compute::<B>(
                    &Repr::new(Repr::<B>::BASE.into(), 0),
                    self.precision + guard,
                    false,
                    None,
                )
                .to_value_radius::<R>()
                .0
            }
        }
    }

    /// Calculate L(n) = acoth(n) = atanh(1/n) = 1/2 log((n+1)/(n-1)), given by the
    /// series
    ///
    /// ```text
    ///                1     n + 1              1
    ///   atanh(1/n) = — log(—————) = Σ   ——————————————————
    ///                2     n - 1   i≥0 n^(2i+1) · (2i+1)
    /// ```
    ///
    /// This method is intended to be used in logarithm calculation,
    /// so the precision of the output will be larger than desired precision.
    ///
    /// Evaluated by binary splitting (see [`iacoth_bs`][crate::math::cache::iacoth_bs]):
    /// the exact integer tree state `(P, Q, T)` over `[1, N)` satisfies
    /// `L(n) = (Q + T)/(n·Q)`, with `Q` kept at O(p) digits by the ratio-form
    /// term recurrence.
    fn iacoth<const B: Word>(&self, n: IBig) -> FBig<R, B> {
        let n: u32 = (&n).try_into().expect("iacoth argument must fit in u32");

        // number of series terms until r_k < B^{-p}:  (2k+1)·log_B(n) > p.
        // The count is generously over-provisioned, so a truncating cast stands in
        // for a ceiling.
        let log_b_n = n.log2_est() / B.log2_est();
        let num_terms = (self.precision as f32 / (2.0 * log_b_n)) as usize + 10;

        let (_p, q, t) = crate::math::cache::iacoth_bs(n, 1, num_terms + 1);

        // L(n) = (Q + T) / (n·Q). Extra guard digits absorb the division's rounding
        // (the binary-splitting state is exact, so only this single round loses anything).
        let guard_digits = self.base_guard_digits::<B>();
        let work_context = Self::new(self.precision + guard_digits + 2);

        let num = work_context.convert_int::<B>(q.as_ibig() + &t).value();
        let denom = work_context.convert_int::<B>(IBig::from(n) * &q).value();
        num / denom
    }

    /// Evaluate `ln(x)` (or `ln(x+1)` when `one_plus`) at `work_precision` via the atanh series,
    /// returning a [`Ball`] whose radius is derived mechanically by Ball arithmetic (error
    /// propagates term-by-term through the series; cancellation and the `s·ln(B)` reconstruction
    /// flow through the Ball scale factors).
    ///
    /// This is the near-correct computation core shared by the public Ziv-backed `ln`/`ln_1p`
    /// (which wrap it in a retry loop) and by `ln_base` (which only needs a near-correct constant
    /// `ln(B)`). It lives on `R: Round` so those near-correct callers don't inherit the
    /// `ErrorBounds` bound.
    pub(crate) fn ln_compute<const B: Word>(
        &self,
        x: &Repr<B>,
        mut work_precision: usize,
        one_plus: bool,
        mut cache: Option<&mut ConstCache>,
    ) -> Ball<B> {
        // Round the input to the working precision; the input's own rounding is the only error
        // introduced here.
        let context = Context::<mode::HalfEven>::new(work_precision);
        let x_rounded = context.repr_round_ref(x);
        let x_n = if matches!(x_rounded, Inexact(..)) {
            IBig::ONE
        } else {
            IBig::ZERO
        };
        let x_ball = Ball::with_error(FBig::new(x_rounded.value(), context), x_n);

        // When one_plus is true and |x| < 1/B, the input is fed into the Maclaurin without scaling.
        let no_scaling = one_plus && x_ball.mid.log2_est() < -B.log2_est();

        let (s, mut x_scaled) = if no_scaling {
            (0, x_ball)
        } else {
            let x_ball = if one_plus {
                x_ball.add(&Ball::exact_int(work_precision, IBig::ONE))
            } else {
                x_ball
            };

            let log2 = x_ball.mid.log2_bounds().0;
            let s = log2 as isize - (log2 < 0.) as isize; // floor(log2(x))

            let x_scaled = if B == 2 {
                x_ball.shift(s) // exact (power-of-base shift)
            } else if s > 0 {
                x_ball.div(&Ball::with_error(
                    FBig::from(IBig::ONE << s as usize), // exact 2^s
                    IBig::ZERO,
                ))
            } else {
                x_ball.scale_int(&(IBig::ONE << (-s) as usize))
            };
            debug_assert!(x_scaled.mid >= FBig::<mode::HalfEven, B>::ONE);
            (s, x_scaled)
        };

        // The reconstruction 2·sum + s·ln(B) *cancels* for x < 1 (s < 0), so the series runs at
        // double precision to keep the pre-cancellation sum accurate. The finer ulp rescales `n`.
        if s < 0 || x_scaled.mid.repr().sign() == Sign::Negative {
            work_precision += self.precision;
            x_scaled.rescale_precision(self.precision);
        }
        let work_context = Context::<mode::HalfEven>::new(work_precision);

        // after the number is scaled to nearly one, use Maclaurin series on log(x) = 2atanh(z):
        // let z = (x-1)/(x+1) < 1, log(x) = 2atanh(z) = 2Σ(z²ⁱ⁺¹/(2i+1)) for i = 1,3,5,...
        let z = if no_scaling {
            let two = Ball::exact_int(work_precision, IBig::from(2));
            let den = x_scaled.add(&two);
            x_scaled.div(&den)
        } else {
            let one = Ball::exact_int(work_precision, IBig::ONE);
            let num = x_scaled.sub(&one);
            let den = x_scaled.add(&one);
            num.div(&den)
        };
        let z2 = z.mul(&z);
        let mut pow = z.clone();
        let mut sum = z;
        let mut k: usize = 3;
        loop {
            pow = pow.mul(&z2);

            let increase = pow.div_int(k);
            if increase.mid.abs_cmp(&sum.mid.ulp_lb()).is_le() {
                break;
            }

            sum = sum.add(&increase);
            k += 2;
        }

        // Omitted series tail: the first omitted term is ≤ sum.ulp_lb(), and the tail of the
        // atanh series shrinks by z² per step with 1/(1−z²) < B for x_scaled ∈ [1, B), so the
        // tail is < B·sum.ulp_lb() < B ulps of sum.
        sum.inflate(&IBig::from(B));

        // compose the logarithm of the original number
        let sum2 = sum.scale_int(&IBig::from(2));
        if no_scaling {
            sum2
        } else {
            // ln(2) as a ball. The constant evaluates the atanh series via binary splitting at
            // work + guard digits and rounds once to `work_precision`, so its error is a handful
            // of work-precision ulps; 8 is a conservative sound bound for every code path
            // (cached and uncached).
            let ln2 = work_context.ln2::<B>(reborrow_cache(&mut cache));
            let ln2 = Ball::with_error(ln2, IBig::from(8));
            sum2.add(&ln2.scale_int(&IBig::from(s)))
        }
    }
}

// `ln`/`ln_1p` are correctly rounded via the Ziv loop, whose containment test needs the rounding
// preimage (`R: ErrorBounds`). They delegate the series to `ln_compute`.
impl<R: ErrorBounds> Context<R> {
    /// Calculate the natural logarithm function (`log(x)`) on the float number under this context.
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
    /// let a = DBig::from_str("1.234")?;
    /// assert_eq!(context.ln(&a.repr(), None), Ok(Inexact(DBig::from_str("0.21")?, NoOp)));
    /// # Ok::<(), ParseError>(())
    /// ```
    #[inline]
    pub fn ln<const B: Word>(
        &self,
        x: &Repr<B>,
        cache: Option<&mut ConstCache>,
    ) -> FpResult<FBig<R, B>> {
        if x.is_infinite() {
            return Err(FpError::InfiniteInput);
        }
        if x.significand.is_zero() {
            // ln(±0) = -inf (a value, not an error)
            return Ok(Exact(FBig::new(Repr::neg_infinity(), *self)));
        }
        if x.sign() == Sign::Negative {
            return Err(FpError::OutOfDomain);
        }
        self.ln_internal(x, false, cache)
    }

    /// Calculate the natural logarithm function (`log(x+1)`) on the float number under this context.
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
    /// let a = DBig::from_str("0.1234")?;
    /// assert_eq!(context.ln_1p(&a.repr(), None), Ok(Inexact(DBig::from_str("0.12")?, AddOne)));
    /// # Ok::<(), ParseError>(())
    /// ```
    #[inline]
    pub fn ln_1p<const B: Word>(
        &self,
        x: &Repr<B>,
        cache: Option<&mut ConstCache>,
    ) -> FpResult<FBig<R, B>> {
        if x.is_infinite() {
            return Err(FpError::InfiniteInput);
        }
        // Domain of ln_1p is x > -1. x == -1 gives -inf; x < -1 is out of domain.
        if x.sign() == Sign::Negative && !x.significand.is_zero() {
            match FBig::<R, B>::new(x.clone(), *self).abs_cmp(&FBig::ONE) {
                Ordering::Greater => return Err(FpError::OutOfDomain), // x < -1
                Ordering::Equal => return Ok(Exact(FBig::new(Repr::neg_infinity(), *self))),
                _ => {}
            }
        }
        self.ln_internal(x, true, cache)
    }

    fn ln_internal<const B: Word>(
        &self,
        x: &Repr<B>,
        one_plus: bool,
        mut cache: Option<&mut ConstCache>,
    ) -> FpResult<FBig<R, B>> {
        assert_finite(x);

        // Exact special cases first: they need no rounding, so a precision-0 (unlimited)
        // value such as `FBig::ONE` or the one from `try_from(0.0)` must still resolve
        // ln/ln_1p exactly rather than tripping the limited-precision assertion below.
        if !one_plus && x.is_one() {
            return Ok(Exact(FBig::ZERO)); // ln(1) = +0
        }
        if one_plus && x.significand.is_zero() {
            // ln_1p(±0) = ±0
            let zero = if x.is_neg_zero() {
                FBig::new(Repr::neg_zero(), *self)
            } else {
                FBig::ZERO
            };
            return Ok(Exact(zero));
        }

        assert_limited_precision(self.precision);

        // Correct rounding via the Ziv loop: `ln_compute` evaluates the atanh series at `p + guard`
        // and reports a provable error radius; the driver retries with more guard digits until the
        // approximation's error interval lies entirely inside one rounding bin. The guard is a
        // *performance* knob (first-attempt hit rate), not a correctness backstop — Ziv certifies
        // the result. (The pre-Ziv `+ 2` is retained: with the conservative radius below it is still
        // needed for the first attempt to clear the half-ulp preimage at typical precisions.)
        let base_guard = self.base_guard_digits::<B>() + 2;
        self.ziv(base_guard + one_plus as usize, |guard| {
            Ok(self
                .ln_compute::<B>(x, self.precision + guard, one_plus, reborrow_cache(&mut cache))
                .to_value_radius::<R>())
        })
    }

    /// Calculate the base-2 logarithm (`log2(x)`) on the float number under this context.
    ///
    /// Correctly rounded to the context's precision under any rounding mode; for an exact power
    /// of two the result is the exact integer `log2(x)`.
    ///
    /// # Domain
    ///
    /// `log2(±0) = −∞` and a negative (non-zero) input is out of domain; an infinite input is an
    /// error (a finite context cannot produce the infinite `log2(+∞) = +∞` exactly).
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
    /// let context = Context::<HalfAway>::new(4);
    /// let a = DBig::from_str("10")?;
    /// assert_eq!(context.log2(&a.repr(), None), Ok(Inexact(DBig::from_str("3.322")?, AddOne)));
    /// # Ok::<(), ParseError>(())
    /// ```
    #[inline]
    pub fn log2<const B: Word>(
        &self,
        x: &Repr<B>,
        cache: Option<&mut ConstCache>,
    ) -> FpResult<FBig<R, B>> {
        if x.is_infinite() {
            return Err(FpError::InfiniteInput);
        }
        if x.significand.is_zero() {
            // log2(±0) = -inf (a value, not an error)
            return Ok(Exact(FBig::new(Repr::neg_infinity(), *self)));
        }
        if x.sign() == Sign::Negative {
            return Err(FpError::OutOfDomain);
        }
        self.log2_internal(x, cache)
    }

    fn log2_internal<const B: Word>(
        &self,
        x: &Repr<B>,
        mut cache: Option<&mut ConstCache>,
    ) -> FpResult<FBig<R, B>> {
        assert_finite(x);

        // Exact shortcuts first — they also cover unlimited precision, which the Ziv loop below
        // rejects via its limited-precision assertion.
        if x.is_one() {
            return Ok(Exact(FBig::ZERO)); // log2(1) = +0
        }

        // Exact power-of-two shortcut: if x = 2^k for an integer k, log2(x) = k. This is *required*
        // for directed rounding — the Ziv loop below cannot certify an exactly-representable
        // result whose true value sits on a rounding boundary (its shrinking error interval
        // always straddles the boundary), so without this shortcut log2(2^-159) under `Up` would
        // exhaust the retry cap and return k + 1 ulp instead of the exact k.
        //
        // log2(x) = log2(significand) + exponent·log2(B). With significand = 2^m this is an exact
        // integer whenever log2(B) is integral (B a power of two), or — for a non-power-of-two
        // base — when the exponent is zero.
        let mag = (&x.significand).unsigned_abs();
        if mag.is_power_of_two() && (x.exponent == 0 || B.is_power_of_two()) {
            let m = mag.trailing_zeros().unwrap(); // = log2(significand)
            let log2_b = B.trailing_zeros() as isize;
            let k = IBig::from(m) + IBig::from(x.exponent) * IBig::from(log2_b);
            return Ok(self.convert_int::<B>(k));
        }

        assert_limited_precision(self.precision);

        // log2(x) = ln(x)/ln(2), correctly rounded via the Ziv loop. Both logarithms come from the
        // Ball-based `ln_compute`, and dividing them as Balls composes the radius mechanically:
        // the quotient's error is bounded from the two logarithms' relative errors, with no
        // directed-interval bookkeeping or guard-digit constant. The driver certifies the result
        // against the rounding preimage exactly as before.
        let initial_guard = self.base_guard_digits::<B>() + 4;
        self.ziv(initial_guard, |guard| {
            let work_precision = self.precision + guard;
            let lx = self.ln_compute::<B>(x, work_precision, false, reborrow_cache(&mut cache));
            let two = Repr::new(IBig::from(2), 0);
            let l2 = self.ln_compute::<B>(&two, work_precision, false, reborrow_cache(&mut cache));
            Ok(lx.div(&l2).to_value_radius::<R>())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::round::mode;

    #[test]
    fn test_ln_zero_is_neg_infinity() {
        let ctx = Context::<mode::HalfEven>::new(53);
        let r = ctx.ln::<2>(&Repr::<2>::zero(), None).unwrap().value();
        assert!(r.repr().is_infinite());
        assert_eq!(r.repr().sign(), Sign::Negative);
    }

    #[test]
    fn test_iacoth() {
        let context = Context::<mode::Zero>::new(10);
        let binary_6 = context.iacoth::<2>(6.into()).with_precision(10).value();
        assert_eq!(binary_6.repr.significand, IBig::from(689));
        let decimal_6 = context.iacoth::<10>(6.into()).with_precision(10).value();
        assert_eq!(decimal_6.repr.significand, IBig::from(1682361183));

        let context = Context::<mode::Zero>::new(40);
        let decimal_6 = context.iacoth::<10>(6.into()).with_precision(40).value();
        assert_eq!(
            decimal_6.repr.significand,
            IBig::from_str_radix("1682361183106064652522967051084960450557", 10).unwrap()
        );

        let context = Context::<mode::Zero>::new(201);
        let binary_6 = context.iacoth::<2>(6.into()).with_precision(201).value();
        assert_eq!(
            binary_6.repr.significand,
            IBig::from_str_radix(
                "2162760151454160450909229890833066944953539957685348083415205",
                10
            )
            .unwrap()
        );
    }

    #[test]
    fn test_ln2_ln10() {
        let context = Context::<mode::Zero>::new(45);
        let decimal_ln2 = context.ln2::<10>(None).with_precision(45).value();
        assert_eq!(
            decimal_ln2.repr.significand,
            IBig::from_str_radix("693147180559945309417232121458176568075500134", 10).unwrap()
        );
        let decimal_ln10 = context.ln10::<10>(None).with_precision(45).value();
        assert_eq!(
            decimal_ln10.repr.significand,
            IBig::from_str_radix("230258509299404568401799145468436420760110148", 10).unwrap()
        );

        let context = Context::<mode::Zero>::new(180);
        let binary_ln2 = context.ln2::<2>(None).with_precision(180).value();
        assert_eq!(
            binary_ln2.repr.significand,
            IBig::from_str_radix("1062244963371879310175186301324412638028404515790072203", 10)
                .unwrap()
        );
        let binary_ln10 = context.ln10::<2>(None).with_precision(180).value();
        assert_eq!(
            binary_ln10.repr.significand,
            IBig::from_str_radix("882175346869410758689845931257775553286341791676474847", 10)
                .unwrap()
        );
    }

    #[test]
    fn test_log2_domain() {
        let ctx = Context::<mode::HalfEven>::new(53);
        // log2(±0) = -inf (a value, not an error)
        let r = ctx.log2::<2>(&Repr::<2>::zero(), None).unwrap().value();
        assert!(r.repr.is_infinite());
        assert_eq!(r.repr.sign(), Sign::Negative);
        // log2(negative) is out of domain
        assert!(matches!(
            ctx.log2::<2>(&Repr::new((-1).into(), 0), None),
            Err(FpError::OutOfDomain)
        ));
        // an infinite input is rejected
        assert!(matches!(ctx.log2::<2>(&Repr::infinity(), None), Err(FpError::InfiniteInput)));
    }

    #[test]
    fn test_log2_exact_power_of_two() {
        // log2(2^k) = k exactly under every rounding mode. Regression for the directed-rounding
        // defect: rounding ln(x) and ln(2) each toward the mode and dividing once does not bound
        // the quotient, so previously log2(2^-159) under `Up` returned -159 + 1 ulp.
        let p = 53;
        for k in [0isize, 1, -1, 5, 159, -159, 1000, -1000] {
            let x = Repr::<2>::new(IBig::from(1), k); // 2^k
            let r_down = Context::<mode::Down>::new(p)
                .log2::<2>(&x, None)
                .unwrap()
                .value();
            let r_up = Context::<mode::Up>::new(p)
                .log2::<2>(&x, None)
                .unwrap()
                .value();
            let r_zero = Context::<mode::Zero>::new(p)
                .log2::<2>(&x, None)
                .unwrap()
                .value();
            let r_he = Context::<mode::HalfEven>::new(p)
                .log2::<2>(&x, None)
                .unwrap()
                .value();
            // Every directed mode produces the identical value — no mode-dependent ulp.
            assert_eq!(r_down.repr, r_he.repr, "Down != HalfEven for log2(2^{k})");
            assert_eq!(r_up.repr, r_he.repr, "Up != HalfEven for log2(2^{k})");
            assert_eq!(r_zero.repr, r_he.repr, "Zero != HalfEven for log2(2^{k})");
            // And that value is exactly k.
            assert_eq!(r_he.to_int().value(), IBig::from(k), "value for log2(2^{k})");
        }
    }

    #[test]
    fn test_log2_exact_power_of_two_decimal_base() {
        // In a non-power-of-two base the shortcut still fires when the exponent is zero: a
        // significand that is itself a power of two makes x = 2^m exactly.
        let p = 53;
        for (sig, want) in [(8i32, 3isize), (1024, 10), (2, 1), (32, 5)] {
            let x = Repr::<10>::new(IBig::from(sig), 0);
            let r_down = Context::<mode::Down>::new(p)
                .log2::<10>(&x, None)
                .unwrap()
                .value();
            let r_up = Context::<mode::Up>::new(p)
                .log2::<10>(&x, None)
                .unwrap()
                .value();
            let r_he = Context::<mode::HalfEven>::new(p)
                .log2::<10>(&x, None)
                .unwrap()
                .value();
            assert_eq!(r_down.repr, r_he.repr, "Down != HalfEven for log2({sig}) base 10");
            assert_eq!(r_up.repr, r_he.repr, "Up != HalfEven for log2({sig}) base 10");
            assert_eq!(r_he.to_int().value(), IBig::from(want), "value for log2({sig}) base 10");
        }
    }

    /// For a non-power-of-two significand `sig` (so `log2` is irrational and never lands on a
    /// rounding boundary), each directed result must equal a high-precision oracle rounded to the
    /// target precision under the same mode — the definition of correct rounding.
    fn check_log2_directed_matches_oracle<const B: Word>(sig: u32, p: usize) {
        let oracle_ctx = Context::<mode::HalfEven>::new(p + 40);
        let x = Repr::<B>::new(IBig::from(sig), 0);
        let oracle = oracle_ctx.log2::<B>(&x, None).unwrap().value();

        let want_down = Context::<mode::Down>::new(p)
            .repr_round_ref(&oracle.repr)
            .value();
        let want_up = Context::<mode::Up>::new(p)
            .repr_round_ref(&oracle.repr)
            .value();
        let want_he = Context::<mode::HalfEven>::new(p)
            .repr_round_ref(&oracle.repr)
            .value();

        let got_down = Context::<mode::Down>::new(p)
            .log2::<B>(&x, None)
            .unwrap()
            .value();
        let got_up = Context::<mode::Up>::new(p)
            .log2::<B>(&x, None)
            .unwrap()
            .value();
        let got_he = Context::<mode::HalfEven>::new(p)
            .log2::<B>(&x, None)
            .unwrap()
            .value();

        assert_eq!(got_down.repr, want_down, "log2({sig}) base {B} under Down");
        assert_eq!(got_up.repr, want_up, "log2({sig}) base {B} under Up");
        assert_eq!(got_he.repr, want_he, "log2({sig}) base {B} under HalfEven");
    }

    #[test]
    fn test_log2_directed_matches_oracle() {
        let p = 24;
        for sig in [3u32, 7, 10, 12345, 65537] {
            check_log2_directed_matches_oracle::<2>(sig, p);
        }
        // Exercise a non-power-of-two base through the Ziv interval path too.
        for sig in [3u32, 7, 10, 12345] {
            check_log2_directed_matches_oracle::<10>(sig, p);
        }
    }

    // log2 of a value whose result sits within ~1 work-ulp of a power of two must still round to
    // the correct neighbor under directed modes. log2(f64::MAX) ≈ 1024 − 2^-53/ln2 sits just below
    // 1024; under Down at p=53 the answer is 1024 − 2^-42 (the largest p=53 value ≤ it), but an
    // unsound radius previously let Ziv certify 1024 on the first attempt.
    #[test]
    fn test_log2_just_below_power_of_two_directed() {
        let x = FBig::<mode::HalfEven, 2>::try_from(f64::MAX).unwrap();
        // High-precision oracle, then re-rounded to the target precision under each mode.
        let oracle = Context::<mode::HalfEven>::new(200)
            .log2::<2>(x.repr(), None)
            .unwrap()
            .value();
        for p in [24usize, 40, 53, 64] {
            let want_down = Context::<mode::Down>::new(p)
                .repr_round_ref(&oracle.repr)
                .value();
            let want_up = Context::<mode::Up>::new(p)
                .repr_round_ref(&oracle.repr)
                .value();
            let got_down = Context::<mode::Down>::new(p)
                .log2::<2>(x.repr(), None)
                .unwrap()
                .value();
            let got_up = Context::<mode::Up>::new(p)
                .log2::<2>(x.repr(), None)
                .unwrap()
                .value();
            assert_eq!(got_down.repr(), &want_down, "p={p} Down");
            assert_eq!(got_up.repr(), &want_up, "p={p} Up");
            // Directed invariant: Up ≥ Down.
            assert!(got_up.repr() >= got_down.repr(), "p={p} Up < Down");
        }
    }

    /// Directed `ln` of `x ∈ [1, 2)` must match a high-precision oracle re-rounded under the same
    /// mode. This binade (s = 0) is where the radius under-estimated the error: `result` inherits
    /// `ln_base`'s over-delivered context, and for `x` just above 1 the scaling even classifies
    /// `s = −1`, so `2·sum + s·ln2` cancels and the error stays at `sum`'s magnitude while
    /// `result`'s collapses — both make `result.ulp()` the wrong scale for the radius.
    fn check_ln_directed_in_unit_binade(k: usize, p: usize) {
        // x = (2^k + 1) * 2^-k = 1 + 2^-k, exactly representable at precision p when k < p.
        let x = Repr::<2>::new(IBig::from(1i64 << k) + IBig::ONE, -(k as isize));
        let oracle = Context::<mode::HalfEven>::new(p + 60)
            .ln::<2>(&x, None)
            .unwrap()
            .value();
        let want_down = Context::<mode::Down>::new(p)
            .repr_round_ref(&oracle.repr)
            .value();
        let want_up = Context::<mode::Up>::new(p)
            .repr_round_ref(&oracle.repr)
            .value();
        let got_down = Context::<mode::Down>::new(p)
            .ln::<2>(&x, None)
            .unwrap()
            .value();
        let got_up = Context::<mode::Up>::new(p)
            .ln::<2>(&x, None)
            .unwrap()
            .value();
        assert_eq!(got_down.repr(), &want_down, "ln(1+2^-{k}) p={p} Down");
        assert_eq!(got_up.repr(), &want_up, "ln(1+2^-{k}) p={p} Up");
        assert!(got_up.repr() >= got_down.repr(), "ln(1+2^-{k}) p={p} Up < Down");
    }

    #[test]
    fn test_ln_directed_near_one() {
        // Sweep the near-1 binade at low precision, including the k close to p cases that
        // classify as s = −1 and cancel.
        for p in [24usize, 40, 53] {
            for k in 1..p.saturating_sub(1) {
                check_ln_directed_in_unit_binade(k, p);
            }
        }
    }

    /// Fixed inputs for the `log2` oracle differential: moderate magnitudes and the
    /// near-boundary regimes the legacy directed-interval implementation was specifically sized for.
    fn log2_diff_inputs() -> Vec<Repr<2>> {
        let mut v = Vec::new();
        for x in [0.5f64, 1.5, 2.0, 3.0, 10.0, 1000.0, 1e-6, 123.456, 2.5e-10] {
            v.push(FBig::<mode::HalfEven, 2>::try_from(x).unwrap().into_repr());
        }
        // Exact powers of two.
        for k in [-100isize, -50, -10, -1, 0, 1, 10, 50, 100] {
            v.push(Repr::new(IBig::ONE, k));
        }
        // Just below the largest f64 (log2 ≈ 1024, the directed-regime case in the old comment).
        v.push(
            FBig::<mode::HalfEven, 2>::try_from(f64::MAX)
                .unwrap()
                .into_repr(),
        );
        // The [1, 2) unit binade and its mirror below 1: 1 ± 2^-k and 2 − 2^-k exercise the
        // s = −1 cancellation (the second-classified-s-−1 case the doubling compensates).
        for k in 1usize..=60 {
            v.push(Repr::new(IBig::from(1u64 << k) + IBig::ONE, -(k as isize))); // 1 + 2^-k
            v.push(Repr::new(IBig::from((1u64 << k) - 1), -(k as isize))); // 1 − 2^-k
            v.push(Repr::new(IBig::from((1u64 << (k + 1)) - 1), -(k as isize)));
            // 2 − 2^-k
        }
        v
    }

    /// The Ball-based `log2` must round exactly like a high-precision oracle (the definition of
    /// correct rounding) across precisions, modes, and the near-boundary inputs.
    ///
    /// The legacy directed-interval implementation is *not* used as the oracle: it has its own
    /// residual 1-ulp bug under directed rounding for `log2(1 − 2^-k)` at p=50 (verified against
    /// an independent high-precision computation) — exactly the class of defect this pilot
    /// replaces.
    fn check_log2_differential<R: ErrorBounds>(p: usize, x: &Repr<2>, oracle: &Repr<2>) {
        let ctx = Context::<R>::new(p);
        let want = ctx.repr_round_ref(oracle).value();
        let got = ctx.log2_internal::<2>(x, None).unwrap().value();
        assert_eq!(got.repr, want, "p={p} {} x={x:?}", std::any::type_name::<R>(),);
    }

    #[test]
    fn log2_ball_matches_oracle() {
        let inputs = log2_diff_inputs();
        // Moderate precisions: full input sweep, all five modes.
        for p in [20usize, 50, 100] {
            for x in &inputs {
                // The oracle is mode-independent: a high-precision HalfEven value re-rounded
                // under each target mode.
                let oracle = Context::<mode::HalfEven>::new(p + 60)
                    .log2::<2>(x, None)
                    .unwrap()
                    .value();
                check_log2_differential::<mode::HalfEven>(p, x, &oracle.repr);
                check_log2_differential::<mode::Down>(p, x, &oracle.repr);
                check_log2_differential::<mode::Up>(p, x, &oracle.repr);
                check_log2_differential::<mode::Zero>(p, x, &oracle.repr);
                check_log2_differential::<mode::Away>(p, x, &oracle.repr);
            }
        }
        // The arbitrary-precision regime: a reduced sweep (directed modes still exercised).
        for x in inputs.iter().step_by(9) {
            let oracle = Context::<mode::HalfEven>::new(560)
                .log2::<2>(x, None)
                .unwrap()
                .value();
            check_log2_differential::<mode::HalfEven>(500, x, &oracle.repr);
            check_log2_differential::<mode::Down>(500, x, &oracle.repr);
            check_log2_differential::<mode::Up>(500, x, &oracle.repr);
        }
    }
}
