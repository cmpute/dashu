# dashu v0.5 Release Plan

Last updated: 2026-06-28

This document is the consolidated plan for the **v0.5** release — a **major (breaking)** bump.
Because it is a major release, its two organizing goals are:

1. **Clear every breaking TODO** accumulated since 0.4 (deprecations, API warts, format changes).
2. **Ship the headline features**: a shared math-constant cache for `dashu-float`, an
   arbitrary-precision complex-number crate (`dashu-cmplx`, targeting GNU MPC parity), and a
   full mdBook user guide under `guide/`.

A hard precondition from the maintainer: **before adding new features, close the test, benchmark,
and fuzz gaps on existing functions** so the new features cannot regress performance or correctness.

---

## Guiding principles & ordering rationale

The phases below are ordered by dependency, not by "importance". The logic is:

- **Hardening first** — explicitly requested as a gate; it also becomes the safety net that lets us
  touch core code (breaking changes, cache, complex) with confidence.
- **Breaking changes before features** — API surgery is cheapest while no new code depends on the
  old shapes; several breaking items (e.g. `Context: !Copy` from the cache, `UBig::to_digits`
  enabling rational-fmt cleanup) are prerequisites for the feature work itself.
- **Float cache before complex** — the cache makes the real transcendental functions (`exp`, `ln`,
  `sin`, `cos`, …) fast and is itself a breaking change to `Context`; complex arithmetic leans on
  those real functions heavily, so building CBig on an already-cached float gives correct + fast
  results by default. It is also ~half done on `origin/float-cache`.
- **Complex is the big new surface** — it depends on a solid, cached `dashu-float`.
- **Guide content last** — it must document the *final* 0.5 API (including CBig). Its *infrastructure*
  (mdBook config + CI deploy) can start in parallel with Phase 0.
- **Release prep last** — version sync, changelog consolidation, meta-crate wiring.

### Roadmap at a glance

