# dashu Roadmap — v0.6 → v1.0

Last updated: 2026-08-10

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

- **More transcendentals** — `rootofunity`.
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
  ([ref](https://en.cppreference.com/c/header/tgmath)). The individual pieces shipped across
  v0.5.x/v0.6 are the first incremental steps toward this.
- **`exp2` / `exp10` for `FBig` and `CBig`, and complex `log10`.** `exp2`/`exp10` need
  their own reduction (`x·log2(e)` → power-of-two scale, mirroring `exp`'s Ball engine);
  complex `log10` follows `CBig::log`'s `ln|z| + i·arg(z)` split divided by `ln 10`.
- **Complex `agm`** — the arithmetic–geometric mean for `CBig` (and real `FBig` if wanted),
  via the standard AGM iteration with Ziv certification. The branch-cut behavior on `]−∞, 0]`
  needs care; no algorithm has been sketched yet.
- **Vector ops** — `dot`/mean helpers. (`Sum` for `CBig` is already exact-accumulating,
  matching `FBig: Sum`; `Product` for `CBig` remains a fold, matching `FBig: Product`.)
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
