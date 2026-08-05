//! Ball arithmetic: a midpoint paired with an integer relative-error count.
//!
//! A [`Ball`] represents a real number as `mid ± n·ulp(mid)`, where `ulp(mid)` is the
//! working-precision ulp of the midpoint: `|mid − true| ≤ n · ulp(mid)`.
//!
//! The error is tracked as an exact integer `n` (a base-`B` ulp count at the midpoint's own
//! precision) rather than as a rounded floating-point radius, so every propagation step is an
//! exact integer operation — no rounding guard is needed, and cancellation is handled
//! automatically by the exponent scale factors (a `c`-digit cancel multiplies `n` by `B^c`, so
//! the error stays referenced to the pre-cancellation magnitude). Each operation rounds the new
//! midpoint via `mid`'s own context and folds one rounding ulp into `n`.
//!
//! The midpoint is always [`mode::HalfEven`] (tightest). The target rounding mode `R` does not
//! parameterize a [`Ball`] — it enters only at the Ziv boundary, via
//! [`to_value_radius`](Ball::to_value_radius), which re-tags the midpoint to `R` at the working
//! precision and converts `n` into an absolute radius `n·ulp(mid)`. Ziv re-rounds to the target
//! precision far coarser than the work ulp, so the intermediate mode is immaterial to the final
//! correct rounding.

use dashu_base::{Abs, Approximation, BitTest, EstimatedLog2};
use dashu_int::{IBig, UBig};

use crate::{
    error::FpError,
    fbig::FBig,
    repr::{Context, Repr, Word},
    round::{mode, Rounded},
    utils::{shl_digits, shr_digits_ceil},
};

/// `x · B^exp` when `exp ≥ 0`; `⌈x / B^(−exp)⌉` when `exp < 0`.
///
/// The rounding is always *up* (never down), keeping the result a sound upper bound for the
/// underlying rational — the propagation rules below rely on this monotonicity. Both directions
/// delegate to the shared radix-shift primitives ([`shl_digits`](crate::utils::shl_digits),
/// [`shr_digits_ceil`](crate::utils::shr_digits_ceil)).
#[inline]
pub(crate) fn ceil_shift<const B: Word>(x: IBig, exp: isize) -> IBig {
    if exp >= 0 {
        // `x·B^exp` is the shared "multiply by a radix power" primitive (fast path for power-of-two
        // bases, `(x·5^k)<<k` for base 10).
        shl_digits::<B>(&x, exp as usize)
    } else {
        // Small numerator: x < 2^k ≤ B^k ⇒ ⌈x/B^k⌉ = 1 (the common series-tail case), avoiding
        // the shift/division inside the shared primitive.
        let k = (-exp) as usize;
        if !x.is_zero() && x.log2_bounds().1 <= k as f32 {
            return IBig::ONE;
        }
        shr_digits_ceil::<B>(&x, k)
    }
}

/// An approximate real: midpoint + integer error count.
#[derive(Clone)]
pub(crate) struct Ball<const B: Word> {
    /// The midpoint, always round-to-nearest. Carries its own context (precision + mode), so `n`
    /// is unambiguously "base-`B` ulps at `mid`'s precision".
    pub(crate) mid: FBig<mode::HalfEven, B>,
    /// Error bound: `|mid − true| ≤ n · ulp(mid)`, an exact integer (no round-up guard).
    pub(crate) n: IBig,
}

impl<const B: Word> Ball<B> {
    /// Leading digit position `E = exponent + digits`, so `ulp(mid) = B^(E − precision)`.
    ///
    /// Uses the cheap `digits_ub` (over-estimate): `E` appears only as a *positive* contribution
    /// to error exponents, so an inflated `E` only loosens (never tightens) a radius.
    #[inline]
    pub(crate) fn lead_exp(mid: &FBig<mode::HalfEven, B>) -> isize {
        // Saturating: at the exponent-range extremes (`±isize::MAX`) a plain add could wrap; a
        // saturated E only loosens (never tightens) a radius.
        mid.repr()
            .exponent
            .saturating_add(mid.repr().digits_ub() as isize)
    }

    /// Wrap an exactly-represented value (n = 0).
    #[inline]
    pub(crate) fn exact(mid: FBig<mode::HalfEven, B>) -> Self {
        Self { mid, n: IBig::ZERO }
    }

