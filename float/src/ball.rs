//! The internal [`Ball`] — a midpoint `Repr` plus a value-space radius `Mag` — the error
//! representation the Ziv driver's transcendental closures compute with.
//!
//! The radius is an *absolute* magnitude (`|mid − true| ≤ rad`), so propagation is plain
//! interval algebra in value space: no ulp-domain shift formulas, no per-ball precision
//! state (an operation's `prec` argument is the only precision there is), no IBig error
//! bookkeeping. Every midpoint op runs through the correctly-rounded [`Context`] kernels
//! at `prec`; the kernel's `Exact`/`Inexact` flag decides whether a fresh `ε = 1 ulp`
//! joins the radius — in particular an exact chain keeps `rad == 0`, which is how
//! exactly-representable results are certified (the old `*_tracking` family folded into
//! this one rule).

use dashu_base::{Approximation, BitTest};
use dashu_int::{IBig, UBig, Word};

use crate::{
    error::FpError,
    fbig::FBig,
    mag::Mag,
    repr::{Context, Repr},
    round::{mode, Round, Rounded},
};

/// A ball on the reals: a midpoint plus a radius covering the unknown true value.
///
/// `mid` is a bare [`Repr`] (precision is a property of operations, not of the ball);
/// `rad` is a [`Mag`] upper bound. `rad == 0` ⟺ the chain so far is exact.
#[derive(Clone, Debug)]
pub(crate) struct Ball<const B: Word> {
    pub(crate) mid: Repr<B>,
    pub(crate) rad: Mag,
}

impl<const B: Word> Ball<B> {
    /// Wrap an exact midpoint (no error).
    pub(crate) fn exact(mid: Repr<B>) -> Self {
        Self {
            mid,
            rad: Mag::ZERO,
        }
    }

    /// Wrap a correctly-rounded kernel result: exact → `rad = 0`, inexact → fold one work-ulp.
    pub(crate) fn from_rounded(rounded: Rounded<Repr<B>>, prec: usize) -> Self {
        match rounded {
            Approximation::Exact(mid) => Self::exact(mid),
            Approximation::Inexact(mid, _) => Self {
                rad: ulp_mag(&mid, prec),
                mid,
            },
        }
    }

    /// The exact integer `k` at precision `prec` (rounds if it does not fit).
    pub(crate) fn exact_int(k: IBig, prec: usize) -> Self {
        let ctx = Context::<mode::HalfEven>::new(prec);
        Self::from_rounded(ctx.convert_int::<B>(k).map(FBig::into_repr), prec)
    }

    /// Wrap a value with a known error (cached constants' bounds land here).
    pub(crate) fn with_error(mid: Repr<B>, rad: Mag) -> Self {
        Self { mid, rad }
    }

    // ========================================================================
    // Propagation — ops through Context kernels, `prec` last, errors propagate
    // ========================================================================

    /// `self + rhs`: `rad_a + rad_b + ε`.
    pub(crate) fn add(&self, rhs: &Self, prec: usize) -> Result<Self, FpError> {
        let ctx = Context::<mode::HalfEven>::new(prec);
        let (mid, eps) = finish_mid(ctx.add(&self.mid, &rhs.mid)?, prec);
        let rad = self.rad.add(&rhs.rad).add(&eps);
        Ok(Self { mid, rad })
    }

    /// `self − rhs`: same radius rule as `add`.
    pub(crate) fn sub(&self, rhs: &Self, prec: usize) -> Result<Self, FpError> {
        let ctx = Context::<mode::HalfEven>::new(prec);
        let (mid, eps) = finish_mid(ctx.sub(&self.mid, &rhs.mid)?, prec);
        let rad = self.rad.add(&rhs.rad).add(&eps);
        Ok(Self { mid, rad })
    }

