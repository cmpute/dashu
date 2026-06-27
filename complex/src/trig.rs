//! Complex trigonometric functions via the real–imaginary decomposition, reusing `dashu-float`'s
//! real `sin`/`cos` and cancellation-free `sinh`/`cosh`.
//!
//! `sin(x+iy) = sin x·cosh y + i·cos x·sinh y`, `cos(x+iy) = cos x·cosh y − i·sin x·sinh y`. This
//! form avoids the `exp(±iz)` identity's exponential blow-up for large `|Im z|`.

use crate::cbig::CBig;
use crate::context::{combine_parts, reborrow_cache, CfpResult, Context};
use dashu_float::round::Round;
use dashu_float::{ConstCache, FBig, FpError, Repr};
use dashu_int::Word;

/// Guard digits (base-B) for the forward trig. Composes real `sin_cos` + `sinh`/`cosh` + two
/// products; the cancellation near the trig zeros is absorbed by the re-round.
const TRIG_GUARD: usize = 16;

impl<R: Round> Context<R> {
    /// Simultaneously compute `sin z` and `cos z` (context layer). Returns `(sin, cos)` each as a
    /// [`CfpResult`]. An infinite input maps to [`FpError::Indeterminate`] (the C99 NaN cases).
    pub fn sin_cos<const B: Word>(
        &self,
        z: &CBig<R, B>,
        mut cache: Option<&mut ConstCache>,
    ) -> (CfpResult<R, B>, CfpResult<R, B>) {
        if z.is_infinite() {
            return (Err(FpError::Indeterminate), Err(FpError::Indeterminate));
        }
        if z.is_zero() {
            let zero = ok_exact_zero(*self);
            let one = ok_exact_one(*self);
            return (zero, one);
        }

        let gctx = self.guard(TRIG_GUARD);
        let p = self.precision();
        let (sinx, cosx) = gctx.sin_cos(z.re(), reborrow_cache(&mut cache));
        let sinx = match sinx {
            Ok(v) => v.value(),
            Err(e) => return (Err(e), Err(FpError::Indeterminate)),
        };
        let cosx = match cosx {
            Ok(v) => v.value(),
            Err(e) => return (Err(FpError::Indeterminate), Err(e)),
        };
        let sinhy = match gctx.sinh(z.imag(), reborrow_cache(&mut cache)) {
            Ok(v) => v.value(),
            Err(e) => return (Err(e), Err(FpError::Indeterminate)),
        };
        let coshy = match gctx.cosh(z.imag(), reborrow_cache(&mut cache)) {
            Ok(v) => v.value(),
            Err(e) => return (Err(FpError::Indeterminate), Err(e)),
        };

        // sin z = (sinx·coshy) + i·(cosx·sinhy); cos z = (cosx·coshy) − i·(sinx·sinhy).
        // `sin_cos` returns a tuple, so the products are matched explicitly (no `?`).
        let prod = |a: &FBig<R, B>, b: &FBig<R, B>| -> Result<_, FpError> {
            Ok(gctx.mul(a.repr(), b.repr())?.value().with_precision(p))
        };
        let sin_re = match prod(&sinx, &coshy) {
            Ok(v) => v,
            Err(e) => return (Err(e), Err(FpError::Indeterminate)),
        };
        let sin_im = match prod(&cosx, &sinhy) {
            Ok(v) => v,
            Err(e) => return (Err(e), Err(FpError::Indeterminate)),
        };
        let cos_re = match prod(&cosx, &coshy) {
            Ok(v) => v,
            Err(e) => return (Err(FpError::Indeterminate), Err(e)),
        };
        let neg_sinx = -sinx;
        let cos_im = match prod(&neg_sinx, &sinhy) {
            Ok(v) => v,
            Err(e) => return (Err(FpError::Indeterminate), Err(e)),
        };
        (Ok(combine_parts(sin_re, sin_im)), Ok(combine_parts(cos_re, cos_im)))
    }

    /// Complex sine (context layer).
    pub fn sin<const B: Word>(
        &self,
        z: &CBig<R, B>,
        cache: Option<&mut ConstCache>,
    ) -> CfpResult<R, B> {
        self.sin_cos(z, cache).0
    }

    /// Complex cosine (context layer).
    pub fn cos<const B: Word>(
        &self,
        z: &CBig<R, B>,
        cache: Option<&mut ConstCache>,
    ) -> CfpResult<R, B> {
        self.sin_cos(z, cache).1
    }

    /// Complex tangent `sin z / cos z` (context layer).
    pub fn tan<const B: Word>(
        &self,
        z: &CBig<R, B>,
        cache: Option<&mut ConstCache>,
    ) -> CfpResult<R, B> {
        let (sin_z, cos_z) = self.sin_cos(z, cache);
        let sin_z = sin_z?;
        let cos_z = cos_z?;
        self.div(&sin_z.value(), &cos_z.value())
    }