    /// Wrap a correctly-rounded result (an input rounding or a `Context` op): exact values carry
    /// no error, inexact ones one work-ulp. Use this instead of hand-picking `with_error(f, ONE)`,
    /// which loses the exactness information.
    #[inline]
    pub(crate) fn from_rounded(rounded: Rounded<FBig<mode::HalfEven, B>>) -> Self {
        match rounded {
            Approximation::Exact(v) => Self::exact(v),
            Approximation::Inexact(v, _) => Self::with_error(v, IBig::ONE),
        }
    }

    /// Wrap a value with a known error count.
    #[inline]
    pub(crate) fn with_error(mid: FBig<mode::HalfEven, B>, n: IBig) -> Self {
        Self { mid, n }
    }

    /// The exact integer `k` at precision `p` (n = 0).
    #[inline]
    pub(crate) fn exact_int(p: usize, k: IBig) -> Self {
        let mid = Context::<mode::HalfEven>::new(p)
            .convert_int::<B>(k)
            .value();
        Self::exact(mid)
    }

    /// The error of `self`, expressed in ulps of a target with leading position `lead_target` and
    /// precision `p_target`: `n · B^(lead_exp(mid) − lead_target + p_target − p_self)`, rounded up
    /// when the exponent is negative.
    ///
    /// The precision difference matters: `ulp(mid) = B^(lead_exp(mid) − p_mid)`, so converting an
    /// error from `self`'s ulp to the target's ulp shifts by both the leading-position *and* the
    /// precision difference. (Operands normally share the working precision; the exception is a
    /// value that over-delivers its context, e.g. the uncached `ln(2)` constant.)
    #[inline]
    fn term_in_ulps(&self, lead_target: isize, p_target: usize) -> IBig {
        let diff = Self::lead_exp(&self.mid) - lead_target + p_target as isize
            - self.mid.precision() as isize;
        ceil_shift::<B>(self.n.clone(), diff)
    }

    /// `self ± rhs`, rounding the midpoint to the working precision.
    pub(crate) fn add(&self, rhs: &Self) -> Self {
        let mid = &self.mid + &rhs.mid;
        let (lead_r, p_r) = (Self::lead_exp(&mid), mid.precision());
        // |mid_r − true| ≤ err_a + err_b + ½·ulp_r  ⟹  n_r = term_a + term_b + 1 (the +1 covers
        // the midpoint's own rounding).
        let n = self.term_in_ulps(lead_r, p_r) + rhs.term_in_ulps(lead_r, p_r) + IBig::ONE;
        Self { mid, n }
    }

    pub(crate) fn sub(&self, rhs: &Self) -> Self {
        let mid = &self.mid - &rhs.mid;
        let (lead_r, p_r) = (Self::lead_exp(&mid), mid.precision());
        let n = self.term_in_ulps(lead_r, p_r) + rhs.term_in_ulps(lead_r, p_r) + IBig::ONE;
        Self { mid, n }
    }

    /// The relative error of `self` as an exact rational `δ = num/den`, where
    /// `δ = err / |mid| = n·B^(d − p) / sig` (in base-`B` significand terms).
    fn rel_err(&self) -> (IBig, IBig) {
        let p = self.mid.precision();
        // `digits_ub`: an over-estimated digit count only inflates δ (a positive contribution).
        let d = self.mid.repr().digits_ub();
        let sig = self.mid.repr().significand.clone().abs();
        if d >= p {
            (shl_digits::<B>(&self.n, d - p), sig)
        } else {
            (self.n.clone(), shl_digits::<B>(&sig, p - d))
        }
    }