| Phase | Theme | Blocking? | Depends on |
|-------|-------|-----------|------------|
| 0 | Test / benchmark / fuzz hardening | **GATE for all feature work** | — |
| 1 | Breaking changes & deprecation cleanup | must land in 0.5 | 0 (ideally) |
| 2 | `dashu-float` shared constant cache | ✅ done (#83, as `CachedFBig`/`ConstCache`) | 0, 1 |
| 3 | `dashu-cmplx` (`CBig`) — new crate | ✅ done (M1–M6) | 0, 2 |
| 4 | The mdBook guide | required deliverable | 1, 2, 3 (content); infra can start now |
| 5 | Release prep & version sync | — | 1–4 |

> Parallelism: Phase 0 hardening, Phase 1 cleanups, and Phase 4 mdBook **infrastructure** can all
> proceed concurrently. Phase 4 **content** must trail Phases 1–3.

---

## Phase 0 — Test, Benchmark & Fuzz Hardening  *(GATE)*

> **Implemented.** A pure-Rust `proptest` net now runs in the existing per-PR `test` job across the
> full `force_bits` 16/32/64 matrix (no GMP, no new CI jobs). It covers float trig / exp / ln / pow /
> nth-root identities + a `ln` correct-rounding self-oracle, the `_dword` fast paths (differential vs
> the generic path), the `arch` `add_with_carry`/`sub_with_borrow`/digit-SWAR kernels, and rational
> arithmetic identities. `proptest` is pinned to `~1.7` (MSRV 1.66, matches the existing `rand_v09`
> copy; the MSRV CI job drops dev-deps, so it is unaffected). The strong rug/MPFR differential stays
> in the excluded `fuzz/` crate, run manually before a release. `PROPTEST_CASES=256` is set in the
> `test` job env to keep per-PR cost bounded.
>
> Benchmark gaps filled (compile-guarded by the existing clippy `--all-targets` job; not run in CI):
> `float/benches/trig.rs`, FBig groups in `float/benches/io.rs`, `integer/benches/shift.rs` (+IBig),
> IBig groups in `integer/benches/primitive.rs`, and rational reduction + RBig→FBig conversion benches.
>
> **Bonus:** the trig property tests surfaced and fixed a real signed-zero regression — `sin`/`cos`/
> `tan`/`sin_cos`/`asin`/`acos`/`atan`/`atan2` panicked on tiny *negative* inputs, because `round()`
> produced `-0` (sentinel exponent) which `IBig::try_from` rejected during argument reduction. Fixed
> by extracting the quadrant integer via `to_int`; see `float/CHANGELOG.md` and the
> `test_trig_tiny_negative_no_panic` regression test.

> *No committed baseline.* Benchmark numbers are hardware-dependent, so none are checked in — run the
> benches locally before/after a perf-sensitive change to spot regressions.


---

## Phase 1 — Breaking Changes & Deprecation Cleanup

Every item here changes public API and **must** land in 0.5. File:line refs are from the current
`v05`/`master` tree. Grouped by crate.

### 1.1 `dashu-base`
- [x] **Remove `AbsEq` trait** (deprecated `since = "0.5.0"` at `base/src/sign.rs:43`). Fold its
      semantics into `AbsOrd`, then delete the trait. Cleanup sites:
      `integer/src/cmp.rs:2`, `rational/src/cmp.rs:1` (remove the module-level
      `#![allow(deprecated)]` + their TODOs), and `base/src/sign.rs:296`/`:328` test `#[allow]`s.

### 1.2 `dashu-int`
- [x] **`IBig` serde wire format** (`integer/src/third_party/serde.rs:63`): switch to
      `IBig::to_le_bytes()` for interop robustness. (Breaking serialization format.)
- [ ] **`#![deny(missing_docs)]`** across all crates (`integer/src/lib.rs:72-74` also proposes
      `#![deny(clippy::allow_attributes_without_reason)]`). Requires documenting every public item
      first — pair with Phase 4 guide work (move prose to the guide, keep doc-comments concise).
      **→ deferred to Phase 4** (paired with the guide doc-pass).
- [x] **`UBig::to_digits` / `from_digits`** (`integer/src/convert.rs:1142`): new public API
      supporting base up to `Word::MAX` — **shipped as full base-up-to-`Word::MAX` with `Vec<Word>`
      digits**; the `u8` note in the TODO referred to the separate `UBig::in_radix`/`IBig::in_radix`
      and internal `Digit` type `u32→u8` change (also done). Enables `rational` fmt cleanup (see 1.4).
- [x] Consolidate already-applied breaking changes from `## Unreleased`: NTT threshold-var renames
      (`_MUL` suffix), Proth-prime NTT, etc. — these just need to land under the 0.5 changelog.

### 1.3 `dashu-float`
- [x] **Remove `from_str_native`** (deprecated `since = "0.5.0"` at `parse.rs:23` on `Repr`,
      `parse.rs:228` on `FBig`). Make private; users go through `core::str::FromStr`. Cleanup:
      `parse.rs:1` module `#![allow(deprecated)]`, `third_party/serde.rs:64` & `:140`,
      `third_party/num_traits.rs:139`.
- [x] **Float serde precision padding** (`third_party/serde.rs:39`): pads the human-readable string
      with trailing zeros to `precision` significant digits so precision round-trips. *(Decision
      resolved: apply.)*

> *Implemented in #83 and removed from this list:* the infinity/NaN panic policy (infinities are now
> terminal values; `FpResult<T> = Result<Rounded<T>, FpError>`; full IEEE-754 signed zero) — see
> `guide/src/ieee754.md` and `float/CHANGELOG.md`.

### 1.4 `dashu-ratio`
- [x] **`From<Repr> for FBig` → `TryFrom`** (`rational/src/third_party/dashu_float.rs:12`): make the
      conversion fallible (succeed only when exact). A ready stub `fbig_try_from_rbig` exists at
      `:26`. Remove the dead-code stub TODO at `:24` and the `#[allow(dead_code)]`.
- [x] Wire `UBig::to_digits` into `fmt/expanded.rs` (integer part, direct) and batch the fractional
      long division through a precomputed `dashu_int::fast_div::ConstDivisor` (one division per word
      of digits) on the non-repetend path — done. The remaining `write_digits`→`DigitWriter` SIMD
      TODO is the only fast-fmt item left for 0.5.x. Non-breaking internal perf, gated on 1.2.

### 1.5 Doc / internal (non-breaking, fold in opportunistically)
- [ ] **Move verbose type prose to the guide** — `integer/src/ubig.rs:10` `TODO(v0.5)` (leave a brief
      summary + link in the doc-comment). Pairs with Phase 4.
- [ ] **`integer/src/pow.rs:67`** — switch to right-to-left exponentiation (cheaper squaring schedule).
- [ ] **`float/src/div.rs:344`** — avoid the double power in the division kernel; let `q += q0` become
      `|=` when `B` is a power of 2.
