//! Trigonometric functions, built on top of the cached constants π/2 and the real
//! [`exp`](crate::FBig::exp)/[`ln`](crate::FBig::ln) primitives:
//!
//! - Circular: `sin`, `cos`, `tan`, `sin_cos`, and their inverses `asin`, `acos`, `atan`.
//!
//! Argument reduction to the first quadrant reuses the cached π so that repeated
//! calls at increasing precision extend the shared constant state.

use crate::{
    ball::{ulps, Ball},
    cmp::repr_cmp_same_base,
    error::{assert_limited_precision, FpError},
    fbig::FBig,
    math::{
        cache::{compute_e, reborrow_cache, ConstCache},
        FpResult,
    },
    repr::{Context, Repr, Word},
    round::{mode, ErrorBounds, Round, Rounded},
};
use core::convert::TryFrom;
use dashu_base::{AbsOrd, Approximation::Exact, RemEuclid, Sign};
use dashu_int::IBig;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Quadrant {
    First,
    Second,
    Third,
    Fourth,
}

/// Build a `Normal` result equal to `±0`, preserving the sign of `x` (used by `sin`/`tan`/`sin_cos`
/// at zero input, where `sin(-0) = -0` and `tan(-0) = -0`).
fn signed_zero_normal<R: Round, const B: Word>(
    ctx: &Context<R>,
    x: &Repr<B>,
) -> FpResult<FBig<R, B>> {
    let zero = if x.is_neg_zero() {
        Repr::neg_zero()
    } else {
        Repr::zero()
    };
    Ok(Exact(FBig::<R, B>::new(zero, *ctx)))
}

impl<R: ErrorBounds> Context<R> {
    /// Work context for trigonometric functions: enough guard digits to absorb the catastrophic
    /// cancellation in `x − k·(π/2)` for large `|x|`. `guard` (the Ziv retry's growing margin)
    /// replaces the fixed base; `x_mag/10` covers cumulative reduction error scaling with `|x|`.
    /// Always [`mode::HalfEven`] — the Ball arithmetic the trig functions now run on.
    fn compute_work_context_trig<const B: Word>(
        self,
        x: &Repr<B>,
        guard: usize,
    ) -> Context<mode::HalfEven> {
        // x_mag estimates m = floor(log_BASE(|x|))
        let x_mag = (x.exponent.saturating_add(x.digits_ub() as isize)).max(0) as usize;
        let extra_guards = guard + x_mag / 10;
        let work_precision = self
            .precision
            .saturating_add(x_mag)
            .saturating_add(extra_guards);
        Context::<mode::HalfEven>::new(work_precision)
    }

    /// Reduces the argument to the first quadrant: `r = x − k·(π/2)` with `r ∈ (−π/4, π/4]`.
    /// Returns the work context, `r` as a [`Ball`] whose radius already covers the reduction
    /// error (dominated by `|k|·ulp(π/2)` for huge `|x|` — the cancellation is tracked by the
    /// Ball subtraction), and the quadrant `k % 4`.
    fn reduce_to_quadrant<const B: Word>(
        self,
        x: &Repr<B>,
        guard: usize,
        mut cache: Option<&mut ConstCache>,
    ) -> Result<(Context<mode::HalfEven>, Ball<B>, Quadrant), FpError> {
        let work_context = self.compute_work_context_trig(x, guard);
        let work_precision = work_context.precision;
        let x_ball = Ball::from_rounded(work_context.repr_round(x.clone()), work_precision);
        // `x_f` is exactly the ball's midpoint (the same rounded value), so no second rounding.
        // `div_rem_euclid`-style quotient extraction lives on FBig, so wrap the bare mids
        // (zero-cost).
        let x_f = FBig::new(x_ball.mid.clone(), work_context);

        let pi = work_context.pi::<B>(reborrow_cache(&mut cache)).value();
        let half_pi = &pi / 2u8;
        // π as a ball: the cached constant is correctly rounded to the work precision; 8 is a
        // conservative sound radius (as for the ln(2) constant).
        let rad = ulps::<B>(&half_pi.repr, work_precision, 8);
        let half_pi_ball = Ball::with_error(half_pi.clone().into_repr(), rad);

        let x_scaled = &x_f / &half_pi;
        let k_f = x_scaled.round();
        // `k_f` is the integer nearest `x_scaled`, so it's exact (or a signed zero for a tiny
        // argument in (-1, 0), which `IBig::try_from` treats as plain 0).
        let k = IBig::try_from(k_f).expect("k_f is an exact integer or signed zero");

        // r = x − k·(π/2): the cancellation and π's error (scaled by |k|) are tracked by the Ball.
        let scaled = half_pi_ball.scale_int(&k, work_precision)?;
        let r_ball = x_ball.sub(&scaled, work_precision)?;

        let k_mod_4_big = k.rem_euclid(IBig::from(4));
        let Ok(k_mod_4_int) = i8::try_from(k_mod_4_big) else {
            unreachable!("k % 4 is always in [0, 3]");
        };
        let quadrant = match k_mod_4_int {
            0 => Quadrant::First,
            1 => Quadrant::Second,
            2 => Quadrant::Third,
            3 => Quadrant::Fourth,
            _ => unreachable!(),
        };

        Ok((work_context, r_ball, quadrant))
    }

