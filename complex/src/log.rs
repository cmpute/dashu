//! Complex natural logarithm `log(z) = ln|z| + i·arg(z)` (principal branch; cut on `]−∞, 0]`).

use crate::cbig::CBig;
use crate::repr::{combine_parts, exact, reborrow_cache, CfpResult, Context};
use dashu_base::{Approximation, Sign};
use dashu_float::round::{ErrorBounds, Rounding};
use dashu_float::{ConstCache, Context as FloatCtxt, FBig, Repr};
use dashu_int::{IBig, Word};

/// Guard digits (base-B) for `log`. Composes `hypot` (for `|z|`), `ln`, and `atan2`.
const LOG_GUARD: usize = 14;

impl<R: ErrorBounds> Context<R> {
    /// Complex natural logarithm under this context (context layer). `log z = ln|z| + i·arg(z)`,
    /// with the imaginary part in `]−π, π]`. The cache threads into `ln` and `atan2`.
    ///
    /// Special values: `log(0) = -∞ + i·0`; `log(±∞) = +∞`; the branch cut on `]−∞, 0]` is handled
    /// by the signed-zero `atan2` (so `log(-r ± i0) = ln r ± iπ`).
    pub fn log<const B: Word>(
        &self,
        z: &CBig<R, B>,
        mut cache: Option<&mut ConstCache>,
    ) -> CfpResult<R, B> {
        if z.is_zero() {
            let (re, im) = (z.re(), z.im());
            let neg_inf = FBig::from_repr(Repr::neg_infinity(), self.float());
            // log(±0 ± i·0) = -∞ + i·arg(±0 ± i·0), where arg is the signed-zero atan2:
            // atan2(±0, +0) = ±0 (exact) and atan2(±0, −0) = ±π (inexact), both carrying the
            // imaginary part's sign — so `clog(-0 ± i·0) = -∞ + i·±π`.
            if re.is_neg_zero() {
                if self.precision() == 0 {
                    // π isn't exactly representable at unlimited precision — keep the historical
                    // `+0` imaginary part (the zero shortcut bypasses the Ziv precision check).
                    return Ok(exact(
                        neg_inf,
                        FBig::from_repr(Repr::zero_with_sign(Sign::Positive), self.float()),
                    ));
                }
                let mut pi = self.float().pi::<B>(reborrow_cache(&mut cache));
                if im.is_neg_zero() {
                    // Negate π, flipping the rounding adjustment (`AddOne` ↔ `SubOne`): negating a
                    // value negates its significand, so the adjustment applied to the truncated
                    // significand flips sign (`Approximation::map` would keep the error unchanged).
                    pi = match pi {
                        Approximation::Exact(v) => Approximation::Exact(-v),
                        Approximation::Inexact(v, Rounding::AddOne) => {
                            Approximation::Inexact(-v, Rounding::SubOne)
                        }
                        Approximation::Inexact(v, Rounding::SubOne) => {
                            Approximation::Inexact(-v, Rounding::AddOne)
                        }
                        Approximation::Inexact(v, Rounding::NoOp) => {
                            Approximation::Inexact(-v, Rounding::NoOp)
                        }
                    };
                }
                return Ok(combine_parts(Approximation::Exact(neg_inf), pi));
            }
            return Ok(exact(
                neg_inf,
                FBig::from_repr(Repr::zero_with_sign(im.sign()), self.float()),
            ));
        }
        // An infinite input is a terminal value: it is not short-circuited here (the `log(0)`
        // shortcut above is exact and needs no precision) — it falls through and the float `hypot`
        // rejects it, panicking at the convenience layer.

        // `ln|z| + i·arg(z)`. The float `hypot`/`ln`/`atan2` are correctly-rounded at the working
        // precision. The imaginary part (`atan2` of the exact parts) carries only `atan2`'s own
        // rounding; the real part `ln|z|` additionally propagates `hypot`'s relative error through
        // `ln`, which dominates near `|z| = 1` (where `ln|z| → 0`) — so its radius carries an extra
        // absolute `B^{1-pw}` term. The Ziv driver asserts a limited context.
        let p = self.precision();
        let [re, im] = self.ziv(LOG_GUARD, |guard| {
            let pw = p + guard;
            let gctx = FloatCtxt::<R>::new(pw);
            // ln|z|
            let r = gctx.hypot(z.re(), z.im())?.value();
            let ln_r = gctx.ln(r.repr(), reborrow_cache(&mut cache))?.value();
            // arg(z) = atan2(im, re)
            let arg = gctx
                .atan2(z.im(), z.re(), reborrow_cache(&mut cache))?
                .value();
            // The float transcendentals return unlimited-precision exact constants for exact cases
            // (e.g. `ln 1 = 0`, `atan2(0,1) = 0`); re-root to the working precision so `.ulp()`
            // (which rejects unlimited) is well-defined.
            let ln_r = ln_r.with_precision(pw).value();
            let arg = arg.with_precision(pw).value();
            // `B^{1-pw}` upper-bounds `hypot`'s propagated error `ulp(r)/|r| ≤ B^{1-pw}`, which
            // dominates `ulp(ln_r)` when `|ln_r| < 1` (`|z| ≈ 1`).
            let propagated = FBig::<R, B>::from_parts(IBig::from(1), 1 - pw as isize);
            let re_rad = ln_r.ulp() * 4 + propagated * 4;
            let im_rad = arg.ulp() * 4;
            Ok([(ln_r, re_rad), (arg, im_rad)])
        })?;
        Ok(combine_parts(re, im))
    }
}

