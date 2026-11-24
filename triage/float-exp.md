# float-exp: exp / log / pow / root functions of dashu-float

33 reports, 14 distinct issues. All line references in "Root cause" are to the triaged commit
(`a0fc535`). Tests live in `float/tests/{exp,log,root,round}.rs`; every new test was run against
a copy of the triaged commit to confirm it fails (or does not return) there — see
"Verification" at the end.

## Clusters

### ziv-certification — wrong preimage at a power of the base, exact ties rejected, flags taken from the approximation

Reports: E45, E229, E174, E69, D29, D40, E225

Verdict: real, fixed

Root cause: three defects in the correct-rounding certification shared by every Ziv-backed
function.
1. `ErrorBounds::error_bounds` (`float/src/round.rs:199-450`) used `f.ulp()` on both sides of the
   value. For a power of the base the neighbour toward zero is in the finer binade below, so the
   claimed preimage was a factor `B` too wide on that side: at 3 decimal digits `1.00` claimed
   `[0.995, 1.005)` instead of `[0.9995, 1.005)`. An approximation that landed just above the
   power (within its honest error radius) was therefore certified as that power. The HalfEven
   inclusion flags also read the parity of the *normalized* significand
   (`round.rs:445`), which is not the parity at the precision.
2. A zero radius (the approximation *is* the true value) still went through the containment
   test (`float/src/ziv.rs:148-151`, `290-325`), which rejects a true value sitting on a
   preimage edge — an exact tie, or (directed modes) any exactly representable value.
3. An attempt was accepted while the candidate was inside the error interval, so the returned
   `Exact`/`Inexact` flag was that of rounding the *approximation*: `Exact` whenever the
   working-precision value happened to round to itself, or an `Inexact` flag pointing the wrong
   way. Under `Zero` the same hole let a result above the true value through.

Test: `float/tests/round.rs::test_error_bounds_are_the_rounding_preimage` (exhaustive preimage
check, bases 2/4/10, all modes); `float/tests/exp.rs::test_exp_powi_round_just_below_power_of_base`
(E45), `::test_powi_exact_tie` (D29), `::test_powi_negative_exponent_matches_division` (E174, all
modes, 1–500 bits, against the correctly rounded division), `::test_powf_half_matches_sqrt`
(E229); `float/tests/log.rs::test_log_rounding_flag` (E69);
`float/tests/root.rs::test_hypot_inexact_result_is_not_exact` (D40),
`::test_hypot_toward_zero_does_not_round_up` (E225), `::test_hypot_exact_oracle_sweep`.

Fix: `float/src/round.rs` — the toward-zero bound of `±B^k` is `ulp/B` (helper `inner_ulp`), the
HalfEven tie inclusion uses the significand parity at the precision (`is_even_at_precision`) and,
on the inner side of a power of the base, the parity in the finer binade. `float/src/ziv.rs`
`contained` — a zero radius is accepted outright; a positive radius must leave the candidate
strictly outside the error interval (`|a − y| > e`), which makes the flag of rounding the
approximation the flag of rounding the true value.

Notes:
- D40 has two halves: the `hypot` flags are this cluster; the `sqrt(10^20+1)` flag is the same
  flag defect reached through `sqrt`'s Ziv loop, and is covered by the `sqrt-single-step`
  cluster's rewrite. E225 is the same: `hypot`'s ball `sqrt` took the false `Exact` of the
  base-3 `Context::sqrt` as a zero radius. Both are also covered by the `hypot` rewrite.
- E174's "values not correctly rounded at low precision" (`15^-16` at 1 bit) is defect 1, its
  `Exact` flags are defect 3 — Hegel found inputs whose 65-bit working value has 12 trailing
  zero bits.
- Consequence of defect 3's fix: a function whose true result is exactly representable must now
  report a zero radius or resolve the case before the loop, in *every* mode (it already had to
  under the directed modes). That is what the `powf-exact`, `log-exact-power` and tiny-argument
  changes below do for this group's functions.

### sqrt-single-step — sqrt ties / square-factor bases / extreme exponents

Reports: E29, E75, E203

Verdict: real, fixed

