//! The complex ball — a pair of real [`Ball`]s (one per component) with complex error
//! propagation. This is the value space the transcendental closures compute in, replacing the
//! hand-written `ulp()·k` radii: every rounding of the composition is tracked mechanically, and
//! an exact chain keeps `rad == 0` on both components, which is how exactly-representable
//! results (and their exact-zero components) stay certifiable under the directed rounding modes.
//!
//! The real halves are float's `Ball`/`Mag` (shared in lockstep through float's `#[doc(hidden)]`
//! re-exports), so the per-component propagation rules are float's own; only the *complex* rules
//! live here — the product rule (4 tracked products + add/sub), the quotient through `‖z‖²`,
//! and the input-error folds for the float kernels, which run on the midpoints and get their own
//! contribution from the kernel's rounding via `Ball::from_rounded` (exact → radius 0, inexact →
//! one work-ulp; the public kernels are correctly rounded, so one ulp covers them with a 2×
//! margin).
//!
//! Soundness of every fold below rests on the same two facts as float's ball: a [`Mag`] is a
//! rigorous upper bound by construction (all its ops round up), and the folds are stated as
//! exact inequalities — `ln(1+t) ≤ t`, `√(a+b) ≤ √a+√b`, `√t ≤ 1+t`, `‖∇arg‖ = 1/‖z‖` — so no
//! fudge constants appear anywhere.

use dashu_base::{Approximation, Sign};
use dashu_float::{ulp_mag, Ball, ConstCache, Context as FloatCtxt, FBig, FpError, Mag, Repr};
use dashu_int::{IBig, Word};

use crate::repr::reborrow_cache;
use crate::round::{mode, ErrorBounds, Round};

/// A complex ball: midpoints plus radii per component, `|component − true| ≤ rad`.
#[derive(Clone, Debug)]
pub(crate) struct CBall<const B: Word> {
    pub(crate) re: Ball<B>,
    pub(crate) im: Ball<B>,
}

impl<const B: Word> CBall<B> {
    // ========================================================================
    // Seeds
    // ========================================================================

    /// An exact complex value (both radii zero) — the entry point for a Ziv closure's input.
    pub(crate) fn exact(re: Repr<B>, im: Repr<B>) -> Self {
        Self {
            re: Ball::exact(re),
            im: Ball::exact(im),
        }
    }

    /// Seed from an input's parts: the parts are re-rooted to the working precision (an input
    /// carrying more digits than the working precision rounds here, and that rounding joins the
    /// radius like any other). The re-round runs under [`mode::HalfEven`] regardless of the
    /// caller's mode — one work-ulp covers any direction's rounding error.
    pub(crate) fn from_parts(re: &Repr<B>, im: &Repr<B>, prec: usize) -> Self {
        Self {
            re: seed_ball(re, prec),
            im: seed_ball(im, prec),
        }
    }

    // ========================================================================
    // Exact operations — no prec, no error
    // ========================================================================

    /// Multiply by `±i` — an exact rotation, mirroring `CBig::mul_i`: `(x, y)·i = (−y, x)`,
    /// `(x, y)·(−i) = (y, −x)`. Radii swap unchanged.
    pub(crate) fn mul_i(&self, negative: bool) -> Self {
        if negative {
            Self {
                re: self.im.clone(),
                im: -self.re.clone(),
            }
        } else {
            Self {
                re: -self.im.clone(),
                im: self.re.clone(),
            }
        }
    }

    // ========================================================================
    // Propagation — component rules over the real balls
    // ========================================================================

    /// Componentwise `a + b` (the complex addition is real addition per component).
    pub(crate) fn add(&self, rhs: &Self, prec: usize) -> Result<Self, FpError> {
        Ok(Self {
            re: self.re.add(&rhs.re, prec)?,
            im: self.im.add(&rhs.im, prec)?,
        })
    }

    /// Componentwise `a − b`.
    pub(crate) fn sub(&self, rhs: &Self, prec: usize) -> Result<Self, FpError> {
        Ok(Self {
            re: self.re.sub(&rhs.re, prec)?,
            im: self.im.sub(&rhs.im, prec)?,
        })
    }

