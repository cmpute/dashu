//! Complex division and reciprocal (near-correctly rounded via Smith's method + guard re-round).

use crate::cbig::CBig;
use crate::repr::{combine_parts, exact, riemann, CfpResult, Context};
use core::ops::{Div, DivAssign};
use dashu_base::{AbsOrd, Inverse, Sign::*};
use dashu_float::round::Round;
use dashu_float::{FBig, FpError};
use dashu_int::Word;

/// Guard digits (base-B) for `div`/`inv`. The naive complex-division error is `~(3+√5)·u`; a fixed
/// guard comfortably absorbs it for well-conditioned denominators.
const DIV_GUARD: usize = 14;

impl<R: Round> Context<R> {
    /// Reciprocal `1/z = conj(z)/|z|²` under this context (context layer).
    pub fn inv<const B: Word>(&self, z: &CBig<R, B>) -> CfpResult<R, B> {
        if z.is_zero() {
            return Ok(riemann(*self)); // 1/0 = ∞
        }
        // An infinite input is a terminal value: it falls through and the float layer rejects it.
        let gctx = self.work_context(DIV_GUARD);
        let p = self.precision();
        let (x, y) = (z.re(), z.im());
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
        let gctx = self.work_context(DIV_GUARD);
        let p = self.precision();
        let (x, y) = (z.re(), z.im());
        let (u, v) = (w.re(), w.im());
        // Determine |u| >= |v| via abs_cmp on temporary FBig views (Smith's method).
        let u_ge_v = {
            let fu = FBig::from_repr(u.clone(), gctx);
            let fv = FBig::from_repr(v.clone(), gctx);
            fu.abs_cmp(&fv).is_ge()
        };

        // r, d depend on which of |u|, |v| is larger
        let (r, d) = if u_ge_v {
            // r = v/u, d = u + r·v  (fuse r·v with the add via FMA)
            let r = gctx.div(v, u)?.value();
            let d = gctx.fma(r.repr(), v, u, Positive)?.value();
            (r, d)
        } else {
            // r = u/v, d = v + r·u
            let r = gctx.div(u, v)?.value();
            let d = gctx.fma(r.repr(), u, v, Positive)?.value();
            (r, d)
        };

        let (re, im) = if u_ge_v {
            // re = (x + r·y)/d, im = (y − r·x)/d  (both fuse via FMA)
            let num_re = gctx.fma(r.repr(), y, x, Positive)?.value();
            let num_im = gctx.fma(r.repr(), x, y, Negative)?.value();
            (
                gctx.div(num_re.repr(), d.repr())?.value().with_precision(p),
                gctx.div(num_im.repr(), d.repr())?.value().with_precision(p),
            )
        } else {
            // re = (r·x + y)/d, im = (r·y − x)/d. The numerator products fuse via
            // FMA; `r·y − x` is `−(x − r·y)`, so one FMA plus an exact negation.
            let num_re = gctx.fma(r.repr(), x, y, Positive)?.value();
            let num_im = -gctx.fma(r.repr(), y, x, Negative)?.value();
            (
                gctx.div(num_re.repr(), d.repr())?.value().with_precision(p),
                gctx.div(num_im.repr(), d.repr())?.value().with_precision(p),
            )
        };
        Ok(combine_parts(re, im))
    }

    /// Divide a complex number by a real scalar (context layer): `(x+iy)/s = (x/s) + i(y/s)`.
    pub fn div_real<const B: Word>(&self, z: &CBig<R, B>, s: &FBig<R, B>) -> CfpResult<R, B> {
        if !z.is_infinite() && (s.repr().is_pos_zero() || s.repr().is_neg_zero()) {
            // Division by zero for a finite numerator produces ∞ as a terminal output.
            if z.is_zero() {
                return Err(FpError::Indeterminate); // 0/0 (incl. ±0)
            }
            return Ok(riemann(*self)); // z/±0 (z≠0) = ∞
        }
        // An infinite operand (numerator or divisor) is a terminal value: it falls through and the
        // float layer rejects it (panicking at the convenience layer).
        let gctx = self.work_context(DIV_GUARD);
        let p = self.precision();
        let re = gctx.div(z.re(), s.repr())?.value().with_precision(p);
        let im = gctx.div(z.im(), s.repr())?.value().with_precision(p);
        Ok(combine_parts(re, im))
    }
}

/// Short-circuit for `z / w` with a zero operand (finite numerators only).
///
/// An infinite numerator or divisor is a terminal value and is **not** accepted here — it returns
/// `None` so the normal path runs and the float layer rejects it (panicking at the convenience
/// layer). Only the exact zero cases, whose results are fully determined by finite operands, are
/// short-circuited: `0/0 → Indeterminate`, `finite/0 → ∞`, `0/finite → 0`.
fn div_special<R: Round, const B: Word>(z: &CBig<R, B>, w: &CBig<R, B>) -> Option<CfpResult<R, B>> {
    if z.is_infinite() || w.is_infinite() {
        return None;
    }
    let (zz, wz) = (z.is_zero(), w.is_zero());
    let ctx = Context::max(z.context(), w.context());
    if zz && wz {
        Some(Err(FpError::Indeterminate)) // 0/0
    } else if wz {
        Some(Ok(riemann(ctx))) // z/0 (z ≠ 0) = ∞
    } else if zz {
        Some(Ok(exact(FBig::ZERO, FBig::ZERO))) // 0/w (w ≠ 0) = 0
    } else {
        None
    }
}

