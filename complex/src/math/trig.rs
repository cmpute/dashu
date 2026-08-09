//! Complex trigonometric functions via the real–imaginary decomposition, reusing `dashu-float`'s
//! real `sin`/`cos` and cancellation-free `sinh`/`cosh`.
//!
//! `sin(x+iy) = sin x·cosh y + i·cos x·sinh y`, `cos(x+iy) = cos x·cosh y − i·sin x·sinh y`. This
//! form avoids the `exp(±iz)` identity's exponential blow-up for large `|Im z|`.

use crate::cbig::CBig;
use crate::repr::{combine_parts, reborrow_cache, CfpResult, Context};
use dashu_base::Sign;
use dashu_float::round::ErrorBounds;
use dashu_float::{ConstCache, Context as FloatCtxt, FBig, FpError, Repr};
use dashu_int::{IBig, Word};

/// Guard digits (base-B) for the forward trig. Composes real `sin_cos` + `sinh_cosh` + two
/// products; the cancellation near the trig zeros is absorbed by the re-round.
const TRIG_GUARD: usize = 16;

impl<R: ErrorBounds> Context<R> {
    /// Simultaneously compute `sin z` and `cos z` (context layer), correctly rounded via a shared
    /// Ziv loop. Returns `(sin, cos)` each as a [`CfpResult`]. An infinite input maps to
    /// [`FpError::Indeterminate`] (the C99 NaN cases).
    pub fn sin_cos<const B: Word>(
        &self,
        z: &CBig<R, B>,
        mut cache: Option<&mut ConstCache>,
    ) -> (CfpResult<R, B>, CfpResult<R, B>) {
        if z.is_infinite() {
            return (Err(FpError::Indeterminate), Err(FpError::Indeterminate));
        }
        if z.is_zero() {
            let (re, im) = (z.re(), z.im());
            // sin(x+iy) = sinx·coshy + i·cosx·sinhy: at ±0 the parts carry the input zeros' signs
            // (sin(±0) = ±0, sinh(±0) = ±0, cos(±0) = cosh(±0) = 1), so e.g. `csin(-0 + i·0) = -0 + i·0`.
            let sin = crate::repr::exact(
                FBig::from_repr(Repr::zero_with_sign(re.sign()), self.float()),
                FBig::from_repr(Repr::zero_with_sign(im.sign()), self.float()),
            );
            // cos(x+iy) = cosx·coshy − i·sinx·sinhy: real = 1; the imaginary part is the signed
            // product `x·y` (`−0` iff the two parts are opposite-signed zeros) — the Annex-G table
            // value, which differs from the naive `−sinx·sinhy` propagation (`ccos(-0 + i·0) = 1 - i·0`).
            let cos_im = if re.sign() != im.sign() {
                Sign::Negative
            } else {
                Sign::Positive
            };
            let cos = crate::repr::exact(
                FBig::from_repr(Repr::one(), self.float()),
                FBig::from_repr(Repr::zero_with_sign(cos_im), self.float()),
            );
            return (Ok(sin), Ok(cos));
        }

        // `sin z = sinx·coshy + i·cosx·sinhy`, `cos z = cosx·coshy − i·sinx·sinhy`. The four products
        // share one evaluation of the real `sin_cos`/`sinh_cosh` (each correctly-rounded at the
        // working precision, contributing ~0); only the products round, a few working-ULPs each. A
        // single 4-part Ziv loop certifies all of `sin` and `cos` together.
        let p = self.precision();
        let parts = self.ziv(TRIG_GUARD, |guard| {
            let gctx = FloatCtxt::<R>::new(p + guard);
            let (sinx, cosx) = gctx.sin_cos(z.re(), reborrow_cache(&mut cache));
            let sinx = sinx?.value();
            let cosx = cosx?.value();
            let (sinhy, coshy) = gctx.sinh_cosh(z.im(), reborrow_cache(&mut cache));
            let sinhy = sinhy?.value();
            let coshy = coshy?.value();
            let sin_re = gctx.mul(sinx.repr(), coshy.repr())?.value();
            let sin_im = gctx.mul(cosx.repr(), sinhy.repr())?.value();
            let cos_re = gctx.mul(cosx.repr(), coshy.repr())?.value();
            let neg_sinx = -sinx; // cos z's imaginary part is −sinx·sinhy
            let cos_im = gctx.mul(neg_sinx.repr(), sinhy.repr())?.value();
            Ok([
                (sin_re.clone(), sin_re.ulp() * 8),
                (sin_im.clone(), sin_im.ulp() * 8),
                (cos_re.clone(), cos_re.ulp() * 8),
                (cos_im.clone(), cos_im.ulp() * 8),
            ])
        });
        let [sin_re, sin_im, cos_re, cos_im] = match parts {
            Ok(arr) => arr,
            // an overflow (e.g. `cosh` of a huge imaginary part) fails both sin and cos together.
            Err(e) => return (Err(e), Err(e)),
        };
        (Ok(combine_parts(sin_re, sin_im)), Ok(combine_parts(cos_re, cos_im)))
    }