    /// Calculate the sine of the floating point representation.
    pub fn sin<const B: Word>(
        &self,
        x: &Repr<B>,
        mut cache: Option<&mut ConstCache>,
    ) -> FpResult<FBig<R, B>> {
        if x.is_infinite() {
            return Err(FpError::InfiniteInput);
        }
        assert_limited_precision(self.precision);
        if x.significand.is_zero() {
            // sin(±0) = ±0
            return signed_zero_normal(self, x);
        }

        // Ziv: reduce to the first quadrant (the guard grows per retry, enlarging the work precision
        // that absorbs the `x − k·(π/2)` cancellation), evaluate the series. The reduction error is
        // already inside the reduced argument's Ball radius.
        self.ziv(50, |guard| {
            let (work, r, quadrant) =
                self.reduce_to_quadrant(x, guard, reborrow_cache(&mut cache))?;
            let val = match quadrant {
                Quadrant::First => work.sin_compute(&r)?,
                Quadrant::Second => work.cos_compute(&r)?,
                Quadrant::Third => work.sin_compute(&r)?.neg(),
                Quadrant::Fourth => work.cos_compute(&r)?.neg(),
            };
            Ok(val.to_value_radius::<R>(&Context::<R>::new(work.precision)))
        })
    }

    /// Near-correct sine series `S(x) = x − x³/3! + x⁵/5! − …` on the reduced argument, returning a
    /// [`Ball`] whose radius is tracked mechanically (each term's rounding plus the truncated tail).
    fn sin_compute<const B: Word>(self, x: &Ball<B>) -> Result<Ball<B>, FpError> {
        if x.mid.significand.is_zero() {
            return Ok(Ball::exact(x.mid.clone()));
        }
        let wp = self.precision;
        let x2 = x.mul(x, wp)?;
        let mut sum = x.clone();
        let mut term = x.clone();
        let mut k = 1usize;
        loop {
            term = term.mul(&x2, wp)?.div_int((2 * k) * (2 * k + 1), wp)?;
            if term.mid_le_ulp_lb(&sum, wp) {
                break;
            }
            if k % 2 == 1 {
                sum = sum.sub(&term, wp)?;
            } else {
                sum = sum.add(&term, wp)?;
            }
            k += 1;
        }
        // Omitted tail: the alternating series tail is < the first omitted term < 1 ulp.
        // A zero sum means the argument (and every term) is exactly zero — the tail is 0.
        if !sum.mid.significand().is_zero() {
            sum.add_error(ulps::<B>(&sum.mid, wp, 2));
        }
        Ok(sum)
    }

    /// Calculate the cosine of the floating point representation.
    pub fn cos<const B: Word>(
        &self,
        x: &Repr<B>,
        mut cache: Option<&mut ConstCache>,
    ) -> FpResult<FBig<R, B>> {
        if x.is_infinite() {
            return Err(FpError::InfiniteInput);
        }
        assert_limited_precision(self.precision);

        if x.significand.is_zero() {
            // cos(±0) = 1
            return Ok(FBig::<R, B>::ONE.with_precision(self.precision));
        }

        self.ziv(50, |guard| {
            let (work, r, quadrant) =
                self.reduce_to_quadrant(x, guard, reborrow_cache(&mut cache))?;
            let val = match quadrant {
                Quadrant::First => work.cos_compute(&r)?,
                Quadrant::Second => work.sin_compute(&r)?.neg(),
                Quadrant::Third => work.cos_compute(&r)?.neg(),
                Quadrant::Fourth => work.sin_compute(&r)?,
            };
            Ok(val.to_value_radius::<R>(&Context::<R>::new(work.precision)))
        })
    }

