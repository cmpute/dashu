## Overview

dashu is a library set of arbitrary precision numbers implemented in pure Rust, aiming to be a Rust-native alternative to GNU GMP + MPFR.

**MSRV is a hard constraint for core crates only.** Core crates are the `dashu` meta-crate and its direct dependencies: `dashu-base`, `dashu-int`, `dashu-float`, `dashu-ratio`, `dashu-macros`, `dashu-cmplx`. The current MSRV is maintained in each crate's `Cargo.toml` and the top-level `README.md`. When modifying code in core crates, ensure it remains MSRV-compatible.

Secondary crates (`dashu-python`, `benchmark/`, fuzz tests) are **not** bounded by the workspace MSRV policy. They may use newer Rust versions and dependency versions as needed.

## Workspace structure

| Crate | Directory | Description |
|---|---|---|
| `dashu-base` | `base/` | Common trait definitions and utilities |
| `dashu-int` | `integer/` | Arbitrary precision integers (`UBig`, `IBig`) |
| `dashu-float` | `float/` | Arbitrary precision floats (`FBig`, `DBig`, `CachedFBig`) |
| `dashu-ratio` | `rational/` | Arbitrary precision rationals (`RBig`, `Relaxed`) |
| `dashu-macros` | `macros/` | Procedural macros for literal big numbers |
| `dashu-cmplx` | `complex/` | Arbitrary precision complex numbers (`CBig`) |
| `dashu-python` | `python/` | PyO3 Python bindings (not in default members) |
| *(benchmark)* | `benchmark/` | Profiling scratchpad, not a comprehensive benchmark suite |

The `dashu` crate at the root is a meta-crate that re-exports all types from the sub-crates as nested modules (`dashu::base`, `dashu::integer`, etc.).

## Build & test

```sh
# Check (matches CI for stable)
cargo check --all-features --tests

# Test (for local testing, differ from CI)
cargo test --workspace --exclude dashu-python

# Lint (warnings are errors)
cargo clippy --all-features --all-targets --workspace --exclude dashu-python -- -D warnings

# Format check
cargo fmt --all -- --check
```

Note: always `--exclude dashu-python` when running workspace-wide commands, since `dashu-python` is in early development.

## Code style

- Rust edition 2021
- `rustfmt.toml`: `fn_call_width = 80` (only config)
- Every crate supports `no_std` via `#![cfg_attr(not(feature = "std"), no_std)]` — avoid using `std` APIs in default code paths
- Doc comments use `# Examples` sections with runnable code — every public function on primitive number types must include a usage example
- Modules are organized by operation (add, div, mul, cmp, convert, etc.)
- Third-party trait implementations go in a `third_party/` module per crate, feature-gated
- When borrowing an algorithm idea from GMP (or any other library), do **not** reference its function names in our docstrings or comments. Describe the algorithm in our own terms and use our own function names (e.g. write `add_mul_dword_same_len_in_place`, never `addmul_2` / `mpn_addmul_2`). External function names must not appear anywhere in the repo.
- Tests for a specific algorithm/kernel belong in the same source file as the implementation, as a `#[cfg(test)] mod tests` block at the bottom — not in a separate integration test file under `tests/`. Reserve `tests/` for cross-cutting or public-API tests.
- When debugging or writing test assertions, use `{:?}` (or `{:#?}` for the verbose form with digit/bit counts) to inspect arbitrary precision values. The [`Debug`] format prints a compact head‥tail representation (most significant digits `..` least significant digits) instead of dumping the entire number, making it readable even for thousand-digit integers.
- In-crate tests (`#[cfg(test)] mod tests` and `tests/`) must use **fixed, deterministic inputs** — never pseudo-random generators, fuzzing harnesses, or property-test styles that draw random data. Tests must fail or pass identically on every run. The only exception is code that tests the `rand` integration itself. For randomized / exploratory input, use the fuzz targets under `fuzz/` (the proper home for that work), not unit tests.

## Feature flags

Feature flags are defined in each crate's `Cargo.toml` — read them directly for the current list. The top-level `dashu` crate forwards features to sub-crates.

When adding a new feature that integrates a third-party crate, support each major version as a separate feature with a versioned suffix (e.g. `rand_v08` for rand 0.8.x, `num-traits_v02` for num-traits 0.2.x). Add an unversioned alias feature that points to the default/latest version. Update all relevant crate Cargo.tomls and the top-level `Cargo.toml`.

## Changelog

Each sub-crate has its own `CHANGELOG.md` (e.g. `integer/CHANGELOG.md`). **Every change must be documented** in the `## Unreleased` section of the affected crate's changelog as part of the same commit.

Format:

```markdown
## Unreleased

### Add
- Description of new feature

### Fix
- Description of bug fix

## 0.4.2
- Change descriptions (older entries use flat lists)
```

Keep the `## Unreleased` section updated as you go. 

## dashu-float internals

- Estimating the number of digits can be costly — prefer using `log2_bounds` and `repr.digits_ub`/`digits_lb` instead of computing exact digit counts.
- The number of digits in an `FBig` significand is at most the context precision, with one intentional exception: the result of an inexact addition or subtraction may carry a single **guard digit** (up to `precision + 1` digits). During internal calculations the bound can be violated more freely; use the methods on `Context` instead of the public API in that case.

## Testing precision for `FBig` and `CBig`

