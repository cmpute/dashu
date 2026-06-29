//! Complex natural logarithm `log(z) = ln|z| + i·arg(z)` (principal branch; cut on `]−∞, 0]`).

use crate::cbig::CBig;
use crate::repr::{combine_parts, exact, reborrow_cache, riemann, CfpResult, Context};
use dashu_float::round::Round;
use dashu_float::{ConstCache, FBig, Repr};
use dashu_int::Word;

/// Guard digits (base-B) for `log`. Composes `hypot` (for `|z|`), `ln`, and `atan2`.
const LOG_GUARD: usize = 14;

impl<R: Round> Context<R> {
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

        let gctx = self.guard(LOG_GUARD);
        let p = self.precision();
        // ln|z|
        let r = gctx.hypot(z.re(), z.im())?.value();
        let ln_r = gctx.ln(r.repr(), reborrow_cache(&mut cache))?.value();
        // arg(z) = atan2(im, re)
        let arg = gctx
            .atan2(z.im(), z.re(), reborrow_cache(&mut cache))?
            .value();
        let re = ln_r.with_precision(p);
        let im = arg.with_precision(p);
        Ok(combine_parts(re, im))
    }
}

impl<R: Round, const B: Word> CBig<R, B> {
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
        assert!(C::ONE.ln() == C::ZERO);
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
}
