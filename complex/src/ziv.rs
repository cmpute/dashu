//! The complex Ziv retry loop — guaranteed-correct rounding for complex transcendentals.
//!
//! Each complex transcendental (`exp`, `log`, …) approximates its result parts at a working
//! precision `p + guard`, together with a provable absolute error radius per part, and this driver
//! rounds each part to the target precision and checks that the whole per-part error interval
//! `[ã − E, ã + E]` lies inside the candidate's [`ErrorBounds`] preimage — the set of reals that
//! round to it. If every part fits, the rounded value is *guaranteed* correct; otherwise the driver
//! retries with more guard digits, sharing the recomputed working evaluation across all parts
//! (retrying while *any* part straddles a rounding boundary). The loop provably terminates (a true
//! tie is resolved deterministically by the mode), with a large sanity cap as an unreachable
//! backstop.
//!
//! The driver is generic in the part count `N`: most functions compute one complex result (`N = 2`
//! parts), while `sin_cos` computes two (`N = 4`, sharing one evaluation of the real `sin_cos` and
//! `sinh_cosh`). This mirrors `dashu-float`'s Ziv loop, with two complex-specific choices:
//! * The approximation closure is **fallible** — a complex composition can overflow mid-evaluation
//!   (e.g. `exp` of a large real part), and propagating that [`FpError`] with `?` avoids hoisting
//!   a per-function overflow probe out of the loop. Such errors are terminal (precision-independent)
//!   and never trigger a retry.
//! * The per-part containment test runs on [`FBig`]s at unlimited precision
//!   (`FloatCtxt::new(0)`, where `+`/`−` are lossless), expressing `dashu-float`'s raw-`Repr`
//!   interval arithmetic through the public `FBig` API so this crate need not reach into float's
//!   internal arithmetic. The two are exactly equivalent.
//!
//! As in the float layer, each correctly-rounded float sub-operation contributes ~0 to the
//! composition radius (it is certified by its own Ziv loop); only the arithmetic composition steps
//! contribute, bounded by a small constant × working-ULP (plus any input-error amplification, which
//! each caller folds into its radius — e.g. `log`'s `ln|z|` near `|z| = 1`).

use core::cmp::Ordering;

use dashu_base::Approximation;
use dashu_float::round::{ErrorBounds, Rounding};
use dashu_float::{Context as FloatCtxt, FBig, FpError};
use dashu_int::Word;

use crate::repr::Context;

/// Maximum number of Ziv retries before falling back to the best-effort rounded value.
///
/// A sanity backstop only — the loop converges as soon as the working precision is large enough
/// that no part's error interval straddles a rounding boundary, which happens in one attempt for
/// essentially all inputs (the guard-digit heuristic is sized for that) and in a handful of
/// attempts only for inputs pathologically close to a tie. The cap exists so a bug in an error
/// radius can never produce an infinite loop.
const MAX_ZIV_RETRIES: usize = 32;

// A test-only retry counter (extra attempts beyond the first), mirroring `dashu-float`'s counter
// so tests can assert the loop converges on the first attempt for typical inputs — i.e. `0` means
// first-attempt success.
#[cfg(all(test, feature = "std"))]
thread_local! {
    pub(crate) static LAST_ZIV_RETRIES: core::cell::Cell<usize> = const { core::cell::Cell::new(0) };
}

impl<R: ErrorBounds> Context<R> {
    /// Correctly round a complex transcendental's `N` result parts to this context's precision using
    /// a Ziv retry loop that certifies **every** part.
    ///
    /// `approx(guard)` evaluates the function at working precision `self.precision() + guard` and
    /// returns `Ok([(v₀, e₀), …])` — each part's value and its provable absolute error radius, all
    /// as [`FBig`]s at the working context — or an [`FpError`] (overflow / domain), which propagates
    /// immediately without retry (such errors are terminal, not precision problems). The driver
    /// rounds each part to the target precision and retries while *any* part's error interval
    /// straddles a rounding boundary, sharing the recomputed evaluation. Returns each part's rounded
    /// value (with its inexactness flag) for the caller to assemble.
    ///
    /// Rejects an unlimited context (the recipe is a limited-precision technique); each caller's
    /// exact special-value shortcuts run before this driver and are unaffected.
    pub(crate) fn ziv<const B: Word, const N: usize>(
        &self,
        initial_guard: usize,
        mut approx: impl FnMut(usize) -> Result<[(FBig<R, B>, FBig<R, B>); N], FpError>,
    ) -> Result<[Approximation<FBig<R, B>, Rounding>; N], FpError> {
        self.assert_limited();
        let p = self.precision();
        let mut guard = initial_guard;
        #[cfg(all(test, feature = "std"))]
        LAST_ZIV_RETRIES.with(|c| c.set(0));
        for _ in 0..MAX_ZIV_RETRIES {
            let parts = approx(guard)?;
            // Round each part to the target precision. `with_precision` consumes the value, but the
            // containment test still needs the working-precision original, so round clones.
            let candidates = parts.clone().map(|(v, _)| v.with_precision(p));
            if parts
                .iter()
                .zip(candidates.iter())
                .all(|((v, e), c)| Self::contained::<B>(v, e, c.value_ref()))
            {
                return Ok(candidates);
            }

            // Grow the guard aggressively so a near-tie resolves in a couple of retries, while the
            // first attempt (with the heuristic guard) handles the common case (matches float).
            let step = core::cmp::max(guard, p / 2).max(1);
            guard += step;
            #[cfg(all(test, feature = "std"))]
            LAST_ZIV_RETRIES.with(|c| c.set(c.get() + 1));
        }

        // Unreachable in practice: a radius-bound bug would otherwise loop forever. Report it
        // instead of silently returning possibly-1-ULP-wrong best-effort parts.
        Err(FpError::ZivRetryLimitExceeded)
    }

