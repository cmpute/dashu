//! Trigonometric functions, built on top of the cached constants π/2 and the real
//! [`exp`](crate::FBig::exp)/[`ln`](crate::FBig::ln) primitives:
//!
//! - Circular: `sin`, `cos`, `tan`, `sin_cos`, and their inverses `asin`, `acos`, `atan`.
//!
//! Argument reduction to the first quadrant reuses the cached π so that repeated
//! calls at increasing precision extend the shared constant state.

use crate::{
    error::{assert_limited_precision, FpError},
    fbig::FBig,
    math::{
        cache::{compute_e, reborrow_cache, ConstCache},
        FpResult,
    },
    repr::{Context, Repr, Word},
    round::{ErrorBounds, Round, Rounded},
};
use core::convert::TryFrom;
use dashu_base::{Abs, AbsOrd, Approximation::Exact, RemEuclid, Sign, UnsignedAbs};
use dashu_int::IBig;

/// A near-correct value paired with its provable error radius (the Ziv closure contract).
pub(crate) type Rad<R, const B: Word> = (FBig<R, B>, FBig<R, B>);

/// Series-truncation error radius shared by the Maclaurin/Euler cores (`sin`/`cos`/`sin_cos`/
/// `atan` here, and `ln`). Each accumulated term contributes `< 1 ulp` of rounding and the
/// truncated tail adds another `< 1 ulp`, so `|value − true| < (4·terms + 12)·ulp(value)`: the
/// `4·terms` covers per-step rounding, the `12` the reconstruction (the `×2` atanh factor, the
/// `s·ln2`/powering recombination, and a safety margin).
pub(crate) fn series_radius<R: Round, const B: Word>(
    value: &FBig<R, B>,
    terms: usize,
) -> FBig<R, B> {
    value.ulp() * (4 * terms + 12)
}

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
    fn compute_work_context_trig<const B: Word>(self, x: &Repr<B>, guard: usize) -> Self {
        // x_mag estimates m = floor(log_BASE(|x|))
        let x_mag = (x.exponent.saturating_add(x.digits_ub() as isize)).max(0) as usize;
        let extra_guards = guard + x_mag / 10;
        let work_precision = self
            .precision
            .saturating_add(x_mag)
            .saturating_add(extra_guards);
        Self::new(work_precision)
    }

    /// Reduces the argument to the first quadrant: `r = x − k·(π/2)` with `r ∈ (−π/4, π/4]`.
    /// Returns the work context, `r`, the quadrant `k % 4`, and the **reduction error** — a provable
    /// bound on `|r_computed − r_true|` (dominated by `|k|·ulp(half_pi)` for huge `|x|`), which the
    /// Ziv wrapper folds into the result radius so the containment test is sound.
    fn reduce_to_quadrant<const B: Word>(
        self,
        x: &Repr<B>,
        guard: usize,
        mut cache: Option<&mut ConstCache>,
    ) -> (Self, FBig<R, B>, Quadrant, FBig<R, B>) {
        let work_context = self.compute_work_context_trig(x, guard);
        let x_f = FBig::<R, B>::new(work_context.repr_round(x.clone()).value(), work_context);

        let pi = work_context.pi::<B>(reborrow_cache(&mut cache)).value();
        let half_pi = &pi / 2u8;
        let x_scaled: FBig<R, B> = &x_f / &half_pi;
        let k_f = x_scaled.round();
        // Reduce `r = x − k·(π/2)` with a single rounding via FMA: the product
        // `k·(π/2)` nearly cancels `x` for large arguments, so fusing the multiply
        // with the subtract (instead of mul-then-sub's two roundings) tightens the
        // reduction error that `reduction_err` below bounds and Ziv then certifies.
        // The conservative `r_ulp·4` term stays sound — FMA only reduces actual
        // error, never the bound.
        let r = k_f.fma(&half_pi, &x_f, Sign::Negative);
        // `k_f` is the integer nearest `x_scaled`, so it's exact (or a signed zero
        // for a tiny argument in (-1, 0), which `IBig::try_from` treats as plain 0).
        let k = IBig::try_from(k_f).expect("k_f is an exact integer or signed zero");

        // Reduction error bound: the rounded `half_pi` carries `< 1 ulp`, scaled by `|k|`; the `x`
        // rounding and the subtraction add a few `r`-ULPs. Computed at the work precision (|k| fits
        // in its digits, so this is accurate; the full-ulp factors and the +4 over-estimate) — kept
        // off unlimited precision so `tan`'s `/|cos|` radius division stays legal.
        let half_pi_ulp = half_pi.ulp();
        let r_ulp = r.ulp();
        let k_abs = k.clone().unsigned_abs();
        let reduction_err = half_pi_ulp * k_abs + r_ulp * 4;

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

        (work_context, r, quadrant, reduction_err)
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
        // that absorbs the `x − k·(π/2)` cancellation), evaluate the series, and fold the reduction
        // error into the radius so the containment test is sound even for huge |x|.
        Ok(self.ziv(50, |guard| {
            let (work, r, quadrant, reduction_err) =
                self.reduce_to_quadrant(x, guard, reborrow_cache(&mut cache));
            let (val, series_radius) = match quadrant {
                Quadrant::First => work.sin_compute(&r),
                Quadrant::Second => work.cos_compute(&r),
                Quadrant::Third => {
                    let (v, e) = work.sin_compute(&r);
                    (-v, e)
                }
                Quadrant::Fourth => {
                    let (v, e) = work.cos_compute(&r);
                    (-v, e)
                }
            };
            (val, series_radius + reduction_err)
        }))
    }

    /// Near-correct sine series `S(x) = x − x³/3! + x⁵/5! − …` on the reduced argument, returning
    /// `(value, error_radius)`. The radius covers series truncation (`< 1 working-ULP` by the break
    /// test) plus `~3K` steps of rounding accumulation. Used by the Ziv-backed `sin`/`cos`/`tan`.
    fn sin_compute<const B: Word>(self, x: &FBig<R, B>) -> (FBig<R, B>, FBig<R, B>) {
        if x.repr.significand.is_zero() {
            return (FBig::ZERO, FBig::ZERO);
        }
        let x2 = x.sqr();
        let mut sum = x.clone();
        let mut term = x.clone();
        let mut k = 1usize;
        let threshold = sum.ulp_lb();
        loop {
            term *= &x2;
            term /= (2 * k) * (2 * k + 1);
            if term.abs_cmp(&threshold).is_le() {
                break;
            }
            if k % 2 == 1 {
                sum -= &term;
            } else {
                sum += &term;
            }
            k += 1;
        }
        let radius = series_radius(&sum, k);
        (sum, radius)
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

        Ok(self.ziv(50, |guard| {
            let (work, r, quadrant, reduction_err) =
                self.reduce_to_quadrant(x, guard, reborrow_cache(&mut cache));
            let (val, series_radius) = match quadrant {
                Quadrant::First => work.cos_compute(&r),
                Quadrant::Second => {
                    let (v, e) = work.sin_compute(&r);
                    (-v, e)
                }
                Quadrant::Third => {
                    let (v, e) = work.cos_compute(&r);
                    (-v, e)
                }
                Quadrant::Fourth => work.sin_compute(&r),
            };
            (val, series_radius + reduction_err)
        }))
    }

    /// Near-correct cosine series `C(x) = 1 − x²/2! + x⁴/4! − …`, returning `(value, radius)`.
    /// (See [`sin_compute`](Self::sin_compute) for the radius derivation.)
    fn cos_compute<const B: Word>(self, x: &FBig<R, B>) -> (FBig<R, B>, FBig<R, B>) {
        if x.repr.significand.is_zero() {
            return (FBig::ONE.with_precision(self.precision).value(), FBig::ZERO);
        }
        let x2 = x.sqr();
        let mut sum = FBig::<R, B>::ONE.with_precision(self.precision).value();
        let mut term = sum.clone();
        let mut k = 1usize;
        let threshold = sum.ulp_lb();
        loop {
            term *= &x2;
            term /= (2 * k) * (2 * k - 1);
            if term.abs_cmp(&threshold).is_le() {
                break;
            }
            if k % 2 == 1 {
                sum -= &term;
            } else {
                sum += &term;
            }
            k += 1;
        }
        let radius = series_radius(&sum, k);
        (sum, radius)
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
            let (work, r, quadrant, reduction_err) =
                self.reduce_to_quadrant(x, guard, reborrow_cache(&mut cache));
            let ((sin_r, sin_e), (cos_r, cos_e)) = work.sin_cos_compute(&r);
            let (s, c) = match quadrant {
                Quadrant::First => (sin_r, cos_r),
                Quadrant::Second => (cos_r, -sin_r),
                Quadrant::Third => (-sin_r, -cos_r),
                Quadrant::Fourth => (-cos_r, sin_r),
            };
            ((s, sin_e + reduction_err.clone()), (c, cos_e + reduction_err))
        });
        (Ok(s), Ok(c))
    }

    /// Simultaneously evaluate the sine and cosine series, returning both values and their radii.
    pub(crate) fn sin_cos_compute<const B: Word>(self, x: &FBig<R, B>) -> (Rad<R, B>, Rad<R, B>) {
        if x.repr.significand.is_zero() {
            return (
                (FBig::ZERO, FBig::ZERO),
                (FBig::ONE.with_precision(self.precision).value(), FBig::ZERO),
            );
        }
        let x2 = x.sqr();
        let mut sin_sum = x.clone();
        let mut cos_sum = FBig::<R, B>::ONE.with_precision(self.precision).value();
        let mut sin_term = x.clone();
        let mut cos_term = cos_sum.clone();
        let mut k = 1usize;
        let sin_threshold = sin_sum.ulp_lb();
        let cos_threshold = cos_sum.ulp_lb();
        loop {
            cos_term *= &x2;
            cos_term /= (2 * k) * (2 * k - 1);
            sin_term *= &x2;
            sin_term /= (2 * k) * (2 * k + 1);

            if sin_term.abs_cmp(&sin_threshold).is_le() && cos_term.abs_cmp(&cos_threshold).is_le()
            {
                break;
            }

            if k % 2 == 1 {
                cos_sum -= &cos_term;
                sin_sum -= &sin_term;
            } else {
                cos_sum += &cos_term;
                sin_sum += &sin_term;
            }
            k += 1;
        }
        (
            (sin_sum.clone(), series_radius(&sin_sum, k)),
            (cos_sum.clone(), series_radius(&cos_sum, k)),
        )
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

        // tan = sin/cos, correctly rounded via the Ziv loop. Near a pole (an odd multiple of π/2)
        // the value is large but finite at the working precision — dashu's wide exponent range holds
        // it, and the sign is carried by the arithmetic (s/−|c| is negative), so there is no pole
        // special-case here. The closure's `significand.is_zero()` guard below handles the
        // unreachable exact-pole case (cos cancelling to a zero significand) by forcing a retry.
        // Skipping a hoisted pole check avoids recomputing the sin/cos series twice (once for the
        // check, once for the first Ziv attempt).
        Ok(self.ziv(50, |guard| {
            let (work, r, quadrant, reduction_err) =
                self.reduce_to_quadrant(x, guard, reborrow_cache(&mut cache));
            let ((sin_r, sin_e), (cos_r, cos_e)) = work.sin_cos_compute(&r);
            let (s, c) = match quadrant {
                Quadrant::First => (sin_r, cos_r),
                Quadrant::Second => (cos_r, -sin_r),
                Quadrant::Third => (-sin_r, -cos_r),
                Quadrant::Fourth => (-cos_r, sin_r),
            };
            if c.repr.significand.is_zero() {
                // cos rounded to a zero significand at this guard (the input sits on a work-
                // precision pole — unreachable for finite-precision x): force a retry. A higher guard
                // makes cos representable (nonzero), yielding a large finite tan.
                return (FBig::ZERO, FBig::ONE);
            }
            let result = work.div(&s.repr, &c.repr).unwrap().value();
            // tan = s/c: the sin/cos radii propagate as (e_s + |tan|·e_c)/|c| plus the division
            // rounding, all at the working precision (the only term that needed unlimited precision
            // — the reduction error — is already work-precision, so the `/|c|` stays legal).
            let e_s = sin_e + reduction_err.clone();
            let e_c = cos_e + reduction_err;
            let radius = (e_s + result.clone().abs() * e_c) / c.clone().abs() + result.ulp() * 8;
            (result, radius)
        }))
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
            // asin(±0) = ±0 (asin is odd), exact. Like the other inverse trig/hyperbolic functions,
            // short-circuit before the Ziv loop: a zero result carries a positive radius that can't
            // be certified against 0's one-sided preimage under directed rounding.
            return signed_zero_normal(self, x);
        }

        let x_orig = FBig::<R, B>::new(x.clone(), *self);
        // Domain check: |x| must be <= 1
        if x_orig.abs_cmp(&FBig::ONE).is_gt() {
            return Err(FpError::OutOfDomain);
        }

        Ok(self.ziv(50, |guard| {
            let work = Context::<R>::new(self.precision + guard);
            let x_f = FBig::<R, B>::new(work.repr_round_ref(x).value(), work);
            let one = FBig::<R, B>::ONE.with_precision(work.precision).value();
            let d = work
                .sqrt(&(one.clone() - x_f.clone().sqr()).repr)
                .unwrap()
                .value();
            if d.repr.is_pos_zero() || d.repr.is_neg_zero() {
                // |x| = 1: asin(±1) = ±π/2.
                let pi = work.pi::<B>(reborrow_cache(&mut cache)).value();
                let half_pi = pi / 2u8;
                let res = if x_f.sign() == Sign::Positive {
                    half_pi
                } else {
                    -half_pi
                };
                let radius = res.ulp() * 4;
                return (res, radius);
            }
            // asin(x) = atan(x / sqrt(1−x²)); `atan`/`sqrt` are Ziv-correct at the working
            // precision, so the radius is just the accumulated `sqrt`+`div` rounding (well-conditioned
            // near |x|=1, where atan's derivative → 0).
            let arg = &x_f / &d;
            let res = work
                .atan(&arg.repr, reborrow_cache(&mut cache))
                .unwrap()
                .value();
            let radius = res.ulp() * 16;
            (res, radius)
        }))
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
            // Ziv containment test can never certify it (any positive radius dips the interval below
            // 0) and would infinite-retry. acos(-1) = π is handled here too, for symmetry.
            return Ok(if x.sign() == Sign::Positive {
                Exact(FBig::<R, B>::new(Repr::zero(), *self))
            } else {
                self.pi::<B>(reborrow_cache(&mut cache))
            });
        }

        Ok(self.ziv(50, |guard| {
            let work = Context::<R>::new(self.precision + guard);
            // acos(x) = π/2 − asin(x); `asin`/`pi` are Ziv-correct (or exact) at the working
            // precision. The radius covers the propagated asin/π rounding plus the subtraction,
            // which cancels near x = 1 — the radius grows there and Ziv retries with more guard.
            let asin_x = work.asin(x, reborrow_cache(&mut cache)).unwrap().value();
            let pi = work.pi::<B>(reborrow_cache(&mut cache)).value();
            let res = (pi / 2u8) - &asin_x;
            let radius = asin_x.ulp().clone().with_precision(0).value() * 2
                + res.ulp().clone().with_precision(0).value() * 4;
            (res, radius)
        }))
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

        Ok(self.ziv(50, |guard| {
            let work = Context::<R>::new(self.precision + guard);
            let x_f = FBig::<R, B>::new(work.repr_round_ref(x).value(), work);
            let sign = x_f.sign();
            let x_abs = x_f.abs();
            let one = FBig::<R, B>::ONE.with_precision(work.precision).value();
            let (res, radius) = if x_abs >= one {
                // |x| ≥ 1: atan(x) = π/2 − atan(1/x); the series runs on 1/x ∈ (0, 1].
                let pi = work.pi::<B>(reborrow_cache(&mut cache)).value();
                let inv_x = &one / &x_abs;
                let (atan_val, atan_radius) = work.atan_compute(&inv_x);
                let res = (pi / 2u8) - atan_val;
                let radius = atan_radius + res.ulp() * 4;
                (res, radius)
            } else {
                work.atan_compute(&x_abs)
            };
            let res = if sign == Sign::Negative { -res } else { res };
            (res, radius)
        }))
    }

    /// Near-correct Euler series for `atan(x)` (`|x| ≤ 1`), returning `(value, radius)`. The radius
    /// covers series truncation plus `~3N` steps of accumulation.
    fn atan_compute<const B: Word>(self, x: &FBig<R, B>) -> (FBig<R, B>, FBig<R, B>) {
        // Euler's series for atan(x)
        let x2 = x.sqr();
        let one_plus_x2 = FBig::ONE + &x2;
        let mut term = x / &one_plus_x2;
        let mut sum = term.clone();
        let factor = (2 * &x2) / one_plus_x2;
        let mut n = 1usize;
        let threshold = sum.ulp_lb();
        loop {
            term *= &factor;
            term *= n;
            term /= 2 * n + 1;
            if term.abs_cmp(&threshold).is_le() {
                break;
            }
            sum += &term;
            n += 1;
        }
        let radius = series_radius(&sum, n);
        (sum, radius)
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

        // x ≠ 0, finite: atan2 = atan(y/x) ± (quadrant π). `atan` is Ziv-correct at the working
        // precision, so the radius is the accumulated div/π-arithmetic rounding.
        Ok(self.ziv(50, |guard| {
            let work = Context::<R>::new(self.precision + guard);
            let y_f = FBig::<R, B>::new(work.repr_round_ref(y).value(), work);
            let x_f = FBig::<R, B>::new(work.repr_round_ref(x).value(), work);
            let ratio = &y_f / &x_f;
            let atan_val = work
                .atan(&ratio.repr, reborrow_cache(&mut cache))
                .unwrap()
                .value();
            let (res, radius) = if x.sign() == Sign::Positive {
                (atan_val.clone(), atan_val.ulp() * 6)
            } else {
                let pi = work.pi::<B>(reborrow_cache(&mut cache)).value();
                let r = if y_f.sign() == Sign::Positive {
                    &atan_val + &pi
                } else {
                    &atan_val - &pi
                };
                let radius = atan_val.ulp() * 2 + r.ulp() * 6;
                (r, radius)
            };
            (res, radius)
        }))
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
