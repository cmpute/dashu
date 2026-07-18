//! The Ziv retry loop — guaranteed-correct rounding for transcendentals.
//!
//! Transcendentals (`exp`, `ln`, …) cannot compute an exact result the way arithmetic can,
//! so they approximate at a working precision `p + guard` and round down. A single guard-digit
//! heuristic is only *near*-correct: for an input whose true value sits on a rounding boundary,
//! the rounded result can be off by one ULP. The Ziv loop closes that gap.
//!
//! Each transcendental reports its approximation together with a provable absolute error radius
//! `E` (a true upper bound on `|approx − true|`, built from the fact that every `+`/`-`/`*`/`/`
//! in the algorithm is itself correctly rounded). The driver rounds the approximation to the
//! target precision and checks whether the entire error interval `[ã − E, ã + E]` lies inside
//! the candidate's [`ErrorBounds`] preimage — the set of reals that round to it. If it does,
//! the rounded value is *guaranteed* correct; otherwise the driver retries with more guard
//! digits. The loop provably terminates (a true tie is resolved deterministically by the mode),
//! with a large sanity cap as an unreachable backstop.

use dashu_base::Approximation::*;

use crate::{fbig::FBig, repr::Context, round::ErrorBounds, round::Rounded};
use dashu_int::Word;

/// Maximum number of Ziv retries before falling back to the best-effort rounded value.
///
/// This is a sanity backstop only — the loop converges as soon as the working precision is
/// large enough that the error interval no longer straddles a rounding boundary, which happens
/// in one attempt for essentially all inputs (the guard-digit heuristic is sized for that) and
/// in a handful of attempts only for inputs pathological close to a tie. The cap exists so a
/// bug in an error-radius bound can never produce an infinite loop.
const MAX_ZIV_RETRIES: usize = 32;

// A test-only retry counter, so tests can assert the loop converges on the first attempt for
// typical inputs (validating that the guard-digit heuristic wasn't over-tightened). Reads as
// the number of *extra* attempts beyond the first, i.e. `0` means first-attempt success.
#[cfg(test)]
thread_local! {
    pub(crate) static LAST_ZIV_RETRIES: core::cell::Cell<usize> = const { core::cell::Cell::new(0) };
}

impl<R: ErrorBounds> Context<R> {
    /// Correctly round a transcendental approximation to this context's precision using a Ziv
    /// retry loop.
    ///
    /// `approx(guard)` computes the function at working precision `self.precision + guard` and
    /// returns `(value, error_radius)` — the value and a provable upper bound on its absolute
    /// error, both as [`FBig`]s at the working context. The closure is expected to capture and
    /// reborrow any [`ConstCache`](crate::ConstCache) from the enclosing scope; the driver calls
    /// it once per attempt and grows `guard` when the result cannot be certified.
    ///
    /// The loop preserves the [`Exact`](Rounded)/[`Inexact`](Rounded) flag from rounding the
    /// approximation to the target precision.
    pub(crate) fn ziv<const B: Word>(
        &self,
        initial_guard: usize,
        mut approx: impl FnMut(usize) -> (FBig<R, B>, FBig<R, B>),
    ) -> Rounded<FBig<R, B>> {
        // Unlimited precision: the approximation is exact, so report it as-is.
        if !self.is_limited() {
            let (value, _err) = approx(0);
            return Exact(value);
        }

        let mut guard = initial_guard;
        let mut last = None;
        #[cfg(test)]
        LAST_ZIV_RETRIES.with(|c| c.set(0));
        for _ in 0..MAX_ZIV_RETRIES {
            let (a, e) = approx(guard);
            // `with_precision` consumes `a`, but the containment test still needs it, so round a
            // clone and keep the original for the interval check.
            let candidate = a.clone().with_precision(self.precision);
            if Self::contained::<B>(&a, &e, candidate.value_ref()) {
                return candidate;
            }
            last = Some(candidate);

            // Grow the guard aggressively so a near-tie resolves in a couple of retries, while
            // the first attempt (with the heuristic guard) handles the common case.
            let step = core::cmp::max(guard, self.precision / 2).max(1);
            guard += step;
            #[cfg(test)]
            LAST_ZIV_RETRIES.with(|c| c.set(c.get() + 1));
        }

        // Unreachable in practice: return the best-effort candidate from the last attempt,
        // matching the pre-Ziv near-correct behavior rather than looping forever.
        last.expect("MAX_ZIV_RETRIES is non-zero")
    }

