# `dashu-float` / `dashu-cmplx` — Phase 2

[`PLAN.md`](PLAN.md) records **phase 1**: moving `dashu-float`'s correctly-rounded
transcendentals off hand-derived ulp-count error bounds and onto mechanical `Ball` propagation
over a `Mag` value-space radius. That phase is done — this file collects what it deliberately
left behind, so the design record stays readable.

Two independent workstreams follow; neither depends on the other.

---

## 1. `dashu-cmplx`: mechanical error propagation

**Status: not started.** Every radius in `complex/` is still a hand-written `ulp() · k`.

Phase 1's migration is float-only. `dashu-cmplx` was unaffected by it and keeps its own
approach:

- **Its own Ziv driver**, `complex/src/ziv.rs` — a fallible-closure `ziv` generic over the
  number of parts (`N`), analogous to float's `ziv`/`ziv_pair` but not shared with it.
- **Hand radii at every transcendental**, expressed as `ulp()·k` over float's certified
  primitives:

  | function | radius | where |
  |---|---|---|
  | `exp` | `ulp()*6` per part | `complex/src/exp.rs` |
  | `powi` | `ulp() << (nlen+3)`, or `ZERO` on the exact chain | `complex/src/exp.rs` |
  | `powf` | `ulp() * amp`, `amp = (\|Re w·log b\| + 1) * 16` | `complex/src/exp.rs` |
  | `log` | `ln_r.ulp()*4 + B^(1-pw)*4`; `arg.ulp()*4` | `complex/src/log.rs` |
  | `sqrt` | `ulp()*10` per part | `complex/src/root.rs` |
  | `sin_cos` | `ulp()*8` per part | `complex/src/math/trig.rs` |
  | `tan` | `ulp()*8` | `complex/src/math/trig.rs` |
  | `asin`/`acos`/`atan` | `ulp()*20` (flat) | `complex/src/math/trig.rs` |

  The hyperbolic family has no driver of its own — it rotates through ±i and inherits the
  circular functions' radii. `abs`/`arg` delegate to float's `hypot`/`atan2`.

### 1a. Tighten the verification contract *first*

This is a prerequisite, not a nicety.

`dashu-float`'s directed-rounding differentials assert **bit-exact** agreement with MPFR
(`directed_eq`, `fuzz/tests/float_transcendental.rs`). `dashu-cmplx`'s assert agreement to
within **16 ulps per component** (`CLOSE_K` in `fuzz/src/lib.rs`, used by 22 call sites in
`fuzz/tests/cmplx_transcendental.rs` and `fuzz/tests/cmplx_random.rs`).

So a radius that is wrong-but-close in complex is **invisible to the current fuzzers** —
migrating under that contract would be flying blind. Either bring the complex differential up
to bit-exact directed checks first, or state explicitly which parts of the surface cannot
reach that bar and why.

Related: `AGENTS.md` claims the float *and* complex differentials "assert bit-exact agreement
with MPFR/MPC under every rounding mode (not a loose ulp tolerance)". That is true of float's
directed checks and **false of complex**. The claim should be corrected whether or not the
migration happens.

### 1b. The `CBall` decision

`PLAN.md` §6 rules out sharing float's `Mag`/`Ball`: a public `Mag` API is out of scope, and
`dashu-cmplx` depends only on float's *public* surface. The options:

1. **A private `Mag` copy inside `complex/`** — self-contained, duplicated maintenance.
2. **A `#[doc(hidden)]` lockstep-shared surface** — one implementation, an internal-only
   coupling between two crates.
3. **Leave complex on hand radii** and keep its caveats documented — the status quo.

