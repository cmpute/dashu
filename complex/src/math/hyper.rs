//! Complex hyperbolic functions, built from the complex circular functions via the rotation
//! identities
//!
//! ```text
//!   sinh z = -i·sin(i·z)      asinh z = -i·asin(i·z)
//!   cosh z =  cos(i·z)        acosh z =  ±i·acos z       (sign of ± follows Im z)
//!   tanh z = -i·tan(i·z)      atanh z = -i·atan(i·z)
//! ```
//!
//! Each identity rotates the argument by `i` (an exact part swap `(x, y) → (−y, x)`), evaluates the
//! already correctly-rounded complex circular function, and rotates the result back
//! (`(u, v) → (v, −u)`). The rotations are sign-exact — they move existing digits, never round — so
//! the hyperbolic result carries the inner function's rounding certification unchanged; no separate
//! Ziv loop or guard precision is needed.
//!
//! The exactly-zero cases are handled explicitly with the Annex-G signed-zero values: the rotation
//! through `cos` would mis-sign `cosh`'s imaginary zero (the circular `cos`'s signed-product rule
//! applies to the *rotated* argument's swapped signs), and `tan`/`tanh` at zero has no exact shortcut
//! of its own (it would trip the Ziv limited-precision check for an unlimited-precision zero).

use dashu_base::Approximation;
use dashu_base::Sign;
use dashu_float::round::{ErrorBounds, Rounding};
use dashu_float::{ConstCache, FBig, FpError, Repr};
use dashu_int::Word;

use crate::cbig::CBig;
use crate::repr::{exact, reborrow_cache, CRounded, CfpResult, Context};

impl<R: ErrorBounds> Context<R> {
    /// Hyperbolic sine (context layer), correctly rounded via `sinh z = -i·sin(i·z)`.
    ///
    /// Special values: `sinh(±0 ± i·0) = ±0 + i·±0` (the signed zeros carry the input parts'
    /// signs); an infinite input maps to [`FpError::Indeterminate`].
    pub fn sinh<const B: Word>(
        &self,
        z: &CBig<R, B>,
        mut cache: Option<&mut ConstCache>,
    ) -> CfpResult<R, B> {
        if z.is_infinite() {
            return Err(FpError::Indeterminate);
        }
        if z.is_zero() {
            return Ok(exact(
                FBig::from_repr(Repr::zero_with_sign(z.re().sign()), self.float()),
                FBig::from_repr(Repr::zero_with_sign(z.im().sign()), self.float()),
            ));
        }
        self.assert_limited();
        let iz = CBig::new(-z.im().clone(), z.re().clone(), *self);
        Ok(rotate_neg_i(self.sin::<B>(&iz, reborrow_cache(&mut cache))?))
    }

    /// Hyperbolic cosine (context layer), correctly rounded via `cosh z = cos(i·z)`.
    ///
    /// Special values: `cosh(±0 ± i·0) = 1 ± i·0`, where the imaginary zero is `−0` iff the two
    /// parts are opposite-signed zeros (the signed product); an infinite input maps to
    /// [`FpError::Indeterminate`].
    pub fn cosh<const B: Word>(
        &self,
        z: &CBig<R, B>,
        mut cache: Option<&mut ConstCache>,
    ) -> CfpResult<R, B> {
        if z.is_infinite() {
            return Err(FpError::Indeterminate);
        }
        if z.is_zero() {
            let im = if z.re().sign() != z.im().sign() {
                Repr::zero_with_sign(Sign::Negative)
            } else {
                Repr::zero_with_sign(Sign::Positive)
            };
            return Ok(exact(
                FBig::from_repr(Repr::one(), self.float()),
                FBig::from_repr(im, self.float()),
            ));
        }
        self.assert_limited();
        let iz = CBig::new(-z.im().clone(), z.re().clone(), *self);
        self.cos::<B>(&iz, reborrow_cache(&mut cache))
    }