    /// Complex sine (context layer).
    #[inline]
    pub fn sin<const B: Word>(
        &self,
        z: &CBig<R, B>,
        cache: Option<&mut ConstCache>,
    ) -> CfpResult<R, B> {
        self.sin_cos(z, cache).0
    }

    /// Complex cosine (context layer).
    #[inline]
    pub fn cos<const B: Word>(
        &self,
        z: &CBig<R, B>,
        cache: Option<&mut ConstCache>,
    ) -> CfpResult<R, B> {
        self.sin_cos(z, cache).1
    }

    /// Complex tangent (context layer), correctly rounded via a Ziv loop, using the cancellation-free
    /// double-angle identity
    ///
    /// `tan(x+iy) = (sin 2x + i·sinh 2y) / (cos 2x + cosh 2y)`.
    ///
    /// The denominator `cos 2x + cosh 2y` is a sum of a bounded term (`cos 2x ∈ [−1, 1]`) and a
    /// term `≥ 1` (`cosh 2y`), so it never catastrophically cancels — unlike `sin z / cos z`, whose
    /// `sin·conj(cos)` real part cancels from `~cosh²y` down to `O(1)` for large `|Im z|`. The result
    /// is accurate for all finite `|Im z|`; the only small-denominator points are the real-axis poles
    /// (`y = 0, x = π/2 + kπ`), where the large value is genuine, not an artifact.
    pub fn tan<const B: Word>(
        &self,
        z: &CBig<R, B>,
        mut cache: Option<&mut ConstCache>,
    ) -> CfpResult<R, B> {
        if z.is_infinite() {
            // An infinite input maps to Indeterminate (the C99 NaN case), like the other complex
            // transcendentals — not the float layer's InfiniteInput.
            return Err(FpError::Indeterminate);
        }
        if z.is_zero() {
            let (re, im) = (z.re(), z.im());
            // tan(x+iy) = sin(x+iy)/cos(x+iy); at ±0 the parts carry the input zeros' signs, like
            // sin(±0) = ±0 — so ctan(-0 + i·0) = -0 + i·0. (Also bypasses the Ziv loop, which
            // rejects unlimited precision, e.g. `CBig::ZERO.tan()`.)
            return Ok(crate::repr::exact(
                FBig::from_repr(Repr::zero_with_sign(re.sign()), self.float()),
                FBig::from_repr(Repr::zero_with_sign(im.sign()), self.float()),
            ));
        }

        let p = self.precision();
        let [re, im] = self.ziv(TRIG_GUARD, |guard| {
            let pw = p + guard;
            let gctx = FloatCtxt::<R>::new(pw);
            // 2x, 2y (exact doublings — same significand, exponent +1).
            let x2 = gctx.add(z.re(), z.re())?.value();
            let y2 = gctx.add(z.im(), z.im())?.value();
            let (sin2x, cos2x) = gctx.sin_cos(x2.repr(), reborrow_cache(&mut cache));
            let sin2x = sin2x?.value();
            let cos2x = cos2x?.value();
            let (sinh2y, cosh2y) = gctx.sinh_cosh(y2.repr(), reborrow_cache(&mut cache));
            let sinh2y = sinh2y?.value();
            let cosh2y = cosh2y?.value();
            // D = cos 2x + cosh 2y  (a benign sum: a bounded term plus one ≥ 1).
            let denom = gctx.add(cos2x.repr(), cosh2y.repr())?.value();
            let re = gctx.div(sin2x.repr(), denom.repr())?.value();
            let im = gctx.div(sinh2y.repr(), denom.repr())?.value();
            // re-root to the working precision (`sin_cos`/`sinh_cosh`/`div` may return exact
            // constants for exact cases such as `tan(0) = 0`).
            let re = re.with_precision(pw).value();
            let im = im.with_precision(pw).value();
            Ok([(re.clone(), re.ulp() * 8), (im.clone(), im.ulp() * 8)])
        })?;
        Ok(combine_parts(re, im))
    }