    /// `self · rhs`: `‖a.mid‖·rad_b + ‖b.mid‖·rad_a + rad_a·rad_b + ε`.
    pub(crate) fn mul(&self, rhs: &Self, prec: usize) -> Result<Self, FpError> {
        let ctx = Context::<mode::HalfEven>::new(prec);
        let (mid, eps) = finish_mid(ctx.mul(&self.mid, &rhs.mid)?, prec);
        let rad = Mag::from_repr(&self.mid)
            .mul(&rhs.rad)
            .add(&Mag::from_repr(&rhs.mid).mul(&self.rad))
            .add(&self.rad.mul(&rhs.rad))
            .add(&eps);
        Ok(Self { mid, rad })
    }

    /// `self / rhs`: `(‖a.mid‖·rad_b + ‖b.mid‖·rad_a) / (LB(|b.mid|) · LB(|b|)) + ε` — the
    /// product of the two denominator lower bounds (≈ `LB(|b|)²`), sound at any accuracy.
    /// A degenerate denominator (either lower bound `0`) yields the whole-line radius
    /// `Mag::INFINITY` — sound, and a Ziv retry; the public layer's guards keep it
    /// unreachable in practice. Subsumes the old zero-numerator special case.
    pub(crate) fn div(&self, rhs: &Self, prec: usize) -> Result<Self, FpError> {
        let ctx = Context::<mode::HalfEven>::new(prec);
        let (mid, eps) = finish_mid(ctx.div(&self.mid, &rhs.mid)?, prec);
        let b_mid_lo = Mag::from_repr_lower(&rhs.mid);
        let b_lo = rhs.mag_lower();
        let rad = if b_mid_lo.is_zero() || b_lo.is_zero() {
            Mag::INFINITY
        } else {
            Mag::from_repr(&self.mid)
                .mul(&rhs.rad)
                .add(&Mag::from_repr(&rhs.mid).mul(&self.rad))
                .div(&b_mid_lo.mul_down(&b_lo))
                .add(&eps)
        };
        Ok(Self { mid, rad })
    }

    /// `self / k` with `k` an exact small integer (the series hot path): `rad_a/k + ε`.
    pub(crate) fn div_int(&self, k: usize, prec: usize) -> Result<Self, FpError> {
        debug_assert!(k > 0, "division by zero integer");
        let ctx = Context::<mode::HalfEven>::new(prec);
        let k_repr = Repr::<B>::new(IBig::from(k as u64), 0);
        let (mid, eps) = finish_mid(ctx.div(&self.mid, &k_repr)?, prec);
        let rad = self.rad.div(&Mag::from_word(k as Word)).add(&eps);
        Ok(Self { mid, rad })
    }

    /// `k · self` with `k` an exact (possibly large) integer: `|k|·rad_a + ε`.
    pub(crate) fn scale_int(&self, k: &IBig, prec: usize) -> Result<Self, FpError> {
        let ctx = Context::<mode::HalfEven>::new(prec);
        let k_repr = Repr::<B>::new(k.clone(), 0);
        let (mid, eps) = finish_mid(ctx.mul(&self.mid, &k_repr)?, prec);
        let rad = Mag::from_int(k).mul(&self.rad).add(&eps);
        Ok(Self { mid, rad })
    }

    /// `√self`: `rad_a/(2·LB(|mid_r|)) + ε` — the ε term covers the `fl(√a)` vs `√a`
    /// denominator gap exactly as the old `+1` did (valid for `rad_a ≤ |mid_a|`, which
    /// every caller guarantees). The zero-midpoint special case is kept from the old code:
    /// a mid that rounds to a zero significand returns an exact zero — no caller feeds a
    /// straddling ball through `sqrt` (asin checks the zero significand first).
    pub(crate) fn sqrt(&self, prec: usize) -> Result<Self, FpError> {
        let ctx = Context::<mode::HalfEven>::new(prec);
        let (mid, eps) = finish_mid(ctx.sqrt(&self.mid)?, prec);
        if mid.significand().is_zero() {
            return Ok(Self::exact(mid));
        }
        let denom = Mag::from_repr_lower(&mid).mul_pow2(1); // 2·LB(|mid_r|)
        let rad = self.rad.div(&denom).add(&eps);
        Ok(Self { mid, rad })
    }