    /// `(a+bi)(c+di) = (ac−bd) + (ad+bc)i` through four tracked products. The exact cross-term
    /// algebra (`|ac − âĉ| ≤ |â|rad_c + |ĉ|rad_a + rad_a·rad_c`, identically for the other
    /// three) is precisely `Ball::mul`'s rule, so each component's radius is the sum of its two
    /// product radii plus the final add/sub rounding — nothing is dropped, second-order terms
    /// included.
    pub(crate) fn mul(&self, rhs: &Self, prec: usize) -> Result<Self, FpError> {
        let ac = self.re.mul(&rhs.re, prec)?;
        let bd = self.im.mul(&rhs.im, prec)?;
        let ad = self.re.mul(&rhs.im, prec)?;
        let bc = self.im.mul(&rhs.re, prec)?;
        Ok(Self {
            re: ac.sub(&bd, prec)?,
            im: ad.add(&bc, prec)?,
        })
    }

    /// `self² = (a²−b²) + (2ab)i` — the square kernels are cheaper than a general product, and
    /// the doubling `ab + ab` rounds exactly (same significand, exponent+1), so the imaginary
    /// radius costs a single ε.
    pub(crate) fn sqr(&self, prec: usize) -> Result<Self, FpError> {
        let a2 = self.re.sqr(prec)?;
        let b2 = self.im.sqr(prec)?;
        let ab = self.re.mul(&self.im, prec)?;
        Ok(Self {
            re: a2.sub(&b2, prec)?,
            im: ab.add(&ab, prec)?,
        })
    }

    /// The squared modulus `a² + b²` — a real ball (the division denominators).
    pub(crate) fn norm_sqr(&self, prec: usize) -> Result<Ball<B>, FpError> {
        self.re.sqr(prec)?.add(&self.im.sqr(prec)?, prec)
    }

    /// `1/self = conj(self)/‖self‖²` — the only complex division the transcendentals need.
    /// A denominator ball touching zero yields an infinite radius (a Ziv retry, soundly); an
    /// exact zero denominator fails the midpoint division first.
    pub(crate) fn inv(&self, prec: usize) -> Result<Self, FpError> {
        let d = self.norm_sqr(prec)?;
        Ok(Self {
            re: self.re.div(&d, prec)?,
            im: -self.im.clone().div(&d, prec)?,
        })
    }

    /// Componentwise division by a *real* ball: `(u/d, v/d)`. Serves `tan`'s shared denominator
    /// `cos(2x) + cosh(2y)` and `atan`'s exact halving.
    pub(crate) fn div_by_real(&self, d: &Ball<B>, prec: usize) -> Result<Self, FpError> {
        Ok(Self {
            re: div_real(&self.re, d, prec)?,
            im: div_real(&self.im, d, prec)?,
        })
    }

    // ========================================================================
    // Transcendental compositions (float kernels on the midpoints + input folds)
    // ========================================================================

