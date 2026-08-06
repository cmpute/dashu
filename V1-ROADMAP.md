# dashu Roadmap — v0.6 → v1.0

Last updated: 2026-08-05

**Release strategy.** v0.6 is the **breaking-changes testbed**: every planned breaking
change ships there first, so downstream users can migrate and validate early, and v1.0
then freezes the API. v0.5.x (0.5.0 / 0.5.1 / 0.5.2) is released; the items below carry
their current status. File:line references are anchors from the v0.5.x tree and may
drift.

*v0.6.0-rc.x already carries several breaking/contract changes — see the per-crate
CHANGELOGs (`ParseError::InvalidSyntax`, `Context::exp` underflow → `Err(Underflow)`,
Ziv transcendentals requiring `R: ErrorBounds`, `TryFrom<RBig> for FBig`, …, plus the
signed-`isize` `RBig::pow`/`Relaxed::pow` widening).*

> The v0.6 release mechanics — version sync, `## Unreleased` → `## 0.6.0` changelog
> folding, MSRV bookkeeping, and the pre-publish check suite — are driven by the
> `pre-publish-check` process, not roadmap items.

---

## v0.6 — non-breaking backlog (additive; land when ready)

Carried from the v0.5.x deferral list. Not breaking, so they are not v0.6 test content
per se — they broaden the validation surface and bundle naturally with the breaking
pull-forward above.

- **`dashu-cmplx` hyperbolic & inverse-hyperbolic family** — `sinh`/`cosh`/`tanh`/
  `asinh`/`acosh`/`atanh`. (Real hyperbolics already exist on `Context<R>` and are *used*
  by `CBig` trig; the complex-valued functions themselves are deferred.)
- **More transcendentals** — `rootofunity`, complex `agm`, and `exp2`/`exp10`/`log10`.
  (`log2` shipped in v0.6 for the real float and the python bindings; complex `fma`
  shipped in 0.5.x — `CBig::fma` / `Context::fma` in `complex/src/mul.rs`, commit 76502a2,
  via chained real FMA; a truly-correctly-rounded single-rounding complex FMA remains open.)
- **Vector ops** — `dot`/mean helpers. (`Sum` for `CBig` is already exact-accumulating,
  matching `FBig: Sum`; `Product` for `CBig` remains a fold, matching `FBig: Product`.)
- **`num_complex::Complex<FBig>` interop** — conversions both ways. (`CBig` `serde`/`rkyv`/
  `zeroize` shipped in v0.6; the `num-complex` feature flag is scaffolded, the conversions are
  deferred.)
- **Test organization — clear `src` in-file vs `tests/` boundary.** Tests are scattered:
  many operations have *both* an in-file `#[cfg(test)] mod tests` (in `src/<op>.rs`) *and*
  a parallel `tests/<op>.rs` integration file, and the two frequently overlap.
  `dashu-float` is the worst case — `add`, `mul`, `div`, `exp`, `log`, `trig`, `hyper`,
  `root`, `shift`, `round`, `convert`, `cmp`, `io`, and `iter` each appear in both places.
  `AGENTS.md` already states the intended convention (in-file `mod tests` for
  algorithm/kernel correctness; `tests/` for cross-cutting and public-API/property tests),
  but it is not consistently followed. Consolidate: assign each test to the side the
  convention dictates, deduplicate the overlaps, and tighten the `AGENTS.md` wording so
  the boundary is explicit and enforceable. As part of this, clarify the status of the
  `tests/*_prop.rs` property tests against the `AGENTS.md` "in-crate tests must use fixed,
  deterministic inputs" rule — fixed-seed / enum-driven, or moved to `fuzz/`.