    /// `self / rhs`, with `rhs` mostly correct (`δb < ½`, asserted in debug builds).
    ///
    /// The quotient's relative error is `δr ≤ (δa + δb)/(1 − δb)` (exact, not first-order: the
    /// `(1 − δb)` denominator keeps `rhs`'s sign), plus the midpoint's own rounding. The radius
    /// is `n_r = ⌈m·(δa+δb)/(1−δb)⌉ + 1` where `m ≈ |mid_r|/ulp_r = sig_r·B^(p−d_r)`.
    pub(crate) fn div(&self, rhs: &Self) -> Self {
        debug_assert!(!rhs.mid.repr().significand.is_zero(), "division by a zero ball");
        let mid = &self.mid / &rhs.mid;

        // A numerator that rounds to zero cannot use the relative-error form (δa is infinite);
        // bound the absolute quotient directly: |r.true| ≤ err_a/|b.true| ≤ 2·err_a/|b.mid|.
        if self.mid.repr().significand.is_zero() {
            let (lead_r, p_r) = (Self::lead_exp(&mid), mid.precision());
            let e_b = rhs.mid.repr().exponent;
            let sig_b = rhs.mid.repr().significand.clone().abs();
            let lead_a = Self::lead_exp(&self.mid);
            let p_a = self.mid.precision();
            // n_r·ulp_r ≥ 2·n_a·B^(lead_a−p_a) / (sig_b·B^(e_b))  with ulp_r = B^(lead_r−p_r)
            let exp = lead_a - e_b - lead_r + p_r as isize - p_a as isize;
            let n2 = 2 * &self.n;
            let (num, den) = if exp >= 0 {
                (shl_digits::<B>(&n2, exp as usize), sig_b)
            } else {
                (n2, shl_digits::<B>(&sig_b, (-exp) as usize))
            };
            let n = (num + &den - IBig::ONE) / den;
            return Self { mid, n };
        }

        let p = mid.precision();
        // `digits_lb`: here `d_r` is *subtracted* (`B^(p−d_r)`), so an under-estimate only
        // inflates `m` — the safe direction.
        let d_r = mid.repr().digits_lb();
        let sig_r = mid.repr().significand.clone().abs();

        let (da_num, da_den) = self.rel_err();
        let (db_num, db_den) = rhs.rel_err();
        debug_assert!(2 * &db_num < db_den, "div denominator ball must be mostly correct");

        // |q|/ulp_r ≤ sig_r·B^(p−d_r) + 1 (the +1 covers the midpoint's half-ulp rounding).
        let m = if p >= d_r {
            shl_digits::<B>(&sig_r, p - d_r) + IBig::ONE
        } else {
            let d = shl_digits::<B>(&IBig::ONE, d_r - p);
            (sig_r + &d - IBig::ONE) / d + IBig::ONE
        };
        // S = (δa+δb)/(1−δb) = (da_num·db_den + db_num·da_den) / (da_den·(db_den − db_num)).
        let s_num = da_num * &db_den + db_num.clone() * &da_den;
        let s_den = da_den * (&db_den - &db_num);
        // n_r = ⌈m·S⌉ + 1.
        let n = (m * &s_num + &s_den - IBig::ONE) / s_den + IBig::ONE;
        Self { mid, n }
    }

    /// `self / k` with `k` an exact (possibly large) integer.
    ///
    /// The exact divisor shrinks the error by exactly `k`: `err_r ≤ err_a/k + ½·ulp_r`, so
    /// `n_r = ⌈n_a·B^(lead_a−lead_r+p_r−p_a)/k⌉ + 1`. This avoids the general [`div`](Self::div)'s
    /// big-int rational division (O(p²) — which would dominate the FBig division by a small
    /// integer and is the series' hot path).
    pub(crate) fn div_exact(&self, k: &IBig) -> Self {
        let mid = &self.mid / &FBig::<mode::HalfEven, B>::from(k.clone());
        let (lead_r, p_r) = (Self::lead_exp(&mid), mid.precision());
        let lead_a = Self::lead_exp(&self.mid);
        let p_a = self.mid.precision();
        let shift = lead_a - lead_r + p_r as isize - p_a as isize;
        // n_r = ⌈n_a·B^shift / k⌉ + 1  (ceil_shift already rounds up, so the /k re-round is sound).
        let num = ceil_shift::<B>(self.n.clone(), shift);
        let n = (num + k - IBig::ONE) / k + IBig::ONE;
        Self { mid, n }
    }

    /// `self / k` with `k` a small exact integer (the series hot path).
    #[inline]
    pub(crate) fn div_int(&self, k: usize) -> Self {
        self.div_exact(&IBig::from(k))
    }

