//! Complex exponential and powers.
//!
//! * [`Context::exp`] / [`CBig::exp`]: `exp(x+iy) = e^x·(cos y + i sin y)`.
//! * [`Context::powi`] / [`CBig::powi`]: integer exponent via repeated squaring (branch-cut-free,
//!   cheaper than `exp(n·log z)`).
//! * [`Context::powf`] / [`CBig::powf`]: `exp(w·log z)` on the principal branch.
//!
//! Mirroring `dashu-float`, the power family lives alongside `exp` in a single module.

use crate::ball::CBall;
use crate::cbig::CBig;
use crate::repr::{combine_parts, exact, reborrow_cache, riemann, CfpResult, Context};
use dashu_base::Approximation::*;
use dashu_base::{BitTest, Sign};
use dashu_float::round::ErrorBounds;
use dashu_float::{Ball, Repr};
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
            // z⁰ = 1 is exact at every precision, so the unlimited-precision constant is the
            // correctly rounded result under every mode
            return Ok(Exact(CBig::ONE));
        }
        let negative = sign == Sign::Negative;
        if z.is_infinite() {
            // |n| >= 2: an infinite base can't be raised to a higher power (the terminal-infinity
            // model rejects infinite operands); report Indeterminate like the other complex
            // transcendentals rather than leaking the float layer's InfiniteInput.
            return Err(FpError::Indeterminate);
        }

        let p = self.precision();
        let nlen = n.bit_len();

        // Diagonal shortcut: z = t·(1+i) with *identical exact parts* ⇒ `zⁿ = tⁿ·(1+i)ⁿ`, where
        // `(1+i)ⁿ` sits on the exact half-power-of-two lattice (magnitude `2^(n/2)` in steps of
        // √2, arguments in multiples of π/4), so both components are the one certified real
        // power `tⁿ` scaled by an exact power of two — and the zero component of an even power
        // is *exactly* zero. The generic squaring chain cannot certify that zero: its `a² − b²`
        // cancels onto exact 0 at every working precision (the parts are the same value), but
        // the tracked radius stays a division-rounding ε > 0, which the strict zero-candidate
        // certification (no nonzero real rounds to 0) correctly refuses — an endless retry. The
        // near-diagonal z (parts *rounding* together at low precision but distinct) deliberately
        // does not enter here: their true zero component is nonzero, and the retry that
        // separates the squares is the certification working.
        if z.re() == z.im() && !z.re().significand().is_zero() {
            let [re, im] = self.ziv(nlen + 6, |guard| {
                let pw = p + guard;
                let fctx = FloatCtxt::<R>::new(pw);
                let exp: IBig = if negative {
                    -IBig::from(n.clone())
                } else {
                    IBig::from(n.clone())
                };
                // `tⁿ`, certified by the real Ziv loop under this mode (negative `n` included)
                let tp =
                    Ball::from_rounded(fctx.powi(z.re(), exp.clone())?.map(FBig::into_repr), pw);
                // `(1+i)ⁿ = 2^((n−r)/2)·(±1, ±1)` on the exact lattice: `r = n mod 2`, and the
                // signs follow the angle `n·π/4` — `q = n mod 8` (floor) indexes the table.
                let eight = IBig::from(8);
                let q = ((&exp % &eight) + &eight) % &eight;
                let r = &q % 2u32;
                let k = (exp.clone() - r) / 2u32;
                let lattice: [(i32, i32); 8] = [
                    (1, 0),
                    (1, 1),
                    (0, 1),
                    (-1, 1),
                    (-1, 0),
                    (-1, -1),
                    (0, -1),
                    (1, -1),
                ];
                let (a, b) = lattice[usize::try_from(q).unwrap()];
                let k: isize = match isize::try_from(k) {
                    Ok(k) => k,
                    // a lattice exponent past the representable range only occurs when `tⁿ`
                    // itself cannot produce a finite certified value
                    Err(_) => return Err(FpError::Overflow(Sign::Positive)),
                };
                let part = |v: i32, k: isize| -> Result<Ball<B>, FpError> {
                    if v == 0 {
                        return Ok(Ball::exact(Repr::<B>::zero()));
                    }
                    // the lattice factor is exact: its product with `tⁿ` costs one rounding
                    let s = Ball::exact(
                        FBig::<R, B>::from_parts(IBig::from(v), k)
                            .with_precision(pw)
                            .value()
                            .into_repr(),
                    );
                    s.mul(&tp, pw)
                };
                Ok([
                    part(a, k)?.to_value_radius::<R>(&fctx),
                    part(b, k)?.to_value_radius::<R>(&fctx),
                ])
            })?;
            return Ok(combine_parts(re, im));
        }

        // Initial guard scales with `nlen` (each squaring roughly doubles the relative error,
        // which the ball product rule tracks); sized so the first attempt certifies a non-tie
        // result.
        let initial_guard = nlen + 6;
        let [re, im] = self.ziv(initial_guard, |guard| {
            let pw = p + guard;
            let fctx = FloatCtxt::<R>::new(pw);
            // start from z (positive exponent, always exact) or its ball reciprocal (negative
            // exponent, exact only when 1/z is exactly representable — `rad == 0` knows).
            let entry = CBall::from_parts(z.re(), z.im(), pw);
            let start = if negative { entry.inv(pw)? } else { entry };
            // left-to-right binary exponentiation; every product's compounding rounding is
            // tracked, and an all-exact chain keeps `rad == 0` on both components — which is
            // what lets the directed rounding modes certify the exactly-representable zⁿ (the
            // hand-written radius carried the same special case as an `Exact`-flag counter).
            let mut acc = start.clone();
            for i in (0..nlen - 1).rev() {
                acc = acc.sqr(pw)?;
                if n.bit(i) {
                    acc = acc.mul(&start, pw)?;
                }
            }
            Ok(acc.to_parts_radius(&fctx))
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

        // `e^x·(cos y + i·sin y)` through the ball composition (`CBall::exp`): the float
        // `exp`/`sin_cos` kernels run on the midpoints and the input radii (zero here — the
        // entry is exact) propagate mechanically through the two tracked products. The Ziv
        // driver asserts a limited context (the special-value shortcuts above are exact and
        // need no precision); overflow from a large real part propagates from the closure
        // with `?`.
        let p = self.precision();
        let [re, im] = self.ziv(EXP_GUARD, |guard| {
            let pw = p + guard;
            let gctx = FloatCtxt::<R>::new(pw);
            let out =
                CBall::from_parts(z.re(), z.im(), pw).exp(&gctx, pw, reborrow_cache(&mut cache))?;
            Ok(out.to_parts_radius(&gctx))
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
        // Through the ball compositions: `log` folds its kernels' input radii, the complex
        // product is tracked by the ball rule, and `exp`'s fold amplifies the radius of
        // `w·log base` by the result magnitude — the old hand-written `(|w·log z|₁+1)·16`
        // L1 factor, paid unconditionally on both parts, is replaced by the per-component,
        // relative-condition-correct mechanical fold.
        let p = self.precision();
        let [re, im] = self.ziv(POWF_GUARD, |guard| {
            let pw = p + guard;
            let fctx = FloatCtxt::<R>::new(pw);
            let lz = CBall::from_parts(base.re(), base.im(), pw).log(
                &fctx,
                pw,
                reborrow_cache(&mut cache),
            )?;
            let wlogz = CBall::from_parts(w.re(), w.im(), pw).mul(&lz, pw)?;
            let hi = wlogz.exp(&fctx, pw, reborrow_cache(&mut cache))?;
            Ok(hi.to_parts_radius(&fctx))
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
    use dashu_float::FBig;

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
    fn powi_infinite_base_rejects() {
        let inf = CBig::from(F::INFINITY);
        let ctx = Context::new(53);
        // |n| >= 2 with an infinite base → Indeterminate (not the float layer's InfiniteInput).
        assert_eq!(ctx.powi(&inf, 2.into()), Err(FpError::Indeterminate));
        assert_eq!(ctx.powi(&inf, (-2).into()), Err(FpError::Indeterminate));
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

    // The mechanically tracked radius must certify at the target precision across the width
    // sweep: each result equals the same op computed at `p + 60` and re-rounded to `p` (both
    // sides are correctly rounded on the same exact integer input, so they agree bit for bit).
    #[test]
    fn exp_matches_oracle_across_precisions() {
        type C2 = CBig<mode::HalfEven, 2>;
        type F2 = FBig<mode::HalfEven, 2>;
        let inputs = [(1i64, 0i64), (0, 2), (-2, 1), (3, -4), (0, -5), (-4, 0)];
        for p in [20usize, 50, 100, 500] {
            for (re, im) in inputs {
                let mk = |v: i64| F2::from(v).with_precision(p).value();
                let mk_hi = |v: i64| F2::from(v).with_precision(p + 60).value();
                let (hre, him) = C2::from_parts(mk_hi(re), mk_hi(im)).exp().into_parts();
                let expect_re = hre.with_precision(p).value();
                let expect_im = him.with_precision(p).value();
                let got = C2::from_parts(mk(re), mk(im)).exp();
                assert_eq!(got.re(), expect_re.repr(), "re p={p} z=({re},{im})");
                assert_eq!(got.im(), expect_im.repr(), "im p={p} z=({re},{im})");
            }
        }
    }

    // An exactly-representable zⁿ certifies under the outward modes through the chain's zero
    // radius — (3+4i)² = −7+24i and (3+4i)⁻² = (−7−24i)/625 are exact rationals, and the
    // one-sided preimages of exact results admit no nonzero radius.
    #[test]
    fn powi_exact_chain_certifies_directed() {
        macro_rules! check {
            ($mode:ty) => {{
                type C = CBig<$mode, 10>;
                type F = FBig<$mode, 10>;
                let mk = |v: i32| F::from(v).with_precision(30).value();
                let ctx = Context::<$mode>::new(30);
                let z = C::from_parts(mk(3), mk(4));
                let got = ctx.powi(&z, 2.into()).unwrap().value().clone();
                assert_eq!(got.re(), mk(-7).repr());
                assert_eq!(got.im(), mk(24).repr());
                let got = ctx.powi(&z, (-2).into()).unwrap().value().clone();
                // (3+4i)⁻² = (−7 − 24i)/625 = −0.0112 − 0.0384i: the reciprocal stays exact
                // because 25 divides 10², so the whole chain is exact at this precision
                let q = |v: i32| F::from_parts(IBig::from(v), -4).with_precision(30).value();
                assert_eq!(got.re(), q(-112).repr()); // −7/625 = −0.0112
                assert_eq!(got.im(), q(-384).repr()); // −24/625 = −0.0384
            }};
        }
        check!(mode::Up);
        check!(mode::Down);
        check!(mode::Zero);
        check!(mode::HalfEven);
    }

    // A z whose parts round onto the diagonal at the work precision (`a² − b²` collapses onto
    // exact 0 there) used to certify `re = 0` through the wide ±ulp preimage of ±0 — while the
    // true value (`7.6e-22` here) rounds to a *nonzero* 20-bit float, ~2⁶⁹ ulps from zero. The
    // strict zero-candidate guard forces the retry that separates the squares, and the result
    // then matches the high-precision oracle. (Found by the directed `cbig_powi_fuzz`; its
    // proptest shrink loop was also the mysterious multi-hour `cbig_powi` shard "tail".)
    #[test]
    fn powi_diagonal_collapse_certifies_nonzero() {
        type C = CBig<mode::Zero, 2>;
        let mk = |v: f64| FBig::<mode::Zero, 2>::try_from(v).unwrap();
        let z = C::from_parts(mk(-2.826), mk(-2.8259999999999996));
        let ctx = Context::<mode::Zero>::new(20);
        let got = ctx.powi(&z, (-10).into()).unwrap().value().clone();

        // oracle: the same op nearest at p + 60, both sides re-rounded to p (the rung under
        // `mode::Zero` so the comparison rounding matches the mode under test)
        let z_hi = CBig::<mode::HalfEven, 2>::from_parts(
            FBig::<mode::HalfEven, 2>::try_from(-2.826).unwrap(),
            FBig::<mode::HalfEven, 2>::try_from(-2.8259999999999996).unwrap(),
        );
        let (hi_re, hi_im) = Context::<mode::HalfEven>::new(80)
            .powi(&z_hi, (-10).into())
            .unwrap()
            .value()
            .clone()
            .into_parts();
        let want_re = hi_re
            .with_rounding::<mode::Zero>()
            .with_precision(20)
            .value();
        let want_im = hi_im
            .with_rounding::<mode::Zero>()
            .with_precision(20)
            .value();

        assert!(
            !got.re().significand().is_zero(),
            "diagonal-collapse re certified as exact zero; true value is ~7.6e-22"
        );
        assert_eq!(got.re(), want_re.repr(), "re mismatch vs oracle");
        assert_eq!(got.im(), want_im.repr(), "im mismatch vs oracle");
    }

    // The ball-tracked radius must certify at the target precision across the width sweep.
    #[test]
    fn powi_matches_oracle_across_precisions() {
        type C2 = CBig<mode::HalfEven, 2>;
        type F2 = FBig<mode::HalfEven, 2>;
        let cases = [(3i64, 4i64, 5i32), (1, -1, 13), (2, 1, -7), (3, 2, 11)];
        for p in [20usize, 50, 100, 500] {
            for (re, im, n) in cases {
                let mk = |v: i64| F2::from(v).with_precision(p).value();
                let mk_hi = |v: i64| F2::from(v).with_precision(p + 60).value();
                let (hre, him) = C2::from_parts(mk_hi(re), mk_hi(im))
                    .powi(n.into())
                    .into_parts();
                let expect_re = hre.with_precision(p).value();
                let expect_im = him.with_precision(p).value();
                let got = C2::from_parts(mk(re), mk(im)).powi(n.into());
                assert_eq!(got.re(), expect_re.repr(), "re p={p} z=({re},{im})^{n}");
                assert_eq!(got.im(), expect_im.repr(), "im p={p} z=({re},{im})^{n}");
            }
        }
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
