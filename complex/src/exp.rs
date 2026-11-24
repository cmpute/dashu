//! Complex exponential and powers.
//!
//! * [`Context::exp`] / [`CBig::exp`]: `exp(x+iy) = e^x·(cos y + i sin y)`.
//! * [`Context::powi`] / [`CBig::powi`]: integer exponent via repeated squaring (branch-cut-free,
//!   cheaper than `exp(n·log z)`).
//! * [`Context::powf`] / [`CBig::powf`]: `exp(w·log z)` on the principal branch.
//!
//! Mirroring `dashu-float`, the power family lives alongside `exp` in a single module.

use crate::ball::{kernel_ball, powi_is_exact, CBall};
use crate::cbig::CBig;
use crate::repr::{combine_parts, exact, reborrow_cache, riemann, CRounded, CfpResult, Context};
use dashu_base::{BitTest, Sign};
use dashu_float::round::ErrorBounds;
use dashu_float::{Ball, Repr};
use dashu_float::{ConstCache, Context as FloatCtxt, FBig, FpError};
use dashu_int::{IBig, UBig, Word};

/// Guard digits (base-B) for `exp`. Composes a real `exp`, a `sin_cos`, and two products.
const EXP_GUARD: usize = 14;

/// Guard digits (base-B) for `powf`. Composes `log`, a complex product, and `exp` — the
/// cancellation-prone path, so a larger guard than the bare arithmetic ops.
const POWF_GUARD: usize = 22;

/// The largest `k` for which a real exponent `m/2ᵏ` is evaluated through the exact
/// square-root chain (see [`Context::powf`]).
const POWF_DYADIC_MAX_SQRTS: usize = 64;

/// The widest integer numerator (in bits) `powf` will lift out of a real exponent. An exponent
/// stored as `1·B^(10¹⁸)` is an "integer" too, but materializing it would allocate its whole
/// value; anything this large over- or underflows through the general path anyway.
const POWF_INT_EXPONENT_MAX_BITS: usize = 256;

/// Left-to-right binary exponentiation `startⁿ` (`n ≥ 1`) on a complex ball. Every product's
/// compounding rounding is tracked, and an all-exact chain keeps `rad == 0` on both components.
fn pow_chain<const B: Word>(start: &CBall<B>, n: &UBig, pw: usize) -> Result<CBall<B>, FpError> {
    let nlen = n.bit_len();
    let mut acc = start.clone();
    for i in (0..nlen - 1).rev() {
        acc = acc.sqr(pw)?;
        if n.bit(i) {
            acc = acc.mul(start, pw)?;
        }
    }
    Ok(acc)
}

/// Write a finite real exponent as `m / 2ᵏ` with `m` an integer of at most
/// [`POWF_INT_EXPONENT_MAX_BITS`] bits and `0 ≤ k ≤` [`POWF_DYADIC_MAX_SQRTS`], if it has that
/// form (`k` minimal). In base 10 this recognizes `0.5 = 1/2`, `0.75 = 3/4`, … as well.
fn dyadic_exponent<const B: Word>(w: &Repr<B>) -> Option<(IBig, usize)> {
    if w.is_infinite() || w.significand().is_zero() {
        return None;
    }
    // work at unlimited precision: the doublings are exact
    let x = FloatCtxt::<dashu_float::round::mode::Zero>::new(0);
    let two = Repr::<B>::new(IBig::from(2u8), 0);
    let mut t = w.clone();
    for k in 0..=POWF_DYADIC_MAX_SQRTS {
        // a normalized `Repr` is an integer iff its exponent is non-negative
        if t.exponent() >= 0 {
            let digits = (t.exponent() as usize).checked_add(t.digits_ub())?;
            // `digits` base-B digits fit in `digits·⌈log₂B⌉` bits
            let bits = digits.checked_mul((Word::BITS - (B - 1).leading_zeros()) as usize)?;
            if bits > POWF_INT_EXPONENT_MAX_BITS {
                return None;
            }
            let m = t.significand() * IBig::from(B).pow(t.exponent() as usize);
            return Some((m, k));
        }
        // a doubling shortens the fraction by at most one digit; a fraction longer than the
        // remaining budget cannot clear
        if t.exponent().unsigned_abs() > POWF_DYADIC_MAX_SQRTS - k {
            return None;
        }
        t = x.mul(&t, &two).ok()?.value().into_repr();
    }
    None
}

