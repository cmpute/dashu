//! Complex division and reciprocal (near-correctly rounded via Smith's method + guard re-round).

use crate::cbig::CBig;
use crate::context::{combine_parts, exact, CRounded, CfpResult, Context};
use core::ops::{Div, DivAssign};
use dashu_float::round::Round;
use dashu_float::{FBig, FpError, Repr};
use dashu_int::Word;

/// Guard digits (base-B) for `div`/`inv`. The naive complex-division error is `~(3+√5)·u`; a fixed
/// guard comfortably absorbs it for well-conditioned denominators.
const DIV_GUARD: usize = 14;

/// The Riemann point at infinity `+∞ + i·0` as an exact [`CRounded`] result.
fn riemann<R: Round, const B: Word>(context: Context<R>) -> CRounded<R, B> {
    exact(
        FBig::from_repr(Repr::infinity(), context.float()),
        FBig::from_repr(Repr::zero(), context.float()),
    )
}

/// Magnitude comparison of two real parts: `|a| >= |b|`?
fn abs_ge<const B: Word>(a: &Repr<B>, b: &Repr<B>) -> bool {
    use dashu_base::Sign;
    let a_neg = a.sign() == Sign::Negative;
    let b_neg = b.sign() == Sign::Negative;
    let a_mag = if a_neg { -a.clone() } else { a.clone() };
    let b_mag = if b_neg { -b.clone() } else { b.clone() };
    a_mag.cmp(&b_mag).is_ge()
}

impl<R: Round> Context<R> {
    /// Reciprocal `1/z = conj(z)/|z|²` under this context (context layer).
    pub fn inv<const B: Word>(&self, z: &CBig<R, B>) -> CfpResult<R, B> {
        if z.is_infinite() {
            return Ok(exact(FBig::ZERO, FBig::ZERO)); // 1/∞ = 0
        }
        if z.is_zero() {
            return Ok(riemann(*self)); // 1/0 = ∞
        }
        let gctx = self.guard(DIV_GUARD);
        let p = self.precision();
        let (x, y) = (z.re(), z.imag());
        // n = x² + y²
        let x2 = gctx.sqr(x)?.value();
        let y2 = gctx.sqr(y)?.value();
        let n = gctx.add(x2.repr(), y2.repr())?.value();
        // 1/z = (x/n) + i(-y/n)
        let re = gctx.div(x, n.repr())?.value().with_precision(p);
        let neg_y = -y.clone();
        let im = gctx.div(&neg_y, n.repr())?.value().with_precision(p);
        Ok(combine_parts(re, im))
    }

    /// Divide two complex numbers under this context (context layer), using Smith's overflow-safe
    /// method: the branch `|u| >= |v|` avoids forming `|denominator|²`.
    pub fn div<const B: Word>(&self, z: &CBig<R, B>, w: &CBig<R, B>) -> CfpResult<R, B> {
        if let Some(special) = div_special(z, w) {
            return special;
        }
        let gctx = self.guard(DIV_GUARD);
        let p = self.precision();
        let (x, y) = (z.re(), z.imag());
        let (u, v) = (w.re(), w.imag());

        // r, d depend on which of |u|, |v| is larger (Smith's method)
        let (r, d) = if abs_ge(u, v) {
            // r = v/u, d = u + r·v
            let r = gctx.div(v, u)?.value();
            let rv = gctx.mul(r.repr(), v)?.value();
            let d = gctx.add(u, rv.repr())?.value();
            (r, d)
        } else {
            // r = u/v, d = v + r·u
            let r = gctx.div(u, v)?.value();
            let ru = gctx.mul(r.repr(), u)?.value();
            let d = gctx.add(v, ru.repr())?.value();
            (r, d)
        };

        let (re, im) = if abs_ge(u, v) {
            // re = (x + r·y)/d, im = (y - r·x)/d
            let ry = gctx.mul(r.repr(), y)?.value();
            let rx = gctx.mul(r.repr(), x)?.value();
            let num_re = gctx.add(x, ry.repr())?.value();
            let num_im = gctx.sub(y, rx.repr())?.value();
            (
                gctx.div(num_re.repr(), d.repr())?.value().with_precision(p),
                gctx.div(num_im.repr(), d.repr())?.value().with_precision(p),
            )
        } else {
            // re = (r·x + y)/d, im = (r·y - x)/d
            let rx = gctx.mul(r.repr(), x)?.value();
            let ry = gctx.mul(r.repr(), y)?.value();
            let num_re = gctx.add(rx.repr(), y)?.value();
            let num_im = gctx.sub(ry.repr(), x)?.value();
            (
                gctx.div(num_re.repr(), d.repr())?.value().with_precision(p),
                gctx.div(num_im.repr(), d.repr())?.value().with_precision(p),
            )
        };
        Ok(combine_parts(re, im))
    }

