//! Complex squaring and multiplication (near-correctly rounded via the guard-digit recipe).

use crate::cbig::CBig;
use crate::repr::{combine_parts, exact, CfpResult, Context};
use core::ops::{Mul, MulAssign};
use dashu_base::Sign::{self, *};
use dashu_float::round::Round;
use dashu_float::{FBig, Repr};
use dashu_int::Word;

/// Guard digits (base-B) for `sqr`/`mul`. The published normwise error bound for complex
/// multiplication is `< √5·u` (Brent–Percival–Zimmermann), so a small fixed guard comfortably
/// settles the accumulated rounding of the 2–4 component products for non-cancelling inputs.
const MUL_GUARD: usize = 10;

impl<R: Round> Context<R> {
    /// Square a complex number under this context: `(x+iy)² = (x²-y²) + i(2xy)`.
    pub fn sqr<const B: Word>(&self, z: &CBig<R, B>) -> CfpResult<R, B> {
        if z.is_zero() {
            // (x+iy)² = (x²−y²) + i·2xy: the real part is always `+0` (both squares are `+0`); the
            // imaginary part carries the signed product `2·x·y` — `−0` iff the two parts are
            // opposite-signed zeros (IEEE: `(−0)(+0) = −0`).
            let im_sign = if z.re().sign() == z.im().sign() {
                Sign::Positive
            } else {
                Sign::Negative
            };
            return Ok(exact(
                FBig::from_repr(Repr::zero(), self.float()),
                FBig::from_repr(Repr::zero_with_sign(im_sign), self.float()),
            ));
        }
        let gctx = self.work_context(MUL_GUARD);
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
        let gctx = self.work_context(MUL_GUARD);
        let p = self.precision();
        let (x, y) = (z.re(), z.im());
        let (u, v) = (w.re(), w.im());
        // real part: xu − yv. Fuse y·v with the subtract via FMA (one rounding
        // instead of mul-then-sub's two), which also preserves the cancellation
        // structure when xu ≈ yv.
        let xu = gctx.mul(x, u)?.value();
        let re = gctx
            .fma(y, v, xu.repr(), Negative)?
            .value()
            .with_precision(p);
        // imaginary part: xv + yu
        let xv = gctx.mul(x, v)?.value();
        let im = gctx
            .fma(y, u, xv.repr(), Positive)?
            .value()
            .with_precision(p);
        Ok(combine_parts(re, im))
    }

    /// Multiply a complex number by a real scalar (context layer): `(x+iy)·s = (xs) + i(ys)`.
    pub fn mul_real<const B: Word>(&self, z: &CBig<R, B>, s: &FBig<R, B>) -> CfpResult<R, B> {
        let gctx = self.work_context(MUL_GUARD);
        let p = self.precision();
        let re = gctx.mul(z.re(), s.repr())?.value().with_precision(p);
        let im = gctx.mul(z.im(), s.repr())?.value().with_precision(p);
        Ok(combine_parts(re, im))
    }

    /// Fused complex multiply–add under this context: `z1·z2 + sign·z3`, computed
    /// as chained real FMA per component (each product fused with the running sum,
    /// so two real roundings per component rather than the four roundings of the
    /// naive mul-then-add form). `sign` scales `z3` ([`Sign::Positive`] adds it,
    /// [`Sign::Negative`] subtracts).
    ///
    /// An infinite operand is a terminal value and is not accepted — the per-component float
    /// `fma` rejects it ([`FpError::InfiniteInput`], panicking at the convenience layer).
    pub fn fma<const B: Word>(
        &self,
        z1: &CBig<R, B>,
        z2: &CBig<R, B>,
        z3: &CBig<R, B>,
        sign: Sign,
    ) -> CfpResult<R, B> {
        let gctx = self.work_context(MUL_GUARD);
        let p = self.precision();
        let (a, b) = (z1.re(), z1.im());
        let (c, d) = (z2.re(), z2.im());
        let (e, f) = (z3.re(), z3.im());
        // Form z1·z2 with each cross product fused into the sum via FMA — the
        // subtraction `a·c − b·d` (and addition `a·d + b·c`) is the
        // cancellation-prone part, so fusing it is the accuracy win. Then add or
        // subtract z3 wholesale. (FMA's sign scales the product, so it cannot put
        // the sign on the z3 addend directly; fusing the z1·z2 cross terms instead
        // is the clean placement.)
        let ac = gctx.mul(a, c)?.value();
        let z12_re = gctx.fma(b, d, ac.repr(), Negative)?.value(); // a·c − b·d
        let re = FBig::from_repr(gctx.addsub_vr(z12_re.into_repr(), e, sign).value(), gctx)
            .with_precision(p);
        let ad = gctx.mul(a, d)?.value();
        let z12_im = gctx.fma(b, c, ad.repr(), Positive)?.value(); // a·d + b·c
        let im = FBig::from_repr(gctx.addsub_vr(z12_im.into_repr(), f, sign).value(), gctx)
            .with_precision(p);
        Ok(combine_parts(re, im))
    }
}

