# dashu Roadmap — v0.5.x and beyond

Last updated: 2026-07-28

Feature work deferred out of the **v0.5.0** release. The v0.5.x items are all **additive**
(no breaking changes) and safe to ship as point releases on top of 0.5.0; the post-v1 items
are longer-term goals. File:line references are anchors from the v0.5.0 tree and may drift.

> The v0.5.0 release itself — version sync, `## Unreleased` → `## 0.5.0` changelog folding,
> MSRV bookkeeping, and the pre-publish check suite — is release mechanics, not a roadmap item,
> and is driven by the `pre-publish-check` process.

---

## v0.5.x — planned point releases

### Performance & internal cleanups (non-breaking)

- **`dashu-float` division kernel** (`float/src/div.rs:344`). *Investigated 2026-07; the two
  micro-opts in the inline TODO are not viable.* (1) `q |= q0` is unsafe: `Repr::significand` is a
  *signed* `IBig`, so the quotient can be negative and `|=` ≠ `+=` (it broke `to_f64` rounding).
  (2) Sharing the radix power across the two `shl_digits_in_place` calls — and precomputing a
  `ConstDivisor` for the shared `rhs.significand` divisor — both measured **neutral** on
  `float/benches/primitive.rs` (dbig_div/1e3: p=0.11 and p=0.49). The big-int `div_rem` dominates,
  and the affected refinement branch is only hit in a fraction of the random-input benchmark. A
  real win needs an algorithmic change to the division itself, not these micro-opts.
- **`dashu-float` `exp` guard-bit formulation** (`float/src/exp.rs:87`). Write down the exact
  formulation of the required guard bits (currently an inline TODO).
- **Expose ownership-aware kernels from `dashu-float`.** The `add_val_val` / `add_val_ref` /
  `add_ref_val` / `add_ref_ref` kernels in `float/src/add.rs` are currently private; make
  them `pub` (or mirror them as `pub` methods on `Context<R>`, e.g.
  `add_val_val(&self, lhs: Repr<B>, rhs: Repr<B>)`), and likewise for `sub`/`mul`/`div` and
  potentially the transcendentals. This lets `dashu-cmplx`'s by-value operator impls exploit
  ownership instead of borrowing every `CBig` operand through `Context::add(&CBig, &CBig)`
  (which takes `&Repr` internally and clones as needed) — today the ownership advantage of
  `impl Add for CBig` is lost.
- **Test organization — clear `src` in-file vs `tests/` boundary.** Tests are scattered: many
  operations have *both* an in-file `#[cfg(test)] mod tests` (in `src/<op>.rs`) *and* a
  parallel `tests/<op>.rs` integration file, and the two frequently overlap. `dashu-float`
  is the worst case — `add`, `mul`, `div`, `exp`, `log`, `trig`, `hyper`, `root`, `shift`,
  `round`, `convert`, `cmp`, `io`, and `iter` each appear in both places. `AGENTS.md` already
  states the intended convention (in-file `mod tests` for algorithm/kernel correctness;
  `tests/` for cross-cutting and public-API/property tests), but it is not consistently
  followed. Consolidate: assign each test to the side the convention dictates, deduplicate the
  overlaps, and tighten the `AGENTS.md` wording so the boundary is explicit and enforceable.
  As part of this, clarify the status of the `tests/*_prop.rs` property tests against the
  `AGENTS.md` "in-crate tests must use fixed, deterministic inputs" rule — fixed-seed /
  enum-driven, or moved to `fuzz/`.

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
- **More transcendentals** — `rootofunity`, complex `agm`, and `exp2`/`exp10`/`log2`/`log10`.
  (Complex `fma` shipped in 0.5.x — `CBig::fma` / `Context::fma` in `complex/src/mul.rs`,
  commit 76502a2, via chained real FMA; a truly-correctly-rounded single-rounding complex
  FMA remains open.)
- **Vector ops** — `dot`/mean helpers. (`Sum` for `CBig` is already exact-accumulating, matching
  `FBig: Sum`; `Product` for `CBig` remains a fold, matching `FBig: Product`.)
- **Independent re/im rounding** — a `CRound` trait giving MPC `mpc_rnd_t` parity (0.5 uses a
  single `R` for both parts).
