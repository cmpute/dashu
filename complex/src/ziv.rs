//! The complex Ziv retry loop — guaranteed-correct rounding for complex transcendentals.
//!
//! Each complex transcendental (`exp`, `log`, …) approximates its result parts at a working
//! precision `p + guard`, together with a provable absolute error radius per part, and this driver
//! rounds each part to the target precision and checks that the whole per-part error interval
//! `[ã − E, ã + E]` lies inside the candidate's [`ErrorBounds`] preimage — the set of reals that
//! round to it. If every part fits, the rounded value is *guaranteed* correct; otherwise the driver
//! retries with more guard digits, sharing the recomputed working evaluation across all parts
//! (retrying while *any* part straddles a rounding boundary).
//!
//! A part whose true value is exactly representable — or exactly a rounding tie — sits *on* a
//! boundary, and no nonzero radius can ever fit around it. Such parts are certified through a
//! **zero radius**: every closure computes on balls whose operations stay exact whenever their
//! result is, and the functions route their structurally exact parts (the axes, the branch
//! cuts, integer and dyadic powers) through exact operations or exact shortcuts. The same rule
//! settles the inexactness *flag*: a part is reported `Exact`, or rounded in a given direction,
//! only when that is proven — with a nonzero radius the candidate must lie strictly outside the
//! error interval, otherwise the driver retries. The loop terminates for every such input; a
//! large retry cap remains as a backstop.
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
use dashu_float::{Ball, ConstCache, Context as FloatCtxt, FBig, FpError, Repr};
use dashu_int::{IBig, Word};

use crate::ball::kernel_ball;
use crate::repr::{reborrow_cache, Context};

/// Maximum number of Ziv retries before falling back to the best-effort rounded value.
///
/// A sanity backstop only — the loop converges as soon as the working precision is large enough
/// that no part's error interval straddles a rounding boundary, which happens in one attempt for
/// essentially all inputs (the guard-digit heuristic is sized for that) and in a handful of
/// attempts only for inputs pathologically close to a tie. The cap exists so a bug in an error
/// radius can never produce an infinite loop.
pub(crate) const MAX_ZIV_RETRIES: usize = 32;