    /// The error-count contribution of a multiplication with midpoint `mid` (see [`mul`](Self::mul)).
    fn mul_error(&self, rhs: &Self, mid: &FBig<mode::HalfEven, B>) -> IBig {
        let (lead_r, p_r) = (Self::lead_exp(mid), mid.precision());
        let e_a = self.mid.repr().exponent;
        let e_b = rhs.mid.repr().exponent;
        let p_a = self.mid.precision();
        let p_b = rhs.mid.precision();
        let lead_a = Self::lead_exp(&self.mid);
        let lead_b = Self::lead_exp(&rhs.mid);

        // |mid_r − true| ≤ err_a·|b.mid| + err_b·|a.mid| + err_a·err_b + ½·ulp_r, in ulps of r:
        // The `|n · sig|` products avoid cloning the operand significands (n ≥ 0, so the product's
        // sign is the significand's and `.abs()` is a no-op for positive values).
        let t1 = ceil_shift::<B>(
            (self.n.clone() * &rhs.mid.repr().significand).abs(),
            lead_a + e_b - lead_r + p_r as isize - p_a as isize,
        );
        let t2 = ceil_shift::<B>(
            (rhs.n.clone() * &self.mid.repr().significand).abs(),
            lead_b + e_a - lead_r + p_r as isize - p_b as isize,
        );
        let t3 = ceil_shift::<B>(
            &self.n * &rhs.n,
            lead_a + lead_b - lead_r + p_r as isize - p_a as isize - p_b as isize,
        );
        t1 + t2 + t3 + IBig::ONE
    }

    /// `self · rhs`, rounding the midpoint to the working precision.
    pub(crate) fn mul(&self, rhs: &Self) -> Self {
        let mid = &self.mid * &rhs.mid;
        let n = self.mul_error(rhs, &mid);
        Self { mid, n }
    }

    /// Like [`mul`](Self::mul) but reporting whether the multiplication rounded: an exact product
    /// of exact operands contributes no error (n = 0), which the caller needs to certify an
    /// exactly-representable result under directed rounding. `exact` is cleared when any rounding
    /// (or a propagated operand error) occurs. Propagates a range error from the multiplication.
    pub(crate) fn mul_tracking(&self, rhs: &Self, exact: &mut bool) -> Result<Self, FpError> {
        let ctx = Context::<mode::HalfEven>::new(self.mid.precision().max(rhs.mid.precision()));
        let rounded = ctx.mul(self.mid.repr(), rhs.mid.repr())?;
        let (mid, is_exact) = rounded.value_with_exact();
        if !is_exact || !self.n.is_zero() || !rhs.n.is_zero() {
            *exact = false;
        }
        let n = if *exact {
            IBig::ZERO
        } else {
            self.mul_error(rhs, &mid)
        };
        Ok(Self { mid, n })
    }

    /// Like [`scale_int`](Self::scale_int) but reporting whether the multiplication rounded: an
    /// exact product of an exact operand contributes no error (n = 0). This is the operation the
    /// `ln_compute` s<0 reduction relies on — scaling by a power of two is exact, and without this
    /// the spurious `+1` gets amplified by `rescale_precision` into a `B^precision`-sized error
    /// count that never shrinks across Ziv retries. `exact` is cleared when the multiplication
    /// rounds or the operand carries error.
    pub(crate) fn scale_int_tracking(&self, k: &IBig, exact: &mut bool) -> Self {
        let ctx = Context::<mode::HalfEven>::new(self.mid.precision());
        let k_fbig = FBig::<mode::HalfEven, B>::from(k.clone());
        // Finite mid · finite integer at the working precision stays within the exponent range
        // (unlike the squaring in `mul_tracking`), so the multiplication cannot range-error.
        let rounded = ctx
            .mul(self.mid.repr(), k_fbig.repr())
            .expect("scale_int_tracking: finite mid · finite integer cannot range-error");
        let (mid, is_exact) = rounded.value_with_exact();
        if !is_exact || !self.n.is_zero() {
            *exact = false;
        }
        let n = if *exact {
            IBig::ZERO
        } else {
            let (lead_r, p_r) = (Self::lead_exp(&mid), mid.precision());
            let lead_a = Self::lead_exp(&self.mid);
            let p_a = self.mid.precision();
            let base = ceil_shift::<B>(
                self.n.clone() * k.clone().abs(),
                lead_a - lead_r + p_r as isize - p_a as isize,
            );
            // |mid_r − true| ≤ |k|·err_a + (½·ulp_r only if the product itself rounded).
            if !is_exact {
                base + IBig::ONE
            } else {
                base
            }
        };
        Self { mid, n }
    }

