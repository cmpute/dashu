# Changelog

## Unreleased

### Add
- Initial `dashu-lints` crate (not a workspace member; builds against a pinned nightly, see
  `dashu-lints/README.md`) with the `if_sign_mul` lint: flags `if`/`else` expressions that select
  between a value and its negation based on a `dashu_base::Sign` (e.g.
  `if sign == Sign::Negative { -v } else { v }`, including the `Sign::Positive` and `!=` mirrors)
  and suggests `sign * v`.