    /// Near-correct cosine series `C(x) = 1 − x²/2! + x⁴/4! − …`, returning a [`Ball`] with a
    /// mechanically tracked radius. (See [`sin_compute`](Self::sin_compute).)
    fn cos_compute<const B: Word>(self, x: &Ball<B>) -> Result<Ball<B>, FpError> {
        let wp = self.precision;
        if x.mid.significand.is_zero() {
            return Ok(Ball::exact_int(IBig::ONE, wp));
        }
        let x2 = x.mul(x, wp)?;
        let one = Ball::exact_int(IBig::ONE, wp);
        let mut sum = one.clone();
        let mut term = one.clone();
        let mut k = 1usize;
        loop {
            term = term.mul(&x2, wp)?.div_int((2 * k) * (2 * k - 1), wp)?;
            if term.mid_le_ulp_lb(&sum, wp) {
                break;
            }
            if k % 2 == 1 {
                sum = sum.sub(&term, wp)?;
            } else {
                sum = sum.add(&term, wp)?;
            }
            k += 1;
        }
        sum.add_error(ulps::<B>(&sum.mid, wp, 2));
        Ok(sum)
    }

    /// Calculate both the sine and cosine of the floating point representation.
    ///
    /// This is more efficient than calling `sin` and `cos` separately.
    pub fn sin_cos<const B: Word>(
        &self,
        x: &Repr<B>,
        mut cache: Option<&mut ConstCache>,
    ) -> (FpResult<FBig<R, B>>, FpResult<FBig<R, B>>) {
        if x.is_infinite() {
            return (Err(FpError::InfiniteInput), Err(FpError::InfiniteInput));
        }
        assert_limited_precision(self.precision);

        if x.significand.is_zero() {
            // sin(±0) = ±0, cos(±0) = 1
            let s = signed_zero_normal(self, x);
            let c = Ok(FBig::<R, B>::ONE.with_precision(self.precision));
            return (s, c);
        }

        let (s, c) = self.ziv_pair(50, |guard| {
            let (work, r, quadrant) =
                self.reduce_to_quadrant(x, guard, reborrow_cache(&mut cache))?;
            let (sin_ball, cos_ball) = work.sin_cos_compute(&r)?;
            let (s, c) = match quadrant {
                Quadrant::First => (sin_ball, cos_ball),
                Quadrant::Second => (cos_ball, sin_ball.neg()),
                Quadrant::Third => (sin_ball.neg(), cos_ball.neg()),
                Quadrant::Fourth => (cos_ball.neg(), sin_ball),
            };
            let ctx = Context::<R>::new(work.precision);
            Ok((s.to_value_radius::<R>(&ctx), c.to_value_radius::<R>(&ctx)))
        });
        (s, c)
    }

    /// Simultaneously evaluate the sine and cosine series, returning both [`Ball`]s with
    /// mechanically tracked radii.
    pub(crate) fn sin_cos_compute<const B: Word>(
        self,
        x: &Ball<B>,
    ) -> Result<(Ball<B>, Ball<B>), FpError> {
        let wp = self.precision;
        if x.mid.significand.is_zero() {
            return Ok((Ball::exact(x.mid.clone()), Ball::exact_int(IBig::ONE, wp)));
        }
        let x2 = x.mul(x, wp)?;
        let one = Ball::exact_int(IBig::ONE, wp);
        let mut sin_sum = x.clone();
        let mut cos_sum = one.clone();
        let mut sin_term = x.clone();
        let mut cos_term = one.clone();
        let mut k = 1usize;
        loop {
            cos_term = cos_term.mul(&x2, wp)?.div_int((2 * k) * (2 * k - 1), wp)?;
            sin_term = sin_term.mul(&x2, wp)?.div_int((2 * k) * (2 * k + 1), wp)?;

            if sin_term.mid_le_ulp_lb(&sin_sum, wp) && cos_term.mid_le_ulp_lb(&cos_sum, wp) {
                break;
            }

            if k % 2 == 1 {
                cos_sum = cos_sum.sub(&cos_term, wp)?;
                sin_sum = sin_sum.sub(&sin_term, wp)?;
            } else {
                cos_sum = cos_sum.add(&cos_term, wp)?;
                sin_sum = sin_sum.add(&sin_term, wp)?;
            }
            k += 1;
        }
        if !sin_sum.mid.significand().is_zero() {
            sin_sum.add_error(ulps::<B>(&sin_sum.mid, wp, 2));
        }
        cos_sum.add_error(ulps::<B>(&cos_sum.mid, wp, 2));
        Ok((sin_sum, cos_sum))
    }