    /// Like [`add`](Self::add) but reporting whether the addition rounded (see
    /// [`mul_tracking`](Self::mul_tracking)).
    pub(crate) fn add_tracking(&self, rhs: &Self, exact: &mut bool) -> Result<Self, FpError> {
        let ctx = Context::<mode::HalfEven>::new(self.mid.precision().max(rhs.mid.precision()));
        let rounded = ctx.add(self.mid.repr(), rhs.mid.repr())?;
        let (mid, is_exact) = rounded.value_with_exact();
        if !is_exact || !self.n.is_zero() || !rhs.n.is_zero() {
            *exact = false;
        }
        let n = if *exact {
            IBig::ZERO
        } else {
            let (lead_r, p_r) = (Self::lead_exp(&mid), mid.precision());
            self.term_in_ulps(lead_r, p_r) + rhs.term_in_ulps(lead_r, p_r) + IBig::ONE
        };
        Ok(Self { mid, n })
    }

    /// Like [`sqrt`](Self::sqrt) but reporting whether the root rounded (see
    /// [`mul_tracking`](Self::mul_tracking)).
    pub(crate) fn sqrt_tracking(&self, exact: &mut bool) -> Result<Self, FpError> {
        let ctx = Context::<mode::HalfEven>::new(self.mid.precision());
        let rounded = ctx.sqrt(self.mid.repr())?;
        let (mid, is_exact) = rounded.value_with_exact();
        if !is_exact || !self.n.is_zero() {
            *exact = false;
        }
        let n = if *exact || mid.repr().significand.is_zero() {
            IBig::ZERO
        } else {
            // `e_r` is the raw significand exponent (`mid_r = sig_r·B^(e_r)`), `lead_*` is the
            // leading position (`lead_exp`), so `ulp_r = B^(lead_r − p_r)` and `ulp_a = B^(lead_a − p_a)`.
            let e_r = mid.repr().exponent;
            let lead_r = Self::lead_exp(&mid);
            let p_r = mid.precision();
            let sig_r = mid.repr().significand.clone().abs();
            let lead_a = Self::lead_exp(&self.mid);
            let p_a = self.mid.precision();
            // |√(a+ε) − √a| ≤ |ε|/(2·√a) ⇒
            //   n_r·ulp_r ≥ n_a·ulp_a / (2·|mid_r|) = n_a·B^(lead_a−p_a) / (2·sig_r·B^(e_r)·B^(lead_r−p_r)),
            // so n_r = ⌈n_a·B^(lead_a−p_a−e_r−lead_r+p_r) / (2·sig_r)⌉ + 1. `e_r` is the *raw*
            // exponent, not `lead_r` — substituting `lead_r` would shift out the digits and
            // under-estimate the radius.
            let shift = lead_a - p_a as isize - e_r - lead_r + p_r as isize;
            let num = ceil_shift::<B>(self.n.clone(), shift);
            let den = 2 * sig_r;
            (num + &den - IBig::ONE) / den + IBig::ONE
        };
        Ok(Self { mid, n })
    }

    /// `self^k` for an exact integer exponent `k ≥ 2`, by left-to-right binary exponentiation
    /// (squaring chain). The compounding rounding of the chain is tracked by the multiplication
    /// rules, so the result's radius grows by the powering amplification mechanically. Returns
    /// whether the whole chain was exact (no rounding anywhere), for the exactly-representable
    /// directed-rounding case. Propagates a range error from the chain.
    pub(crate) fn pow_exact(&self, k: &UBig) -> Result<(Self, bool), FpError> {
        let nlen = k.bit_len();
        debug_assert!(nlen >= 2, "pow_exact requires k >= 2");
        let mut exact = self.n.is_zero();
        let mut res = self.mul_tracking(self, &mut exact)?;
        let mut p = nlen - 2;
        loop {
            if k.bit(p) {
                res = res.mul_tracking(self, &mut exact)?;
            }
            if p == 0 {
                break;
            }
            p -= 1;
            res = res.mul_tracking(&res, &mut exact)?;
        }
        Ok((res, exact))
    }

    /// Negation: the error count is unchanged.
    pub(crate) fn neg(self) -> Self {
        Self {
            mid: -self.mid,
            n: self.n,
        }
    }