    /// Inverse sine `asin z = -i·log(iz + sqrt(1-z²))` (context layer, Kahan form). The argument of
    /// the inner `log` always has positive real part, so the branch cut comes entirely from the
    /// `sqrt`; an infinite input maps to [`FpError::Indeterminate`].
    pub fn asin<const B: Word>(
        &self,
        z: &CBig<R, B>,
        mut cache: Option<&mut ConstCache>,
    ) -> CfpResult<R, B> {
        if z.is_infinite() {
            return Err(FpError::Indeterminate);
        }
        let gctx = Context::new(self.precision() + ITRIG_GUARD);
        let p = self.precision();
        let one = cbig_one(gctx);
        let z2 = gctx.sqr(z)?.value();
        let one_m_z2 = gctx.sub(&one, &z2)?.value();
        let sqrt_term = gctx.sqrt(&one_m_z2)?.value();
        let iz = z.mul_i(false); // exact rotation
        let w = gctx.add(&iz, &sqrt_term)?.value();
        let log_w = gctx.log(&w, reborrow_cache(&mut cache))?.value();
        let asin_z = log_w.mul_i(true); // -i·log(w)
        reround(asin_z, p)
    }

    /// Inverse cosine `acos z = -i·log(z + i·sqrt(1-z²))` (context layer, Kahan form).
    pub fn acos<const B: Word>(
        &self,
        z: &CBig<R, B>,
        mut cache: Option<&mut ConstCache>,
    ) -> CfpResult<R, B> {
        if z.is_infinite() {
            return Err(FpError::Indeterminate);
        }
        let gctx = Context::new(self.precision() + ITRIG_GUARD);
        let p = self.precision();
        let one = cbig_one(gctx);
        let z2 = gctx.sqr(z)?.value();
        let one_m_z2 = gctx.sub(&one, &z2)?.value();
        let sqrt_term = gctx.sqrt(&one_m_z2)?.value();
        let i_sqrt = sqrt_term.mul_i(false); // i·sqrt(1-z²)
        let w = gctx.add(z, &i_sqrt)?.value();
        let log_w = gctx.log(&w, reborrow_cache(&mut cache))?.value();
        let acos_z = log_w.mul_i(true); // -i·log(w)
        reround(acos_z, p)
    }

    /// Inverse tangent `atan z = (i/2)·(log(1-iz) - log(1+iz))` (context layer).
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
        let gctx = Context::new(self.precision() + ITRIG_GUARD);
        let p = self.precision();
        let one = cbig_one(gctx);
        let iz = z.mul_i(false);
        let a = gctx.sub(&one, &iz)?.value(); // 1 - iz
        let b = gctx.add(&one, &iz)?.value(); // 1 + iz
        let log_a = gctx.log(&a, reborrow_cache(&mut cache))?.value();
        let log_b = gctx.log(&b, reborrow_cache(&mut cache))?.value();
        let diff = gctx.sub(&log_a, &log_b)?.value();
        let i_half_diff = diff.mul_i(false); // i·diff, then /2 below
        let two = cbig_real(gctx, 2);
        let atan_z = gctx.div(&i_half_diff, &two)?.value();
        reround(atan_z, p)
    }
}

/// Guard digits (base-B) for the inverse trig (squares, a sqrt, logs, and a divide).
const ITRIG_GUARD: usize = 18;

fn cbig_one<R: Round, const B: Word>(ctx: Context<R>) -> CBig<R, B> {
    CBig::from(FBig::from_repr(Repr::one(), ctx.float()))
}

fn cbig_real<R: Round, const B: Word>(ctx: Context<R>, v: i32) -> CBig<R, B> {
    CBig::from(FBig::from_repr(Repr::new(v.into(), 0), ctx.float()))
}

fn reround<R: Round, const B: Word>(z: CBig<R, B>, p: usize) -> CfpResult<R, B> {
    let (re, im) = z.into_parts();
    Ok(combine_parts(re.with_precision(p), im.with_precision(p)))
}

fn ok_exact_zero<R: Round, const B: Word>(ctx: Context<R>) -> CfpResult<R, B> {
    Ok(crate::context::exact(
        FBig::from_repr(Repr::zero(), ctx.float()),
        FBig::from_repr(Repr::zero(), ctx.float()),
    ))
}

fn ok_exact_one<R: Round, const B: Word>(ctx: Context<R>) -> CfpResult<R, B> {
    Ok(crate::context::exact(
        FBig::from_repr(Repr::one(), ctx.float()),
        FBig::from_repr(Repr::zero(), ctx.float()),
    ))
}

impl<R: Round, const B: Word> CBig<R, B> {
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
    fn sin_i_is_i_sinh_one() {
        // sin(i) = i·sinh(1) = i·1.1752… ; purely imaginary
        let s = C::I.sin();
        assert!(s.re().significand().is_zero());
        assert!(!s.imag().significand().is_zero());
    }

    #[test]
    fn asin_zero_is_zero() {
        assert!(C::ZERO.asin() == C::ZERO);
    }

    #[test]
    fn asin_one_is_half_pi() {
        use dashu_base::{Abs, AbsOrd};
        // asin(1) = π/2
        let (re, im) = C::ONE.asin().into_parts();
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
        let (re, _im) = C::ZERO.acos().into_parts();
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
        // atan(1) = π/4
        let (re, _im) = C::ONE.atan().into_parts();
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
}
