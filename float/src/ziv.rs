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

use core::cmp::Ordering;

use dashu_base::Approximation::*;

use crate::{
    error::{FpError, FpResult},
    fbig::FBig,
    repr::{Context, Repr},
    round::ErrorBounds,
};
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
#[cfg(any(all(test, feature = "std"), feature = "profiling"))]
thread_local! {
    pub(crate) static LAST_ZIV_RETRIES: core::cell::Cell<usize> = const { core::cell::Cell::new(0) };
}

// Reset/bump the retry counter, with no-op fallbacks when the `profiling` feature (or test mode)
// is absent — the Ziv loop body stays clean.
#[cfg(any(all(test, feature = "std"), feature = "profiling"))]
fn ziv_retries_reset_impl() {
    LAST_ZIV_RETRIES.with(|c| c.set(0));
}
#[cfg(not(any(all(test, feature = "std"), feature = "profiling")))]
fn ziv_retries_reset_impl() {}

#[cfg(any(all(test, feature = "std"), feature = "profiling"))]
fn ziv_retries_bump() {
    LAST_ZIV_RETRIES.with(|c| c.set(c.get().saturating_add(1)));
}
#[cfg(not(any(all(test, feature = "std"), feature = "profiling")))]
fn ziv_retries_bump() {}

/// Number of *extra* Ziv attempts beyond the first in the most recent Ziv loop (0 = first-attempt
/// success). Profiling only — available when the `profiling` feature (or `cfg(test)`) is enabled.
#[cfg(any(all(test, feature = "std"), feature = "profiling"))]
pub fn ziv_retries() -> usize {
    LAST_ZIV_RETRIES.with(|c| c.get())
}

/// Reset the retry counter to 0 (before a measurement, so exact short-circuits that never enter a
/// Ziv loop report 0 rather than the previous call's count).
#[cfg(any(all(test, feature = "std"), feature = "profiling"))]
pub fn ziv_retries_reset() {
    ziv_retries_reset_impl();
}

impl<R: ErrorBounds> Context<R> {
    /// Correctly round a transcendental approximation to this context's precision using a Ziv
    /// retry loop.
    ///
    /// `approx(guard)` computes the function at working precision `self.precision + guard` and
    /// returns `Ok((value, error_radius))` — the value and a provable upper bound on its absolute
    /// error, both as [`FBig`]s at the working context — or an [`FpError`] when the computation
    /// over/underflows the finite range mid-attempt. The driver propagates that error immediately
    /// (on the first overflowing attempt), so a closure that can detect overflow *as it computes*
    /// (e.g. `powi`'s squaring chain, whose `sqr`/`mul` hit the `±isize::MAX` exponent sentinel)
    /// need not pre-probe; a closure that cannot overflow simply never returns `Err`. The closure is
    /// expected to capture and reborrow any [`ConstCache`](crate::ConstCache) from the enclosing
    /// scope; the driver calls it once per attempt and grows `guard` when the result cannot be
    /// certified.
    ///
    /// The loop preserves the [`Exact`](Rounded)/[`Inexact`](Rounded) flag from rounding the
    /// approximation to the target precision.
    pub(crate) fn ziv<const B: Word>(
        &self,
        initial_guard: usize,
        mut approx: impl FnMut(usize) -> Result<(FBig<R, B>, FBig<R, B>), FpError>,
    ) -> FpResult<FBig<R, B>> {
        // Unlimited precision: the approximation is exact, so report it as-is.
        if !self.is_limited() {
            let (value, _err) = approx(0)?;
            return Ok(Exact(value));
        }

        let mut guard = initial_guard;
        let mut last = None;
        ziv_retries_reset_impl();
        for _ in 0..MAX_ZIV_RETRIES {
            let (a, e) = approx(guard)?;
            // `with_precision` consumes `a`, but the containment test still needs it, so round a
            // clone and keep the original for the interval check.
            let candidate = a.clone().with_precision(self.precision);
            if Self::contained::<B>(&a.repr, &e.repr, candidate.value_ref()) {
                return Ok(candidate);
            }
            last = Some(candidate);

            // Grow the guard aggressively so a near-tie resolves in a couple of retries, while
            // the first attempt (with the heuristic guard) handles the common case.
            let step = core::cmp::max(guard, self.precision / 2).max(1);
            guard += step;
            ziv_retries_bump();
        }

        // Unreachable in practice: return the best-effort candidate from the last attempt,
        // matching the pre-Ziv near-correct behavior rather than looping forever.
        Ok(last.expect("MAX_ZIV_RETRIES is non-zero"))
    }