    /// Square root, rounding the midpoint at the working precision.
    ///
    /// `|√(a+ε) − √a| ≤ |ε|/(2·√a)`, so `n_r = ⌈n_a·ulp_a / (2·|mid_r|·ulp_r)⌉ + 1` (the +1 covers
    /// the midpoint's own rounding).
    pub(crate) fn sqrt(&self) -> Self {
        let mid = self.mid.sqrt();
        if mid.repr().significand.is_zero() {
            // √0 = 0 exactly; the relative-error formula would divide by the zero significand. A
            // zero-mid ball whose true value is *exactly* zero (e.g. `asin(±1)`'s `1 − x²`, where
            // `n` may still be nonzero from the +1 rounding allowance) is handled soundly here.
            // A zero-mid ball with a genuinely nonzero true value (a cancellation like `1.049 − 1`)
            // has a nonzero true root, which this branch cannot express — no caller feeds such a
            // ball through `sqrt` today (asin/asin-adjacent check the zero significand first and
            // take the ±π/2 endpoint; the rest feed strictly positive inputs).
            return Self::exact(mid);
        }
        // `e_r` is the raw significand exponent (`mid_r = sig_r·B^(e_r)`), `lead_*` is the leading
        // position (`lead_exp`), so `ulp_r = B^(lead_r − p_r)` and `ulp_a = B^(lead_a − p_a)`.
        let e_r = mid.repr().exponent;
        let lead_r = Self::lead_exp(&mid);
        let p_r = mid.precision();
        let sig_r = mid.repr().significand.clone().abs();
        let lead_a = Self::lead_exp(&self.mid);
        let p_a = self.mid.precision();
        // |√(a+ε) − √a| ≤ |ε|/(2·√a) ⇒
        //   n_r·ulp_r ≥ n_a·ulp_a / (2·|mid_r|) = n_a·B^(lead_a−p_a) / (2·sig_r·B^(e_r)·B^(lead_r−p_r)),
        // so n_r = ⌈n_a·B^(lead_a−p_a−e_r−lead_r+p_r) / (2·sig_r)⌉ + 1. `e_r` is the *raw*
        // exponent, not `lead_r` — substituting `lead_r` would shift out the digits and
        // under-estimate the radius.
        let shift = lead_a - p_a as isize - e_r - lead_r + p_r as isize;
        let num = ceil_shift::<B>(self.n.clone(), shift);
        let den = 2 * sig_r;
        let n = (num + &den - IBig::ONE) / den + IBig::ONE;
        Self { mid, n }
    }

    /// `k · self` with `k` an exact integer.
    pub(crate) fn scale_int(&self, k: &IBig) -> Self {
        let mid = &self.mid * &FBig::<mode::HalfEven, B>::from(k.clone());
        let (lead_r, p_r) = (Self::lead_exp(&mid), mid.precision());
        let lead_a = Self::lead_exp(&self.mid);
        let p_a = self.mid.precision();
        // |mid_r − true| ≤ |k|·err_a + ½·ulp_r.
        let n = ceil_shift::<B>(
            self.n.clone() * k.clone().abs(),
            lead_a - lead_r + p_r as isize - p_a as isize,
        ) + IBig::ONE;
        Self { mid, n }
    }

    /// Exact shift by a power of the base (`mid >> s`, value unchanged, ulp scaled with it), so
    /// the error count is unchanged.
    pub(crate) fn shift(&self, s: isize) -> Self {
        Self {
            mid: self.mid.clone() >> s,
            n: self.n.clone(),
        }
    }

    /// Inflate the error count by `extra` ulps at the current midpoint's magnitude.
    pub(crate) fn inflate(&mut self, extra: &IBig) {
        self.n += extra;
    }

    /// Re-tag the midpoint to a *finer* precision (`delta > 0`): same value, ulp `B^delta` finer,
    /// so the error count scales by `B^delta`.
    pub(crate) fn rescale_precision(&mut self, delta: usize) {
        if delta > 0 {
            self.n = shl_digits::<B>(&self.n, delta);
            self.mid = FBig::new(
                self.mid.repr().clone(),
                Context::<mode::HalfEven>::new(self.mid.precision() + delta),
            );
        }
    }