    /// Inverse sine `asin z = -i·log(iz + sqrt(1-z²))` (context layer, Kahan form), correctly
    /// rounded via a Ziv loop. The argument of the inner `log` always has positive real part, so the
    /// branch cut comes entirely from the `sqrt`; an infinite input maps to
    /// [`FpError::Indeterminate`]. The `1-z²` under the `sqrt` is computed in the factored form
    /// `(1-z)(1+z)`, which is Sterbenz-exact near `z = ±1` (where the direct `1-z²` would
    /// catastrophically cancel against the `sqr` rounding error), so the well-conditioned regime
    /// extends right up to the singularities. A generous constant radius covers the
    /// square/subtract/sqrt/add/log composition; the Ziv retries absorb the `sqrt` amplification as
    /// `1-z² → 0`.
    pub fn asin<const B: Word>(
        &self,
        z: &CBig<R, B>,
        mut cache: Option<&mut ConstCache>,
    ) -> CfpResult<R, B> {
        if z.is_infinite() {
            return Err(FpError::Indeterminate);
        }
        let p = self.precision();
        let [re, im] = self.ziv(ITRIG_GUARD, |guard| {
            let pw = p + guard;
            let gctx = Context::new(pw);
            let one = CBig::ONE;
            // Factor `1-z² = (1-z)(1+z)`. Near `z = ±1` the direct `1 - z²` subtracts a value
            // dominated by the `sqr` rounding error from 1 (catastrophic cancellation); the factored
            // form is Sterbenz-exact there (`1-z` is computed exactly, since the subtraction's
            // significand difference is exact), so the radius stays sound right up to the singularity.
            let one_m_z = gctx.sub(&one, z)?.value();
            let one_p_z = gctx.add(&one, z)?.value();
            let one_m_z2 = gctx.mul(&one_m_z, &one_p_z)?.value();
            let sqrt_term = gctx.sqrt(&one_m_z2)?.value();
            let iz = z.mul_i(false); // exact rotation
            let w = gctx.add(&iz, &sqrt_term)?.value();
            let log_w = gctx.log(&w, reborrow_cache(&mut cache))?.value();
            let asin_z = log_w.mul_i(true); // -i·log(w)
            let (re, im) = asin_z.into_parts();
            // re-root to the working precision (`log` may return an exact constant for exact cases).
            let re = re.with_precision(pw).value();
            let im = im.with_precision(pw).value();
            Ok([(re.clone(), re.ulp() * 20), (im.clone(), im.ulp() * 20)])
        })?;
        Ok(combine_parts(re, im))
    }