    /// Divide a complex number by a real scalar (context layer): `(x+iy)/s = (x/s) + i(y/s)`.
    pub fn div_real<const B: Word>(&self, z: &CBig<R, B>, s: &FBig<R, B>) -> CfpResult<R, B> {
        if z.is_infinite() || s.repr().is_infinite() {
            if z.is_infinite() && s.repr().is_infinite() {
                return Err(FpError::Indeterminate); // ∞/∞
            }
            if s.repr().is_infinite() {
                return Ok(exact(FBig::ZERO, FBig::ZERO)); // finite/∞ = 0
            }
            // z infinite, s finite nonzero → ∞
            return Ok(riemann(*self));
        }
        if s.repr().is_zero() {
            if z.is_zero() {
                return Err(FpError::Indeterminate); // 0/0
            }
            return Ok(riemann(*self)); // z/0 (z≠0) = ∞
        }
        let gctx = self.guard(DIV_GUARD);
        let p = self.precision();
        let re = gctx.div(z.re(), s.repr())?.value().with_precision(p);
        let im = gctx.div(z.imag(), s.repr())?.value().with_precision(p);
        Ok(combine_parts(re, im))
    }
}

/// Annex-G short-circuit for `z / w`.
fn div_special<R: Round, const B: Word>(z: &CBig<R, B>, w: &CBig<R, B>) -> Option<CfpResult<R, B>> {
    let (zi, wi) = (z.is_infinite(), w.is_infinite());
    let (zz, wz) = (z.is_zero(), w.is_zero());
    let ctx = Context::max(z.context(), w.context());
    if (zi && wi) || (zz && wz) {
        Some(Err(FpError::Indeterminate)) // ∞/∞ or 0/0
    } else if wi {
        Some(Ok(exact(FBig::ZERO, FBig::ZERO))) // (finite or 0) / ∞ = 0
    } else if wz || zi {
        Some(Ok(riemann(ctx))) // (nonzero or ∞) / 0, or ∞ / finite = ∞
    } else if zz {
        Some(Ok(exact(FBig::ZERO, FBig::ZERO))) // 0 / finite = 0
    } else {
        None
    }
}

impl<R: Round, const B: Word> CBig<R, B> {
    /// Reciprocal `1/z` (convenience layer).
    #[inline]
    pub fn inv(&self) -> Self {
        self.context().unwrap_cfp(self.context().inv(self))
    }
}

// CBig / CBig operators
crate::helper_macros::impl_cbig_binop!(Div, div, DivAssign, div_assign);

// --- scalar division by a real FBig (mixed-type operators) ---

