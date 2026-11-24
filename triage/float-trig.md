# float-trig

Notes for the `float-trig` group (15 reports): dashu-float trigonometric, hyperbolic and
inverse functions, and the `Ball` helper. All 15 reports reproduced and are real; 11 distinct
root causes, all fixed.

How each was reproduced: the regression test was run against the unfixed `float/src`
(the commit before the fix) and seen to fail, except where noted — the two out-of-memory
clusters and the directed-mode slowness would hang or exhaust memory in the test suite, so
they were reproduced in a scratch program with a capped allocator / a timeout instead.

Oracles: MPFR is not installed system-wide, but the workspace-excluded `fuzz/` crate builds
(its `rug` dependency bundles MPFR/MPC). `fuzz/tests/float_transcendental.rs`,
`float_trig_random.rs` and `cmplx_transcendental.rs` (`-- --ignored`, `FUZZ_CASES=1024`)
pass on the final commit. The new tests themselves use exact or certified oracles (exact
rationals, Taylor brackets in integer arithmetic) or a higher-precision re-rounding.

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

### atan / atan2 special values rounded twice
Reports: E27
Verdict: real, fixed
Root cause: `Context::atan` at `±inf` and `Context::atan2` at infinite arguments or `x = 0`
(`float/src/math/trig.rs`) took π already rounded to the target precision and then scaled it
(`/2`, `/4`, `*3/4`), a second rounding — wrong last digit at 10 digits, wrong direction under
`Up`, and `4` instead of `2` for 3π/4 at 1 bit.
Test: `float/tests/trig.rs::test_atan_atan2_pi_multiples_correctly_rounded` (every special row,
6 modes, 1/2/4/20/50/100/500 bits, against `k·π/2^s` formed exactly from a 64-bit-wider π).
Fix: new `Context::pi_fraction` — a small Ziv loop on a wider π ball. The results now carry
`Inexact` (they were tagged by the second rounding only).