    /// Simultaneously compute `sinh z` and `cosh z` (context layer), sharing one evaluation of the
    /// inner `sin_cos(iz)`. Returns `(sinh_result, cosh_result)`, each an [`CfpResult`].
    pub fn sinh_cosh<const B: Word>(
        &self,
        z: &CBig<R, B>,
        mut cache: Option<&mut ConstCache>,
    ) -> (CfpResult<R, B>, CfpResult<R, B>) {
        if z.is_infinite() {
            return (Err(FpError::Indeterminate), Err(FpError::Indeterminate));
        }
        if z.is_zero() {
            let sinh = exact(
                FBig::from_repr(Repr::zero_with_sign(z.re().sign()), self.float()),
                FBig::from_repr(Repr::zero_with_sign(z.im().sign()), self.float()),
            );
            let cosh_im = if z.re().sign() != z.im().sign() {
                Repr::zero_with_sign(Sign::Negative)
            } else {
                Repr::zero_with_sign(Sign::Positive)
            };
            let cosh = exact(
                FBig::from_repr(Repr::one(), self.float()),
                FBig::from_repr(cosh_im, self.float()),
            );
            return (Ok(sinh), Ok(cosh));
        }
        self.assert_limited();
        let iz = CBig::new(-z.im().clone(), z.re().clone(), *self);
        let (sin_iz, cos_iz) = self.sin_cos::<B>(&iz, reborrow_cache(&mut cache));
        match (sin_iz, cos_iz) {
            (Ok(s), Ok(c)) => (Ok(rotate_neg_i(s)), Ok(c)),
            // an error in the shared evaluation fails both parts together.
            (Err(e), _) | (_, Err(e)) => (Err(e), Err(e)),
        }
    }

    /// Hyperbolic tangent (context layer), correctly rounded via `tanh z = -i·tan(i·z)`.
    ///
    /// Special values: `tanh(±0 ± i·0) = ±0 + i·±0`; an infinite input maps to
    /// [`FpError::Indeterminate`].
    pub fn tanh<const B: Word>(
        &self,
        z: &CBig<R, B>,
        mut cache: Option<&mut ConstCache>,
    ) -> CfpResult<R, B> {
        if z.is_infinite() {
            return Err(FpError::Indeterminate);
        }
        if z.is_zero() {
            return Ok(exact(
                FBig::from_repr(Repr::zero_with_sign(z.re().sign()), self.float()),
                FBig::from_repr(Repr::zero_with_sign(z.im().sign()), self.float()),
            ));
        }
        self.assert_limited();
        let iz = CBig::new(-z.im().clone(), z.re().clone(), *self);
        Ok(rotate_neg_i(self.tan::<B>(&iz, reborrow_cache(&mut cache))?))
    }

    /// Inverse hyperbolic sine (context layer), correctly rounded via `asinh z = -i·asin(i·z)`.
    ///
    /// An infinite input maps to [`FpError::Indeterminate`] (mirroring `asin`).
    pub fn asinh<const B: Word>(
        &self,
        z: &CBig<R, B>,
        mut cache: Option<&mut ConstCache>,
    ) -> CfpResult<R, B> {
        if z.is_infinite() {
            return Err(FpError::Indeterminate);
        }
        self.assert_limited();
        let iz = CBig::new(-z.im().clone(), z.re().clone(), *self);
        Ok(rotate_neg_i(self.asin::<B>(&iz, reborrow_cache(&mut cache))?))
    }

    /// Inverse hyperbolic cosine (context layer), correctly rounded via `acosh z = ±i·acos z`,
    /// where the sign follows `Im z` so the branch cut on `]−∞, 1]` lands on the correct side
    /// (a negative — including negative-zero — imaginary part selects `−i`, matching `acos`'s
    /// own signed-zero cut).
    ///
    /// An infinite input maps to [`FpError::Indeterminate`] (mirroring `acos`).
    pub fn acosh<const B: Word>(
        &self,
        z: &CBig<R, B>,
        mut cache: Option<&mut ConstCache>,
    ) -> CfpResult<R, B> {
        if z.is_infinite() {
            return Err(FpError::Indeterminate);
        }
        self.assert_limited();
        let acos_z = self.acos::<B>(z, reborrow_cache(&mut cache))?;
        if z.im().sign() == Sign::Negative {
            Ok(rotate_neg_i(acos_z))
        } else {
            Ok(rotate_pos_i(acos_z))
        }
    }

    /// Inverse hyperbolic tangent (context layer), correctly rounded via `atanh z = -i·atan(i·z)`.
    ///
    /// An infinite input maps to [`FpError::Indeterminate`] (mirroring `atan`).
    pub fn atanh<const B: Word>(
        &self,
        z: &CBig<R, B>,
        mut cache: Option<&mut ConstCache>,
    ) -> CfpResult<R, B> {
        if z.is_infinite() {
            return Err(FpError::Indeterminate);
        }
        self.assert_limited();
        let iz = CBig::new(-z.im().clone(), z.re().clone(), *self);
        Ok(rotate_neg_i(self.atan::<B>(&iz, reborrow_cache(&mut cache))?))
    }
}