    /// Inverse cosine `acos z = -i·log(z + i·sqrt(1-z²))` (context layer, Kahan form), correctly
    /// rounded via a Ziv loop. Same composition and singularity structure as `asin` (including the
    /// factored `1-z² = (1-z)(1+z)` near `z = ±1`).
    pub fn acos<const B: Word>(
        &self,
        z: &CBig<R, B>,
        mut cache: Option<&mut ConstCache>,
    ) -> CfpResult<R, B> {
        if z.is_infinite() {
            return Err(FpError::Indeterminate);
        }
        let p = self.precision();
        let [re, im] = self.ziv(ITRIG_GUARD, |guard| {
            let pw = p + guard;
            let gctx = Context::new(pw);
            let one = CBig::ONE;
            // Factored `1-z² = (1-z)(1+z)` — Sterbenz-exact near `z = ±1` (see `asin`).
            let one_m_z = gctx.sub(&one, z)?.value();
            let one_p_z = gctx.add(&one, z)?.value();
            let one_m_z2 = gctx.mul(&one_m_z, &one_p_z)?.value();
            let sqrt_term = gctx.sqrt(&one_m_z2)?.value();
            let i_sqrt = sqrt_term.mul_i(false); // i·sqrt(1-z²)
            let w = gctx.add(z, &i_sqrt)?.value();
            let log_w = gctx.log(&w, reborrow_cache(&mut cache))?.value();
            let acos_z = log_w.mul_i(true); // -i·log(w)
            let (re, im) = acos_z.into_parts();
            let re = re.with_precision(pw).value();
            let im = im.with_precision(pw).value();
            Ok([(re.clone(), re.ulp() * 20), (im.clone(), im.ulp() * 20)])
        })?;
        Ok(combine_parts(re, im))
    }

    /// Inverse tangent `atan z = (i/2)·(log(1-iz) - log(1+iz))` (context layer), correctly rounded
    /// via a Ziv loop. The two logs nearly cancel for small `z`; near `z = ±i` one of `1∓iz`
    /// vanishes and its log diverges. The Ziv retries absorb the cancellation in the
    /// well-conditioned regime.
    pub fn atan<const B: Word>(
        &self,
        z: &CBig<R, B>,
        mut cache: Option<&mut ConstCache>,
    ) -> CfpResult<R, B> {
        if z.is_infinite() {
            // atan(±∞) = ±π/2; defer the exact constant to the formula via the limit, but the
            // 1±iz terms become infinite and the log diverges — report Indeterminate for now.
            return Err(FpError::Indeterminate);
        }
        let p = self.precision();
        let [re, im] = self.ziv(ITRIG_GUARD, |guard| {
            let pw = p + guard;
            let gctx = Context::new(pw);
            let one = CBig::ONE;
            let iz = z.mul_i(false);
            let a = gctx.sub(&one, &iz)?.value(); // 1 - iz
            let b = gctx.add(&one, &iz)?.value(); // 1 + iz
            let log_a = gctx.log(&a, reborrow_cache(&mut cache))?.value();
            let log_b = gctx.log(&b, reborrow_cache(&mut cache))?.value();
            let diff = gctx.sub(&log_a, &log_b)?.value();
            let i_half_diff = diff.mul_i(false); // i·diff, then /2 below
            let two: CBig<R, B> = IBig::from(2).into();
            let atan_z = gctx.div(&i_half_diff, &two)?.value();
            let (re, im) = atan_z.into_parts();
            let re = re.with_precision(pw).value();
            let im = im.with_precision(pw).value();
            Ok([(re.clone(), re.ulp() * 20), (im.clone(), im.ulp() * 20)])
        })?;
        Ok(combine_parts(re, im))
    }
}

/// Guard digits (base-B) for the inverse trig (squares, a sqrt, logs, and a divide).
const ITRIG_GUARD: usize = 18;

impl<R: ErrorBounds, const B: Word> CBig<R, B> {
    /// Complex sine (convenience layer). Panics on an indeterminate special value.
    #[inline]
    pub fn sin(&self) -> Self {
        self.context().unwrap_cfp(self.context().sin(self, None))
    }

