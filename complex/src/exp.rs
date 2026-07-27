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
use dashu_base::{Abs, BitTest, Sign};
use dashu_float::round::ErrorBounds;
use dashu_float::{ConstCache, Context as FloatCtxt, FBig, FpError};
use dashu_int::{IBig, Word};

/// Guard digits (base-B) for `exp`. Composes a real `exp`, a `sin_cos`, and two products.
const EXP_GUARD: usize = 14;

/// Guard digits (base-B) for `powf`. Composes `log`, a complex product, and `exp` — the
/// cancellation-prone path, so a larger guard than the bare arithmetic ops.
const POWF_GUARD: usize = 22;

impl<R: ErrorBounds> Context<R> {
    /// Raise a complex number to an integer power under this context (context layer), correctly
    /// rounded via a Ziv loop over the binary-exponentiation (repeated-squaring) chain. No cache.
    ///
    /// `powi(z, 0) = 1`; a negative exponent computes `(1/z)^|n|` directly, so the
    /// sign-dependent overflow/underflow propagates from the closure with `?`. Repeated squaring
    /// compounds the relative error (it roughly doubles per step), so after `bit_len(n)` squarings
    /// the per-part error is bounded by about `2^nlen · ulp`, which the radius reflects; complex
    /// `sqr`/`mul` are near-correct (a few ulp, not 0.5), so the bound carries an extra margin.
    pub fn powi<const B: Word>(&self, z: &CBig<R, B>, exp: IBig) -> CfpResult<R, B> {
        let (sign, n) = exp.into_parts();
        if n.is_zero() {
            return Ok(Exact(CBig::ONE));
        }
        let negative = sign == Sign::Negative;
        if n.is_one() {
            // |n| == 1: z (positive) or 1/z (negative), a single op.
            return if negative {
                self.inv(z)
            } else {
                Ok(Exact(z.clone()))
            };
        }

        let p = self.precision();
        let nlen = n.bit_len();
        // Initial guard scales with `nlen` (the squaring-compounding loss) plus a margin for the
        // near-correct complex `sqr`/`mul`; sized so the first attempt certifies a non-tie result.
        let initial_guard = nlen + 6;
        let [re, im] = self.ziv(initial_guard, |guard| {
            let pw = p + guard;
            let gctx = Context::new(pw);
            // start from z (positive exponent, always exact) or its working-precision reciprocal
            // (negative exponent, exact only when 1/z is exactly representable).
            let (start, mut exact) = if negative {
                let inv = gctx.inv(z)?;
                let ex = matches!(inv, Exact(_));
                (inv.value(), ex)
            } else {
                (z.clone(), true)
            };
            // left-to-right binary exponentiation, tracking whether every step rounded Exact.
            let mut acc = start.clone();
            for i in (0..nlen - 1).rev() {
                let s = gctx.sqr(&acc)?;
                exact = exact && matches!(s, Exact(_));
                acc = s.value();
                if n.bit(i) {
                    let m = gctx.mul(&acc, &start)?;
                    exact = exact && matches!(m, Exact(_));
                    acc = m.value();
                }
            }
            let (re, im) = acc.into_parts();
            // re-root to the working precision (parts may be exact constants).
            let re = re.with_precision(pw).value();
            let im = im.with_precision(pw).value();
            let shift = (nlen + 3) as isize;
            // When the whole chain is exact the result is the mathematically exact zⁿ: report a zero
            // radius. This is required under the directed rounding modes (the `CBig` default), where
            // an exactly-representable result lies on a one-sided rounding boundary that no nonzero
            // radius can fit inside. (See `dashu-float`'s `powi` for the same reasoning.)
            let re_r = if exact { FBig::ZERO } else { re.ulp() << shift };
            let im_r = if exact { FBig::ZERO } else { im.ulp() << shift };
            Ok([(re.clone(), re_r), (im.clone(), im_r)])
        })?;
        Ok(combine_parts(re, im))
    }

