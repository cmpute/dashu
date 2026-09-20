//! Trigonometric functions, built on top of the cached constants π/2 and the real
//! [`exp`](crate::FBig::exp)/[`ln`](crate::FBig::ln) primitives:
//!
//! - Circular: `sin`, `cos`, `tan`, `sin_cos`, the ×π variants `sin_pi`, `cos_pi`, `tan_pi`,
//!   `sin_cos_pi`, and the inverses `asin`, `acos`, `atan`.
//!
//! Argument reduction to the first quadrant reuses the cached π so that repeated
//! calls at increasing precision extend the shared constant state. The ×π variants reduce
//! exactly in integer arithmetic instead (see [`Context::sin_pi`]), so they need no
//! magnitude-dependent guard digits and resolve quarter-integer arguments exactly.

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
    utils::{digit_len, shl_digits},
};
use core::convert::TryFrom;
use dashu_base::{Abs, AbsOrd, Approximation::Exact, DivRem, EstimatedLog2, RemEuclid, Sign};
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

/// `B^e mod M` by binary exponentiation — the residue of a radix power without ever
/// materializing the power (e can be astronomically large, e.g. an exponent near the `isize`
/// range). All intermediates stay below `M`, which the callers keep small (`u` or `24u`).
fn digits_powmod<const B: Word>(e: usize, modulus: &IBig) -> IBig {
    // `rem_euclid` yields the (non-negative) `UBig` residue; wrap back for the IBig pipeline.
    let mut result = IBig::ONE;
    let mut base = IBig::from(crate::utils::base_as_ibig::<B>().rem_euclid(modulus.clone()));
    let mut exp = e;
    while exp > 0 {
        if exp & 1 == 1 {
            result = IBig::from((result * &base).rem_euclid(modulus.clone()));
        }
        base = IBig::from((&base * &base).rem_euclid(modulus.clone()));
        exp >>= 1;
    }
    result
}

/// Classify the ×u exact cases: `Some(j)` with `j = (k·x/u) mod 2k` (an euclidean modulus on
/// the signed value, so negative arguments classify directly) when `k·x/u` is an integer,
/// `None` otherwise. The two keys the ×u family uses:
/// - `k = 12` (sine/cosine): `j ≡ 0 (mod 3)` means the angle is a multiple of π/2 — the
///   quarter table keyed by `(4x/u) mod 8 = (j/3) mod 8`; `j` odd (`j mod 12 ∈ {1, 5}` /
///   `{7, 11}`) means ±π/6 modulo π — sine is `±1/2`; `j` even with `j mod 12 ∈ {2, 10}` /
///   `{4, 8}` means ±π/3 modulo π — cosine is `±1/2`.
/// - `k = 8` (tangent): `j mod 8` is the eighth-of-turn index — `0`/`4` map to `±0`, `2`/`6`
///   to the poles, `1`/`5` to `+1`, `3`/`7` to `−1`.
///
/// Powers of B are only materialized when bounded by the input's own size; astronomical
/// exponents are reduced by modular exponentiation.
fn unit_residue<const B: Word>(x: &Repr<B>, u: usize, k: usize) -> Option<i8> {
    debug_assert!(u > 0, "the callers reject u = 0");
    let u_ibig = IBig::from(u);
    let k_ibig = IBig::from(k);
    let modulus = IBig::from(2 * k) * &u_ibig; // 2k·u
    let m = &x.significand;
    if m.is_zero() {
        // ±0 — the infinities are screened out by the callers before this.
        return Some(0);
    }
    let e = x.exponent;
    if e >= 0 {
        // j = (k·m·B^e / u) mod 2k = (k·m·B^e mod 2ku) / u — all modular, so the
        // astronomically large B^e is reduced by binary exponentiation instead of being
        // materialized.
        let km = &k_ibig * m;
        let n = IBig::from(km.rem_euclid(modulus.clone()))
            * digits_powmod::<B>(e.unsigned_abs(), &modulus);
        let n = IBig::from(n.rem_euclid(modulus));
        if &n % &u_ibig != IBig::ZERO {
            return None;
        }
        let j = n / &u_ibig; // in [0, 2k)
        Some(i8::try_from(j).expect("j < 2k ≤ 24 fits in i8"))
    } else {
        // x = m/B^s: k·x/u = k·m/(u·B^s) is an integer iff u·B^s divides k·m — possible only
        // when u·B^s ≤ |k·m|, so astronomical exponents are rejected by log2 bounds before any
        // power of B is materialized (the materialized power stays within the input's size).
        let s = e.unsigned_abs();
        let km = &k_ibig * m;
        let (_, ub_km) = km.log2_bounds();
        let (u_lb, _) = u.log2_bounds();
        let (b_lb, _) = B.log2_bounds();
        if ub_km < u_lb + s as f32 * b_lb {
            // u·B^s ≥ 2^(u_lb + s·b_lb) > |k·m|: cannot divide.
            return None;
        }
        let d = &u_ibig * shl_digits::<B>(&IBig::ONE, s);
        let (v, rem) = km.div_rem(d);
        if !rem.is_zero() {
            return None;
        }
        let j = v.rem_euclid(IBig::from(2 * k));
        Some(i8::try_from(j).expect("j mod 2k ≤ 23 fits in i8"))
    }
}

/// The quadrant of a half-integer index k — the `k = round(x/(π/2))` of the radian reduction
/// and the `k = round(4|x|/u)` of the ×u reduction alike (both subtract k·π/2 from the angle).
fn quadrant_of(k: &IBig) -> Quadrant {
    let r = k.rem_euclid(IBig::from(4));
    match i8::try_from(r).expect("k mod 4 fits in i8") {
        0 => Quadrant::First,
        1 => Quadrant::Second,
        2 => Quadrant::Third,
        3 => Quadrant::Fourth,
        _ => unreachable!(),
    }
}

/// The exact ×u argument reduction (on `|x|`): first `|x| mod u` (exact, `∈ [0, u)`), then
/// `k = round(4·|x|/u)` with ties rounded up and `r = |x| − k·u/4 ∈ [−u/8, u/8]` as an exact
/// rational, plus the quadrant `k mod 4`. Unlike the radian
/// [`Context::reduce_to_quadrant`], the reduction is exact integer arithmetic on the
/// significand, so it holds for arbitrarily large `|x|` — the working precision needs no
/// guard scaling with the magnitude.
enum UnitReduced<const B: Word> {
    /// The `k = 0` case (`|x| mod u < u/8`, quadrant `First`): `r` itself, kept as a raw
    /// [`Repr`] because the power of B behind its exponent can be astronomically large
    /// (e.g. `3·2^−10⁹`) and must not be materialized — the argument ball is formed by a
    /// product instead of a rational split.
    Small(Repr<B>),
    /// `r = num/den` as an exact rational — never a `Repr`, since an odd base cannot
    /// represent the fractions exactly. `den` is bounded by the input's own digit count,
    /// because the astronomically-scaled cases were taken by the `k = 0` fast path.
    Split {
        quadrant: Quadrant,
        num: IBig,
        den: IBig,
    },
}

fn reduce_unit_argument<const B: Word>(x: &Repr<B>, u: usize) -> UnitReduced<B> {
    debug_assert!(!x.significand.is_zero());
    debug_assert!(u > 0);

    // Step 1: r0 = |x| mod u ∈ [0, u), exact. (A zero r0 means x is a multiple of u — a
    // quarter case, resolved by the exact-case table before the reduction.)
    let u_ibig = IBig::from(u);
    let m = x.significand.clone().abs();
    let e = x.exponent;
    // log2 lower bounds of u and of the base B, for the `u·B^s` magnitude comparisons below.
    let (u_lb, _) = u.log2_bounds();
    let (b_lb, _) = B.log2_bounds();
    let (sig0, e0) = if e >= 0 {
        // Integer argument: m·B^e mod u by modular exponentiation — B^e is never materialized.
        let r = IBig::from(m.rem_euclid(u_ibig.clone()))
            * digits_powmod::<B>(e.unsigned_abs(), &u_ibig);
        (IBig::from(r.rem_euclid(u_ibig.clone())), 0isize)
    } else {
        // |x| = m/B^s < u ⟺ m < u·B^s (checked by bounds, no materialization): r0 = |x| itself.
        let s = e.unsigned_abs();
        let (_, ub_m) = m.log2_bounds();
        if ub_m < u_lb + s as f32 * b_lb {
            (m, e)
        } else {
            // |x| ≥ u, so u·B^s ≤ m: the power is bounded by the input's own size.
            let d = &u_ibig * shl_digits::<B>(&IBig::ONE, s);
            (IBig::from(m.rem_euclid(d)), e)
        }
    };
    debug_assert!(!sig0.is_zero(), "multiples of u are table-resolved");
    let s0 = e0.unsigned_abs(); // e0 ≤ 0

    // Step 2 — k = 0 fast path: 8·sig0 < u·B^(s0) (guaranteed by the bounds), so 4r0/u < 1/2
    // rounds to k = 0 and no power of B is materialized — crucially so, since here it can be
    // astronomically large.
    let (_, ub8m) = (IBig::from(8) * &sig0).log2_bounds();
    if ub8m < u_lb + s0 as f32 * b_lb {
        return UnitReduced::Small(Repr::new(sig0, e0));
    }

    // Not tiny: within the bounds' slack sig0 ≥ u·B^(s0)/8, so the power stays within the
    // input's own digit count. k = round-half-up of 4r0/u = ⌊(8·sig0 + u·B^(s0)) /
    // (2·u·B^(s0))⌋ (truncating division of non-negatives is a floor); the split
    // r = (4·sig0 − k·u·B^(s0)) / (4·B^(s0)) is exact integer arithmetic — the cancellation a
    // float subtraction would suffer is free here.
    let b_pow = shl_digits::<B>(&IBig::ONE, s0);
    let u_bs = &u_ibig * &b_pow; // u·B^(s0)
    let k = (IBig::from(8) * &sig0 + &u_bs) / (IBig::from(2) * &u_bs);
    let num = IBig::from(4) * sig0 - &k * &u_bs;
    let den = IBig::from(4) * b_pow;
    UnitReduced::Split {
        quadrant: quadrant_of(&k),
        num,
        den,
    }
}

/// The ×π argument as a ball on a raw (unreduced) value: `π·x`, with π's conservative radius
/// and x's rounding to the work precision both tracked by the ball arithmetic. Used by the
/// hyperbolic ×π functions (which have no periodic argument reduction at all).
pub(crate) fn pi_scaled_ball<const B: Word>(
    work: &Context<mode::HalfEven>,
    x: &Repr<B>,
    mut cache: Option<&mut ConstCache>,
) -> Result<Ball<B>, FpError> {
    let wp = work.precision;
    let pi = work.pi::<B>(reborrow_cache(&mut cache)).value();
    // π as a ball: the cached constant is correctly rounded to the work precision; 8 is a
    // conservative sound radius (as for the ln(2) constant).
    let rad = ulps::<B>(&pi.repr, wp, 8);
    let pi_ball = Ball::with_error(pi.into_repr(), rad);
    let x_ball = Ball::from_rounded(work.repr_round_ref(x), wp);
    pi_ball.mul(&x_ball, wp)
}