    /// Complex cosine (convenience layer). Panics on an indeterminate special value.
    #[inline]
    pub fn cos(&self) -> Self {
        self.context().unwrap_cfp(self.context().cos(self, None))
    }

    /// Simultaneously compute `(sin z, cos z)` (convenience layer).
    #[inline]
    pub fn sin_cos(&self) -> (Self, Self) {
        let (s, c) = self.context().sin_cos(self, None);
        (self.context().unwrap_cfp(s), self.context().unwrap_cfp(c))
    }

    /// Complex tangent (convenience layer).
    #[inline]
    pub fn tan(&self) -> Self {
        self.context().unwrap_cfp(self.context().tan(self, None))
    }

    /// Inverse sine (convenience layer).
    #[inline]
    pub fn asin(&self) -> Self {
        self.context().unwrap_cfp(self.context().asin(self, None))
    }

    /// Inverse cosine (convenience layer).
    #[inline]
    pub fn acos(&self) -> Self {
        self.context().unwrap_cfp(self.context().acos(self, None))
    }

    /// Inverse tangent (convenience layer).
    #[inline]
    pub fn atan(&self) -> Self {
        self.context().unwrap_cfp(self.context().atan(self, None))
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
        CBig::from_parts(mk(re), mk(im))
    }

    #[test]
    fn sin_zero_is_zero() {
        assert!(C::ZERO.sin() == C::ZERO);
    }

    #[test]
    fn tan_zero_is_zero() {
        // tan has an exact-zero shortcut, so it works even at unlimited precision
        // (the constants are precision 0, which the Ziv loop rejects).
        assert!(C::ZERO.tan() == C::ZERO);
    }

    #[test]
    fn tan_infinite_is_indeterminate() {
        let inf = C::from(F::INFINITY);
        let ctx = Context::new(53);
        assert_eq!(ctx.tan(&inf, None), Err(FpError::Indeterminate));
    }

    #[test]
    fn cos_zero_is_one() {
        assert!(C::ZERO.cos() == C::ONE);
    }

    #[test]
    fn pythagorean_identity() {
        // sin²z + cos²z = 1
        let z = c(1, 1);
        let s = z.sin();
        let co = z.cos();
        let sum = &s.sqr() + &co.sqr();
        // purely real ≈ 1, imaginary ≈ 0
        let (re, im) = sum.into_parts();
        use dashu_base::{Abs, AbsOrd};
        assert!((re.clone() - F::ONE)
            .abs()
            .abs_cmp(&F::from_parts(1.into(), -12))
            .is_le());
        assert!(im.abs_cmp(&F::from_parts(1.into(), -12)).is_le());
    }

    #[test]
    fn tan_large_imaginary_is_near_i() {
        use dashu_base::{Abs, AbsOrd};
        // tan(x + i·100) ≈ i: real part → 0, imaginary → tanh(100) ≈ 1. The cancellation-free
        // double-angle form computes this accurately; the naive `sin/cos` division would cancel the
        // real part to noise for such a large `|Im z|` (the motivating case for the new formula).
        let (re, im) = c(1, 100).tan().into_parts();
        let tol = F::from_parts(1.into(), -40);
        assert!(re.abs().abs_cmp(&tol).is_le());
        assert!((im - F::from(1)).abs().abs_cmp(&tol).is_le());
    }

    #[test]
    fn sin_i_is_i_sinh_one() {
        // sin(i) = i·sinh(1) = i·1.1752… ; purely imaginary. Use a *limited*-precision input —
        // `sin` rejects unlimited precision (it would otherwise silently compute at `TRIG_GUARD`).
        let s = c(0, 1).sin();
        assert!(s.re().significand().is_zero());
        assert!(!s.im().significand().is_zero());
    }

    #[test]
    fn asin_zero_is_zero() {
        // limited-precision input (asin rejects unlimited precision)
        assert!(c(0, 0).asin() == C::ZERO);
    }

