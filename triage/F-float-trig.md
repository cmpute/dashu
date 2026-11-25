# F-float-trig

Notes for the `float-trig` group of run F (10 reports, against `8137b6e`): dashu-float
trigonometric and hyperbolic functions, including the `_pi` and `_unit` variants. All 10
reports reproduced and are real; 7 distinct root causes, all fixed.

How each was reproduced: every report's program was run in a scratch crate against this tree
(`b0705ed`, i.e. `8137b6e` plus the triage notes) and against the base checkout `a0fc535`, in
release builds, each call under a timeout. The regression tests were then seen to fail without
the fix where that is cheap to show (noted per cluster); the two non-termination clusters were
confirmed by running the test binary under an alarm with the fix switched off.

Oracles: the new tests use exact constructions (integer arithmetic for the neighbours of a
midpoint, an exact `±(1 − 2^-(p+5))` for a result that hugs `±1`, the same significand at an
ordinary exponent for the scaling functions) or a wider π re-rounded once. The
workspace-excluded `fuzz/` crate builds here (its `rug` dependency bundles MPFR);
`fuzz/tests/float_transcendental.rs` and `fuzz/tests/float_trig_random.rs`
(`--release -- --ignored`, `FUZZ_CASES=1024`, 43 tests) pass on the final commit.

## Clusters