    /// Pair variant of [`ziv`](Self::ziv) for functions that return two values (e.g. `sin_cos`,
    /// `sinh_cosh`). `approx(guard)` returns `Ok(((v1, e1), (v2, e2)))` — both values and their
    /// provable radii at the working context, sharing whatever computation is common — or an
    /// [`FpError`], propagated to both slots as `(Err(e), Err(e))`. The driver certifies **both**
    /// values: it retries while *either* containment test fails, and returns both only when both fit
    /// their rounding preimages. Shares the guard-growth loop and retry counter with [`ziv`](Self::ziv).
    pub(crate) fn ziv_pair<const B: Word>(
        &self,
        initial_guard: usize,
        mut approx: impl FnMut(
            usize,
        )
            -> Result<((FBig<R, B>, FBig<R, B>), (FBig<R, B>, FBig<R, B>)), FpError>,
    ) -> (FpResult<FBig<R, B>>, FpResult<FBig<R, B>>) {
        // Unlimited precision: both approximations are exact, report them as-is.
        if !self.is_limited() {
            let ((v1, _), (v2, _)) = match approx(0) {
                Ok(v) => v,
                Err(e) => return (Err(e), Err(e)),
            };
            return (Ok(Exact(v1)), Ok(Exact(v2)));
        }

        let mut guard = initial_guard;
        let mut last = None;
        ziv_retries_reset_impl();
        for _ in 0..MAX_ZIV_RETRIES {
            let ((a1, e1), (a2, e2)) = match approx(guard) {
                Ok(v) => v,
                Err(e) => return (Err(e), Err(e)),
            };
            let c1 = a1.clone().with_precision(self.precision);
            let c2 = a2.clone().with_precision(self.precision);
            if Self::contained::<B>(&a1.repr, &e1.repr, c1.value_ref())
                && Self::contained::<B>(&a2.repr, &e2.repr, c2.value_ref())
            {
                return (Ok(c1), Ok(c2));
            }
            last = Some((c1, c2));

            // Grow the guard aggressively so a near-tie resolves in a couple of retries.
            let step = core::cmp::max(guard, self.precision / 2).max(1);
            guard += step;
            ziv_retries_bump();
        }

        // Unreachable in practice: return the best-effort pair from the last attempt.
        let (l1, l2) = last.expect("MAX_ZIV_RETRIES is non-zero");
        (Ok(l1), Ok(l2))
    }

    /// Containment test: is the approximation's error interval `[a − e, a + e]` entirely inside
    /// the rounding preimage of `y` (every real in `[y − lb, y + rb]` rounds to `y` under `R`)?
    ///
    /// `a` and `e` are the working-precision approximation and its provable error radius; `y` is
    /// the candidate rounded to the target precision (kept as an [`FBig`] only because
    /// [`ErrorBounds::error_bounds`] is defined on [`FBig`]). The interval arithmetic runs on the
    /// raw [`Repr`]s, which carry no precision limit, so the additions are lossless — there is no
    /// rounding that could drop a guard digit and mis-decide (a wrong call here yields a wrong
    /// ULP). The old path promoted every value to unlimited precision via `with_precision(0)`;
    /// the [`Repr`]s are already exact, so that was a chain of no-op clones, now removed.
    ///
    /// The test compares sums rather than differences — algebraically identical for exact
    /// arithmetic, and it reads as a single shared inequality per endpoint:
    ///   `a − e ≥ y − lb  ⟺  a + lb ≥ y + e`
    ///   `a + e ≤ y + rb  ⟺  y + rb ≥ a + e`
    fn contained<const B: Word>(a: &Repr<B>, e: &Repr<B>, y: &FBig<R, B>) -> bool {
        let (lb, rb, incl_l, incl_r) = R::error_bounds::<B>(y);

        let y = &y.repr;
        let lb = lb.into_repr();
        let rb = rb.into_repr();

        let left = (a + &lb).cmp(&(y + e));
        let right = (y + &rb).cmp(&(a + e));
        let left_ok = if incl_l {
            left != Ordering::Less
        } else {
            left == Ordering::Greater
        };
        let right_ok = if incl_r {
            right != Ordering::Less
        } else {
            right == Ordering::Greater
        };
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
        let r = ctx.ziv(4, |_| Ok((F::ONE, F::ZERO)));
        assert!(matches!(r, Ok(Exact(_))));
        assert_eq!(LAST_ZIV_RETRIES.with(|c| c.get()), 0);
    }