impl<R: ErrorBounds, const B: Word> CBig<R, B> {
    /// Complex natural logarithm (principal branch; convenience layer).
    ///
    /// # Panics
    ///
    /// Panics if the precision is unlimited.
    #[inline]
    pub fn ln(&self) -> Self {
        self.context().unwrap_cfp(self.context().log(self, None))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dashu_base::{Abs, AbsOrd, Sign};
    use dashu_float::round::mode;

    type C = CBig<mode::HalfAway, 10>;
    type F = FBig<mode::HalfAway, 10>;

    fn c(re: i32, im: i32) -> C {
        let mk = |v: i32| -> F { F::from(v).with_precision(53).value() };
        CBig::from_parts(mk(re), mk(im))
    }

    fn within(a: &F, b: &F, k: u32) -> bool {
        if a == b {
            return true;
        }
        let diff = (a.clone() - b.clone()).abs();
        diff.abs_cmp(&(a.ulp() * F::from(k))).is_le()
    }

    #[test]
    fn ln_one_is_zero() {
        // Use a *limited*-precision input — `log` rejects unlimited precision (it would otherwise
        // silently compute at the fixed `LOG_GUARD`).
        assert!(c(1, 0).ln() == C::ZERO);
    }

    #[test]
    fn ln_exp_roundtrip() {
        // ln(exp z) ≈ z (the imaginary 1 sits inside ]-π, π], so no 2πi wrap)
        let z = c(1, 1);
        let l = z.exp().ln();
        let (zr, zi) = z.into_parts();
        let (lr, li) = l.into_parts();
        assert!(within(&zr, &lr, 16));
        assert!(within(&zi, &li, 16));
    }

    #[test]
    fn ln_zero_is_neg_infinity() {
        let l = C::ZERO.ln();
        assert!(l.re().is_infinite());
        assert_eq!(l.re().sign(), Sign::Negative);
    }

    // log on an unlimited-precision CBig must panic, not silently compute at LOG_GUARD digits
    // (the `log(0)` / `log(∞)` shortcuts above are exact, so they don't hit this).
    #[test]
    #[should_panic(expected = "precision cannot be 0")]
    fn complex_log_unlimited_precision_panics() {
        let _ = C::ONE.ln();
    }

    #[test]
    fn log_signed_zero() {
        // Annex-G signed-zero cases: `clog(-0 ± i·0) = -∞ + i·±π`, and the exact `+0` imaginary
        // part carries the imaginary part's sign for a positive-real zero.
        let fctx = dashu_float::Context::<mode::HalfAway>::new(53);
        let (neg0, pos0) = (F::from_repr(Repr::neg_zero(), fctx), F::from_repr(Repr::zero(), fctx));
        let pi53 = F::pi(53);
        // clog(-0 + i·0) = -∞ + i·π
        let l = C::from_parts(neg0.clone(), pos0.clone()).ln();
        assert!(l.re().is_infinite() && l.re().sign() == Sign::Negative);
        assert_eq!(l.im(), pi53.repr());
        // clog(-0 − i·0) = -∞ − i·π
        let l = C::from_parts(neg0.clone(), neg0.clone()).ln();
        assert!(l.re().is_infinite() && l.re().sign() == Sign::Negative);
        assert_eq!(l.im(), (-pi53).repr());
        // clog(+0 − i·0) = -∞ − i·0
        let l = C::from_parts(pos0.clone(), neg0.clone()).ln();
        assert!(l.re().is_infinite() && l.re().sign() == Sign::Negative);
        assert!(l.im().is_neg_zero());
        // clog(+0 + i·0) = -∞ + i·0
        let l = C::from_parts(pos0.clone(), pos0).ln();
        assert!(l.re().is_infinite() && l.re().sign() == Sign::Negative);
        assert!(l.im().is_pos_zero());
    }
}
