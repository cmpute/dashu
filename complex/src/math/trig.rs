//! Complex trigonometric functions via the real–imaginary decomposition, reusing `dashu-float`'s
//! real `sin`/`cos` and cancellation-free `sinh`/`cosh` — plus their ×π variants on the real
//! `sin_cos_pi`/`sinh_cosh_pi`.
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

/// The blanket per-part radius of 8 working-ulps, with the exact-zero exemption: a part whose
/// value has a zero significand is the *exactly* zero result of the composition (a nonzero
/// product or quotient never rounds to a zero significand — unbounded exponents keep every
/// nonzero magnitude), so its provable error is 0. The exemption is not cosmetic: under a
/// directed mode the preimage of `+0` is one-sided (`[0, ulp)` under `Down`), which no
/// nonzero symmetric interval ever fits — the loop would retry to its cap.
fn ulp8<R: ErrorBounds, const B: Word>(v: &FBig<R, B>) -> FBig<R, B> {
    v.ulp()
        * if v.repr().significand().is_zero() {
            0
        } else {
            8
        }
}

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
                (sin_re.clone(), ulp8(&sin_re)),
                (sin_im.clone(), ulp8(&sin_im)),
                (cos_re.clone(), ulp8(&cos_re)),
                (cos_im.clone(), ulp8(&cos_im)),
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
            // D = cos 2x + cosh 2y  (a benign sum: a bounded term plus one ≥ 1). As in
            // `tan_pi`, near the real-axis poles the sum cancels into the addends' ~B^(1−pw)
            // absolute rounding noise (onto an exact zero or a few garbage digits); the
            // same lead(D) + guard ≥ 8 surviving-digits threshold separates the trustworthy
            // denominators from the ones that must retry at higher guard.
            let denom = gctx.add(cos2x.repr(), cosh2y.repr())?.value();
            let d_lead = denom
                .repr()
                .exponent()
                .saturating_add(denom.repr().digits_ub() as isize);
            if denom.repr().significand().is_zero() || d_lead + (guard as isize) < 8 {
                let zero = FBig::from_repr(Repr::zero(), FloatCtxt::<R>::new(pw));
                let one = FBig::from_repr(Repr::<B>::one(), FloatCtxt::<R>::new(pw));
                return Ok([(zero.clone(), one.clone()), (zero, one)]);
            }
            let re = gctx.div(sin2x.repr(), denom.repr())?.value();
            let im = gctx.div(sinh2y.repr(), denom.repr())?.value();
            // re-root to the working precision (`sin_cos`/`sinh_cosh`/`div` may return exact
            // constants for exact cases such as `tan(0) = 0`).
            let re = re.with_precision(pw).value();
            let im = im.with_precision(pw).value();
            Ok([(re.clone(), ulp8(&re)), (im.clone(), ulp8(&im))])
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

        // `sin zπ = sin_pi(x)·cosh(πy) + i·cos_pi(x)·sinh(πy)`,
        // `cos zπ = cos_pi(x)·cosh(πy) − i·sin_pi(x)·sinh(πy)`. Each factor is correctly
        // rounded at the working precision (contributing ~½ ulp each), so only the products
        // round — a few working-ULPs, like the radian `sin_cos`. The π-scaling lives inside the
        // real ×π kernels (their argument balls carry π's radius), NOT in a pre-multiplied
        // `π·y` (whose ½-ulp error the hyperbolic derivative would amplify to ~2π|y| ulps).
        let p = self.precision();
        let parts = self.ziv(TRIG_GUARD, |guard| {
            let gctx = FloatCtxt::<R>::new(p + guard);
            let (sinx, cosx) = gctx.sin_cos_pi(z.re(), reborrow_cache(&mut cache));
            let sinx = sinx?.value();
            let cosx = cosx?.value();
            let (sinhy, coshy) = gctx.sinh_cosh_pi(z.im(), reborrow_cache(&mut cache));
            let sinhy = sinhy?.value();
            let coshy = coshy?.value();
            let sin_re = gctx.mul(sinx.repr(), coshy.repr())?.value();
            let sin_im = gctx.mul(cosx.repr(), sinhy.repr())?.value();
            let cos_re = gctx.mul(cosx.repr(), coshy.repr())?.value();
            let neg_sinx = -sinx; // cos zπ's imaginary part is −sin_pi(x)·sinh(πy)
            let cos_im = gctx.mul(neg_sinx.repr(), sinhy.repr())?.value();
            Ok([
                (sin_re.clone(), ulp8(&sin_re)),
                (sin_im.clone(), ulp8(&sin_im)),
                (cos_re.clone(), ulp8(&cos_re)),
                (cos_im.clone(), ulp8(&cos_im)),
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

        let p = self.precision();
        let [re, im] = self.ziv(TRIG_GUARD, |guard| {
            let pw = p + guard;
            let gctx = FloatCtxt::<R>::new(pw);
            // 2x, 2y (exact doublings — same significand, exponent +1).
            let x2 = gctx.add(z.re(), z.re())?.value();
            let y2 = gctx.add(z.im(), z.im())?.value();
            let (sin2x, cos2x) = gctx.sin_cos_pi(x2.repr(), reborrow_cache(&mut cache));
            let sin2x = sin2x?.value();
            let cos2x = cos2x?.value();
            let (sinh2y, cosh2y) = gctx.sinh_cosh_pi(y2.repr(), reborrow_cache(&mut cache));
            let sinh2y = sinh2y?.value();
            let cosh2y = cosh2y?.value();
            // D = cos_pi(2x) + cosh(2πy)  (a benign sum: a bounded term plus one ≥ 1). With
            // y ≠ 0 (the pure-real case was delegated above) the true D is > 0, but each
            // addend is O(1) with absolute rounding error ~B^(1−pw) — near the poles the sum
            // cancels down into that noise (onto an exact zero, or onto a few garbage
            // digits). The quotient only carries target accuracy when D's surviving digits
            // reach past the guard: `lead(D) + guard ≥ 8` is exactly that condition (D's
            // relative error ~B^(2−pw−lead(D)) must sit below B^(−p−6); `lead` is the value's
            // leading position, `exponent + digits_ub` — the raw stored exponent is *not*
            // normalized after a cancelling add). Short of it — and on a collapsed zero —
            // report an all-straddling interval instead of dividing (0/0 would surface as a
            // terminal Indeterminate, x/0 as an infinity the part arithmetic panics on), and
            // let the driver retry at higher guard, where the always-positive true
            // difference re-emerges with enough digits.
            let denom = gctx.add(cos2x.repr(), cosh2y.repr())?.value();
            let d_lead = denom
                .repr()
                .exponent()
                .saturating_add(denom.repr().digits_ub() as isize);
            if denom.repr().significand().is_zero() || d_lead + (guard as isize) < 8 {
                let zero = FBig::from_repr(Repr::zero(), FloatCtxt::<R>::new(pw));
                let one = FBig::from_repr(Repr::<B>::one(), FloatCtxt::<R>::new(pw));
                return Ok([(zero.clone(), one.clone()), (zero, one)]);
            }
            let re = gctx.div(sin2x.repr(), denom.repr())?.value();
            let im = gctx.div(sinh2y.repr(), denom.repr())?.value();
            // re-root to the working precision (`sin_cos_pi`/`sinh_cosh_pi`/`div` may return
            // exact constants for exact cases such as `tan_pi(1/4) = 1`).
            let re = re.with_precision(pw).value();
            let im = im.with_precision(pw).value();
            Ok([(re.clone(), ulp8(&re)), (im.clone(), ulp8(&im))])
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

    /// Complex sine of `z·π` (convenience layer). Panics on an indeterminate special value.
    #[inline]
    pub fn sin_pi(&self) -> Self {
        self.context().unwrap_cfp(self.context().sin_pi(self, None))
    }

    /// Complex cosine of `z·π` (convenience layer). Panics on an indeterminate special value.
    #[inline]
    pub fn cos_pi(&self) -> Self {
        self.context().unwrap_cfp(self.context().cos_pi(self, None))
    }

    /// Simultaneously compute `(sin(z·π), cos(z·π))` (convenience layer).
    #[inline]
    pub fn sin_cos_pi(&self) -> (Self, Self) {
        let (s, c) = self.context().sin_cos_pi(self, None);
        (self.context().unwrap_cfp(s), self.context().unwrap_cfp(c))
    }

    /// Complex tangent of `z·π` (convenience layer). Panics at the real-axis poles
    /// (`y = 0`, x an odd multiple of `1/2`, where the result is indeterminate).
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
}