- [ ] **`float/src/exp.rs:87`** — write down the exact formulation of the required guard bits.

---

## Phase 2 — `dashu-float` Shared Constant Cache

> **Implemented in #83** as the public **`ConstCache`** type + the **`CachedFBig`** wrapper (carries
> `Rc<RefCell<ConstCache>>`; its transcendental ops thread the handle through `Context`). `Context`
> and `FBig` stay `Copy + Send + Sync + no_std`; the constant-source `Context` methods take a breaking
> `cache: Option<&mut ConstCache>` parameter (high-level `FBig` passes `None`). `ConstCache` is
> `Send + Sync`, so `Arc<Mutex<ConstCache>>` variants are also possible. π's base-free `√10005` isqrt
> is cached too. See `float/src/math/cache.rs`, `float/src/fbig_cached.rs`, and `guide/src/construct.md`.

### Cached aliases (approach B)

Keep `dashu::Real`/`dashu::Decimal` as the safe baseline (`FBig`/`DBig` — complete trait surface,
`Send + Sync`), and **promote `CachedFBig` under short aliases** so transcendental-heavy users reach
for the faster type by name. Rationale for *not* redirecting the bare aliases: `CachedFBig` is
`!Send + !Sync` (carries `Rc<RefCell<ConstCache>>`), has a thinner trait surface than `FBig` today,
and its per-value cache only helps *within one computation chain* — so it is not a safe drop-in for
the primary `Real`/`Decimal`.

- [ ] **Add aliases** in the meta-crate alongside `Real`/`Decimal`:
      `pub type FastReal = dashu_float::CachedFBig;` and
      `pub type FastDecimal = dashu_float::CachedFBig<dashu_float::round::mode::HalfAway, 10>;`.
- [ ] **Bring `CachedFBig` to `FBig`'s trait surface** so the aliases are ergonomic (delegate to the
      inner `FBig`): `Display`/`FromStr`, `PartialOrd`/`Ord`, `Sum`/`Product`, and the `third_party`
      impls (serde/num-traits/rand).
- [ ] **Guide:** document `FastReal`/`FastDecimal` as the recommended type when calling many
      transcendentals (π/ln2 reuse, progressive cache extension), with the `!Send + !Sync` +
      per-value-cache caveats (add to `guide/src/construct.md`).

### Still open

- [ ] **Memory growth:** no eviction/cap/shrink policy — a 1M-digit π lives in the cache until
      `clear_cache()`/drop. Decide whether 0.5 ships a cap or just documents it.

> *Resolved:* **no `thread_local!` / global-cache convenience layer.** The explicit `CachedFBig` API
> (plus the `FastReal`/`FastDecimal` aliases above) is the supported fast path; thread-local
> hidden state is rejected (and is a `no_std`/`std`-only friction).

---

## Phase 3 — `dashu-cmplx` (`CBig`) — Arbitrary-Precision Complex Numbers

> **✅ Implemented** (M1–M6 complete). The `dashu-cmplx` crate (dir `complex/`) provides `CBig` — two
> `Repr` parts over a single shared `Context` — targeting GNU MPC parity for the "common
> functionalities," built on a cached `dashu-float` (Phase 2). Two-layer API mirroring `FBig`
> (`Context::mul → CfpResult<CRounded<CBig>>` at the context layer; operators → `CBig` at the
> convenience layer), near-correct rounding via the guard-digit recipe, and the C99 Annex G / Kahan
> no-NaN model (C99 NaN-producing cases → `FpError`). The module layout mirrors `dashu-float`
> (`add`/`mul`/`div`; `exp` hosts the power family; `math/` for transcendentals; `repr.rs` for
> `Context`). Verified with proptest identities + self-oracles, deterministic Annex-G vectors, and a
> manual `rug::Complex`/MPC oracle in `fuzz/`. `NumHash` mirrors `num-complex`'s `Complex<f64>`
> algebraic hash (verified against the `num-order` reference). See `complex/CHANGELOG.md` (0.5.0).

**Goal:** a new crate `dashu-cmplx` (dir `complex/`) providing an arbitrary-precision complex type
`CBig`, targeting GNU MPC parity for "common functionalities." It composes two parts (`re`, `im`)
over a shared precision, with a single rounding mode applied to both components.

