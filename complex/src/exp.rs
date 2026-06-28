//! Complex exponential and powers.
//!
//! * [`Context::exp`] / [`CBig::exp`]: `exp(x+iy) = e^x·(cos y + i sin y)`.
//! * [`Context::powi`] / [`CBig::powi`]: integer exponent via repeated squaring (branch-cut-free,
//!   cheaper than `exp(n·log z)`).
//! * [`Context::powf`] / [`CBig::powf`]: `exp(w·log z)` on the principal branch.
//!
//! Mirroring `dashu-float`, the power family lives alongside `exp` in a single module.

use crate::cbig::CBig;
use crate::repr::{combine_parts, exact, reborrow_cache, riemann, CfpResult, Context};
use dashu_base::Approximation::*;
use dashu_base::{BitTest, Sign};
use dashu_float::round::Round;
use dashu_float::{ConstCache, FBig, FpError};
use dashu_int::{IBig, Word};

/// Guard digits (base-B) for `exp`. Composes a real `exp`, a `sin_cos`, and two products.
const EXP_GUARD: usize = 14;

/// Guard digits (base-B) for `powf`. Composes `log`, a complex product, and `exp` — the
/// cancellation-prone path, so a larger guard than the bare arithmetic ops.
const POWF_GUARD: usize = 22;

impl<R: Round> Context<R> {
    /// Complex exponential under this context (context layer). Reuses `dashu-float`'s `exp` and
    /// `sin_cos`; the cache is threaded into both (the convenience layer passes `None`).
    ///
    /// Special values: `exp(0) = 1`; `exp(+inf + i·finite) = +∞` (Riemann point);
    /// `exp(-inf + i·finite) = 0`; an infinite imaginary part makes the trig undefined
    /// (`Indeterminate`).
    pub fn exp<const B: Word>(
        &self,
        z: &CBig<R, B>,
        mut cache: Option<&mut ConstCache>,
    ) -> CfpResult<R, B> {
        if z.is_zero() {
            return Ok(exact(FBig::ONE, FBig::ZERO));
        }
        if z.is_infinite() {
            if z.imag().is_infinite() {
                return Err(FpError::Indeterminate); // cos/sin(±inf) undefined
            }
            return if z.re().sign() == Sign::Positive {
                Ok(riemann(*self))
            } else {
                Ok(exact(FBig::ZERO, FBig::ZERO))
            };
        }

        let gctx = self.guard(EXP_GUARD);
        let p = self.precision();
        let ex = gctx.exp(z.re(), reborrow_cache(&mut cache))?.value();
        let (sin_y, cos_y) = gctx.sin_cos(z.imag(), reborrow_cache(&mut cache));
        let cos_y = cos_y?.value();
        let sin_y = sin_y?.value();
        let re = gctx.mul(ex.repr(), cos_y.repr())?.value().with_precision(p);
        let im = gctx.mul(ex.repr(), sin_y.repr())?.value().with_precision(p);
        Ok(combine_parts(re, im))
    }

    /// Raise a complex number to an integer power under this context (context layer), via repeated
    /// squaring (branch-cut-free, cheaper than `exp(n·log z)`). No cache.
    ///
    /// `powi(z, 0) = 1`; a negative exponent computes `powi(z, |n|)` then inverts.
    pub fn powi<const B: Word>(&self, z: &CBig<R, B>, exp: IBig) -> CfpResult<R, B> {
        let (sign, n) = exp.into_parts();
        if n.is_zero() {
            return Ok(Exact(CBig::ONE));
        }
        let negative = sign == Sign::Negative;
        let bitlen = n.bit_len();
        // left-to-right binary exponentiation, starting from the leading set bit
        let mut acc = z.clone();
        for i in (0..bitlen - 1).rev() {
            acc = self.sqr(&acc)?.value();
            if n.bit(i) {
                acc = self.mul(&acc, z)?.value();
            }
        }
        // The intermediate rounding flags are folded away (the value is near-correctly rounded);
        // for a negative exponent the final `inv` carries its own flags.
        if negative {
            self.inv(&acc)
        } else {
            Ok(Exact(acc))
        }
    }

