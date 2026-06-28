//! Complex square root (principal branch; cut on `]−∞, 0]`).

use crate::cbig::CBig;
use crate::repr::{combine_parts, exact, CfpResult, Context};
use dashu_base::Sign;
use dashu_float::round::Round;
use dashu_float::{FBig, Repr};
use dashu_int::Word;

/// Guard digits (base-B) for `sqrt`. Composes `hypot` + two real `sqrt`s + adds; a modest fixed
/// guard absorbs the accumulated rounding.
const SQRT_GUARD: usize = 12;

/// A signed-infinity [`Repr`] (the public-API stand-in for the private `infinity_with_sign`).
fn signed_inf<const B: Word>(sign: Sign) -> Repr<B> {
    match sign {
        Sign::Positive => Repr::infinity(),
        Sign::Negative => Repr::neg_infinity(),
    }
}

impl<R: Round> Context<R> {
    /// Principal square root of a complex number (context layer).
    ///
    /// The result has non-negative real part; when the real part is zero the imaginary part is
    /// non-negative. The branch cut lies on `]−∞, 0]`; `sqrt(conj z) == conj(sqrt z)` holds, which
    /// signed zero makes continuous across the cut.
    pub fn sqrt<const B: Word>(&self, z: &CBig<R, B>) -> CfpResult<R, B> {
        if let Some(special) = sqrt_special(z, *self) {
            return special;
        }

        let gctx = self.guard(SQRT_GUARD);
        let p = self.precision();
        let two = FBig::from_repr(Repr::new(2.into(), 0), gctx);
        let x = z.re();
        let y = z.imag();

        // r = |z| (overflow-safe). Use the cancellation-free form: for x ≥ 0 compute `a` from
        // `(r+x)/2` (large) and `b = y/(2a)`; for x < 0 compute `b` from `(r-x)/2` (large) and
        // `a = y/(2b)`. This avoids subtracting nearly-equal magnitudes when |y| ≪ |x|.
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
        let re = a.with_precision(p);
        let im = b.with_precision(p);
        Ok(combine_parts(re, im))
    }
}

impl<R: Round, const B: Word> CBig<R, B> {
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

/// Annex G `csqrt` special-value table (the subset expressible without NaN).
fn sqrt_special<R: Round, const B: Word>(
    z: &CBig<R, B>,
    ctx: Context<R>,
) -> Option<CfpResult<R, B>> {
    let f = ctx.float();
    // sqrt(±0 + i·0) = ±0 + i·0 (preserve the real sign of zero)
    if z.is_zero() {
        return Some(Ok(exact(
            FBig::from_repr(z.re().clone(), f),
            FBig::from_repr(z.imag().clone(), f),
        )));
    }
    if !z.is_infinite() {
        return None;
    }

    let x_pos_inf = z.re().is_infinite() && z.re().sign() == Sign::Positive;
    let x_neg_inf = z.re().is_infinite() && z.re().sign() == Sign::Negative;
    let y_sign = z.imag().sign();

    let (re, im) = if x_pos_inf {
        // sqrt(+inf + iy) = +inf + i·0 (the zero carries the sign of y)
        (
            Repr::infinity(),
            if y_sign == Sign::Negative {
                Repr::neg_zero()
            } else {
                Repr::zero()
            },
        )
    } else if x_neg_inf {
        // sqrt(-inf + iy) = +0 + i·sign(y)·inf
        (Repr::zero(), signed_inf::<B>(y_sign))
    } else {
        // y infinite, x finite: sqrt(x ± i·inf) = +inf ± i·inf
        (Repr::infinity(), signed_inf::<B>(y_sign))
    };
    Some(Ok(exact(FBig::from_repr(re, f), FBig::from_repr(im, f))))
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
        assert_eq!(s.imag().significand(), &2.into());
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
    fn sqrt_pos_infinity() {
        let inf = CBig::from(F::INFINITY);
        let s = inf.sqrt();
        assert!(s.re().is_infinite());
        assert!(s.imag().is_zero());
    }
}
