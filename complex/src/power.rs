//! Complex powers: `powi` (integer exponent, repeated squaring) and `powf` (`exp(w·log z)`).

use crate::cbig::CBig;
use crate::context::{combine_parts, reborrow_cache, CfpResult, Context};
use dashu_base::Approximation::*;
use dashu_base::{BitTest, Sign};
use dashu_float::round::Round;
use dashu_float::ConstCache;
use dashu_int::{IBig, Word};

/// Guard digits (base-B) for `powf`. Composes `log`, a complex product, and `exp` — the
/// cancellation-prone path, so a larger guard than the bare arithmetic ops.
const POWF_GUARD: usize = 22;

impl<R: Round> Context<R> {
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
    use dashu_float::FBig;

    type C = CBig<mode::HalfAway, 10>;
    type F = FBig<mode::HalfAway, 10>;

    fn c(re: i32, im: i32) -> C {
        let mk = |v: i32| -> F { F::from(v).with_precision(53).value() };
        CBig::from_parts(mk(re), mk(im))
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