    /// Complex exponential under this context (context layer). Computes `e^x·(cos y + i·sin y)`
    /// from `dashu-float`'s (correctly-rounded) `exp` and `sin_cos`, wrapped in a Ziv loop that
    /// certifies both parts; the cache is threaded into both (the convenience layer passes `None`).
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
            if z.im().is_infinite() {
                return Err(FpError::Indeterminate); // cos/sin(±inf) undefined
            }
            return if z.re().sign() == Sign::Positive {
                Ok(riemann(*self))
            } else {
                Ok(exact(FBig::ZERO, FBig::ZERO))
            };
        }

        // `e^x·(cos y + i·sin y)`. The float `exp`/`sin_cos` are correctly-rounded at the working
        // precision, so each contributes ~0 to the composition radius; only the two products round,
        // at a few working-ULPs. The Ziv driver asserts a limited context (the special-value
        // shortcuts above are exact and need no precision); overflow from a large real part
        // propagates from the closure with `?`.
        let p = self.precision();
        let [re, im] = self.ziv(EXP_GUARD, |guard| {
            let gctx = FloatCtxt::<R>::new(p + guard);
            let ex = gctx.exp(z.re(), reborrow_cache(&mut cache))?.value();
            let (sin_y, cos_y) = gctx.sin_cos(z.im(), reborrow_cache(&mut cache));
            let cos_y = cos_y?.value();
            let sin_y = sin_y?.value();
            let re = gctx.mul(ex.repr(), cos_y.repr())?.value();
            let im = gctx.mul(ex.repr(), sin_y.repr())?.value();
            Ok([(re.clone(), re.ulp() * 6), (im.clone(), im.ulp() * 6)])
        })?;
        Ok(combine_parts(re, im))
    }

    /// Raise `base` to a complex power under this context (context layer): `exp(w·log base)` on the
    /// principal branch, correctly rounded via a Ziv loop. `powf(0, 0) = 1` (matching `FBig::powf`).
    ///
    /// The result's error is amplified by the exponent magnitude `‖w·log base‖`: the outer `exp`
    /// multiplies the error in `w·log base` by the result magnitude, so the per-part radius carries
    /// a data-dependent `‖w·log base‖` factor (mirroring `FBig::powf`). Overflow (a large exponent)
    /// propagates from the closure with `?`.
    pub fn powf<const B: Word>(
        &self,
        base: &CBig<R, B>,
        w: &CBig<R, B>,
        mut cache: Option<&mut ConstCache>,
    ) -> CfpResult<R, B> {
        if w.is_zero() {
            return Ok(Exact(CBig::ONE)); // powf(z, 0) = 1, incl. powf(0, 0)
        }
        let p = self.precision();
        let [re, im] = self.ziv(POWF_GUARD, |guard| {
            let pw = p + guard;
            let gctx = Context::new(pw);
            let log_z = gctx.log(base, reborrow_cache(&mut cache))?.value();
            let wlogz = gctx.mul(w, &log_z)?.value();
            let hi = gctx.exp(&wlogz, reborrow_cache(&mut cache))?.value();
            // The outer `exp` amplifies the error in `w·log base` by the result magnitude, so the
            // radius scales with `‖w·log base‖`. `re()`/`im()` are raw `Repr`s — wrap to take the
            // absolute value; the L1 norm `|re|+|im|` upper-bounds the magnitude.
            let fctx = gctx.float();
            let l1 = FBig::<R, B>::from_repr(wlogz.re().clone(), fctx).abs()
                + FBig::<R, B>::from_repr(wlogz.im().clone(), fctx).abs();
            let amp = (l1 + FBig::<R, B>::ONE) * 16i32;
            let (re, im) = hi.into_parts();
            // re-root to the working precision (`exp`/`log` may return exact constants).
            let re = re.with_precision(pw).value();
            let im = im.with_precision(pw).value();
            Ok([(re.clone(), re.ulp() * &amp), (im.clone(), im.ulp() * &amp)])
        })?;
        Ok(combine_parts(re, im))
    }
}

impl<R: ErrorBounds, const B: Word> CBig<R, B> {
    /// Integer power (convenience layer).
    ///
    /// # Panics
    ///
    /// Panics on an indeterminate / out-of-domain result (e.g. `0⁻¹`).
    #[inline]
    pub fn powi(&self, exp: IBig) -> Self {
        self.context().unwrap_cfp(self.context().powi(self, exp))
    }

    /// Complex exponential `e^z` (convenience layer).
    ///
    /// # Panics
    ///
    /// Panics if the precision is unlimited or on an indeterminate special value.
    #[inline]
    pub fn exp(&self) -> Self {
        self.context().unwrap_cfp(self.context().exp(self, None))
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
        // exp(1+0i) = e ≈ 2.71828…; check 2 < e < 3 via the real part. Use a *limited*-precision
        // input — `exp` rejects unlimited precision (it would otherwise silently compute at the
        // fixed `EXP_GUARD`).
        let e = c(1, 0).exp();
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
        assert!(r.im().is_pos_zero());
    }

    #[test]
    fn exp_huge_real_overflows() {
        // exp of a huge real part overflows the isize exponent range. The error propagates from the
        // Ziv closure via `?` (no hoisted probe), and the convenience layer saturates it to +∞.
        let huge = F::from_parts(IBig::from(1) << 100, 0)
            .with_precision(53)
            .value();
        let z = CBig::from_parts(huge, F::from(0).with_precision(53).value());
        let e = z.exp();
        assert!(e.re().is_infinite());
        assert_eq!(e.re().sign(), Sign::Positive);
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

    // exp/powf on an unlimited-precision CBig must panic, not silently compute at the fixed guard
    // precision (`C::ONE` / `C::ZERO` are unlimited-precision constants).
    #[test]
    #[should_panic(expected = "precision cannot be 0")]
    fn complex_exp_unlimited_precision_panics() {
        let _ = C::ONE.exp();
    }

    #[test]
    #[should_panic(expected = "precision cannot be 0")]
    fn complex_powf_unlimited_precision_panics() {
        let _ = C::ONE.powf(&C::ONE);
    }
}