    // An approximation whose error interval straddles a rounding boundary must retry; once the
    // shrinking radius makes the interval unambiguous, ziv accepts.
    #[test]
    fn ziv_retries_until_contained() {
        let ctx: Context<mode::HalfEven> = Context::new(4);
        let r = ctx.ziv(2, |guard| {
            // value 1.0, radius 2^(-guard): large on the first attempt, tiny later.
            Ok((F::ONE, F::ONE >> guard as isize))
        });
        let _ = r.unwrap().value();
        assert!(LAST_ZIV_RETRIES.with(|c| c.get()) >= 1);
    }

    // Unlimited-precision context short-circuits to a single Exact call (counter untouched).
    #[test]
    fn ziv_unlimited_short_circuits() {
        let ctx: Context<mode::HalfEven> = Context::new(0);
        LAST_ZIV_RETRIES.with(|c| c.set(usize::MAX));
        let r = ctx.ziv(4, |_| Ok((F::from(7u8), F::ZERO)));
        assert!(matches!(r, Ok(Exact(_))));
        assert_eq!(LAST_ZIV_RETRIES.with(|c| c.get()), usize::MAX);
    }

    // A closure that overflows on the first attempt propagates the error immediately (no retries).
    #[test]
    fn ziv_propagates_closure_error() {
        let ctx: Context<mode::HalfEven> = Context::new(10);
        let r = ctx.ziv::<2>(4, |_| Err(FpError::OutOfDomain));
        assert_eq!(r, Err(FpError::OutOfDomain));
    }

    // ziv_pair accepts an exact pair (both radii 0) on the first attempt.
    #[test]
    fn ziv_pair_accepts_exact_first_attempt() {
        let ctx: Context<mode::HalfEven> = Context::new(10);
        LAST_ZIV_RETRIES.with(|c| c.set(usize::MAX));
        let (r1, r2) = ctx.ziv_pair(4, |_| Ok(((F::ONE, F::ZERO), (F::from(2u8), F::ZERO))));
        assert!(matches!(r1, Ok(Exact(_))));
        assert!(matches!(r2, Ok(Exact(_))));
        assert_eq!(LAST_ZIV_RETRIES.with(|c| c.get()), 0);
    }

    // ziv_pair retries while *either* value's interval straddles a boundary; here the second value
    // carries the shrinking radius, so the pair must retry together.
    #[test]
    fn ziv_pair_retries_until_both_contained() {
        let ctx: Context<mode::HalfEven> = Context::new(4);
        let (r1, r2) = ctx.ziv_pair(2, |guard| {
            let radius = F::ONE >> guard as isize;
            Ok(((F::ONE, F::ZERO), (F::ONE, radius)))
        });
        let _ = (r1.unwrap().value(), r2.unwrap().value());
        assert!(LAST_ZIV_RETRIES.with(|c| c.get()) >= 1);
    }

    // ziv_pair propagates a closure error to *both* slots on the first overflowing attempt.
    #[test]
    fn ziv_pair_propagates_closure_error() {
        let ctx: Context<mode::HalfEven> = Context::new(10);
        let (r1, r2) = ctx.ziv_pair::<2>(4, |_| Err(FpError::OutOfDomain));
        assert_eq!(r1, Err(FpError::OutOfDomain));
        assert_eq!(r2, Err(FpError::OutOfDomain));
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