    /// `exp(self) = eˣ(cos y + i·sin y)`: the float `exp`/`sin_cos` kernels evaluate on the
    /// midpoints (each certified to ≤ 1 work-ulp by its own Ziv loop, which `from_rounded`
    /// folds), then the input radii propagate:
    /// * real part `|e^{x+θ} − eˣ| ≤ rad_x·e^{x+rad_x}` — float's own fold
    ///   (`‖result‖·exp_upper(rad_x)·rad_x`, both factors essential — see float's `exp_ball`);
    /// * imaginary part `|Δsin|, |Δcos| ≤ |θ| = rad_y` (the derivatives are bounded by 1),
    ///   which the two product rules then carry into both components.
    pub(crate) fn exp<R: ErrorBounds>(
        &self,
        fctx: &FloatCtxt<R>,
        prec: usize,
        mut cache: Option<&mut ConstCache>,
    ) -> Result<Self, FpError> {
        let mut ex = Ball::from_rounded(
            fctx.exp(&self.re.mid, reborrow_cache(&mut cache))?
                .map(FBig::into_repr),
            prec,
        );
        if !self.re.rad.is_zero() {
            // float's exp fold, verbatim (the magnitude before the error, never clamped)
            let factor = ex.mag().mul(&self.re.rad.exp_upper());
            ex.add_error(factor.mul(&self.re.rad));
        }
        let (sin_y, cos_y) = fctx.sin_cos(&self.im.mid, reborrow_cache(&mut cache));
        let mut sy = Ball::from_rounded(sin_y?.map(FBig::into_repr), prec);
        let mut cy = Ball::from_rounded(cos_y?.map(FBig::into_repr), prec);
        if !self.im.rad.is_zero() {
            sy.add_error(self.im.rad);
            cy.add_error(self.im.rad);
        }
        Ok(Self {
            re: ex.mul(&cy, prec)?,
            im: ex.mul(&sy, prec)?,
        })
    }

    /// `log(self) = (ln‖self‖, arg self)`: the float `hypot`/`ln`/`atan2` kernels evaluate on
    /// the midpoints; the input radii propagate through the exact inequalities
    /// `|Δln r| ≤ ln(hi/lo) ≤ (hi−lo)/lo` (monotonicity + `ln(1+t) ≤ t`, `[lo, hi]` the input
    /// ball's bracket of the true `‖z‖`) and `|Δarg| ≤ ‖δz‖/lo` (`‖∇arg‖ = 1/‖z‖`, mean value
    /// theorem along the input segment). Near `‖z‖ = 1` this reproduces the old hand-written
    /// `B^{1−pw}` amplification term at the same order; away from it the fold is far tighter
    /// because it is no longer paid unconditionally.
    pub(crate) fn log<R: ErrorBounds>(
        &self,
        fctx: &FloatCtxt<R>,
        prec: usize,
        mut cache: Option<&mut ConstCache>,
    ) -> Result<Self, FpError> {
        // ‖z‖ with the hypot fold (the Euclidean norm is 1-Lipschitz: |Δ‖z‖| ≤ |δx| + |δy|)
        let mut r =
            Ball::from_rounded(fctx.hypot(&self.re.mid, &self.im.mid)?.map(FBig::into_repr), prec);
        r.add_error(self.re.rad.add(&self.im.rad));

        // A lower bound of the true ‖z‖ (drives both folds). The *bracket width* must not be
        // taken from `Mag::from_repr`/`from_repr_lower`: at base ≠ 2 the `B^e` scaling enters
        // the Mag only through the fixed-point `log₂B` bracket, so the two bounds of an exact
        // mid can sit a factor ~2 apart — a value-space `hi − lo` there would inflate the ln
        // fold to O(1) (retrying forever). The `rad` side is a true Mag (1-word tight), so the
        // fold keeps the bracket on `rad` alone:
        // |Δln| ≤ ln((m+rad)/(m−rad)) ≤ 2·rad/(m−rad), with `lo` a lower bound of `m−rad`.
        let lo = Mag::from_repr_lower(&r.mid).sub_down(&r.rad);
        let ln_fold = if lo.is_zero() {
            Mag::INFINITY
        } else {
            r.rad.mul_pow2(1).div(&lo)
        };
        let arg_fold = if lo.is_zero() {
            Mag::INFINITY
        } else {
            self.re.rad.add(&self.im.rad).div(&lo)
        };

        let mut ln_r = Ball::from_rounded(
            fctx.ln(&r.mid, reborrow_cache(&mut cache))?
                .map(FBig::into_repr),
            prec,
        );
        ln_r.add_error(ln_fold);

        let mut arg = Ball::from_rounded(
            fctx.atan2(&self.im.mid, &self.re.mid, reborrow_cache(&mut cache))?
                .map(FBig::into_repr),
            prec,
        );
        arg.add_error(arg_fold);

        Ok(Self { re: ln_r, im: arg })
    }

