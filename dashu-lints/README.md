# dashu-lints

Development-only [Dylint](https://github.com/trailofbits/dylint) lints for Dashu.
This standalone crate has its own workspace and pinned nightly; the published core crates
keep their existing MSRV. Bump `rust-toolchain` and the `clippy_utils` git revision together.
CI pins `cargo-dylint` and `dylint-link` to the version in this crate's lockfile.
The `Lints` workflow builds and loads the lint library against `examples/` through the UI
tests, checks diagnostic snapshots, and checks the implementation's formatting and Clippy
warnings. The separate `House lints` workflow scans all workspace features and targets
(excluding dashu-python), treating warnings as errors. It runs when a non-draft PR is
opened, reopened, or marked ready for review, and supports manual dispatch. Additional
commits do not trigger it; use manual dispatch on the PR's head branch to check a later revision.

## Rules

| Lint | Default | Pattern / advice |
|---|---|---|
| `if_sign_mul` | warn | `if sign == Negative { -v } else { v }` → `v * sign`; preserve clones |
| `manual_sign_from_bool` | warn | `if condition { Negative } else { Positive }` → `Sign::from(condition)`; invert the condition for the mirror |
| `manual_sign_arithmetic` | warn | Branches implementing sign multiplication, negation, or identity |
| `manual_is_multiple_of` | warn | `(a % b).is_zero()` or a remainder compared with zero → `a.is_multiple_of(&b)` |
| `manual_bit_test` | warn | `(a & ONE).is_zero()` / `.is_one()` → `!a.bit(0)` / `a.bit(0)` |
| `manual_div_rem` | warn | Adjacent quotient/remainder calculations, including a nonzero-remainder early-return guard → `DivRem` or checked `DivExact` |
| `redundant_abs` | warn | Absolute-value temporaries before `abs_cmp`, integer `log2_bounds`, or comparisons of two absolute values |
| `prefer_sqr_cubic` | warn | Repeated products or integer `pow(2)` / `pow(3)` → dedicated `sqr` / `cubic` methods |
| `unfused_mul_add` | allow | Optional advice for floating `(a * b) + c` when a single rounding via `fma` is intended |

The new rules emit help rather than automatic edits. `if_sign_mul` suggestions are marked
`MaybeIncorrect` and retain the positive branch's clone. It only diagnoses known types with
right-hand `Mul<Sign>`, avoiding unsupported internal interval types.

## Semantic boundaries

- Type identity is checked through rustc, including types defined in the crate being linted;
  similarly named user types and primitive arithmetic are excluded from the numerical rules.
- Sign construction and arithmetic rewrites are skipped in const contexts: the suggested
  trait calls are not const. Numeric performance rules also skip const contexts.
- Quotient/remainder and repeated-product matching uses the same local bindings or fields.
  Calls and indexing are not treated as equivalent expressions. Quotient/remainder matching
  does not cross intervening statements or shadowed operand bindings.
- Integer performance rules, powers, and FMA advice skip `#[test]` functions and `#[cfg(test)]`
  modules so independent arithmetic references and owned/ref operator coverage are preserved.
  Use a local `#[allow(lint_name)]` for reference helpers outside those scopes.
- `IBig::sqr()` returns `UBig`; its diagnostic explicitly asks the caller to preserve the
  required signed result type. Complex multiplication/squaring is not diagnosed.
- Floating `x * x * x` rounds twice; `cubic()` rounds the exact cube once. Similarly, `fma`
  differs from a separately rounded multiplication followed by addition, even with the same
  rounding-mode type. These diagnostics explain the changed rounding semantics and never
  supply an automatic replacement.
- `abs(x) < y` is not a magnitude comparison when `y` might be negative. Only comparisons
  with absolute values on both sides, or an existing magnitude-only operation, are diagnosed.

The library does not enable broad Clippy groups or alter the arithmetic implementations.
Existing Clippy rules such as `redundant_clone` remain separate from the house lints.

## Run

```sh
cargo +nightly-2026-04-16 install cargo-dylint dylint-link --version 6.1.0 --locked
DYLINT_RUSTFLAGS="-D warnings" cargo +nightly-2026-04-16 dylint --all -- --workspace --exclude dashu-python --all-features --all-targets
```

To enable optional FMA advice:

```sh
DYLINT_RUSTFLAGS="-W unfused_mul_add" cargo +nightly-2026-04-16 dylint --all -- -p dashu-float --lib
```

## Test

Run from this directory so Cargo selects the pinned toolchain and linker configuration:

```sh
cargo fmt --all -- --check
cargo test --lib --locked
cargo clippy --lib --locked -- -D warnings
```

Each example has a normalized `.stderr` snapshot when warnings are expected. Negative cases
cover const contexts, unrelated types, effects, shadowing, test exemptions, and rounding-aware
help. To update a snapshot after an intentional diagnostic change, run the relevant UI test,
review the output, and copy the file reported as `Actual stderr saved to ...` to its matching
`examples/<name>.stderr`.