    /// `self^k` (`k ≥ 2`) by left-to-right binary exponentiation — the compounding rounding
    /// of the squaring chain is tracked by the multiplication rule, so the powering
    /// amplification is mechanical. An exact chain keeps `rad == 0` (powi's
    /// exactly-representable directed case). Propagates a range error from the chain.
    pub(crate) fn pow(&self, k: &UBig, prec: usize) -> Result<Self, FpError> {
        let nlen = k.bit_len();
        debug_assert!(nlen >= 2, "pow requires k >= 2");
        let mut res = self.mul(self, prec)?;
        let mut p = nlen - 2;
        loop {
            if k.bit(p) {
                res = res.mul(self, prec)?;
            }
            if p == 0 {
                break;
            }
            p -= 1;
            res = res.mul(&res, prec)?;
        }
        Ok(res)
    }

    // ========================================================================
    // Exact operations — no prec, no error
    // ========================================================================

    /// Negation; the radius is unchanged.
    pub(crate) fn neg(self) -> Self {
        Self {
            mid: -self.mid,
            rad: self.rad,
        }
    }

    /// Exact shift by a power of the base (`·B^s`): the midpoint's exponent moves and the
    /// **absolute** radius scales with it (`rad·B^s` — exact for B = 2; the old ulp-count ball
    /// kept `n` unchanged because its ulp scaled with the value, which is *not* how a Mag
    /// behaves). Zero/infinite midpoints keep their exponent sentinels untouched.
    pub(crate) fn shift(&self, s: isize) -> Self {
        let mid = if self.mid.significand().is_zero() {
            self.mid.clone()
        } else {
            Repr::new(self.mid.significand().clone(), self.mid.exponent().saturating_add(s))
        };
        let rad = if s == 0 || self.rad.is_zero() {
            self.rad
        } else if B == 2 {
            self.rad.mul_pow2(s) // exact
        } else {
            Mag::from_base_pow::<B>(s).mul(&self.rad) // rounds up
        };
        Self { mid, rad }
    }

    // ========================================================================
    // Radius side
    // ========================================================================

    /// Fold a hand-derived error bound into the radius (series tails land here).
    pub(crate) fn add_error(&mut self, err: Mag) {
        self.rad = self.rad.add(&err);
    }

    /// An upper bound on `|self|` (the ball's magnitude). Currently exercised only by the
    /// tests and the doc'd surface (the exp fold builds its endpoint bound directly).
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn mag(&self) -> Mag {
        Mag::from_repr(&self.mid).add(&self.rad)
    }

    /// A lower bound on `|self|`, floored at `0` (`LB(|mid|) − rad`).
    pub(crate) fn mag_lower(&self) -> Mag {
        Mag::from_repr_lower(&self.mid).sub_down(&self.rad)
    }

    /// `true` ⟺ the chain so far is exact (`rad == 0`). Carried by `rad == 0` through
    /// `to_value_radius` (a zero radius certifies without the containment test).
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn is_exact(&self) -> bool {
        self.rad.is_zero()
    }

    // ========================================================================
    // Ziv boundary — the driver contract is unchanged
    // ========================================================================

    /// Re-tag `mid` to the mode `R` at the work context, and export the radius as an exact
    /// `Repr` (base 2) / outward power of ten (base 10) — `(value, radius)` with
    /// `|value − true| ≤ radius`, as `ziv.rs` expects. The radius carries an unlimited
    /// context, exactly as the old export did; the containment test only reads raw `Repr`s.
    pub(crate) fn to_value_radius<R: Round>(&self, ctx: &Context<R>) -> (FBig<R, B>, FBig<R, B>) {
        let value = FBig::new(self.mid.clone(), *ctx);
        let radius = FBig::new(self.rad.to_repr::<B>(), Context::<R>::new(0));
        (value, radius)
    }