    /// Calculate the tangent of the floating point representation.
    ///
    /// # Note
    /// Near odd multiples of π/2 the value grows without bound; dashu's wide exponent range holds
    /// it as a large finite number rather than saturating to ±∞.
    pub fn tan<const B: Word>(
        &self,
        x: &Repr<B>,
        mut cache: Option<&mut ConstCache>,
    ) -> FpResult<FBig<R, B>> {
        if x.is_infinite() {
            return Err(FpError::InfiniteInput);
        }
        assert_limited_precision(self.precision);

        if x.significand.is_zero() {
            // tan(±0) = ±0
            return signed_zero_normal(self, x);
        }

        // tan = sin/cos, correctly rounded via the Ziv loop; the sin/cos error propagation into the
        // quotient is tracked by the Ball division. The closure's `significand.is_zero()` guard
        // below handles the unreachable exact-pole case (cos cancelling to a zero significand) by
        // forcing a retry.
        self.ziv(50, |guard| {
            let (work, r, quadrant) =
                self.reduce_to_quadrant(x, guard, reborrow_cache(&mut cache))?;
            let (sin_ball, cos_ball) = work.sin_cos_compute(&r)?;
            let (s, c) = match quadrant {
                Quadrant::First => (sin_ball, cos_ball),
                Quadrant::Second => (cos_ball, sin_ball.neg()),
                Quadrant::Third => (sin_ball.neg(), cos_ball.neg()),
                Quadrant::Fourth => (cos_ball.neg(), sin_ball),
            };
            if c.mid.significand.is_zero() {
                // cos rounded to a zero significand at this guard (the input sits on a work-
                // precision pole — unreachable for finite-precision x): force a retry.
                return Ok((FBig::<R, B>::ZERO, FBig::<R, B>::ONE));
            }
            Ok(s.div(&c, work.precision)?
                .to_value_radius::<R>(&Context::<R>::new(work.precision)))
        })
    }

    /// Calculate the arcsine of the floating point representation.
    ///
    /// # Methodology
    /// Uses the identity: `asin(x) = atan(x / sqrt(1 - x^2))`
    /// Returns `Err(OutOfDomain)` if `|x| > 1`.
    pub fn asin<const B: Word>(
        &self,
        x: &Repr<B>,
        mut cache: Option<&mut ConstCache>,
    ) -> FpResult<FBig<R, B>> {
        if x.is_infinite() {
            return Err(FpError::InfiniteInput);
        }
        assert_limited_precision(self.precision);
        if x.significand.is_zero() {
            // asin(±0) = ±0 (asin is odd), exact.
            return signed_zero_normal(self, x);
        }

        let x_orig = FBig::<R, B>::new(x.clone(), *self);
        // Domain check: |x| must be <= 1
        if x_orig.abs_cmp(&FBig::ONE).is_gt() {
            return Err(FpError::OutOfDomain);
        }

        self.ziv(50, |guard| {
            let work = Context::<mode::HalfEven>::new(self.precision + guard);
            let x_ball = Ball::from_rounded(work.repr_round_ref(x), work.precision);
            Ok(work
                .asin_ball::<B>(&x_ball, reborrow_cache(&mut cache))?
                .to_value_radius::<R>(&Context::<R>::new(work.precision)))
        })
    }

    /// `asin` of a ball: `atan(x / √(1−x²))`, with the `|x| = 1` endpoint `±π/2` handled directly
    /// (the composition's `√(1−x²)` denominator would round to zero there).
    fn asin_ball<const B: Word>(
        &self,
        x: &Ball<B>,
        mut cache: Option<&mut ConstCache>,
    ) -> Result<Ball<B>, FpError> {
        let wp = self.precision;
        let one = Ball::exact_int(IBig::ONE, wp);
        let d = one.sub(&x.mul(x, wp)?, wp)?.sqrt(wp)?;
        if d.mid.significand.is_zero() {
            // |x| = 1: asin(±1) = ±π/2 (an exact-ish endpoint; the π radius is folded in).
            let pi = Context::<mode::HalfEven>::new(wp)
                .pi::<B>(reborrow_cache(&mut cache))
                .value();
            let half_pi = &pi / 2u8;
            let rad = ulps::<B>(&half_pi.repr, wp, 8);
            let half_pi = Ball::with_error(half_pi.into_repr(), rad);
            Ok(if x.mid.sign() == Sign::Negative {
                half_pi.neg()
            } else {
                half_pi
            })
        } else {
            let arg = x.div(&d, wp)?;
            self.atan_ball::<B>(&arg, reborrow_cache(&mut cache))
        }
    }