### 3.1 Type & context model — ✅
- [x] `CBig<R: Round = Zero, const B: Word = 2> { re: Repr<B>, im: Repr<B>, context: Context<R> }` —
      two parts over a single shared `Context<R>` (re/im kept at the same precision; MPC allows
      different precisions but we start uniform — simpler, matches `FBig`'s single-context model).
- [x] A single `R: Round` applies to both the real and imaginary parts (simpler than MPC's `(R, R)`
      pair; per-axis independent rounding is deferred to 0.5.x). Reuses `dashu-float`'s `Round` trait;
      no new rounding machinery.
- [x] Constants: `CBig::ZERO`, `ONE`, `I` (the imaginary unit). No `INFINITY` constant — complex
      infinity is the single Riemann point produced by `proj` (`+∞ + i·0`), per the C99 Annex G model
      `dashu-float` already follows (`Repr` already encodes ±∞).

### 3.2 Core surface for v0.5 ("common functionalities") — ✅
- [x] **Construction & conversion:** `from_parts`, `From<FBig>`/`From<UBig>`/`From<IBig>`,
      `TryFrom<CBig> for FBig`/`for IBig`, `FromStr`, `TryFrom<f32>`/`<f64>` (`num_complex::Complex`
      interop is deferred to 0.5.x — see 3.4).
- [x] **Field arithmetic:** `add`/`sub`/`mul`/`div`/`neg`/`sqr`/`inv`, `powi`, scalar `mul`/`div` by
      real `FBig` (mixed-type operators, not named methods), and operator overloads.
      **Near-correctly-rounded** `mul`/`div` via Smith's method + guard-digit re-round (mirroring
      `FBig`'s own transcendentals; a guaranteed-correct Ziv loop is deferred to 0.5.x).
- [x] **Comparison:** `PartialEq`/`Eq`, a lexicographic `Ord` (by `re`, then `im`), and
      `AbsOrd`/`NumOrd`/`NumHash` — mirroring `FBig`'s surface, not MPC's "complex has no order" stance.
- [x] **Decomposition / misc:** `re()`/`imag()`/`into_parts()`/`from_parts()`, `conj()`, `abs()`
      (modulus via `hypot`), `norm()` (squared modulus), `arg()` (principal argument), `proj()`
      (Riemann projection), `mul_i()`.
- [x] **Powers & elementary transcendentals:** `sqrt` (non-negative real part; ties to non-negative
      imaginary), `exp`, `log` (principal, branch cut on the negative real axis, `Im ∈ ]-π, π]`),
      `powf` (complex^complex) and `powi` (complex^integer), `sin`, `cos`, `tan`, `sin_cos`,
      `asin`, `acos`, `atan`.
      *Reuses `FBig`'s real implementations; the complex identities are*
      `exp(x+iy)=eˣ(cos y + i sin y)`, `log z = ln|z| + i·arg z`, and
      `sin/cos` via the real–imaginary form using `FBig`'s `sin`/`cos` + `sinh`/`cosh` (`exp(±iz)` only as a test cross-check).
- [x] **I/O:** `Display`/`Debug`/`FromStr` in algebraic `a+bi` form (the `num-complex` idiom, not
      MPC's `(re im)` parenthesized pair).
- [x] **Integration:** `complex/` in the workspace `members`/`default-members`; re-exported as
      `dashu::complex` with alias `dashu::Complex = CBig`. The `cbig!`/`static_cbig!` literal macros
      shipped (M6). `rand` generation: `UniformCBig` (box sampler) + builtin `Standard`/`Open01`/
      `OpenClosed01` (unit square), default `rand_v08` with `rand_v09`/`rand_v010` opt-in.

### 3.3 Correctness bar — ✅
- [x] Follows **C99 Annex G / Kahan** branch cuts and principal values exactly (`sqrt`/`log` cut on
      `]-∞, 0]`, etc.).
- [x] Signed-zero and infinite-operand edge cases (`powf(0,0) = 1`, `proj` on infinities, C99
      NaN-producing cases mapped to `FpError`), wired into the `FpResult`/`CfpResult` machinery.
- [x] **Fuzz vs MPC/rug oracle**: property tests (identities: `exp(log z) ≈ z`, `log z · conj`
      realness, `sin²+cos²≈1`, de Moivre) in `complex/tests/{arith,rounding,transcendental}_prop.rs`,
      deterministic Annex-G vectors in `special_values.rs`, and `rug::Complex`/MPC oracle comparisons
      in the manual `fuzz/` crate.

### 3.4 Deferred to v0.5.x *(explicitly out of scope for this release)*

Consolidated from the original `CBig` design doc (`TODO-cmplx.md`, now folded into this section and
removed). All additive — safe as point releases under 0.5.x.

- **Guaranteed-correct rounding (Ziv retry loop)** — 0.5 ships near-correct guard-digit rounding
  (matching `FBig`); a Ziv loop is expected to land in `FBig` first, then inherited by `CBig`.
- **Complex hyperbolic & inverse-hyperbolic family** (`sinh`/`cosh`/`tanh`/`asinh`/`acosh`/`atanh`).
  (Real hyperbolics already exist on `Context<R>` and are *used* by `CBig` trig in 0.5; the
  complex-valued functions themselves are deferred.)
- **`fma`** (complex fused multiply-add — hard to round correctly), **`rootofunity`**, complex
  **`agm`**, **`exp2`/`exp10`/`log2`/`log10`**.
- **Vector ops** (`sum`/`dot`/mean) — note `Sum`/`Product` for `CBig` (the `iter` analog of `FBig`)
  are also not yet implemented.
- **Third-party integration:** `CBig` `serde`/`rkyv`/`zeroize`; `num_complex::Complex<FBig>` interop
  (the `serde`/`num-traits`/`num-complex` feature flags are scaffolded; impls deferred).
- **Independent re/im rounding** (`CRound` trait; MPC `mpc_rnd_t` parity — 0.5 uses one `R` for both
  parts).
- **A `ComplexFloat`-style trait** unifying `FBig` and `CBig` (sealed, for generic real/complex code).
- **Ball arithmetic** (the `mpcb_t` analogue — interval/uncertainty complex).
- **`CachedCBig`** — a cache-backed variant mirroring `CachedFBig`. Its structure is settled (so 0.5
  is forward-compatible): it wraps a `CBig` plus a shared `Rc<RefCell<dashu_float::ConstCache>>`
  handle, reusing `ConstCache` unchanged from `dashu-float` (there are no complex-specific constants
  to cache — `CBig`'s transcendentals are built entirely from real `FBig` ops). `CachedCBig` is
  `!Send + !Sync` while `CBig` stays `Send + Sync` (so `static_cbig!` produces `CBig`). **This is why
  0.5 already threads `cache: Option<&mut ConstCache>` through the transcendental `Context` ops:** the
  convenience layer passes `None`, `CachedCBig` will pass `Some(&mut cache)`, so adding the cached
  variant needs no signature change.

---

## Phase 4 — The mdBook Guide

**Goal:** a complete user guide under `guide/`, built with mdBook, deployed from CI. Today the guide
is ~15–20% complete: `index.md`, `SUMMARY.md`, `types.md`, `construct.md`, and most of `convert.md`
are real; the other 13 files are stubs or empty; `book.toml` is minimal (no plugins, no renderer);
and nothing in CI builds or deploys it.

### 4.1 Infrastructure (can start in Phase 0, parallel)
- [ ] Extend `guide/book.toml`: add `[output.html]`, `mdbook-katex` (math typesetting — essential
      for float/complex), `mdbook-toc`, and `mdbook-admonish` if desired. Pin versions.
- [ ] Stop committing the rendered `guide/book/` output (`.gitignore` already lists `book`); build
      in CI instead. (The committed `book/` + `.nojekyll` suggests a stale GitHub-Pages deploy.)
- [ ] Add a CI workflow: `cargo install mdbook` + plugins → `mdbook build guide` → deploy to GitHub
      Pages. Re-enable the commented-out **Book** badge in `README.md`.

### 4.2 Content (after Phases 1–3 so it documents the final API)
- [ ] Fill the 13 stub/empty pages: `io/{index,parse,print,serialize,interop}.md`,
      `ops/{index,basic,cmp,bit,exp_log,num_theory}.md`, `faq.md`, `cheatsheet.md`, and the
      `convert.md` FBig TODO sections.
- [ ] Expand `SUMMARY.md` as needed and **add new chapters** for v0.5 surfaces:
      - the **constant cache** (`ConstCache` / `CachedFBig`; a section already exists in
        `construct.md` — promote/expand it into full coverage),
      - **`CBig` complex numbers** (construction, arithmetic, transcendentals, branch cuts).
- [ ] Migrate verbose API prose out of doc-comments into the guide (per `integer/src/ubig.rs:10`),
      leaving concise rustdoc behind — pairs with the `#![deny(missing_docs)]` work in 1.2.
- [ ] Use the existing crate-level doctests (`dashu-int`, `dashu-float`, `dashu-ratio` `lib.rs`)
      and `integer/examples/factorial.rs` as seed material.
- [ ] Cross-check MSRV statement in `guide/src/index.md` (currently "1.68") against the 0.5 decision.

---

## Phase 5 — Release Preparation

- [ ] **Version sync:** bump *all* crates to `0.5.0` and align (currently skewed: float 0.4.4,
      meta 0.4.3, base/int/ratio/macros 0.4.2). Refresh all internal `version = "0.4.x"` path pins.
- [ ] **Workspace:** add `complex` to `members`/`default-members`; wire meta-crate feature
      forwarding for any new `dashu-cmplx` features (serde, num-traits, etc.).
- [ ] **Feature flags:** if `dashu-cmplx` adds third-party integration, follow the `xxx_vYY` +
      unversioned-alias convention; update the top-level `Cargo.toml` forwarding table.
- [ ] **Changelogs:** fold every crate's `## Unreleased` into a `## 0.5.0` section (breaking
      changes under `### Change`/`### Remove`, features under `### Add`).
- [ ] **MSRV:** review whether 0.5 still targets 1.68 (mdBook is a build-time tool and does not
      affect runtime MSRV; cache uses only `alloc`; no forced bump expected — confirm and keep
      unless a desired feature needs newer). Update `README.md` badge + all `rust-version` fields.
- [ ] **CI:** run the pre-publish checks (`pre-publish-check` skill):
      `cargo check --all-features --tests`, `cargo test --workspace --exclude dashu-python`,
      `cargo clippy --all-features --all-targets --workspace --exclude dashu-python -- -D warnings`,
      `cargo fmt --all -- --check`.
- [ ] **Docs:** confirm `dashu::complex` / `dashu::Complex` render on docs.rs; publish order base →
      int → float → ratio → complex → macros → meta.

---

## Risk Register

| Risk | Mitigation |
|------|------------|
| Near-correctly-rounded complex mul/div is hard | Guard-digit re-round mirroring `FBig`; fuzz vs MPC/rug oracle (Phase 0.2/3.3). Guaranteed-correct Ziv is a 0.5.x follow-up |
| Trig/exp correctness is currently unverified in CI | Phase 0.2 *before* CBig consumes those functions |
| Cache memory unbounded growth | Decide cap/shrink policy (Phase 2); at minimum document |
| Guide content churn if written before API freeze | Content trails Phases 1–3; only infra starts early |
| Version skew complicates publishing | Phase 5 sync; pin internal deps to `0.5.0` |
| `_dword` paths under-tested yet "first-class" | Phase 0.2 direct tests; required before trusting complex on float |

---

## Out of Scope for v0.5

- `dashu-python` remains excluded and out of the release critical path (per `AGENTS.md`).
- All `dashu-cmplx` follow-ups (complex hyperbolics, `fma`, `rootofunity`, `agm`, Ziv correct
  rounding, `CBig` serde/rkyv/zeroize, `num_complex` interop, `CachedCBig`, ball arithmetic,
  `CRound` independent re/im rounding, vector ops) — see §3.4 for the full consolidated list.
- The full **C `<tgmath.h>` type-generic math surface** — the complete C standard math library for
  *both* real and complex (trig & inverse; hyperbolic & inverse; exp/log family including
  `exp2`/`exp10`/`expm1`/`log2`/`log10`/`log1p`; power/root `cbrt`/`hypot`/`pow`/`sqrt`; error & gamma
  `erf`/`erfc`/`tgamma`/`lgamma`; `fma`; rounding/remainder; fp-classification), unified by a
  type-generic `ComplexFloat`-style trait dispatching over `FBig`/`CBig`
  ([ref](https://en.cppreference.com/c/header/tgmath)). Desirable as a long-term goal, but explicitly
  out of scope for **0.5 and 0.5.x**; the individual pieces already deferred to 0.5.x (complex
  hyperbolics, `fma`, `exp2`/`log2`, …, see §3.4) are the first incremental steps toward it.
- Any MSRV bump — deferred unless forced.
- SIMD optimized FFT multiplications - it seems that we can leverage the `wide` crate for this, but this won't be considered until v1.0