    /// Principal square root through the cancellation-free form (for `x ≥ 0`:
    /// `a = √((r+x)/2)`, `b = y/(2a)`; `x < 0` mirrored, `b` carrying the sign of `y`) — the
    /// same recipe as the hand-radius version, but every add/sub/div/mul/sqrt is a tracked real
    /// ball op and only `hypot` is a kernel. The real `sqrt` fold is float's own
    /// `rad_arg/(2·LB(|mid|))`; when the argument ball touches zero the bound
    /// `√A ≤ √M + √rad ≤ mid + ε + (1 + rad)` (subadditivity, then `√t ≤ 1+t`) keeps a *finite*
    /// radius that shrinks as the guard grows, so a straddling argument is a Ziv retry, not a
    /// dead end.
    pub(crate) fn sqrt<R: ErrorBounds>(
        &self,
        fctx: &FloatCtxt<R>,
        prec: usize,
    ) -> Result<Self, FpError> {
        // an exact (0, 0) ball roots to exactly 0 — short-circuit before the cancellation-free
        // form divides 0/(2·0) (asin/acosh at z = ±1 reach exactly this interior zero). The
        // infinite check matters: an infinite midpoint's significand is zero too.
        if !self.re.mid.is_infinite()
            && !self.im.mid.is_infinite()
            && self.re.mid.significand().is_zero()
            && self.im.mid.significand().is_zero()
            && self.re.rad.is_zero()
            && self.im.rad.is_zero()
        {
            return Ok(Self::exact(Repr::zero(), Repr::zero()));
        }
        let mut r =
            Ball::from_rounded(fctx.hypot(&self.re.mid, &self.im.mid)?.map(FBig::into_repr), prec);
        r.add_error(self.re.rad.add(&self.im.rad));

        let two = Ball::exact(Repr::<B>::new(IBig::from(2u8), 0));
        if self.re.mid.sign() != Sign::Negative {
            let a = sqrt_real(r.add(&self.re, prec)?.div(&two, prec)?, fctx, prec)?;
            let b = div_real(&self.im, &a.mul(&two, prec)?, prec)?;
            Ok(Self { re: a, im: b })
        } else {
            // r − x = r + |x| — no cancellation
            let b0 = sqrt_real(r.sub(&self.re, prec)?.div(&two, prec)?, fctx, prec)?;
            let b = if self.im.mid.sign() == Sign::Negative {
                -b0
            } else {
                b0
            };
            let a = div_real(&self.im, &b.mul(&two, prec)?, prec)?;
            Ok(Self { re: a, im: b })
        }
    }

    // ========================================================================
    // Ziv boundary
    // ========================================================================

    /// Export both components as `(value, radius)` pairs — the driver's closure contract.
    pub(crate) fn to_parts_radius<R: Round>(
        &self,
        ctx: &FloatCtxt<R>,
    ) -> [(FBig<R, B>, FBig<R, B>); 2] {
        [self.re.to_value_radius(ctx), self.im.to_value_radius(ctx)]
    }
}

/// Re-root one input part to the working precision (an over-precise input rounds here, and the
/// rounding joins the radius like any other).
fn seed_ball<const B: Word>(re: &Repr<B>, prec: usize) -> Ball<B> {
    // build the part as an exact (unlimited) value, then round to the working precision — an
    // over-precise significand genuinely rounds here (same-precision `with_precision` is a no-op)
    let rooted =
        FBig::<mode::HalfEven, B>::from_repr(re.clone(), FloatCtxt::<mode::HalfEven>::new(0))
            .with_precision(prec);
    Ball::from_rounded(rooted.map(FBig::into_repr), prec)
}