    #[test]
    fn asin_one_is_half_pi() {
        use dashu_base::{Abs, AbsOrd};
        // asin(1) = π/2 (limited-precision input — asin rejects unlimited precision)
        let (re, im) = c(1, 0).asin().into_parts();
        let half_pi = F::from_parts(15707963267948966i64.into(), -16)
            .with_precision(60)
            .value();
        assert!((re.clone() - half_pi)
            .abs()
            .abs_cmp(&F::from_parts(1.into(), -12))
            .is_le());
        assert!(im.abs_cmp(&F::from_parts(1.into(), -12)).is_le());
    }

    #[test]
    fn acos_zero_is_half_pi() {
        use dashu_base::{Abs, AbsOrd};
        // limited-precision input (acos rejects unlimited precision)
        let (re, _im) = c(0, 0).acos().into_parts();
        let half_pi = F::from_parts(15707963267948966i64.into(), -16)
            .with_precision(60)
            .value();
        assert!((re - half_pi)
            .abs()
            .abs_cmp(&F::from_parts(1.into(), -12))
            .is_le());
    }

    #[test]
    fn atan_one_is_quarter_pi() {
        use dashu_base::{Abs, AbsOrd};
        // atan(1) = π/4 (limited-precision input — atan rejects unlimited precision)
        let (re, _im) = c(1, 0).atan().into_parts();
        let quarter_pi = F::from_parts(7853981633974483i64.into(), -16)
            .with_precision(60)
            .value();
        assert!((re - quarter_pi)
            .abs()
            .abs_cmp(&F::from_parts(1.into(), -12))
            .is_le());
    }

    #[test]
    fn sin_asin_roundtrip() {
        // asin(sin z) ≈ z for a small z (within the principal range)
        let z = c(1, 1);
        let r = z.sin().asin();
        assert!(r == z);
    }

    // The trig functions reject unlimited precision. `sin`/`cos`/`tan` do so via `guard`; the
    // inverse trig (`asin`/`acos`/`atan`) build their work context directly and assert explicitly
    // (like `powf`). The zero shortcuts (`C::ZERO.sin()` etc.) bypass the check, as they're exact.
    #[test]
    #[should_panic(expected = "precision cannot be 0")]
    fn complex_sin_unlimited_panics() {
        let _ = C::I.sin();
    }

    #[test]
    #[should_panic(expected = "precision cannot be 0")]
    fn complex_asin_unlimited_panics() {
        let _ = C::ONE.asin();
    }

    #[test]
    fn sin_cos_signed_zero() {
        // Annex-G signed-zero cases for exactly-zero input: `csin(±0 ± i·0)` carries the input
        // zeros' signs per part; `ccos`'s imaginary part is the signed product `x·y` (`−0` iff the
        // two parts are opposite-signed — e.g. `ccos(-0 + i·0) = 1 - i·0`).
        let fctx = dashu_float::Context::<mode::HalfAway>::new(53);
        let (neg0, pos0) = (F::from_repr(Repr::neg_zero(), fctx), F::from_repr(Repr::zero(), fctx));
        for (z, s_re_neg, s_im_neg, c_im_neg) in [
            (C::from_parts(pos0.clone(), pos0.clone()), false, false, false), // +0 + i·0
            (C::from_parts(pos0.clone(), neg0.clone()), false, true, true),   // +0 − i·0
            (C::from_parts(neg0.clone(), pos0.clone()), true, false, true),   // −0 + i·0
            (C::from_parts(neg0.clone(), neg0.clone()), true, true, false),   // −0 − i·0
        ] {
            let (s, c) = z.sin_cos();
            assert_eq!(s.re().is_neg_zero(), s_re_neg, "sin re sign for {z}");
            assert_eq!(s.im().is_neg_zero(), s_im_neg, "sin im sign for {z}");
            assert!(c.re() == &Repr::one(), "cos re is 1 for {z}");
            assert_eq!(c.im().is_neg_zero(), c_im_neg, "cos im sign for {z}");
        }
    }
}
