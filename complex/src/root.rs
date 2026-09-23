//! Complex square root (principal branch; cut on `]−∞, 0]`).

use crate::ball::CBall;
use crate::cbig::CBig;
use crate::repr::{combine_parts, exact, CfpResult, Context};
use dashu_float::round::{ErrorBounds, Round};
use dashu_float::{Context as FloatCtxt, FBig};
use dashu_int::Word;

/// Guard digits (base-B) for `sqrt`. Composes `hypot` + two real `sqrt`s + adds; a modest fixed
/// guard absorbs the accumulated rounding.
const SQRT_GUARD: usize = 12;

impl<R: ErrorBounds> Context<R> {
    /// Principal square root of a complex number (context layer).
    ///
    /// The result has non-negative real part; when the real part is zero the imaginary part is
    /// non-negative. The branch cut lies on `]−∞, 0]`; `sqrt(conj z) == conj(sqrt z)` holds, which
    /// signed zero makes continuous across the cut.
    pub fn sqrt<const B: Word>(&self, z: &CBig<R, B>) -> CfpResult<R, B> {
        if let Some(special) = sqrt_special(z, *self) {
            return special;
        }

        // Principal sqrt through the cancellation-free form (`CBall::sqrt`: for x ≥ 0,
        // `a = sqrt((r+x)/2)`, `b = y/(2a)`; for x < 0 mirrored with `b` carrying the sign of
        // `y`), which avoids the near-cancellation in `r−x` when `|y| ≪ |x|`. Every composition
        // step is a tracked ball op, so the radius is mechanical: an exactly-representable
        // result (√4 = 2, √(3+4i) = 2+i, …) carries a zero radius — the only thing the directed
        // rounding modes can certify against their one-sided preimages. The Ziv driver asserts a
        // limited context (the special-value shortcut above is exact).
        let p = self.precision();
        let [re, im] = self.ziv(SQRT_GUARD, |guard| {
            let pw = p + guard;
            let gctx = FloatCtxt::<R>::new(pw);
            let out = CBall::from_parts(z.re(), z.im(), pw).sqrt(&gctx, pw)?;
            Ok(out.to_parts_radius(&gctx))
        })?;
        Ok(combine_parts(re, im))
    }
}

impl<R: ErrorBounds, const B: Word> CBig<R, B> {
    /// Principal square root (convenience layer).
    ///
    /// # Panics
    ///
    /// Panics if the precision is unlimited, or on an out-of-domain / indeterminate special value.
    #[inline]
    pub fn sqrt(&self) -> Self {
        self.context().unwrap_cfp(self.context().sqrt(self))
    }
}