/// Guard digits (base-B) for re-rounding a single real kernel result or constant.
const KERNEL_GUARD: usize = 8;

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
            // Cap it so `guard` (and the closure's working precision `p + guard`) can't overflow
            // `usize` — reachable only if a radius-bound bug drives the loop to its retry cap.
            let step = core::cmp::max(guard, p / 2).max(1);
            guard = guard.saturating_add(step).min(usize::MAX - p);
            #[cfg(all(test, feature = "std"))]
            LAST_ZIV_RETRIES.with(|c| c.set(c.get() + 1));
        }

        // Unreachable in practice: a radius-bound bug would otherwise loop forever. Report it
        // instead of silently returning possibly-1-ULP-wrong best-effort parts.
        Err(FpError::ZivRetryLimitExceeded)
    }

    /// Correctly round `k` or `−k` to this context's precision, where `k` is a real
    /// `dashu-float` kernel (`|ctx| ctx.acosh(x, cache)`, …).
    ///
    /// `+k` is the kernel's own correctly-rounded result at this precision. `−k` is **not**
    /// the negation of that: negating a value rounded under a directed mode (`Down`/`Up`)
    /// yields the value rounded in the *opposite* direction. So `−k` evaluates the kernel at a
    /// working precision (correctly rounded there, i.e. within one work-ulp), negates the
    /// resulting ball, and rounds once through the Ziv driver. An exact kernel result stays
    /// exact (`−(+0) = −0`) — `can_be_exact` says whether the kernel has an exact value at this
    /// argument at all (see `kernel_ball`).
    pub(crate) fn round_kernel<const B: Word>(
        &self,
        negate: bool,
        can_be_exact: bool,
        mut kernel: impl FnMut(&FloatCtxt<R>) -> Result<Approximation<FBig<R, B>, Rounding>, FpError>,
    ) -> Result<Approximation<FBig<R, B>, Rounding>, FpError> {
        if !negate {
            return kernel(&self.float());
        }
        let p = self.precision();
        let [part] = self.ziv::<B, 1>(KERNEL_GUARD, |guard| {
            let pw = p + guard;
            let fctx = FloatCtxt::<R>::new(pw);
            let ball = -kernel_ball(kernel(&fctx)?, pw, |_| can_be_exact);
            Ok([ball.to_value_radius(&fctx)])
        })?;
        Ok(part)
    }

    /// Correctly round `±π` (or `±π/2` with `halve`) to this context's precision — the
    /// constant real/imaginary parts on the branch cuts (`log(−0 − 0i) = −∞ − iπ`,
    /// `asin(x ± 0i) = ±π/2 …` for `|x| > 1`, …). As in [`round_kernel`](Self::round_kernel),
    /// the sign and the halving are applied *before* the single final rounding.
    pub(crate) fn round_pi_multiple<const B: Word>(
        &self,
        negate: bool,
        halve: bool,
        mut cache: Option<&mut ConstCache>,
    ) -> Result<Approximation<FBig<R, B>, Rounding>, FpError> {
        let p = self.precision();
        let [part] = self.ziv::<B, 1>(KERNEL_GUARD, |guard| {
            let pw = p + guard;
            let fctx = FloatCtxt::<R>::new(pw);
            // π is irrational: never exact
            let mut ball = kernel_ball(fctx.pi::<B>(reborrow_cache(&mut cache)), pw, |_| false);
            if halve {
                ball = ball.div(&Ball::exact(Repr::new(IBig::from(2u8), 0)), pw)?;
            }
            if negate {
                ball = -ball;
            }
            Ok([ball.to_value_radius(&fctx)])
        })?;
        Ok(part)
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
        // The zero-candidate guard, same as `dashu-float`'s driver: no nonzero real rounds to
        // exactly zero (significant-digit rounding with unbounded exponents keeps every nonzero
        // magnitude), so a zero candidate can only be certified by a zero radius. The documented
        // ±ulp `error_bounds` preimage of ±0 is an f64-style artifact (where subnormals bound the
        // exponent range) and would otherwise certify a cancellation that collapsed onto exact 0
        // while its radius still covers nonzero neighbors — `powi(z, −10)` of a z whose parts
        // round onto the diagonal at the work precision returned `re = 0` for a true value of
        // `7.6e-22` (2⁶⁹ ulps off, caught by the directed fuzz): the honest ball `(0 ± 2⁻⁴²)`
        // fit the wide preimage although the true value rounds to a *nonzero* 20-bit float.
        // The structural zeros of the axis/cancellation identities are the closures' duty —
        // they must carry a zero radius (exact chains do; the axis shortcuts pin their exact
        // components), which this guard then certifies on the first attempt.
        // An unbounded radius — the `Mag::INFINITY` "unknown, retry at a higher guard" signal
        // that a collapsed denominator exports (see `ball::div_real`) — can never fit a finite
        // preimage, so it is answered here rather than by the arithmetic below (which asserts
        // its operands finite, as `Repr` ops do).
        if target.repr().significand().is_zero() && !radius.repr().significand().is_zero() {
            return false;
        }
        if radius.repr().is_infinite() {
            return false;
        }
        // A radius at least as large as the candidate itself can never fit the candidate's
        // preimage (which is narrower than the candidate's magnitude). Answer that from the
        // exponents alone: the exact sums below align their operands, and a radius that is
        // astronomically larger than the value (an input-error fold amplified through `exp`,
        // say) would otherwise allocate a number spanning the whole exponent gap.
        if !radius.repr().significand().is_zero() {
            let target_top = target
                .repr()
                .exponent()
                .saturating_add(target.repr().digits_ub() as isize);
            if radius.repr().exponent() >= target_top {
                return false;
            }
        }
        let (lb, rb, incl_l, incl_r) = R::error_bounds::<B>(target);
        let x = FloatCtxt::<R>::new(0);
        let target_hi = x.add(target.repr(), radius.repr()).unwrap().value(); // target + radius
        let value_hi = x.add(value.repr(), radius.repr()).unwrap().value(); // value + radius
        let left = x
            .add(value.repr(), lb.repr())
            .unwrap()
            .value()
            .cmp(&target_hi);
        let right = x
            .add(target.repr(), rb.repr())
            .unwrap()
            .value()
            .cmp(&value_hi);
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
        if !(left_ok && right_ok) {
            return false;
        }
        // The candidate's inexactness flag is read off the *approximation* (whether, and in
        // which direction, `value` was rounded to reach `target`). That is the flag of the true
        // value only when the true value provably lies on the same side of `target` as `value`:
        // either the approximation is exact (zero radius), or `target` lies strictly outside
        // `[value − radius, value + radius]`. Otherwise the true value may sit on the other
        // side of `target` — and a rounded part would be tagged `Exact`, or an exact one
        // `Inexact`, or with the wrong direction — so retry at a higher guard. (A part that is
        // exactly representable must come with a zero radius: that is the closures' duty, as
        // for the structural zeros above.)
        radius.repr().significand().is_zero()
            || value.repr().cmp(target_hi.repr()) == Ordering::Greater
            || target.repr().cmp(value_hi.repr()) == Ordering::Greater
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
            // value 1 + (1 + 2^-5)i, imaginary radius 2^(-guard): large on the first attempt,
            // tiny later (the imaginary value sits off the 4-bit grid, so its rounding
            // direction is decidable once the radius shrinks).
            Ok([
                (F::ONE, F::ZERO),
                (F::ONE + (F::ONE >> 5), F::ONE >> guard as isize),
            ])
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
                // 3 + 2^-5: off the 4-bit grid, so its rounding direction is decidable
                (F::from_parts(97.into(), -5), rad), // this part forces the retry
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
