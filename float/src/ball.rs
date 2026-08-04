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

use dashu_base::Abs;
use dashu_int::IBig;

use crate::{
    fbig::FBig,
    repr::{Context, Repr, Word},
    round::mode,
};

/// `x · B^exp` when `exp ≥ 0`; `⌈x / B^(−exp)⌉` when `exp < 0`.
///
/// The rounding is always *up* (never down), keeping the result a sound upper bound for the
/// underlying rational — the propagation rules below rely on this monotonicity.
#[inline]
fn ceil_shift<const B: Word>(x: IBig, exp: isize) -> IBig {
    if exp >= 0 {
        x * Repr::<B>::BASE.pow(exp as usize)
    } else {
        let d = Repr::<B>::BASE.pow((-exp) as usize);
        (x + &d - IBig::ONE) / d
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
    #[inline]
    fn lead_exp(mid: &FBig<mode::HalfEven, B>) -> isize {
        mid.repr().exponent + mid.repr().digits() as isize
    }

    /// Wrap an exactly-represented value (n = 0).
    #[inline]
    pub(crate) fn exact(mid: FBig<mode::HalfEven, B>) -> Self {
        Self { mid, n: IBig::ZERO }
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

    /// The error of `self`, expressed in ulps of a target with leading position `e_target` and
    /// precision `p_target`: `n · B^(E(self) − e_target + p_target − p_self)`, rounded up when the
    /// exponent is negative.
    ///
    /// The precision difference matters: `ulp(mid) = B^(E(mid) − p_mid)`, so converting an error
    /// from `self`'s ulp to the target's ulp shifts by both the leading-position *and* the
    /// precision difference. (Operands normally share the working precision; the exception is a
    /// value that over-delivers its context, e.g. the uncached `ln(2)` constant.)
    #[inline]
    fn term_in_ulps(&self, e_target: isize, p_target: usize) -> IBig {
        let diff = Self::lead_exp(&self.mid) - e_target + p_target as isize
            - self.mid.precision() as isize;
        ceil_shift::<B>(self.n.clone(), diff)
    }

    /// `self ± rhs`, rounding the midpoint to the working precision.
    pub(crate) fn add(&self, rhs: &Self) -> Self {
        let mid = &self.mid + &rhs.mid;
        let (e_r, p_r) = (Self::lead_exp(&mid), mid.precision());
        // |mid_r − true| ≤ err_a + err_b + ½·ulp_r  ⟹  n_r = term_a + term_b + 1 (the +1 covers
        // the midpoint's own rounding).
        let n = self.term_in_ulps(e_r, p_r) + rhs.term_in_ulps(e_r, p_r) + IBig::ONE;
        Self { mid, n }
    }

    pub(crate) fn sub(&self, rhs: &Self) -> Self {
        let mid = &self.mid - &rhs.mid;
        let (e_r, p_r) = (Self::lead_exp(&mid), mid.precision());
        let n = self.term_in_ulps(e_r, p_r) + rhs.term_in_ulps(e_r, p_r) + IBig::ONE;
        Self { mid, n }
    }

    /// The relative error of `self` as an exact rational `δ = num/den`, where
    /// `δ = err / |mid| = n·B^(d − p) / sig` (in base-`B` significand terms).
    fn rel_err(&self) -> (IBig, IBig) {
        let p = self.mid.precision();
        let d = self.mid.repr().digits();
        let sig = self.mid.repr().significand.clone().abs();
        if d >= p {
            (self.n.clone() * Repr::<B>::BASE.pow(d - p), sig)
        } else {
            (self.n.clone(), sig * Repr::<B>::BASE.pow(p - d))
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
            let (e_r, p_r) = (Self::lead_exp(&mid), mid.precision());
            let e_b = rhs.mid.repr().exponent;
            let sig_b = rhs.mid.repr().significand.clone().abs();
            // n_r·ulp_r ≥ 2·n_a·B^(E(a)−p_a) / (sig_b·B^(e_b))  with ulp_r = B^(E(r)−p_r)
            let exp = Self::lead_exp(&self.mid) - e_b - e_r + p_r as isize
                - self.mid.precision() as isize;
            let (num, den) = if exp >= 0 {
                (2 * &self.n * Repr::<B>::BASE.pow(exp as usize), sig_b)
            } else {
                (2 * &self.n, sig_b * Repr::<B>::BASE.pow((-exp) as usize))
            };
            let n = (num + &den - IBig::ONE) / den;
            return Self { mid, n };
        }

        let p = mid.precision();
        let d_r = mid.repr().digits();
        let sig_r = mid.repr().significand.clone().abs();

        let (da_num, da_den) = self.rel_err();
        let (db_num, db_den) = rhs.rel_err();
        debug_assert!(2 * &db_num < db_den, "div denominator ball must be mostly correct");

        // |q|/ulp_r ≤ sig_r·B^(p−d_r) + 1 (the +1 covers the midpoint's half-ulp rounding).
        let m = if p >= d_r {
            sig_r * Repr::<B>::BASE.pow(p - d_r) + IBig::ONE
        } else {
            let d = Repr::<B>::BASE.pow(d_r - p);
            (sig_r + &d - IBig::ONE) / d + IBig::ONE
        };
        // S = (δa+δb)/(1−δb) = (da_num·db_den + db_num·da_den) / (da_den·(db_den − db_num)).
        let s_num = da_num * &db_den + db_num.clone() * &da_den;
        let s_den = da_den * (&db_den - &db_num);
        // n_r = ⌈m·S⌉ + 1.
        let n = (m * &s_num + &s_den - IBig::ONE) / s_den + IBig::ONE;
        Self { mid, n }
    }

    /// `self / k` with `k` an exact integer.
    pub(crate) fn div_int(&self, k: usize) -> Self {
        let k = Self::exact_int(self.mid.precision(), IBig::from(k));
        self.div(&k)
    }

    /// `self · rhs`, rounding the midpoint to the working precision.
    pub(crate) fn mul(&self, rhs: &Self) -> Self {
        let mid = &self.mid * &rhs.mid;
        let (e_r, p_r) = (Self::lead_exp(&mid), mid.precision());
        let e_a = self.mid.repr().exponent;
        let e_b = rhs.mid.repr().exponent;
        let sig_a = self.mid.repr().significand.clone().abs();
        let sig_b = rhs.mid.repr().significand.clone().abs();
        let p_a = self.mid.precision();
        let p_b = rhs.mid.precision();

        // |mid_r − true| ≤ err_a·|b.mid| + err_b·|a.mid| + err_a·err_b + ½·ulp_r, in ulps of r:
        let t1 = ceil_shift::<B>(
            &self.n * sig_b,
            Self::lead_exp(&self.mid) + e_b - e_r + p_r as isize - p_a as isize,
        );
        let t2 = ceil_shift::<B>(
            &rhs.n * sig_a,
            Self::lead_exp(&rhs.mid) + e_a - e_r + p_r as isize - p_b as isize,
        );
        let t3 = ceil_shift::<B>(
            &self.n * &rhs.n,
            Self::lead_exp(&self.mid) + Self::lead_exp(&rhs.mid) - e_r + p_r as isize
                - p_a as isize
                - p_b as isize,
        );
        let n = t1 + t2 + t3 + IBig::ONE;
        Self { mid, n }
    }

    /// `k · self` with `k` an exact integer.
    pub(crate) fn scale_int(&self, k: &IBig) -> Self {
        let mid = &self.mid * &FBig::<mode::HalfEven, B>::from(k.clone());
        let (e_r, p_r) = (Self::lead_exp(&mid), mid.precision());
        // |mid_r − true| ≤ |k|·err_a + ½·ulp_r.
        let n = ceil_shift::<B>(
            self.n.clone() * k.clone().abs(),
            Self::lead_exp(&self.mid) - e_r + p_r as isize - self.mid.precision() as isize,
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
            self.n *= Repr::<B>::BASE.pow(delta);
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
        // radius = n·B^(E(mid) − p), built directly as a repr (exact at unlimited precision).
        let radius = FBig::new(
            Repr::new(self.n.clone(), Self::lead_exp(&self.mid) - self.mid.precision() as isize),
            Context::<R>::new(0),
        );
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