    /// Raise `base` to a complex power under this context (context layer): `exp(w·log base)` on the
    /// principal branch, evaluated at `p + POWF_GUARD` and re-rounded. `powf(0, 0) = 1` (matching
    /// `FBig::powf`).
    ///
    /// Unlike `exp`, this drives whole-[`CBig`] operations (`log`/`mul`/`exp`), so it builds a
    /// complex working [`Context`] at guard precision directly rather than the float
    /// [`Context::guard`] (which yields a `FloatCtxt` for per-part math).
    pub fn powf<const B: Word>(
        &self,
        base: &CBig<R, B>,
        w: &CBig<R, B>,
        mut cache: Option<&mut ConstCache>,
    ) -> CfpResult<R, B> {
        if w.is_zero() {
            return Ok(Exact(CBig::ONE)); // powf(z, 0) = 1, incl. powf(0, 0)
        }
        let gctx = Context::new(self.precision() + POWF_GUARD);
        let log_z = gctx.log(base, reborrow_cache(&mut cache))?.value();
        let wlogz = gctx.mul(w, &log_z)?.value();
        let hi = gctx.exp(&wlogz, reborrow_cache(&mut cache))?.value();
        let p = self.precision();
        let (re, im) = hi.into_parts();
        Ok(combine_parts(re.with_precision(p), im.with_precision(p)))
    }
}

impl<R: Round, const B: Word> CBig<R, B> {
    /// Complex exponential `e^z` (convenience layer).
    ///
    /// # Panics
    ///
    /// Panics if the precision is unlimited or on an indeterminate special value.
    #[inline]
    pub fn exp(&self) -> Self {
        self.context().unwrap_cfp(self.context().exp(self, None))
    }

    /// Integer power (convenience layer).
    ///
    /// # Panics
    ///
    /// Panics on an indeterminate / out-of-domain result (e.g. `0⁻¹`).
    #[inline]
    pub fn powi(&self, exp: IBig) -> Self {
        self.context().unwrap_cfp(self.context().powi(self, exp))
    }

    /// Complex power `self^w` (convenience layer).
    ///
    /// `powf(z, 0) = 1` (including `powf(0, 0) = 1`), matching `FBig::powf` and the real `0⁰ = 1`
    /// convention.
    #[inline]
    pub fn powf(&self, w: &Self) -> Self {
        self.context()
            .unwrap_cfp(self.context().powf(self, w, None))
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
    fn exp_zero_is_one() {
        assert!(C::ZERO.exp() == C::ONE);
    }

    #[test]
    fn exp_one_is_e() {
        // exp(1+0i) = e ≈ 2.71828…; check 2 < e < 3 via the real part
        let e = C::ONE.exp();
        let (re, _im) = e.into_parts();
        assert!(re > F::from(2));
        assert!(re < F::from(3));
    }

    #[test]
    fn exp_pi_i_is_neg_one() {
        use dashu_base::{Abs, AbsOrd};
        // exp(iπ) = -1 + i·0; use a π literal precise enough that sin(π_approx) ≈ 0
        let pi = F::from_parts(31415926535897932i64.into(), -16)
            .with_precision(60)
            .value();
        let z = CBig::from_parts(F::ZERO, pi);
        let (re, im) = z.exp().into_parts();
        let re_err = (re + F::ONE).abs();
        let tol = F::from_parts(1.into(), -12);
        assert!(re_err.abs_cmp(&tol).is_le());
        assert!(im.abs_cmp(&tol).is_le());
    }

    #[test]
    fn exp_pos_infinity_is_riemann() {
        let inf = CBig::from(F::INFINITY);
        let r = inf.exp();
        assert!(r.re().is_infinite());
        assert!(r.imag().is_zero());
    }

    #[test]
    fn powi_zero_is_one() {
        assert!(c(3, 4).powi(0.into()) == C::ONE);
    }

    #[test]
    fn powi_one_is_self() {
        let z = c(3, 4);
        assert!(z.powi(1.into()) == z);
    }

    #[test]
    fn powi_two_is_sqr() {
        let z = c(1, 2);
        assert!(z.powi(2.into()) == z.sqr());
    }

    #[test]
    fn powi_negative_is_inv() {
        // z^(-1) = inv(z); z · z^(-1) = 1
        let z = c(3, 4);
        let r = z.powi((-1).into());
        let one = &z * &r;
        assert!(one == C::ONE);
    }

    #[test]
    fn powf_zero_exponent_is_one() {
        // powf(z, 0) = 1, including powf(0, 0)
        assert!(c(3, 4).powf(&C::ZERO) == C::ONE);
        assert!(C::ZERO.powf(&C::ZERO) == C::ONE);
    }

    #[test]
    fn powf_one_exponent_is_self() {
        let z = c(2, 1);
        assert!(z.powf(&C::ONE) == z);
    }
}