### Trig/hyperbolic functions panic on a precision-0 zero
Reports: E223
Verdict: real, fixed
Root cause: `assert_limited_precision` ran before the exact rows in every function of
`float/src/math/trig.rs` and `float/src/math/hyper.rs`, so `FBig::ZERO.sin()` panicked
although `exp_m1`/`ln_1p`/`sqrt` (and the crate's own `test_exact_results_on_unlimited_precision`)
treat such exact results as valid.
Test: `float/tests/trig.rs::test_exact_results_at_unlimited_precision_zero`,
`float/tests/hyper.rs::test_exact_results_at_unlimited_precision_zero`
Fix: the exact rows at zero (and `acos(1) = 0`, `acosh(1) = 0`) come before the assertion in
sin, cos, sin_cos, tan, asin, atan, the `_pi`/`_unit` forward variants, asin_unit, atan_unit,
sinh, cosh, sinh_cosh, their `_pi` variants, tanh, asinh, atanh. Not changed: `atanh(±1) = ±inf`
and the rows that are a rounded constant (`acos(0)`, `acos_unit(0)`, …) still need a precision.

### sin returns 0 next to a multiple of π for an over-precise input
Reports: E108
Verdict: real, fixed
Root cause: `sin_compute`/`cos_compute`/`sin_cos_compute` (`float/src/math/trig.rs`) treated a
ball whose *midpoint* is zero as the exact argument 0 and returned an exact result, dropping
the ball's radius. For an input wider than the context the reduction `x − k·π/2` cancels onto a
zero midpoint at the first working precision, so the Ziv loop was handed an exact 0.
Test: `float/tests/trig.rs::test_sin_next_to_multiple_of_pi_is_not_zero` (pins the reported
value, and checks 4 modes × 5 precisions against `π − x` from a 400-bit π).
Fix: `sin_zero_mid`/`cos_zero_mid` keep the radius (`|sin t| ≤ |t|`, `1 − cos t ≤ t²`), which
makes the loop retry at a higher guard.

### Ball::sqrt does not cover its input ball, and asserts rad <= |mid|
Reports: E181, E159, E234
Verdict: real, fixed
Root cause: `Ball::sqrt` (`float/src/ball.rs`) used the radius `rad/(2·√mid)` — the slope at
the midpoint. The root is concave, so the lower end `√(mid − rad)` is up to `rad/√mid` away
(E181: the result misses it whenever the midpoint's root is exact or `rad` is a sizeable
fraction of `mid`). The same function `debug_assert!`ed `rad ≤ |mid|`, which its callers do
not guarantee: `asin`/`acos` next to `±1` and `acosh` next to `1` hand it a `1 − x²` / `x − 1`
that is all rounding error when the argument is wider than the first working precision
(E159, E234 — debug-only panics; release builds returned correct values, as the reports say).
Test: `float/src/ball.rs::tests::sqrt_covers_both_ends` (private helper, fixed inputs incl. the
reported ones); `float/tests/trig.rs::test_acos_asin_next_to_one_wide_argument`;
`float/tests/hyper.rs::test_acosh_just_above_one_wide_argument`.
Fix: radius `rad/LB(|mid_r|) + ε` for `2·rad ≤ |mid|` (derivation in the doc comment), and the
whole-line radius — a Ziv retry — for a wider ball, replacing the assertion.
Notable: E181 is about the `#[doc(hidden)]` `Ball`; the report shows no wrong public result and
I found none either. The radius is now twice as wide as before for small `rad`, which costs
one bit of the ≥ 50 guard digits.

### error_bounds is symmetric at a power of the base
Reports: E239 (the wrong value at 1 bit)
Verdict: real, fixed
Root cause: `ErrorBounds::error_bounds` (`float/src/round.rs`) returns one ulp (or half) on
both sides of every value. Just inside `±B^k` the representable values are `B` times denser,
so the set of reals that round to `±B^k` reaches only `ulp/B` on the side facing zero. The Ziv
containment test (`float/src/ziv.rs::contained`) therefore certified `0.75 − tiny` as `1` at
1 bit (`acos_unit(2^-52, 3)`), where 0.75 is exactly the midpoint of 0.5 and 1.
Test: `float/tests/round.rs::test_error_bounds_at_power_of_base` (points just inside/outside
each bound must/must not round to the value; bases 2 and 10, 6 modes);
`float/tests/trig.rs::test_inverse_unit_next_to_quarter_turn`.
Fix: `inner_ulp`/`half_ulps` helpers in `float/src/round.rs`; `Away`, `Up`, `Down`, `HalfAway`
and `HalfEven` use the narrower bound on the zero-facing side.
Notable: this is shared machinery (see cross-group notes). Not addressed here: the half-ulp of
an odd base is still rounded up (`(B+1)/2`, report E59 in float-convert), and the tie
inclusivity of `HalfEven` looks at the parity of the stored (normalized) significand.

### Results that hug an exact value: wrong Exact flag, slow directed modes, underflow to 0
Reports: E251, E86, E165, E239 (the `Exact` tag at 2 bits)
Verdict: real, fixed
Root cause: for a small argument `sin/tan/asin/atan/sinh/tanh/asinh/atanh(x) = x ± δ` with
`δ < |x|³`, and `cos/cosh(x) = 1 ∓ δ` with `δ < x²`. Three consequences in the Ziv path:
- E251: the approximation is exactly `x` (or `1`) until the working precision reaches
  `2·|log x|` digits, and `Context::ziv` (`float/src/ziv.rs`) passed on the `Exact` flag of
  rounding that approximation — `cos(2^-40)` was `Exact(1)`. More generally the flag of the
  candidate describes the approximation, not the result: under a nearest mode an approximation
  just below a representable `y` with the true result just above it returned `AddOne` for a
  result whose own flag is `NoOp` (found by the new property test in `atanh`).
- E86: under a directed mode that approximation sits on the boundary of the rounding preimage,
  so the loop only certified at that precision — seconds for `2^-1000000`.
- E165: for an exponent below `isize::MIN/2`, `x²` is not representable; the series kernels
  returned `Err(Underflow)`, which `unwrap_fp` turns into 0 — for `cos` (true value 1) too.
Test: `float/tests/small_arg_prop.rs` (new; 10 functions × 6 modes × 20/50/100/500 bits, value
and flag against a certified Taylor bracket in integer arithmetic; fixed cases + proptest);
`float/tests/trig.rs::{test_sin_cos_small_argument_is_inexact, test_sin_cos_tiny_argument_directed,
test_trig_argument_with_exponent_near_half_isize_min, test_trig_argument_at_bottom_of_exponent_range,
test_inverse_unit_next_to_quarter_turn}`; `float/tests/hyper.rs::{test_tiny_argument,
test_argument_at_bottom_of_exponent_range}`; `float/src/ziv.rs::tests::{round_near_offsets,
ziv_does_not_certify_exact_with_radius, ziv_resolves_the_side_of_the_result,
ziv_gives_up_on_the_side_of_an_exact_result}`.
Fix:
- `Context::round_near` (`float/src/ziv.rs`): rounds `v + dir·δ` for an exactly known `v` and an
  offset proven too small to reach a rounding boundary — value and flag in one step.
- `tiny_odd` / `tiny_even` (`float/src/math/trig.rs`) apply it in sin, cos, sin_cos, tan, asin,
  atan, sinh, cosh, sinh_cosh, tanh, asinh, atanh; `cos_unit`/`sin_cos_unit` (next to a multiple
  of `u`), `cosh_pi`/`sinh_cosh_pi`; and `round_near_quarter` in `acos_unit` (tiny argument),
  `atan_unit` (huge argument) and `atan2_unit` (next to an axis), which hug `k·u/4`.
  `atan2(y, B^k)` reuses `atan`.
- The series kernels (`sin_compute`, `cos_compute`, `sin_cos_compute`, `asin_ball`, `atan_ball`,
  `sinh_series`, `cosh_series`) return the argument (or 1) widened by an ulp when its square is
  below the working precision, instead of forming the square.
- `Context::certified` (`float/src/ziv.rs`): an attempt is only accepted once its error interval
  no longer reaches the returned value (so the flag is the result's), pursued up to a working
  precision of about `2p`; then the attempt is accepted as before.
Notable:
- The `2p` backstop exists because some operations outside this group still reach the loop
  with an exactly representable result (`powf(4, 0.5)` → 2). Under the nearest modes those
  keep returning the right value (with an `Inexact` flag, as before); under the directed modes
  they do not terminate — before and after this change (see cross-group notes).
- An argument whose exponent is within the precision of `isize::MIN` returns `Err(Underflow)`
  from the eight odd functions (the neighbours' last place is not representable).
- E86 and the far-exponent cases of E165 were not run against the unfixed code inside the test
  suite (seconds to unbounded). E86 was accepted from the report's timings plus the mechanism;
  the fixed tests run in milliseconds in a debug build.

### tanh flag at the saturation shortcut
Reports: E224
Verdict: real, fixed
Root cause: `Context::near_one_endpoint` (`float/src/ziv.rs`), which `tanh` uses once
`1 − |tanh x|` is below half an ulp, tagged the all-nines result `SubOne`/`AddOne` and a result
of `±1` `NoOp` — the adjustment relative to `±1` rather than to the truncated significand,
which is how `Rounding` is documented and what the Ziv path reports one step below the
threshold.
Test: `float/tests/hyper.rs::test_tanh_saturation_flag`; the pinned flags in
`float/src/math/hyper.rs::tests::test_tanh_large_argument_saturation` and
`complex/src/math/trig.rs::tests::tan_tanh_large_imaginary_saturation` are updated.
Fix: the endpoint is an ordinary rounding of `±0.99…9…` through the `round_near` machinery.
Notable: `exp_m1` (large negative argument) and dashu-cmplx's `tan`/`tanh` share the endpoint
and get the same flags. In dashu-cmplx, `combine_parts` reports a pair of `NoOp` flags as
`Exact`, so its saturated `tan` under a toward-zero mode now reads `Exact` (it read `Inexact`
before, and the nearest modes — now `Inexact` — read `Exact` before); that collapse is the
complex groups' issue (E83/E94/E118/E172/E228).

### sinh_pi / cosh_pi out of memory at a large argument and low precision
Reports: E163
Verdict: real, fixed
Root cause: `sinh_pi`/`cosh_pi`/`sinh_cosh_pi` (`float/src/math/hyper.rs`) form `t = π·x` at
the working precision, whose guard did not grow with `|x|`; at 1 bit and `x = 2^45` the ball of
`t` had radius `2^39`, `exp_ball` turned that into a radius of `e^(2^39)` times the value, and
`Context::contained` (`float/src/ziv.rs`) tried to add the two exactly (an 8·10^11-bit
alignment). Reproduced with a capped allocator (backtrace ends in `contained` → `shl_digits`);
on macOS the uncapped program does not panic, it just grinds.
Test: `float/tests/hyper.rs::test_sinh_cosh_pi_large_argument`;
`float/src/ziv.rs::tests::contained_rejects_astronomical_radius`.
Fix: `pi_argument_guard` adds the argument's integer digits to the initial guard;
`contained` rejects a radius wider than the whole preimage from `log2` bounds before the exact
sums; `exp_certainly_overflows` makes sinh, cosh, sinh_cosh and the `_pi` variants return
`Err(Overflow)` at once for `|x| ≥ 2^(usize::BITS + 7)` (they previously asked for `ln(B)` to as
many digits as the argument has — 0.2 s at `2^100000`, no return at `2^(2^40)`).

### asinh / acosh out of memory at a huge argument
Reports: E259
Verdict: real, fixed
Root cause: `asinh`/`acosh` (`float/src/math/hyper.rs`) always formed `x² + 1`,
`(x − 1)(x + 1)` and `ln_1p`'s `1 + arg`; only an *overflow* of `x²` switched to `ln(2|x|)`.
For `x = 2^(2^40)` nothing overflows and `ln_1p_ball` (`float/src/log.rs`) adds 1 to a
`2^(2^40)`-sized value exactly. Reproduced with a capped allocator.
Test: `float/tests/hyper.rs::test_asinh_acosh_huge_argument` (equality with `ln(2x)` up to
`2^(isize::MAX − 100)`, and agreement across the switch-over magnitude with a wider precision
that stays on the general composition, 4 modes × 20/50/100/500 bits).
Fix: `square_dominates` / `ln_2x_ball` — once `|x| ≥ B^(wp/2 + 3)` the functions use
`ln(2|x|)` widened by one ulp (the difference is below `1/(2x²)`).

## Per-report table

| Report | Cluster | Verdict |
|---|---|---|
| E27 | atan / atan2 special values rounded twice | real, fixed |
| E86 | Results that hug an exact value | real, fixed |
| E99 | atan2_unit diagonals swapped for negative y | real, fixed |
| E108 | sin returns 0 next to a multiple of π for an over-precise input | real, fixed |
| E155 | asin_unit / acos_unit mistake other arguments for 1/2 in odd bases | real, fixed |
| E159 | Ball::sqrt does not cover its input ball, and asserts rad <= \|mid\| | real, fixed (debug-only panic) |
| E163 | sinh_pi / cosh_pi out of memory at a large argument and low precision | real, fixed |
| E165 | Results that hug an exact value | real, fixed |
| E181 | Ball::sqrt does not cover its input ball, and asserts rad <= \|mid\| | real, fixed |
| E223 | Trig/hyperbolic functions panic on a precision-0 zero | real, fixed |
| E224 | tanh flag at the saturation shortcut | real, fixed |
| E234 | Ball::sqrt does not cover its input ball, and asserts rad <= \|mid\| | real, fixed (debug-only panic) |
| E239 | error_bounds is symmetric at a power of the base (value); Results that hug an exact value (flag) | real, fixed |
| E251 | Results that hug an exact value | real, fixed |
| E259 | asinh / acosh out of memory at a huge argument | real, fixed |

## Changelog entries

`float/CHANGELOG.md`, under `## Unreleased`:

```markdown
### Fix
- `atan2_unit` returned the two negative-`y` diagonals swapped (`atan2_unit(-1, 1, 360)` was
  `-135`).
- `asin_unit` / `acos_unit` treated 2/3 and 1/3 (base 3), 2/5 (base 5), … as the exact `±1/2`
  row in odd bases.
- `atan(±inf)` and `atan2` at infinite arguments or `x = 0` are correctly rounded (they scaled
  an already rounded π) and report `Inexact`.
- The trigonometric and hyperbolic functions no longer panic on an exact zero carrying
  unlimited precision (`FBig::ZERO.sin()`); `acos(1)` and `acosh(1)` likewise.
- `sin` / `cos` / `tan` no longer return an exact `0` for an argument next to a multiple of
  π/2 that has more digits than the context.
- `acos` / `asin` next to `±1` and `acosh` next to `1` no longer trip a debug assertion when
  the argument is wider than the working precision; the internal `Ball::sqrt` now covers the
  lower end of its input ball.
- `ErrorBounds::error_bounds` is narrower on the zero-facing side of a power of the base, where
  the representable values are denser; the Ziv loop no longer certifies a value from the binade
  below as the power of the base (`acos_unit(2^-52, 3)` at 1 bit).
- `sin`, `cos`, `tan`, `asin`, `atan`, `sinh`, `cosh`, `tanh`, `asinh`, `atanh` (and `cos_unit`,
  `cosh_pi`, `acos_unit`, `atan_unit`, `atan2_unit` next to their exact values) of a tiny
  argument are rounded directly: they report `Inexact` with the correct `Rounding` (not
  `Exact`), take microseconds under the directed modes at any exponent, and no longer underflow
  to `0` for exponents below `isize::MIN / 2`.
- The transcendental functions report the `Rounding` flag of the true result, not of the
  working approximation, when the two fall on opposite sides of the returned value.
- `tanh` at saturation (and `exp_m1` at its `-1` saturation) report the `Rounding` adjustment
  relative to the truncated significand, consistently with the unsaturated path.
- `sinh_pi` / `cosh_pi` / `sinh_cosh_pi` no longer run out of memory for a large argument at a
  low precision, and `sinh` / `cosh` and their ×π variants report an astronomically large
  argument as an overflow immediately.
- `asinh` / `acosh` no longer run out of memory for huge arguments (from about `2^(2^40)`).
```

`complex/CHANGELOG.md`, under `## Unreleased`:

```markdown
### Change
- The saturated `tan` / `tanh` endpoint reports the `Rounding` adjustment relative to the
  truncated significand (inherited from dashu-float's `near_one_endpoint`).
```

## Cross-group notes

Files touched outside the trig/hyperbolic/`Ball` sources (`float/src/math/{trig,hyper}.rs`,
`float/src/ball.rs`):

- `float/src/ziv.rs` (shared Ziv machinery; float-exp may also touch it):
  - `ziv` / `ziv_pair` call a new `Context::certified(a, e, candidate, guard)` instead of
    `contained`. It adds one condition to containment: when the radius is nonzero and
    `|a − y| ≤ e` (the interval still reaches the candidate `y`), the attempt is retried while
    `guard < precision + SIDE_GUARD` (16), and accepted as before once the guard is past that.
    Effect on other groups' functions: correct flags when the result is within reach of a
    representable value; at most a few extra retries (to a working precision below ~`3p`) when
    the result is exactly representable but not special-cased (`powf(4, 0.5)`), and for tiny
    arguments of `exp`/`exp_m1`/`ln_1p`, which still come back `Exact` after those retries —
    unchanged behaviour, slightly slower. Those functions can adopt `round_near` the same way.
  - `contained` gains an early `return false` when `log2(e)` exceeds the preimage width by a
    margin (before the exact sums).
  - New `near_exact_exp`, `round_offset`, `round_near` (pub(crate)).
  - `near_one_endpoint` reimplemented on `round_offset`; its *flags* changed (values did not):
    all-nines result → `NoOp`; `+1` → `AddOne`; `−1` → `SubOne`. This changes the flags of
    `exp_m1` at its `−1` saturation (float-exp) and of dashu-cmplx `tan`/`tanh` saturation.
  - Two existing unit tests (`ziv_retries_until_contained`, `ziv_pair_retries_until_both_contained`)
    modelled a value of exactly 1 with a shrinking radius; they now use `1 + 2^-10`.
- `float/src/round.rs` (`error_bounds`; float-convert has E59/E166 there): `Away`, `Up`, `Down`,
  `HalfAway`, `HalfEven` take the zero-facing bound from new `inner_ulp` / `half_ulps` helpers.
  The `(B+1)/2` half-ulp and the `precision == 0` handling of `Up`/`Down` are untouched, so a
  fix for E59/E166 should merge by keeping both changes (apply their half-ulp fix inside
  `half_ulps`).
- `complex/src/math/trig.rs`: only the expected flags in the test
  `tan_tanh_large_imaginary_saturation` (see the tanh cluster).
- `float/tests/round.rs`: new `test_error_bounds_at_power_of_base`.

Observed but not this group's (not fixed here):

- Directed-mode `powf` with an exactly representable result does not terminate
  (`Context::<Down>::new(20).powf(4, 0.5)`); the nearest modes return `Inexact(2, NoOp)`.
- `exp(2^(2^40))` does not return (its reduction asks for `ln(B)` to 2^40 digits); the
  hyperbolic functions no longer reach that path.
- `ln_1p_ball` (`float/src/log.rs`) adds `1 + arg` exactly, which is what ran out of memory in
  E259; `ln_1p` of a huge argument probably does the same.
- `Context::sub`/`add` are fine with astronomically distant operands (used by `acos` of a tiny
  argument), nothing to report there.