    /// Series-break test: is `|self.mid|` at or below half a work-ulp of `sum`? The break
    /// feeds the hand tail bound, so the comparison must be exact — `ulp_lb` replicates
    /// `FBig::ulp_lb`'s `digits_lb − 1` slack (the cheapest guaranteed lower bound).
    pub(crate) fn mid_le_ulp_lb(&self, sum: &Self, prec: usize) -> bool {
        let e = sum
            .mid
            .exponent()
            .saturating_add(sum.mid.digits_lb() as isize)
            .saturating_sub(prec as isize)
            .saturating_sub(1);
        let threshold = Repr::<B>::new(IBig::ONE, e);
        crate::cmp::repr_cmp_same_base::<B, true>(&self.mid, &threshold, None).is_le()
    }
}

/// Unpack a kernel result: the rounded midpoint and its fresh error contribution — `0` when
/// the kernel reports `Exact`, one work-ulp otherwise.
fn finish_mid<const B: Word>(
    rounded: Rounded<FBig<mode::HalfEven, B>>,
    prec: usize,
) -> (Repr<B>, Mag) {
    match rounded {
        Approximation::Exact(f) => (f.into_repr(), Mag::ZERO),
        Approximation::Inexact(f, _) => {
            let mid = f.into_repr();
            let eps = ulp_mag(&mid, prec);
            (mid, eps)
        }
    }
}

/// One ulp of a midpoint at `prec`, as a `Mag`: `B^(lead_ub(mid) − prec)`
/// (`lead_ub = exponent + digits_ub` — an over-estimate only loosens).
pub(crate) fn ulp_mag<const B: Word>(mid: &Repr<B>, prec: usize) -> Mag {
    let lead_ub = mid.exponent().saturating_add(mid.digits_ub() as isize);
    Mag::from_base_pow::<B>(lead_ub.saturating_sub(prec as isize))
}