impl<R: ErrorBounds> Context<R> {
    /// The exact constant `1 + 0i` at this context's precision.
    fn exact_one<const B: Word>(&self) -> CRounded<R, B> {
        exact(
            FBig::from_repr(Repr::one(), self.float()),
            FBig::from_repr(Repr::zero(), self.float()),
        )
    }

    /// Raise a complex number to an integer power under this context (context layer), correctly
    /// rounded via a Ziv loop over the binary-exponentiation (repeated-squaring) chain. No cache.
    ///
    /// `powi(z, 0) = 1`; a negative exponent computes `(1/z)^|n|` directly, so the
    /// sign-dependent overflow/underflow propagates from the closure with `?`. The chain runs on
    /// a `CBall`, so each squaring's compounding relative error joins the radius mechanically
    /// — and an all-exact chain keeps `rad == 0`, which is what certifies an
    /// exactly-representable `zⁿ` under the directed modes. A base whose parts have identical
    /// magnitudes takes the exact `t·(1±i)` lattice shortcut instead (see the branch below).
    ///
    /// At unlimited precision a non-negative power is computed exactly (like `z·z`); a negative
    /// one panics, since a reciprocal is not exactly representable in general.
    ///
    /// # Panics
    ///
    /// Panics if the precision is unlimited and the exponent is negative.
    pub fn powi<const B: Word>(&self, z: &CBig<R, B>, exp: IBig) -> CfpResult<R, B> {
        let (sign, n) = exp.into_parts();
        if n.is_zero() {
            // z⁰ = 1 is exact at every precision; the result carries this context's precision
            // like every other power does
            return Ok(self.exact_one());
        }
        let negative = sign == Sign::Negative;
        if z.is_infinite() {
            // |n| >= 2: an infinite base can't be raised to a higher power (the terminal-infinity
            // model rejects infinite operands); report Indeterminate like the other complex
            // transcendentals rather than leaking the float layer's InfiniteInput.
            return Err(FpError::Indeterminate);
        }

        if self.precision() == 0 && !negative {
            return self.powi_unlimited(z, &n);
        }

        match self.powi_rounded(z, negative, &n) {
            // The chain reports the sign of whichever intermediate product left the exponent
            // range (a square, so `Positive`). For a real base the sign of the true result is
            // known — negative iff the base is negative and the exponent odd — and it is what
            // the signed zero / infinity of the convenience layer must carry.
            Err(e @ (FpError::Overflow(_) | FpError::Underflow(_)))
                if z.im().significand().is_zero() =>
            {
                let sign = if z.re().sign() == Sign::Negative && n.bit(0) {
                    Sign::Negative
                } else {
                    Sign::Positive
                };
                Err(match e {
                    FpError::Overflow(_) => FpError::Overflow(sign),
                    _ => FpError::Underflow(sign),
                })
            }
            other => other,
        }
    }

    /// `zⁿ` (`n ≥ 1`) at unlimited precision: the squaring chain on exact products.
    fn powi_unlimited<const B: Word>(&self, z: &CBig<R, B>, n: &UBig) -> CfpResult<R, B> {
        let x = FloatCtxt::<R>::new(0);
        // every op below is exact at unlimited precision; only a range error can surface
        let mul = |a: &Repr<B>, b: &Repr<B>| x.mul(a, b).map(|r| r.value().into_repr());
        let (a0, b0) = (z.re().clone(), z.im().clone());
        let (mut a, mut b) = (a0.clone(), b0.clone());
        for i in (0..n.bit_len() - 1).rev() {
            // (a + bi)² = (a² − b²) + 2ab·i
            let re = x.sub(&mul(&a, &a)?, &mul(&b, &b)?)?.value().into_repr();
            let ab = mul(&a, &b)?;
            let im = x.add(&ab, &ab)?.value().into_repr();
            a = re;
            b = im;
            if n.bit(i) {
                let re = x.sub(&mul(&a, &a0)?, &mul(&b, &b0)?)?.value().into_repr();
                let im = x.add(&mul(&a, &b0)?, &mul(&b, &a0)?)?.value().into_repr();
                a = re;
                b = im;
            }
        }
        Ok(exact(FBig::from_repr(a, self.float()), FBig::from_repr(b, self.float())))
    }

