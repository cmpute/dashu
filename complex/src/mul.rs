//! Complex squaring and multiplication (near-correctly rounded via the guard-digit recipe).

use crate::cbig::CBig;
use crate::repr::{combine_parts, exact, riemann, CfpResult, Context};
use core::ops::{Mul, MulAssign};
use dashu_float::round::Round;
use dashu_float::{FBig, FpError};
use dashu_int::Word;

/// Guard digits (base-B) for `sqr`/`mul`. The published normwise error bound for complex
/// multiplication is `< √5·u` (Brent–Percival–Zimmermann), so a small fixed guard comfortably
/// settles the accumulated rounding of the 2–4 component products for non-cancelling inputs.
const MUL_GUARD: usize = 10;

impl<R: Round> Context<R> {
    /// Square a complex number under this context: `(x+iy)² = (x²-y²) + i(2xy)`.
    pub fn sqr<const B: Word>(&self, z: &CBig<R, B>) -> CfpResult<R, B> {
        if z.is_infinite() {
            return Ok(riemann(*self)); // ∞·∞ = Riemann infinity
        }
        if z.is_zero() {
            return Ok(exact(FBig::ZERO, FBig::ZERO));
        }
        let gctx = self.guard(MUL_GUARD);
        let p = self.precision();
        let (x, y) = (z.re(), z.im());
        // real part: x² - y²
        let x2 = gctx.sqr(x)?.value();
        let y2 = gctx.sqr(y)?.value();
        let re = gctx.sub(x2.repr(), y2.repr())?.value().with_precision(p);
        // imaginary part: 2·x·y
        let xy = gctx.mul(x, y)?.value();
        let im = gctx.add(xy.repr(), xy.repr())?.value().with_precision(p);
        Ok(combine_parts(re, im))
    }

    /// Multiply two complex numbers under this context: `(x+iy)(u+iv) = (xu-yv) + i(xv+yu)`
    /// (naive 4-mul form; near-correctly rounded via the guard re-round).
    pub fn mul<const B: Word>(&self, z: &CBig<R, B>, w: &CBig<R, B>) -> CfpResult<R, B> {
        if z.is_infinite() || w.is_infinite() {
            if z.is_zero() || w.is_zero() {
                return Err(FpError::Indeterminate); // 0·∞
            }
            return Ok(riemann(Context::max(z.context(), w.context()))); // ∞·finite = Riemann infinity
        }
        let gctx = self.guard(MUL_GUARD);
        let p = self.precision();
        let (x, y) = (z.re(), z.im());
        let (u, v) = (w.re(), w.im());
        // real part: xu - yv
        let xu = gctx.mul(x, u)?.value();
        let yv = gctx.mul(y, v)?.value();
        let re = gctx.sub(xu.repr(), yv.repr())?.value().with_precision(p);
        // imaginary part: xv + yu
        let xv = gctx.mul(x, v)?.value();
        let yu = gctx.mul(y, u)?.value();
        let im = gctx.add(xv.repr(), yu.repr())?.value().with_precision(p);
        Ok(combine_parts(re, im))
    }

    /// Multiply a complex number by a real scalar (context layer): `(x+iy)·s = (xs) + i(ys)`.
    pub fn mul_real<const B: Word>(&self, z: &CBig<R, B>, s: &FBig<R, B>) -> CfpResult<R, B> {
        if z.is_infinite() || s.repr().is_infinite() {
            if z.is_zero() || s.repr().is_zero() || s.repr().is_neg_zero() {
                return Err(FpError::Indeterminate); // 0·∞
            }
            return Ok(riemann(*self));
        }
        let gctx = self.guard(MUL_GUARD);
        let p = self.precision();
        let re = gctx.mul(z.re(), s.repr())?.value().with_precision(p);
        let im = gctx.mul(z.im(), s.repr())?.value().with_precision(p);
        Ok(combine_parts(re, im))
    }
}

impl<R: Round, const B: Word> CBig<R, B> {
    /// Square the complex number (convenience layer).
    #[inline]
    pub fn sqr(&self) -> Self {
        self.context().unwrap_cfp(self.context().sqr(self))
    }
}

// CBig · CBig operators — forwarded through the standard macro (mirroring `dashu-float`'s `mul.rs`).
crate::helper_macros::impl_cbig_binop!(Mul, mul, MulAssign, mul_assign);

// --- scalar multiplication by a real FBig (mixed-type operators) ---