impl<R: ErrorBounds, const B: Word> CBig<R, B> {
    /// Hyperbolic sine (convenience layer).
    ///
    /// # Panics
    ///
    /// Panics if the precision is unlimited or on an indeterminate special value.
    #[inline]
    pub fn sinh(&self) -> Self {
        self.context().unwrap_cfp(self.context().sinh(self, None))
    }

    /// Hyperbolic cosine (convenience layer).
    ///
    /// # Panics
    ///
    /// Panics if the precision is unlimited or on an indeterminate special value.
    #[inline]
    pub fn cosh(&self) -> Self {
        self.context().unwrap_cfp(self.context().cosh(self, None))
    }

    /// Simultaneously compute `(sinh z, cosh z)` (convenience layer).
    #[inline]
    pub fn sinh_cosh(&self) -> (Self, Self) {
        let (s, c) = self.context().sinh_cosh(self, None);
        (self.context().unwrap_cfp(s), self.context().unwrap_cfp(c))
    }

    /// Hyperbolic tangent (convenience layer).
    ///
    /// # Panics
    ///
    /// Panics if the precision is unlimited or on an indeterminate special value.
    #[inline]
    pub fn tanh(&self) -> Self {
        self.context().unwrap_cfp(self.context().tanh(self, None))
    }

    /// Inverse hyperbolic sine (convenience layer).
    ///
    /// # Panics
    ///
    /// Panics if the precision is unlimited or on an indeterminate special value.
    #[inline]
    pub fn asinh(&self) -> Self {
        self.context().unwrap_cfp(self.context().asinh(self, None))
    }

    /// Inverse hyperbolic cosine (convenience layer).
    ///
    /// # Panics
    ///
    /// Panics if the precision is unlimited or on an indeterminate special value.
    #[inline]
    pub fn acosh(&self) -> Self {
        self.context().unwrap_cfp(self.context().acosh(self, None))
    }

    /// Inverse hyperbolic tangent (convenience layer).
    ///
    /// # Panics
    ///
    /// Panics if the precision is unlimited or on an indeterminate special value.
    #[inline]
    pub fn atanh(&self) -> Self {
        self.context().unwrap_cfp(self.context().atanh(self, None))
    }
}

/// `-i·(u + iv) = v - i·u`: swap the parts and negate the new imaginary part. Sign-exact, so an
/// `Exact` result stays exact. For an inexact result the parts' rounding flags swap along with the
/// parts, and the negated part's direction flips (`AddOne ↔ SubOne`).
fn rotate_neg_i<R: ErrorBounds, const B: Word>(r: CRounded<R, B>) -> CRounded<R, B> {
    match r {
        Approximation::Exact(v) => {
            let (re, im) = v.into_parts();
            Approximation::Exact(CBig::from_parts(im, -re))
        }
        Approximation::Inexact(v, (r_re, r_im)) => {
            let (re, im) = v.into_parts();
            Approximation::Inexact(CBig::from_parts(im, -re), (r_im, neg_rounding(r_re)))
        }
    }
}

/// `+i·(u + iv) = -v + i·u`: swap the parts and negate the new real part.
fn rotate_pos_i<R: ErrorBounds, const B: Word>(r: CRounded<R, B>) -> CRounded<R, B> {
    match r {
        Approximation::Exact(v) => {
            let (re, im) = v.into_parts();
            Approximation::Exact(CBig::from_parts(-im, re))
        }
        Approximation::Inexact(v, (r_re, r_im)) => {
            let (re, im) = v.into_parts();
            Approximation::Inexact(CBig::from_parts(-im, re), (neg_rounding(r_im), r_re))
        }
    }
}