    /// Re-tag the midpoint to the target mode `R` at the working precision, and convert `n` into
    /// an absolute radius `n·ulp(mid)` (exact, at unlimited precision). The ziv driver contract is
    /// identical to the old `*_compute` returns: `(value, radius)` with `|value − true| ≤ radius`.
    pub(crate) fn to_value_radius<R: crate::round::Round>(&self) -> (FBig<R, B>, FBig<R, B>) {
        let value = FBig::new(self.mid.repr().clone(), Context::<R>::new(self.mid.precision()));
        // radius = n·B^(lead_exp(mid) − p), built directly as a repr (exact at unlimited
        // precision). The exponent saturates at the range extremes: an over-wide radius is sound.
        // For n = 0 (the exact case) the radius must be a plain +0 — `Repr::new(0, isize::MIN)`
        // would otherwise survive normalization as the −∞ sentinel and poison the containment test.
        let radius_repr = if self.n.is_zero() {
            Repr::<B>::zero()
        } else {
            Repr::new(
                self.n.clone(),
                Self::lead_exp(&self.mid).saturating_sub(self.mid.precision() as isize),
            )
        };
        let radius = FBig::new(radius_repr, Context::<R>::new(0));
        (value, radius)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::round::mode;
    use dashu_base::Abs;

    type F = FBig<mode::HalfEven, 10>;
    type B10 = Ball<10>;

    /// Build a ball with midpoint `sig·B^exp` (at precision `p`) and error count `n`, whose
    /// *true* value is `true_sig·B^true_exp` (exact, unlimited precision). The caller must pick
    /// `true_*` within `n·ulp(mid)` of `mid`.
    fn ball(sig: i128, exp: isize, p: usize, n: i128, true_sig: i128, true_exp: isize) -> (B10, F) {
        let mid = F::from_parts(IBig::from(sig), exp)
            .with_precision(p)
            .value();
        let ball = B10::with_error(mid, IBig::from(n));
        let true_val = F::from_parts(IBig::from(true_sig), true_exp); // unlimited precision
        assert!(
            (ball.mid.clone().with_precision(0).value() - true_val.clone()).abs()
                <= ball.mid.ulp().with_precision(0).value() * IBig::from(n),
            "test setup: true value must lie within n·ulp of mid"
        );
        (ball, true_val)
    }

    /// Check the invariant `|mid − true| ≤ n·ulp(mid)` using exact (unlimited precision) arithmetic.
    fn assert_invariant(ball: &B10, true_val: &F) {
        let mid_unlim = ball.mid.clone().with_precision(0).value();
        let diff = (mid_unlim - true_val).abs();
        let bound = F::from(ball.n.clone()) * ball.mid.ulp().with_precision(0).value();
        assert!(
            diff <= bound,
            "|mid − true| = {diff} > n·ulp = {bound} (mid = {}, n = {}, true = {})",
            ball.mid,
            ball.n,
            true_val
        );
    }

    #[test]
    fn add() {
        let (a, ta) = ball(123, -2, 3, 2, 125, -2); // mid 1.23 ± 0.02, true 1.25
        let (b, tb) = ball(456, -2, 3, 1, 4555, -3); // mid 4.56 ± 0.01, true 4.555
        let r = a.add(&b);
        assert_invariant(&r, &(&ta + &tb).with_precision(0).value());
    }

    #[test]
    fn add_cancellation() {
        // 1.00 − 1.00 = 0: the error must stay referenced to the pre-cancellation magnitude.
        let (a, ta) = ball(100, -2, 3, 5, 1049, -3); // mid 1.00 ± 0.05, true 1.049
        let (b, tb) = ball(100, -2, 3, 0, 100, -2); // mid 1.00 exact
        let r = a.sub(&b);
        assert_invariant(&r, &(&ta - &tb).with_precision(0).value());
        // The radius must be ≫ ulp(0): the pre-cancellation error survives the subtraction.
        let radius = r.mid.ulp().with_precision(0).value() * F::from(r.n.clone());
        assert!(radius > F::from_parts(IBig::from(1), -4));
    }

    #[test]
    fn mul() {
        let (a, ta) = ball(15000, -4, 4, 1, 15001, -4); // mid 1.5000 ± 0.0001, true 1.5001
        let (b, tb) = ball(20000, -4, 4, 1, 19999, -4); // mid 2.0000 ± 0.0001, true 1.9999
        let r = a.mul(&b);
        assert_invariant(&r, &(&ta * &tb).with_precision(0).value());
    }

    #[test]
    fn div() {
        let (a, ta) = ball(10000, -4, 4, 1, 10001, -4); // mid 1.0000 ± 0.0001, true 1.0001
        let (b, tb) = ball(20000, -4, 4, 0, 20000, -4); // mid 2.0000 exact
        let r = a.div(&b);
        assert_invariant(&r, &(&ta / &tb).with_precision(0).value());
    }

    #[test]
    fn div_zero_numerator() {
        // A numerator that rounds to zero must still bound the quotient.
        let (a, ta) = ball(0, -4, 4, 3, 3, -4); // mid 0 ± 3·ulp(0), true 3e-4
        let (b, tb) = ball(20000, -4, 4, 0, 20000, -4); // mid 2.0000 exact
        let r = a.div(&b);
        assert_invariant(&r, &(&ta / &tb).with_precision(0).value());
    }

    #[test]
    fn sqrt_bounds_error() {
        // Regression: the error shift used the leading position `lead_r` (= e_r + digits) where
        // the raw significand exponent `e_r` belongs, so the radius under-estimated (the Ball
        // invariant broke even for a small input error count). The true root is computed at
        // precision 60 and widened.
        let (a, ta) = ball(14400, -4, 5, 5, 14405, -4); // mid 1.4400 ± 5·ulp, true 1.4405
        let r = a.sqrt();
        let sqrt_true = ta
            .with_precision(60)
            .value()
            .sqrt()
            .with_precision(0)
            .value();
        assert_invariant(&r, &sqrt_true);

        // A larger input error must amplify the radius correspondingly.
        let (a2, ta2) = ball(14400, -4, 5, 90, 14490, -4); // mid 1.4400 ± 90·ulp, true 1.4490
        let r2 = a2.sqrt();
        let sqrt_true2 = ta2
            .with_precision(60)
            .value()
            .sqrt()
            .with_precision(0)
            .value();
        assert_invariant(&r2, &sqrt_true2);
    }

    #[test]
    fn scale_int() {
        let (a, ta) = ball(10000, -4, 4, 1, 10001, -4); // mid 1.0000 ± 0.0001, true 1.0001
        let r = a.scale_int(&IBig::from(3));
        assert_invariant(
            &r,
            &(&ta * F::from_parts(IBig::from(3), 0))
                .with_precision(0)
                .value(),
        );
    }

    #[test]
    fn scale_int_tracking_exact() {
        // An exactly-representable operand scaled by an exactly-representable integer keeps n = 0.
        // This is the `ln_compute` s<0 reduction: without it the spurious `+1` is amplified by
        // `rescale_precision` into a `B^precision`-sized error count (the powf-of-base-<1 hang).
        // Precision 10: 0.2668·4 = 1.0672 has 5 digits, well within the working precision, so the
        // scaling is exact (in ln_compute the input is rounded to the work precision first, far
        // above the input's own digit count).
        let a = B10::exact(
            F::from_parts(IBig::from(2668), -4)
                .with_precision(10)
                .value(),
        ); // 0.2668, n=0
        let mut exact = true;
        let r = a.scale_int_tracking(&IBig::from(4), &mut exact);
        assert!(exact, "exact scaling of an exact operand stays exact");
        assert_eq!(r.n, IBig::ZERO, "exact scaling keeps n = 0, got n = {}", r.n);
        assert_eq!(
            r.mid,
            F::from_parts(IBig::from(10672), -4)
                .with_precision(10)
                .value()
        );
    }

    #[test]
    fn scale_int_tracking_inexact_operand() {
        // An inexact operand scaled exactly: the operand error propagates (no +1 for the exact
        // product's rounding), and `exact` is cleared.
        let (a, ta) = ball(10000, -4, 4, 1, 10001, -4); // mid 1.0000 ± 0.0001
        let mut exact = true;
        let r = a.scale_int_tracking(&IBig::from(3), &mut exact);
        assert!(!exact, "inexact operand clears the exact flag");
        assert_eq!(r.n, IBig::from(3), "3·(1 ulp) propagates exactly, no +1: got n = {}", r.n);
        assert_invariant(
            &r,
            &(&ta * F::from_parts(IBig::from(3), 0))
                .with_precision(0)
                .value(),
        );
    }

    #[test]
    fn shift_preserves_error() {
        // Exact shift by a power of the base: n unchanged, ulp scaled.
        let (a, ta) = ball(123, -2, 3, 2, 125, -2);
        let r = a.shift(3);
        assert_invariant(&r, &(ta >> 3).with_precision(0).value());
    }

    #[test]
    fn rescale_precision_scales_error() {
        let (a, ta) = ball(123, -2, 3, 2, 125, -2);
        let mut r = a;
        r.rescale_precision(2);
        assert_invariant(&r, &ta);
    }

    #[test]
    fn to_value_radius_returns_sound_bounds() {
        let (a, ta) = ball(123, -2, 3, 2, 125, -2);
        let (value, radius) = a.to_value_radius::<mode::HalfEven>();
        let diff = (value - ta).abs();
        assert!(diff <= radius, "|value − true| = {diff} > radius = {radius}");
    }
}