    /// Containment test: is the approximation's error interval `[a − e, a + e]` entirely inside
    /// the rounding preimage of `y` (every real in `[y − L, y + R]` rounds to `y` under `R`)?
    ///
    /// The arithmetic is done at unlimited precision (an exact no-op promotion via
    /// [`FBig::with_precision`](crate::FBig)`(0)`), so the comparison cannot lose a guard digit
    /// and mis-decide — a soundness requirement, since a wrong decision here yields a wrong ULP.
    fn contained<const B: Word>(a: &FBig<R, B>, e: &FBig<R, B>, y: &FBig<R, B>) -> bool {
        let (lb, rb, incl_l, incl_r) = R::error_bounds::<B>(y);

        // Promote to unlimited precision so the interval arithmetic is exact.
        let a = a.clone().with_precision(0).value();
        let e = e.clone().with_precision(0).value();
        let y = y.clone().with_precision(0).value();
        let lb = lb.with_precision(0).value();
        let rb = rb.with_precision(0).value();

        // [a − e, a + e] ⊆ [y − lb, y + rb], respecting each endpoint's inclusivity.
        let lo = &a - &e; // a − e
        let hi = &a + &e; // a + e
        let pre_lo = &y - &lb; // y − L
        let pre_hi = &y + &rb; // y + R
        let left_ok = if incl_l { lo >= pre_lo } else { lo > pre_lo };
        let right_ok = if incl_r { hi <= pre_hi } else { hi < pre_hi };
        left_ok && right_ok
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::round::mode;

    type F = crate::FBig<mode::HalfEven>;

    // An exact approximation (radius 0) is accepted on the first attempt as Exact.
    #[test]
    fn ziv_accepts_exact_first_attempt() {
        let ctx: Context<mode::HalfEven> = Context::new(10);
        LAST_ZIV_RETRIES.with(|c| c.set(usize::MAX));
        let r = ctx.ziv(4, |_| (F::ONE, F::ZERO));
        assert!(matches!(r, Exact(_)));
        assert_eq!(LAST_ZIV_RETRIES.with(|c| c.get()), 0);
    }

    // An approximation whose error interval straddles a rounding boundary must retry; once the
    // shrinking radius makes the interval unambiguous, ziv accepts.
    #[test]
    fn ziv_retries_until_contained() {
        let ctx: Context<mode::HalfEven> = Context::new(4);
        let r = ctx.ziv(2, |guard| {
            // value 1.0, radius 2^(-guard): large on the first attempt, tiny later.
            (F::ONE, F::ONE >> guard as isize)
        });
        let _ = r.value();
        assert!(LAST_ZIV_RETRIES.with(|c| c.get()) >= 1);
    }

    // Unlimited-precision context short-circuits to a single Exact call (counter untouched).
    #[test]
    fn ziv_unlimited_short_circuits() {
        let ctx: Context<mode::HalfEven> = Context::new(0);
        LAST_ZIV_RETRIES.with(|c| c.set(usize::MAX));
        let r = ctx.ziv(4, |_| (F::from(7u8), F::ZERO));
        assert!(matches!(r, Exact(_)));
        assert_eq!(LAST_ZIV_RETRIES.with(|c| c.get()), usize::MAX);
    }

    // The guard-digit heuristic should let exp/ln converge in at most one retry for typical
    // inputs. A single retry on a near-tie is by design (Ziv certifies correctness; the guard only
    // controls the first-attempt hit rate). This catches gross guard mis-sizing (many retries),
    // not the occasional near-tie retry.
    #[test]
    fn ziv_few_retries_for_typical_inputs() {
        let cases = [
            F::try_from(0.5f64).unwrap(),
            F::try_from(1.5f64).unwrap(),
            F::try_from(2.0f64).unwrap(),
            F::try_from(10.0f64).unwrap(),
            F::try_from(1000.0f64).unwrap(),
            F::try_from(1e-6f64).unwrap(),
        ];
        const MAX_RETRIES: usize = 1;
        for p in [10usize, 24, 53, 100, 200] {
            for x in &cases {
                let x = x.clone().with_precision(p).value();
                LAST_ZIV_RETRIES.with(|c| c.set(usize::MAX));
                let _ = x.ln();
                let ln_retries = LAST_ZIV_RETRIES.with(|c| c.get());
                assert!(
                    ln_retries <= MAX_RETRIES,
                    "ln({x}) at p={p} took {ln_retries} retries (expected <= {MAX_RETRIES})"
                );
                LAST_ZIV_RETRIES.with(|c| c.set(usize::MAX));
                let _ = x.exp();
                let exp_retries = LAST_ZIV_RETRIES.with(|c| c.get());
                assert!(
                    exp_retries <= MAX_RETRIES,
                    "exp({x}) at p={p} took {exp_retries} retries (expected <= {MAX_RETRIES})"
                );
            }
        }
    }
}