impl<R: Round, const B: Word> CBig<R, B> {
    /// Square the complex number (convenience layer).
    #[inline]
    pub fn sqr(&self) -> Self {
        self.context().unwrap_cfp(self.context().sqr(self))
    }

    /// Fused complex multiply–add (convenience layer, see [`Context::fma`]).
    #[inline]
    pub fn fma(&self, b: &Self, c: &Self, sign: Sign) -> Self {
        self.context()
            .unwrap_cfp(self.context().fma(self, b, c, sign))
    }
}

// CBig · CBig operators — forwarded through the standard macro (mirroring `dashu-float`'s `mul.rs`).
crate::helper_macros::impl_cbig_binop!(Mul, mul, MulAssign, mul_assign);

// --- scalar multiplication by a real FBig (mixed-type operators) ---

// CBig · FBig (componentwise, via the shared scalar macro).
crate::helper_macros::impl_cbig_scalar_binop!(Mul, mul, mul_real);

// FBig · CBig (commutative: FBig·CBig = CBig·FBig).
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
        assert!(p.im().is_pos_zero() || p.im().is_neg_zero());
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

    // At unlimited precision `mul`/`sqr` are exact (`work_context` uses `self.float()`), so this
    // must NOT panic (unlike the transcendentals). i·i = −1 exactly.
    #[test]
    fn mul_at_unlimited_is_exact() {
        let i = C::I;
        assert_eq!(&i * &i, C::NEG_ONE);
    }

    #[test]
    fn fma_basic() {
        // (1+2i)(3+4i) = -5+10i
        let z1 = c(1, 2);
        let z2 = c(3, 4);
        let z3 = c(5, 6);
        // + z3: (-5+10i) + (5+6i) = 0 + 16i
        let r = z1.fma(&z2, &z3, Positive);
        assert!(r == c(0, 16));
        // − z3: (-5+10i) − (5+6i) = -10 + 4i
        let r = z1.fma(&z2, &z3, Negative);
        assert!(r == c(-10, 4));
    }

    #[test]
    fn sqr_signed_zero() {
        // (x+iy)² = (x²−y²) + i·2xy: the real part is always `+0`, and the imaginary part is
        // `2·x·y` — `−0` iff the two zero parts are opposite-signed.
        let fctx = dashu_float::Context::<mode::HalfAway>::new(53);
        let neg_zero = F::from_repr(Repr::neg_zero(), fctx);
        let pos_zero = F::from_repr(Repr::zero(), fctx);
        let s = C::from_parts(neg_zero.clone(), pos_zero.clone()).sqr(); // -0 + i·0
        assert!(s.re().is_pos_zero());
        assert!(s.im().is_neg_zero());
        let s = C::from_parts(pos_zero.clone(), neg_zero.clone()).sqr(); // +0 - i·0
        assert!(s.re().is_pos_zero());
        assert!(s.im().is_neg_zero());
        let s = C::from_parts(neg_zero.clone(), neg_zero).sqr(); // -0 - i·0
        assert!(s.re().is_pos_zero());
        assert!(s.im().is_pos_zero());
    }
}