Root cause: `Context::sqrt` (`float/src/root.rs:204-318`) certified non-power-of-two bases
through a Ziv loop around its one-step kernel. (a) An exact root that is a tie can never be
certified by the loop (E29: `sqrt(225)` at 1 decimal digit). (b) For an exact `p+1`-digit root
the kernel always called `round_low_part` and always reported `Inexact`
(`root.rs:267-270`, `285-289`), even when the dropped digit is zero — which happens in a base
with a square factor, where normalization leaves an odd exponent (`36 = 9·4¹`): `Up` returned 7
in base 4, and base 9 did not terminate (E75). (c) `x.exponent - shift` (`root.rs:237`) overflows
next to `isize::MIN`/`MAX` (E203: debug panic, `10^(+4.6e18)` in release).

Test: `float/tests/root.rs::test_sqrt_exact_oracle_sweep` (exact oracle, bases 2/3/4/9/10/16, all
modes), `::test_sqrt_exact_oracle_standard_precisions`, `::test_sqrt_exact_tie_decimal` (E29),
`::test_sqrt_perfect_square_in_square_factor_base` (E75),
`::test_sqrt_inexact_root_is_not_exact` (D40/E225 sqrt halves), `::test_sqrt_extreme_exponents`
(E203).

Fix: `float/src/root.rs` — the rounding decision from the integer root, round digit and
remainder is exact in every base once the odd-base half-way case (`2d + 1 = B`) consults the
remainder, so the Ziv loop is removed; an exact root with a zero round digit is `Exact`; the
exponent is computed in `i128`.

### exact-shortcut-precision — exact special cases return precision 0

Reports: D17, E61, E221 (duplicates of each other)

Verdict: real, fixed

Root cause: the exact shortcuts returned the precision-0 constants `FBig::ONE` / `FBig::ZERO`
instead of a value in the operation's context: `float/src/exp.rs:392` (`powi(x, 0)`), `:562`
(`powf(x, ±0)`), `:576` (`powf(0, y>0)`), `:581` (`powf(1, y)`), `:739` (`exp_m1(-inf)`),
`:768-773` (`exp(0)`, `exp_m1(±0)`); `float/src/log.rs:615` (`ln(1)`), `:622` (`ln_1p(0)`),
`:696` (`log2(1)`), `:789` (`log10(1)`). The next limited-precision op on the result panics.

Test: `float/tests/exp.rs::test_exact_shortcuts_keep_precision`,
`float/tests/log.rs::test_exact_shortcuts_keep_precision`.

Fix: `float/src/exp.rs`, `float/src/log.rs` — all of them return `FBig::new(repr, *self)`.

Notes: the same pattern exists in the trig/hyperbolic functions (float-trig's area; not touched
here). An unlimited-precision input still gets an unlimited-precision exact result.

### zero-root-precision — cbrt / nth_root / hypot panic on a precision-0 zero

Reports: E123, E141

Verdict: real, fixed

Root cause: `nth_root` asserted a limited precision before its zero shortcut
(`float/src/root.rs:113`), and `hypot` before its `hypot(0, 0)` shortcut (`root.rs:336`); `sqrt`
does it the other way round.

Test: `float/tests/root.rs::test_roots_of_unlimited_precision_zero` (plus
`::test_nth_root_unlimited_precision`, `::test_hypot_unlimited_precision` pinning that a
non-trivial root still panics).

Fix: `float/src/root.rs` — the exact shortcuts (zero, and `n = 1` for `nth_root`) run before the
assertion; docs of `Context::cbrt`/`nth_root` updated.

### hypot-range — hypot fails or crawls away from the middle of the exponent range

Reports: E131, E185, E195

Verdict: real, fixed

Root cause: `Context::hypot` (`float/src/root.rs:332-406`) ran a Ziv loop over a ball
computation. Its scaling `k` (`root.rs:386`) only handled the *top* of the range and used raw
exponents: operands near `B^(isize::MIN/2)` underflowed in the squares or overflowed the exponent
arithmetic (E131); rounding an operand with more digits than the working precision raised its
exponent past the scaling (E185), and `large.exponent >= isize::MAX - 1` was rejected outright
(`root.rs:367`). Under directed modes a far-apart pair needed a working precision proportional to
the exponent gap before the loop could tell the result from the larger operand (E195).

