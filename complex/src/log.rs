//! Complex natural logarithm `log(z) = ln|z| + i·arg(z)` (principal branch; cut on `]−∞, 0]`).

use crate::cbig::CBig;
use crate::repr::{combine_parts, exact, reborrow_cache, riemann, CfpResult, Context};
use dashu_float::round::ErrorBounds;
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
            // log(±0) = -∞ + i·arg(±0); arg(0,0) is undefined — report the real -∞ via ln(0)
            return Ok(exact(
                FBig::from_repr(Repr::neg_infinity(), self.float()),
                FBig::from_repr(Repr::zero(), self.float()),
            ));
        }
        if z.is_infinite() {
            return Ok(riemann(*self)); // log(∞) = +∞ (Riemann point)
        }

        // `ln|z| + i·arg(z)`. The float `hypot`/`ln`/`atan2` are correctly-rounded at the working
        // precision. The imaginary part (`atan2` of the exact parts) carries only `atan2`'s own
        // rounding; the real part `ln|z|` additionally propagates `hypot`'s relative error through
        // `ln`, which dominates near `|z| = 1` (where `ln|z| → 0`) — so its radius carries an extra
        // absolute `B^{1-pw}` term. The Ziv driver asserts a limited context (the `log(0)`/`log(∞)`
        // shortcuts above are exact and need no precision).
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
}