    /// Calculate the arccosine of the floating point representation.
    ///
    /// # Methodology
    /// Uses the identity: `acos(x) = pi/2 - asin(x)`.
    /// Higher precision is used internally to avoid catastrophic cancellation near x ≈ 1.
    pub fn acos<const B: Word>(
        &self,
        x: &Repr<B>,
        mut cache: Option<&mut ConstCache>,
    ) -> FpResult<FBig<R, B>> {
        if x.is_infinite() {
            return Err(FpError::InfiniteInput);
        }
        assert_limited_precision(self.precision);

        let x_orig = FBig::<R, B>::new(x.clone(), *self);
        let cmp_one = x_orig.abs_cmp(&FBig::ONE);
        if cmp_one.is_gt() {
            return Err(FpError::OutOfDomain);
        }
        if cmp_one.is_eq() {
            // |x| = 1: the composition π/2 − asin(±1) cancels onto an exact value. acos(1) = 0 is
            // the acute case — under directed rounding 0's preimage is one-sided ([0, ulp)), so the
            // Ziv containment test can never certify it. acos(-1) = π is handled here too.
            return Ok(if x.sign() == Sign::Positive {
                Exact(FBig::<R, B>::new(Repr::zero(), *self))
            } else {
                self.pi::<B>(reborrow_cache(&mut cache))
            });
        }

        self.ziv(50, |guard| {
            let work = Context::<mode::HalfEven>::new(self.precision + guard);
            let x_ball = Ball::from_rounded(work.repr_round_ref(x), work.precision);
            let asin_ball = work.asin_ball::<B>(&x_ball, reborrow_cache(&mut cache))?;
            let pi = work.pi::<B>(reborrow_cache(&mut cache)).value();
            let half_pi = &pi / 2u8;
            let rad = ulps::<B>(&half_pi.repr, work.precision, 8);
            let half_pi = Ball::with_error(half_pi.into_repr(), rad);
            Ok(half_pi
                .sub(&asin_ball, work.precision)?
                .to_value_radius::<R>(&Context::<R>::new(work.precision)))
        })
    }

    /// Calculate the arctangent of the floating point representation.
    pub fn atan<const B: Word>(
        &self,
        x: &Repr<B>,
        mut cache: Option<&mut ConstCache>,
    ) -> FpResult<FBig<R, B>> {
        if x.is_infinite() {
            // atan(±inf) = ±π/2 — preserved (a well-defined finite result for an infinite input)
            let pi = self.pi::<B>(reborrow_cache(&mut cache)).value();
            let half_pi: FBig<R, B> = pi / 2;
            let res: FBig<R, B> = if x.sign() == Sign::Positive {
                half_pi
            } else {
                -half_pi
            };
            return Ok(res.with_precision(self.precision));
        }

        assert_limited_precision(self.precision);

        if x.significand.is_zero() {
            // atan(±0) = ±0
            return signed_zero_normal(self, x);
        }

        self.ziv(50, |guard| {
            let work = Context::<mode::HalfEven>::new(self.precision + guard);
            let x_ball = Ball::from_rounded(work.repr_round_ref(x), work.precision);
            Ok(work
                .atan_ball::<B>(&x_ball, reborrow_cache(&mut cache))?
                .to_value_radius::<R>(&Context::<R>::new(work.precision)))
        })
    }

    /// `atan` of a ball, with the `|x| ≥ 1` branch (`π/2 − atan(1/x)`). Odd: the sign of `x` is
    /// applied last (the `|x| ≥ 1` branch's `1/x` would otherwise lose it).
    fn atan_ball<const B: Word>(
        &self,
        x: &Ball<B>,
        mut cache: Option<&mut ConstCache>,
    ) -> Result<Ball<B>, FpError> {
        let wp = self.precision;
        let sign = x.mid.sign();
        let x_abs = if sign == Sign::Negative {
            x.clone().neg()
        } else {
            x.clone()
        };
        let one = Ball::exact_int(IBig::ONE, wp);
        let abs_ge_one = repr_cmp_same_base::<B, true>(&x_abs.mid, &one.mid, None).is_ge();
        let res = if abs_ge_one {
            let pi = Context::<mode::HalfEven>::new(wp)
                .pi::<B>(reborrow_cache(&mut cache))
                .value();
            let half_pi = &pi / 2u8;
            let rad = ulps::<B>(&half_pi.repr, wp, 8);
            let half_pi = Ball::with_error(half_pi.into_repr(), rad);
            let inv_x = one.div(&x_abs, wp)?;
            half_pi.sub(&self.atan_compute(&inv_x)?, wp)
        } else {
            self.atan_compute(&x_abs)
        };
        Ok(if sign == Sign::Negative {
            res?.neg()
        } else {
            res?
        })
    }