impl<R: Round, const B: Word> Inverse for CBig<R, B> {
    type Output = CBig<R, B>;

    #[inline]
    fn inv(self) -> Self::Output {
        self.context().unwrap_cfp(self.context().inv(&self))
    }
}

impl<R: Round, const B: Word> Inverse for &CBig<R, B> {
    type Output = CBig<R, B>;

    #[inline]
    fn inv(self) -> Self::Output {
        self.context().unwrap_cfp(self.context().inv(self))
    }
}

// CBig / CBig operators
crate::helper_macros::impl_cbig_binop!(Div, div, DivAssign, div_assign);

// --- scalar division by a real FBig (mixed-type operators) ---

// CBig / FBig (componentwise, via the shared scalar macro).
crate::helper_macros::impl_cbig_scalar_binop!(Div, div, div_real);

// FBig / CBig = (s + 0i) / z, reusing complex division.
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
        let this = self.clone();
        let s = CBig::from(this);
        let ctx = Context::max(s.context(), rhs.context());
        ctx.unwrap_cfp(ctx.div(&s, &rhs))
    }
}
impl<R: Round, const B: Word> Div<&CBig<R, B>> for FBig<R, B> {
    type Output = CBig<R, B>;
    #[inline]
    fn div(self, rhs: &CBig<R, B>) -> CBig<R, B> {
        let s = CBig::from(self);
        let ctx = Context::max(s.context(), rhs.context());
        ctx.unwrap_cfp(ctx.div(&s, rhs))
    }
}
impl<R: Round, const B: Word> Div<CBig<R, B>> for FBig<R, B> {
    type Output = CBig<R, B>;
    #[inline]
    fn div(self, rhs: CBig<R, B>) -> CBig<R, B> {
        let s = CBig::from(self);
        let ctx = Context::max(s.context(), rhs.context());
        ctx.unwrap_cfp(ctx.div(&s, &rhs))
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
    fn div_inverse() {
        // z / z = 1 for z != 0
        let z = c(3, 4);
        let q = &z / &z;
        assert_eq!(q.re().significand(), &1.into());
        assert!(q.im().significand().is_zero());
    }

    #[test]
    fn div_basic() {
        // (6+8i)/(3+4i) = 2  (since 6+8i = 2·(3+4i))
        let z = c(6, 8);
        let w = c(3, 4);
        let q = &z / &w;
        assert_eq!(q.re().significand(), &2.into());
        assert!(q.im().significand().is_zero());
    }

    #[test]
    fn inv_basic() {
        // 1/(3+4i) = (3-4i)/25 = 0.12 - 0.16i — use precision 53 for exact-ish
        type F = FBig<mode::HalfAway, 10>;
        let mk = |v: i32| -> F { F::from(v).with_precision(53).value() };
        let z = C::from_parts(mk(3), mk(4));
        let r = (&z).inv();
        // (3-4i)/25: re = 3/25, im = -4/25
        assert_eq!(r.context().precision(), 53);
        // re ≈ 0.12, im ≈ -0.16; check via multiplying back: z·inv(z) = 1
        let one = &z * &r;
        assert_eq!(one.re().significand(), &1.into());
        assert!(one.im().significand().is_zero());
    }

    #[test]
    fn scalar_div_by_real() {
        let z = c(6, 8);
        let s = FBig::<mode::HalfAway, 10>::from(2);
        let q = &z / &s;
        assert_eq!(q.re().significand(), &3.into());
        assert_eq!(q.im().significand(), &4.into());
    }

    #[test]
    fn div_real_by_negative_zero_matches_positive_zero() {
        use dashu_float::Repr;
        // Dividing a nonzero complex by the scalar -0 must give the same Riemann infinity as +0,
        // not fall through to the general path (which would divide by -0 componentwise).
        let z = c(3, 4);
        let neg_zero = F::from_repr(Repr::neg_zero(), z.context().float());
        let q_neg = &z / &neg_zero;
        let q_pos = &z / &F::ZERO;
        assert!(q_neg.is_infinite());
        assert!(q_neg == q_pos); // both are +∞ + i·0
    }

    // `div` at unlimited precision panics via the float layer (`work_context` → `self.float()` →
    // float `div`, which rejects unlimited: a quotient isn't exactly representable in general).
    #[test]
    #[should_panic(expected = "precision cannot be 0")]
    fn complex_div_unlimited_panics() {
        let one = C::ONE;
        let i = C::I;
        let _ = &one / &i; // 1/i at unlimited
    }
}