/// Negating a value flips the direction of its rounding adjustment (`AddOne ↔ SubOne`); see
/// `log`'s zero-path `-π` handling for the same reasoning.
fn neg_rounding(r: Rounding) -> Rounding {
    match r {
        Rounding::NoOp => Rounding::NoOp,
        Rounding::AddOne => Rounding::SubOne,
        Rounding::SubOne => Rounding::AddOne,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dashu_base::{Abs, AbsOrd};
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

    fn within_parts(a: &C, b: &C, k: u32) -> bool {
        let (ar, ai) = a.clone().into_parts();
        let (br, bi) = b.clone().into_parts();
        within(&ar, &br, k) && within(&ai, &bi, k)
    }

    #[test]
    fn sinh_zero_is_zero() {
        // `C::ZERO` is unlimited-precision — the exact zero shortcut must not trip the
        // limited-precision check.
        assert!(C::ZERO.sinh() == C::ZERO);
    }

    #[test]
    fn cosh_zero_is_one() {
        assert!(C::ZERO.cosh() == C::ONE);
    }

    #[test]
    fn sinh_cosh_zero_is_zero_one() {
        let (s, c) = C::ZERO.sinh_cosh();
        assert!(s == C::ZERO);
        assert!(c == C::ONE);
    }

    #[test]
    fn tanh_zero_is_zero() {
        assert!(C::ZERO.tanh() == C::ZERO);
    }

    #[test]
    fn sinh_cosh_identity() {
        // cosh²z - sinh²z = 1 (small z, so no cancellation against 1)
        let z = c(1, 1);
        let (s, co) = z.sinh_cosh();
        let diff = &co.sqr() - &s.sqr();
        let (re, im) = diff.into_parts();
        assert!((re.clone() - F::ONE)
            .abs()
            .abs_cmp(&F::from_parts(1.into(), -12))
            .is_le());
        assert!(im.abs_cmp(&F::from_parts(1.into(), -12)).is_le());
    }

    #[test]
    fn sinh_i_is_i_sin() {
        // sinh(i·z) = i·sin(z) for a limited-precision z — cross-checks the hyperbolic against
        // the independently-certified circular sin.
        let z = c(1, 1);
        let lhs = z.mul_i(false).sinh();
        let rhs = z.sin().mul_i(false);
        assert!(within_parts(&lhs, &rhs, 16));
    }

    #[test]
    fn cosh_i_is_cos() {
        // cosh(i·z) = cos(z)
        let z = c(1, 1);
        let lhs = z.mul_i(false).cosh();
        let rhs = z.cos();
        assert!(within_parts(&lhs, &rhs, 16));
    }

    #[test]
    fn tanh_i_is_i_tan() {
        // tanh(i·z) = i·tan(z)
        let z = c(1, 1);
        let lhs = z.mul_i(false).tanh();
        let rhs = z.tan().mul_i(false);
        assert!(within_parts(&lhs, &rhs, 16));
    }

    #[test]
    fn asinh_zero_is_zero() {
        // limited-precision zero (asinh rejects unlimited precision)
        assert!(c(0, 0).asinh() == C::ZERO);
    }

    #[test]
    fn asinh_sinh_roundtrip() {
        let z = c(1, 1);
        let r = z.sinh().asinh();
        assert!(within_parts(&r, &z, 32));
    }

    #[test]
    fn acosh_one_is_zero() {
        // limited-precision one (acosh rejects unlimited precision)
        assert!(c(1, 0).acosh() == C::ZERO);
    }

    #[test]
    fn acosh_cosh_roundtrip() {
        let z = c(1, 1);
        let r = z.cosh().acosh();
        assert!(within_parts(&r, &z, 32));
    }

    #[test]
    fn atanh_zero_is_zero() {
        // limited-precision zero (atanh rejects unlimited precision)
        assert!(c(0, 0).atanh() == C::ZERO);
    }

    #[test]
    fn atanh_tanh_roundtrip() {
        let z = c(1, 1);
        let r = z.tanh().atanh();
        assert!(within_parts(&r, &z, 32));
    }

    // The forward hyperbolics' exact zero shortcuts work at unlimited precision (like `sin_cos`);
    // the inverse hyperbolics reject it (like the inverse trig).
    #[test]
    #[should_panic(expected = "precision cannot be 0")]
    fn complex_sinh_unlimited_panics() {
        let _ = C::I.sinh();
    }

    #[test]
    #[should_panic(expected = "precision cannot be 0")]
    fn complex_cosh_unlimited_panics() {
        let _ = C::I.cosh();
    }

    #[test]
    #[should_panic(expected = "precision cannot be 0")]
    fn complex_tanh_unlimited_panics() {
        let _ = C::I.tanh();
    }

    #[test]
    #[should_panic(expected = "precision cannot be 0")]
    fn complex_asinh_unlimited_panics() {
        let _ = C::ONE.asinh();
    }

    #[test]
    #[should_panic(expected = "precision cannot be 0")]
    fn complex_acosh_unlimited_panics() {
        let _ = C::ONE.acosh();
    }

    #[test]
    #[should_panic(expected = "precision cannot be 0")]
    fn complex_atanh_unlimited_panics() {
        let _ = C::ONE.atanh();
    }

    #[test]
    fn sinh_cosh_signed_zero() {
        // Annex-G signed-zero cases: `csinh(±0 ± i·0)` carries the input zeros' signs per part;
        // `ccosh`'s imaginary part is the signed product (`−0` iff the two parts are
        // opposite-signed zeros).
        let fctx = dashu_float::Context::<mode::HalfAway>::new(53);
        let (neg0, pos0) = (F::from_repr(Repr::neg_zero(), fctx), F::from_repr(Repr::zero(), fctx));
        for (z, s_re_neg, s_im_neg, c_im_neg) in [
            (C::from_parts(pos0.clone(), pos0.clone()), false, false, false), // +0 + i·0
            (C::from_parts(pos0.clone(), neg0.clone()), false, true, true),   // +0 − i·0
            (C::from_parts(neg0.clone(), pos0.clone()), true, false, true),   // −0 + i·0
            (C::from_parts(neg0.clone(), neg0.clone()), true, true, false),   // −0 − i·0
        ] {
            let (s, c) = z.sinh_cosh();
            assert_eq!(s.re().is_neg_zero(), s_re_neg, "sinh re sign for {z}");
            assert_eq!(s.im().is_neg_zero(), s_im_neg, "sinh im sign for {z}");
            assert!(c.re() == &Repr::one(), "cosh re is 1 for {z}");
            assert_eq!(c.im().is_neg_zero(), c_im_neg, "cosh im sign for {z}");
        }
    }

    #[test]
    fn tanh_signed_zero() {
        let fctx = dashu_float::Context::<mode::HalfAway>::new(53);
        let (neg0, pos0) = (F::from_repr(Repr::neg_zero(), fctx), F::from_repr(Repr::zero(), fctx));
        for (z, re_neg, im_neg) in [
            (C::from_parts(pos0.clone(), pos0.clone()), false, false), // +0 + i·0
            (C::from_parts(pos0.clone(), neg0.clone()), false, true),  // +0 − i·0
            (C::from_parts(neg0.clone(), pos0.clone()), true, false),  // −0 + i·0
            (C::from_parts(neg0.clone(), neg0.clone()), true, true),   // −0 − i·0
        ] {
            let t = z.tanh();
            assert_eq!(t.re().is_neg_zero(), re_neg, "tanh re sign for {z}");
            assert_eq!(t.im().is_neg_zero(), im_neg, "tanh im sign for {z}");
        }
    }

    // The rotation identities must hold at every precision the AGENTS.md convention requires
    // (20/50/100/500 bits), cross-checked against the independently-certified circular functions.
    #[test]
    fn hyper_matches_trig_via_rotation() {
        type C2 = CBig<mode::HalfEven, 2>;
        type F2 = FBig<mode::HalfEven, 2>;

        fn within2(a: &F2, b: &F2, k: u32) -> bool {
            if a == b {
                return true;
            }
            let diff = (a.clone() - b.clone()).abs();
            diff.abs_cmp(&(a.ulp() * F2::from(k))).is_le()
        }

        for p in [20usize, 50, 100, 500] {
            let mk = |re: f64, im: f64| -> C2 {
                CBig::from_parts(
                    F2::try_from(re).unwrap().with_precision(p).value(),
                    F2::try_from(im).unwrap().with_precision(p).value(),
                )
            };
            for (re, im) in [(0.3, 1.7), (-2.5, 0.5), (1.0, -0.25)] {
                let z = mk(re, im);
                let iz = z.mul_i(false);
                // sinh(i·z) = i·sin(z)
                let lhs = iz.sinh();
                let rhs = z.sin().mul_i(false);
                let (lr, li) = lhs.into_parts();
                let (rr, ri) = rhs.into_parts();
                assert!(within2(&lr, &rr, 32) && within2(&li, &ri, 32), "sinh(i·z) p={p} z={z:?}");
                // cosh(i·z) = cos(z)
                let lhs = iz.cosh();
                let rhs = z.cos();
                let (lr, li) = lhs.into_parts();
                let (rr, ri) = rhs.into_parts();
                assert!(within2(&lr, &rr, 32) && within2(&li, &ri, 32), "cosh(i·z) p={p} z={z:?}");
                // tanh(i·z) = i·tan(z)
                let lhs = iz.tanh();
                let rhs = z.tan().mul_i(false);
                let (lr, li) = lhs.into_parts();
                let (rr, ri) = rhs.into_parts();
                assert!(within2(&lr, &rr, 32) && within2(&li, &ri, 32), "tanh(i·z) p={p} z={z:?}");
            }
        }
    }
}