    /// Near-correct Euler series for `atan(x)` (`|x| ≤ 1`), returning a [`Ball`] with a
    /// mechanically tracked radius.
    fn atan_compute<const B: Word>(self, x: &Ball<B>) -> Result<Ball<B>, FpError> {
        let wp = self.precision;
        let x2 = x.mul(x, wp)?;
        let one = Ball::exact_int(IBig::ONE, wp);
        let one_plus_x2 = one.add(&x2, wp)?;
        let mut term = x.div(&one_plus_x2, wp)?;
        let mut sum = term.clone();
        let factor = x2.scale_int(&IBig::from(2), wp)?.div(&one_plus_x2, wp)?;
        let mut n = 1usize;
        loop {
            term = term
                .mul(&factor, wp)?
                .scale_int(&IBig::from(n), wp)?
                .div_int(2 * n + 1, wp)?;
            if term.mid_le_ulp_lb(&sum, wp) {
                break;
            }
            sum = sum.add(&term, wp)?;
            n += 1;
        }
        // Omitted tail: the Euler terms shrink by (2x²/(1+x²))·n/(2n+1) < 1/2, so the tail is < 2 ulps.
        // A zero sum means x (and every term) is exactly zero — the tail is 0 (atan2(±0, ·)).
        if !sum.mid.significand().is_zero() {
            sum.add_error(ulps::<B>(&sum.mid, wp, 2));
        }
        Ok(sum)
    }

    /// Calculate the arctangent of y / x.
    ///
    /// Handles signed infinities according to IEEE 754 standards.
    /// Returns `Err(OutOfDomain)` if both arguments are zero.
    pub fn atan2<const B: Word>(
        &self,
        y: &Repr<B>,
        x: &Repr<B>,
        mut cache: Option<&mut ConstCache>,
    ) -> FpResult<FBig<R, B>> {
        if y.is_finite() && x.is_finite() && y.significand.is_zero() && x.significand.is_zero() {
            return Err(FpError::OutOfDomain);
        }

        assert_limited_precision(self.precision);

        // Handle Infinities according to IEEE 754 (computed at the target precision).
        if y.is_infinite() || x.is_infinite() {
            let (sy, sx) = (y.sign() == Sign::Positive, x.sign() == Sign::Positive);
            let pi_val = self.pi::<B>(reborrow_cache(&mut cache)).value();
            let res: FBig<R, B> = match (y.is_infinite(), x.is_infinite(), sy, sx) {
                (true, true, true, true) => pi_val.clone() / 4u8,
                (true, true, true, false) => pi_val.clone() * 3u8 / 4u8,
                (true, true, false, true) => -(pi_val.clone() / 4u8),
                (true, true, false, false) => -(pi_val.clone() * 3u8 / 4u8),
                (true, false, true, _) => pi_val.clone() / 2u8,
                (true, false, false, _) => -(pi_val.clone() / 2u8),
                (false, true, _, true) => {
                    // atan2(±finite, +inf) = ±0 (signed zero of y)
                    if sy {
                        FBig::<R, B>::ZERO
                    } else {
                        FBig::<R, B>::new(Repr::neg_zero(), *self)
                    }
                }
                (false, true, true, false) => pi_val.clone(),
                (false, true, false, false) => -pi_val,
                _ => unreachable!(),
            };
            return Ok(res.with_precision(self.precision));
        }

        // x == 0, y finite nonzero: atan2 = ±π/2.
        if x.significand.is_zero() {
            let half_pi = self.pi::<B>(reborrow_cache(&mut cache)).value() / 2u8;
            let res = if y.sign() == Sign::Positive {
                half_pi
            } else {
                -half_pi
            };
            return Ok(res.with_precision(self.precision));
        }

        // x ≠ 0, finite: atan2 = atan(y/x) ± (quadrant π), all as Ball composition.
        self.ziv(50, |guard| {
            let work = Context::<mode::HalfEven>::new(self.precision + guard);
            let y_ball = Ball::from_rounded(work.repr_round_ref(y), work.precision);
            let x_ball = Ball::from_rounded(work.repr_round_ref(x), work.precision);
            let ratio = y_ball.div(&x_ball, work.precision)?;
            let atan_val = work.atan_ball::<B>(&ratio, reborrow_cache(&mut cache))?;
            let res = if x.sign() == Sign::Positive {
                atan_val
            } else {
                let pi = work.pi::<B>(reborrow_cache(&mut cache)).value();
                let rad = ulps::<B>(&pi.repr, work.precision, 8);
                let pi_ball = Ball::with_error(pi.into_repr(), rad);
                if y.sign() == Sign::Positive {
                    atan_val.add(&pi_ball, work.precision)?
                } else {
                    atan_val.sub(&pi_ball, work.precision)?
                }
            };
            Ok(res.to_value_radius::<R>(&Context::<R>::new(work.precision)))
        })
    }
}

