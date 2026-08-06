//! Complex square root (principal branch; cut on `]−∞, 0]`).

use crate::cbig::CBig;
use crate::repr::{combine_parts, exact, CfpResult, Context};
use dashu_base::Sign;
use dashu_float::round::{ErrorBounds, Round};
use dashu_float::{Context as FloatCtxt, FBig, Repr};
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

        // Principal sqrt via the cancellation-free form: for x ≥ 0, `a = sqrt((r+x)/2)`,
        // `b = y/(2a)`; for x < 0, `b = sign(y)·sqrt((r-x)/2)`, `a = y/(2b)` — this avoids the
        // near-cancellation in `r−x` when `|y| ≪ |x|`. The float `hypot`/`sqrt` are correctly-rounded
        // at the working precision, and the adds/divs/mul each round at a few working-ULPs, so a
        // small constant radius certifies both parts. The Ziv driver asserts a limited context (the
        // special-value shortcut above is exact).
        let p = self.precision();
        let [re, im] = self.ziv(SQRT_GUARD, |guard| {
            let gctx = FloatCtxt::<R>::new(p + guard);
            let two = FBig::from_repr(Repr::new(2.into(), 0), gctx);
            let x = z.re();
            let y = z.im();
            let r = gctx.hypot(x, y)?.value();
            let (a, b) = if x.sign() != Sign::Negative {
                // x ≥ 0
                let rpx = gctx.add(r.repr(), x)?.value();
                let half_rpx = gctx.div(rpx.repr(), two.repr())?.value();
                let a = gctx.sqrt(half_rpx.repr())?.value();
                let two_a = gctx.mul(two.repr(), a.repr())?.value();
                let b = gctx.div(y, two_a.repr())?.value();
                (a, b)
            } else {
                // x < 0: b carries the sign of y
                let rmx = gctx.sub(r.repr(), x)?.value(); // r − x = r + |x|
                let half_rmx = gctx.div(rmx.repr(), two.repr())?.value();
                let b_mag = gctx.sqrt(half_rmx.repr())?.value();
                let b = if y.sign() == Sign::Negative {
                    -b_mag
                } else {
                    b_mag
                };
                let two_b = gctx.mul(two.repr(), b.repr())?.value();
                let a = gctx.div(y, two_b.repr())?.value();
                (a, b)
            };
            let a_rad = a.ulp() * 10;
            let b_rad = b.ulp() * 10;
            Ok([(a, a_rad), (b, b_rad)])
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
}
