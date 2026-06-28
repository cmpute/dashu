//! Decomposition and miscellaneous operations: `neg`, `conj`, `proj`, `mul_i`, `norm`, `arg`.

use crate::cbig::CBig;
use crate::repr::{exact, CfpResult, Context};
use core::ops::Neg;
use dashu_base::Sign;
use dashu_float::round::Round;
use dashu_float::{FBig, FpResult, Repr};
use dashu_int::Word;

/// Guard digits (base-B) used by `norm` — well-conditioned (sum of squares, no cancellation), so a
/// small fixed guard comfortably settles the accumulated rounding of two squarings and an add.
const NORM_GUARD: usize = 8;

/// Guard digits (base-B) for `abs`. The inner `hypot` already carries its own guard; this extra
/// margin absorbs the final re-round to the CBig precision.
const ABS_GUARD: usize = 8;

impl<R: Round, const B: Word> CBig<R, B> {
    /// The complex conjugate `x - iy`. Exact (sign flip of the imaginary part, including `-0`/`-inf`).
    #[inline]
    pub fn conj(&self) -> Self {
        self.context.unwrap_cfp(self.context.conj(self))
    }

    /// Project onto the Riemann sphere (`proj`): any part-infinite value maps to `+∞ + i·0` (the
    /// imaginary zero carrying the sign of the original imaginary part); finite values are unchanged.
    #[inline]
    pub fn proj(&self) -> Self {
        self.context.unwrap_cfp(self.context.proj(self))
    }

    /// Multiply by `±i` (exact rotation): `×i` maps `(re, im) -> (-im, re)`, `×(-i)` maps
    /// `(re, im) -> (im, -re)`.
    #[inline]
    pub fn mul_i(&self, negative: bool) -> Self {
        self.context.unwrap_cfp(self.context.mul_i(self, negative))
    }

    /// The squared modulus `re² + im²` (a real [`FBig`]). Cheap and near-exact — it avoids the
    /// `sqrt` of [`CBig::abs`]. Matches num-complex's `norm_sqr`.
    #[inline]
    pub fn norm(&self) -> FBig<R, B> {
        self.context.float().unwrap_fp(self.context.norm(self))
    }

    /// The modulus `|z| = sqrt(re² + im²)` (a real [`FBig`]). A thin composition over
    /// [`dashu_float::Context::hypot`] (the overflow-safe scaled sum-of-squares), evaluated at guard
    /// precision and re-rounded. Near-correctly rounded.
    ///
    /// # Panics
    ///
    /// Panics if the precision is unlimited.
    #[inline]
    pub fn abs(&self) -> FBig<R, B> {
        self.context.float().unwrap_fp(self.context.abs(self))
    }

    /// The argument (phase) `atan2(im, re) ∈ ]-π, π]`. The branch cut lies on `]−∞, 0]`; signed zero
    /// and infinities are handled per the C99 Annex G `atan2` table (reused from `dashu-float`).
    #[inline]
    pub fn arg(&self) -> FBig<R, B> {
        self.context.float().unwrap_fp(self.context.arg(self, None))
    }
}

impl<R: Round, const B: Word> Neg for CBig<R, B> {
    type Output = CBig<R, B>;
    #[inline]
    fn neg(self) -> Self::Output {
        self.context().unwrap_cfp(self.context().neg(&self))
    }
}

impl<R: Round, const B: Word> Neg for &CBig<R, B> {
    type Output = CBig<R, B>;
    #[inline]
    fn neg(self) -> Self::Output {
        self.context().unwrap_cfp(self.context().neg(self))
    }
}

impl<R: Round> Context<R> {
    /// Negate under this context (context layer). Exact.
    pub fn neg<const B: Word>(&self, z: &CBig<R, B>) -> CfpResult<R, B> {
        Ok(exact(
            FBig::from_repr(-z.re.clone(), self.float()),
            FBig::from_repr(-z.im.clone(), self.float()),
        ))
    }

    /// Complex conjugate under this context (context layer). Exact.
    pub fn conj<const B: Word>(&self, z: &CBig<R, B>) -> CfpResult<R, B> {
        Ok(exact(
            FBig::from_repr(z.re.clone(), self.float()),
            FBig::from_repr(-z.im.clone(), self.float()),
        ))
    }

    /// Riemann projection under this context (context layer). Exact.
    pub fn proj<const B: Word>(&self, z: &CBig<R, B>) -> CfpResult<R, B> {
        if z.is_infinite() {
            // +∞ on the real part; the imaginary zero carries the sign of the original imag part.
            let im = if z.imag().sign() == Sign::Negative {
                Repr::neg_zero()
            } else {
                Repr::zero()
            };
            Ok(exact(
                FBig::from_repr(Repr::infinity(), self.float()),
                FBig::from_repr(im, self.float()),
            ))
        } else {
            Ok(exact(
                FBig::from_repr(z.re.clone(), self.float()),
                FBig::from_repr(z.im.clone(), self.float()),
            ))
        }
    }

