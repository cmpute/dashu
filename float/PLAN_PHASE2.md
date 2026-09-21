# `dashu-float` / `dashu-cmplx` — Phase 2

[`PLAN.md`](PLAN.md) records **phase 1**: moving `dashu-float`'s correctly-rounded
transcendentals off hand-derived ulp-count error bounds and onto mechanical `Ball` propagation
over a `Mag` value-space radius. That phase is done — this file collected what it deliberately
left behind, so the design record stays readable.

---

## 1. `dashu-cmplx`: mechanical error propagation

**Status: done.** Every transcendental radius in `complex/` is now mechanically propagated.

What landed (per family, one commit each, on the `cmplx-mag-ball` branch):

- **The shared substrate.** float's `Ball`/`Mag` are re-exported `#[doc(hidden)]` from float's
  crate root (option 2 of §1b — the lockstep-shared surface, excluded from the semver and
  stability guarantees; both crates release together). `complex/src/ball.rs` composes the
  private `CBall { re: Ball, im: Ball }` on top: the 4-product complex mul, the quotient
  through `‖z‖²`, the exact ±i rotation, the cancellation-free `sqrt` (with a straddle-safe
  real-sqrt fold), and the `exp`/`log` compositions whose float kernels run on the midpoints
  with Mag-level input-error folds (`ln(1+t) ≤ t`, `‖∇arg‖ = 1/‖z‖`, float's own exp fold).
- **All twelve radius sites migrated**: `sqrt`, `exp`, `log`, `sin_cos`, `tan`, `sin_cos_pi`,
  `tan_pi`, `powi`, `powf`, `asin`, `acos`, `atan`. `math/hyper.rs` needed nothing (pure
  rotations) — it inherited the new radii, as predicted.
- **The verification contract first (§1a)**: the complex fuzz differentials now assert
  **bit-exact per-component agreement with MPC** under `Up`/`Down`/`Zero`/`HalfEven` — the same
  straddle contract as float's directed tests (nearest MPC reference at `2·prec+512` bits,
  re-rounded per component; both sides converted exactly to `rug::Float`). Two families
  deliberately stay on the documented `CLOSE_K` tolerance:
  - the **×π family** — MPC has no ×π entry, and a premultiplied-π reference changes the
    *function under test* (the premultiplication's rounding error is amplified by the
    hyperbolic derivative), so a straddle oracle of that modified function licenses nothing;
  - **`powf`** — MPC documents `mpc_pow` as not guaranteed correctly rounded, so even a
    high-precision reference is a tolerance oracle, not a straddle oracle.
- **`AGENTS.md`** now states the actual per-suite contracts (the old claim that the float
  *and* complex differentials were all bit-exact was false for complex; it is now true for
  every family listed above, with the exceptions documented).

What the migration bought beyond tighter radii:

- **Exactly-representable results certify** through the chain's zero radius (`rad == 0`),
  fixing the `ZivRetryLimitExceeded` the old nonzero radii produced on inputs like
  `√4`, `√(3+4i)`, `acos(1)` under the outward rounding modes (§1d's invariant).
- **§1c resolved empirically, in favor of the mechanism**: the near-singularity accuracy the
  old comments claimed is now real — `asin`/`acos`' radius grows as `1−z² → 0` (the `sqrt`
  fold divides by the shrinking root), `atan`'s log-difference cancellation is *tracked*
  rather than absorbed, and `powi`'s compounding squaring error is mechanical. The directed
  differentials hold at 20/50/100+ bits.
- `powi(z, ±1)` now respects the context precision (the directed differential caught the old
  `|n| = 1` shortcut returning the unrounded input).
- One deliberate divergence from float's driver, documented in `complex/src/ziv.rs`: complex
  does **not** carry float's zero-candidate guard. Complex compositions contain correlated
  cancellations whose result component is *exactly* zero while no mid±radius ball can know it
  (`powi` of `|re| = |im|` inputs, `atan` of axis inputs) — the strict guard would deadlock
  them into exponential retries, so the ±ulp `error_bounds` preimage of ±0 stays certifiable
  here (it bounds any error to sub-target-ulp scale). The Annex-G axis dispatch in `atan`
  covers its axis cases with genuinely zero radii.

Notes for the record:

- The `B^{1-pw}`-style hand terms and the per-family guard constants remain as first-attempt
  heuristics only; shrinking them (EXP_GUARD 14 → ~8 etc.) is a measured follow-up, gated by
  `ziv_few_retries_for_typical_inputs` and the transcendental bench.
- The ln input fold keeps its bracket on the Mag radius alone: at base ≠ 2 the two Mag bounds
  of an exact midpoint sit up to a factor ~2 apart (the fixed-point `log₂B` granularity), and
  a value-space `hi − lo` there inflates the fold to O(1).
