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

use dashu_base::{Approximation::*, Sign};

use crate::{
    cmp::repr_cmp_same_base,
    error::{FpError, FpResult},
    fbig::FBig,
    repr::{Context, Repr},
    round::{ErrorBounds, Rounded},
};
use dashu_int::{IBig, UBig, Word};

/// Maximum number of Ziv retries before falling back to the best-effort rounded value.
///
/// This is a sanity backstop only — the loop converges as soon as the working precision is
/// large enough that the error interval no longer straddles a rounding boundary, which happens
/// in one attempt for essentially all inputs (the guard-digit heuristic is sized for that) and
/// in a handful of attempts only for inputs pathological close to a tie. The cap exists so a
/// bug in an error-radius bound can never produce an infinite loop.
const MAX_ZIV_RETRIES: usize = 32;

/// Guard digits beyond the target precision up to which the Ziv loop keeps retrying only to
/// learn on which side of the returned value the true result lies (see `Context::certified`).
const SIDE_GUARD: usize = 16;

// A test-only retry counter, so tests can assert the loop converges on the first attempt for
// typical inputs (validating that the guard-digit heuristic wasn't over-tightened). Reads as
// the number of *extra* attempts beyond the first, i.e. `0` means first-attempt success.
#[cfg(any(all(test, feature = "std"), feature = "tuning"))]
thread_local! {
    pub(crate) static LAST_ZIV_RETRIES: core::cell::Cell<usize> = const { core::cell::Cell::new(0) };
}

// Reset/bump the retry counter, with no-op fallbacks when the `tuning` feature (or test mode)
// is absent — the Ziv loop body stays clean and ungated.
#[cfg(any(all(test, feature = "std"), feature = "tuning"))]
fn ziv_retries_reset_impl() {
    LAST_ZIV_RETRIES.with(|c| c.set(0));
}
#[cfg(not(any(all(test, feature = "std"), feature = "tuning")))]
fn ziv_retries_reset_impl() {}

#[cfg(any(all(test, feature = "std"), feature = "tuning"))]
fn ziv_retries_bump() {
    LAST_ZIV_RETRIES.with(|c| c.set(c.get().saturating_add(1)));
}
#[cfg(not(any(all(test, feature = "std"), feature = "tuning")))]
fn ziv_retries_bump() {}

/// Number of *extra* Ziv attempts beyond the first in the most recent Ziv loop (0 = first-attempt
/// success). Profiling only (via the `tuning` feature) — available when the `tuning` feature (or `cfg(test)`) is enabled.
#[cfg(any(all(test, feature = "std"), feature = "tuning"))]
pub fn ziv_retries() -> usize {
    LAST_ZIV_RETRIES.with(|c| c.get())
}

/// Reset the retry counter to 0 (before a measurement, so exact short-circuits that never enter a
/// Ziv loop report 0 rather than the previous call's count).
#[cfg(any(all(test, feature = "std"), feature = "tuning"))]
pub fn ziv_retries_reset() {
    LAST_ZIV_RETRIES.with(|c| c.set(0));
}

/// The signature of the per-attempt Ziv trace hook: `(guard, &radius)`. The radius formats
/// through [`core::fmt::Debug`] (the compact head‥tail form).
///
/// Not part of the stable API surface (`#[doc(hidden)]`) — a profiling hook, not a
/// typed-in-stone contract.
#[cfg(any(all(test, feature = "std"), feature = "tuning"))]
#[doc(hidden)]
pub type ZivTraceFn = fn(usize, &dyn core::fmt::Debug);

// Per-attempt Ziv trace hook: `Some(f)` makes every attempt of the single-value Ziv loop
// call `f(guard, &radius)`; `None` (the default) costs a single `Cell` read, so a `tuning`
// build stays usable for timing runs.
#[cfg(any(all(test, feature = "std"), feature = "tuning"))]
thread_local! {
    static ZIV_TRACE_HOOK: core::cell::Cell<Option<ZivTraceFn>> =
        const { core::cell::Cell::new(None) };
}