### Results next to a midpoint are never certified in an odd base
Reports: F51, F72
Verdict: real, fixed
Root cause: in an odd base half an ulp has no finite expansion, and `half_ulp`
(`float/src/round.rs:215`) cuts it off after `256/bits(B) + 1` digits (129 in base 3), so the
round-to-nearest `error_bounds` stop about `3^-129` ulps short of the midpoint.
`Context::contained` (`float/src/ziv.rs`) compared the Ziv interval against those bounds only:
an interval inside that last sliver failed at every working precision, however small its
radius, and the loop ran to its retry cap at astronomically large precisions. Results that hug
a midpoint are structural there: every half-integer is a midpoint (`1/2 = 0.111…₃`), which is
where `acos_unit(tiny, 2)`, `atan_unit(huge, 2)` and `atan2_unit(1, tiny, 2)` sit (F51), and
`x − x³/6` is an exact midpoint of two 255-digit floats for `x = c·3^-100` (F72, `asinh`). The
driver of dashu-cmplx (`complex/src/ziv.rs`) has the same test and the same hang.
Base behaviour: (run) all seven calls of the two reports return at `a0fc535` in milliseconds.
F51's three calls return the correct `29524·3^-10`; F72's `asinh(7·3^-100)` is correct, and
`asinh(3^-100)` at 255 digits comes back **one ulp low** (significand ending `…422`; the
correctly rounded value, which the new test builds exactly, ends `…423`).
Provenance: **regression**, from `f2b0479` ("float: keep ErrorBounds::error_bounds inside the
values that round to the float"). The base used the half-ulp digit `(B + 1)/2` — two thirds of
an ulp in base 3, a bound that reaches past the midpoint — so it terminated, and certified the
wrong neighbour for a result within a sixth of an ulp of a midpoint (the wrong `asinh(3^-100)`
above). `f2b0479` made the bound sound but short, which turned that into non-termination. So
for F72 it is a mixture: wrong value at the base, hang after the fix.
Test: `float/src/ziv.rs::tests::contained_next_to_a_midpoint_in_an_odd_base` (fails without the
fix); `float/tests/trig.rs::test_inverse_unit_next_to_half_in_odd_base` (bases 3, 5, 7; 1, 10,
20, 50 digits; distances from `B^-10` to `B^-(isize::MAX/4)`; exact neighbours and flags);
`float/tests/hyper.rs::test_asinh_near_tie_in_odd_base` (exact expected significands);
`complex/tests/hyper.rs::asinh_near_tie_in_odd_base` (hangs without the complex half of the
fix — checked under an alarm).
Fix:
- `Context::contained` in both drivers: when the bounds test fails in an odd base, the interval
  is accepted iff both of its exact ends round to the candidate (rounding is monotone). Even
  bases are untouched. `ErrorBounds::error_bounds` itself is unchanged (its docs already say
  the odd-base bound falls short).
- `round_near_quarter` (`float/src/math/trig.rs`) used to decline when `k·u/4` has no finite
  expansion ("nothing nearby is a rounding boundary") — wrong for a half-integer in an odd
  base. New `round_near_quarter_ratio` rounds the exact rational `k·u/4 ± B^(g−2)/4` instead,
  so these calls no longer need a working precision of `log(1/x)` digits (the report's
  `k = 1_000_000` took 3.5 s through the loop even with the containment fix; it is now
  immediate, as is `3^-(isize::MAX/4)`).
Notable: F72's `asinh` needs no shortcut — its near-tie resolves at about twice the precision
once containment works.

### The Rounding flag next to a representable value
Reports: F19
Verdict: real, fixed
Root cause: `Context::certified` (`float/src/ziv.rs`) pursues the side of the true result only
while `guard < precision + 16`, then accepts the attempt with the approximation's flag (item 11
of "Changes that need a maintainer's decision"). The trig loops start at a guard of 50, so
below 34 digits nothing was pursued at all. With an argument that carries more digits than the
context, the approximation is exactly `±1` at the working precision and its `Exact` was
returned. The `cos_unit` shortcut that the earlier triage added (`tiny_cos_unit`) only covered
a reduced argument next to a multiple of `u` (`UnitReduced::Small`), not the other extrema
(`2 − 10⁻⁴⁰`, `0.5 + 10⁻⁴⁰`), which is why `cos_pi(1e-40)` was right and its mirror images were
not.
Base behaviour: (run) the same three wrong `Exact(1)`, plus `cos_pi(1e-40) = Exact(1)`, which
the earlier triage fixed.
Provenance: **incomplete-fix** of the earlier cluster "Results that hug an exact value"
(E251; commit `71a46a0`). The defect is in base code and fails identically at the base; the
E251 fix covered tiny arguments and a bounded pursuit, not these.
Test: `float/tests/trig.rs::test_unit_next_to_extremum_is_inexact` (the report's inputs; all
quadrants × 5 modes × 20/50/100/500 bits against the exact `±(1 − 2^-(p+5))`, at a distance
the loop decides and at two the shortcut decides);
`float/tests/trig.rs::test_flag_next_to_a_representable_value` (first-order neighbours of
`tan_pi(1/4)`, `sin_unit(30°)`, `cos_unit(60°)`, `asin_unit(1/2)` at `10^-40/-100/-300`, and
`sin`/`cos` of π/2 and π held to 100–400 digits);
`float/src/ziv.rs::tests::ziv_irrational_resolves_the_side_at_any_distance`. Both integration
tests fail with the two changes below switched off.
Fix:
- New `Context::ziv_irrational` (and the pair driver, now `ziv_pair_irrational`): for a
  function whose result is never a rounding boundary off its exact rows, the loop continues
  until the side is known. Every loop in `float/src/math/trig.rs` and `hyper.rs` uses it — the
  results there are transcendental at nonzero rational arguments (Lindemann–Weierstrass;
  Gelfond–Schneider for the ×π hyperbolics), or rational only at the tabulated rows of the ×u
  family (Niven), all of which are resolved before the loop. Plain `ziv` keeps its bound for
  the callers that can still reach an unresolved exact result (`powf(4, 0.5)`).
- `sin_unit`, `cos_unit` and `sin_cos_unit` round `±(1 − tiny)` directly next to any extremum
  (`tiny_cos_unit` now takes the `Split` reduction too).
Notable:
- This removes the bounded-flag compromise for this group's functions only. The cost is that a
  nearest-mode call next to a representable value now does the work a directed-mode call
  already did (the same separation decides the value there): proportional to the digits of
  the argument. A sweep of all 26 functions over `m·B^e` (`|m| ≤ 24`, `−6 ≤ e ≤ 3`, bases 2,
  10, 3, precisions 1–40) shows no call above 61 ms and none that hangs.
