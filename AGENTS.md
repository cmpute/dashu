## Overview

dashu is a library set of arbitrary precision numbers (big integers, floats, rationals) implemented in pure Rust. It aims to be a Rust-native alternative to GNU GMP + MPFR.

**MSRV is a hard constraint** — do not bump it unless absolutely necessary. The current MSRV is maintained in `README.md`; when modifying code, ensure it remains compatible.

## Workspace structure

| Crate | Directory | Description |
|---|---|---|
| `dashu-base` | `base/` | Common trait definitions and utilities |
| `dashu-int` | `integer/` | Arbitrary precision integers (`UBig`, `IBig`) |
| `dashu-float` | `float/` | Arbitrary precision floats (`FBig`) |
| `dashu-ratio` | `rational/` | Arbitrary precision rationals (`RBig`) |
| `dashu-macros` | `macros/` | Procedural macros for literal big numbers |
| `dashu-python` | `python/` | PyO3 Python bindings (not in default members) |
| *(benchmark)* | `benchmark/` | Profiling scratchpad, not a comprehensive benchmark suite |

The `dashu` crate at the root is a meta-crate that re-exports all types from the sub-crates as nested modules (`dashu::base`, `dashu::integer`, etc.).

## Build & test

```sh
# Check (matches CI for stable)
cargo check --all-features --tests

# Test
cargo test --all-features --workspace --exclude dashu-python --no-fail-fast

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

## Commit style

- Always run `cargo fmt --all` before committing — formatting is strictly enforced in CI
- Imperative mood: "Fix overflow in division" not "Fixed overflow"
- Common prefixes: `Fix`, `Add`, `Implement`, `Bump`, `Extract`, `Replace`
- Branch naming: `fix-<topic>`, `feat/<topic>`
- Squash-merge is used on PRs

## dashu-int internals

When implementing algorithms that manipulate word arrays (`&[Word]`), prefer the existing `Buffer` type over `Vec<Word>`. `Buffer` provides in-place operations like `erase_front`, `push_zeros_front`, `truncate`, and works with `MemoryAllocation` for scratch space — all without `std` or extra allocations. If you find yourself reaching for `Vec<Word>`, consider whether `Buffer` or `MemoryAllocation` would be a better fit.

## Common pitfalls

- **dashu-python is excluded** from workspace tests and clippy — always add `--exclude dashu-python`
- **diesel has two major versions** in the dependency tree — use `diesel@2` (not `diesel` or `diesel@2.x.y`) when pinning in CI
- **MSRV compatibility** — if you add a new dependency, check whether it supports the current MSRV; if not, it may need to be stripped for MSRV builds
- **Sub-crate versions can differ** in minor/patch (e.g. `dashu-int` 0.4.2, `dashu-float` 0.4.4) — keep them in sync when making cross-crate changes