/// Install the per-attempt Ziv trace hook (`None` restores the silent default). Pairs with
/// [`ziv_retries`] when correlating traces with retry counts; a harness would typically pass a
/// capture-less closure, e.g.
/// `ziv_set_trace_hook(Some(|guard, radius| eprintln!("ZIV g={guard} r={radius:?}")))`.
///
/// Not part of the stable API surface (`#[doc(hidden)]`) — a profiling hook, not a
/// typed-in-stone contract.
#[cfg(any(all(test, feature = "std"), feature = "tuning"))]
#[doc(hidden)]
pub fn ziv_set_trace_hook(hook: Option<ZivTraceFn>) {
    ZIV_TRACE_HOOK.with(|h| h.set(hook));
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
    /// The returned [`Exact`]/[`Inexact`] flag is that of rounding the approximation to the target
    /// precision. That flag is the true result's own only if the two lie on the same side of the
    /// returned value, which the loop also establishes before accepting an attempt (see
    /// [`certified`](Self::certified)). A function whose true result can be exactly
    /// representable (or an exact tie) should report a zero radius for it, or resolve it before
    /// entering the loop: under a directed mode a positive radius can never certify it.
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
        ziv_retries_reset_impl();
        for _ in 0..MAX_ZIV_RETRIES {
            let (a, e) = approx(guard)?;
            // Per-attempt trace hook — the diagnostic that pairs with `ziv_retries()` when
            // profiling why a function retries. `None` (the default) reads one `Cell`; install
            // a printer with `ziv_set_trace_hook` (see its docs).
            #[cfg(any(all(test, feature = "std"), feature = "tuning"))]
            if let Some(hook) = ZIV_TRACE_HOOK.with(|h| h.get()) {
                hook(guard, &e);
            }
            // `with_precision` consumes `a`, but the containment test still needs it, so round a
            // clone and keep the original for the interval check.
            let candidate = a.clone().with_precision(self.precision);
            if self.certified::<B>(&a.repr, &e.repr, &candidate, guard) {
                return Ok(candidate);
            }

            // Grow the guard aggressively so a near-tie resolves in a couple of retries, while
            // the first attempt (with the heuristic guard) handles the common case. Cap it so
            // `guard` (and the closure's working precision `p + guard`) can't overflow `usize` —
            // reachable only if a radius-bound bug drives the loop to its retry cap.
            let step = core::cmp::max(guard, self.precision / 2).max(1);
            guard = guard.saturating_add(step).min(usize::MAX - self.precision);
            ziv_retries_bump();
        }

        // Unreachable in practice: a radius-bound bug would otherwise loop forever. Report it
        // instead of silently returning a possibly-1-ULP-wrong best-effort candidate.
        Err(FpError::ZivRetryLimitExceeded)
    }

    /// The correctly rounded value of `sign·(1 − δ)` for a `0 < δ` below half an ulp of the
    /// values just inside `±1` (`δ < B^(−p)/2`): `±1` itself, or the next representable inside it
    /// (`sign·(B^p − 1) × B^(−p)`) under the modes that round the magnitude down (`Up`/`Zero`
    /// just above `−1`, `Down`/`Zero` just below `+1`).
    ///
    /// Shared by `exp_m1` (whose value is `−1` plus a sub-representable positive residual once
    /// `exp(x)` has underflowed below the smallest representable FBig — the reduction quotient
    /// `s = floor(x/ln B)` overflows `isize`), `tanh` (whose value saturates at
    /// `sign(x)·(1 − 2e^{−2|x|})` for large `|x|`), and dashu-cmplx's complex `tan` (whose imaginary
    /// part saturates the same way for a large `|Im z|`; its `tanh` reaches it through
    /// `−i·tan(i·z)`). In all cases the Ziv loop cannot certify the candidate: the
    /// working-precision mid collapses onto exactly `±1`, which sits on the boundary
    /// of a directed rounding preimage (one-sided), so the containment test never resolves and the
    /// loop would run its retry cap at an astronomically large working precision.
    ///
    /// No rounding boundary lies between `±1` and `±(1 − B^(−p)/2)`, so every such `δ` rounds
    /// like one fixed tiny offset — an ordinary rounding of `±0.99…9…`, which also yields the
    /// ordinary [`Rounding`](crate::round::Rounding) flag: the adjustment applied to the
    /// *truncated* significand (`NoOp` when the result is the all-nines value, `AddOne`/`SubOne`
    /// when the magnitude is carried up to `1`), the same flag the Ziv loop reports for a value
    /// that is not quite saturated.
    ///
    /// Panics at unlimited precision (there is nothing to round to).
    ///
    /// Not part of the stable API surface (`#[doc(hidden)]`) — shared with dashu-cmplx's `tan`
    /// saturation gate so the endpoint logic lives in one place, not a typed-in-stone contract.
    #[doc(hidden)]
    pub fn near_one_endpoint<const B: Word>(&self, sign: Sign) -> Rounded<FBig<R, B>> {
        let one = if sign == Sign::Positive {
            Repr::<B>::one()
        } else {
            Repr::<B>::neg_one()
        };
        // two digits below the spacing just inside ±1: well within half an ulp
        let eps_exp = self
            .near_exact_exp(&one)
            .expect("the precision is limited and fits the exponent range");
        // the value sits just *inside* sign·1, so the offset carries the opposite sign
        self.round_offset(&one, -sign, eps_exp)
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
        ziv_retries_reset_impl();
        for _ in 0..MAX_ZIV_RETRIES {
            let ((a1, e1), (a2, e2)) = match approx(guard) {
                Ok(v) => v,
                Err(e) => return (Err(e), Err(e)),
            };
            let c1 = a1.clone().with_precision(self.precision);
            let c2 = a2.clone().with_precision(self.precision);
            if self.certified::<B>(&a1.repr, &e1.repr, &c1, guard)
                && self.certified::<B>(&a2.repr, &e2.repr, &c2, guard)
            {
                return (Ok(c1), Ok(c2));
            }

            // Grow the guard aggressively so a near-tie resolves in a couple of retries. Cap it so
            // `guard` (and the closure's working precision `p + guard`) can't overflow `usize`.
            let step = core::cmp::max(guard, self.precision / 2).max(1);
            guard = guard.saturating_add(step).min(usize::MAX - self.precision);
            ziv_retries_bump();
        }

        // Unreachable in practice: a radius-bound bug would otherwise loop forever. Report it
        // instead of silently returning possibly-1-ULP-wrong best-effort candidates.
        (Err(FpError::ZivRetryLimitExceeded), Err(FpError::ZivRetryLimitExceeded))
    }

    /// Is the rounded `candidate` certified as the correctly rounded result *with its flag*?
    ///
    /// The [containment test](Self::contained) settles the value. The flag needs one thing more:
    /// the `Rounding` adjustment, and `Exact` itself, describe where the *true* result sits
    /// relative to the returned `y`, while the candidate's flag describes where the
    /// *approximation* sits. The two agree once the error interval `[a − e, a + e]` no longer
    /// reaches `y`. Until then they need not: under a nearest mode an approximation just below
    /// `y` rounds to it with `AddOne` while the true result just above it would be `NoOp`, and
    /// an approximation that lands exactly on `y` reads as `Exact` (`cos(2^-10)` at 4 bits is
    /// exactly `1` at any working precision below 21 bits; the result is `1 − 2^-21`). Such an
    /// attempt is retried at a higher guard.
    ///
    /// A result that *is* exactly representable never separates from `y`, and the loop cannot
    /// tell it from one that is merely close. Operations are expected to resolve their exact
    /// results before entering the loop (under a directed mode the containment test cannot
    /// certify them either); as a backstop, the side is only pursued up to a working precision
    /// of about `2p` — past every near-miss built from a few terms of the operation's own
    /// series, whose distance from `y` is bounded by the digits of the argument — after which
    /// the attempt is accepted with the approximation's flag.
    fn certified<const B: Word>(
        &self,
        a: &Repr<B>,
        e: &Repr<B>,
        candidate: &Rounded<FBig<R, B>>,
        guard: usize,
    ) -> bool {
        let y = candidate.value_ref();
        if !Self::contained::<B>(a, e, y) {
            return false;
        }
        if e.significand.is_zero() {
            return true; // an exact approximation: its flag is the result's
        }
        let side_known = repr_cmp_same_base::<B, true>(&(a - &y.repr), e, None).is_gt();
        side_known || guard >= self.precision.saturating_add(SIDE_GUARD)
    }

    /// The exponent `k` such that an offset `0 < δ ≤ B^k` on either side of the exactly known
    /// `v` cannot reach a rounding boundary of this context — neither a representable value nor
    /// a midpoint between two. `None` at unlimited precision or if the exponent leaves `isize`.
    ///
    /// The boundaries around `v` are multiples of half the spacing of the binade *below* `v`'s
    /// leading digit (the finest one in reach), `g = B^(lead − 1 − p)/2`, and `v` itself is a
    /// multiple of its last digit `B^e`. If `B^e` is the coarser of the two, `v` is a boundary
    /// and the next one is `g` away; otherwise the nearest boundary is at least `B^e/2` away.
    /// Two digits below the finer of the two is inside both.
    fn near_exact_exp<const B: Word>(&self, v: &Repr<B>) -> Option<isize> {
        debug_assert!(v.is_finite() && !v.significand.is_zero());
        if !self.is_limited() {
            return None;
        }
        let p = isize::try_from(self.precision).ok()?;
        // A lower bound on the leading digit's exponent keeps the grid on the fine side
        // (`digits_lb` is a floor of the logarithm: at least one short of the digit count).
        let lead_lb = v.exponent.checked_add(v.digits_lb() as isize)?;
        let grid = v.exponent.min(lead_lb.checked_sub(p)?);
        grid.checked_sub(2)
    }

    /// Round `v + dir·B^eps_exp` for an `eps_exp` at or below [`near_exact_exp`](Self::near_exact_exp):
    /// the correctly rounded value, and flag, of every `v + dir·δ` with `0 < δ ≤ B^eps_exp`.
    fn round_offset<const B: Word>(
        &self,
        v: &Repr<B>,
        dir: Sign,
        eps_exp: isize,
    ) -> Rounded<FBig<R, B>> {
        let shift = (v.exponent - eps_exp) as usize; // ≥ 2
        let unit = IBig::from_parts(dir, UBig::ONE);
        // the last digit is ±1, so the significand is normalized
        let offset = Repr {
            significand: crate::utils::shl_digits::<B>(&v.significand, shift) + unit,
            exponent: eps_exp,
        };
        self.repr_round(offset).map(|repr| FBig::new(repr, *self))
    }

    /// Correctly round a result known to be `v + dir·δ` — the exactly representable `v` plus an
    /// unknown offset `0 < δ < B^delta_exp` on the side `dir` — when that offset is too small to
    /// reach a rounding boundary. `None` when it is not (the caller falls back to its Ziv loop).
    ///
    /// This is the shortcut for the functions that hug an exact value: `sin(x) = x − δ` and
    /// `cos(x) = 1 − δ` for a tiny `x`, `cosh(x) = 1 + δ`, … There the Ziv loop only certifies
    /// once the working precision reaches `δ` itself — `~2·|log_B x|` digits, however small the
    /// target precision — because until then the approximation sits exactly on `v`, a boundary
    /// of the rounding preimage.
    pub(crate) fn round_near<const B: Word>(
        &self,
        v: &Repr<B>,
        dir: Sign,
        delta_exp: isize,
    ) -> Option<Rounded<FBig<R, B>>> {
        let eps_exp = self.near_exact_exp(v)?;
        if delta_exp > eps_exp {
            return None;
        }
        Some(self.round_offset(v, dir, eps_exp))
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
    ///
    /// A zero radius means `a` is the true value, so rounding it is correct by definition —
    /// including a true value that is itself representable or an exact tie, which sits on the
    /// boundary of the preimage and which no positive radius could ever certify.
    ///
    /// Containment settles the *value* only. Whether the candidate's `Exact`/`Inexact` flag is
    /// also the true result's flag is decided by [`certified`](Self::certified).
    fn contained<const B: Word>(a: &Repr<B>, e: &Repr<B>, y: &FBig<R, B>) -> bool {
        if e.significand.is_zero() && !e.is_infinite() {
            return true;
        }
        // No nonzero real rounds to exactly zero (significant-digit rounding with unbounded
        // exponents keeps every nonzero magnitude), so a zero midpoint can only be certified
        // with a zero radius — the documented `error_bounds` interval for ±0 (±ulp) would
        // otherwise certify a cancellation that collapsed onto exact 0 while its radius still
        // covers nonzero neighbors (a wrong result, e.g. `log2` just above 1 at low precision).
        if y.repr.significand.is_zero() && !e.significand.is_zero() {
            return false;
        }
        // An unbounded radius — the `Mag::INFINITY` "unknown, retry at a higher guard" signal that
        // a degenerate denominator, a pole the ball straddles, or a root that rounded onto zero
        // exports — can never fit a finite preimage, so it is answered here rather than by the
        // arithmetic below (which asserts its operands finite, as `Repr` ops do).
        if e.is_infinite() {
            return false;
        }
        // A radius at least as large as the candidate cannot fit a preimage, which is at most an
        // ulp wide. Deciding that on the exponents alone keeps a wildly over-wide radius (an
        // attempt whose working precision cannot resolve the input yet, e.g. `exp` of a ball
        // millions wide) away from the exact interval arithmetic below, which would materialize
        // the whole exponent gap between the radius and the value.
        let e_lower = e.exponent as i128 + e.digits_lb() as i128 - 1; // e ≥ B^e_lower
        let y_upper = y.repr.exponent as i128 + y.repr.digits_ub() as i128; // |y| < B^y_upper
        if e_lower >= y_upper {
            return false;
        }
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

// The tests read `LAST_ZIV_RETRIES` (a `thread_local`), which only exists under `std` — gating the
// module on `all(test, std)` (rather than just `test`) keeps no-std test builds compiling.
#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;
    use crate::round::{mode, Rounding::*};

    type F = crate::FBig<mode::HalfEven>;

    thread_local! {
        static TRACE_CALLS: core::cell::Cell<usize> = const { core::cell::Cell::new(0) };
    }

    // The trace hook fires once per attempt while installed, and restores silence on `None`.
    #[test]
    fn ziv_trace_hook_fires_per_attempt() {
        TRACE_CALLS.with(|c| c.set(0));
        ziv_set_trace_hook(Some(|_guard, _radius| {
            TRACE_CALLS.with(|c| c.set(c.get() + 1));
        }));
        let ctx: Context<mode::HalfEven> = Context::new(6);
        let x = Repr::<10>::new(1.into(), 100);
        let _ = ctx.ln::<10>(&x, None).unwrap();
        ziv_set_trace_hook(None);
        assert!(TRACE_CALLS.with(|c| c.get()) >= 1, "trace hook must fire on each attempt");

        // After uninstalling, the same computation leaves the counter untouched.
        TRACE_CALLS.with(|c| c.set(0));
        let _ = ctx.ln::<10>(&x, None).unwrap();
        assert_eq!(TRACE_CALLS.with(|c| c.get()), 0, "hook must stay uninstalled");
    }

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
            // value 1 + 2^-10, radius 2^(-guard): large on the first attempt, tiny later.
            Ok((F::ONE + (F::ONE >> 10), F::ONE >> guard as isize))
        });
        assert_eq!(r, Ok(Inexact(F::ONE, NoOp)));
        assert!(LAST_ZIV_RETRIES.with(|c| c.get()) >= 1);
    }

    // An approximation that lands exactly on a representable value while its radius is nonzero
    // is not certified as `Exact`: the true value is on one side of it, and both the flag and
    // (under a directed mode) the value depend on which. Modelled on cos(2^-10) = 1 − 2^-21 at
    // 4 bits, whose approximation is exactly 1 until the working precision reaches 21 bits.
    #[test]
    fn ziv_does_not_certify_exact_with_radius() {
        fn run<R: ErrorBounds>() -> FpResult<FBig<R, 2>> {
            let exact = FBig::<R, 2>::ONE - (FBig::<R, 2>::ONE >> 21);
            Context::<R>::new(4).ziv(2, |guard| {
                let wp = 4 + guard;
                let a = exact.clone().with_rounding::<mode::HalfEven>();
                let a = a.with_precision(wp).value().with_rounding::<R>();
                Ok((a, FBig::ONE >> wp as isize))
            })
        }
        fn one<R: ErrorBounds>(sig: u8, exp: isize) -> FBig<R, 2> {
            FBig::from_parts(sig.into(), exp)
        }
        assert_eq!(run::<mode::HalfEven>(), Ok(Inexact(one(1, 0), AddOne)));
        assert!(LAST_ZIV_RETRIES.with(|c| c.get()) >= 1);
        assert_eq!(run::<mode::Up>(), Ok(Inexact(one(1, 0), AddOne)));
        assert_eq!(run::<mode::Down>(), Ok(Inexact(one(15, -4), NoOp)));
        // an exact approximation (zero radius) is still reported as Exact
        let r = Context::<mode::Down>::new(4).ziv(2, |_| Ok((FBig::<_, 2>::ONE, FBig::ZERO)));
        assert_eq!(r, Ok(Exact(one(1, 0))));
    }

    // The same holds for an approximation on the other side of the returned value from the true
    // result: 1 + 2^-21 approximated from below by 1 − 2^-wp rounds to 1 with `AddOne`, but the
    // result's own flag is `NoOp`.
    #[test]
    fn ziv_resolves_the_side_of_the_result() {
        let ctx: Context<mode::HalfEven> = Context::new(4);
        let exact = F::ONE + (F::ONE >> 21);
        let r = ctx.ziv(2, |guard| {
            let wp = 4 + guard;
            let ulp = F::ONE >> wp as isize;
            let a = if wp < 21 {
                F::ONE - ulp.clone()
            } else {
                exact.clone()
            };
            Ok((a, ulp << 1))
        });
        assert_eq!(r, Ok(Inexact(F::ONE, NoOp)));
        assert!(LAST_ZIV_RETRIES.with(|c| c.get()) >= 1);
    }

    // A result that really is the representable value never separates from it. Under a nearest
    // mode the loop stops pursuing the side at about twice the target precision and returns the
    // (correct) value instead of running to its retry cap.
    #[test]
    fn ziv_gives_up_on_the_side_of_an_exact_result() {
        let ctx: Context<mode::HalfEven> = Context::new(4);
        let r = ctx.ziv(2, |guard| Ok((F::ONE, F::ONE >> (4 + guard) as isize)));
        assert_eq!(r.map(|v| v.value()), Ok(F::ONE));
        let retries = LAST_ZIV_RETRIES.with(|c| c.get());
        assert!((1..=5).contains(&retries), "{retries} retries");
    }

    // `round_near` rounds `v ± δ` for an offset too small to reach a rounding boundary, and
    // declines when the bound on `δ` is not small enough.
    #[test]
    fn round_near_offsets() {
        let r = |sig: i32, exp: isize| Repr::<2>::new(sig.into(), exp);
        fn f<R: ErrorBounds>(sig: i32, exp: isize) -> FBig<R, 2> {
            FBig::from_parts(sig.into(), exp)
        }
        let up = Context::<mode::Up>::new(4);
        let down = Context::<mode::Down>::new(4);
        let even = Context::<mode::HalfEven>::new(4);
        // 1 − δ: the binade below 1 has spacing 2^-4
        assert_eq!(down.round_near(&r(1, 0), Sign::Negative, -6), Some(Inexact(f(15, -4), NoOp)));
        assert_eq!(up.round_near(&r(1, 0), Sign::Negative, -6), Some(Inexact(f(1, 0), AddOne)));
        assert_eq!(even.round_near(&r(1, 0), Sign::Negative, -6), Some(Inexact(f(1, 0), AddOne)));
        assert_eq!(down.round_near(&r(1, 0), Sign::Negative, -5), None);
        // 1 + δ
        assert_eq!(down.round_near(&r(1, 0), Sign::Positive, -6), Some(Inexact(f(1, 0), NoOp)));
        assert_eq!(up.round_near(&r(1, 0), Sign::Positive, -6), Some(Inexact(f(9, -3), AddOne)));
        // −5·2^-20 ± δ (3 of 4 digits used): the binade below its leading digit has spacing 2^-22
        assert_eq!(
            down.round_near(&r(-5, -20), Sign::Negative, -24),
            Some(Inexact(f(-11, -21), SubOne))
        );
        assert_eq!(
            down.round_near(&r(-5, -20), Sign::Positive, -24),
            Some(Inexact(f(-5, -20), SubOne))
        );
        assert_eq!(
            up.round_near(&r(-5, -20), Sign::Positive, -24),
            Some(Inexact(f(-9, -21), NoOp))
        );
        assert_eq!(down.round_near(&r(-5, -20), Sign::Positive, -23), None);
        // a value wider than the context: 37 = 100101b at 4 bits, its own last digit decides
        assert_eq!(even.round_near(&r(37, 0), Sign::Positive, -2), Some(Inexact(f(9, 2), NoOp)));
        assert_eq!(up.round_near(&r(37, 0), Sign::Negative, -2), Some(Inexact(f(5, 3), AddOne)));
        assert_eq!(up.round_near(&r(37, 0), Sign::Negative, -1), None);
        // unlimited precision has nothing to round to
        assert_eq!(Context::<mode::Up>::new(0).round_near(&r(1, 0), Sign::Negative, -100), None);
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

    // An approximation whose error interval always straddles a rounding boundary (a radius-bound
    // bug) exhausts the retry budget and reports `ZivRetryLimitExceeded` instead of silently
    // returning a possibly-1-ULP-wrong best-effort value.
    #[test]
    fn ziv_reports_retry_limit_exceeded() {
        let ctx: Context<mode::HalfEven> = Context::new(4);
        // radius 10 >> 1 ulp at any working precision, so containment always fails.
        let r = ctx.ziv(2, |_| Ok((F::ONE, F::from(10u8))));
        assert_eq!(r, Err(FpError::ZivRetryLimitExceeded));
        // the pair variant reports it on both slots.
        let (r1, r2) = ctx.ziv_pair(2, |_| Ok(((F::ONE, F::from(10u8)), (F::ONE, F::ZERO))));
        assert_eq!(r1, Err(FpError::ZivRetryLimitExceeded));
        assert_eq!(r2, Err(FpError::ZivRetryLimitExceeded));
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
            Ok(((F::ONE, F::ZERO), (F::ONE + (F::ONE >> 10), radius)))
        });
        assert_eq!(r1, Ok(Exact(F::ONE)));
        assert_eq!(r2, Ok(Inexact(F::ONE, NoOp)));
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
                // (reset to 0: a tiny argument is rounded directly, without entering the loop)
                LAST_ZIV_RETRIES.with(|c| c.set(0));
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