Test: `float/tests/root.rs::test_hypot_is_exponent_invariant` (E131, E185),
`::test_hypot_range_errors`, `::test_hypot_negligible_operand` (E195),
`::test_hypot_exact_oracle_sweep`, `::test_hypot_exact_oracle_standard_precisions`.

Fix: `float/src/root.rs` — both operands are scaled by an exact exponent shift that puts the
larger just below one, the sum of squares is computed *exactly* and handed to the single-step
`sqrt`, and the root is scaled back with a range check (no Ziv loop). A negligible smaller
operand is resolved first by rounding the larger one with a positive sticky part — new internal
helper `Context::round_sticky` in `float/src/repr.rs`.

Notes: E195's reported test also asserts a wall-clock bound; the regression test asserts the
values for gaps up to `isize::MAX - 2` instead (they would not return otherwise).

### log-exact-power — log2 of a power of two in a non-binary base does not return

Reports: E82

Verdict: real, fixed

Root cause: the exact shortcut of `log2` (`float/src/log.rs:708-714`) only recognized a
power-of-two *significand* with a zero exponent (or a power-of-two base), so `0.5 = 5·10⁻¹` fell
into the Ziv loop, which cannot terminate on an exactly representable result under a directed
mode. (`exact_pow10_log`, `log.rs:830-855`, had the mirror-image gap for a base with a prime
factor other than 2 and 5: `0.1 = 3·30⁻¹`.)

Test: `float/tests/log.rs::test_log_exact_integer_results`.

Fix: `float/src/log.rs` — both shortcuts use one test on prime valuations
(`exact_int_log`, built on the new `utils::Factored` in `float/src/utils.rs`).

### ln-min-exponent — ln panics on 1·B^isize::MIN

Reports: E241

Verdict: real, fixed