impl<R: ErrorBounds, const B: Word> FBig<R, B> {
    /// Calculate the sine of the floating point number.
    ///
    /// # Panics
    /// Panics if the input is infinite.
    #[inline]
    pub fn sin(&self) -> Self {
        self.context.unwrap_fp(self.context.sin(&self.repr, None))
    }

    /// Calculate the cosine of the floating point number.
    ///
    /// # Panics
    /// Panics if the input is infinite.
    #[inline]
    pub fn cos(&self) -> Self {
        self.context.unwrap_fp(self.context.cos(&self.repr, None))
    }

    /// Calculate both the sine and cosine of the floating point number.
    ///
    /// This is more efficient than calling `sin` and `cos` separately.
    ///
    /// # Panics
    /// Panics if the input is infinite.
    #[inline]
    pub fn sin_cos(&self) -> (Self, Self) {
        let (s, c) = self.context.sin_cos(&self.repr, None);
        (self.context.unwrap_fp(s), self.context.unwrap_fp(c))
    }

    /// Calculate the tangent of the floating point number.
    ///
    /// At odd multiples of π/2 the result is an infinity (returned as a value).
    ///
    /// # Panics
    /// Panics if the input is infinite.
    #[inline]
    pub fn tan(&self) -> Self {
        self.context.unwrap_fp(self.context.tan(&self.repr, None))
    }

    /// Calculate the arcsine of the floating point number.
    ///
    /// # Panics
    /// Panics if the input is infinite or `|self| > 1` (out of domain).
    #[inline]
    pub fn asin(&self) -> Self {
        self.context.unwrap_fp(self.context.asin(&self.repr, None))
    }

    /// Calculate the arccosine of the floating point number.
    ///
    /// # Panics
    /// Panics if the input is infinite or `|self| > 1` (out of domain).
    #[inline]
    pub fn acos(&self) -> Self {
        self.context.unwrap_fp(self.context.acos(&self.repr, None))
    }

    /// Calculate the arctangent of the floating point number. `atan(±inf) = ±π/2`.
    #[inline]
    pub fn atan(&self) -> Self {
        self.context.unwrap_fp(self.context.atan(&self.repr, None))
    }

    /// Calculate the arctangent of `self / x`.
    ///
    /// # Panics
    /// Panics if both arguments are zero.
    #[inline]
    pub fn atan2(&self, x: &Self) -> Self {
        self.context
            .unwrap_fp(self.context.atan2(&self.repr, &x.repr, None))
    }
}

impl<R: Round> Context<R> {
    /// Calculate π using the Chudnovsky algorithm with binary splitting.
    ///
    /// The Chudnovsky algorithm is one of the most efficient methods for
    /// high-precision π calculation, providing ~14.18 decimal digits per term.
    ///
    /// # Methodology
    /// We use Binary Splitting to evaluate the series. This technique transforms
    /// the linear-time summation into a recursive tree evaluation. By combining
    /// terms into large products, it allows the library to leverage fast
    /// multiplication algorithms (like Toom-3 or FFT) as the numbers grow,
    /// leading to significant performance gains over simple iterative summation.
    #[must_use]
    pub fn pi<const B: Word>(&self, cache: Option<&mut ConstCache>) -> Rounded<FBig<R, B>> {
        if let Some(c) = cache {
            return c.pi::<B, R>(self.precision);
        }

        // No shared cache: compute via a one-shot ConstCache so the Chudnovsky series
        // and the 426880·√10005·Q/T finalization live in exactly one place (see
        // ConstCache::pi), instead of being duplicated here.
        let mut fresh = ConstCache::new();
        fresh.pi::<B, R>(self.precision)
    }

    /// Calculate *e* (Euler's number) by binary splitting on `e = Σ 1/k!`.
    ///
    /// Unlike [`pi`](Self::pi), this takes no constant cache: *e* depends on no
    /// other cached constant and is itself reused by no operation, so there is no
    /// state worth sharing across calls. The factorial series is the optimal
    /// algorithm for *e* (`O(M(n) log n)`, faster than π) and avoids the
    /// argument-reduction and `√p`-fold powering that `exp(1)` would pay for.
    ///
    /// # Panics
    ///
    /// Panics if the context precision is 0.
    #[must_use]
    pub fn e<const B: Word>(&self) -> Rounded<FBig<R, B>> {
        compute_e::<B, R>(self.precision)
    }
}

