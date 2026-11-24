# float-trig

Notes for the `float-trig` group (15 reports): dashu-float trigonometric, hyperbolic and
inverse functions, and the `Ball` helper.

## Clusters

### atan2_unit diagonals swapped for negative y
Reports: E99
Verdict: real, fixed
Root cause: the diagonal table in `Context::atan2_unit` (`float/src/math/trig.rs`) was keyed
by `(y > 0, sign(y) == sign(x))` but its rows were written as if the second key were `x > 0`,
so `(−y, +x)` and `(−y, −x)` were swapped.
Test: `float/tests/trig.rs::test_atan2_unit_diagonals`
Fix: key the table by `(y > 0, x > 0)`.

### asin_unit / acos_unit mistake other arguments for 1/2 in odd bases
Reports: E155
Verdict: real, fixed
Root cause: the `±1/2` exact row compared `x` against `FBig::ONE / 2`, a precision-0 division
that is inexact in an odd base and comes back as a one-digit neighbour (2/3 or 1/3 in base 3,
2/5 in base 5, depending on the rounding mode) — `asin_unit`/`acos_unit` in
`float/src/math/trig.rs`.
Test: `float/tests/trig.rs::test_asin_acos_unit_half_row_is_exact`
Fix: new `is_abs_half` compares `2|x| = 1` exactly on the significand; used by both functions.

## Per-report table

| Report | Cluster | Verdict |
|---|---|---|
| E99 | atan2_unit diagonals swapped for negative y | real, fixed |
| E155 | asin_unit / acos_unit mistake other arguments for 1/2 in odd bases | real, fixed |

## Changelog entries

## Cross-group notes