/// The reduced ×u argument as a ball: `t = (2π/u)·r` with `|t| ≤ π/4`, plus the quadrant. The
/// constant `2π/u` carries π's conservative radius (as in `Context::reduce_to_quadrant`) and
/// divides by the *exact* integer u (held at its own digit width — a work precision narrower
/// than u would round the divisor); the rational `r` is rounded to the work precision, with
/// every rounding tracked by the ball arithmetic.
fn unit_argument_ball<const B: Word>(
    work: &Context<mode::HalfEven>,
    reduced: &UnitReduced<B>,
    u: usize,
    mut cache: Option<&mut ConstCache>,
) -> Result<(Ball<B>, Quadrant), FpError> {
    let wp = work.precision;
    let pi = work.pi::<B>(reborrow_cache(&mut cache)).value();
    // π as a ball: the cached constant is correctly rounded to the work precision; 8 is a
    // conservative sound radius (as for the ln(2) constant).
    let rad = ulps::<B>(&pi.repr, wp, 8);
    let pi_ball = Ball::with_error(pi.into_repr(), rad);
    let two_pi = pi_ball.scale_int(&IBig::from(2), wp)?;
    let u_digits = digit_len::<B>(&IBig::from(u));
    let u_ball = Ball::exact_int(IBig::from(u), u_digits.max(wp));
    let two_pi_over_u = two_pi.div(&u_ball, wp)?;
    Ok(match reduced {
        UnitReduced::Small(r) => {
            let r_ball = Ball::from_rounded(work.repr_round_ref(r), wp);
            (two_pi_over_u.mul(&r_ball, wp)?, Quadrant::First)
        }
        UnitReduced::Split { quadrant, num, den } => {
            // num and den are rounded to the work precision before the ball division — for a
            // long input significand this caps the intermediate width instead of carrying the
            // input's full digit count through the division.
            let num_ball = Ball::from_rounded(work.repr_round_ref(&Repr::new(num.clone(), 0)), wp);
            let den_ball = Ball::from_rounded(work.repr_round_ref(&Repr::new(den.clone(), 0)), wp);
            (two_pi_over_u.mul(&num_ball, wp)?.div(&den_ball, wp)?, *quadrant)
        }
    })
}

/// `v·u/(2π)` of a ball — the inverse ×u family's final scaling. 2π carries the usual
/// conservative π radius; the integer scaling is exact.
fn scale_unit<const B: Word>(
    work: &Context<mode::HalfEven>,
    v: &Ball<B>,
    u: usize,
    mut cache: Option<&mut ConstCache>,
) -> Result<Ball<B>, FpError> {
    let wp = work.precision;
    let pi = work.pi::<B>(reborrow_cache(&mut cache)).value();
    // π as a ball: the cached constant is correctly rounded to the work precision; 8 is a
    // conservative sound radius (as for the ln(2) constant).
    let rad = ulps::<B>(&pi.repr, wp, 8);
    let two_pi = Ball::with_error(pi.into_repr(), rad).scale_int(&IBig::from(2), wp)?;
    v.scale_int(&IBig::from(u), wp)?.div(&two_pi, wp)
}