When modifying an `FBig` or `CBig` implementation (arithmetic, conversion, transcendentals, rounding, …), test it under **at least** these base-2 precision settings — each exercises a distinct code path / significand width, and bugs often surface at only one of them:

- **20 bits** — within the `f32` significand range (exercises the narrow-precision and f32-conversion paths).
- **50 bits** — within the `f64` significand range (exercises the f64-conversion paths).
- **100 bits** — significand fits in a `u128` (exercises the double-word / multi-word-but-small fast paths).
- **500 bits** — well into the arbitrary-precision `bigint` range (exercises the general multi-word kernels and large-significand edge cases).

Prefer a high-precision oracle (e.g. the same op computed at `p + 60` under `HalfEven`, then re-rounded to `p` under the mode under test) for directed-rounding checks — remember that in-crate tests must use fixed, deterministic inputs (see [Code style](#code-style)); sweep these four precisions with a hand-chosen input set rather than a random generator.

## Cached wrappers (`CachedFBig`, `CachedCBig`)

**`CachedFBig` is a drop-in replacement for `FBig`, and `CachedCBig` for `CBig`.** Each cached wrapper must mirror the full public API and trait surface of its non-cached counterpart, delegating every impl to the inner value. **Whenever you add or change a trait impl on `FBig` or `CBig`, mirror it on `CachedFBig` / `CachedCBig` in the same change** — otherwise the `FastReal` / `FastDecimal` / `FastComplex` aliases regress: code that compiles with `FBig`/`CBig` must compile unchanged with `CachedFBig`/`CachedCBig`. The only intentional divergences are that the cached type's transcendental ops thread the shared `ConstCache`, it is `!Send + !Sync`, construction takes a cache handle, `CachedCBig::into_parts` returns `(CachedFBig, CachedFBig)` sharing the handle (not `CBig`'s `(FBig, FBig)`), and **third-party crate traits (serde, num-traits, num-order, rand, zeroize, postgres/diesel) are intentionally not mirrored** — reach them through `.as_fbig()` / `.as_cbig()`.

## dashu-int internals

When implementing algorithms that manipulate word arrays (`&[Word]`), prefer the existing `Buffer` type over `Vec<Word>`. `Buffer` provides in-place operations like `erase_front`, `push_zeros_front`, `truncate`, and works with `MemoryAllocation` for scratch space — all without `std` or extra allocations. If you find yourself reaching for `Vec<Word>`, consider whether `Buffer` or `MemoryAllocation` would be a better fit.

**Double-word is a first-class citizen** in this crate. The `DoubleWord` type (from `dashu-base`) and `_dword` operation suffix (e.g. `add_dword_in_place`, `split_dword`, `div_rem_dword`) are treated as peer primitives to single-word ones, not special cases. Whenever planning a new feature or algorithm, actively consider a double-word variant from the start — many operations have a meaningfully faster path when the operand fits in two words, and the crate is structured to expose those paths as first-class APIs.

**Algorithm thresholds** are named `THRESHOLD_<NAME>_DEFAULT` (e.g. `mul::THRESHOLD_SIMPLE_DEFAULT`, `div::THRESHOLD_SIMPLE_DEFAULT`, `THRESHOLD_DIV_EXACT_DEFAULT`) and are tunable at runtime via a matching `DASHU_THRESHOLD_<NAME>` environment variable, behind the `tuning` feature. Follow the pattern in `mul::threshold` / `div::threshold`: a `mod threshold` with an accessor that checks the env var (only under `#[cfg(feature = "tuning")]`) and falls back to the `THRESHOLD_<NAME>_DEFAULT` constant.

## Bilingual documentation

The README and the user guide each have a Simplified-Chinese mirror — **update both languages in the same change** so they never drift:

| English | Chinese mirror |
|---|---|
| `README.md` | `README-zh.md` |
| `guide/` (mdBook) | `guide-zh/` (mdBook, `language = "zh"`) |
| `python/USAGE.md` | `python/USAGE-zh.md` (included by both guides' `python.md`) |

- `guide-zh/src/assets` is a **symlink** to `guide/src/assets` — images are shared, don't duplicate them.
- The two READMEs cross-link each other; the two guides cross-link `https://zyxin.xyz/dashu/` ↔ `https://zyxin.xyz/dashu-zh/`.
- **mdBook anchor gotcha:** mdBook 0.5 (comrak) keeps CJK characters in auto-generated heading IDs, so translating or renaming a heading in `guide-zh/` changes its `#fragment`. Update every `[text](page.md#anchor)` cross-reference to the new slug — mdBook does **not** error on a dangling fragment, so a stale anchor breaks silently. Verify with `mdbook build guide` + `mdbook build guide-zh` (see `.github/workflows/guide.yml` for the pinned mdBook + mdbook-katex toolchain).

## Common pitfalls

- **dashu-python is excluded** from workspace tests and clippy — always add `--exclude dashu-python`
- **diesel has two major versions** in the dependency tree — use `diesel@2` (not `diesel` or `diesel@2.x.y`) when pinning in CI
- **MSRV compatibility** — for core crates only: if you add a new dependency to a core crate, check whether it supports the current MSRV; if not, it may need to be stripped for MSRV builds. Secondary crates (dashu-python, benchmarks, fuzz tests) are exempt from this check.
- **Sub-crate versions can differ** in minor/patch (e.g. `dashu-int` 0.4.2, `dashu-float` 0.4.4) — keep them in sync when making cross-crate changes