/// `csqrt` special values for a zero input (preserving signed zeros). An infinite input is a
/// terminal value and is **not** short-circuited here — it returns `None` so the normal path runs
/// and the float `sqrt` rejects it (panicking at the convenience layer), matching `dashu-float`.
fn sqrt_special<R: Round, const B: Word>(
    z: &CBig<R, B>,
    ctx: Context<R>,
) -> Option<CfpResult<R, B>> {
    let f = ctx.float();
    // sqrt(±0 + i·0) = ±0 + i·0 (preserve the real sign of zero)
    if z.is_zero() {
        return Some(Ok(exact(
            FBig::from_repr(z.re().clone(), f),
            FBig::from_repr(z.im().clone(), f),
        )));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use dashu_float::round::mode;
    use dashu_float::Repr;

    type C = CBig<mode::HalfAway, 10>;
    type F = FBig<mode::HalfAway, 10>;

    fn c(re: i32, im: i32) -> C {
        let mk = |v: i32| -> F { F::from(v).with_precision(53).value() };
        CBig::from_parts(mk(re), mk(im))
    }

    #[test]
    fn sqrt_basic() {
        // sqrt(3+4i) = 2+i  (since (2+i)² = 3+4i)
        let z = c(3, 4);
        let s = z.sqrt();
        let chk = &s * &s;
        assert!(chk == z);
    }

    #[test]
    #[should_panic(expected = "arithmetic operations with the infinity are not allowed")]
    fn sqrt_infinite_imaginary_dominates() {
        // ∞ is terminal: sqrt of any infinite input is rejected (matching dashu-float's `sqrt`),
        // regardless of which component is infinite.
        let ctx = Context::<mode::HalfAway>::new(53);
        let _ = C::new(Repr::infinity(), Repr::infinity(), ctx).sqrt();
    }

    #[test]
    fn sqrt_real() {
        // sqrt(9+0i) = 3+0i
        let z = c(9, 0);
        let s = z.sqrt();
        assert!(s == c(3, 0));
    }

    #[test]
    fn sqrt_negative_real_is_imaginary() {
        // sqrt(-4+0i) = 0+2i
        let z = c(-4, 0);
        let s = z.sqrt();
        assert!(s.re().significand().is_zero());
        assert_eq!(s.im().significand(), &2.into());
    }

    #[test]
    fn sqrt_conj_identity() {
        // sqrt(conj z) == conj(sqrt z)
        let z = c(3, 4);
        let lhs = z.conj().sqrt();
        let rhs = z.sqrt().conj();
        assert!(lhs == rhs);
    }

    #[test]
    fn sqrt_zero() {
        let s = C::ZERO.sqrt();
        assert!(s.is_zero());
    }

    #[test]
    #[should_panic(expected = "arithmetic operations with the infinity are not allowed")]
    fn sqrt_pos_infinity() {
        // ∞ is terminal: sqrt(+∞) is rejected (matching dashu-float's `sqrt`).
        let ctx = Context::<mode::HalfAway>::new(53);
        let inf = C::new(Repr::infinity(), Repr::zero(), ctx);
        let _ = inf.sqrt();
    }

    // `sqrt` at unlimited precision panics via `guard` (the special-value shortcut above only
    // catches zero, so a finite nonzero input reaches the guard context).
    #[test]
    #[should_panic(expected = "precision cannot be 0")]
    fn complex_sqrt_unlimited_panics() {
        let _ = C::I.sqrt();
    }

    // The mechanically tracked radius must certify at the target precision across the width
    // sweep: each result equals the same op computed at `p + 60` and re-rounded to `p` (both
    // sides are correctly rounded, so they must agree bit for bit).
    #[test]
    fn sqrt_matches_oracle_across_precisions() {
        // Both bases: the `sqrt` fold divides by a lower bound of `2·√mid` taken from the
        // base-power bracket, so a non-binary base exercises a looser bound.
        // Rungs: bit precisions at base 2, digit precisions at base 10, picked to span the
        // same significand widths (7 ≈ 20 bits, …) so both bases cover the same paths.
        macro_rules! sweep {
            ($base:literal, $name:literal, $precs:expr) => {{
                type C2 = CBig<mode::HalfEven, $base>;
                type F2 = FBig<mode::HalfEven, $base>;
                let inputs = [
                    (3i64, 4i64),
                    (5, -12),
                    (7, 1),
                    (1, 1),
                    (-3, 4),
                    (9, 0),
                    (121, 0),
                ];
                for p in $precs {
                    for (re, im) in inputs {
                        let mk = |v: i64| F2::from(v).with_precision(p).value();
                        let z = C2::from_parts(mk(re), mk(im));
                        let mk_hi = |v: i64| F2::from(v).with_precision(p + 60).value();
                        let (hre, him) = C2::from_parts(mk_hi(re), mk_hi(im)).sqrt().into_parts();
                        let expect_re = hre.with_precision(p).value();
                        let expect_im = him.with_precision(p).value();
                        let got = z.sqrt();
                        assert_eq!(got.re(), expect_re.repr(), "[$name] re p={p} z=({re},{im})");
                        assert_eq!(got.im(), expect_im.repr(), "[$name] im p={p} z=({re},{im})");
                    }
                }
            }};
        }
        sweep!(2, "base 2", [20, 50, 100, 500]);
        sweep!(10, "base 10", [7, 17, 34, 160]);
    }

    // An exactly-representable result certifies under the outward modes through its zero
    // radius — the hand-written `ulp·10` radius could never fit a one-sided preimage (it
    // exhausted the Ziv retry budget instead).
    #[test]
    fn sqrt_exact_results_certify_directed() {
        macro_rules! check {
            ($mode:ty) => {{
                type C = CBig<$mode, 10>;
                type F = FBig<$mode, 10>;
                let mk = |v: i32| F::from(v).with_precision(30).value();
                let ctx = Context::<$mode>::new(30);
                // √4 = 2
                let got = ctx
                    .sqrt(&C::from_parts(mk(4), mk(0)))
                    .unwrap()
                    .value()
                    .clone();
                assert_eq!(got.re(), mk(2).repr());
                assert!(got.im().significand().is_zero());
                // √(−4) = 2i
                let got = ctx
                    .sqrt(&C::from_parts(mk(-4), mk(0)))
                    .unwrap()
                    .value()
                    .clone();
                assert!(got.re().significand().is_zero());
                assert_eq!(got.im(), mk(2).repr());
            }};
        }
        check!(mode::Up);
        check!(mode::Down);
        check!(mode::Zero);
        check!(mode::HalfEven);
    }
}