    /// Multiply by `±i` under this context (context layer). Exact rotation.
    pub fn mul_i<const B: Word>(&self, z: &CBig<R, B>, negative: bool) -> CfpResult<R, B> {
        let (re, im) = if negative {
            // ×(-i): (x, y) -> (y, -x)
            (z.im.clone(), -z.re.clone())
        } else {
            // ×i: (x, y) -> (-y, x)
            (-z.im.clone(), z.re.clone())
        };
        Ok(exact(FBig::from_repr(re, self.float()), FBig::from_repr(im, self.float())))
    }

    /// The squared modulus `re² + im²` (context layer). Near-exact; returns `+∞` for an infinite
    /// input and propagates overflow to a signed infinity via the float `unwrap_fp` policy.
    pub fn norm<const B: Word>(&self, z: &CBig<R, B>) -> FpResult<FBig<R, B>> {
        if z.is_infinite() {
            return Ok(dashu_base::Approximation::Exact(FBig::from_repr(
                Repr::infinity(),
                self.float(),
            )));
        }
        let gctx = self.guard(NORM_GUARD);
        let re2 = gctx.unwrap_fp(gctx.sqr(z.re()));
        let im2 = gctx.unwrap_fp(gctx.sqr(z.imag()));
        let n = gctx.unwrap_fp(gctx.add(re2.repr(), im2.repr()));
        Ok(n.with_precision(self.precision()))
    }

    /// The argument `atan2(im, re)` (context layer). Delegates to `dashu-float`'s Annex-G `atan2`;
    /// the cache threads into it (the convenience layer passes `None`).
    pub fn arg<const B: Word>(
        &self,
        z: &CBig<R, B>,
        cache: Option<&mut dashu_float::ConstCache>,
    ) -> FpResult<FBig<R, B>> {
        self.float().atan2(z.imag(), z.re(), cache)
    }

    /// The modulus `|z| = hypot(re, im)` (context layer). Near-correctly rounded; returns `+∞` for
    /// an infinite input. Thin composition over [`dashu_float::Context::hypot`].
    ///
    /// # Panics
    ///
    /// Panics if the precision is unlimited.
    pub fn abs<const B: Word>(&self, z: &CBig<R, B>) -> FpResult<FBig<R, B>> {
        let gctx = self.guard(ABS_GUARD);
        let h = gctx.hypot(z.re(), z.imag())?;
        Ok(h.value().with_precision(self.precision()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dashu_float::round::mode;

    type C = CBig<mode::HalfAway, 10>;

    #[test]
    fn neg_and_conj() {
        let z = C::from_parts(3.into(), 4.into());
        let n = -&z; // Neg by reference (no inherent neg method)
        assert_eq!(n.re().significand(), &(-3i32).into());
        assert_eq!(n.imag().significand(), &(-4i32).into());
        let c = z.conj();
        assert_eq!(c.re().significand(), &3.into());
        assert_eq!(c.imag().significand(), &(-4i32).into());
    }

    #[test]
    fn mul_i_rotation() {
        let z = C::from_parts(3.into(), 4.into());
        // ×i: (3,4) -> (-4, 3)
        let zi = z.mul_i(false);
        assert_eq!(zi.re().significand(), &(-4i32).into());
        assert_eq!(zi.imag().significand(), &3.into());
        // ×(-i): (3,4) -> (4, -3)
        let zni = z.mul_i(true);
        assert_eq!(zni.re().significand(), &4.into());
        assert_eq!(zni.imag().significand(), &(-3i32).into());
        // mul_i^4 == identity
        let id = z.mul_i(false).mul_i(false).mul_i(false).mul_i(false);
        assert!(id == z);
    }

    #[test]
    fn proj_finite_unchanged() {
        let z = C::from_parts(3.into(), 4.into());
        assert!(z.proj() == z);
    }

    #[test]
    fn proj_infinite_is_riemann_point() {
        let inf = C::new(Repr::infinity(), Repr::<10>::new(5.into(), 0), Context::new(53));
        let p = inf.proj();
        assert!(p.re().is_infinite());
        assert_eq!(p.re().sign(), Sign::Positive);
        assert!(p.imag().is_zero());
    }

    #[test]
    fn norm_of_3_4_is_25() {
        // build at precision 53 so the 2-digit result 25 is exact
        type F = FBig<mode::HalfAway, 10>;
        let mk = |v: i32| -> F { F::from(v).with_precision(53).value() };
        let z = C::from_parts(mk(3), mk(4));
        let n = z.norm();
        assert_eq!(n.repr().significand(), &25.into());
    }

    #[test]
    fn arg_of_1_1_is_pi_quarter() {
        type F = FBig<mode::HalfAway, 10>;
        let mk = |v: i32| -> F { F::from(v).with_precision(53).value() };
        let z = C::from_parts(mk(1), mk(1));
        let a = z.arg();
        // atan(1) = π/4 ≈ 0.7854, strictly between 0 and 1
        assert!(a > F::ZERO);
        assert!(a < F::ONE);
    }
}