impl<R: Round, const B: Word> FBig<R, B> {
    /// Calculate π with the given precision and the default rounding mode.
    #[inline]
    #[must_use]
    pub fn pi(precision: usize) -> Self {
        Context::<R>::new(precision).pi(None).value()
    }

    /// Calculate *e* (Euler's number) with the given precision and the default
    /// rounding mode.
    #[inline]
    #[must_use]
    pub fn e(precision: usize) -> Self {
        Context::<R>::new(precision).e::<B>().value()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::round::mode;
    use crate::DBig;
    use core::str::FromStr;

    #[test]
    fn test_atan_infinity_is_preserved() {
        let ctx = Context::<mode::HalfEven>::new(53);
        // atan(±inf) = ±π/2 — a finite result, preserved (not an error)
        let r = ctx.atan::<2>(&Repr::<2>::infinity(), None).unwrap().value();
        assert!(r.repr().sign() == Sign::Positive);
        // it should be approximately π/2
        assert!(r > FBig::<mode::HalfEven>::ONE);
    }

    /// Regression: a tiny *negative* argument used to panic in `reduce_to_quadrant`.
    /// `round()` of a value in (-1, 0) yields signed zero (exponent sentinel -1),
    /// which `IBig::try_from` now accepts as plain 0.
    #[test]
    fn test_trig_tiny_negative_no_panic() {
        let ctx = Context::<mode::HalfAway>::new(30);
        for &e in &[-1isize, -2, -10, -30] {
            // x = -1 * BASE^e, a tiny negative value
            let x = Repr::<10>::new(IBig::from(-1), e);
            let s = ctx.sin::<10>(&x, None).unwrap().value();
            let c = ctx.cos::<10>(&x, None).unwrap().value();
            let (ss, cc) = ctx.sin_cos::<10>(&x, None);
            let ss = ss.unwrap().value();
            let cc = cc.unwrap().value();
            // sin is odd, cos is even: sin(x) ≈ x (negative), cos(x) ≈ 1
            assert_eq!(s.sign(), Sign::Negative);
            assert_eq!(c.sign(), Sign::Positive);
            assert_eq!(ss.sign(), Sign::Negative);
            assert_eq!(cc.sign(), Sign::Positive);
        }
    }

    /// Regression: a 49-digit significand at precision 100 used to assertion-fail in `Context::sin`'s
    /// rounding logic (found during fuzzing). Promoted here from the excluded `fuzz/` crate so it runs
    /// in CI; rewritten to the current `Context::sin` API.
    #[test]
    fn test_sin_many_digit_rounding_no_panic() {
        let x = DBig::from_str("-5.525474318981006776603409487767135633516667011547942409467e-3")
            .unwrap();
        let ctx = Context::<mode::HalfEven>::new(100);
        let s = ctx.sin::<10>(x.repr(), None).unwrap().value();
        // sin(x) ≈ x for a small negative x — completing without panicking is the regression guard.
        assert_eq!(s.sign(), Sign::Negative);
    }

    /// tan near a pole (π/2) must not panic, and its sign must follow the pole side: just below →
    /// large positive (→ +∞), just above → large negative (→ −∞). Guards the pole check, which
    /// tests `cos` with `significand.is_zero()` (not `is_pos_zero`, which would miss `-0`) and
    /// assigns the infinity sign as `sign(sin)·sign(cos)`.
    #[test]
    fn test_tan_near_pole_signs_and_no_panic() {
        let p = 53usize;
        let ctx = Context::<mode::HalfEven>::new(p);
        let half_pi = FBig::<mode::HalfEven>::pi(p) / 2u8;
        // a clear offset either side of the pole (≈2⁻¹⁰, far larger than half_pi's rounding error)
        let eps = FBig::<mode::HalfEven>::ONE >> 10;
        let below = ctx
            .tan::<2>((half_pi.clone() - &eps).repr(), None)
            .unwrap()
            .value();
        let above = ctx
            .tan::<2>((half_pi.clone() + &eps).repr(), None)
            .unwrap()
            .value();
        assert_eq!(below.sign(), Sign::Positive, "tan just below π/2 is large positive");
        assert_eq!(above.sign(), Sign::Negative, "tan just above π/2 is large negative");
        // sanity: tan(π/4) = 1
        let pi = FBig::<mode::HalfEven>::pi(p);
        let q = ctx.tan::<2>((pi / 4u8).repr(), None).unwrap().value();
        assert!(
            (q.clone() - FBig::ONE).abs_cmp(&(FBig::ONE >> 40)).is_le(),
            "tan(π/4) ≈ 1, got {q:?}"
        );
    }
}