// CBig · FBig (componentwise) and FBig · CBig (commutative: FBig·CBig = CBig·FBig).
impl<R: Round, const B: Word> Mul<&FBig<R, B>> for &CBig<R, B> {
    type Output = CBig<R, B>;
    #[inline]
    fn mul(self, rhs: &FBig<R, B>) -> CBig<R, B> {
        let ctx = Context::max(self.context(), Context(rhs.context()));
        ctx.unwrap_cfp(ctx.mul_real(self, rhs))
    }
}
impl<R: Round, const B: Word> Mul<FBig<R, B>> for &CBig<R, B> {
    type Output = CBig<R, B>;
    #[inline]
    fn mul(self, rhs: FBig<R, B>) -> CBig<R, B> {
        let ctx = Context::max(self.context(), Context(rhs.context()));
        ctx.unwrap_cfp(ctx.mul_real(self, &rhs))
    }
}
impl<R: Round, const B: Word> Mul<&FBig<R, B>> for CBig<R, B> {
    type Output = CBig<R, B>;
    #[inline]
    fn mul(self, rhs: &FBig<R, B>) -> CBig<R, B> {
        let ctx = Context::max(self.context(), Context(rhs.context()));
        ctx.unwrap_cfp(ctx.mul_real(&self, rhs))
    }
}
impl<R: Round, const B: Word> Mul<FBig<R, B>> for CBig<R, B> {
    type Output = CBig<R, B>;
    #[inline]
    fn mul(self, rhs: FBig<R, B>) -> CBig<R, B> {
        let ctx = Context::max(self.context(), Context(rhs.context()));
        ctx.unwrap_cfp(ctx.mul_real(&self, &rhs))
    }
}
impl<R: Round, const B: Word> Mul<&CBig<R, B>> for &FBig<R, B> {
    type Output = CBig<R, B>;
    #[inline]
    fn mul(self, rhs: &CBig<R, B>) -> CBig<R, B> {
        rhs * self
    }
}
impl<R: Round, const B: Word> Mul<CBig<R, B>> for &FBig<R, B> {
    type Output = CBig<R, B>;
    #[inline]
    fn mul(self, rhs: CBig<R, B>) -> CBig<R, B> {
        &rhs * self
    }
}
impl<R: Round, const B: Word> Mul<&CBig<R, B>> for FBig<R, B> {
    type Output = CBig<R, B>;
    #[inline]
    fn mul(self, rhs: &CBig<R, B>) -> CBig<R, B> {
        rhs * &self
    }
}
impl<R: Round, const B: Word> Mul<CBig<R, B>> for FBig<R, B> {
    type Output = CBig<R, B>;
    #[inline]
    fn mul(self, rhs: CBig<R, B>) -> CBig<R, B> {
        &rhs * &self
    }
}

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
    fn sqr_basic() {
        // (3+4i)² = -7+24i
        let z = c(3, 4);
        let s = z.sqr();
        assert_eq!(s.re().significand(), &(-7i32).into());
        assert_eq!(s.im().significand(), &24.into());
    }

    #[test]
    fn mul_basic() {
        // (1+2i)(3+4i) = -5+10i  (compare full values: 10 normalizes to 1·10¹ in base 10)
        let z = c(1, 2);
        let w = c(3, 4);
        let p = &z * &w;
        assert!(p == c(-5, 10));
    }

    #[test]
    fn mul_assign_val_and_ref() {
        let z = c(1, 2);
        let w = c(3, 4);
        // (1+2i)(3+4i) = -5+10i
        let mut acc = z.clone();
        acc *= w.clone();
        assert!(acc == c(-5, 10));
        let mut acc = z.clone();
        acc *= &w;
        assert!(acc == c(-5, 10));
    }

    #[test]
    fn mul_by_one_is_identity() {
        let z = c(3, 4);
        let p = &z * &CBig::ONE;
        assert!(p == z);
    }

    #[test]
    fn mul_by_conj_is_norm() {
        // z·conj(z) = norm(z), purely real
        let z = c(3, 4);
        let p = &z * &z.conj();
        assert!(p.im().is_zero() || p.im().is_neg_zero());
        assert_eq!(p.re().significand(), &25.into());
    }

    #[test]
    fn scalar_mul_by_real() {
        let z = c(3, 4);
        let s = FBig::<mode::HalfAway, 10>::from(2);
        let p = &z * &s;
        assert_eq!(p.re().significand(), &6.into());
        assert_eq!(p.im().significand(), &8.into());
        // commutes: s * z
        let p2 = &s * &z;
        assert_eq!(p2.re().significand(), &6.into());
    }
}