Root cause: `ln_compute` shifts the argument onto the unit binade with `x_ball.shift(-e_base)`
(`float/src/log.rs:366`); `-e_base` overflows for `e_base = isize::MIN` (debug: "attempt to
negate with overflow"; release: the wrapped shift allocates until "out of memory").

Test: `float/tests/log.rs::test_ln_extreme_exponents`.

Fix: `float/src/log.rs` — the shift is applied in two halves.

Notes: `1·B^isize::MIN` is at the sentinel exponent, which arithmetic results are not allowed to
have, but it is the value `exp` saturates to under an outward directed mode, so it is a valid
input.

### exp-input-rounding — exp returns Err(Underflow) at low precision for |x| ~ 6e18

Reports: E227

Verdict: real, fixed

Root cause: `exp_compute` rounded the argument to the *working* precision before the reduction
(`float/src/exp.rs:150-151`). For `x ≈ -5.8e18` at 1 bit that is an absolute error of ~2^52, far
beyond what the reduction (`extra` digits for `ln B` only) absorbs.

Test: `float/tests/exp.rs::exp_range::test_exp_huge_arguments` (against `exp(x/2)²`).

Fix: `float/src/exp.rs::exp_reduced` — the argument keeps the same `⌈log_B|x|⌉ + 2` extra digits
as `ln(B)`.

### exp-range-edges — exp hangs / overflows / is wrong within the guard digits of the exponent limits

Reports: E230, E252, E249, E144

Verdict: real, fixed (E249 partly not a bug — see notes)

Root cause: the working-precision value was moved to its final scale before rounding:
`v_ball.shift(s)` (`float/src/exp.rs:247`), and `Ball::shift` *saturates* the exponent
(`float/src/ball.rs:222`). A working value has `guard` more digits than the result, so for a
result within `guard` digits of `isize::MIN` its exponent does not fit: the saturated value is
wrong by the clipped power of two (E144: `powf` up to `2^260` too large) and/or its radius has no
`Repr` export, so the loop never certifies (E230, from 222 bits on). At the top, `s` near
`isize::MAX` hit the sentinel (E252: no return just above `ln(10)·isize::MAX`), and the
`s.try_into::<isize>()` gate (`exp.rs:188`, `:821`) reported an overflow for results whose
significand still fits below the sentinel (E249).

Test: `float/tests/exp.rs::exp_range::test_exp_huge_arguments` (E230, E252, E249),
`::pow_range::test_powf_near_smallest_exponent` (E144).

Fix: `float/src/exp.rs` — `exp_reduced` returns `exp(x) = ball · B^s` with the scale split off
(`s: i128`); `exp`, `powf` (through `exp_ball_reduced`) certify the ball in its own frame and
`Context::scale_rounded` applies the scale to the *rounded* result, where the range check is
exact. `exp_compute`/`exp_ball` (used by the hyperbolic functions and base conversion) check the
range instead of saturating (`shift_in_range`).

Notes:
- E249 is partly not a bug. The crate's convention (`Repr::check_finite_exponent`) is that a
  finite result has its normalized exponent strictly between `isize::MIN` and `isize::MAX`.
  `sinh(21237598959199934514) ≈ 323·10^isize::MAX` and `exp(...514) ≈ 647·10^isize::MAX` have a
  3-digit significand at exponent `isize::MAX`, i.e. at the sentinel, so `Err(Overflow)` is the
  consistent answer and the test pins it. The report's other examples are real and fixed:
  `exp(21237598959199934314)` (`895·10^(isize::MAX-87)`) and `exp(21237598949199934514)`
  (`6·10^(isize::MAX-4342944817)`) now return their values, as does everything up to
  `118·10^(isize::MAX-1)`. (`overflow_repr_endpoint` does use exponent `isize::MAX` for its
  "largest finite" saturation value, which is what the report reasoned from — see "unsure".)
- E144 is a `powf` report with this root cause (`pow_exp_log` → `exp_ball`).

### exp-huge-argument — time to report overflow grows with the argument's exponent

Reports: E266

Verdict: real, fixed

Root cause: the overflow probe in `exp_internal` (`float/src/exp.rs:810-834`) and the reduction
in `exp_compute` evaluate `ln(B)` with `⌈log_B|x|⌉ + 2` extra digits and divide — 10^8 digits for
`x = 2^100000000` — although any `|x| > isize::MAX·ln B` is a range error.

Test: `float/tests/exp.rs::exp_range::test_exp_astronomical_arguments` (exp, exp_m1, sinh, cosh).

Fix: `float/src/exp.rs::exp_reduced` — `log2|x| > isize::BITS + 6` is a range error before
anything is evaluated (this also covers `sinh`/`cosh`, which call `exp_compute`); the separate
probe is gone.

### powi-range — powi range decisions (huge exponents, exponent limits)

Reports: E258, E167, E162, E170

Verdict: real, fixed

Root cause: four independent defects in `Context::powi` (`float/src/exp.rs:380-518`) and its
`exp(y·ln x)` fallback.
- E258: the squaring chain ran on the full base (`exp.rs:491-517`), so for a result within the
  guard digits of `isize::MIN` the working-precision intermediate underflowed
  (`Err(Underflow)`, 0 through `FBig::powi`).
- E167: an exponent that does not fit `i64` was treated as unbounded (`exp.rs:439-450`): any
  base whose log2 bound is negative "underflows" — `0.999^(2^63)` is about `10^(-4.0e15)`.
- E162: the "base is exactly ±1" shortcut tested `base.significand.is_one()` (`exp.rs:467`),
  which is false for `-1`; the fallback then computes `exp(n·ln 1)`, an exact 1 behind a positive
  radius, which a directed mode never certifies.
- E170: on the first attempts of the fallback the working precision cannot resolve a base
  `1 + 10^-k`, so `n·ln x` carries a radius of ~10^8 and its exponential an astronomically large
  one; `contained` (`float/src/ziv.rs:312`) then did exact `Repr` arithmetic on that radius,
  materializing the whole exponent gap (seconds to minutes, then "out of memory").

Test: `float/tests/exp.rs::pow_range::test_powi_is_exponent_invariant` (E258),
`::test_powi_exponent_past_i64` (E167), `::test_powi_unit_base_huge_exponent` (E162),
`::test_powi_base_near_one_huge_exponent` (E170).

Fix: `float/src/exp.rs` — the chain runs on the significand with the scale kept apart
(`Scaled`), applied to the rounded result by `scale_rounded`; the range pre-check uses the
exponent as an `f64` (infinite past the `f64` range); `(±1)^n` is resolved for every `n` before
anything else. `float/src/ziv.rs` — `contained` rejects a radius at least as large as the
candidate on exponents alone.

### powf-exact — powf with an exactly representable result does not return

Reports: D23, E48 (duplicates)

Verdict: real, fixed

Root cause: `Context::powf` sent every non-integer exponent to `pow_exp_log`
(`float/src/exp.rs:599`, `620-646`), whose `exp(y·ln x)` ball always has a positive radius. An
exactly representable power (`4^0.5 = 2`) is the edge of a directed preimage, so the loop never
certifies it and doubles its working precision "forever" (32 doublings); under the nearest modes
it returned 2 flagged `Inexact`. `Zero` is the default mode of `FBig`.

Test: `float/tests/exp.rs::pow_range::test_powf_exact_results`.

Fix: `float/src/exp.rs::powf_exact` — with `y = n/d` in lowest terms, decides on the prime
valuations of `x` and `y` (`utils::Factored`) whether `x` is a perfect `d`-th power, and if the
power has few enough digits to be a rounding boundary (representable, or an exact tie) returns it
correctly rounded.

### cache-ln-base — ConstCache::ln_base for a generic base

Reports: E188

Verdict: real, fixed

Root cause: the generic-base arm (`float/src/math/cache.rs:222-237`) returned `ln_compute`'s raw
ball midpoint, evaluated at `precision` with no guard digits and tagged with `precision`: it has
`precision + 2` digits (the reconstruction constants are evaluated wider) and carries the
series' several-ulp error.

Test: `float/tests/log.rs::test_const_cache_ln_base_generic`.

Fix: `float/src/math/cache.rs` — evaluated with guard digits and rounded to `precision`, growing
the guard until both ends of the error interval round to the same value (so the result is the
correctly rounded `ln(B)` under the requested mode, needing only `R: Round`).

## Per-report table

| Report | Cluster | Verdict |
|---|---|---|
| D17 | exact-shortcut-precision | real, fixed |
| D23 | powf-exact | real, fixed |
| D29 | ziv-certification | real, fixed |
| D40 | ziv-certification (sqrt half: sqrt-single-step) | real, fixed |
| E29 | sqrt-single-step | real, fixed |
| E45 | ziv-certification | real, fixed |
| E48 | powf-exact | duplicate of D23 (real, fixed) |
| E61 | exact-shortcut-precision | duplicate of D17 (real, fixed); adds `ln(1)` |
| E69 | ziv-certification | real, fixed |
| E75 | sqrt-single-step | real, fixed |
| E82 | log-exact-power | real, fixed |
| E123 | zero-root-precision | real, fixed |
| E131 | hypot-range | real, fixed |
| E141 | zero-root-precision | real, fixed |
| E144 | exp-range-edges | real, fixed |
| E162 | powi-range | real, fixed |
| E167 | powi-range | real, fixed |
| E170 | powi-range | real, fixed |
| E174 | ziv-certification | real, fixed |
| E185 | hypot-range | real, fixed |
| E188 | cache-ln-base | real, fixed |
| E195 | hypot-range | real, fixed |
| E203 | sqrt-single-step | real, fixed |
| E221 | exact-shortcut-precision | duplicate of D17 (real, fixed); adds `powf(x, 0)`, `powf(1, y)` |
| E225 | ziv-certification (via sqrt-single-step) | real, fixed |
| E227 | exp-input-rounding | real, fixed |
| E229 | ziv-certification | real, fixed |
| E230 | exp-range-edges | real, fixed |
| E241 | ln-min-exponent | real, fixed |
| E249 | exp-range-edges | real, fixed for `exp`; the `sinh`/`cosh`/`exp` examples at exponent `isize::MAX` are not a bug (sentinel exponent) |
| E252 | exp-range-edges | real, fixed |
| E258 | powi-range | real, fixed |
| E266 | exp-huge-argument | real, fixed |

## Changelog entries

`float/CHANGELOG.md`, under `## Unreleased`:

```markdown
### Fix
- Fix the correct-rounding certification (Ziv loop) next to a power of the base: `ErrorBounds::error_bounds` used a full ulp on the toward-zero side of `±B^k`, where the neighbour is in the finer binade below, so a value just below a power of the base could be returned as that power (`exp`/`powi` returning `1` for `0.99949…` at 3 digits, `powf(x, 0.5)` returning `8` for `3.9993` at 1 bit, `15^-16` at 1 bit). The ties-to-even inclusion flags of `HalfEven::error_bounds` now use the significand parity at the precision.
- Fix the `Exact`/`Inexact` flag (and the rounding direction) reported by the Ziv-backed functions (`exp`, `ln`, `log2`, `log10`, `powi`, `powf`, `hypot`, …): it described the working approximation rather than the true value, so an inexact result could be flagged `Exact` (`ln(55)` at 2 bits, `183^-8` at 53 bits, `hypot(1, 2^40)`) or carry the wrong direction, and under `Zero` a result above the true value could be returned.
- Fix `powi` failing with `ZivRetryLimitExceeded` (a panic through `FBig::powi`) when the exact power is a rounding tie (`5^2` at 1 decimal digit under `HalfEven`).
- Fix `sqrt` in non-binary bases: an exact root that is a rounding tie did not terminate (`sqrt(225)` at 1 decimal digit), a perfect square was reported `Inexact` or mis-rounded in base 4 and did not terminate in base 9 (`sqrt(36)`, `sqrt(441)`), and an inexact root could be flagged `Exact`. `sqrt` is now a single correctly rounded step in every base.
- Fix `sqrt` of a value with an exponent next to `isize::MIN`/`isize::MAX` (debug overflow panic, wrong exponent in release builds).
- Fix `hypot` for operands far from the middle of the exponent range: spurious `Underflow`/zero or a wrong exponent near `B^(isize::MIN/2)`, spurious `Overflow` near `B^(isize::MAX/2)` when a significand is wider than the precision, and a running time proportional to the exponent gap under directed rounding modes. `hypot` is now computed from the exact sum of squares.
- Fix `exp(0)`, `exp_m1(±0)`, `exp_m1(-inf)`, `ln(1)`, `ln_1p(±0)`, `log2(1)`, `log10(1)`, `powi(x, 0)`, `powf(x, ±0)`, `powf(1, y)` and `powf(±0, y)` returning a result with precision 0 (unlimited) instead of the context's precision, which made the next operation on the result panic.
- Fix `cbrt`, `nth_root` and `hypot` panicking on an unlimited-precision zero (`FBig::try_from(0.0)`, `DBig::ZERO`).
- Fix `log2` not terminating under directed rounding for a negative power of two in a non-binary base (`log2(0.5)` in base 10), and `log10` likewise for a power of ten in a base with other prime factors.
- Fix `ln`/`log2`/`log10` panicking on a value with exponent `isize::MIN` (the value a directed-mode underflow of `exp` saturates to).
- Fix `powf` not terminating under directed rounding (including `FBig`'s default `Zero`) when the result is exactly representable (`powf(4, 0.5)`, `powf(0.25, 1.5)`); such results are now returned as `Exact`.
- Fix `exp` for arguments around `±isize::MAX·ln(B)`: a spurious `Underflow` at low precision, no termination or a spurious `Overflow` for a result within a few hundred digits of either end of the exponent range, and a running time proportional to the argument's exponent before an overflow was reported (also through `sinh`/`cosh`).
- Fix `powf` returning a value up to `2^260` times too large for a result just above the smallest exponent.
- Fix `powi`: a spurious `Underflow`/`Overflow` for an exponent of `2^63` or more (`0.999^(2^63)`) and for a negative exponent with a result just above the smallest exponent; no termination for a base of `-1` with an exponent past `2^64` under `Down`/`Up`; and an out-of-memory panic for a base near one with an exponent past `2^64`.
- Fix `ConstCache::ln_base` for a base other than 2, 10 or a power of two returning a value with `precision + 2` digits and several ulps of error; it is now the correctly rounded `ln(B)`.

### Change
- `exp`, `exp_m1` and `ln_1p` of an argument too small to be visible next to the result's leading term are rounded directly (no series evaluation), in time independent of the argument's exponent.
```

## Cross-group notes

Files touched outside this group's own `exp.rs` / `log.rs` / `root.rs`:

- `float/src/ziv.rs` (shared with float-trig) — only `Context::contained` and two unit tests:
  1. a zero radius returns `true` before any other test;
  2. a radius whose magnitude is at least the candidate's returns `false` on exponents alone
     (before any `Repr` arithmetic);
  3. a positive radius must satisfy `|a − y| > e` (candidate strictly outside the error
     interval) in addition to the preimage containment.
  The unit tests `ziv_retries_until_contained` / `ziv_pair_retries_until_both_contained` used an
  approximation equal to the candidate with a positive radius, which (3) can never accept; they
  now use `1 + 2^-6`. `ziv_few_retries_for_typical_inputs` resets the counter to 0 instead of
  `usize::MAX` before `exp`, because `exp(1e-6)` at 10 bits no longer enters a loop.
  **Consequence for float-trig**: any Ziv-backed function whose true value is exactly
  representable must report a zero radius or resolve the case before the loop in *every* mode
  (previously the nearest modes returned such a value with a possibly wrong flag). And a result
  that is a known value plus an invisible tail (`sin(x) ≈ x`, `cos(x) ≈ 1`, `sinh`, `atan`, … for
  tiny `x`) now retries until the working precision resolves the tail — correct, but slow for
  very small arguments (measured here before the shortcut was added: `exp(2^-100000)` at 20 bits
  under `HalfEven` went from 20 µs with a wrong `Exact` flag to 114 ms; it already behaved that
  way under the directed modes). `Context::round_sticky` (below) is the shortcut for those.
- `float/src/round.rs` — the `ErrorBounds::error_bounds` impls of all six modes (public trait).
  Other consumers: `complex/src/ziv.rs:145` and `rational/src/third_party/dashu_float.rs:206`.
  Both get the tighter (correct) preimage; their test suites pass unchanged.
- `float/src/repr.rs` — new `pub(crate) Context::round_sticky(v, sticky_sign)`.
- `float/src/utils.rs` — new `pub(crate) struct Factored` (prime valuations of `sig·B^e`).
- `float/src/math/cache.rs` — `ConstCache::ln_base`, generic-base arm only (E188).

Not touched, but relevant to other groups:

- `float/src/ball.rs` (float-trig): `Ball::shift` saturates the exponent silently
  (`ball.rs:222`). `exp.rs` now range-checks before calling it (`shift_in_range`); other callers
  still get the saturated value at the ends of the exponent range.
- `float/src/math/hyper.rs` (float-trig): `sinh`/`cosh` map *every* error of their closure to
  `Overflow`. They benefit from the `exp_compute` changes (E266's `sinh`/`cosh` half is fixed and
  tested here; `sinh(21237598959199934507)` now returns `295·10^(isize::MAX-3)` instead of an
  overflow), but were not otherwise changed. The hyperbolic/trig exact shortcuts that return
  precision-0 constants, if any, are theirs.
- `complex/src/ziv.rs` (cmplx) has its own copy of the retry loop and containment test; the three
  `contained` changes above were not mirrored there.
- E144 is a `powf` report whose root cause is the `exp` scale handling; E225 and half of D40 are
  `hypot`/`sqrt` reports whose root cause is the shared Ziv flag defect.

## Verification

- Each new integration test was run against an extracted copy of `a0fc535` (the new test files
  dropped into it), one test per process with a 40 s limit — see the final report for the table.
- `cargo test -p dashu-float` (unit, integration and doc tests), `cargo test --workspace
  --exclude dashu-python`, `cargo fmt --all -- --check` and `cargo clippy --all-features
  --all-targets --workspace --exclude dashu-python -- -D warnings` pass at the head of this
  branch.