/// `num / d` by a real ball, with the exact-zero numerator shortcut: a precisely-zero numerator
/// divides to a precisely-zero quotient no matter how uncertain the denominator is — the generic
/// `Ball::div` rule would price the touching-zero denominator as an infinite radius, which no
/// Ziv loop can ever certify (e.g. the `y/(2a)` of `√(x + 0i)` with an inexact root `a`).
fn div_real<const B: Word>(num: &Ball<B>, d: &Ball<B>, prec: usize) -> Result<Ball<B>, FpError> {
    if num.rad.is_zero() && num.mid.significand().is_zero() && !d.mid.significand().is_zero() {
        return Ok(Ball::exact(num.mid.clone()));
    }
    // an exact zero numerator over an exact zero denominator is a genuine 0/0: fall through so
    // the kernel division reports `Indeterminate` (e.g. `tan_pi` at a real-axis pole)
    num.div(d, prec)
}

/// The tracked real `√`: the float kernel on the midpoint, then `rad_arg/(2·LB(|mid|))` —
/// float's documented `Ball::sqrt` fold — with the straddle fallback for a zero-touching
/// argument (see [`CBall::sqrt`]).
fn sqrt_real<R: ErrorBounds, const B: Word>(
    arg: Ball<B>,
    fctx: &FloatCtxt<R>,
    prec: usize,
) -> Result<Ball<B>, FpError> {
    // float's `finish_mid` rule: the fresh ε joins only when the kernel rounded inexact — an
    // exact root keeps `rad = 0` so the exact chain (e.g. √4) stays certifiable
    let (mid, eps) = match fctx.sqrt(&arg.mid)?.map(FBig::into_repr) {
        Approximation::Exact(repr) => (repr, Mag::ZERO),
        Approximation::Inexact(repr, _) => {
            let eps = ulp_mag::<B>(&repr, prec);
            (repr, eps)
        }
    };
    if mid.significand().is_zero() {
        // √(exact 0): the kernel returned an exact zero — keep the chain exact (directed modes
        // can only certify a zero component through a zero radius).
        return Ok(Ball::exact(mid));
    }
    let lo = Mag::from_repr_lower(&arg.mid).sub_down(&arg.rad);
    let mut rad = if lo.is_zero() {
        Mag::ONE.add(&Mag::from_repr(&mid)).add(&arg.rad)
    } else {
        arg.rad.div(&Mag::from_repr_lower(&mid).mul_pow2(1))
    };
    rad = rad.add(&eps);
    Ok(Ball { mid, rad })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::round::mode;
    use core::cmp::Ordering;

    type F = FBig<mode::HalfEven, 2>;

    fn repr(sig: i64, exp: isize) -> Repr<2> {
        Repr::new(IBig::from(sig), exp)
    }

    fn cb(re: Repr<2>, im: Repr<2>) -> CBall<2> {
        CBall::exact(re, im)
    }

    /// Assert `[mid − rad, mid + rad]` covers the exact `true_v`, decided on lossless
    /// unlimited-precision sums (the radius enters as its exported exact `Repr`).
    fn covers(ball: &Ball<2>, true_v: &F) -> bool {
        let x = FloatCtxt::<mode::HalfEven>::new(0);
        let (v, r) = ball.to_value_radius(&x);
        let hi = x.add(v.repr(), r.repr()).unwrap().value();
        let lo = x.sub(v.repr(), r.repr()).unwrap().value();
        hi.cmp(true_v) != Ordering::Less && lo.cmp(true_v) != Ordering::Greater
    }

    fn covers2(out: &CBall<2>, true_re: &F, true_im: &F) -> bool {
        covers(&out.re, true_re) && covers(&out.im, true_im)
    }

    #[test]
    fn mul_exact_chain() {
        let p = 10;
        let a = cb(repr(3, 0), repr(4, 0));
        let b = cb(repr(1, 0), repr(-2, 0));
        let prod = a.mul(&b, p).unwrap();
        // (3+4i)(1−2i) = 11 − 2i, exactly representable → exact chain
        assert!(prod.re.rad.is_zero() && prod.im.rad.is_zero());
        assert_eq!(prod.re.mid, repr(11, 0));
        assert_eq!(prod.im.mid, repr(-2, 0));
    }

    #[test]
    fn sqr_exact_and_covers() {
        let p = 10;
        let z = cb(repr(3, 0), repr(4, 0));
        let sq = z.sqr(p).unwrap();
        // (3+4i)² = −7 + 24i exactly
        assert!(sq.re.rad.is_zero() && sq.im.rad.is_zero());
        assert_eq!(sq.re.mid, repr(-7, 0));
        assert_eq!(sq.im.mid, repr(24, 0));
        // and it agrees with the general product rule on an inexact input (coverage)
        let one = cb(repr(1, 0), Repr::<2>::zero());
        let via_mul = z.mul(&one, p).unwrap().mul(&z, p).unwrap();
        let true_re = F::from(-7).with_precision(50).value();
        let true_im = F::from(24).with_precision(50).value();
        assert!(covers2(&sq, &true_re, &true_im));
        assert!(covers2(&via_mul, &true_re, &true_im));
    }

    #[test]
    fn mul_i_rotations_exact() {
        let z = cb(repr(3, 0), repr(4, 0));
        // (3+4i)·i = −4 + 3i
        let t = z.mul_i(false);
        assert!(t.re.rad.is_zero() && t.im.rad.is_zero());
        assert_eq!(t.re.mid, repr(-4, 0));
        assert_eq!(t.im.mid, repr(3, 0));
        // (3+4i)·(−i) = 4 − 3i
        let t = z.mul_i(true);
        assert!(t.re.rad.is_zero() && t.im.rad.is_zero());
        assert_eq!(t.re.mid, repr(4, 0));
        assert_eq!(t.im.mid, repr(-3, 0));
    }

    #[test]
    fn inv_covers_reciprocal() {
        let p = 20;
        let z = cb(repr(3, 0), repr(4, 0));
        let inv = z.inv(p).unwrap();
        // (3+4i)⁻¹ = (3 − 4i)/25, an exact rational — compare 25·(mid ± rad) against the exact
        // integer numerators on raw Reprs (lossless)
        let x = FloatCtxt::<mode::HalfEven>::new(0);
        let twenty5 = repr(25, 0);
        let (vre, rre) = inv.re.to_value_radius(&x);
        let (vim, rim) = inv.im.to_value_radius(&x);
        let v25 = vre.repr() * &twenty5;
        let r25 = rre.repr() * &twenty5;
        let w25 = vim.repr() * &twenty5;
        let s25 = rim.repr() * &twenty5;
        let lo_re = &v25 - &r25;
        let hi_re = &v25 + &r25;
        let lo_im = &w25 - &s25;
        let hi_im = &w25 + &s25;
        assert!(lo_re.cmp(&repr(3, 0)) != Ordering::Greater);
        assert!(hi_re.cmp(&repr(3, 0)) != Ordering::Less);
        assert!(lo_im.cmp(&repr(-4, 0)) != Ordering::Greater);
        assert!(hi_im.cmp(&repr(-4, 0)) != Ordering::Less);
    }

    #[test]
    fn sqrt_exact_chains() {
        let p = 10;
        let fctx = FloatCtxt::<mode::HalfEven>::new(p);
        // √(4) = 2: both components exact (directed modes certify only through rad == 0)
        let out = cb(repr(4, 0), Repr::<2>::zero()).sqrt(&fctx, p).unwrap();
        assert!(out.re.rad.is_zero() && out.im.rad.is_zero());
        assert_eq!(out.re.mid, repr(2, 0));
        assert!(out.im.mid.significand().is_zero());
        // √(−4) = 2i: the real component collapses to exact zero with a zero radius — the
        // containment guard's load-bearing case
        let out = cb(repr(-4, 0), Repr::<2>::zero()).sqrt(&fctx, p).unwrap();
        assert!(out.re.rad.is_zero() && out.im.rad.is_zero());
        assert!(out.re.mid.significand().is_zero());
        assert_eq!(out.im.mid, repr(2, 0));
    }

    #[test]
    fn sqrt_covers_exact_result() {
        let p = 20;
        let fctx = FloatCtxt::<mode::HalfEven>::new(p);
        // √(3+4i) = 2 + i exactly
        let out = cb(repr(3, 0), repr(4, 0)).sqrt(&fctx, p).unwrap();
        let two = F::from(2).with_precision(50).value();
        let one = F::from(1).with_precision(50).value();
        assert!(covers2(&out, &two, &one));
    }

    #[test]
    fn sqrt_straddling_argument_stays_finite() {
        let p = 30;
        let fctx = FloatCtxt::<mode::HalfEven>::new(p);
        // a real ball touching zero: the straddle fold keeps a finite radius covering 0
        let mut z = cb(repr(1, -10), Repr::<2>::zero());
        z.re = Ball::with_error(repr(1, -10), Mag::from_repr(&repr(1, -9)));
        let out = z.sqrt(&fctx, p).unwrap();
        assert!(!out.re.rad.is_infinite());
        assert!(covers(&out.re, &F::ZERO.with_precision(0).value()));
        assert!(covers(&out.im, &F::ZERO.with_precision(0).value()));
    }

    #[test]
    fn exp_matches_high_precision_oracle() {
        let p = 20;
        let z = cb(repr(3, -2), repr(5, -2)); // 0.75 + 1.25i
        let fctx = FloatCtxt::<mode::HalfEven>::new(p);
        let hi = FloatCtxt::<mode::HalfEven>::new(p + 64);
        let out = z.exp(&fctx, p, None).unwrap();
        let ex = hi.exp(&z.re.mid, None).unwrap().value();
        let (sy, cy) = hi.sin_cos(&z.im.mid, None);
        let true_re = &ex * &cy.unwrap().value();
        let true_im = &ex * &sy.unwrap().value();
        assert!(covers2(&out, &true_re, &true_im));
    }

    #[test]
    fn log_matches_high_precision_oracle() {
        let p = 20;
        let z = cb(repr(3, 0), repr(4, 0)); // ln 5, atan(4/3)
        let fctx = FloatCtxt::<mode::HalfEven>::new(p);
        let hi = FloatCtxt::<mode::HalfEven>::new(p + 64);
        let out = z.log(&fctx, p, None).unwrap();
        let five = hi.hypot(&repr(3, 0), &repr(4, 0)).unwrap().value();
        let true_re = hi.ln(five.repr(), None).unwrap().value();
        let true_im = hi.atan2(&repr(4, 0), &repr(3, 0), None).unwrap().value();
        assert!(covers2(&out, &true_re, &true_im));
    }

    #[test]
    fn exp_input_error_fold_keeps_coverage() {
        // a nonzero input radius must not shrink the certified interval: compare against the
        // same composition from an exact input — the fold can only widen the radius
        let p = 20;
        let z = cb(repr(3, -2), repr(5, -2));
        let fctx = FloatCtxt::<mode::HalfEven>::new(p);
        let mut z_err = z.clone();
        z_err.re = Ball::with_error(z.re.mid.clone(), Mag::from_repr(&repr(1, -30)));
        z_err.im = Ball::with_error(z.im.mid.clone(), Mag::from_repr(&repr(1, -30)));
        let out = z.exp(&fctx, p, None).unwrap();
        let out_err = z_err.exp(&fctx, p, None).unwrap();
        // the perturbed input's export must cover the exact-input result values
        let (re_true, _) = out.re.to_value_radius(&fctx);
        let (im_true, _) = out.im.to_value_radius(&fctx);
        assert!(covers(&out_err.re, &re_true));
        assert!(covers(&out_err.im, &im_true));
    }

    #[test]
    fn from_parts_of_fitting_parts_is_exact() {
        let z = CBall::from_parts(&repr(3, 0), &repr(4, 0), 20);
        assert!(z.re.rad.is_zero() && z.im.rad.is_zero());
        // an over-precise part rounds down at the seed precision, and the rounding joins the
        // radius
        let wide = Repr::new(IBig::from(1025), -10); // 21 bits at seed precision 10
        let z = CBall::from_parts(&wide, &Repr::<2>::zero(), 10);
        assert!(!z.re.rad.is_zero());
    }
}