    /// Per-part containment test: is the approximation's error interval `[value ± radius]`
    /// entirely inside the rounding preimage of `target` (every real in `[target − lb, target + rb]`
    /// rounds to `target` under `R`)?
    ///
    /// The interval arithmetic runs on [`FBig`]s at unlimited precision (`FloatCtxt::new(0)`),
    /// where addition is lossless — no rounding can drop a guard digit and mis-decide the call (a
    /// wrong call here yields a wrong ULP). The sums are compared rather than the differences
    /// (algebraically identical for exact arithmetic, reading as one shared inequality per endpoint):
    ///   `value − radius ≥ target − lb  ⟺  value + lb ≥ target + radius`
    ///   `value + radius ≤ target + rb  ⟺  target + rb ≥ value + radius`
    fn contained<const B: Word>(
        value: &FBig<R, B>,
        radius: &FBig<R, B>,
        target: &FBig<R, B>,
    ) -> bool {
        let (lb, rb, incl_l, incl_r) = R::error_bounds::<B>(target);
        let x = FloatCtxt::<R>::new(0);
        let left = x
            .add(value.repr(), lb.repr())
            .unwrap()
            .value()
            .cmp(&x.add(target.repr(), radius.repr()).unwrap().value());
        let right = x
            .add(target.repr(), rb.repr())
            .unwrap()
            .value()
            .cmp(&x.add(value.repr(), radius.repr()).unwrap().value());
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

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;
    use crate::cbig::CBig;
    use dashu_float::round::mode;

    type F = FBig<mode::HalfEven>;

    // Build a modest limited-precision complex input from f64 parts (away from poles/singularities).
    fn z(re: f64, im: f64) -> CBig<mode::HalfEven, 2> {
        let mk = |v: f64| F::try_from(v).unwrap().with_precision(53).value();
        CBig::from_parts(mk(re), mk(im))
    }

    // The guard constants should let every migrated transcendental converge in ≤1 retry for typical
    // inputs (Ziv certifies correctness; the guard only sizes the first-attempt hit rate). This
    // catches gross guard mis-sizing, not the occasional near-tie retry.
    #[test]
    fn ziv_few_retries_for_typical_inputs() {
        let ctx: Context<mode::HalfEven> = Context::new(53);
        const MAX_RETRIES: usize = 1;
        let cases = [
            z(0.5, 0.3),
            z(1.0, -0.5),
            z(2.0, 0.25),
            z(0.25, 0.75),
            z(1.5, 0.4),
        ];
        for c in cases {
            let assert_few = |name: &str, retries: usize| {
                assert!(
                    retries <= MAX_RETRIES,
                    "{name} took {retries} retries (expected <= {MAX_RETRIES})"
                );
            };
            LAST_ZIV_RETRIES.with(|cell| cell.set(usize::MAX));
            drop(ctx.exp(&c, None));
            assert_few("exp", LAST_ZIV_RETRIES.with(|cell| cell.get()));

            LAST_ZIV_RETRIES.with(|cell| cell.set(usize::MAX));
            drop(ctx.log(&c, None));
            assert_few("log", LAST_ZIV_RETRIES.with(|cell| cell.get()));

            LAST_ZIV_RETRIES.with(|cell| cell.set(usize::MAX));
            drop(ctx.sin_cos(&c, None));
            assert_few("sin_cos", LAST_ZIV_RETRIES.with(|cell| cell.get()));

            LAST_ZIV_RETRIES.with(|cell| cell.set(usize::MAX));
            drop(ctx.sqrt(&c));
            assert_few("sqrt", LAST_ZIV_RETRIES.with(|cell| cell.get()));

            LAST_ZIV_RETRIES.with(|cell| cell.set(usize::MAX));
            drop(ctx.asin(&c, None));
            assert_few("asin", LAST_ZIV_RETRIES.with(|cell| cell.get()));

            LAST_ZIV_RETRIES.with(|cell| cell.set(usize::MAX));
            drop(ctx.atan(&c, None));
            assert_few("atan", LAST_ZIV_RETRIES.with(|cell| cell.get()));

            // the hyperbolic family routes through the circular functions, so its retry
            // behavior is inherited (the rotation plumbing adds no extra Ziv precision).
            LAST_ZIV_RETRIES.with(|cell| cell.set(usize::MAX));
            drop(ctx.sinh(&c, None));
            assert_few("sinh", LAST_ZIV_RETRIES.with(|cell| cell.get()));

            LAST_ZIV_RETRIES.with(|cell| cell.set(usize::MAX));
            drop(ctx.cosh(&c, None));
            assert_few("cosh", LAST_ZIV_RETRIES.with(|cell| cell.get()));

            LAST_ZIV_RETRIES.with(|cell| cell.set(usize::MAX));
            drop(ctx.tanh(&c, None));
            assert_few("tanh", LAST_ZIV_RETRIES.with(|cell| cell.get()));

            LAST_ZIV_RETRIES.with(|cell| cell.set(usize::MAX));
            drop(ctx.asinh(&c, None));
            assert_few("asinh", LAST_ZIV_RETRIES.with(|cell| cell.get()));

            LAST_ZIV_RETRIES.with(|cell| cell.set(usize::MAX));
            drop(ctx.acosh(&c, None));
            assert_few("acosh", LAST_ZIV_RETRIES.with(|cell| cell.get()));

            LAST_ZIV_RETRIES.with(|cell| cell.set(usize::MAX));
            drop(ctx.atanh(&c, None));
            assert_few("atanh", LAST_ZIV_RETRIES.with(|cell| cell.get()));

            LAST_ZIV_RETRIES.with(|cell| cell.set(usize::MAX));
            drop(ctx.powf(&c, &z(0.7, 0.2), None));
            assert_few("powf", LAST_ZIV_RETRIES.with(|cell| cell.get()));
        }
    }

    // An exact approximation (both radii 0) is accepted on the first attempt (0 retries).
    #[test]
    fn ziv_accepts_exact_first_attempt() {
        let ctx: Context<mode::HalfEven> = Context::new(10);
        LAST_ZIV_RETRIES.with(|c| c.set(usize::MAX));
        let r = ctx.ziv(4, |_| Ok([(F::ONE, F::ZERO), (F::from(2u8), F::ZERO)]));
        assert!(r.is_ok());
        assert_eq!(LAST_ZIV_RETRIES.with(|c| c.get()), 0);
    }

    // An approximation whose second part's error interval straddles a boundary must retry; once the
    // shrinking radius makes the interval unambiguous, the driver accepts.
    #[test]
    fn ziv_retries_until_contained() {
        let ctx: Context<mode::HalfEven> = Context::new(4);
        let r = ctx.ziv(2, |guard| {
            // value 1+1i, imaginary radius 2^(-guard): large on the first attempt, tiny later.
            Ok([(F::ONE, F::ZERO), (F::ONE, F::ONE >> guard as isize)])
        });
        drop(r.unwrap());
        assert!(LAST_ZIV_RETRIES.with(|c| c.get()) >= 1);
    }

    // The driver certifies all N parts together: here the 4-part form (as `sin_cos` uses) retries
    // while any one part straddles a boundary.
    #[test]
    fn ziv_retries_until_all_four_contained() {
        let ctx: Context<mode::HalfEven> = Context::new(4);
        let r = ctx.ziv(2, |guard| {
            let rad = F::ONE >> guard as isize;
            Ok([
                (F::ONE, F::ZERO),
                (F::from(2u8), F::ZERO),
                (F::from(3u8), rad), // this part forces the retry
                (F::from(4u8), F::ZERO),
            ])
        });
        drop(r.unwrap());
        assert!(LAST_ZIV_RETRIES.with(|c| c.get()) >= 1);
    }

    // The driver rejects unlimited precision (its callers' exact shortcuts run first).
    #[test]
    #[should_panic(expected = "precision cannot be 0")]
    fn ziv_rejects_unlimited() {
        let ctx: Context<mode::HalfEven> = Context::new(0);
        drop(ctx.ziv(4, |_| Ok([(F::ONE, F::ZERO), (F::ONE, F::ZERO)])));
    }

    // An approximation whose error interval always straddles a rounding boundary (a radius-bound
    // bug) exhausts the retry budget and reports `ZivRetryLimitExceeded` instead of silently
    // returning possibly-wrong best-effort parts.
    #[test]
    fn ziv_reports_retry_limit_exceeded() {
        let ctx: Context<mode::HalfEven> = Context::new(4);
        // one part carries radius 10 >> 1 ulp at any working precision, so containment always fails.
        let r = ctx.ziv(2, |_| Ok([(F::ONE, F::ZERO), (F::ONE, F::from(10u8))]));
        assert_eq!(r, Err(FpError::ZivRetryLimitExceeded));
    }
}