Nothing in the complex driver needs to change beyond accepting ball radii; the work is
rewriting the radius sites in `exp.rs`, `log.rs`, `root.rs` and `math/trig.rs` (plus
`sin_cos`'s 4-part closure), after which `math/hyper.rs` follows for free.

### 1c. Accuracy caveats to resolve on the way

`asin`/`acos`/`atan` use a flat `ulp()*20` with no inflation as `1 − z² → 0`, where the
underlying `log`/`sqrt` composition amplifies. The source comments claim the opposite of the
known weakness — that "the Ziv retries absorb the `sqrt` amplification as `1-z² → 0`" and that
"the radius stays sound right up to the singularity". The deferred-accuracy limitation is
recorded only outside the code. Whether the comments or the limitation is right needs deciding;
either way the code should say which.

`atan`'s near-`±i` cancellation is absorbed the same way, with no radius inflation.

### 1d. Invariants the migration must preserve

- **`rad == 0` as the exact chain is load-bearing.** Under directed rounding an
  exactly-representable result can only be certified with a zero radius (`Down`'s preimage
  `[y, y+ulp)` admits no nonzero bracket). float's `sqrt` was the last place that forgot this
  and hung on `sqrt(4)` in base 10; complex's `powi` already special-cases it.
- The driver's containment test reads the public `ErrorBounds` preimage — do not change those
  semantics, complex depends on the same trait.

---

## 2. Performance follow-ups from phase 1

Measured at the time of the migration (`--quick`, same machine — 2 background cores busy, so
absolute numbers are conservative; old side at `12875d8`): **ln recovered across the board**
(1e1 2.9× faster, 1e2 0.77×, **1e3 0.85×** — the 1.4× @10³-bit regression over-recovered, 1e4
0.94×); **exp mixed** (1e2 0.67×, 1e3 0.84× faster, 1e4 1.09× ≈ noise, **1e1 1.6× slower**).

Outstanding items, by expected impact:

1. **`exp` low-precision series overhead (the 1e1 regression, ~1.6×)** — the hot loop's
   per-term `pow.div(&Ball::exact(Repr::new(factorial.clone(), 0)), wp)` pays an `IBig` clone +
   `Repr` construction + `from_repr` on the divisor every iteration (the old `div_exact` was
   leaner), and every `Ball` op constructs a fresh `Context::<mode::HalfEven>::new(prec)`.
   Fix sketch: a `div_ubig(&UBig, prec)`-style entry that builds the divisor once per term
   without the ball wrapper (radius rule is just `rad_a/|k| + ε`), and/or hoist the work
   context through the series loops. Low precision only — at 1e2+ the removed IBig bookkeeping
   dominates.
2. **Generic-base `from_repr` pays one `UBig::pow(B, |e|)` per call** (`mag.rs`'s
   `scale_by_base_pow`, bases ∉ {2, 10}) — that is every mul/div radius rule on such mids.
   Replace with a precomputed tight rational `log₂B` (from `B`'s top bits, ~30-bit accuracy,
   computed once) → O(1) integer multiplies, ≤ 2⁻³⁰ exponent slack. The exact-pow version stays
   as the `|e| > 8192` fallback. Only generic-base paths care today.
3. **`to_repr::<10>` export carries ≤ 10× slack** — it exports an outward power of ten and
   drops the significand. Three-line fix: export `Repr::new(IBig::from(self.man), k′)` with
   `k′` computed from `exp − BITS` so the 19-digit significand rides along; slack drops to ≤ 1
   decimal digit + the significand's own round-up. `ziv.rs`'s containment reads raw `Repr`s, so
   there is no precision-tag interplay. Theoretical: only knife-edge near-tie retry counts can
   improve.
4. **Lift `MAX_POWI_CHAIN_BITS = 64`** (`exp.rs`) — the cap existed because the ulp count
   compounds to ~`2^nlen` and no fixed-width counter fits; a `Mag` radius is O(1)-width for any
   `nlen`, so that motivation is gone. What remains is the algorithmic concern that the chain's
   working precision grows with `nlen` (`initial_guard = nlen + …`), so re-measure the crossover
   against the `powi_via_exp_log` fallback before lifting, and keep an allocation-bound sanity
   cap.
5. **Re-tune the Ziv guards** (measurement-driven, optional): the initial guards (`trig`'s 50,
   `exp`'s `series_guard + n`, `ln`'s `base_guard + 2`) were sized against the old ulp-count
   radius scale; `Mag` radii are systematically tighter (conditional ε = 0 on exact kernel
   results vs the old unconditional `+1`), so the first attempt now over-delivers on some
   functions. Use the `tuning` feature's retry counter (and the per-attempt radius it prints)
   to profile the per-function retry distribution and shave guards where the first-attempt
   radius clears the preimage with margin to spare.