/// The exact rational `num/den` rounded under the caller's mode — the exact-value rows of the
/// ×u tables (exact whenever den's prime factors divide the base, e.g. `u/4` in base 2 or 10;
/// correctly rounded otherwise).
fn exact_ratio<R: Round, const B: Word>(
    ctx: &Context<R>,
    num: IBig,
    den: usize,
) -> FpResult<FBig<R, B>> {
    ctx.div(&Repr::new(num, 0), &Repr::from(den))
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

        Ok((work_context, r_ball, quadrant_of(&k)))
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

    /// Calculate the sine of `2π·x/u` — the ×u family with `u = 2` (i.e. `sin(x·π)`).
    ///
    /// See [`sin_unit`](Self::sin_unit) for the semantics; this is a thin wrapper.
    pub fn sin_pi<const B: Word>(
        &self,
        x: &Repr<B>,
        cache: Option<&mut ConstCache>,
    ) -> FpResult<FBig<R, B>> {
        self.sin_unit(x, 2, cache)
    }

    /// Calculate the sine of `2π·x/u` (the angle x is measured in units of the full turn
    /// divided by `u`; e.g. `u = 360` gives degrees).
    ///
    /// # Methodology
    /// The ×u variant has rational special points: the argument reduces *exactly* mod u in
    /// integer arithmetic, so arguments where `12x/u` is an integer resolve to exact values
    /// (`0`, `±1`, `±1/2`) and — unlike [`sin`](Self::sin) — the accuracy does not degrade
    /// with the magnitude of x. The exact cases are resolved before the Ziv loop, whose
    /// containment test cannot certify exact values under directed rounding; the general path
    /// evaluates the sine/cosine series on the exact rational remainder
    /// `r = |x| − k·u/4 ∈ [−u/8, u/8]` scaled by `2π/u`.
    ///
    /// Returns `Err(OutOfDomain)` if `u = 0`.
    pub fn sin_unit<const B: Word>(
        &self,
        x: &Repr<B>,
        u: usize,
        mut cache: Option<&mut ConstCache>,
    ) -> FpResult<FBig<R, B>> {
        if u == 0 {
            return Err(FpError::OutOfDomain);
        }
        if x.is_infinite() {
            return Err(FpError::InfiniteInput);
        }
        assert_limited_precision(self.precision);

        // Exact cases (±0 lands in the quarter table's q = 0 row). Resolved outside the Ziv
        // loop: its containment test cannot certify an exact 0/±1/±1/2 under directed rounding.
        if let Some(j) = unit_residue(x, u, 12) {
            if j % 3 == 0 {
                // quarter table, keyed by q = (4x/u) mod 8 = (j/3) mod 8 — sin(q·π/2)
                let q = j / 3;
                return match q {
                    0 | 2 | 4 | 6 => {
                        Ok(Exact(FBig::<R, B>::new(Repr::zero_with_sign(x.sign()), *self)))
                    }
                    1 | 5 => Ok(FBig::<R, B>::ONE.with_precision(self.precision)),
                    3 | 7 => Ok(FBig::<R, B>::NEG_ONE.with_precision(self.precision)),
                    _ => unreachable!("q = 4x/u mod 8 covers all rows"),
                };
            } else if j % 2 == 1 {
                // sine sixths: j mod 12 ∈ {1, 5} → +1/2, {7, 11} → −1/2
                let positive = j % 12 == 1 || j % 12 == 5;
                return exact_ratio(self, if positive { IBig::ONE } else { -IBig::ONE }, 2);
            }
            // j even with 3 ∤ j: the cosine sixths — sine is ±√3/2 there, not exact.
        }

        // sin is odd: reduce and evaluate on |x|, apply the sign at the end.
        let reduced = reduce_unit_argument(x, u);
        let negative = x.sign() == Sign::Negative;
        self.ziv(50, |guard| {
            let work = Context::<mode::HalfEven>::new(self.precision + guard);
            let (t, quadrant) = unit_argument_ball(&work, &reduced, u, reborrow_cache(&mut cache))?;
            let val = match quadrant {
                Quadrant::First => work.sin_compute(&t)?,
                Quadrant::Second => work.cos_compute(&t)?,
                Quadrant::Third => work.sin_compute(&t)?.neg(),
                Quadrant::Fourth => work.cos_compute(&t)?.neg(),
            };
            let val = if negative { val.neg() } else { val };
            Ok(val.to_value_radius::<R>(&Context::<R>::new(work.precision)))
        })
    }

    /// Calculate the cosine of `2π·x/u` — the ×u family with `u = 2` (i.e. `cos(x·π)`).
    ///
    /// See [`cos_unit`](Self::cos_unit) for the semantics; this is a thin wrapper.
    pub fn cos_pi<const B: Word>(
        &self,
        x: &Repr<B>,
        cache: Option<&mut ConstCache>,
    ) -> FpResult<FBig<R, B>> {
        self.cos_unit(x, 2, cache)
    }

    /// Calculate the cosine of `2π·x/u` (see [`sin_unit`](Self::sin_unit) for the ×u
    /// semantics).
    ///
    /// # Methodology
    /// Exact whenever `12x/u` is an integer (integers map to `±1`, odd multiples of `u/4` to
    /// `+0`, the cosine sixths to `±1/2`); the general path is [`sin_unit`](Self::sin_unit)'s
    /// with the cos quadrant mapping. cos is even, so the result never depends on the sign
    /// of x.
    ///
    /// Returns `Err(OutOfDomain)` if `u = 0`.
    pub fn cos_unit<const B: Word>(
        &self,
        x: &Repr<B>,
        u: usize,
        mut cache: Option<&mut ConstCache>,
    ) -> FpResult<FBig<R, B>> {
        if u == 0 {
            return Err(FpError::OutOfDomain);
        }
        if x.is_infinite() {
            return Err(FpError::InfiniteInput);
        }
        assert_limited_precision(self.precision);

        // Exact cases — outside the Ziv loop, whose containment test cannot certify exact
        // values under directed rounding.
        if let Some(j) = unit_residue(x, u, 12) {
            if j % 3 == 0 {
                // quarter table, keyed by q = (4x/u) mod 8 — cos(q·π/2)
                let q = j / 3;
                return match q {
                    0 | 4 => Ok(FBig::<R, B>::ONE.with_precision(self.precision)),
                    2 | 6 => Ok(FBig::<R, B>::NEG_ONE.with_precision(self.precision)),
                    1 | 3 | 5 | 7 => Ok(Exact(FBig::<R, B>::new(Repr::zero(), *self))),
                    _ => unreachable!("q = 4x/u mod 8 covers all rows"),
                };
            } else if j % 2 == 0 {
                // cosine sixths: 6x/u mod 6 ∈ {1, 5} → +1/2, {2, 4} → −1/2; from
                // j = 12x/u mod 24: j mod 12 ∈ {2, 10} → +1/2, {4, 8} → −1/2.
                let m = j % 12;
                let positive = m == 2 || m == 10;
                return exact_ratio(self, if positive { IBig::ONE } else { -IBig::ONE }, 2);
            }
            // j odd (the sine sixths): cosine is ±√3/2 there, not exact.
        }

        let reduced = reduce_unit_argument(x, u);
        self.ziv(50, |guard| {
            let work = Context::<mode::HalfEven>::new(self.precision + guard);
            let (t, quadrant) = unit_argument_ball(&work, &reduced, u, reborrow_cache(&mut cache))?;
            let val = match quadrant {
                Quadrant::First => work.cos_compute(&t)?,
                Quadrant::Second => work.sin_compute(&t)?.neg(),
                Quadrant::Third => work.cos_compute(&t)?.neg(),
                Quadrant::Fourth => work.sin_compute(&t)?,
            };
            Ok(val.to_value_radius::<R>(&Context::<R>::new(work.precision)))
        })
    }

    /// Calculate both the sine and cosine of `2π·x/u` — the ×u family with `u = 2`.
    ///
    /// See [`sin_cos_unit`](Self::sin_cos_unit); this is a thin wrapper.
    pub fn sin_cos_pi<const B: Word>(
        &self,
        x: &Repr<B>,
        cache: Option<&mut ConstCache>,
    ) -> (FpResult<FBig<R, B>>, FpResult<FBig<R, B>>) {
        self.sin_cos_unit(x, 2, cache)
    }

    /// Calculate both the sine and cosine of `2π·x/u`.
    ///
    /// This is more efficient than calling [`sin_unit`](Self::sin_unit) and
    /// [`cos_unit`](Self::cos_unit) separately — except at the sixth-type exact points, where
    /// exactly one of the two results is exact (`±1/2` vs `±√3/2`) and the other is evaluated
    /// on its own.
    ///
    /// Returns `Err(OutOfDomain)` in both slots if `u = 0`.
    pub fn sin_cos_unit<const B: Word>(
        &self,
        x: &Repr<B>,
        u: usize,
        mut cache: Option<&mut ConstCache>,
    ) -> (FpResult<FBig<R, B>>, FpResult<FBig<R, B>>) {
        if u == 0 {
            return (Err(FpError::OutOfDomain), Err(FpError::OutOfDomain));
        }
        if x.is_infinite() {
            return (Err(FpError::InfiniteInput), Err(FpError::InfiniteInput));
        }
        assert_limited_precision(self.precision);

        // Exact cases — outside the Ziv loop, whose containment test cannot certify exact
        // values under directed rounding.
        if let Some(j) = unit_residue(x, u, 12) {
            if j % 3 == 0 {
                // both exact: the two quarter tables, keyed by q = (4x/u) mod 8
                let q = j / 3;
                let s = match q {
                    0 | 2 | 4 | 6 => {
                        Ok(Exact(FBig::<R, B>::new(Repr::zero_with_sign(x.sign()), *self)))
                    }
                    1 | 5 => Ok(FBig::<R, B>::ONE.with_precision(self.precision)),
                    3 | 7 => Ok(FBig::<R, B>::NEG_ONE.with_precision(self.precision)),
                    _ => unreachable!("q = 4x/u mod 8 covers all rows"),
                };
                let c = match q {
                    0 | 4 => Ok(FBig::<R, B>::ONE.with_precision(self.precision)),
                    2 | 6 => Ok(FBig::<R, B>::NEG_ONE.with_precision(self.precision)),
                    1 | 3 | 5 | 7 => Ok(Exact(FBig::<R, B>::new(Repr::zero(), *self))),
                    _ => unreachable!("q = 4x/u mod 8 covers all rows"),
                };
                return (s, c);
            } else {
                // sixth-type: one result is the exact ±1/2, the other (±√3/2) is evaluated on
                // its own (its own Ziv loop — the re-classification inside the callee lands on
                // the general path).
                if j % 2 == 1 {
                    let positive = j % 12 == 1 || j % 12 == 5;
                    let s = exact_ratio(self, if positive { IBig::ONE } else { -IBig::ONE }, 2);
                    let c = self.cos_unit(x, u, reborrow_cache(&mut cache));
                    return (s, c);
                }
                let m = j % 12;
                let positive = m == 2 || m == 10;
                let c = exact_ratio(self, if positive { IBig::ONE } else { -IBig::ONE }, 2);
                let s = self.sin_unit(x, u, reborrow_cache(&mut cache));
                return (s, c);
            }
        }

        // sin is odd (cos even): reduce and evaluate on |x|, apply the sign to sin at the end.
        let reduced = reduce_unit_argument(x, u);
        let negative = x.sign() == Sign::Negative;
        let (s, c) = self.ziv_pair(50, |guard| {
            let work = Context::<mode::HalfEven>::new(self.precision + guard);
            let (t, quadrant) = unit_argument_ball(&work, &reduced, u, reborrow_cache(&mut cache))?;
            let (sin_ball, cos_ball) = work.sin_cos_compute(&t)?;
            let (s, c) = match quadrant {
                Quadrant::First => (sin_ball, cos_ball),
                Quadrant::Second => (cos_ball, sin_ball.neg()),
                Quadrant::Third => (sin_ball.neg(), cos_ball.neg()),
                Quadrant::Fourth => (cos_ball.neg(), sin_ball),
            };
            let s = if negative { s.neg() } else { s };
            let ctx = Context::<R>::new(work.precision);
            Ok((s.to_value_radius::<R>(&ctx), c.to_value_radius::<R>(&ctx)))
        });
        (s, c)
    }

    /// Calculate the tangent of `2π·x/u` — the ×u family with `u = 2` (i.e. `tan(x·π)`).
    ///
    /// See [`tan_unit`](Self::tan_unit) for the semantics; this is a thin wrapper.
    pub fn tan_pi<const B: Word>(
        &self,
        x: &Repr<B>,
        cache: Option<&mut ConstCache>,
    ) -> FpResult<FBig<R, B>> {
        self.tan_unit(x, 2, cache)
    }

    /// Calculate the tangent of `2π·x/u` (see [`sin_unit`](Self::sin_unit) for the ×u
    /// semantics).
    ///
    /// # Methodology
    /// Every quarter-type argument (u a multiple of `u/4`) is exact: integers map to `±0`,
    /// quarter-integers to `±1`. The odd multiples of `u/4` are the poles: tan approaches
    /// `+∞` from one side and `−∞` from the other, so the sign of an infinity is unknown
    /// there — like `0/0`, the case is reported as [`FpError::Indeterminate`] instead of
    /// guessing a sign. Otherwise [`sin_unit`](Self::sin_unit)/[`cos_unit`](Self::cos_unit)
    /// share one series evaluation; near a pole the wide exponent range holds a large finite
    /// value, as for [`tan`](Self::tan).
    ///
    /// Returns `Err(OutOfDomain)` if `u = 0`.
    pub fn tan_unit<const B: Word>(
        &self,
        x: &Repr<B>,
        u: usize,
        mut cache: Option<&mut ConstCache>,
    ) -> FpResult<FBig<R, B>> {
        if u == 0 {
            return Err(FpError::OutOfDomain);
        }
        if x.is_infinite() {
            return Err(FpError::InfiniteInput);
        }
        assert_limited_precision(self.precision);

        // tan hits an exact value at every eighth-of-turn argument (`8x/u` an integer) — the
        // whole table resolves before the Ziv loop (its containment test cannot certify exact
        // 0/±1 under directed rounding). The poles (odd multiples of u/4, m ≡ 2 or 6 mod 8) are
        // indeterminate: the two one-sided limits are +∞ and −∞, so no signed infinity can be
        // certified — unlike 1/0, whose sign is well-defined.
        if let Some(j) = unit_residue(x, u, 8) {
            let m = j % 8; // the eighth-of-turn index (mod its period 8)
            return match m {
                0 => Ok(Exact(FBig::<R, B>::new(Repr::zero_with_sign(x.sign()), *self))),
                1 | 5 => Ok(FBig::<R, B>::ONE.with_precision(self.precision)),
                2 | 6 => Err(FpError::Indeterminate),
                3 | 7 => Ok(FBig::<R, B>::NEG_ONE.with_precision(self.precision)),
                4 => Ok(Exact(FBig::<R, B>::new(Repr::neg_zero(), *self))),
                _ => unreachable!("m = 8x/u mod 8 covers all rows"),
            };
        }

        // tan is odd: reduce and evaluate on |x|, apply the sign at the end.
        let reduced = reduce_unit_argument(x, u);
        let negative = x.sign() == Sign::Negative;
        self.ziv(50, |guard| {
            let work = Context::<mode::HalfEven>::new(self.precision + guard);
            let (t, quadrant) = unit_argument_ball(&work, &reduced, u, reborrow_cache(&mut cache))?;
            let (sin_ball, cos_ball) = work.sin_cos_compute(&t)?;
            let (s, c) = match quadrant {
                Quadrant::First => (sin_ball, cos_ball),
                Quadrant::Second => (cos_ball, sin_ball.neg()),
                Quadrant::Third => (sin_ball.neg(), cos_ball.neg()),
                Quadrant::Fourth => (cos_ball.neg(), sin_ball),
            };
            if c.mid.significand.is_zero() {
                // cos rounded to a zero significand at this guard — unreachable here (the
                // exact poles were resolved by the table, and |t| ≤ π/4 keeps cos ≥ √2/2);
                // force a retry to stay on the safe side.
                return Ok((FBig::<R, B>::ZERO, FBig::<R, B>::ONE));
            }
            let val = s.div(&c, work.precision)?;
            let val = if negative { val.neg() } else { val };
            Ok(val.to_value_radius::<R>(&Context::<R>::new(work.precision)))
        })
    }

    /// Calculate `asin(x)·u/(2π)` — the arc sine expressed in units of the full turn divided
    /// by `u` (e.g. `u = 360` gives degrees). The inverse of [`sin_unit`](Self::sin_unit).
    ///
    /// # Methodology
    /// The exact-value rows resolve outside the Ziv loop (directed rounding cannot certify
    /// them there): `asin(±0) = ±0`, `asin(±1) = ±u/4`, and `asin(±1/2) = ±u/12`. The general
    /// path scales the [`Ball`](crate) composition of the radian [`asin`](Self::asin) by
    /// `u/(2π)`.
    ///
    /// `u = 0` is the limit `u·asin(x)/(2π) → ±0` (the signed zero, matching the function's
    /// oddness); returns `Err(OutOfDomain)` if `|x| > 1`.
    pub fn asin_unit<const B: Word>(
        &self,
        x: &Repr<B>,
        u: usize,
        mut cache: Option<&mut ConstCache>,
    ) -> FpResult<FBig<R, B>> {
        if x.is_infinite() {
            return Err(FpError::InfiniteInput);
        }
        assert_limited_precision(self.precision);
        if u == 0 {
            // the u → 0 limit: the signed zero (asin is odd)
            return signed_zero_normal(self, x);
        }
        if x.significand.is_zero() {
            // asin(±0) = ±0 (asin is odd), exact.
            return signed_zero_normal(self, x);
        }

        let x_orig = FBig::<R, B>::new(x.clone(), *self);
        // Domain check: |x| must be <= 1
        if x_orig.abs_cmp(&FBig::ONE).is_gt() {
            return Err(FpError::OutOfDomain);
        }
        // exact rows (±u/4, ±u/12), correctly rounded for any base by the exact ratio
        let cmp_one = x_orig.abs_cmp(&FBig::ONE);
        if cmp_one.is_eq() {
            let num = if x.sign() == Sign::Negative {
                -(IBig::from(u))
            } else {
                IBig::from(u)
            };
            return exact_ratio(self, num, 4);
        }
        if x_orig.abs_cmp(&(FBig::<R, B>::ONE / 2u8)).is_eq() {
            let num = if x.sign() == Sign::Negative {
                -(IBig::from(u))
            } else {
                IBig::from(u)
            };
            return exact_ratio(self, num, 12);
        }

        self.ziv(50, |guard| {
            let work = Context::<mode::HalfEven>::new(self.precision + guard);
            let x_ball = Ball::from_rounded(work.repr_round_ref(x), work.precision);
            let a = work.asin_ball::<B>(&x_ball, reborrow_cache(&mut cache))?;
            Ok(scale_unit(&work, &a, u, reborrow_cache(&mut cache))?
                .to_value_radius::<R>(&Context::<R>::new(work.precision)))
        })
    }

    /// Calculate `acos(x)·u/(2π)` — the arc cosine in units of the full turn divided by `u`.
    /// The inverse of [`cos_unit`](Self::cos_unit).
    ///
    /// # Methodology
    /// Uses the identity `acos(x) = π/2 − asin(x)` scaled by `u/(2π)`, i.e. `u/4 −
    /// asin_unit(x)`. Exact rows (outside the Ziv loop): `acos(+1) = 0`, `acos(−1) = u/2`,
    /// `acos(±0) = u/4`, `acos(±1/2) = u/6` / `u/3`.
    ///
    /// `u = 0` is the limit `u·acos(x)/(2π) → +0` (acos ≥ 0); returns `Err(OutOfDomain)` if
    /// `|x| > 1`.
    pub fn acos_unit<const B: Word>(
        &self,
        x: &Repr<B>,
        u: usize,
        mut cache: Option<&mut ConstCache>,
    ) -> FpResult<FBig<R, B>> {
        if x.is_infinite() {
            return Err(FpError::InfiniteInput);
        }
        assert_limited_precision(self.precision);
        if u == 0 {
            // the u → 0 limit: +0 (acos ≥ 0)
            return Ok(Exact(FBig::<R, B>::new(Repr::zero(), *self)));
        }

        let x_orig = FBig::<R, B>::new(x.clone(), *self);
        let cmp_one = x_orig.abs_cmp(&FBig::ONE);
        if cmp_one.is_gt() {
            return Err(FpError::OutOfDomain);
        }
        // exact rows — see asin_unit for why they stay outside the Ziv loop
        if cmp_one.is_eq() {
            return if x.sign() == Sign::Positive {
                Ok(Exact(FBig::<R, B>::new(Repr::zero(), *self)))
            } else {
                exact_ratio(self, IBig::from(u), 2)
            };
        }
        if x.significand.is_zero() {
            return exact_ratio(self, IBig::from(u), 4);
        }
        if x_orig.abs_cmp(&(FBig::<R, B>::ONE / 2u8)).is_eq() {
            return exact_ratio(
                self,
                if x.sign() == Sign::Positive {
                    IBig::from(u)
                } else {
                    IBig::from(2) * IBig::from(u)
                },
                6,
            );
        }

        // acos_unit(x) = u/4 − asin_unit(x): the composition avoids the catastrophic
        // cancellation `π/2 − asin(x)` would suffer near x = 1 once scaled.
        self.ziv(50, |guard| {
            let work = Context::<mode::HalfEven>::new(self.precision + guard);
            let x_ball = Ball::from_rounded(work.repr_round_ref(x), work.precision);
            let a = work.asin_ball::<B>(&x_ball, reborrow_cache(&mut cache))?;
            let quarter = scale_unit(&work, &a, u, reborrow_cache(&mut cache))?;
            // u/4 as an exact ball (held at u's own digit width — see unit_argument_ball)
            let u_digits = digit_len::<B>(&IBig::from(u)).max(work.precision);
            let wider = u_digits.max(2);
            let u_over_4 = Ball::exact_int(IBig::from(u), u_digits)
                .div(&Ball::exact_int(IBig::from(4), wider), wider)?;
            Ok(u_over_4
                .sub(&quarter, wider)?
                .to_value_radius::<R>(&Context::<R>::new(work.precision)))
        })
    }

    /// Calculate `atan(x)·u/(2π)` — the arc tangent in units of the full turn divided by `u`.
    /// The inverse of [`tan_unit`](Self::tan_unit).
    ///
    /// # Methodology
    /// Exact rows (outside the Ziv loop): `atan(±0) = ±0`, `atan(±1) = ±u/8`,
    /// `atan(±inf) = ±u/4`. The general path scales the radian [`atan`](Self::atan)
    /// composition by `u/(2π)`.
    ///
    /// `u = 0` is the limit `u·atan(x)/(2π) → ±0` (the signed zero, matching the oddness).
    pub fn atan_unit<const B: Word>(
        &self,
        x: &Repr<B>,
        u: usize,
        mut cache: Option<&mut ConstCache>,
    ) -> FpResult<FBig<R, B>> {
        if x.is_infinite() {
            // atan(±inf) = ±u/4 — the scaled form of ±π/2
            if u == 0 {
                return signed_zero_normal(self, x);
            }
            let num = if x.sign() == Sign::Negative {
                -(IBig::from(u))
            } else {
                IBig::from(u)
            };
            return exact_ratio(self, num, 4);
        }
        assert_limited_precision(self.precision);
        if u == 0 {
            return signed_zero_normal(self, x);
        }
        if x.significand.is_zero() {
            // atan(±0) = ±0
            return signed_zero_normal(self, x);
        }
        // atan(±1) = ±u/8 — outside the Ziv loop
        if FBig::<R, B>::new(x.clone(), *self)
            .abs_cmp(&FBig::ONE)
            .is_eq()
        {
            let num = if x.sign() == Sign::Negative {
                -(IBig::from(u))
            } else {
                IBig::from(u)
            };
            return exact_ratio(self, num, 8);
        }

        self.ziv(50, |guard| {
            let work = Context::<mode::HalfEven>::new(self.precision + guard);
            let x_ball = Ball::from_rounded(work.repr_round_ref(x), work.precision);
            let a = work.atan_ball::<B>(&x_ball, reborrow_cache(&mut cache))?;
            Ok(scale_unit(&work, &a, u, reborrow_cache(&mut cache))?
                .to_value_radius::<R>(&Context::<R>::new(work.precision)))
        })
    }

    /// Calculate `atan2(y, x)·u/(2π)` — the four-quadrant arc tangent in units of the full
    /// turn divided by `u`. The inverse of [`tan_unit`](Self::tan_unit) with quadrant
    /// disambiguation.
    ///
    /// # Methodology
    /// The axis/infinity special values follow the same C99 signed-zero model as
    /// [`atan2`](Self::atan2), with the π-multiples mapped to their `u`-fractions (`π/4 →
    /// u/8`, `π/2 → u/4`, `3π/4 → 3u/8`, `π → u/2`). The general path scales the radian
    /// atan2 by `u/(2π)`.
    ///
    /// `u = 0` is the limit `u·atan2(y, x)/(2π) → ±0` (the sign of the angle). Returns
    /// `Err(OutOfDomain)` if both arguments are zero.
    pub fn atan2_unit<const B: Word>(
        &self,
        y: &Repr<B>,
        x: &Repr<B>,
        u: usize,
        mut cache: Option<&mut ConstCache>,
    ) -> FpResult<FBig<R, B>> {
        if y.is_finite() && x.is_finite() && y.significand.is_zero() && x.significand.is_zero() {
            return Err(FpError::OutOfDomain);
        }
        if u == 0 {
            // the u → 0 limit: the signed zero carrying the angle's sign (atan2's sign is y's)
            return Ok(Exact(FBig::<R, B>::new(Repr::zero_with_sign(y.sign()), *self)));
        }

        assert_limited_precision(self.precision);

        // The `k·π/8` angles as exact `k·u/8` ratios (shared by the infinity table and the
        // finite axis/diagonal rows below).
        let u8th = |k: i8| -> FpResult<FBig<R, B>> {
            let num = IBig::from(u) * IBig::from(k.unsigned_abs());
            exact_ratio::<R, B>(self, if k < 0 { -num } else { num }, 8)
        };

        // Infinities, per the C99 model — the π-multiples map to u-fractions.
        if y.is_infinite() || x.is_infinite() {
            let (sy, sx) = (y.sign() == Sign::Positive, x.sign() == Sign::Positive);
            return match (y.is_infinite(), x.is_infinite(), sy, sx) {
                (true, true, true, true) => u8th(1),   // atan2(+inf, +inf) = π/4
                (true, true, true, false) => u8th(3),  // 3π/4
                (true, true, false, true) => u8th(-1), // −π/4
                (true, true, false, false) => u8th(-3), // −3π/4
                (true, false, true, _) => u8th(2),     // atan2(+inf, finite) = π/2
                (true, false, false, _) => u8th(-2),   // −π/2
                (false, true, _, true) => {
                    // atan2(±finite, +inf) = ±0 (signed zero of y)
                    Ok(Exact(FBig::<R, B>::new(Repr::zero_with_sign(y.sign()), *self)))
                }
                (false, true, true, false) => u8th(4), // atan2(+y, −inf) = π
                (false, true, false, false) => u8th(-4), // −π
                _ => unreachable!(),
            };
        }

        // The finite axis/diagonal rows, where atan2(y, x) is an exact multiple of π/4 (so the
        // scaled result `k·u/8` is exactly representable). Like the other exact rows they must
        // stay outside the Ziv loop: an exactly-representable result has a one-sided rounding
        // preimage under the directed modes, which no ball interval can certify.
        if y.significand.is_zero() {
            // y = ±0, x ≠ 0 (both-zero was screened): atan2(±0, +x) = ±0 (C99 signed zero),
            // atan2(±0, −x) = ±π → the u/2 row with the sign of y.
            return if x.sign() == Sign::Positive {
                Ok(Exact(FBig::<R, B>::new(Repr::zero_with_sign(y.sign()), *self)))
            } else {
                u8th(if y.sign() == Sign::Positive { 4 } else { -4 })
            };
        }
        if x.significand.is_zero() {
            // x = 0, y nonzero: atan2 = ±π/2 → ±u/4.
            return u8th(if y.sign() == Sign::Positive { 2 } else { -2 });
        }
        // |y| == |x| (finite, nonzero): the diagonals ±π/4, ±3π/4.
        let y_abs =
            FBig::<R, B>::new(Repr::new(y.significand.clone().abs(), y.exponent), Context::new(0));
        let x_abs =
            FBig::<R, B>::new(Repr::new(x.significand.clone().abs(), x.exponent), Context::new(0));
        if y_abs.abs_cmp(&x_abs).is_eq() {
            let same_half = (y.sign() == Sign::Positive) == (x.sign() == Sign::Positive);
            return u8th(match (y.sign() == Sign::Positive, same_half) {
                (true, true) => 1,   // (+y, +x): π/4
                (true, false) => 3,  // (+y, −x): 3π/4
                (false, true) => -1, // (−y, +x): −π/4
                (false, false) => -3,
            });
        }

        // x ≠ 0, finite: atan2(y/x) ± (quadrant π), scaled by u/(2π). The inner atan2 is
        // itself correctly rounded at the working precision; the scaling's rounding is
        // tracked by the Ball.
        self.ziv(50, |guard| {
            let work = Context::<mode::HalfEven>::new(self.precision + guard);
            let a = work.atan2::<B>(y, x, reborrow_cache(&mut cache))?;
            let a_ball = Ball::from_rounded(a.map(FBig::into_repr), work.precision);
            Ok(scale_unit(&work, &a_ball, u, reborrow_cache(&mut cache))?
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

    /// Calculate the sine of the floating point number multiplied by π, i.e. `sin(self·π)`.
    ///
    /// Unlike [`sin`](Self::sin), the ×π variant resolves rational fractions of π exactly:
    /// integers map to `±0` (with the sign of `self`) and half-integers to `±1`. The argument
    /// reduces exactly in integer arithmetic, so the accuracy is independent of the magnitude
    /// of `self`.
    ///
    /// # Examples
    /// ```
    /// # use core::str::FromStr;
    /// # use dashu_float::DBig;
    /// assert_eq!(DBig::from_str("0.5")?.sin_pi(), DBig::from(1));
    /// // sin(π·10^100) = 0 *exactly*: huge integer arguments lose nothing
    /// assert_eq!(DBig::from_str("1e100")?.sin_pi(), DBig::ZERO);
    /// # Ok::<(), dashu_base::ParseError>(())
    /// ```
    ///
    /// # Panics
    /// Panics if the input is infinite.
    #[inline]
    pub fn sin_pi(&self) -> Self {
        self.context
            .unwrap_fp(self.context.sin_pi(&self.repr, None))
    }

    /// Calculate the cosine of the floating point number multiplied by π, i.e. `cos(self·π)`.
    ///
    /// Unlike [`cos`](Self::cos), the ×π variant resolves rational fractions of π exactly:
    /// integers map to `±1` and odd half-integers to `+0`. The argument reduces exactly in
    /// integer arithmetic, so the accuracy is independent of the magnitude of `self`.
    ///
    /// # Examples
    /// ```
    /// # use core::str::FromStr;
    /// # use dashu_float::DBig;
    /// assert_eq!(DBig::from_str("0.5")?.cos_pi(), DBig::ZERO);
    /// assert_eq!(DBig::from_str("3")?.cos_pi(), DBig::from(-1));
    /// # Ok::<(), dashu_base::ParseError>(())
    /// ```
    ///
    /// # Panics
    /// Panics if the input is infinite.
    #[inline]
    pub fn cos_pi(&self) -> Self {
        self.context
            .unwrap_fp(self.context.cos_pi(&self.repr, None))
    }

    /// Calculate both the sine and cosine of the floating point number multiplied by π.
    ///
    /// This is more efficient than calling `sin_pi` and `cos_pi` separately. At half-integer
    /// arguments both results are exact.
    ///
    /// # Examples
    /// ```
    /// # use core::str::FromStr;
    /// # use dashu_float::DBig;
    /// let (s, c) = DBig::from_str("0.25")?.sin_cos_pi();
    /// // sin(π/4) = cos(π/4) = √2/2
    /// assert!(s == c);
    /// # Ok::<(), dashu_base::ParseError>(())
    /// ```
    ///
    /// # Panics
    /// Panics if the input is infinite.
    #[inline]
    pub fn sin_cos_pi(&self) -> (Self, Self) {
        let (s, c) = self.context.sin_cos_pi(&self.repr, None);
        (self.context.unwrap_fp(s), self.context.unwrap_fp(c))
    }

    /// Calculate the tangent of the floating point number multiplied by π, i.e. `tan(self·π)`.
    ///
    /// Every quarter-integer argument is exact: integers map to `±0` and quarter-integers to
    /// `±1`. At odd half-integers — the poles — the one-sided limits are `+∞` and `−∞`, so no
    /// signed infinity can be certified: the case is indeterminate (like `0/0`) and reported
    /// as an error at the context layer. The argument reduces exactly in integer arithmetic,
    /// so the accuracy is independent of the magnitude of `self`.
    ///
    /// # Examples
    /// ```
    /// # use core::str::FromStr;
    /// # use dashu_float::DBig;
    /// assert_eq!(DBig::from_str("0.25")?.tan_pi(), DBig::from(1));
    /// assert_eq!(DBig::from_str("1.75")?.tan_pi(), DBig::from(-1));
    /// # Ok::<(), dashu_base::ParseError>(())
    /// ```
    ///
    /// # Panics
    /// Panics if the input is infinite or an odd multiple of `1/2` (a pole, where the sign
    /// of the infinity is indeterminate).
    #[inline]
    pub fn tan_pi(&self) -> Self {
        self.context
            .unwrap_fp(self.context.tan_pi(&self.repr, None))
    }

    /// Calculate the sine of `self·2π/u` — the angle `self` measured in units of the full
    /// turn divided by `u` (e.g. `u = 360` gives degrees).
    ///
    /// Unlike [`sin`](Self::sin), the ×u variant has rational special points: the argument
    /// reduces *exactly* mod u in integer arithmetic, so the accuracy is independent of the
    /// magnitude of `self`, and arguments where `12·self/u` is an integer resolve exactly
    /// (e.g. `sin_unit(90, 360)` is exactly `1`).
    ///
    /// # Examples
    /// ```
    /// # use core::str::FromStr;
    /// # use dashu_float::DBig;
    /// let deg = DBig::from_str("90")?;
    /// assert_eq!(deg.sin_unit(360), DBig::from(1));
    /// let thirty = DBig::from_str("30")?;
    /// assert_eq!(thirty.sin_unit(360), DBig::from_str("0.5")?);
    /// # Ok::<(), dashu_base::ParseError>(())
    /// ```
    ///
    /// # Panics
    /// Panics if the input is infinite or `u` is zero.
    #[inline]
    pub fn sin_unit(&self, u: usize) -> Self {
        self.context
            .unwrap_fp(self.context.sin_unit(&self.repr, u, None))
    }

    /// Calculate the cosine of `self·2π/u` (see [`sin_unit`](Self::sin_unit) for the ×u
    /// semantics).
    ///
    /// # Examples
    /// ```
    /// # use core::str::FromStr;
    /// # use dashu_float::DBig;
    /// let deg = DBig::from_str("60")?;
    /// assert_eq!(deg.cos_unit(360), DBig::from_str("0.5")?);
    /// # Ok::<(), dashu_base::ParseError>(())
    /// ```
    ///
    /// # Panics
    /// Panics if the input is infinite or `u` is zero.
    #[inline]
    pub fn cos_unit(&self, u: usize) -> Self {
        self.context
            .unwrap_fp(self.context.cos_unit(&self.repr, u, None))
    }

    /// Calculate both the sine and cosine of `self·2π/u` (see [`sin_unit`](Self::sin_unit)).
    ///
    /// This is more efficient than calling [`sin_unit`](Self::sin_unit) and
    /// [`cos_unit`](Self::cos_unit) separately, except at the sixth-type exact points where
    /// exactly one of the two results is exact.
    ///
    /// # Panics
    /// Panics if the input is infinite or `u` is zero.
    #[inline]
    pub fn sin_cos_unit(&self, u: usize) -> (Self, Self) {
        let (s, c) = self.context.sin_cos_unit(&self.repr, u, None);
        (self.context.unwrap_fp(s), self.context.unwrap_fp(c))
    }

    /// Calculate the tangent of `self·2π/u` (see [`sin_unit`](Self::sin_unit) for the ×u
    /// semantics).
    ///
    /// Every eighth-of-turn argument (`8·self/u` an integer) is exact; at the odd multiples
    /// of `u/4` — the poles — the one-sided limits are `+∞` and `−∞`: the case is
    /// indeterminate and reported as an error at the context layer.
    ///
    /// # Examples
    /// ```
    /// # use core::str::FromStr;
    /// # use dashu_float::DBig;
    /// let deg = DBig::from_str("45")?;
    /// assert_eq!(deg.tan_unit(360), DBig::from(1));
    /// # Ok::<(), dashu_base::ParseError>(())
    /// ```
    ///
    /// # Panics
    /// Panics if the input is infinite, `u` is zero, or `self` is an odd multiple of `u/4`
    /// (a pole).
    #[inline]
    pub fn tan_unit(&self, u: usize) -> Self {
        self.context
            .unwrap_fp(self.context.tan_unit(&self.repr, u, None))
    }

    /// Calculate `asin(self)·u/(2π)` — the arc sine in units of the full turn divided by `u`
    /// (the inverse of [`sin_unit`](Self::sin_unit)).
    ///
    /// # Examples
    /// ```
    /// # use core::str::FromStr;
    /// # use dashu_float::DBig;
    /// let half = DBig::from_str("0.5")?;
    /// assert_eq!(half.asin_unit(360), DBig::from(30)); // asin(1/2) = 30°
    /// # Ok::<(), dashu_base::ParseError>(())
    /// ```
    ///
    /// # Panics
    /// Panics if the input is infinite or `|self| > 1` (out of domain).
    #[inline]
    pub fn asin_unit(&self, u: usize) -> Self {
        self.context
            .unwrap_fp(self.context.asin_unit(&self.repr, u, None))
    }

    /// Calculate `acos(self)·u/(2π)` — the arc cosine in units of the full turn divided by
    /// `u` (the inverse of [`cos_unit`](Self::cos_unit)).
    ///
    /// # Panics
    /// Panics if the input is infinite or `|self| > 1` (out of domain).
    #[inline]
    pub fn acos_unit(&self, u: usize) -> Self {
        self.context
            .unwrap_fp(self.context.acos_unit(&self.repr, u, None))
    }

    /// Calculate `atan(self)·u/(2π)` — the arc tangent in units of the full turn divided by
    /// `u` (the inverse of [`tan_unit`](Self::tan_unit)). `atan_unit(±inf) = ±u/4`.
    ///
    /// # Examples
    /// ```
    /// # use core::str::FromStr;
    /// # use dashu_float::DBig;
    /// let one = DBig::from_str("1.0")?; // two digits: the result 45 needs precision ≥ 2
    /// assert_eq!(one.atan_unit(360), DBig::from(45)); // atan(1) = 45°
    /// # Ok::<(), dashu_base::ParseError>(())
    /// ```
    #[inline]
    pub fn atan_unit(&self, u: usize) -> Self {
        self.context
            .unwrap_fp(self.context.atan_unit(&self.repr, u, None))
    }

    /// Calculate `atan2(self, x)·u/(2π)` — the four-quadrant arc tangent in units of the
    /// full turn divided by `u`. Follows the same C99 signed-zero model as
    /// [`atan2`](Self::atan2).
    ///
    /// # Panics
    /// Panics if both arguments are zero.
    #[inline]
    pub fn atan2_unit(&self, x: &Self, u: usize) -> Self {
        self.context
            .unwrap_fp(self.context.atan2_unit(&self.repr, &x.repr, u, None))
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

    /// Re-round a high-precision `HalfEven` oracle to precision `p` under the mode under test.
    /// The +60 guard bits put the oracle far from any rounding boundary, so the re-round is
    /// the correctly rounded value for the fixed inputs used here.
    fn reround<R: Round, const B: Word>(hi: &FBig<mode::HalfEven, B>, p: usize) -> FBig<R, B> {
        let ctx = Context::<R>::new(p);
        FBig::new(ctx.repr_round_ref(hi.repr()).value(), ctx)
    }

    /// Assert the value is a zero and report its sign; asserts non-zero-ness by construction.
    fn zero_sign<R: Round, const B: Word>(v: &FBig<R, B>) -> bool {
        assert!(v.repr().significand.is_zero());
        v.repr().is_neg_zero()
    }

    /// Assert the value is ±1 and return it as a sign-carrying integer.
    fn plus_minus_one<R: Round, const B: Word>(v: &FBig<R, B>) -> IBig {
        let i = IBig::try_from(v.clone()).unwrap();
        assert!(i == IBig::from(1) || i == IBig::from(-1));
        i
    }

    /// The ×π exact-case table: quarter-integer arguments resolve exactly at every precision
    /// and rounding mode (they bypass the Ziv loop, whose containment test cannot certify an
    /// exact 0/±1 under directed rounding).
    #[test]
    fn test_pi_family_exact_cases() {
        fn check<R: ErrorBounds>(mode_ctx: &Context<R>, p: usize) {
            // sin_pi: integers → ±0 with the sign of x; odd half-integers → ±1
            for (input, neg_zero, expect_one) in [
                ("0", Some(false), None),
                ("1", Some(false), None),
                ("-1", Some(true), None),
                ("2", Some(false), None),
                ("-3", Some(true), None),
                ("0.5", None, Some(true)),
                ("-0.5", None, Some(false)),
                ("1.5", None, Some(false)),
                ("2.5", None, Some(true)),
            ] {
                let x = DBig::from_str(input).unwrap();
                let s = mode_ctx.sin_pi::<10>(x.repr(), None).unwrap().value();
                match (neg_zero, expect_one) {
                    (Some(expected), _) => {
                        assert_eq!(zero_sign(&s), expected, "sin_pi({input}) sign of zero, p={p}")
                    }
                    (_, Some(positive)) => assert_eq!(
                        plus_minus_one(&s),
                        if positive {
                            IBig::from(1)
                        } else {
                            IBig::from(-1)
                        },
                        "sin_pi({input}) magnitude, p={p}"
                    ),
                    _ => panic!("test table row misconfigured"),
                }
            }

            // cos_pi: integers → ±1 by parity; odd half-integers → +0
            for (input, neg_one, is_zero) in [
                ("0", false, false),
                ("1", true, false),
                ("2", false, false),
                ("3", true, false),
                ("0.5", false, true),
                ("1.5", false, true),
                ("-0.5", false, true),
                ("2.5", false, true),
            ] {
                let x = DBig::from_str(input).unwrap();
                let c = mode_ctx.cos_pi::<10>(x.repr(), None).unwrap().value();
                if is_zero {
                    assert!(!zero_sign(&c), "cos_pi({input}) is +0, p={p}");
                } else {
                    let expect = if neg_one {
                        IBig::from(-1)
                    } else {
                        IBig::from(1)
                    };
                    assert_eq!(plus_minus_one(&c), expect, "cos_pi({input}), p={p}");
                }
            }

            // tan_pi: integers → +0 (even, sign of x) or −0 (odd, unconditional);
            // quarter-integers → ±1; odd half-integers are indeterminate poles
            for (input, neg_zero, expect_one) in [
                ("0", Some(false), None),
                ("1", Some(true), None),
                ("2", Some(false), None),
                ("0.25", None, Some(true)),
                ("0.75", None, Some(false)),
                ("1.25", None, Some(true)),
                ("1.75", None, Some(false)),
                ("-0.25", None, Some(false)),
                ("-0.75", None, Some(true)),
            ] {
                let x = DBig::from_str(input).unwrap();
                let t = mode_ctx.tan_pi::<10>(x.repr(), None).unwrap().value();
                match (neg_zero, expect_one) {
                    (Some(expected), _) => {
                        assert_eq!(zero_sign(&t), expected, "tan_pi({input}), p={p}")
                    }
                    (_, Some(positive)) => assert_eq!(
                        plus_minus_one(&t),
                        if positive {
                            IBig::from(1)
                        } else {
                            IBig::from(-1)
                        },
                        "tan_pi({input}), p={p}"
                    ),
                    _ => panic!("test table row misconfigured"),
                }
            }
            for input in ["0.5", "1.5", "-0.5", "2.5"] {
                let x = DBig::from_str(input).unwrap();
                assert!(
                    matches!(mode_ctx.tan_pi::<10>(x.repr(), None), Err(FpError::Indeterminate)),
                    "tan_pi({input}) is an indeterminate pole, p={p}"
                );
            }

            // sin_cos_pi agrees with the two tables at half-integers
            let x = DBig::from_str("-0.5").unwrap();
            let (s, c) = mode_ctx.sin_cos_pi::<10>(x.repr(), None);
            assert_eq!(plus_minus_one(&s.unwrap().value()), IBig::from(-1));
            assert!(!zero_sign(&c.unwrap().value()));

            // the string parser collapses "-0" to +0, so probe the −0 sentinel directly
            let s = mode_ctx
                .sin_pi::<10>(&Repr::neg_zero(), None)
                .unwrap()
                .value();
            assert!(zero_sign(&s), "sin_pi(-0) is -0");
            let t = mode_ctx
                .tan_pi::<10>(&Repr::neg_zero(), None)
                .unwrap()
                .value();
            assert!(zero_sign(&t), "tan_pi(-0) is -0");
            let (s, _) = mode_ctx.sin_cos_pi::<10>(&Repr::neg_zero(), None);
            assert!(zero_sign(&s.unwrap().value()), "sin_cos_pi(-0).0 is -0");
        }

        for &p in &[20usize, 50, 100, 500] {
            check(&Context::<mode::HalfEven>::new(p), p);
            check(&Context::<mode::Down>::new(p), p);
            check(&Context::<mode::Up>::new(p), p);
        }
    }

    /// The ×π general path vs the high-precision oracle (p + 60 under `HalfEven`, re-rounded
    /// under the mode under test), across the four canonical significand widths. The inputs
    /// cover every reduction branch: tiny arguments (k = 0 product path), the rational split,
    /// the quarter-integer ties (odd `n`, so `sin_pi`/`cos_pi` still take the general path),
    /// large integer parts, and negative arguments.
    #[test]
    fn test_pi_family_oracle() {
        let inputs = [
            "0.1",
            "0.2",
            "0.3",
            "0.4",
            "0.6",
            "0.7",
            "0.8",
            "0.9",
            "1.1",
            "1.2",
            "2.3",
            "2.6",
            "-0.3",
            "-1.2",
            "-2.6",
            "10.1",
            "123.456",
            "12345678901234567890.125",
        ];
        for &p in &[20usize, 50, 100, 500] {
            for input in inputs {
                let x = DBig::from_str(input).unwrap();
                let hi_ctx = Context::<mode::HalfEven>::new(p + 60);
                let hi_s = hi_ctx.sin_pi::<10>(x.repr(), None).unwrap().value();
                let hi_c = hi_ctx.cos_pi::<10>(x.repr(), None).unwrap().value();
                let hi_t = hi_ctx.tan_pi::<10>(x.repr(), None).unwrap().value();

                // sin under HalfEven
                let got = Context::<mode::HalfEven>::new(p)
                    .sin_pi::<10>(x.repr(), None)
                    .unwrap()
                    .value();
                assert_eq!(
                    got,
                    reround::<mode::HalfEven, 10>(&hi_s, p),
                    "sin_pi({input}) at p={p}"
                );
                // cos under Down
                let got = Context::<mode::Down>::new(p)
                    .cos_pi::<10>(x.repr(), None)
                    .unwrap()
                    .value();
                assert_eq!(
                    got,
                    reround::<mode::Down, 10>(&hi_c, p),
                    "cos_pi({input}) at p={p} under Down"
                );
                // tan under Up
                let got = Context::<mode::Up>::new(p)
                    .tan_pi::<10>(x.repr(), None)
                    .unwrap()
                    .value();
                assert_eq!(
                    got,
                    reround::<mode::Up, 10>(&hi_t, p),
                    "tan_pi({input}) at p={p} under Up"
                );
                // sin_cos under HalfEven agrees with the separate evaluations
                let (s, c) = Context::<mode::HalfEven>::new(p).sin_cos_pi::<10>(x.repr(), None);
                assert_eq!(
                    s.unwrap().value(),
                    reround::<mode::HalfEven, 10>(&hi_s, p),
                    "sin_cos_pi sin({input}) at p={p}"
                );
                assert_eq!(
                    c.unwrap().value(),
                    reround::<mode::HalfEven, 10>(&hi_c, p),
                    "sin_cos_pi cos({input}) at p={p}"
                );
            }
        }
    }

    /// The exact reduction mod 2 holds for arbitrarily large arguments: a huge integer part
    /// plus a `2^-10` residue must give exactly the same result as the bare residue (the
    /// radian `sin` would need ~1000 extra guard digits here), and huge integers hit the
    /// exact-case table through the no-materialization path.
    #[test]
    fn test_sin_pi_huge_exact_reduction() {
        let p = 100;
        let ctx = Context::<mode::HalfEven>::new(p);

        // x = ((2^60 + 1)·2^950 + 1)·2^-10 ≡ 2^-10 (mod 2)
        let sig = (((IBig::from(1) << 60usize) + 1) << 950usize) + 1;
        let x = Repr::<2>::new(sig, -10);
        let residue = Repr::<2>::new(IBig::from(1), -10);
        assert_eq!(
            ctx.sin_pi::<2>(&x, None).unwrap().value(),
            ctx.sin_pi::<2>(&residue, None).unwrap().value()
        );
        assert_eq!(
            ctx.cos_pi::<2>(&x, None).unwrap().value(),
            ctx.cos_pi::<2>(&residue, None).unwrap().value()
        );

        // an even integer at a huge exponent: quarter_class must not materialize 2^940
        let huge_int = Repr::<2>::new((IBig::from(1) << 60usize) + 1, 940);
        assert_eq!(ctx.sin_pi::<2>(&huge_int, None).unwrap().value(), FBig::<mode::HalfEven>::ZERO);
        assert_eq!(ctx.cos_pi::<2>(&huge_int, None).unwrap().value(), FBig::<mode::HalfEven>::ONE);
        // ...and an odd one at a small exponent: cos_pi(2^60+1) = -1 (odd integer)
        assert_eq!(
            ctx.cos_pi::<2>(&Repr::<2>::new((IBig::from(1) << 60usize) + 1, 0), None)
                .unwrap()
                .value(),
            FBig::<mode::HalfEven>::NEG_ONE
        );
    }

    /// An astronomically-scaled tiny argument (`|x| < 1/4` with s ~ 10⁹) must take the
    /// k = 0 fast path — materializing `B^s` here would allocate ~125 MB — and still round
    /// correctly against the oracle.
    // The 10⁹-scale exponent needs the 64-bit `isize` range; on 32-bit targets the underflow
    // guard fires first (its range is ~4000× smaller), so the test is 64-bit-only.
    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_sin_pi_tiny_no_materialization() {
        let x = Repr::<2>::new(IBig::from(3), -1_000_000_000);
        for &p in &[20usize, 50, 100, 500] {
            let ctx = Context::<mode::HalfEven>::new(p);
            let s = ctx.sin_pi::<2>(&x, None).unwrap().value();
            assert_eq!(s.sign(), Sign::Positive);

            let hi = Context::<mode::HalfEven>::new(p + 60)
                .sin_pi::<2>(&x, None)
                .unwrap()
                .value();
            assert_eq!(s, reround::<mode::HalfEven, 2>(&hi, p));
        }
    }

    /// `tan_pi` just off a pole: large positive below, large negative above (the poles
    /// themselves are `Indeterminate`).
    #[test]
    fn test_tan_pi_near_pole_signs() {
        let p = 53;
        let ctx = Context::<mode::HalfEven>::new(p);
        let eps = FBig::<mode::HalfEven>::ONE >> 10;
        // both operands must carry the context precision: at precision 1 the subtraction
        // 0.5 − 2^-10 rounds back onto the exact pole
        let half: FBig<mode::HalfEven> = FBig::ONE.with_precision(p).value() / 2u8;
        let eps: FBig<mode::HalfEven> = eps.with_precision(p).value();
        let below = ctx
            .tan_pi::<2>((half.clone() - &eps).repr(), None)
            .unwrap()
            .value();
        let above = ctx
            .tan_pi::<2>((half.clone() + &eps).repr(), None)
            .unwrap()
            .value();
        assert_eq!(below.sign(), Sign::Positive, "tan_pi just below 1/2");
        assert_eq!(above.sign(), Sign::Negative, "tan_pi just above 1/2");
    }

    /// The ×u forward family's exact-value tables, in degrees (u = 360 — quarters, sixths and
    /// the tangent eighths all appear) and at u ∈ {1, 4, 12}, across the canonical precision
    /// sweep and rounding modes.
    #[test]
    fn test_unit_family_exact_tables() {
        fn check<R: ErrorBounds>(mode_ctx: &Context<R>) {
            let x = |v: &str| DBig::from_str(v).unwrap();

            // degrees: quarters
            assert_eq!(
                mode_ctx
                    .sin_unit::<10>(x("90").repr(), 360, None)
                    .unwrap()
                    .value(),
                DBig::from(1)
            );
            assert_eq!(
                mode_ctx
                    .sin_unit::<10>(x("270").repr(), 360, None)
                    .unwrap()
                    .value(),
                -DBig::from(1)
            );
            let zero = mode_ctx
                .sin_unit::<10>(x("180").repr(), 360, None)
                .unwrap()
                .value();
            assert!(zero.repr().significand.is_zero() && !zero.repr().is_neg_zero());
            assert_eq!(
                mode_ctx
                    .cos_unit::<10>(x("90").repr(), 360, None)
                    .unwrap()
                    .value(),
                DBig::ZERO
            );
            assert_eq!(
                mode_ctx
                    .cos_unit::<10>(x("180").repr(), 360, None)
                    .unwrap()
                    .value(),
                -DBig::from(1)
            );
            // degrees: sixths
            assert_eq!(
                mode_ctx
                    .sin_unit::<10>(x("30").repr(), 360, None)
                    .unwrap()
                    .value(),
                DBig::from_str("0.5").unwrap()
            );
            assert_eq!(
                mode_ctx
                    .sin_unit::<10>(x("210").repr(), 360, None)
                    .unwrap()
                    .value(),
                DBig::from_str("-0.5").unwrap()
            );
            assert_eq!(
                mode_ctx
                    .cos_unit::<10>(x("60").repr(), 360, None)
                    .unwrap()
                    .value(),
                DBig::from_str("0.5").unwrap()
            );
            assert_eq!(
                mode_ctx
                    .cos_unit::<10>(x("120").repr(), 360, None)
                    .unwrap()
                    .value(),
                DBig::from_str("-0.5").unwrap()
            );
            // degrees: tangent eighths and poles
            assert_eq!(
                mode_ctx
                    .tan_unit::<10>(x("45").repr(), 360, None)
                    .unwrap()
                    .value(),
                DBig::from(1)
            );
            assert_eq!(
                mode_ctx
                    .tan_unit::<10>(x("135").repr(), 360, None)
                    .unwrap()
                    .value(),
                -DBig::from(1)
            );
            let t180 = mode_ctx
                .tan_unit::<10>(x("180").repr(), 360, None)
                .unwrap()
                .value();
            assert!(t180.repr().is_neg_zero(), "tan_unit(180°) is -0");
            for pole in ["90", "270"] {
                assert!(matches!(
                    mode_ctx.tan_unit::<10>(x(pole).repr(), 360, None),
                    Err(FpError::Indeterminate)
                ));
            }
            // u = 1: sin_unit(x, 1) = sin(2πx), same table shape in u/4 = 0.25 steps
            assert_eq!(
                mode_ctx
                    .sin_unit::<10>(x("0.25").repr(), 1, None)
                    .unwrap()
                    .value(),
                DBig::from(1)
            );
            assert_eq!(
                mode_ctx
                    .sin_unit::<10>(x("0.50").repr(), 1, None)
                    .unwrap()
                    .value(),
                DBig::ZERO
            );
            // u = 12: x = 1 = u/12 (sine sixth), x = 2 = u/6 (cosine sixth), x = 3 = u/4
            assert_eq!(
                mode_ctx
                    .sin_unit::<10>(x("1").repr(), 12, None)
                    .unwrap()
                    .value(),
                DBig::from_str("0.5").unwrap()
            );
            assert_eq!(
                mode_ctx
                    .cos_unit::<10>(x("2").repr(), 12, None)
                    .unwrap()
                    .value(),
                DBig::from_str("0.5").unwrap() // cos(π/3) = +1/2
            );
            assert_eq!(
                mode_ctx
                    .cos_unit::<10>(x("3").repr(), 12, None)
                    .unwrap()
                    .value(),
                DBig::ZERO
            );
            // sin_cos_unit agrees with the separate evaluations, including at the
            // sixth-type points (one exact, one general)
            for deg in ["30", "45", "60", "90", "100", "210"] {
                let xd = x(deg);
                let (s, c) = mode_ctx.sin_cos_unit::<10>(xd.repr(), 360, None);
                assert_eq!(
                    s.unwrap().value(),
                    mode_ctx
                        .sin_unit::<10>(xd.repr(), 360, None)
                        .unwrap()
                        .value(),
                    "sin_cos_unit sin {deg}°"
                );
                assert_eq!(
                    c.unwrap().value(),
                    mode_ctx
                        .cos_unit::<10>(xd.repr(), 360, None)
                        .unwrap()
                        .value(),
                    "sin_cos_unit cos {deg}°"
                );
            }
            // u = 0 is out of domain for the forward family
            let r1 = mode_ctx.sin_unit::<10>(x("1").repr(), 0, None);
            let r2 = mode_ctx.cos_unit::<10>(x("1").repr(), 0, None);
            let r3 = mode_ctx.tan_unit::<10>(x("1").repr(), 0, None);
            let (r4s, r4c) = mode_ctx.sin_cos_unit::<10>(x("1").repr(), 0, None);
            for (i, r) in [r1, r2, r3, r4s, r4c].into_iter().enumerate() {
                assert!(
                    matches!(r.map(|_| ()), Err(FpError::OutOfDomain)),
                    "u = 0 is out of domain (slot {i})"
                );
            }
        }

        for &p in &[20usize, 50, 100, 500] {
            check(&Context::<mode::HalfEven>::new(p));
            check(&Context::<mode::Down>::new(p));
            check(&Context::<mode::Up>::new(p));
        }
    }

    /// The ×π family is exactly the ×u family at u = 2: every value agrees.
    #[test]
    fn test_pi_family_equals_unit_family() {
        for input in [
            "0.1", "0.25", "0.3", "0.5", "1.2", "2.5", "-0.75", "10.1", "1e50",
        ] {
            let x = DBig::from_str(input).unwrap();
            for &p in &[20usize, 100] {
                let ctx = Context::<mode::HalfEven>::new(p);
                assert_eq!(
                    ctx.sin_pi::<10>(x.repr(), None).unwrap().value(),
                    ctx.sin_unit::<10>(x.repr(), 2, None).unwrap().value(),
                    "sin_pi == sin_unit(2), x={input}"
                );
                assert_eq!(
                    ctx.cos_pi::<10>(x.repr(), None).unwrap().value(),
                    ctx.cos_unit::<10>(x.repr(), 2, None).unwrap().value()
                );
                let t_pi = ctx.tan_pi::<10>(x.repr(), None);
                let t_u = ctx.tan_unit::<10>(x.repr(), 2, None);
                assert!(
                    t_pi.is_err() == t_u.is_err()
                        && t_pi.map(|v| v.value()) == t_u.map(|v| v.value())
                );
                let (s_pi, c_pi) = ctx.sin_cos_pi::<10>(x.repr(), None);
                let (s_u, c_u) = ctx.sin_cos_unit::<10>(x.repr(), 2, None);
                assert_eq!(s_pi.unwrap().value(), s_u.unwrap().value());
                assert_eq!(c_pi.unwrap().value(), c_u.unwrap().value());
            }
        }
    }

    /// The inverse ×u family: exact rows (in degrees), the u = 0 limits, and the general path
    /// against the high-precision oracle.
    #[test]
    fn test_inverse_unit_family() {
        fn check<R: ErrorBounds>(mode_ctx: &Context<R>) {
            let x = |v: &str| DBig::from_str(v).unwrap();
            let u = 360usize;

            // exact rows
            assert_eq!(
                mode_ctx
                    .asin_unit::<10>(x("1").repr(), u, None)
                    .unwrap()
                    .value(),
                DBig::from_str("90").unwrap()
            );
            assert_eq!(
                mode_ctx
                    .asin_unit::<10>(x("-1").repr(), u, None)
                    .unwrap()
                    .value(),
                DBig::from_str("-90").unwrap()
            );
            assert_eq!(
                mode_ctx
                    .asin_unit::<10>(x("0.5").repr(), u, None)
                    .unwrap()
                    .value(),
                DBig::from_str("30").unwrap()
            );
            assert_eq!(
                mode_ctx
                    .acos_unit::<10>(x("1").repr(), u, None)
                    .unwrap()
                    .value(),
                DBig::ZERO
            );
            assert_eq!(
                mode_ctx
                    .acos_unit::<10>(x("0").repr(), u, None)
                    .unwrap()
                    .value(),
                DBig::from_str("90").unwrap()
            );
            assert_eq!(
                mode_ctx
                    .acos_unit::<10>(x("-0.5").repr(), u, None)
                    .unwrap()
                    .value(),
                DBig::from_str("120").unwrap()
            );
            assert_eq!(
                mode_ctx
                    .atan_unit::<10>(x("1.0").repr(), u, None)
                    .unwrap()
                    .value(),
                DBig::from_str("45").unwrap()
            );
            assert_eq!(
                mode_ctx
                    .atan_unit::<10>(&Repr::infinity(), u, None)
                    .unwrap()
                    .value(),
                DBig::from_str("90").unwrap()
            );

            // atan2_unit's C99 table, in u-fractions
            let quarter = |a: &str, b: &str| {
                mode_ctx
                    .atan2_unit::<10>(x(a).repr(), x(b).repr(), u, None)
                    .unwrap()
                    .value()
            };
            assert_eq!(quarter("1.0", "1.0"), DBig::from_str("45").unwrap());
            assert_eq!(quarter("1.0", "-1.0"), DBig::from_str("135").unwrap());
            assert_eq!(quarter("0", "1"), DBig::ZERO);
            assert_eq!(quarter("1", "0"), DBig::from_str("90").unwrap());
            assert_eq!(quarter("0", "-1"), DBig::from_str("180").unwrap());
            assert_eq!(
                mode_ctx
                    .atan2_unit::<10>(&Repr::infinity(), &Repr::infinity(), u, None)
                    .unwrap()
                    .value(),
                DBig::from_str("45").unwrap()
            );
            assert_eq!(
                mode_ctx
                    .atan2_unit::<10>(&Repr::infinity(), &Repr::neg_infinity(), u, None)
                    .unwrap()
                    .value(),
                DBig::from_str("135").unwrap()
            );
            assert!(matches!(
                mode_ctx.atan2_unit::<10>(x("0").repr(), x("0").repr(), u, None),
                Err(FpError::OutOfDomain)
            ));

            // u = 0: the signed-zero limits
            assert!(mode_ctx
                .asin_unit::<10>(x("0.5").repr(), 0, None)
                .unwrap()
                .value()
                .repr()
                .significand
                .is_zero());
            assert!(mode_ctx
                .acos_unit::<10>(x("0.5").repr(), 0, None)
                .unwrap()
                .value()
                .repr()
                .is_pos_zero());

            // domain errors
            assert!(matches!(
                mode_ctx.asin_unit::<10>(x("1.5").repr(), u, None),
                Err(FpError::OutOfDomain)
            ));
            assert!(matches!(
                mode_ctx.acos_unit::<10>(x("-1.5").repr(), u, None),
                Err(FpError::OutOfDomain)
            ));
        }

        for &p in &[20usize, 50, 100, 500] {
            check(&Context::<mode::HalfEven>::new(p));
            check(&Context::<mode::Down>::new(p));

            // general path vs the oracle, under two modes
            for input in ["0.1", "0.3", "0.7", "-0.2", "0.9"] {
                let x = DBig::from_str(input).unwrap();
                for &u in &[2usize, 7, 360] {
                    let hi = Context::<mode::HalfEven>::new(p + 60);
                    for op in 0..3 {
                        match op {
                            0 => {
                                let expect = reround::<mode::HalfEven, 10>(
                                    &hi.asin_unit::<10>(x.repr(), u, None).unwrap().value(),
                                    p,
                                );
                                assert_eq!(
                                    Context::<mode::HalfEven>::new(p)
                                        .asin_unit::<10>(x.repr(), u, None)
                                        .unwrap()
                                        .value(),
                                    expect,
                                    "asin_unit({input}, {u}) at p={p}"
                                );
                            }
                            1 => {
                                let expect = reround::<mode::Down, 10>(
                                    &hi.acos_unit::<10>(x.repr(), u, None).unwrap().value(),
                                    p,
                                );
                                assert_eq!(
                                    Context::<mode::Down>::new(p)
                                        .acos_unit::<10>(x.repr(), u, None)
                                        .unwrap()
                                        .value(),
                                    expect,
                                    "acos_unit({input}, {u}) at p={p} under Down"
                                );
                            }
                            _ => {
                                let expect = reround::<mode::HalfEven, 10>(
                                    &hi.atan_unit::<10>(x.repr(), u, None).unwrap().value(),
                                    p,
                                );
                                assert_eq!(
                                    Context::<mode::HalfEven>::new(p)
                                        .atan_unit::<10>(x.repr(), u, None)
                                        .unwrap()
                                        .value(),
                                    expect,
                                    "atan_unit({input}, {u}) at p={p}"
                                );
                            }
                        }
                    }
                }
            }

            // atan2_unit general path vs the oracle
            for (ya, xa) in [("1.2", "3.4"), ("-2.5", "0.7"), ("0.3", "-4.0")] {
                let y = DBig::from_str(ya).unwrap();
                let x = DBig::from_str(xa).unwrap();
                let hi = Context::<mode::HalfEven>::new(p + 60);
                let expect = reround::<mode::HalfEven, 10>(
                    &hi.atan2_unit::<10>(y.repr(), x.repr(), 360, None)
                        .unwrap()
                        .value(),
                    p,
                );
                assert_eq!(
                    Context::<mode::HalfEven>::new(p)
                        .atan2_unit::<10>(y.repr(), x.repr(), 360, None)
                        .unwrap()
                        .value(),
                    expect,
                    "atan2_unit({ya}, {xa}, 360) at p={p}"
                );
            }
        }

        // roundtrip: asin_unit(sin_unit(x)) ≈ x on the degrees scale, for x in the principal
        // range [−90°, 90°] (beyond it asin folds into the range)
        type HF = FBig<mode::HalfEven, 10>;
        let ctx = Context::<mode::HalfEven>::new(50);
        let tol = HF::from_parts(IBig::from(1), -40);
        for deg in ["10", "30", "45", "89.9", "90", "-42.7"] {
            let x = HF::from_str(deg).unwrap().with_precision(50).value();
            let s = ctx.sin_unit::<10>(x.repr(), 360, None).unwrap().value();
            let back = ctx.asin_unit::<10>(s.repr(), 360, None).unwrap().value();
            assert!(
                (x.clone() - back).abs().abs_cmp(&tol).is_le(),
                "asin_unit(sin_unit({deg}°)) roundtrip"
            );
        }
    }

    /// The ×u general path vs the high-precision oracle (p + 60 under `HalfEven`, re-rounded
    /// under the mode under test), across u values with no shared factors (7, 11), the
    /// composite 360, and the tight u = 1.
    #[test]
    fn test_unit_family_oracle() {
        let inputs = [
            "0.1", "0.2", "0.3", "0.4", "0.6", "0.7", "0.9", "1.1", "2.3", "-0.3", "-1.2", "10.1",
            "123.456", "1e20",
        ];
        for &p in &[20usize, 50, 100, 500] {
            for input in inputs {
                let x = DBig::from_str(input).unwrap();
                for &u in &[1usize, 7, 11, 360] {
                    let hi_ctx = Context::<mode::HalfEven>::new(p + 60);
                    let hi_s = hi_ctx.sin_unit::<10>(x.repr(), u, None).unwrap().value();
                    let hi_c = hi_ctx.cos_unit::<10>(x.repr(), u, None).unwrap().value();
                    let hi_t = match hi_ctx.tan_unit::<10>(x.repr(), u, None) {
                        Ok(v) => v.value(),
                        Err(_) => continue, // pole (pinpointed by the exact-table test)
                    };

                    let got = Context::<mode::HalfEven>::new(p)
                        .sin_unit::<10>(x.repr(), u, None)
                        .unwrap()
                        .value();
                    assert_eq!(
                        got,
                        reround::<mode::HalfEven, 10>(&hi_s, p),
                        "sin_unit({input}, {u}) at p={p}"
                    );
                    let got = Context::<mode::Down>::new(p)
                        .cos_unit::<10>(x.repr(), u, None)
                        .unwrap()
                        .value();
                    assert_eq!(
                        got,
                        reround::<mode::Down, 10>(&hi_c, p),
                        "cos_unit({input}, {u}) at p={p} under Down"
                    );
                    let got = Context::<mode::Up>::new(p)
                        .tan_unit::<10>(x.repr(), u, None)
                        .unwrap()
                        .value();
                    assert_eq!(
                        got,
                        reround::<mode::Up, 10>(&hi_t, p),
                        "tan_unit({input}, {u}) at p={p} under Up"
                    );
                }
            }
        }
    }

    /// Direct unit checks of the integer classification and reduction kernels, including an
    /// odd base (where the exact points beyond the integers are sparse, and the split's
    /// `r = num/den` is not representable as a base-B float).
    #[test]
    fn test_unit_class_and_reduction_kernels() {
        // classification j = (k·x/u) mod 2k, base 10. For u = 2 the k = 12 residue is always a
        // multiple of 3 (12x/2 = 6x — a half-integer), so the u = 2 rows only exercise the
        // quarter table; the degrees rows hit the sixths and the tangent eighths.
        let uc = |m: i64, e: isize, u: usize, k: usize| {
            unit_residue(&Repr::<10>::new(IBig::from(m), e), u, k)
        };
        // u = 2, k = 12: j = 6x mod 24
        assert_eq!(uc(5, -1, 2, 12), Some(3)); // 0.5 → 3 (q = 1: sin +1)
        assert_eq!(uc(25, -2, 2, 12), None); // 1.5 ∉ ℤ (sin(π/4) = √2/2, general)
        assert_eq!(uc(75, -2, 2, 12), None); // 4.5 ∉ ℤ
        assert_eq!(uc(-25, -2, 2, 12), None);
        assert_eq!(uc(3, 0, 2, 12), Some(18)); // 18 (q = 6: sin ±0)
        assert_eq!(uc(8, 0, 2, 12), Some(0)); // 48 ≡ 0
        assert_eq!(uc(1, 1, 2, 12), Some(12)); // 60 ≡ 12 (q = 4)
        assert_eq!(uc(1, -1, 2, 12), None); // 0.6 ∉ ℤ
        assert_eq!(uc(1, -1000, 2, 12), None); // huge s: rejected by bounds, no materialization
                                               // u = 2, k = 8: j = 4x mod 16
        assert_eq!(uc(5, -1, 2, 8), Some(2)); // 0.5 → 2 (m = 2: the tan pole)
        assert_eq!(uc(25, -2, 2, 8), Some(1)); // 0.25 → 1 (tan(π/4) = 1)
        assert_eq!(uc(75, -2, 2, 8), Some(3)); // 0.75 → 3 (tan(3π/4) = −1)
        assert_eq!(uc(15, -1, 2, 8), Some(6)); // 1.5 → 6 (m = 6: the tan pole)
                                               // degrees, k = 12: j = x/30 mod 24
        assert_eq!(uc(30, 0, 360, 12), Some(1)); // sin 30° = 1/2
        assert_eq!(uc(150, 0, 360, 12), Some(5)); // sin 150° = 1/2
        assert_eq!(uc(210, 0, 360, 12), Some(7)); // sin 210° = −1/2
        assert_eq!(uc(60, 0, 360, 12), Some(2)); // cos 60° = 1/2
        assert_eq!(uc(120, 0, 360, 12), Some(4)); // cos 120° = −1/2
        assert_eq!(uc(90, 0, 360, 12), Some(3)); // sin 90° = 1 (quarter row)
        assert_eq!(uc(45, 0, 360, 12), None); // sin 45° = √2/2 (general path)
        assert_eq!(uc(455, -1, 360, 12), None); // 45.5° — general path
                                                // degrees, k = 8: j = x/45 mod 16
        assert_eq!(uc(45, 0, 360, 8), Some(1)); // tan 45° = 1
        assert_eq!(uc(135, 0, 360, 8), Some(3)); // tan 135° = −1
        assert_eq!(uc(90, 0, 360, 8), Some(2)); // tan 90° — the pole
        assert_eq!(uc(180, 0, 360, 8), Some(4)); // tan 180° → −0 row
                                                 // huge integer exponent reduced by modular exponentiation (10^30 never materialized):
                                                 // 12·10^30/360 = 10^29/3 ∉ ℤ; 12·10^30/8 = 15·10^29 ∈ ℤ with 15·10^29 ≡ 0 (mod 24).
        assert_eq!(uc(1, 30, 360, 12), None);
        assert_eq!(uc(1, 30, 8, 12), Some(0));

        // classification, base 3 (odd): k·x/u ∈ ℤ for integer x whenever u | k·m
        let uc3 = |m: i64, e: isize, u: usize, k: usize| {
            unit_residue(&Repr::<3>::new(IBig::from(m), e), u, k)
        };
        assert_eq!(uc3(7, 0, 2, 12), Some(18)); // 42 ≡ 18 (mod 24)
        assert_eq!(uc3(6, 0, 2, 12), Some(12)); // 36 ≡ 12 (mod 24)
                                                // 1/3 is exact in base 3: 12·(1/3)/2 = 2 ∈ ℤ — a sixth row invisible in base 2/10
        assert_eq!(uc3(1, -1, 2, 12), Some(2));
        assert_eq!(uc3(1, -1, 2, 8), None); // 8/(3·2) = 4/3 ∉ ℤ

        // exact reduction split, base 10, u = 2: r = |x| − k/2 as an exact rational
        match reduce_unit_argument(&Repr::<10>::new(IBig::from(3), -1), 2) {
            // 0.3: k = 1, r = -0.2 = -8/40
            UnitReduced::Split { quadrant, num, den } => {
                assert_eq!(num, IBig::from(-8));
                assert_eq!(den, IBig::from(40));
                assert_eq!(quadrant, Quadrant::Second);
            }
            _ => panic!("0.3 must take the rational split"),
        }
        match reduce_unit_argument(&Repr::<10>::new(IBig::from(26), -1), 2) {
            // 2.6: k = 5, r = 0.1 = 4/40
            UnitReduced::Split { quadrant, num, den } => {
                assert_eq!(num, IBig::from(4));
                assert_eq!(den, IBig::from(40));
                assert_eq!(quadrant, Quadrant::Second); // k = 5 ≡ 1 (mod 4)
            }
            _ => panic!("2.6 must take the rational split"),
        }
        // the tie 0.25 (u = 2): k = 1 (half-up), r = -1/4 = -50/200
        match reduce_unit_argument(&Repr::<10>::new(IBig::from(25), -2), 2) {
            UnitReduced::Split { num, den, .. } => {
                assert_eq!(num, IBig::from(-100));
                assert_eq!(den, IBig::from(400));
            }
            _ => panic!("0.25 must take the rational split"),
        }
        // u = 8, integer argument on the general path: x = 5 ≡ 5 (mod 8), k = 3,
        // r = 5 − 3·8/4 = −1 = −4/4
        match reduce_unit_argument(&Repr::<10>::new(IBig::from(5), 0), 8) {
            UnitReduced::Split { quadrant, num, den } => {
                assert_eq!(num, IBig::from(-4));
                assert_eq!(den, IBig::from(4));
                assert_eq!(quadrant, Quadrant::Fourth); // k = 3
            }
            _ => panic!("sin_unit(5, 8) must take the rational split"),
        }
        // u = 360, a huge integer multiple of u reduces exactly: x = 10^6 degrees,
        // 10^6 mod 360 = 280, k = round(4·280/360) = 3, r = 280 − 270 = 10 = 40/4
        match reduce_unit_argument(&Repr::<10>::new(IBig::from(1_000_000), 0), 360) {
            UnitReduced::Split { quadrant, num, den } => {
                assert_eq!(num, IBig::from(40));
                assert_eq!(den, IBig::from(4));
                assert_eq!(quadrant, Quadrant::Fourth); // k = 3
            }
            _ => panic!("10^6 degrees must take the rational split"),
        }
        // tiny: the k = 0 product path, no denominator materialized
        assert!(matches!(
            reduce_unit_argument(&Repr::<10>::new(IBig::from(1), -1), 2),
            UnitReduced::Small(_)
        ));
        assert!(matches!(
            reduce_unit_argument(&Repr::<2>::new(IBig::from(3), -1_000_000_000), 2),
            UnitReduced::Small(_)
        ));
        // negative input reduces on |x|
        match reduce_unit_argument(&Repr::<10>::new(IBig::from(-3), -1), 2) {
            UnitReduced::Split { num, den, .. } => {
                assert_eq!(num, IBig::from(-8));
                assert_eq!(den, IBig::from(40));
            }
            _ => panic!("-0.3 must take the rational split"),
        }
    }
}