- **`dashu-cmplx` infinity model — documentation.** `CBig` represents complex infinity as
  a **single unsigned value** — the point at ∞ of the Riemann sphere ℂ ∪ {∞} (the
  `riemann()` marker in `complex/src/repr.rs`). This is a deliberate design decision,
  recorded here so it is not relitigated or quietly changed.

  **Why it's the right model.** The one-point (Riemann sphere) compactification is the
  canonical model in complex analysis, and the arithmetic rules `dashu-cmplx` implements
  fall straight out of it:

  - `1/0 = ∞` and `1/∞ = 0` — the swap `z ↦ 1/z` is a bijection of the sphere.
  - `finite ± ∞ = ∞`, `finite·∞ = ∞`, `∞·∞ = ∞` — arithmetic extends continuously at ∞.
  - `0·∞` has no continuous extension on the sphere, so it is `FpError::Indeterminate`.

  This is deliberately **not** the C99 / Python `complex` model. C99 derives infinity
  from *signed real and imaginary parts*, admitting a zoo of "complex infinities" (`inf +
  3i`, `inf + inf·i`, …) and a flood of NaN-producing edge cases — widely considered an
  accident of IEEE-754 component-wise semantics. The single Riemann point sidesteps that
  whole class of bugs and matches MPC / analysis conventions rather than C's. So relative
  to the status quo most users have seen, this is a genuine improvement, not just a
  defensible choice.

  **Doc-only follow-ups** so the model isn't surprising to users arriving from C99 or
  from the real-valued `±∞`:

  - State up front in the `CBig` docs/guide that complex ∞ is the single Riemann point
    (not per-component), and list the identities above.
  - **No direction at ∞** — `arg(∞)` and component accessors like `re(∞)`/`im(∞)` are
    undefined. The unsigned model has no direction, unlike C99's directed infinities;
    users arriving from `complex.h` may expect directed behavior (e.g. `re(∞) → +inf`).
  - **Direction-dependent limits are lost** — functions whose limit at ∞ depends on the
    approach direction (`exp`, an essential singularity: `+Re → ∞`, `−Re → 0`, `iRe`
    oscillates; likewise `sin`/`cos`; `arg`) cannot be summarized by a single `exp(∞)`.
    These need explicit error/∞ handling regardless of the infinity model, but the
    unsigned model makes the loss of direction explicit.
  - **`∞ − ∞` resolves to `∞`, not `Indeterminate`.** With one unsigned ∞, negation is
    the identity there, so `∞ − ∞` collapses to `∞ + ∞ = ∞`. The direction-independent
    limit of `z − w` as both → ∞ is indeed ∞, so this is defensible — but it is the one
    genuinely debatable spot, and deserves a one-line note for symmetry with the
    `0·∞ → Indeterminate` rule.
  - **Asymmetry with `dashu-float`** — the real crate has directed `±∞` (correct for the
    extended real line, which has two ends); the complex crate has one unsigned ∞ (correct
    for the one-point compactification). The asymmetry is mathematically right, just
    non-obvious — worth stating alongside the real crate's directed `±∞`.

- **`dashu-float` `exp` guard-bit formulation.** Anchor drifted (`float/src/exp.rs:87` is
  now a doc example); after the v0.6 Ball migration the exp radius is derived mechanically,
  so this item is largely absorbed — the only live residue is the `(x − s·log2)/2ⁿ`
  reduction-form TODO in `float/src/exp.rs`.
- **`dashu-cmplx` signed-zero preservation.** `sin_cos` and `sqr` take a fast path on
  exactly-zero input that returns `+0` components, so several Annex-G / IEEE signed-zero
  cases are not preserved (all numerically equal to `+0`, hence deferred):
  - `csin(-0 + 0i)` returns `+0 + 0i` (should be `-0 + 0i`)
  - `ccos(-0 + 0i)` returns `1 + 0i` (should be `1 - 0i`)
  - `sqr(-0 + 0i)` returns `+0 + i·0` (should be `+0 + i·(-0)`)
  - `clog(-0 ± 0i)`'s `±π` imaginary part is not produced (the zero short-circuit returns
    `-∞ + i·0`)

  (The real-side `exp_m1(-0) = -0` fix that `CBig` sinh/cosh build on *did* land in 0.5.)

## v1.0 — API freeze

With the one breaking change on the 1.0 path (the signed-`isize` `RBig::pow`) shipped in
v0.6, v1.0 is the **stabilization point**: the API freezes at what shipped through v0.6.
No breaking changes are currently scheduled for v1.0 itself; anything that comes up is
folded back into the v0.6 release cycle (or a v0.7) rather than deferred silently to 1.0.

---

## Post-v1 — long-term goals

- **Full C `<tgmath.h>` type-generic math surface.** The complete C standard math library
  for *both* real and complex — trig & inverse; hyperbolic & inverse; the exp/log family
  including `exp2`/`exp10`/`expm1`/`log2`/`log10`/`log1p`; power/root `cbrt`/`hypot`/
  `pow`/`sqrt`; error & gamma `erf`/`erfc`/`tgamma`/`lgamma`; `fma`; rounding/remainder;
  fp-classification — available on both `FBig` and `CBig`
  ([ref](https://en.cppreference.com/c/header/tgmath)). The individual v0.5.x/v0.6 pieces
  above (complex hyperbolics, `fma`, `exp2`/`log2`, …) are the first incremental steps
  toward this.
- **SIMD-optimized FFT multiplication.** Leverage the [`wide`](https://crates.io/crates/wide)
  crate for SIMD FFT-based multiplication. Not considered until v1.0.
- **High-bits (short-product) multiplication for `dashu-float` `mul`/`sqr`/`cubic`.** These
  currently round the *exact* full product/square/cube — always correctly rounded, but
  O(M(n)) in the operand size regardless of the target precision, which is wasteful when
  operands far exceed `precision` (e.g. unlimited-precision significands multiplied to a
  low target precision). The efficient form keeps only the high limbs needed for rounding
  plus a sticky bit for the discarded tail — MPFR's `mpfr_mulhigh_n` (Mulders' short
  product) with a `mpfr_round_p`-style certify-and-fall-back-to-exact gate, so it never
  materializes the full product. Deferred post-v1.0: it needs a new short-product primitive
  in `dashu-int` (none exists today), and the case it optimizes is uncommon.
  (`float/src/mul.rs:173`.)
- **Constant-cache eviction/cap policy** — revisit only if real workloads report memory
  pressure. (No cap today; a `ConstCache` grows until `clear_cache()`/drop, and callers own
  the lifetime explicitly via the `CachedFBig`/`ConstCache` handle.)