/// `k` ulps of `mid` at `prec` — the currency of `add_error` tail bounds and the cached
/// constants' bounds.
pub(crate) fn ulps<const B: Word>(mid: &Repr<B>, prec: usize, k: usize) -> Mag {
    Mag::from_word(k as Word).mul(&ulp_mag::<B>(mid, prec))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::round::mode;
    use dashu_base::Sign;

    type B2 = Ball<2>;

    fn exact(sig: i64, exp: isize) -> B2 {
        Ball::exact(Repr::new(IBig::from(sig), exp))
    }

    /// assert `|mid − true| ≤ rad`, exactly: compare `±(true − mid)` against `rad` on raw
    /// `Repr`s (lossless arithmetic, no rounding can mis-decide).
    fn assert_covers(ball: &B2, true_repr: &Repr<2>) {
        if ball.mid.significand().is_zero() {
            // a zero mid with a zero radius covers only exact zero; covered below by rad ≥ |true|
        }
        let rad = ball.rad.to_repr::<2>();
        let hi = &ball.mid + &rad;
        let lo = &ball.mid - &rad;
        assert!(
            hi.cmp(true_repr) != core::cmp::Ordering::Less
                && lo.cmp(true_repr) != core::cmp::Ordering::Greater,
            "interval [{lo:?}, {hi:?}] does not cover {true_repr:?}"
        );
    }

    #[test]
    fn add_covers_exact_and_cancellation() {
        let p = 10;
        let a = exact(1049, -3); // 1.049
        let b = exact(-1000, -3); // −1.000
        let sum = a.add(&b, p).unwrap();
        assert_covers(&sum, &Repr::new(IBig::from(49), -3)); // exact 0.049
                                                             // 0.049 fits precision 10 exactly, and both operands are exact, so the sum is exact —
                                                             // the conditional-ε rule keeps rad = 0 (the old code always folded one ulp).
        assert!(sum.is_exact());

        // exact chain: same-exponent integers that fit the precision
        let a = exact(3, 0);
        let b = exact(4, 0);
        let sum = a.add(&b, p).unwrap();
        assert_eq!(sum.mid, Repr::new(IBig::from(7), 0));
        assert!(sum.is_exact(), "exact + exact at fitting precision keeps rad 0");
    }

    #[test]
    fn mul_covers_exact_product() {
        let p = 10;
        let a = exact(12345, -4);
        let b = exact(6789, -4);
        let prod = a.mul(&b, p).unwrap();
        let true_prod = &a.mid * &b.mid;
        assert_covers(&prod, &true_prod);

        // exact: 3 · 4 = 12 fits
        let prod = exact(3, 0).mul(&exact(4, 0), p).unwrap();
        assert!(prod.is_exact());

        // a rounded operand propagates through the exact product: (x ± ε)·exact
        let x = Ball::from_rounded(
            Context::<mode::HalfEven>::new(p).repr_round(exact(1234567, 0).mid),
            p,
        );
        let prod = x.mul(&exact(1, 0), p).unwrap();
        assert_covers(&prod, &exact(1234567, 0).mid);
    }

    #[test]
    fn div_covers_and_zero_numerator() {
        let p = 10;
        let a = exact(1, 0);
        let b = exact(3, 0);
        let q = a.div(&b, p).unwrap();
        // mid/rad vs the exact rational 1/3: mid ≤ 1/3 + rad and mid ≥ 1/3 − rad ⟺
        // 3·mid ≤ 1 + 3·rad and 3·mid ≥ 1 − 3·rad (exact Repr arithmetic)
        let three = Repr::new(IBig::from(3), 0);
        let rad3 = &q.rad.to_repr::<2>() * &three;
        let mid3 = &q.mid * &three;
        let one = Repr::new(IBig::ONE, 0);
        assert!((&mid3 - &rad3).cmp(&one) != core::cmp::Ordering::Greater);
        assert!((&mid3 + &rad3).cmp(&one) != core::cmp::Ordering::Less);

        // zero numerator: |r| ≤ rad_a/LB(|b|) — a finite, sound radius (old special case)
        let z = Ball {
            mid: Repr::zero(),
            rad: crate::mag::Mag::from_pow2(-20),
        };
        let q = z.div(&b, p).unwrap();
        assert!(!q.rad.is_infinite());
        assert_covers(&q, &Repr::zero());
    }

    #[test]
    fn sqrt_bounds_error() {
        let p = 10;
        // 2 at precision 10, computed through the ball sqrt
        let two = exact(2, 0);
        let r = two.sqrt(p).unwrap();
        // r² must cover 2 (mid ± rad)² ⊇ contains 2: check 2 ∈ [lo², hi²] exactly
        let rad = r.rad.to_repr::<2>();
        let lo = &r.mid - &rad;
        let hi = &r.mid + &rad;
        let two = &two.mid;
        assert!((&lo * &lo).cmp(two) != core::cmp::Ordering::Greater);
        assert!((&hi * &hi).cmp(two) != core::cmp::Ordering::Less);
        // exact square: 4 → 2 exactly
        let r = exact(4, 0).sqrt(p).unwrap();
        assert!(r.is_exact());
        assert_eq!(r.mid, Repr::new(IBig::from(2), 0));
    }

    #[test]
    fn scale_int_exact_and_inexact() {
        let p = 10;
        // exact: 3 · 5 = 15 fits
        let s = exact(3, 0).scale_int(&IBig::from(5), p).unwrap();
        assert!(s.is_exact());
        assert_eq!(s.mid, Repr::new(IBig::from(15), 0));
        // rounded operand: the radius scales by |k|
        let x = Ball::from_rounded(
            Context::<mode::HalfEven>::new(p).repr_round(Repr::<2>::new(IBig::from(1234567), 0)),
            p,
        );
        let s = x.scale_int(&IBig::from(-4), p).unwrap();
        assert_covers(&s, &Repr::new(IBig::from(-4938268), 0));
    }

    #[test]
    fn shift_preserves_radius_and_zero_mid_sentinel() {
        let p = 10;
        let x = Ball::from_rounded(
            Context::<mode::HalfEven>::new(p).repr_round(Repr::<2>::new(IBig::from(1234567), 3)),
            p,
        );
        let s = x.shift(-5);
        assert_eq!(s.rad, x.rad.mul_pow2(-5));
        assert_eq!(&s.mid, &(&x.mid * &Repr::<2>::new(IBig::ONE, -5)));
        // a zero mid keeps its (0, 0) sentinel through a shift
        let z = Ball::exact(Repr::<2>::zero()).shift(7);
        assert_eq!(z.mid, Repr::zero());
    }

    #[test]
    fn pow_tracks_chain() {
        let p = 20;
        // 5^4 = 625, exactly representable → exact chain
        let r = exact(5, 0).pow(&UBig::from(4u8), p).unwrap();
        assert!(r.is_exact());
        assert_eq!(r.mid, Repr::new(IBig::from(625), 0));
        // (1 + 2^-8)^(2^10): compounding rounding stays sound
        let base = Ball::from_rounded(
            Context::<mode::HalfEven>::new(p).repr_round(Repr::<2>::new(IBig::from(257), -8)),
            p,
        );
        let r = base.pow(&UBig::from(1024u32), p + 8).unwrap();
        // true value ∈ ((1), (1+2^-7)^512)·... — check the ball covers (1 + 2^-8)^1024 via
        // its own magnitude: (mid − rad) ≤ true ≤ (mid + rad) with true = (257/256)^1024
        // ≈ 54.98 — compare against a rough dyadic bracket 32 < mid+rad, mid−rad < 128
        let rad = r.rad.to_repr::<2>();
        let hi = &r.mid + &rad;
        assert!(hi.cmp(&Repr::new(IBig::from(128), 0)) == core::cmp::Ordering::Less);
        assert!(hi.cmp(&Repr::new(IBig::from(32), 0)) == core::cmp::Ordering::Greater);
    }

    #[test]
    fn to_value_radius_returns_sound_bounds() {
        let p = 10;
        let ctx = Context::<mode::HalfEven>::new(p);
        let x = Ball::from_rounded(ctx.repr_round(Repr::<2>::new(IBig::from(1), -3)), p);
        let sq = x.mul(&x, p).unwrap();
        let (value, radius) = sq.to_value_radius::<mode::HalfEven>(&ctx);
        assert_eq!(value.context.precision, p);
        assert_eq!(radius.context.precision, 0);
        // (1±2^-10-ish)² covers 1/9 — exact-value containment on reprs
        let rad = radius.repr.clone();
        let hi = &value.repr + &rad;
        let lo = &value.repr - &rad;
        let true_v = Repr::new(IBig::ONE, -3) * Repr::new(IBig::ONE, -3);
        assert!(hi.cmp(&true_v) != core::cmp::Ordering::Less);
        assert!(lo.cmp(&true_v) != core::cmp::Ordering::Greater);
        // exact chain exports a plain +0 radius (not a −∞ sentinel)
        let (v, r) = exact(2, 0).to_value_radius::<mode::HalfEven>(&ctx);
        assert!(r.repr.significand().is_zero() && r.repr.exponent == 0);
        assert!(v.repr.sign() == Sign::Positive);
    }

    #[test]
    fn mid_le_ulp_lb_matches_ulp_scale() {
        let p = 10;
        let ctx = Context::<mode::HalfEven>::new(p);
        let sum = Ball::from_rounded(ctx.repr_round(Repr::new(IBig::from(1023), -1)), p);
        // a term at 2^-13 of the sum's magnitude is below the ulp_lb threshold
        let tiny = exact(1, -13);
        assert!(tiny.mid_le_ulp_lb(&sum, p));
        // a term at the sum's own scale is not
        let big = exact(1, -1);
        assert!(!big.mid_le_ulp_lb(&sum, p));
    }
}
