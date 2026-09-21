# `dashu-float` / `dashu-cmplx` — Phase 2

[`PLAN.md`](PLAN.md) records **phase 1**: moving `dashu-float`'s correctly-rounded
transcendentals off hand-derived ulp-count error bounds and onto mechanical `Ball` propagation
over a `Mag` value-space radius. That phase is done — this file collects what it deliberately
left behind, so the design record stays readable.

The workstream below is independent of the phase-1 migration itself.

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
within **16 ulps per component** (`CLOSE_K` in `fuzz/src/lib.rs`, used by 29 call sites in
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

