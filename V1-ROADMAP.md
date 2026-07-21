# dashu Roadmap — v0.5.x and beyond

Last updated: 2026-07-20

Feature work deferred out of the **v0.5.0** release. The v0.5.x items are all **additive**
(no breaking changes) and safe to ship as point releases on top of 0.5.0; the post-v1 items
are longer-term goals. File:line references are anchors from the v0.5.0 tree and may drift.

> The v0.5.0 release itself — version sync, `## Unreleased` → `## 0.5.0` changelog folding,
> MSRV bookkeeping, and the pre-publish check suite — is release mechanics, not a roadmap item,
> and is driven by the `pre-publish-check` process.

---

## v0.5.x — planned point releases

### Performance & internal cleanups (non-breaking)

- **`dashu-float` division kernel micro-opt** (`float/src/div.rs:344`). Avoid the double
  power in the division kernel; let `q += q0` become `|=` when the base `B` is a power of 2.
- **`dashu-float` `exp` guard-bit formulation** (`float/src/exp.rs:87`). Write down the exact
  formulation of the required guard bits (currently an inline TODO).
- **`dashu-ratio` fast-format SIMD.** Finish the `write_digits` → `DigitWriter` SIMD path —
  the last remaining fast-formatting TODO from the `UBig::to_digits`-driven fmt cleanup.
- **Expose ownership-aware kernels from `dashu-float`.** The `add_val_val` / `add_val_ref` /
  `add_ref_val` / `add_ref_ref` kernels in `float/src/add.rs` are currently `pub(crate)`; make
  them `pub` (or mirror them as `pub` methods on `Context<R>`, e.g.
  `add_val_val(&self, lhs: Repr<B>, rhs: Repr<B>)`), and likewise for `sub`/`mul`/`div` and
  potentially the transcendentals. This lets `dashu-cmplx`'s by-value operator impls exploit
  ownership instead of borrowing every `CBig` operand through `Context::add(&CBig, &CBig)`
  (which takes `&Repr` internally and clones as needed) — today the ownership advantage of
  `impl Add for CBig` is lost.

### Correctness

- **Guaranteed-correct rounding (Ziv retry loop).** ✅ *Delivered for all real and complex
  transcendentals.* `exp`, `exp_m1`, `ln`, `ln_1p`, the trigonometric family (`sin`, `cos`,
  `sin_cos`, `tan`, `asin`, `acos`, `atan`, `atan2`), the hyperbolic family (`sinh`, `cosh`,
  `sinh_cosh`, `tanh`, `asinh`, `acosh`, `atanh`), `hypot`, `powi`, and `powf` are
  guaranteed-correctly rounded via a Ziv retry loop (`Context::ziv`/`ziv_pair`, driven by the
  `ErrorBounds` preimage). Series transcendentals (trig, atan) use near-correct `_compute` cores the
  wrapper certifies; composition transcendentals treat the Ziv-correct primitives as black boxes.
  `powf`'s `exp(y·ln x)` amplification is handled by a work-precision radius
  (`result.ulp()·(|y·ln x|+1)·(B+8)`) that shrinks as `B^{-guard}`, so the containment test
  converges. `powi` (repeated squaring) carries a radius that scales with the squaring-compounding
  error (`2^bit_len(n)·ulp`) and drops to zero when the chain is exact — required under the directed
  rounding modes (`FBig`/`CBig` default to `mode::Zero`), where an exactly-representable result sits
  on a one-sided rounding boundary. `dashu-cmplx`'s complex transcendentals
  (`exp`/`log`/`powf`/`powi`/`sin`/`cos`/`tan`/`sin_cos`/`asin`/`acos`/`atan`/`sqrt`) are
  correctly rounded via their own complex Ziv driver (certifies both parts); `asin`/`acos` use the
  factored `1-z²=(1-z)(1+z)` to stay accurate right up to `z = ±1`.
- **Signed-zero preservation in `CBig` zero short-circuits.** `sin_cos` and `sqr` take a fast
  path on exactly-zero input that returns `+0` components, so several Annex-G / IEEE signed-zero
  cases are not preserved (all numerically equal to `+0`, hence deferred):
  - `csin(-0 + 0i)` returns `+0 + 0i` (should be `-0 + 0i`)
  - `ccos(-0 + 0i)` returns `1 + 0i` (should be `1 - 0i`)
  - `sqr(-0 + 0i)` returns `+0 + i·0` (should be `+0 + i·(-0)`)
  - `clog(-0 ± 0i)`'s `±π` imaginary part is not produced (the zero short-circuit returns
    `-∞ + i·0`)

  (The real-side `exp_m1(-0) = -0` fix that `CBig` sinh/cosh build on *did* land in 0.5.)

### `dashu-cmplx` feature follow-ups

Consolidated from the original `CBig` design. All additive.

- **Complex hyperbolic & inverse-hyperbolic family** — `sinh`/`cosh`/`tanh`/`asinh`/`acosh`/`atanh`.
  (Real hyperbolics already exist on `Context<R>` and are *used* by `CBig` trig in 0.5; the
  complex-valued functions themselves are deferred.)
- **More transcendentals** — complex `fma` (fused multiply-add — hard to round correctly),
  `rootofunity`, complex `agm`, and `exp2`/`exp10`/`log2`/`log10`.
- **Vector ops** — `dot`/mean helpers. (`Sum` for `CBig` is already exact-accumulating, matching
  `FBig: Sum`; `Product` for `CBig` remains a fold, matching `FBig: Product`.)
- **Independent re/im rounding** — a `CRound` trait giving MPC `mpc_rnd_t` parity (0.5 uses a
  single `R` for both parts).
- **Third-party integration** — `CBig` `serde`/`rkyv`/`zeroize`, and
  `num_complex::Complex<FBig>` interop. (The `serde`/`num-traits`/`num-complex` feature flags
  are scaffolded in 0.5; the impls are deferred.)

### Lint gates (MSRV-gated)

- **`#![deny(clippy::allow_attributes_without_reason)]`** once the MSRV reaches **≥ 1.81**. It
  needs the `reason = "..."` field on every `#[allow]` (or `#[expect]`), both stabilized in
  Rust 1.81, which conflicts with the current 1.68 MSRV.

---

## Post-v1 — long-term goals

- **Full C `<tgmath.h>` type-generic math surface.** The complete C standard math library for
  *both* real and complex — trig & inverse; hyperbolic & inverse; the exp/log family including
  `exp2`/`exp10`/`expm1`/`log2`/`log10`/`log1p`; power/root `cbrt`/`hypot`/`pow`/`sqrt`; error &
  gamma `erf`/`erfc`/`tgamma`/`lgamma`; `fma`; rounding/remainder; fp-classification — available
  on both `FBig` and `CBig` ([ref](https://en.cppreference.com/c/header/tgmath)). The individual
  v0.5.x pieces above (complex hyperbolics, `fma`, `exp2`/`log2`, …) are the first incremental
  steps toward this.
- **SIMD-optimized FFT multiplication.** Leverage the [`wide`](https://crates.io/crates/wide)
  crate for SIMD FFT-based multiplication. Not considered until v1.0.
- **MSRV bumps** — deferred unless forced by a dependency.
- **Constant-cache eviction/cap policy** — revisit only if real workloads report memory
  pressure. (0.5 has no cap; a `ConstCache` grows until `clear_cache()`/drop, and callers own
  the lifetime explicitly via the `CachedFBig`/`ConstCache` handle.)
