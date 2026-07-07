# dashu v0.5 Release Plan

Last updated: 2026-07-07

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
| 0 | Test / benchmark / fuzz hardening | **GATE for all feature work** — ✅ done | — |
| 1 | Breaking changes & deprecation cleanup | must land in 0.5 — ✅ mostly done (only non-breaking internals open; `missing_docs` done) | 0 (ideally) |
| 2 | `dashu-float` shared constant cache | ✅ done (#83, as `CachedFBig`/`ConstCache`); `FastReal`/`FastDecimal` aliases added | 0, 1 |
| 3 | `dashu-cmplx` (`CBig`) — new crate | ✅ done (M1–M6) | 0, 2 |
| 4 | The mdBook guide | ✅ content done (#88); only GitHub-Pages deploy + README badge pending | 1, 2, 3 (content); infra can start now |
| 5 | Release prep & version sync | ⬜ not started — **the only remaining phase** | 1–4 |

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
- [x] **`#![deny(missing_docs)]`** across all crates — **done.** Documented the 53 previously-
      undocumented public items (42 in `dashu-base`, 8 in `dashu-int`, 2 in `dashu-ratio`, 1 in
      `dashu-float`; `dashu-cmplx`/`dashu-macros` were already clean) and enabled
      `#![deny(missing_docs)]` plus the existing `clippy::dbg_macro`/`undocumented_unsafe_blocks`/
      `let_underscore_must_use` denies on **all** crates (resolving the `v0.5` lint TODOs in
      `integer/src/lib.rs`). Two `let _ =`-on-`#[must_use]` test sites were converted to named
      bindings to satisfy `let_underscore_must_use`.
      **Caveat:** `#![deny(clippy::allow_attributes_without_reason)]` is **not** enabled — satisfying
      it needs the `reason = "..."` field on every `#[allow]` (or `#[expect]`), both stabilized in
      Rust **1.81**, which conflicts with the 1.68 MSRV. Revisit once the MSRV reaches ≥ 1.81 (see
      the note in `integer/src/lib.rs`). The remaining "move verbose prose to the guide" cleanup
      (per `integer/src/ubig.rs:10` `TODO(v0.5)`) is tracked in Phase 1.5 / 4.2 and is independent
      of the lint gate.
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
> `guide/src/compliance.md` and `float/CHANGELOG.md`.

### 1.4 `dashu-ratio`
- [x] **`From<Repr> for FBig` → `TryFrom`** (`rational/src/third_party/dashu_float.rs:12`): make the
      conversion fallible (succeed only when exact). A ready stub `fbig_try_from_rbig` exists at
      `:26`. Remove the dead-code stub TODO at `:24` and the `#[allow(dead_code)]`.
- [x] Wire `UBig::to_digits` into `fmt/expanded.rs` (integer part, direct) and batch the fractional
      long division through a precomputed `dashu_int::fast_div::ConstDivisor` (one division per word
      of digits) on the non-repetend path — done. The remaining `write_digits`→`DigitWriter` SIMD
      TODO is the only fast-fmt item left for 0.5.x. Non-breaking internal perf, gated on 1.2.

### 1.5 Doc / internal (non-breaking, fold in opportunistically)
- [x] **Move verbose type prose to the guide** — `integer/src/ubig.rs:10` `TODO(v0.5)` **done.** The
      verbose construction/parsing-printing/layout prose on `UBig`, `IBig`, and `FBig` (and a light
      trim on `RBig`/`CBig`) was condensed to a brief summary + guide link; the `TODO(v0.5)` marker is
      removed. Runnable `# Examples` kept verbatim (no doctest churn). Guide links point to the
      rendered site at `https://zyxin.xyz/dashu/` (the guide is live — see Phase 4.1). Pairs with
      Phase 4.2.
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

- [x] **Add aliases** in the meta-crate alongside `Real`/`Decimal`:
      `pub type FastReal = dashu_float::CachedFBig;` and
      `pub type FastDecimal = dashu_float::CachedFBig<dashu_float::round::mode::HalfAway, 10>;`.
      *(Done in `src/lib.rs`; both carry doc-comments noting `!Send + !Sync`.)*
- [x] **Bring `CachedFBig` to `FBig`'s trait surface** so the aliases are ergonomic (delegate to the
      inner `FBig`). **Done (always-on surface):** the trait surface now mirrors `FBig` — formatting
      (`Display`/`LowerExp`/`UpperExp` + the base-specific `Binary`/`Octal`/`LowerHex`/`UpperHex`),
      ordering (`PartialOrd`/`Ord`/`AbsOrd`/`Signed`/`EstimatedLog2`), `FromStr`, `From`/`TryFrom` for
      integers and `f32`/`f64`, the shift ops, `Mul<Sign>`, the root/euclid traits (`SquareRoot`/
      `CubicRoot`/`DivEuclid`/`RemEuclid`/`DivRemEuclid`/`Inverse`), and `Sum`/`Product` (the value
      delegates to `FBig`'s impls — so `Sum` stays correctly-rounded — and the result keeps the first
      element's cache). `float/tests/parity.rs` guards the value/format parity. **Third-party crate
      traits (serde/num-traits/num-order/rand/zeroize/postgres) are intentionally not mirrored** —
      reach them via `.as_fbig()` (recorded as a divergence in `AGENTS.md`).
- [x] **Guide:** `guide/src/cached.md` is a full page covering `CachedFBig`/`ConstCache` (creation,
      cache sharing, inspection/clearing, constants, thread-safety, worked example). **Gap:** it does
      not name the `FastReal`/`FastDecimal` aliases — add a short pointer so guide readers find them.

### Still open

- [x] **Memory growth:** *decided — not planned for v0.5.* No eviction/cap/shrink policy; a 1M-digit
      π lives in the cache until `clear_cache()`/drop. This is documented as expected behavior —
      callers own the cache lifetime explicitly via the `CachedFBig`/`ConstCache` handle. Revisit
      only if real workloads report memory pressure.

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
- **Vector ops** (`dot`/mean) — `Sum`/`Product` for `CBig` (the `iter` analog of `FBig`) now exist
  (fold-based, narrowed impls); still missing: `dot`/mean helpers and a correctly-rounded
  (exact-accumulating) `Sum` for `CBig`.
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
- **Expose ownership-aware kernel functions from `dashu-float`** — `dashu-float`'s `add.rs` already
  has `add_val_val` / `add_val_ref` / `add_ref_val` / `add_ref_ref` kernel functions that consume
  owned `FBig`/`Repr` when available (avoiding unnecessary clones at the convenience layer). These are
  currently `pub(crate)`; they should be made `pub` (or mirrored as `pub` methods on `Context<R>` like
  `add_val_val(&self, lhs: Repr<B>, rhs: Repr<B>)`) so that `dashu-cmplx` can call them directly in
  its own per-ownership kernel functions instead of immediately borrowing every `CBig` operand through
  `Context::add(&CBig, &CBig)` (which takes `&Repr` internally and clones as needed). The same applies
  to `sub`/`mul`/`div` and potentially to the transcendental ops. Without this, `CBig`'s by-value
  operator impls (e.g. `impl Add for CBig`) take ownership but cannot exploit it — they immediately
  borrow their parts through the complex `Context`, which in turn borrows the real `Context`, and the
  ownership advantage is lost.

---

## Phase 4 — The mdBook Guide

**Goal:** a complete user guide under `guide/`, built with mdBook, deployed from CI.

> **Content is done** (PR #88 — "Phase 4: complete the mdBook user guide"). All pages that were
> stubs in the previous revision are now real, and four new pages were added (`cached.md`,
> `performance.md`, `compliance.md`, `ops/trig_n_hyper.md`). `CBig` is documented *across* the
> existing pages (construct/convert/io/ops/faq) rather than in a dedicated chapter — intentional, no
> separate chapter is planned. The guide is **deployed** at `https://zyxin.xyz/dashu/`; only the
> README **Book** badge (+ optional CI auto-deploy) remain.

### 4.1 Infrastructure
- [x] Extend `guide/book.toml`: `[output.html]` + `[preprocessor.katex]` are configured. (`mdbook-toc`
      / `mdbook-admonish` were "if desired" — skipped, not needed.)
- [x] Stop committing the rendered `guide/book/` output — `.gitignore` ignores `book`, and
      `git ls-files guide/book` is now empty.
- [x] **Build-check CI** added: `.github/workflows/guide.yml` installs `mdbook` (0.5.3) + the
      mdbook-0.5 build of `mdbook-katex` (pinned by git rev) and runs `mdbook build guide`. Fails on
      errors and on any `SUMMARY.md` entry whose target file is missing (broken-link guard).
- [x] **Deploy the guide** — *live at `https://zyxin.xyz/dashu/`* (custom domain on the `gh-pages`
      orphan branch). The guide was built with mdBook 0.5.3 + mdbook-katex and pushed to an orphan
      `gh-pages` branch (no shared history with `master`/`develop`). This was a **one-shot manual**
      deploy; future guide updates require rebuilding and re-pushing `gh-pages`.
- [ ] **CI auto-deploy** (optional follow-up): `guide.yml` is still build-check only — it does *not*
      rebuild/push `gh-pages` on merge. Adding a deploy step (e.g. `peaceiris/actions-gh-pages` or
      `actions/deploy-pages`) would keep the published site in sync automatically.
- [ ] **Re-enable the Book badge** in `README.md:8` (currently commented out) — point it at
      `https://zyxin.xyz/dashu/`.

### 4.2 Content
- [x] Fill the previously-stub pages: `io/{index,parse,print,serialize,interop}.md`,
      `ops/{index,basic,cmp,bit,exp_log,num_theory,trig_n_hyper}.md`, `faq.md`, `cheatsheet.md`.
- [x] New v0.5 pages: `cached.md` (the constant cache / `CachedFBig`), `performance.md`,
      `compliance.md` (the IEEE-754 / C99 Annex-G compliance notes).
- [x] `CBig` covered across pages (construction, arithmetic, transcendentals, branch cuts, I/O).
- [x] **Migrate verbose API prose out of doc-comments** (per `integer/src/ubig.rs:10` `TODO(v0.5)`)
      — **done** (see Phase 1.5). Concise rustdoc + guide links now; the `TODO(v0.5)` marker is gone.
- [x] Cross-check MSRV statement in `guide/src/index.md` — still "1.68", matching `README.md` and
      `rust-version = "1.68"`; no 0.5 bump is expected (cache is `alloc`-only, mdBook is build-time).

---

## Phase 5 — Release Preparation

- [ ] **Version sync:** bump *all* crates to `0.5.0` and align. Current skew (as of 2026-07-07):
      base/int/rational `0.4.3`, float `0.4.5`, complex `0.4.5`, macros `0.4.2`, meta `0.4.4`.
      Refresh all internal `version = "0.4.x"` path deps to `0.5.0` (including the meta-crate's
      `dashu-cmplx = { version = "0.4.5", path = "./complex" }`).
- [x] **Workspace:** `complex` is already in `members` and `default-members`; meta-crate feature
      forwarding to `dashu-cmplx` is wired (`std`/`serde`/`num-order`/`zeroize`/`rand`/`rand_v08`/
      `rand_v09`/`rand_v010`/`num-traits`/`num-traits_v02`/`num-complex`).
- [ ] **Changelogs:** fold every crate's `## Unreleased` into a `## 0.5.0` section (breaking
      changes under `### Change`/`### Remove`, features under `### Add`). **Gap to fix first:**
      PR #89 (`fix: avoid double rounding in RBig::to_f32/to_f64`) touched `rational/src/convert.rs`
      but has **no entry** in `rational/CHANGELOG.md`'s `## Unreleased` — add it before folding.
      Same audit for any other recently-landed change missing a changelog line.
- [x] **MSRV: 0.5 targets 1.68** *(decided)* — no bump. mdBook is build-time only (no runtime MSRV
      impact), and the constant cache uses only `alloc`. Consequence: `#![deny(clippy::
      allow_attributes_without_reason)]` stays **deferred** — it needs the `reason = "…"` field on
      every `#[allow]` (or `#[expect]`), both stabilized in Rust **1.81**; revisit when MSRV ≥ 1.81.
- [ ] **MSRV bookkeeping:** update `README.md` badge + all `rust-version = "1.68"` fields once
      versions sync to `0.5.0` (no value change — just confirm they survived the bump).
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
| Cache memory unbounded growth | **Not planned for v0.5** — no cap; cache grows until `clear_cache()`/drop (documented; callers own the lifetime via the `CachedFBig`/`ConstCache` handle) |
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