- **Third-party integration** — `CBig` `serde`/`rkyv`/`zeroize`, and
  `num_complex::Complex<FBig>` interop. (The `serde`/`num-traits`/`num-complex` feature flags
  are scaffolded in 0.5; the impls are deferred.)

### `dashu-cmplx` infinity model — documentation (non-breaking)

`dashu-cmplx` represents complex infinity as a **single unsigned value** — the point at ∞
of the Riemann sphere ℂ ∪ {∞} (the `riemann()` marker in `complex/src/repr.rs`). This is a
deliberate design decision, recorded here so it is not relitigated or quietly changed.

**Why it's the right model.** The one-point (Riemann sphere) compactification is the
canonical model in complex analysis, and the arithmetic rules `dashu-cmplx` implements fall
straight out of it:

- `1/0 = ∞` and `1/∞ = 0` — the swap `z ↦ 1/z` is a bijection of the sphere.
- `finite ± ∞ = ∞`, `finite·∞ = ∞`, `∞·∞ = ∞` — arithmetic extends continuously at ∞.
- `0·∞` has no continuous extension on the sphere, so it is `FpError::Indeterminate`.

This is deliberately **not** the C99 / Python `complex` model. C99 derives infinity from
*signed real and imaginary parts*, admitting a zoo of "complex infinities" (`inf + 3i`,
`inf + inf·i`, …) and a flood of NaN-producing edge cases — widely considered an accident of
IEEE-754 component-wise semantics. The single Riemann point sidesteps that whole class of
bugs and matches MPC / analysis conventions rather than C's. So relative to the status quo
most users have seen, this is a genuine improvement, not just a defensible choice.

**Doc-only follow-ups** so the model isn't surprising to users arriving from C99 or from the
real-valued `±∞`:

- State up front in the `CBig` docs/guide that complex ∞ is the single Riemann point (not
  per-component), and list the identities above.
- **No direction at ∞** — `arg(∞)` and component accessors like `re(∞)`/`im(∞)` are
  undefined. The unsigned model has no direction, unlike C99's directed infinities; users
  arriving from `complex.h` may expect directed behavior (e.g. `re(∞) → +inf`).
- **Direction-dependent limits are lost** — functions whose limit at ∞ depends on the
  approach direction (`exp`, an essential singularity: `+Re → ∞`, `−Re → 0`, `iRe`
  oscillates; likewise `sin`/`cos`; `arg`) cannot be summarized by a single `exp(∞)`.
  These need explicit error/∞ handling regardless of the infinity model, but the unsigned
  model makes the loss of direction explicit.
- **`∞ − ∞` resolves to `∞`, not `Indeterminate`.** With one unsigned ∞, negation is the
  identity there, so `∞ − ∞` collapses to `∞ + ∞ = ∞`. The direction-independent limit of
  `z − w` as both → ∞ is indeed ∞, so this is defensible — but it is the one genuinely
  debatable spot, and deserves a one-line note for symmetry with the `0·∞ → Indeterminate`
  rule.
- **Asymmetry with `dashu-float`** — the real crate has directed `±∞` (correct for the
  extended real line, which has two ends); the complex crate has one unsigned ∞ (correct
  for the one-point compactification). The asymmetry is mathematically right, just
  non-obvious — worth stating alongside the real crate's directed `±∞`.

### Lint gates (MSRV-gated)

- **`#![deny(clippy::allow_attributes_without_reason)]`** once the MSRV reaches **≥ 1.81**. It
  needs the `reason = "..."` field on every `#[allow]` (or `#[expect]`), both stabilized in
  Rust 1.81, which conflicts with the current 1.68 MSRV.

---

## v1.0 — planned breaking changes

- **Signed exponent on `RBig::pow` / `Relaxed::pow`.** Both the native `pow(&self, n)`
  method and the `num_traits::Pow` impl currently take an unsigned `usize` exponent, so
  raising to a negative power needs an explicit reciprocal. v1.0 widens them to `isize`
  (negative exponents reciprocate first, matching `powf` on the floats). The signed
  kernel already ships in 0.5.x — a private `Repr::pow_signed` plus a
  `num_traits::Pow<isize>` impl — so the behavior is exercisable today; the v1.0 break
  is purely the signature change on the native `pow` (and folding the `Pow` impl over to
  `isize`).

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
