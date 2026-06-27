//! Complex exponential `exp(x+iy) = e^x·(cos y + i sin y)`.

use crate::cbig::CBig;
use crate::context::{combine_parts, exact, reborrow_cache, riemann, CfpResult, Context};
use dashu_base::Sign;
use dashu_float::round::Round;
use dashu_float::{ConstCache, FBig, FpError};
use dashu_int::Word;

/// Guard digits (base-B) for `exp`. Composes a real `exp`, a `sin_cos`, and two products.
const EXP_GUARD: usize = 14;

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
}

#[cfg(test)]
mod tests {
    use super::*;
    use dashu_float::round::mode;

    type C = CBig<mode::HalfAway, 10>;
    type F = FBig<mode::HalfAway, 10>;

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
}
