//! Complex trigonometric functions via the real–imaginary decomposition, reusing `dashu-float`'s
//! real `sin`/`cos` and cancellation-free `sinh`/`cosh` — plus their ×π variants on the real
//! `sin_cos_pi`/`sinh_cosh_pi`.
//!
//! `sin(x+iy) = sin x·cosh y + i·cos x·sinh y`, `cos(x+iy) = cos x·cosh y − i·sin x·sinh y`. This
//! form avoids the `exp(±iz)` identity's exponential blow-up for large `|Im z|`.

use crate::ball::CBall;
use crate::cbig::CBig;
use crate::repr::{combine_parts, reborrow_cache, CfpResult, Context};
use dashu_base::Approximation;
use dashu_float::round::ErrorBounds;
use dashu_float::{Ball, ConstCache, Context as FloatCtxt, FBig, FpError, Repr};
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
            let cos_im = re.sign() * im.sign(); // Annex G: negative iff the signs differ
            let cos = crate::repr::exact(
                FBig::from_repr(Repr::one(), self.float()),
                FBig::from_repr(Repr::zero_with_sign(cos_im), self.float()),
            );
            return (Ok(sin), Ok(cos));
        }

        // `sin z = sinx·coshy + i·cosx·sinhy`, `cos z = cosx·coshy − i·sinx·sinhy`. The four
        // products share one evaluation of the real `sin_cos`/`sinh_cosh` (each correctly-rounded
        // at the working precision, folded to one work-ulp by `from_rounded`) and are tracked by
        // the ball product rule — the radius is mechanical, so the cancellation near the trig
        // zeros prices itself. A single 4-part Ziv loop certifies all of `sin` and `cos`
        // together; the entry is exact, so the input-error folds are all zero.
        let p = self.precision();
        let parts = self.ziv(TRIG_GUARD, |guard| {
            let pw = p + guard;
            let gctx = FloatCtxt::<R>::new(pw);
            let cb = CBall::from_parts(z.re(), z.im(), pw);
            let (sinx, cosx) = gctx.sin_cos(&cb.re.mid, reborrow_cache(&mut cache));
            let mut sx = Ball::from_rounded(sinx?.map(FBig::into_repr), pw);
            let mut cx = Ball::from_rounded(cosx?.map(FBig::into_repr), pw);
            // the kernels run on the midpoints: the seed rounding of an over-precise input
            // propagates (|Δsin|, |Δcos| ≤ |δx|; |Δsinh|, |Δcosh| ≤ 2·‖cosh ball‖·e^{δy}·|δy|)
            sx.add_error(cb.re.rad);
            cx.add_error(cb.re.rad);
            let (sinhy, coshy) = gctx.sinh_cosh(&cb.im.mid, reborrow_cache(&mut cache));
            let mut shy = Ball::from_rounded(sinhy?.map(FBig::into_repr), pw);
            let mut chy = Ball::from_rounded(coshy?.map(FBig::into_repr), pw);
            let y_fold = chy
                .mag()
                .mul(&cb.im.rad.exp_upper())
                .mul(&cb.im.rad)
                .mul_pow2(1);
            shy.add_error(y_fold);
            chy.add_error(y_fold);
            let sin_re = sx.mul(&chy, pw)?;
            let sin_im = cx.mul(&shy, pw)?;
            let cos_re = cx.mul(&chy, pw)?;
            let cos_im = -sx.mul(&shy, pw)?; // cos z's imaginary part is −sinx·sinhy
            Ok([
                sin_re.to_value_radius(&gctx),
                sin_im.to_value_radius(&gctx),
                cos_re.to_value_radius(&gctx),
                cos_im.to_value_radius(&gctx),
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
    /// term `≥ 1` (`cosh 2y`), so it never cancels to a *true* zero — unlike `sin z / cos z`, whose
    /// `sin·conj(cos)` real part cancels from `~cosh²y` down to `O(1)` for large `|Im z|`. The result
    /// is accurate for all finite `|Im z|`; the only small-denominator points are the real-axis poles
    /// (`y = 0, x = π/2 + kπ`), where the large value is genuine, not an artifact — near them the
    /// *computed* sum can still round onto an exact zero (a precision artifact), which retries at
    /// higher guard rather than dividing (see the collapsed-denominator guard below).
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

        // Through the tracked composition: exact doublings, the real `sin_cos`/`sinh_cosh`
        // kernels on the doubled midpoints (folded to one work-ulp each), the shared
        // denominator `cos 2x + cosh 2y` as a real ball, and one componentwise ball division —
        // the radius (including the genuine amplification near the real-axis poles, where the
        // denominator touches zero) is mechanical. The Ziv driver asserts a limited context.
        let p = self.precision();
        let [re, im] = self.ziv(TRIG_GUARD, |guard| {
            let pw = p + guard;
            let gctx = FloatCtxt::<R>::new(pw);
            let cb = CBall::from_parts(z.re(), z.im(), pw);
            // 2x, 2y (exact doublings — same significand, exponent +1; the entry is exact)
            let x2 = cb.re.add(&cb.re, pw)?;
            let y2 = cb.im.add(&cb.im, pw)?;
            let (sin2x, cos2x) = gctx.sin_cos(&x2.mid, reborrow_cache(&mut cache));
            let mut sx2 = Ball::from_rounded(sin2x?.map(FBig::into_repr), pw);
            let mut cx2 = Ball::from_rounded(cos2x?.map(FBig::into_repr), pw);
            sx2.add_error(x2.rad);
            cx2.add_error(x2.rad);
            let (sinh2y, cosh2y) = gctx.sinh_cosh(&y2.mid, reborrow_cache(&mut cache));
            let mut shy2 = Ball::from_rounded(sinh2y?.map(FBig::into_repr), pw);
            let mut chy2 = Ball::from_rounded(cosh2y?.map(FBig::into_repr), pw);
            let y2_fold = chy2.mag().mul(&y2.rad.exp_upper()).mul(&y2.rad).mul_pow2(1);
            shy2.add_error(y2_fold);
            chy2.add_error(y2_fold);
            // D = cos 2x + cosh 2y  (a benign sum: a bounded term plus one ≥ 1)
            let denom = cx2.add(&chy2, pw)?;
            let out = CBall { re: sx2, im: shy2 }.div_by_real(&denom, pw)?;
            Ok(out.to_parts_radius(&gctx))
        })?;
        Ok(combine_parts(re, im))
    }

    /// Simultaneously compute `sin(z·π)` and `cos(z·π)` (context layer), correctly rounded via
    /// a shared Ziv loop. An infinite input maps to [`FpError::Indeterminate`] (the C99 NaN
    /// cases).
    ///
    /// The real part reduces through the real [`sin_cos_pi`](dashu_float::Context::sin_cos_pi)
    /// (exact quarter-integer cases included — e.g. `sin_pi` of a half-integer real part is
    /// exactly ±1), the imaginary part through the hyperbolic
    /// [`sinh_cosh_pi`](dashu_float::Context::sinh_cosh_pi) on the pre-scaled argument.
    pub fn sin_cos_pi<const B: Word>(
        &self,
        z: &CBig<R, B>,
        mut cache: Option<&mut ConstCache>,
    ) -> (CfpResult<R, B>, CfpResult<R, B>) {
        if z.is_infinite() {
            return (Err(FpError::Indeterminate), Err(FpError::Indeterminate));
        }
        if z.is_zero() {
            // sin(x+iy)·π = sinx·coshy + i·cosx·sinhy: at ±0 the parts carry the input zeros'
            // signs (sin_pi(±0) = ±0, sinh(±0) = ±0, cos_pi(±0) = cosh(±0) = 1), so e.g.
            // `csin_pi(-0 + i·0) = -0 + i·0` — the same table as the radian `sin_cos`.
            let (re, im) = (z.re(), z.im());
            let sin = crate::repr::exact(
                FBig::from_repr(Repr::zero_with_sign(re.sign()), self.float()),
                FBig::from_repr(Repr::zero_with_sign(im.sign()), self.float()),
            );
            // cos(x+iy)·π = cosx·coshy − i·sinx·sinhy: real = 1; the imaginary part is the
            // signed product `x·y` — the Annex-G table value (see the radian `sin_cos`).
            let cos_im = re.sign() * im.sign(); // Annex G: negative iff the signs differ
            let cos = crate::repr::exact(
                FBig::from_repr(Repr::one(), self.float()),
                FBig::from_repr(Repr::zero_with_sign(cos_im), self.float()),
            );
            return (Ok(sin), Ok(cos));
        }

        // `sin zπ = sin_pi(x)·cosh(πy) + i·cos_pi(x)·sinh(πy)`,
        // `cos zπ = cos_pi(x)·cosh(πy) − i·sin_pi(x)·sinh(πy)`. Each factor is correctly
        // rounded at the working precision (contributing ~½ ulp each), so only the products
        // round — a few working-ULPs, like the radian `sin_cos`. The π-scaling lives inside the
        // real ×π kernels (their argument balls carry π's radius), NOT in a pre-multiplied
        // `π·y` (whose ½-ulp error the hyperbolic derivative would amplify to ~2π|y| ulps).
        // Like the radian `sin_cos`: the four products are ball-tracked, the π-scaling lives
        // inside the real ×π kernels (their argument balls carry π's radius), and the entry is
        // exact — so only the kernel work-ulps and the products price the radius.
        let p = self.precision();
        let parts = self.ziv(TRIG_GUARD, |guard| {
            let pw = p + guard;
            let gctx = FloatCtxt::<R>::new(pw);
            let cb = CBall::from_parts(z.re(), z.im(), pw);
            let (sinx, cosx) = gctx.sin_cos_pi(&cb.re.mid, reborrow_cache(&mut cache));
            let mut sx = Ball::from_rounded(sinx?.map(FBig::into_repr), pw);
            let mut cx = Ball::from_rounded(cosx?.map(FBig::into_repr), pw);
            sx.add_error(cb.re.rad);
            cx.add_error(cb.re.rad);
            let (sinhy, coshy) = gctx.sinh_cosh_pi(&cb.im.mid, reborrow_cache(&mut cache));
            let mut shy = Ball::from_rounded(sinhy?.map(FBig::into_repr), pw);
            let mut chy = Ball::from_rounded(coshy?.map(FBig::into_repr), pw);
            let y_fold = chy
                .mag()
                .mul(&cb.im.rad.exp_upper())
                .mul(&cb.im.rad)
                .mul_pow2(1);
            shy.add_error(y_fold);
            chy.add_error(y_fold);
            let sin_re = sx.mul(&chy, pw)?;
            let sin_im = cx.mul(&shy, pw)?;
            let cos_re = cx.mul(&chy, pw)?;
            let cos_im = -sx.mul(&shy, pw)?; // cos zπ's imaginary part is −sin_pi(x)·sinh(πy)
            Ok([
                sin_re.to_value_radius(&gctx),
                sin_im.to_value_radius(&gctx),
                cos_re.to_value_radius(&gctx),
                cos_im.to_value_radius(&gctx),
            ])
        });
        let [sin_re, sin_im, cos_re, cos_im] = match parts {
            Ok(arr) => arr,
            // an overflow (e.g. `cosh` of a huge imaginary part) fails both sin and cos together.
            Err(e) => return (Err(e), Err(e)),
        };
        (Ok(combine_parts(sin_re, sin_im)), Ok(combine_parts(cos_re, cos_im)))
    }

    /// Complex sine of `z·π` (context layer).
    #[inline]
    pub fn sin_pi<const B: Word>(
        &self,
        z: &CBig<R, B>,
        cache: Option<&mut ConstCache>,
    ) -> CfpResult<R, B> {
        self.sin_cos_pi(z, cache).0
    }

    /// Complex cosine of `z·π` (context layer).
    #[inline]
    pub fn cos_pi<const B: Word>(
        &self,
        z: &CBig<R, B>,
        cache: Option<&mut ConstCache>,
    ) -> CfpResult<R, B> {
        self.sin_cos_pi(z, cache).1
    }

    /// Complex tangent of `z·π` (context layer), correctly rounded via a Ziv loop, using the
    /// cancellation-free double-angle identity
    ///
    /// `tan(z·π) = (sin_pi(2x) + i·sinh(2πy)) / (cos_pi(2x) + cosh(2πy))`.
    ///
    /// As for the radian [`tan`](Self::tan), the denominator is a sum of a bounded term
    /// (`cos_pi(2x) ∈ [−1, 1]`) and a term `≥ 1` (`cosh(2πy)`) — for `y ≠ 0` the true value
    /// is `≥ cosh(2πy) − 1 > 0`, so it never cancels to zero exactly. Near the real-axis
    /// poles, though, both terms round to `∓1` and the *computed* sum collapses onto zero;
    /// a purely real argument bypasses that via the real [`tan_pi`](dashu_float::Context::tan_pi)
    /// kernel (whose pole guard certifies the huge near-pole values, and whose exact poles
    /// return [`FpError::Indeterminate`] — the same `0/0` convention), and a nonzero tiny `y`
    /// retries until the (always positive) true difference re-emerges at higher guard.
    pub fn tan_pi<const B: Word>(
        &self,
        z: &CBig<R, B>,
        mut cache: Option<&mut ConstCache>,
    ) -> CfpResult<R, B> {
        if z.is_infinite() {
            return Err(FpError::Indeterminate);
        }
        if z.is_zero() {
            let (re, im) = (z.re(), z.im());
            // tan is odd in both parts: the parts carry the input zeros' signs (bypasses the
            // Ziv loop, which rejects unlimited precision).
            return Ok(crate::repr::exact(
                FBig::from_repr(Repr::zero_with_sign(re.sign()), self.float()),
                FBig::from_repr(Repr::zero_with_sign(im.sign()), self.float()),
            ));
        }
        if z.im().significand().is_zero() {
            // A purely real argument reduces exactly to the real ×π kernel — the double-angle
            // denominator would cancel `cos_pi(2x)` against `cosh(0) = 1` down to (and past)
            // zero near the poles, while the real kernel's own guard handles them. The
            // imaginary part is `sinh(±0)/D` with `D = cos_pi(2x) + 1 ≥ 0`: ±0 with the sign
            // of `y`.
            return FloatCtxt::<R>::new(self.precision())
                .tan_pi::<B>(z.re(), reborrow_cache(&mut cache))
                .map(|t| {
                    crate::repr::exact(
                        t.value(),
                        FBig::from_repr(Repr::zero_with_sign(z.im().sign()), self.float()),
                    )
                });
        }

        // Like the radian `tan`: exact doublings, the ×π kernels on the doubled midpoints
        // (with their input folds), the shared denominator as a real ball, and one tracked
        // componentwise division. An exact pole (`y = 0`, x an odd multiple of `1/2`, where
        // `cos_pi(2x)` is exactly −1) fails the midpoint division first — `0/0` maps to
        // `Err(Indeterminate)` before any radius work.
        let p = self.precision();
        let [re, im] = self.ziv(TRIG_GUARD, |guard| {
            let pw = p + guard;
            let gctx = FloatCtxt::<R>::new(pw);
            let cb = CBall::from_parts(z.re(), z.im(), pw);
            let x2 = cb.re.add(&cb.re, pw)?;
            let y2 = cb.im.add(&cb.im, pw)?;
            let (sin2x, cos2x) = gctx.sin_cos_pi(&x2.mid, reborrow_cache(&mut cache));
            let mut sx2 = Ball::from_rounded(sin2x?.map(FBig::into_repr), pw);
            let mut cx2 = Ball::from_rounded(cos2x?.map(FBig::into_repr), pw);
            sx2.add_error(x2.rad);
            cx2.add_error(x2.rad);
            let (sinh2y, cosh2y) = gctx.sinh_cosh_pi(&y2.mid, reborrow_cache(&mut cache));
            let mut shy2 = Ball::from_rounded(sinh2y?.map(FBig::into_repr), pw);
            let mut chy2 = Ball::from_rounded(cosh2y?.map(FBig::into_repr), pw);
            let y2_fold = chy2.mag().mul(&y2.rad.exp_upper()).mul(&y2.rad).mul_pow2(1);
            shy2.add_error(y2_fold);
            chy2.add_error(y2_fold);
            // D = cos_pi(2x) + cosh(2πy)  (a benign sum: a bounded term plus one ≥ 1)
            let denom = cx2.add(&chy2, pw)?;
            let out = CBall { re: sx2, im: shy2 }.div_by_real(&denom, pw)?;
            Ok(out.to_parts_radius(&gctx))
        })?;
        Ok(combine_parts(re, im))
    }

    /// Inverse sine `asin z = -i·log(iz + sqrt(1-z²))` (context layer, Kahan form), correctly
    /// rounded via a Ziv loop. The argument of the inner `log` always has positive real part, so the
    /// branch cut comes entirely from the `sqrt`; an infinite input maps to
    /// [`FpError::Indeterminate`]. The `1-z²` under the `sqrt` is computed in the factored form
    /// `(1-z)(1+z)`, which is Sterbenz-exact near `z = ±1` (where the direct `1-z²` would
    /// catastrophically cancel against the `sqr` rounding error), so the well-conditioned regime
    /// extends right up to the singularities — and because the composition is ball-tracked, the
    /// radius grows mechanically as `1-z² → 0` (the `sqrt` fold divides by the shrinking root and
    /// the `log` fold by the shrinking `‖w‖`), so the Ziv retries price themselves instead of
    /// relying on a blanket constant.
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
            let fctx = FloatCtxt::<R>::new(pw);
            let cz = CBall::from_parts(z.re(), z.im(), pw);
            let one = CBall::exact(Repr::one(), Repr::zero());
            // Factor `1-z² = (1-z)(1+z)`. Near `z = ±1` the direct `1 - z²` subtracts a value
            // dominated by the `sqr` rounding error from 1 (catastrophic cancellation); the factored
            // form is Sterbenz-exact there (`1-z` is computed exactly, since the subtraction's
            // significand difference is exact), so the radius stays sound right up to the singularity.
            let one_m_z = one.sub(&cz, pw)?;
            let one_p_z = one.add(&cz, pw)?;
            let sqrt_term = one_m_z.mul(&one_p_z, pw)?.sqrt(&fctx, pw)?;
            let w = cz.mul_i(false).add(&sqrt_term, pw)?; // i·z + sqrt(1-z²)
            let log_w = w.log(&fctx, pw, reborrow_cache(&mut cache))?;
            Ok(log_w.mul_i(true).to_parts_radius(&fctx)) // -i·log(w)
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
            let fctx = FloatCtxt::<R>::new(pw);
            let cz = CBall::from_parts(z.re(), z.im(), pw);
            let one = CBall::exact(Repr::one(), Repr::zero());
            // Factored `1-z² = (1-z)(1+z)` — Sterbenz-exact near `z = ±1` (see `asin`).
            let one_m_z = one.sub(&cz, pw)?;
            let one_p_z = one.add(&cz, pw)?;
            let sqrt_term = one_m_z.mul(&one_p_z, pw)?.sqrt(&fctx, pw)?;
            let w = cz.add(&sqrt_term.mul_i(false), pw)?; // z + i·sqrt(1-z²)
            let log_w = w.log(&fctx, pw, reborrow_cache(&mut cache))?;
            Ok(log_w.mul_i(true).to_parts_radius(&fctx)) // -i·log(w)
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
        // Axis dispatch (Annex-G): on the real axis the result is real (`atan(x) ± i·0`), and
        // on the imaginary axis with `|y| < 1` it is `±0 + i·atanh(y)` — in both cases one
        // component is *exactly* zero, which the two-log difference can never certify (its
        // seed-rounding radius stays nonzero while the mids cancel to an exact 0, and no
        // containment can ever fit a nonzero radius around an exact zero). The real kernels
        // are correctly rounded and the zeros are exact, so these paths certify under every
        // mode; off-axis inputs, and `|y| ≥ 1` on the imaginary axis (where the real part is
        // the inexact `±π/2`), take the general path.
        let (re_in, im_in) = (z.re(), z.im());
        let f = self.float();
        if im_in.significand().is_zero() {
            // atan(x ± i·0) = atan(x) ± i·0
            let at = f.atan(re_in, reborrow_cache(&mut cache))?;
            let im_zero =
                Approximation::Exact(FBig::from_repr(Repr::zero_with_sign(im_in.sign()), f));
            return Ok(combine_parts(at, im_zero));
        }
        if re_in.significand().is_zero() {
            // atan(±0 + i·y) = ±0 + i·atanh(y) for |y| < 1; `atanh` rejects |y| ≥ 1 (where the
            // general path's divergence — `atan(±i)` is indeterminate — is the honest answer)
            if let Ok(ath) = f.atanh(im_in, reborrow_cache(&mut cache)) {
                let re_zero =
                    Approximation::Exact(FBig::from_repr(Repr::zero_with_sign(re_in.sign()), f));
                return Ok(combine_parts(re_zero, ath));
            }
        }
        let p = self.precision();
        let [re, im] = self.ziv(ITRIG_GUARD, |guard| {
            let pw = p + guard;
            let fctx = FloatCtxt::<R>::new(pw);
            let cz = CBall::from_parts(z.re(), z.im(), pw);
            let one = CBall::exact(Repr::one(), Repr::zero());
            let iz = cz.mul_i(false); // exact rotation
            let a = one.sub(&iz, pw)?; // 1 - iz
            let b = one.add(&iz, pw)?; // 1 + iz
            let log_a = a.log(&fctx, pw, reborrow_cache(&mut cache))?;
            let log_b = b.log(&fctx, pw, reborrow_cache(&mut cache))?;
            // the near-cancellation of the two logs for small `z` is *tracked* (the difference's
            // radius is the sum), not absorbed by a blanket constant
            let diff = log_a.sub(&log_b, pw)?;
            let two = Ball::exact(Repr::new(IBig::from(2u8), 0));
            let out = diff.mul_i(false).div_by_real(&two, pw)?; // i·diff / 2
            Ok(out.to_parts_radius(&fctx))
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

    /// Complex sine of `z·π` (convenience layer). Panics on an indeterminate special value.
    ///
    /// On the real axis the ×π kernels' exact lattice resolves exactly: `sin_pi(1/2) = 1`.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashu_cmplx::CBig;
    /// use dashu_float::{FBig, round::mode::HalfAway};
    ///
    /// type C = CBig<HalfAway, 10>;
    /// type F = FBig<HalfAway, 10>;
    /// let ctx = |v: i32| F::from(v).with_precision(53).value();
    /// let half = ctx(1) / ctx(2); // the parts share one significant-digit width
    /// let s = C::from_parts(half, ctx(0)).sin_pi();
    /// assert!(s == C::ONE);
    /// ```
    #[inline]
    pub fn sin_pi(&self) -> Self {
        self.context().unwrap_cfp(self.context().sin_pi(self, None))
    }

    /// Complex cosine of `z·π` (convenience layer). Panics on an indeterminate special value.
    ///
    /// The exact lattice applies here too: the cosine of an exactly representable half-integer
    /// real part is exactly zero, and of an integer is `±1`.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashu_cmplx::CBig;
    /// use dashu_float::{FBig, round::mode::HalfAway};
    ///
    /// type C = CBig<HalfAway, 10>;
    /// type F = FBig<HalfAway, 10>;
    /// let ctx = |v: i32| F::from(v).with_precision(53).value();
    /// let c = C::from_parts(ctx(1), ctx(0)).cos_pi();
    /// assert!(c == C::NEG_ONE); // cos(π) = -1
    /// ```
    #[inline]
    pub fn cos_pi(&self) -> Self {
        self.context().unwrap_cfp(self.context().cos_pi(self, None))
    }

    /// Simultaneously compute `(sin(z·π), cos(z·π))` (convenience layer).
    ///
    /// # Examples
    ///
    /// ```
    /// use dashu_cmplx::CBig;
    /// use dashu_float::{FBig, round::mode::HalfAway};
    ///
    /// type C = CBig<HalfAway, 10>;
    /// type F = FBig<HalfAway, 10>;
    /// let ctx = |v: i32| F::from(v).with_precision(53).value();
    /// let half = ctx(1) / ctx(2);
    /// let (s, c) = C::from_parts(half, ctx(0)).sin_cos_pi();
    /// assert!(s == C::ONE);
    /// assert!(c == C::ZERO);
    /// ```
    #[inline]
    pub fn sin_cos_pi(&self) -> (Self, Self) {
        let (s, c) = self.context().sin_cos_pi(self, None);
        (self.context().unwrap_cfp(s), self.context().unwrap_cfp(c))
    }

    /// Complex tangent of `z·π` (convenience layer). Panics at the real-axis poles
    /// (`y = 0`, x an odd multiple of `1/2`, where the result is indeterminate).
    ///
    /// # Examples
    ///
    /// ```
    /// use dashu_cmplx::CBig;
    /// use dashu_float::{FBig, round::mode::HalfAway};
    ///
    /// type C = CBig<HalfAway, 10>;
    /// type F = FBig<HalfAway, 10>;
    /// let ctx = |v: i32| F::from(v).with_precision(53).value();
    /// let quarter = ctx(1) / ctx(4);
    /// let t = C::from_parts(quarter, ctx(0)).tan_pi();
    /// assert!(t == C::ONE); // tan(π/4) = 1
    /// ```
    #[inline]
    pub fn tan_pi(&self) -> Self {
        self.context().unwrap_cfp(self.context().tan_pi(self, None))
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
    use dashu_base::Sign;
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
    fn pi_family_zero_inputs() {
        // The ×π family shares the exact-zero shortcuts, so it also works at unlimited precision
        assert!(C::ZERO.sin_pi() == C::ZERO);
        assert!(C::ZERO.cos_pi() == C::ONE);
        assert!(C::ZERO.tan_pi() == C::ZERO);
        let (s, c) = C::ZERO.sin_cos_pi();
        assert!(s == C::ZERO);
        assert!(c == C::ONE);
    }

    #[test]
    fn pi_family_infinite_is_indeterminate() {
        let ctx = Context::new(53);
        let inf = C::from(F::INFINITY);
        assert_eq!(ctx.sin_pi(&inf, None), Err(FpError::Indeterminate));
        assert_eq!(ctx.cos_pi(&inf, None), Err(FpError::Indeterminate));
        assert_eq!(ctx.tan_pi(&inf, None), Err(FpError::Indeterminate));
        let (s, c) = ctx.sin_cos_pi(&inf, None);
        assert_eq!(s, Err(FpError::Indeterminate));
        assert_eq!(c, Err(FpError::Indeterminate));
    }

    /// The real axis: `*_pi` of a pure-real z must agree with the real ×π kernels exactly,
    /// including the quarter-integer exact cases (sin_pi(1/2 + 0i) = 1 exactly, etc.).
    #[test]
    fn pi_family_real_axis_matches_real_kernels() {
        use core::str::FromStr;
        type HC = CBig<mode::HalfEven, 10>;
        type HF = FBig<mode::HalfEven, 10>;
        let ctx = Context::<mode::HalfEven>::new(53);
        let fctx = FloatCtxt::<mode::HalfEven>::new(53);
        for re in ["0.5", "0.25", "1.5", "2.5", "0.3", "-1.2", "3", "0.125"] {
            let x = HF::from_str(re).unwrap().with_precision(53).value();
            let z = HC::from_parts(x.clone(), HF::ZERO);

            let s = ctx.sin_pi(&z, None).unwrap().value();
            let expect = fctx.sin_pi::<10>(x.repr(), None).unwrap().value();
            assert!(s == HC::from_parts(expect, HF::ZERO), "sin_pi re={re}");
            let c = ctx.cos_pi(&z, None).unwrap().value();
            let expect = fctx.cos_pi::<10>(x.repr(), None).unwrap().value();
            assert!(c == HC::from_parts(expect, HF::ZERO), "cos_pi re={re}");
        }

        // exact quarter-integer cases land exactly
        let f53 = |v: i32| HF::from(v).with_precision(53).value();
        let half = HC::from_parts(f53(1) / 2u8, HF::ZERO);
        let s = ctx.sin_pi(&half, None).unwrap().value();
        assert!(s == HC::ONE);
        let c = ctx.cos_pi(&half, None).unwrap().value();
        assert!(c == HC::ZERO);
        let quarter = HC::from_parts(f53(1) / 4u8, HF::ZERO);
        let t = ctx.tan_pi(&quarter, None).unwrap().value();
        assert!(t == HC::ONE);

        // the real-axis pole: tan_pi(1/2 + 0i) is indeterminate (0/0 in the double-angle form)
        assert_eq!(ctx.tan_pi(&half, None), Err(FpError::Indeterminate));
    }

    /// The ×π family under directed modes: the p-digit result must match the `p + 60` HalfEven
    /// evaluation re-rounded to `p` under the same mode (the definition of correct rounding),
    /// across the precision sweep and including a near-pole tangent. Directed coverage was
    /// previously absent (only Nearest was tested).
    #[test]
    fn pi_family_directed_matches_oracle() {
        use core::str::FromStr;
        type HC = CBig<mode::HalfEven, 10>;
        type HF = FBig<mode::HalfEven, 10>;
        // (re, im): a generic point, a quarter-integer real part, and a near-pole real part
        // with a tiny imaginary part (the collapsed-denominator retry path under Down/Up)
        let points = [("0.3", "0.2"), ("0.25", "-1.5"), ("0.5", "1e-8")];
        for p in [16usize, 34, 50] {
            for (re_s, im_s) in points {
                // the input's p-digit rounding (shared across modes via the raw reprs)
                let re_r = HF::from_str(re_s)
                    .unwrap()
                    .with_precision(p)
                    .value()
                    .repr()
                    .clone();
                let im_r = HF::from_str(im_s)
                    .unwrap()
                    .with_precision(p)
                    .value()
                    .repr()
                    .clone();
                let part = |r: &Repr<10>, unlim: &FloatCtxt<mode::HalfEven>| {
                    HF::from_repr(r.clone(), *unlim)
                };
                let unlim = FloatCtxt::<mode::HalfEven>::new(0);
                let z = HC::from_parts(part(&re_r, &unlim), part(&im_r, &unlim));
                let hi = Context::<mode::HalfEven>::new(p + 60);
                let sin_hi = hi.sin_pi(&z, None).unwrap().value();
                let cos_hi = hi.cos_pi(&z, None).unwrap().value();
                let tan_hi = hi.tan_pi(&z, None).unwrap().value();

                // the directed evaluations need the input in their own rounding mode
                let mk_down = || {
                    let u = FloatCtxt::<mode::Down>::new(0);
                    CBig::<mode::Down, 10>::from_parts(
                        FBig::<mode::Down, 10>::from_repr(re_r.clone(), u),
                        FBig::<mode::Down, 10>::from_repr(im_r.clone(), u),
                    )
                };
                let mk_up = || {
                    let u = FloatCtxt::<mode::Up>::new(0);
                    CBig::<mode::Up, 10>::from_parts(
                        FBig::<mode::Up, 10>::from_repr(re_r.clone(), u),
                        FBig::<mode::Up, 10>::from_repr(im_r.clone(), u),
                    )
                };
                let down = Context::<mode::Down>::new(p);
                let zd = mk_down();
                check_directed(
                    "sin_pi",
                    p,
                    re_s,
                    im_s,
                    "Down",
                    down.sin_pi(&zd, None).unwrap().value(),
                    &sin_hi,
                );
                check_directed(
                    "cos_pi",
                    p,
                    re_s,
                    im_s,
                    "Down",
                    down.cos_pi(&zd, None).unwrap().value(),
                    &cos_hi,
                );
                check_directed(
                    "tan_pi",
                    p,
                    re_s,
                    im_s,
                    "Down",
                    down.tan_pi(&zd, None).unwrap().value(),
                    &tan_hi,
                );
                let up = Context::<mode::Up>::new(p);
                let zu = mk_up();
                check_directed(
                    "sin_pi",
                    p,
                    re_s,
                    im_s,
                    "Up",
                    up.sin_pi(&zu, None).unwrap().value(),
                    &sin_hi,
                );
                check_directed(
                    "cos_pi",
                    p,
                    re_s,
                    im_s,
                    "Up",
                    up.cos_pi(&zu, None).unwrap().value(),
                    &cos_hi,
                );
                check_directed(
                    "tan_pi",
                    p,
                    re_s,
                    im_s,
                    "Up",
                    up.tan_pi(&zu, None).unwrap().value(),
                    &tan_hi,
                );
            }
        }
    }

    /// Directed oracle check: `got` (computed at `p` under mode `M`) must equal the
    /// `p + 60` HalfEven evaluation with each part re-rounded to `p` under `M`.
    fn check_directed<M: ErrorBounds, const B: Word>(
        name: &str,
        p: usize,
        re_s: &str,
        im_s: &str,
        tag: &str,
        got: CBig<M, B>,
        hi: &CBig<mode::HalfEven, B>,
    ) {
        let conv = |v: &FBig<mode::HalfEven, B>| {
            FBig::<M, B>::from_repr(v.repr().clone(), FloatCtxt::<M>::new(0))
                .with_precision(p)
                .value()
        };
        let (hre, him) = hi.clone().into_parts();
        let expect = CBig::<M, B>::from_parts(conv(&hre), conv(&him));
        assert!(got == expect, "{name} p={p} re={re_s} im={im_s} {tag}: {got:?} vs {expect:?}");
    }

    /// Near-pole `tan`/`tan_pi`: when both double-angle denominator terms round onto `∓1`, the
    /// sum collapses to an exact zero — a rounding artifact that must retry at higher guard,
    /// not error (`0/0 → Indeterminate`), not panic (`x/0 →` an infinity in the part
    /// arithmetic). All three shapes are pinned: tiny imaginary part (huge finite value),
    /// pure-real near-pole (delegated to the real kernel), and the radian `tan` near `π/2`.
    #[test]
    fn tan_near_real_poles_retries_instead_of_erroring() {
        use core::str::FromStr;
        use dashu_base::AbsOrd;
        type HC = CBig<mode::HalfEven, 10>;
        type HF = FBig<mode::HalfEven, 10>;
        let ctx = Context::<mode::HalfEven>::new(53);
        let fctx = FloatCtxt::<mode::HalfEven>::new(53);

        // (1) tan_pi(0.5 + 1e-40·i): cos_pi(1) = −1 exact and cosh rounds onto 1, so the
        // denominator collapses at the first guard — the retry must recover
        // i·coth(π·1e-40) ≈ 3.183…e39·i.
        let half = HF::from_str("0.5").unwrap().with_precision(53).value();
        let tiny = HF::from_str("1e-40").unwrap().with_precision(53).value();
        let z = HC::from_parts(half, tiny.clone());
        let t53 = ctx.tan_pi(&z, None).unwrap().value();
        let (t53_re, t53_im) = t53.clone().into_parts();
        assert!(t53_re == HF::ZERO, "re = sin_pi(1)/D = 0/D");
        let lo = HF::from_str("3.18e39").unwrap();
        let hi = HF::from_str("3.19e39").unwrap();
        assert!(t53_im.abs_cmp(&lo).is_gt() && t53_im.abs_cmp(&hi).is_lt());
        // and it is correctly rounded: the p = 113 evaluation re-rounded to 53 agrees
        let t113 = Context::<mode::HalfEven>::new(113)
            .tan_pi(&z, None)
            .unwrap()
            .value();
        let (r113, i113) = t113.into_parts();
        let t113_rerounded =
            HC::from_parts(r113.with_precision(53).value(), i113.with_precision(53).value());
        assert!(t53 == t113_rerounded, "near-pole tan_pi not correctly rounded");

        // (2) pure-real near-pole: delegated to the real kernel — the exact same value
        // (previously the collapsed denominator surfaced as Err(Indeterminate)).
        let x = HF::from_str("0.50000000000000000000000000000000000000001")
            .unwrap()
            .with_precision(53)
            .value();
        let z = HC::from_parts(x.clone(), HF::ZERO);
        let t = ctx.tan_pi(&z, None).unwrap().value();
        let expect = fctx.tan_pi::<10>(x.repr(), None).unwrap().value();
        assert!(t == HC::from_parts(expect, HF::ZERO));

        // (3) both parts nonzero near the pole: tan_pi((0.5 + 1e-60) + 1e-40·i) — the shape
        // that previously panicked through the infinity quotient.
        let x = HF::from_str("0.500000000000000000000000000000000000000000000000001")
            .unwrap()
            .with_precision(53)
            .value();
        let z = HC::from_parts(x, tiny);
        let (_, t_im) = ctx.tan_pi(&z, None).unwrap().value().into_parts();
        assert!(t_im.abs_cmp(&lo).is_gt() && t_im.abs_cmp(&hi).is_lt());

        // (4) the radian twin: tan(x + 0i) with x = π/2 rounded to 53 digits — cos(2x)
        // rounds onto −1 against cosh(0) = 1.
        let x = HF::from_str("1.5707963267948966192313216916397514420985846996875529")
            .unwrap()
            .with_precision(53)
            .value();
        let z = HC::from_parts(x, HF::ZERO);
        let (t_re, t_im) = ctx.tan(&z, None).unwrap().value().into_parts();
        let huge = HF::from_str("1e50").unwrap();
        assert!(t_re.abs_cmp(&huge).is_gt(), "tan near π/2 must be huge");
        assert!(t_im == HF::ZERO);
    }

    /// The imaginary axis: `sin_pi(iy) = i·sinh(πy)` and `cos_pi(iy) = cosh(πy)`, exercising
    /// the π-scaled hyperbolic kernel.
    #[test]
    fn pi_family_imaginary_axis_matches_hyperbolic() {
        use core::str::FromStr;
        type HC = CBig<mode::HalfEven, 10>;
        type HF = FBig<mode::HalfEven, 10>;
        let ctx = Context::<mode::HalfEven>::new(53);
        let fctx = FloatCtxt::<mode::HalfEven>::new(53);
        for im in ["0.5", "0.25", "-1.5", "0.3", "2", "-0.125"] {
            let y = HF::from_str(im).unwrap().with_precision(53).value();
            let z = HC::from_parts(HF::ZERO, y.clone());

            let s = ctx.sin_pi(&z, None).unwrap().value();
            let (sre, sim) = s.into_parts();
            assert!(sre == HF::ZERO, "sin_pi re({im})");
            let expect = fctx.sinh_pi::<10>(y.repr(), None).unwrap().value();
            assert!(sim == expect, "sin_pi im({im})");

            let c = ctx.cos_pi(&z, None).unwrap().value();
            let (cre, cim) = c.into_parts();
            assert!(cim == HF::ZERO, "cos_pi im({im})");
            let expect = fctx.cosh_pi::<10>(y.repr(), None).unwrap().value();
            assert!(cre == expect, "cos_pi re({im})");
        }
    }

    /// `tan_pi(z) = sin_pi(z)/cos_pi(z)` on generic points, and the sin²+cos² = 1 identity.
    #[test]
    fn pi_family_generic_identities() {
        use dashu_base::AbsOrd;
        type HC = CBig<mode::HalfEven, 10>;
        type HF = FBig<mode::HalfEven, 10>;
        let ctx = Context::<mode::HalfEven>::new(53);
        // The identity check cancels from |sin|² ≈ cosh²(π·5) ≈ 10¹³ down to 1, so its own
        // evaluation error is ~1 ulp of 10¹³ at 53 digits ≈ 10⁻³⁹ — the tolerance is set
        // comfortably above that (the parts themselves stay correctly rounded).
        let tol = HF::from_parts(IBig::from(1), -30); // 10^-30
        let tol_ratio = HF::from_parts(IBig::from(1), -40); // 10^-40
        for (re, im) in [(1, 1), (2, -3), (-1, 2), (3, 5), (-2, -1)] {
            let z = HC::from_parts(
                HF::from(re).with_precision(53).value(),
                HF::from(im).with_precision(53).value(),
            );
            let (s, c) = ctx.sin_cos_pi(&z, None);
            let s = s.unwrap().value();
            let c = c.unwrap().value();
            // sin²(zπ) + cos²(zπ) = 1
            let sum = &s.sqr() + &c.sqr();
            let (sre, sim) = sum.into_parts();
            assert!(
                (sre.clone() - HF::ONE).abs_cmp(&tol).is_le(),
                "sin²+cos² re at ({re},{im}): {sre:?}"
            );
            assert!(sim.abs_cmp(&tol).is_le(), "sin²+cos² im at ({re},{im})");
            // tan_pi = sin_pi/cos_pi (re-rooted through the ziv loop, so compare loosely)
            let t = ctx.tan_pi(&z, None).unwrap().value();
            let ratio = &s / &c;
            let diff = &t - &ratio;
            let (dre, dim) = diff.into_parts();
            let tol_scaled = s.abs() * &tol_ratio;
            assert!(dre.abs_cmp(&tol_scaled).is_le(), "tan_pi re at ({re},{im})");
            assert!(dim.abs_cmp(&tol_scaled).is_le(), "tan_pi im at ({re},{im})");
        }
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

    // The mechanically tracked radius must certify at the target precision across the width
    // sweep: each result equals the same op computed at `p + 60` and re-rounded to `p`.
    #[test]
    fn pi_family_matches_oracle_across_precisions() {
        type C2 = CBig<mode::HalfEven, 2>;
        type F2 = FBig<mode::HalfEven, 2>;
        let inputs = [
            (1i64, 1i64),
            (3, 4),
            (-2, 1),
            (1, 0),
            (0, 3),
            (-5, 2),
            (3, 0), // half-integer real part after doubling: sin_pi(3/2·π)... exact cases
            (7, 0),
        ];
        for p in [20usize, 50, 100, 500] {
            for (re, im) in inputs {
                let mk = |v: i64| F2::from(v).with_precision(p).value();
                let mk_hi = |v: i64| F2::from(v).with_precision(p + 60).value();
                let zh = C2::from_parts(mk_hi(re), mk_hi(im));
                for (name, got, expect) in [
                    ("sin_pi", C2::from_parts(mk(re), mk(im)).sin_pi(), zh.sin_pi()),
                    ("tan_pi", C2::from_parts(mk(re), mk(im)).tan_pi(), zh.tan_pi()),
                ] {
                    let (ge, gi) = got.into_parts();
                    let (ee, ei) = expect.into_parts();
                    let expect_re = ee.with_precision(p).value();
                    let expect_im = ei.with_precision(p).value();
                    assert_eq!(ge.repr(), expect_re.repr(), "{name} re p={p} z=({re},{im})");
                    assert_eq!(gi.repr(), expect_im.repr(), "{name} im p={p} z=({re},{im})");
                }
            }
        }
    }

    // The ball-tracked radius must certify at the target precision across the width sweep,
    // including the near-singularity inputs where the old flat `ulp·20` radius never inflated.
    #[test]
    fn inverse_trig_matches_oracle_across_precisions() {
        type C2 = CBig<mode::HalfEven, 2>;
        type F2 = FBig<mode::HalfEven, 2>;
        let inputs = [
            (1i64, 1i64),
            (3, 4),
            (-2, 1),
            (0, 1), // ±i: branch point (atan reports Indeterminate — skipped below)
            (5, -12),
        ];
        for p in [20usize, 50, 100, 500] {
            for (re, im) in inputs {
                let mk = |v: i64| F2::from(v).with_precision(p).value();
                let mk_hi = |v: i64| F2::from(v).with_precision(p + 60).value();
                let zh = C2::from_parts(mk_hi(re), mk_hi(im));
                let z = C2::from_parts(mk(re), mk(im));
                for (name, got, expect) in
                    [("asin", zh.asin(), z.asin()), ("acos", zh.acos(), z.acos())]
                {
                    // both sides re-rounded to p (got comes out at p + 60)
                    let (gre, gim) = got.into_parts();
                    let (ere, eim) = expect.into_parts();
                    let got_re = gre.with_precision(p).value();
                    let got_im = gim.with_precision(p).value();
                    let expect_re = ere.with_precision(p).value();
                    let expect_im = eim.with_precision(p).value();
                    assert_eq!(got_re.repr(), expect_re.repr(), "{name} re p={p} z=({re},{im})");
                    assert_eq!(got_im.repr(), expect_im.repr(), "{name} im p={p} z=({re},{im})");
                }
                if im != 1 {
                    // the general path; `atan(±i)` is indeterminate on both sides
                    let (gre, gim) = zh.atan().into_parts();
                    let (ere, eim) = z.atan().into_parts();
                    let got_re = gre.with_precision(p).value();
                    let got_im = gim.with_precision(p).value();
                    let expect_re = ere.with_precision(p).value();
                    let expect_im = eim.with_precision(p).value();
                    assert_eq!(got_re.repr(), expect_re.repr(), "atan re p={p} z=({re},{im})");
                    assert_eq!(got_im.repr(), expect_im.repr(), "atan im p={p} z=({re},{im})");
                }
            }
        }
    }

    // The axis dispatch certifies under the outward modes: `atan(1 ± i·0) = π/4 ± i·0` — the
    // real kernel is correctly rounded and the imaginary zero is exact (a zero radius, which
    // the two-log difference of the general path could never produce).
    #[test]
    fn atan_axis_certifies_directed() {
        macro_rules! check {
            ($mode:ty) => {{
                type C = CBig<$mode, 10>;
                type F = FBig<$mode, 10>;
                let mk = |v: i32| F::from(v).with_precision(30).value();
                let got = C::from_parts(mk(1), mk(0)).atan();
                let expect = Context::<$mode>::new(30)
                    .float()
                    .atan(mk(1).repr(), None)
                    .unwrap()
                    .value();
                assert_eq!(got.re(), expect.repr());
                assert!(got.im().significand().is_zero());
            }};
        }
        check!(mode::Up);
        check!(mode::Down);
        check!(mode::Zero);
        check!(mode::HalfEven);
    }

    // `asin(±i) = ±i·asinh(1)` — the real part is *exactly* zero (the imaginary axis maps to
    // the imaginary axis), which the argument fold now certifies with a zero radius: the
    // componentwise gradient bound `(|y|·rad_x + |x|·rad_y)/‖z‖²` vanishes on the axis, where
    // the old joint 1-Lipschitz bound `(rad_x + rad_y)/‖z‖` kept a kernel-error radius on the
    // angle and dead-locked under the strict zero-candidate certification.
    #[test]
    fn asin_pure_imaginary_real_part_is_exact() {
        type C2 = CBig<mode::HalfEven, 2>;
        type F2 = FBig<mode::HalfEven, 2>;
        for p in [20usize, 50, 500] {
            let ctx = Context::<mode::HalfEven>::new(p);
            for sign in [1i64, -1] {
                let z = C2::from_parts(
                    F2::from_parts(IBig::ZERO, 0),
                    F2::from_parts(IBig::from(sign), 0),
                );
                let r = ctx.asin(&z, None).unwrap().value();
                assert!(
                    r.re().significand().is_zero(),
                    "asin({sign}i) @p={p}: real part must be exactly zero"
                );
                let want = if sign == 1 {
                    Sign::Positive
                } else {
                    Sign::Negative
                };
                assert_eq!(r.im().sign(), want, "asin({sign}i) @p={p}: imaginary sign");
            }
        }
    }
}