    /// The limited-precision `z^(±n)` (`n ≥ 1`) through the Ziv loop.
    fn powi_rounded<const B: Word>(
        &self,
        z: &CBig<R, B>,
        negative: bool,
        n: &UBig,
    ) -> CfpResult<R, B> {
        let p = self.precision();
        let nlen = n.bit_len();

        // Diagonal shortcut: z = t·(1±i) with parts of *identical exact magnitude* ⇒
        // `zⁿ = tⁿ·(1±i)ⁿ`, where `(1±i)ⁿ` sits on the exact half-power-of-two lattice
        // (magnitude `2^(n/2)` in steps of √2, arguments in multiples of π/4), so both
        // components are the one certified real power `tⁿ` scaled by a power of two — and the
        // zero component of an even power is *exactly* zero. The generic squaring chain cannot
        // certify that zero: its `a² − b²` cancels onto exact 0 at every working precision (the
        // parts have the same magnitude), but once the chain is inexact (a reciprocal, or a
        // power too wide for the working precision) the tracked radius stays an ε > 0, which
        // the strict zero-candidate certification (no nonzero real rounds to 0) correctly
        // refuses — an endless retry. The anti-diagonal `t·(1−i)` is the conjugate lattice
        // (`(1−i)ⁿ = conj((1+i)ⁿ)`). A near-diagonal z (parts *rounding* together at low
        // precision but distinct) deliberately does not enter here: its true zero component is
        // nonzero, and the retry that separates the squares is the certification working.
        let diagonal = z.re() == z.im();
        let anti_diagonal = !diagonal && *z.re() == -z.im().clone();
        if (diagonal || anti_diagonal) && !z.re().significand().is_zero() {
            let [re, im] = self.ziv(nlen + 6, |guard| {
                let pw = p + guard;
                let fctx = FloatCtxt::<R>::new(pw);
                let exp: IBig = if negative {
                    -IBig::from(n.clone())
                } else {
                    IBig::from(n.clone())
                };
                // `tⁿ`, certified by the real Ziv loop under this mode (negative `n` included)
                let tp = kernel_ball(fctx.powi(z.re(), exp.clone())?, pw, |mid| {
                    powi_is_exact(mid, z.re(), &exp, pw)
                });
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
                // `(1−i)ⁿ` is the conjugate lattice point
                let b = if anti_diagonal { -b } else { b };
                // The lattice magnitude `2^k`, evaluated by the float `powi` in *this* base and
                // certified by its own Ziv loop. For the binary base it is a bare exponent shift
                // (exact at every precision); above it, it is exact whenever `2^k` fits the
                // working precision — the common small-|k| case, and what keeps an
                // exactly-representable `zⁿ` certifiable under the directed modes.
                //
                // The magnitude must be built in *base 2*: `2^k = B^(k·log_B 2)` has no exact
                // `Repr<B>` form for a non-binary `B`, so reading the exponent against the base
                // (`FBig::from_parts(v, k)`, the natural-looking spelling) yields `v·B^k` — exact
                // only for `B = 2` and silently wrong everywhere else (base-10 `(1+i)⁴` came out
                // as `−100`). Here the ratio `log_B 2` is *computed*, so the conversion's own
                // rounding joins the radius like any other.
                let two = Repr::<B>::new(IBig::from(2u8), 0);
                let pow2 = kernel_ball(fctx.powi(&two, k.clone())?, pw, |mid| {
                    powi_is_exact(mid, &two, &k, pw)
                });
                let part = |v: i32| -> Result<Ball<B>, FpError> {
                    if v == 0 {
                        return Ok(Ball::exact(Repr::<B>::zero()));
                    }
                    // the lattice factor is exact (or carries its conversion rounding): its
                    // product with `tⁿ` costs one rounding
                    let s = if v < 0 { -pow2.clone() } else { pow2.clone() };
                    s.mul(&tp, pw)
                };
                Ok([
                    part(a)?.to_value_radius::<R>(&fctx),
                    part(b)?.to_value_radius::<R>(&fctx),
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
            Ok(pow_chain(&start, n, pw)?.to_parts_radius(&fctx))
        })?;
        Ok(combine_parts(re, im))
    }

    /// Complex exponential under this context (context layer). Computes `e^x·(cos y + i·sin y)`
    /// from `dashu-float`'s (correctly-rounded) `exp` and `sin_cos`, wrapped in a Ziv loop that
    /// certifies both parts; the cache is threaded into both (the convenience layer passes `None`).
    ///
    /// Special values: `exp(±0 ± i·0) = 1 ± i·0` (the imaginary zero keeps its sign, so
    /// `exp(conj z) = conj(exp z)`); `exp(+inf + i·finite) = +∞` (Riemann point);
    /// `exp(-inf + i·finite) = 0`; an infinite imaginary part makes the trig undefined
    /// (`Indeterminate`).
    pub fn exp<const B: Word>(
        &self,
        z: &CBig<R, B>,
        mut cache: Option<&mut ConstCache>,
    ) -> CfpResult<R, B> {
        if z.is_zero() {
            // exact at every precision; the result carries this context's precision (not the
            // unlimited precision of a bare constant) and the sign of the imaginary zero
            return Ok(exact(
                FBig::from_repr(Repr::one(), self.float()),
                FBig::from_repr(Repr::zero_with_sign(z.im().sign()), self.float()),
            ));
        }
        if z.is_infinite() {
            if z.im().is_infinite() {
                return Err(FpError::Indeterminate); // cos/sin(±inf) undefined
            }
            return if z.re().sign() == Sign::Positive {
                Ok(riemann(*self))
            } else {
                Ok(exact(
                    FBig::from_repr(Repr::zero(), self.float()),
                    FBig::from_repr(Repr::zero(), self.float()),
                ))
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
    /// principal branch, correctly rounded via a Ziv loop. `powf(z, 0) = 1` for every `z`
    /// (including `powf(0, 0) = 1`, matching `FBig::powf`), and `powf(0, w) = 0` when `Re w > 0`.
    ///
    /// A real exponent of the form `m/2ᵏ` (an integer, a half-integer, …) is evaluated as
    /// `(z^(1/2ᵏ))^m` through `k` principal square roots and the integer-power chain — the same
    /// principal value, but through operations that are exact whenever their result is, so an
    /// exactly-representable power (`i² = −1`, `(−1)^½ = i`, `4^½ = 2`, or a power landing
    /// exactly on a rounding tie) is recognized instead of being approached forever by the
    /// transcendental route. An integer exponent is simply [`powi`](Self::powi).
    ///
    /// Otherwise the result's error is amplified by the exponent magnitude `‖w·log base‖`: the
    /// outer `exp` multiplies the error in `w·log base` by the result magnitude, so the per-part
    /// radius carries a data-dependent `‖w·log base‖` factor (mirroring `FBig::powf`). Overflow
    /// (a large exponent) propagates from the closure with `?`.
    ///
    /// A zero base with `Re w ≤ 0` (and `w ≠ 0`) is [`FpError::Indeterminate`], like
    /// `powi(0, −1)`.
    pub fn powf<const B: Word>(
        &self,
        base: &CBig<R, B>,
        w: &CBig<R, B>,
        mut cache: Option<&mut ConstCache>,
    ) -> CfpResult<R, B> {
        if w.is_zero() {
            return Ok(self.exact_one()); // powf(z, 0) = 1, incl. powf(0, 0)
        }
        self.assert_limited();
        if base.is_zero() && !w.is_infinite() {
            // 0^w = exp(w·log 0) with log 0 = −∞: the modulus is 0^(Re w), so the power is 0
            // for Re w > 0; for Re w < 0 it is a pole and for a purely imaginary w it has no
            // limit at all.
            return if w.re().sign() == Sign::Positive && !w.re().significand().is_zero() {
                Ok(exact(
                    FBig::from_repr(Repr::zero(), self.float()),
                    FBig::from_repr(Repr::zero(), self.float()),
                ))
            } else {
                Err(FpError::Indeterminate)
            };
        }
        if w.im().significand().is_zero() && !base.is_infinite() {
            if let Some((m, k)) = dyadic_exponent(w.re()) {
                if k == 0 {
                    return self.powi(base, m);
                }
                return self.powf_dyadic(base, m, k);
            }
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

    /// `base^(m/2ᵏ)` (`k ≥ 1`, `m ≠ 0`) as `(base^(1/2ᵏ))^m`: `k` tracked principal square roots,
    /// then the integer-power chain. Each halving of the argument stays on the principal branch,
    /// so this is exactly `exp((m/2ᵏ)·log base)`.
    fn powf_dyadic<const B: Word>(&self, base: &CBig<R, B>, m: IBig, k: usize) -> CfpResult<R, B> {
        let (sign, n) = m.into_parts();
        let negative = sign == Sign::Negative;
        let p = self.precision();
        let [re, im] = self.ziv(POWF_GUARD + n.bit_len() + k, |guard| {
            let pw = p + guard;
            let fctx = FloatCtxt::<R>::new(pw);
            let mut root = CBall::from_parts(base.re(), base.im(), pw);
            for _ in 0..k {
                root = root.sqrt(&fctx, pw)?;
            }
            let start = if negative { root.inv(pw)? } else { root };
            Ok(pow_chain(&start, &n, pw)?.to_parts_radius(&fctx))
        })?;
        Ok(combine_parts(re, im))
    }
}

impl<R: ErrorBounds, const B: Word> CBig<R, B> {
    /// Integer power (convenience layer). A non-negative power of an unlimited-precision value
    /// is exact.
    ///
    /// # Panics
    ///
    /// Panics on an indeterminate / out-of-domain result (e.g. `0⁻¹`), or if the precision is
    /// unlimited and the exponent is negative.
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
    /// convention; `powf(0, w) = 0` when `Re w > 0`.
    ///
    /// # Panics
    ///
    /// Panics if the precision is unlimited (unless `w` is zero), or on an indeterminate result
    /// (a zero base with `Re w ≤ 0`, an infinite operand).
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
        // Rungs: bit precisions at base 2, digit precisions at base 10, picked to span the
        // same significand widths (7 ≈ 20 bits, …) so both bases cover the same paths.
        macro_rules! sweep {
            ($base:literal, $name:literal, $precs:expr) => {{
                type C2 = CBig<mode::HalfEven, $base>;
                type F2 = FBig<mode::HalfEven, $base>;
                let inputs = [(1i64, 0i64), (0, 2), (-2, 1), (3, -4), (0, -5), (-4, 0)];
                for p in $precs {
                    for (re, im) in inputs {
                        let mk = |v: i64| F2::from(v).with_precision(p).value();
                        let mk_hi = |v: i64| F2::from(v).with_precision(p + 60).value();
                        let (hre, him) = C2::from_parts(mk_hi(re), mk_hi(im)).exp().into_parts();
                        let expect_re = hre.with_precision(p).value();
                        let expect_im = him.with_precision(p).value();
                        let got = C2::from_parts(mk(re), mk(im)).exp();
                        assert_eq!(got.re(), expect_re.repr(), "[$name] re p={p} z=({re},{im})");
                        assert_eq!(got.im(), expect_im.repr(), "[$name] im p={p} z=({re},{im})");
                    }
                }
            }};
        }
        sweep!(2, "base 2", [20, 50, 100, 500]);
        sweep!(10, "base 10", [7, 17, 34, 160]);
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

    // The diagonal shortcut's lattice magnitude is `2^((n−r)/2)` — a power of *two*, which is a
    // bare exponent shift only in the binary base. Sizing it against the float base instead
    // (`FBig::from_parts(v, k)`, whose value is `v·B^k`) silently mis-scaled every other base:
    // base-10 `(1+i)⁴` came out as `−100`, and the base-2-only sweeps beside it never noticed.
    // The exact lattice values are hand-written per base, and every mode is checked — the whole
    // point of the shortcut is that an exactly-representable `zⁿ` certifies under the outward
    // modes through its zero radius.
    #[test]
    fn powi_diagonal_lattice_scales_by_a_power_of_two() {
        macro_rules! check {
            ($mode:ty, $base:literal, $half:literal) => {{
                type C = CBig<$mode, $base>;
                type F = FBig<$mode, $base>;
                let mk =
                    |s: i32, e: isize| F::from_parts(IBig::from(s), e).with_precision(30).value();
                for (n, want_re, want_im) in [
                    (4i32, mk(-4, 0), mk(0, 0)),         // (1+i)⁴ = −4
                    (3, mk(-2, 0), mk(2, 0)),            // (1+i)³ = −2 + 2i
                    (1, mk(1, 0), mk(1, 0)),             // (1+i)¹ = 1 + i
                    (-1, mk($half, -1), mk(-$half, -1)), // (1+i)⁻¹ = 1/2 − i/2
                    (-2, mk(0, 0), mk(-$half, -1)),      // (1+i)⁻² = −i/2
                ] {
                    let got = C::from_parts(mk(1, 0), mk(1, 0)).powi(IBig::from(n));
                    assert_eq!(got.re(), want_re.repr(), "(1+i)^{n} re, base $base");
                    assert_eq!(got.im(), want_im.repr(), "(1+i)^{n} im, base $base");
                }
            }};
        }
        // `1/2` is `5·10⁻¹` in base 10 and `8·16⁻¹` in base 16
        check!(mode::Up, 10, 5);
        check!(mode::Down, 10, 5);
        check!(mode::Zero, 10, 5);
        check!(mode::HalfEven, 10, 5);
        check!(mode::Up, 16, 8);
        check!(mode::Down, 16, 8);
        check!(mode::HalfEven, 16, 8);
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

    // The ball-tracked radius must certify at the target precision across the width sweep, on
    // both float bases — the binary-only version of this sweep is where the `2^k`-vs-`B^k`
    // lattice mistake hid. The case list includes a diagonal input so the shortcut above is
    // actually entered. (The oracle is the *same operation* at `p + 60`, re-rounded, so it
    // bounds precision handling — a systematic value error is invisible to it and needs an
    // exact expectation, as `powi_diagonal_lattice_scales_by_a_power_of_two` carries.)
    #[test]
    fn powi_matches_oracle_across_precisions() {
        // Rungs: bit precisions at base 2, digit precisions at base 10, picked to span the
        // same significand widths (7 ≈ 20 bits, …) so both bases cover the same paths.
        macro_rules! sweep {
            ($base:literal, $name:literal, $precs:expr) => {{
                type C2 = CBig<mode::HalfEven, $base>;
                type F2 = FBig<mode::HalfEven, $base>;
                let cases = [
                    (3i64, 4i64, 5i32),
                    (1, -1, 13),
                    (2, 1, -7),
                    (3, 2, 11),
                    (2, 2, 5),
                ];
                for p in $precs {
                    for (re, im, n) in cases {
                        let mk = |v: i64| F2::from(v).with_precision(p).value();
                        let mk_hi = |v: i64| F2::from(v).with_precision(p + 60).value();
                        let (hre, him) = C2::from_parts(mk_hi(re), mk_hi(im))
                            .powi(n.into())
                            .into_parts();
                        let expect_re = hre.with_precision(p).value();
                        let expect_im = him.with_precision(p).value();
                        let got = C2::from_parts(mk(re), mk(im)).powi(n.into());
                        assert_eq!(got.re(), expect_re.repr(), "[$name] re p={p} n={n}");
                        assert_eq!(got.im(), expect_im.repr(), "[$name] im p={p} n={n}");
                    }
                }
            }};
        }
        sweep!(2, "base 2", [20, 50, 100, 500]);
        sweep!(10, "base 10", [7, 17, 34, 160]);
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
    // `powf` lifts a real exponent `m/2ᵏ` out of any base; anything else (a non-dyadic
    // fraction, too many halvings, a numerator too large to materialize) takes the general path.
    #[test]
    fn dyadic_exponent_recognition() {
        let r2 = |s: i64, e: isize| Repr::<2>::new(IBig::from(s), e);
        let r10 = |s: i64, e: isize| Repr::<10>::new(IBig::from(s), e);
        assert_eq!(dyadic_exponent(&r2(5, 0)), Some((IBig::from(5), 0)));
        assert_eq!(dyadic_exponent(&r2(-3, 4)), Some((IBig::from(-48), 0)));
        assert_eq!(dyadic_exponent(&r2(1, -1)), Some((IBig::from(1), 1)));
        assert_eq!(dyadic_exponent(&r2(-3, -5)), Some((IBig::from(-3), 5)));
        assert_eq!(dyadic_exponent(&r2(1, -64)), Some((IBig::from(1), 64)));
        assert_eq!(dyadic_exponent(&r2(1, -65)), None);
        assert_eq!(dyadic_exponent(&r2(1, 300)), None); // an integer, but 301 bits
        assert_eq!(dyadic_exponent(&r2(0, 0)), None);
        assert_eq!(dyadic_exponent(&Repr::<2>::infinity()), None);
        assert_eq!(dyadic_exponent(&r10(5, -1)), Some((IBig::from(1), 1))); // 0.5
        assert_eq!(dyadic_exponent(&r10(75, -2)), Some((IBig::from(3), 2))); // 0.75
        assert_eq!(dyadic_exponent(&r10(2, 1)), Some((IBig::from(20), 0)));
        assert_eq!(dyadic_exponent(&r10(2, -1)), None); // 0.2 = 1/5
        assert_eq!(dyadic_exponent(&r10(1, 100)), None);
    }
}