/// CBig / FBig (componentwise) and FBig / CBig (= (s+0i)/z, reusing complex division).
macro_rules! impl_scalar_div {
    () => {
        impl<R: Round, const B: Word> Div<&FBig<R, B>> for &CBig<R, B> {
            type Output = CBig<R, B>;
            #[inline]
            fn div(self, rhs: &FBig<R, B>) -> CBig<R, B> {
                let ctx = Context::max(self.context(), Context(rhs.context()));
                ctx.unwrap_cfp(ctx.div_real(self, rhs))
            }
        }
        impl<R: Round, const B: Word> Div<FBig<R, B>> for &CBig<R, B> {
            type Output = CBig<R, B>;
            #[inline]
            fn div(self, rhs: FBig<R, B>) -> CBig<R, B> {
                self / &rhs
            }
        }
        impl<R: Round, const B: Word> Div<&FBig<R, B>> for CBig<R, B> {
            type Output = CBig<R, B>;
            #[inline]
            fn div(self, rhs: &FBig<R, B>) -> CBig<R, B> {
                &self / rhs
            }
        }
        impl<R: Round, const B: Word> Div<FBig<R, B>> for CBig<R, B> {
            type Output = CBig<R, B>;
            #[inline]
            fn div(self, rhs: FBig<R, B>) -> CBig<R, B> {
                &self / &rhs
            }
        }
        // FBig / CBig = (s + 0i) / z, reusing complex division
        impl<R: Round, const B: Word> Div<&CBig<R, B>> for &FBig<R, B> {
            type Output = CBig<R, B>;
            #[inline]
            fn div(self, rhs: &CBig<R, B>) -> CBig<R, B> {
                let s = CBig::from(self.clone());
                let ctx = Context::max(s.context(), rhs.context());
                ctx.unwrap_cfp(ctx.div(&s, rhs))
            }
        }
        impl<R: Round, const B: Word> Div<CBig<R, B>> for &FBig<R, B> {
            type Output = CBig<R, B>;
            #[inline]
            fn div(self, rhs: CBig<R, B>) -> CBig<R, B> {
                self / &rhs
            }
        }
        impl<R: Round, const B: Word> Div<&CBig<R, B>> for FBig<R, B> {
            type Output = CBig<R, B>;
            #[inline]
            fn div(self, rhs: &CBig<R, B>) -> CBig<R, B> {
                &self / rhs
            }
        }
        impl<R: Round, const B: Word> Div<CBig<R, B>> for FBig<R, B> {
            type Output = CBig<R, B>;
            #[inline]
            fn div(self, rhs: CBig<R, B>) -> CBig<R, B> {
                &self / &rhs
            }
        }
    };
}
impl_scalar_div!();

#[cfg(test)]
mod tests {
    use super::*;
    use dashu_float::round::mode;

    type C = CBig<mode::HalfAway, 10>;
    type F = FBig<mode::HalfAway, 10>;

    fn c(re: i32, im: i32) -> C {
        let mk = |v: i32| -> F { F::from(v).with_precision(53).value() };
        C::from_parts(mk(re), mk(im))
    }

    #[test]
    fn div_inverse() {
        // z / z = 1 for z != 0
        let z = c(3, 4);
        let q = &z / &z;
        assert_eq!(q.re().significand(), &1.into());
        assert!(q.imag().significand().is_zero());
    }

    #[test]
    fn div_basic() {
        // (6+8i)/(3+4i) = 2  (since 6+8i = 2·(3+4i))
        let z = c(6, 8);
        let w = c(3, 4);
        let q = &z / &w;
        assert_eq!(q.re().significand(), &2.into());
        assert!(q.imag().significand().is_zero());
    }

    #[test]
    fn inv_basic() {
        // 1/(3+4i) = (3-4i)/25 = 0.12 - 0.16i — use precision 53 for exact-ish
        type F = FBig<mode::HalfAway, 10>;
        let mk = |v: i32| -> F { F::from(v).with_precision(53).value() };
        let z = C::from_parts(mk(3), mk(4));
        let r = z.inv();
        // (3-4i)/25: re = 3/25, im = -4/25
        assert_eq!(r.context().precision(), 53);
        // re ≈ 0.12, im ≈ -0.16; check via multiplying back: z·inv(z) = 1
        let one = &z * &r;
        assert_eq!(one.re().significand(), &1.into());
        assert!(one.imag().significand().is_zero());
    }

    #[test]
    fn scalar_div_by_real() {
        let z = c(6, 8);
        let s = FBig::<mode::HalfAway, 10>::from(2);
        let q = &z / &s;
        assert_eq!(q.re().significand(), &3.into());
        assert_eq!(q.imag().significand(), &4.into());
    }
}
