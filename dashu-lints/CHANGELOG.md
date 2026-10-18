# Changelog

## Unreleased

### Add
- Add a separate house-lint CI workflow that rejects warnings across all workspace features
  and targets (excluding dashu-python). Trigger it on PR opening, reopening, or readiness,
  plus manual dispatch, rather than on each commit.
- Add `manual_sign_from_bool`, `manual_sign_arithmetic`, `manual_is_multiple_of`,
  `manual_bit_test`, `manual_div_rem`, `redundant_abs`, and `prefer_sqr_cubic`.
- Add opt-in `unfused_mul_add` advice, explicitly documenting its changed rounding semantics.
- Add UI snapshots and negative cases for type matching, const contexts, side effects,
  shadowing, and independent arithmetic tests; run UI tests in CI.
- Initial `dashu-lints` crate (not a workspace member; builds against a pinned nightly, see
  `dashu-lints/README.md`) with the `if_sign_mul` lint: flags `if`/`else` expressions that select
  between a value and its negation based on a `dashu_base::Sign` (e.g.
  `if sign == Sign::Negative { -v } else { v }`, including the `Sign::Positive` and `!=` mirrors)
  and suggests `v * sign`.

### Fix
- Limit lint CI to the example UI tests and lint implementation checks; defer full-workspace
  linting while verifying that the library builds, loads, and emits the expected diagnostics.
- Fix CI toolchain installation by passing a separate `--component` for each component.
- Install Dylint 6.1.0 with the pinned nightly and its lockfile to avoid tool/dependency drift.
- Preserve clones in `if_sign_mul` suggestions, use right-hand sign multiplication for
  rational numbers, and skip types without a known multiplication implementation.