- Found on the way, same root cause: `tan_pi(1/4 + 1e-100)` was `Exact(1)`,
  `sin_unit(30 + 1e-100, 360)` was `0.5` with `AddOne` (the result is above 0.5), and `sin` of
  π/2 to 100 digits at 10 digits was `Exact(1)`.

### ×u / ×π functions at the bottom of the exponent range
Reports: F12, F82, F89
Verdict: real, fixed
Root cause: `sin_unit`/`tan_unit` (`float/src/math/trig.rs`) and `sinh_pi`
(`float/src/math/hyper.rs`) form `(2π/u)·x` / `π·x` at the working precision. For an `x`
within that many places of `isize::MIN` the product's last place is below the exponent range
and the ball multiplication reports an underflow — for a result that may be representable.
Then (F82, F89) `sin_unit`/`tan_unit` evaluate on `|x|` and apply the sign to the value only,
so the error came out as `Underflow(Positive)` for a negative `x` (and `FBig<Up>` turned it
into a positive number); and (F12) `sinh_pi`/`sinh_cosh_pi` mapped *every* loop error to
`Overflow(sign)`, so the underflow became `±inf`.
Base behaviour: (run) F82/F89: identical — `Underflow(Positive)` for `−2^(isize::MIN + k)`,
`k = 0, 1, 100`, and at the base also for `k = 1000` and for the radian `sin`/`tan`. F12:
`sinh_pi`, `sinh` and even `cosh_pi` return `Overflow` (`inf`) for all three inputs of the
report, including `2^(isize::MIN + isize::MAX/2)`.
Provenance: F82, F89 — **pre-existing** (same failure at the base; the earlier E165 fix
narrowed the failing band from "below `isize::MIN/2`" to "within the working precision of
`isize::MIN`" without touching the sign). F12 — **incomplete-fix** of E165 ("Results that hug
an exact value", the underflow part; commits `71a46a0`, `38a15d7`): the same `±inf` for a tiny
argument, repaired for `sinh` and for most of the range of `sinh_pi`, not for the band next to
`isize::MIN`.
Test: `float/tests/trig.rs::test_unit_functions_at_bottom_of_exponent_range` (the reported
inputs with exact expected values and error signs; `sin_unit`, `tan_unit`, `asin_unit`,
`atan_unit` × 4 modes × 20/50/100/500 bits against the same significand at `2^-2000`, on both
sides of the exponent where the direct scaling takes over);
`float/tests/hyper.rs::test_sinh_pi_at_bottom_of_exponent_range`.
Fix: new `Context::tiny_scaled` — for an argument in the bottom quarter of the exponent range
these functions are `c·x` to a relative `B^(isize::MIN/2)`, so the loop rounds `c·m` for the
significand `m` and the result is placed at the exponent of `x`, where the range is checked on
the *rounded* value with the sign of the result. Used by `sin_unit`, `tan_unit`, `asin_unit`,
`atan_unit` and `sinh_pi` (and through them `sin_pi`, `tan_pi`, `sin_cos_unit`,
`sinh_cosh_pi`). `sinh_pi`'s error mapping reports an underflow for a fractional argument.
Notable:
- `sin_pi(−2^(isize::MIN + 100))` at 10 bits is now `−201·2^(isize::MIN + 94)` (it is
  representable), not an underflow of either sign; `k = 0, 1` are `Underflow(Negative)`.
- The reports' `Repr::new(1, isize::MIN)` uses the sentinel exponent itself; nothing depends
  on it.
- Not changed: the radian odd functions still report an underflow for an argument within the
  *target* precision of `isize::MIN` (the earlier triage's `tiny_odd` rule), even where the
  nearest float is `x` itself.

### atanh(±1) at unlimited precision
Reports: F21
Verdict: real, fixed
Root cause: `Context::atanh` (`float/src/math/hyper.rs`) ran `assert_limited_precision` before
the `|x| = 1 → ±inf` row.
Base behaviour: (run) the same panic for all three calls (and `acosh(1)` panicked too).
Provenance: **incomplete-fix** of E223 ("Trig/hyperbolic functions panic on a precision-0
zero", commit `3c0de96`), which moved the zero rows and `acosh(1)` ahead of the assertion and
explicitly left this one ("Known leftovers" in `SUMMARY.md`).
Test: `float/tests/hyper.rs::test_atanh_one_is_infinity` (extended),
`test_atanh_unlimited_precision_inexact_panics`.
Fix: the domain match precedes the assertion. The `FBig::atanh` docs said it panics at
`|x| = 1`; they now say it returns the signed infinity.

### Exact rows of the ×u family at unlimited precision
Reports: F30
Verdict: real, fixed
Root cause: `acos_unit` asserted a limited precision first thing, and `sin_unit`, `cos_unit`,
`sin_cos_unit`, `tan_unit`, `asin_unit`, `atan_unit`, `atan2_unit` right after their zero row
— before the tables of exact values (`float/src/math/trig.rs`).
Base behaviour: (run) the same panics, and at the base `acos(1)` and `sin_unit(0)` panicked as
well.
Provenance: **incomplete-fix** of E223 (commit `3c0de96`): it resolved the zero rows and
`acos(1)`, missed `acos_unit(1) = 0`, and deliberately left the rows that are a ratio.
Test: `float/tests/trig.rs::test_unit_exact_rows_at_unlimited_precision`,
`test_unit_inexact_at_unlimited_precision_panics`.
Fix: the assertion follows the exact rows in all eight functions. `exact_ratio` returns a table
ratio at unlimited precision when it has a finite expansion in the base (`u/4 = 90`,
`1/2` in base 2 or 10) and keeps the limited-precision panic otherwise (`1/4` in base 3).
Notable: the report's second expectation, `FBig::ZERO.acos_unit(360) == 90`, goes beyond the
earlier decision ("rows that are a rounded constant still need a precision"); I followed the
report where the row is *not* rounded — an exactly representable ratio — and only there. The
radian functions are unchanged (their non-zero rows are multiples of π).

### atan / atan2 of a huge argument or ratio
Reports: F26
Verdict: real, fixed
Root cause: `atan_ball` (`float/src/math/trig.rs`) computes `π/2 − atan(1/x)` for `|x| ≥ 1`,
and the series in `atan_compute` multiplies terms of the order of `(1/x)³`: past a third of
the exponent range that underflows, and the error was returned as the result. Past the top of
the range `1/x` itself underflows. `atan2` forms `y/x` first, with the same consequences, and
an overflowing or underflowing ratio was returned as that error.
Base behaviour: (run) `Err(Underflow(Positive))` for *both* `3e3074457345618258582` and
`…583`, for `atan_unit` on them, and for the binary case; `atan2(1, 3e-3074457345618258602)`
works and `…603` fails, as at `8137b6e`.
Provenance: **pre-existing** (the earlier triage's changes happen to move the `atan`
threshold up past `…582` and fix `atan_unit` through its quarter-turn shortcut; the defect and
its failure are the base's).
Test: `float/tests/trig.rs::test_atan_of_huge_argument` (the reported inputs; 4 modes ×
20/50/100/500 bits: same value and flag as `atan(±inf)` up to `2^(isize::MAX − 100)`, ratios
beyond the exponent range in both directions, `±π` next to the negative x axis against a wider
π).
Fix: `atan_ball` returns `π/2` widened by an ulp once `1/|x|` is below the working precision;
`atan2` maps an overflowing ratio to `±π/2` and an underflowing one to an underflow with the
sign of `y` (right half plane — the angle *is* the ratio there) or to `±π` (left half plane).

### atan2 / atan2_unit ignore the precision of x
Reports: F48
Verdict: real, fixed
Root cause: `FBig::atan2` / `FBig::atan2_unit` (`float/src/math/trig.rs`) and their
`CachedFBig` mirrors (`float/src/fbig_cached_ops.rs`) computed at `self.context`, where every
other two-operand method (`+ − × ÷`, `powf`, `hypot`) uses `Context::max` of the operands, as
the `Context` docs promise for binary operations.
Base behaviour: (run) identical output.
Provenance: **pre-existing**.
Test: `float/tests/trig.rs::test_atan2_takes_the_higher_precision` (the report's case; base 2
at 20/50/100/500 bits either way round; the cached wrappers).
Fix: both methods and both mirrors use `Context::max(self.context, x.context)`; documented,
with an example on `atan2`.
Notable: a behaviour change for callers that relied on the receiver's precision — see
cross-group notes.

## Per-report table

| Report | Cluster | Verdict | Provenance |
|---|---|---|---|
| F12 | ×u / ×π functions at the bottom of the exponent range | real, fixed | incomplete-fix (E165) |
| F19 | The Rounding flag next to a representable value | real, fixed | incomplete-fix (E251) |
| F21 | atanh(±1) at unlimited precision | real, fixed | incomplete-fix (E223) |
| F26 | atan / atan2 of a huge argument or ratio | real, fixed | pre-existing |
| F30 | Exact rows of the ×u family at unlimited precision | real, fixed | incomplete-fix (E223) |
| F48 | atan2 / atan2_unit ignore the precision of x | real, fixed | pre-existing |
| F51 | Results next to a midpoint are never certified in an odd base | real, fixed | regression (`f2b0479`) |
| F72 | Results next to a midpoint are never certified in an odd base | real, fixed | regression (`f2b0479`); the base returned a value one ulp off |
| F82 | ×u / ×π functions at the bottom of the exponent range | real, fixed | pre-existing |
| F89 | ×u / ×π functions at the bottom of the exponent range | duplicate of F82 (real, fixed) | pre-existing |

## Changelog entries

`float/CHANGELOG.md`, under `## Unreleased`:

```markdown
### Change
- `FBig::atan2` / `FBig::atan2_unit` (and the `CachedFBig` mirrors) return the result at the
  higher of the two operands' precisions, like the binary arithmetic operations; they used the
  receiver's precision only.

### Fix
- In an odd base, the transcendental functions no longer fail to return for a result within
  about `B^-129` ulps of the midpoint of two floats under the round-to-nearest modes
  (`acos_unit(3^-140, 2)`, `atan_unit(3^140, 2)`, `asinh(3^-100)` at 255 digits); the inverse
  ×u functions next to a half-integer axis are rounded directly.
- The trigonometric and hyperbolic functions report the `Rounding` flag of the true result
  however close it lies to the returned value: `cos_pi(2 - 1e-40)`, `sin_pi(0.5 + 1e-40)`,
  `tan_pi(0.25 + 1e-100)` or `sin` of π/2 to 100 digits are no longer `Exact(1)` at a lower
  precision.
- `sin_unit` / `tan_unit` (and `sin_pi`, `tan_pi`), `asin_unit`, `atan_unit` and `sinh_pi` of
  an argument near the bottom of the exponent range return the representable result, or an
  underflow with the sign of the result; `sin_pi` / `tan_pi` reported a positive underflow for
  a negative argument, and `sinh_pi` / `sinh_cosh_pi` reported an overflow (`±inf`).
- `atan` of an argument above about `B^(isize::MAX/3)`, and `atan2` of a ratio that large (or
  beyond the exponent range), return the rounded `±π/2` (or `±π`) instead of an underflow.
- `atanh(±1)` returns `±inf` at unlimited precision instead of panicking, and the docs no
  longer claim a panic at `|x| = 1`.
- The exact rows of `sin_unit`, `cos_unit`, `sin_cos_unit`, `tan_unit`, `asin_unit`,
  `acos_unit`, `atan_unit` and `atan2_unit` (`0`, `±1`, and the table ratios that have a finite
  expansion in the base) no longer panic at unlimited precision (`FBig::ONE.acos_unit(360)`).
```

`complex/CHANGELOG.md`, under `## Unreleased`:

```markdown
### Fix
- In an odd base, a part within about `B^-129` ulps of the midpoint of two floats is certified
  under the round-to-nearest modes; such calls did not return (`asinh(3^-100)` at 255 digits).
```

## Cross-group notes

Files touched outside `float/src/math/{trig,hyper}.rs` and their tests:

- `float/src/ziv.rs` (shared Ziv machinery; the float-exp group may also be in it):
  - `Context::contained`: an odd-base fallback after the bounds test (round the two exact ends
    of the interval). A `let context = y.context;` is taken before `y` is shadowed.
  - `ziv` is now a thin wrapper over a private `ziv_impl(initial_guard, irrational, approx)`;
    new `ziv_irrational`. `certified` takes a fifth argument `irrational: bool`.
  - **`ziv_pair` is renamed `ziv_pair_irrational`** and always pursues the side (all of its
    callers are trig/hyperbolic pairs). Another group that calls `ziv_pair` will need the new
    name — and should check that neither result can be an unresolved exact value, or the
    nearest modes will loop there the way the directed modes already do.
  - New unit tests `contained_next_to_a_midpoint_in_an_odd_base`,
    `ziv_irrational_resolves_the_side_at_any_distance`; the four `ziv_pair_*` tests call the
    renamed driver.
  - This bears on item 11 of "Changes that need a maintainer's decision": the flag
    certification is no longer bounded for the trig and hyperbolic families. It still is for
    `exp`, `ln`, `powf`, roots (plain `ziv`) and in dashu-cmplx. `exp`, `exp_m1`, `ln`, `ln_1p`
    could adopt `ziv_irrational` on the same argument (transcendental off their exact rows);
    `powf`/`powi`/roots cannot without resolving their exact results first.
- `complex/src/ziv.rs` (complex groups' driver): the same odd-base fallback in `contained`,
  through the public `FBig::with_precision`. `complex/tests/hyper.rs`: one new test.
- `float/src/fbig_cached_ops.rs`: `CachedFBig::atan2` / `atan2_unit` (mirrors of F48).
- Not touched: `float/src/round.rs`, `float/src/repr.rs`. The odd-base `half_ulp` cut-off is
  still there and still documented as falling short; with the fallback in both drivers nothing
  depends on how far it reaches.

For a maintainer to look at:

- F48 changes the precision of the result of `FBig::atan2`/`atan2_unit` when the operands'
  precisions differ (the receiver's → the higher one). The Python binding calls the
  context-level functions and is unaffected.
- dashu-cmplx's flag proof is still budgeted (`prove_flag`), so its near-representable flags
  can still be wrong; not this group's.
- The float functions dashu-cmplx calls on its axes (`fctx.sin`, `atan2`, …) now prove their
  flags; `cargo test -p dashu-cmplx` is unchanged in results and in time (the `exp` test file
  takes 16 s with or without the change).

Verification status:

- Run: `cargo test -p dashu-float --all-features`, `cargo test -p dashu-cmplx -p dashu-ratio
  -p dashu --all-features`, `cargo fmt --all -- --check`, `cargo clippy --all-features
  --all-targets --workspace --exclude dashu-python -- -D warnings`, `cargo check -p dashu-float
  -p dashu-cmplx --no-default-features`, and the two fuzz files above — all clean.
- Not run: a build at the MSRV (1.68 is not installed here). The new code uses nothing newer
  than or-patterns under a binding (`q @ (A | B)`, 1.53) and `isize::unsigned_abs` (1.51).
- Inferred, not run: that dashu-cmplx behaved at the base as dashu-float did for the odd-base
  cluster (same mechanism; only the float calls were run against the base).
