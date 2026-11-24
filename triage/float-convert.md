# float-convert

35 reports, 22 distinct issues, all real and all fixed. Every cluster was reproduced at the
base commit by running its regression test against an unmodified export of `a0fc535` (see
"Reproduction" at the end); the report's own reproducer was not re-run separately.

## Clusters

### to_f32/to_f64 just below the smallest subnormal
Reports: A13, D16, E19
Verdict: real, fixed
Root cause: `significand_bits` (float/src/convert.rs) clamps the kept width to 1 bit for a value
whose top bit is below the subnormal grid, so `convert_to_f64/f32` rounded e.g. `1.25·2^-1075` to
one bit (`2^-1075`, the half-way point) and `FloatEncoding::encode` then rounded that tie to 0: a
double rounding. Independent of the `base/` `encode` issue (the value handed to `encode` was
already wrong).
Test: float/tests/convert.rs::test_to_float_below_smallest_subnormal
Fix: `convert_to_f64/f32` decide values in `(0, min subnormal)` once, with
`R::round_low_part` against half the smallest subnormal (new `round_below_subnormal`), and
return `Inexact(±0 | ±min subnormal, flag)`. Also fixes the HalfAway tie at exactly half
(now the subnormal, was 0) and the flag (`AddOne` when rounded up).

### to_f32/to_f64 between MAX and MAX + ½ulp
Reports: E77
Verdict: real, fixed
Root cause: the range check in `convert_to_f64/f32` returned `Overflow` for everything above
`MAX`, and the overflow endpoint for a nearest mode is infinity.
Test: float/tests/convert.rs::test_to_float_just_above_max
Fix: new `strict_range` flag. `to_f32/to_f64` (flag off) only reject `|x| >= 2^emax` up front and
let the mode round values in `(MAX, 2^emax)`; `TryFrom` (flag on) keeps reporting every value
above `MAX` as `OutOfBounds` under every mode, as before.
Committed together with the previous cluster (same function).

### Negative zero in integer conversions and is_int
Reports: B2, E87, E134
Verdict: real, fixed
Root cause: `-0` is stored as significand 0 with exponent `-1`; `FBig::to_int`, `Repr::to_int`,
the primitive `TryFrom` impls (float/src/convert.rs) and `Repr::is_int` (float/src/repr.rs) read
the exponent before checking for zero, and the unsigned impls rejected its negative sign.
Test: float/tests/convert.rs::test_negative_zero_to_int
Fix: zero is checked first in all of them. Side effect: `powf(x, -0)` now takes the integer
exponent path (same result, 1).

### with_base precision (off by one, and zero digits for narrow floats)
Reports: E26, E93, D36, C3 (the subnormal half)
Verdict: real, fixed
Root cause: `with_base` (float/src/convert.rs) computed the new precision as
`log2_bounds(B^p).0 / log2_bounds(NewB).1` in f32: one short whenever `B^p` is an exact power of
`NewB`, and 0 (= unlimited, hence the "precision cannot be 0" panic in `convert_base`) for a
binary float of 1–3 bits. `num_traits::FromPrimitive::from_f32/f64` and `to_decimal` go through it.
Test: float/tests/convert.rs::test_with_base_precision, ::test_base_change_narrow_precision;
float/tests/num_traits.rs::test_num_traits_from_subnormal_float
Fix: new `converted_precision`: exact for bases with a common root, fixed-point `log2_base`
brackets otherwise, at least 1 digit for a limited precision. No big power is built any more.

### Zeros in base conversion (panic on -0, sign of zero lost)
Reports: C3 (the -0.0 half), E38, E122
Verdict: real, fixed
Root cause: `convert_base` had no zero case; `-0` (exponent `-1`) fell through the shortcuts
into whichever path the pair of bases selected: a panic at unlimited precision, `+0` elsewhere.
Test: float/tests/convert.rs::test_base_change_zero
Fix: `convert_base` returns `Exact(zero_with_sign)` for a zero significand before any exponent
arithmetic.

### with_base_and_precision to the same base does not round
Reports: E30
Verdict: real, fixed
Root cause: the `NewB == B` shortcut in `convert_base` returned the input as `Exact` untouched.
Test: float/tests/convert.rs::test_base_change_same_base
Fix: the shortcut goes through `repr_round` (infinities stay exact).

### Base conversion wrong for |exponent| > 38
Reports: D39, E22
Verdict: real, fixed
Root cause: beyond `THRESHOLD_SMALL_EXP` (38 on 64-bit) `convert_base` estimated the result
from `ln`/`exp` at *twice the target precision*, which is far too little for a short target
(2^-39 to one digit gave 4e-12), is never exact, and cannot see ties.
Test: float/tests/convert.rs::test_base_change_large_exponent (exact oracles: the value built
directly in the target base, or one correctly rounded division; 5 modes × 7 precisions incl.
20/50/100/500 × exponents up to ±20000), ::test_base_change_huge_exponent (±10^12),
::test_base_change_common_root
Fix: `convert_base` now (a) converts exactly, at any exponent, between bases with a common root
(4↔8), (b) evaluates the value exactly whenever an exact result or an exact tie is still
possible (bound derived in the code comment: `|e| <= bits(NewB)·((p+2)·bits(NewB) + bits(sig))`),
and (c) beyond that brackets `|sig|·B^e` with `Down`/`Up` `powi`+`mul` at a growing working
precision until both ends round identically (new `convert_base_far`). The logarithm path and
the unused `cache` parameter are gone; `utils::common_root` is new. An integer (exponent >= 0)
of unlimited precision now converts losslessly to any base instead of panicking.
Committed together with the three previous clusters (all in `convert_base`/`with_base`).

### abs_cmp against UBig/IBig uses the signed order
Reports: D31, E31
Verdict: real, fixed
Root cause: `repr_cmp_ubig`/`repr_cmp_ibig` (float/src/cmp.rs) honour `ABS` in the sign and
estimate steps but ended with a signed `cmp` of the aligned integers.
Test: float/tests/cmp.rs::test_abs_cmp_with_integers
Fix: the final step uses `abs_cmp` when `ABS`. (This also corrects the overflow range check in
`convert_to_f64/f32` for negative values, which calls `Repr::abs_cmp(&UBig)`.)

### NumOrd: zero ranks above small positive floats
Reports: D27, E55
Verdict: real, fixed
Root cause: `NumOrd<f32/f64> for Repr` (float/src/third_party/num_order.rs) estimates the
magnitude from `bit_len(significand) + ...`, which is 0 for a zero, i.e. it treats zero like 1/2.
Test: float/tests/num_order.rs::test_ord_zero_with_float
Fix: a zero `Repr` against a nonzero float is answered from the sign before the estimate.

### NumOrd against floats overflows for extreme exponents
Reports: E175
Verdict: real, fixed (debug: panic; release: wrong order — both checked)
Root cause: same function, `B.bit_len() as isize * self.exponent` overflows `isize`.
Test: float/tests/num_order.rs::test_ord_with_float_extreme_exponent
Fix: the estimate is computed in `i128`.

### Same-base NumOrd allocates the full power of the base
Reports: E257
Verdict: real, fixed
Root cause: `NumOrd<Repr<B2>> for Repr<B1>` scales both significands by their full
`B^exponent` when the f32 log2 estimates do not separate them, even when `B1 == B2`.
Test: float/tests/num_order.rs::test_ord_same_base_large_exponent
Fix: `B1 == B2` delegates to `repr_cmp_same_base`. Not fixed: two *different* bases with huge,
nearly equal magnitudes still take the exact path (see "unsure" in the final report).
Committed together with the two previous clusters.

### log2_bounds invalid for exponents above 2^24
Reports: E255
Verdict: real, fixed
Root cause: `EstimatedLog2 for Repr` (float/src/log.rs) converted the exponent to f32 before
scaling it by log2(base); past 24 bits the conversion alone is off by whole units.
Test: float/tests/log.rs::test_log2_bounds_large_exponent
Fix: the sum is formed in f64, widened by an explicit bound on the f64 rounding, then narrowed
outwards to f32.

### num-traits Signed::abs_sub
Reports: E72
Verdict: real, fixed
Root cause: float/src/third_party/num_traits.rs implemented it as `|self - other|`.
Test: float/tests/num_traits.rs::test_abs_sub
Fix: positive difference (zero when `self <= other`), infinities handled without arithmetic.

### num-traits integer ToPrimitive panics on infinity
Reports: E242
Verdict: real, fixed
Root cause: `to_i8` etc. call `to_int()`, which panics on infinities (and would write out a
float with a huge exponent as an integer).
Test: float/tests/num_traits.rs::test_to_primitive_out_of_range
Fix: `None` for infinities and for magnitudes above 2^129, before `to_int`.
Committed together with the previous cluster.

### Uniform samples outside the range
Reports: D25, E85
Verdict: real, fixed
Root cause: `UniformFBig` (float/src/third_party/rand.rs) stored `high - low` rounded to the
operands' precision (`6 - (-9)` = `2e1` at one digit) and added the offset at a sampling
precision that could not represent `low`.
Test: float/tests/random.rs::test_uniform_stays_in_range
Fix: the width is exact, `low + unit·width` is evaluated exactly and rounded down once, and the
sampling precision is raised to the digit count of the bounds.

### Uniform over unlimited-precision bounds collapses
Reports: E80
Verdict: real, fixed
Root cause: precision 0 (unlimited, e.g. `ZERO..ONE`) was passed straight to `Uniform01`, i.e.
zero digits.
Test: float/tests/random.rs::test_uniform_unlimited_precision_bounds
Fix: an unlimited precision falls back to the inline precision of the builtin distributions.

### rand 0.9/0.10 Uniform::new on empty or reversed ranges
Reports: E176
Verdict: real, fixed
Root cause: float/src/third_party/rand_v09.rs / rand_v010.rs always returned `Ok`; the inherent
constructor asserted `low <= high` (so `new(x, x)` sampled the excluded bound, reversed panicked).
Test: float/tests/random.rs::test_uniform_errors_v09, ::test_uniform_errors_v010 (features
`rand_v09`, `rand_v010`), ::test_uniform_empty_range, ::test_uniform_reversed_range
Fix: `Err(EmptyRange)` / `Err(NonFinite)` in the 0.9/0.10 impls; `UniformFBig::new` now panics
with "empty range for random generation" on `low >= high` (rand 0.8 convention, same message as
dashu-int), `new_inclusive` on `low > high`. Behaviour change: `UniformFBig::new(x, x, p)`
used to succeed.

### Uniform01::new_open(0)
Reports: E200
Verdict: real, fixed
Root cause: no number with zero digits lies strictly inside (0, 1): base 2 rejected zeros
forever, other bases hit a debug assertion in dashu-int.
Test: float/tests/random.rs::test_uniform01_open_zero_precision_binary, ::..._decimal,
::test_uniform01_zero_precision (pins the other three constructors at precision 0)
Fix: `new_open` panics at construction with the usual "precision cannot be 0" message; documented.
The four rand clusters are one commit.

### from_parts_const precision
Reports: E128, E222
Verdict: real, fixed
Root cause: `normalize_word_const` (float/src/repr.rs) counted digits after stripping trailing
zeros (1230 → 3) and its running power overflowed `u128` one digit early (10^38 + 1 → 38).
Test: float/tests/io.rs::test_from_parts_const_matches_from_parts
Fix: digits are counted by division, on the significand as given.

### Repr::new_const exponent overflow
Reports: E235
Verdict: real, fixed (debug: panic; release: exponent wraps to isize::MIN — the fix is the same)
Root cause: same function, `exponent += shift` unchecked where `Repr::new` saturates.
Test: float/tests/io.rs::test_from_parts_const_matches_from_parts
Fix: `saturating_add`. Committed together with the previous cluster.

### error_bounds too wide in odd bases
Reports: E59
Verdict: real, fixed
Root cause: `HalfAway`/`HalfEven::error_bounds` (float/src/round.rs) used `ceil(B/2)` one digit
below the ulp as "half an ulp": 2/3 ulp in base 3, so the Ziv containment test certified
results up to 1/6 ulp past a midpoint.
Test: float/tests/round.rs::test_error_bounds_round_back (every point of the reported interval
must round back to the float; all six modes, bases 2/3/7/10/35),
float/tests/log.rs::test_ln_odd_base_nearest, rational/tests/dashu_float.rs::test_from_float_rounds_back
Fix: new `half_ulp`: exact in even bases; in odd bases the expansion `0.ddd…` cut off after
more than 128 bits, i.e. strictly inside half an ulp. `RBig::simplest_from_float` computes its
interval with exact arithmetic, since the bound can now be longer than the float.
Two further defects of the same function turned up through the round-back test and are fixed
in the same commit (not in any report of this group):
- `HalfEven` had the tie inclusion inverted (included for an odd last digit).
- At a power of the base the bound on the side facing zero used the ulp *above* the float; the
  floats below are `B` times denser. Affects `Away`, `Down`, `Up`, `HalfAway`, `HalfEven`.

### error_bounds panics on unlimited precision under Down/Up
Reports: E166
Verdict: real, fixed
Root cause: `mode::Down`/`mode::Up::error_bounds` called `f.ulp()` without the precision-0
case the other four modes have.
Test: float/tests/round.rs::test_error_bounds_unlimited_precision,
rational/tests/dashu_float.rs::test_from_float_unlimited_precision
Fix: `(ZERO, ZERO, true, true)` for precision 0. Committed together with the previous cluster.

## Per-report table

| Report | Cluster | Verdict |
|---|---|---|
| A13 | to_f32/to_f64 just below the smallest subnormal | real, fixed |
| D16 | to_f32/to_f64 just below the smallest subnormal | duplicate of A13 |
| E19 | to_f32/to_f64 just below the smallest subnormal | duplicate of A13 |
| E77 | to_f32/to_f64 between MAX and MAX + ½ulp | real, fixed |
| B2 | Negative zero in integer conversions and is_int | real, fixed |
| E87 | Negative zero in integer conversions and is_int | duplicate of B2 |
| E134 | Negative zero in integer conversions and is_int | real, fixed (same root cause, `is_int`) |
| E26 | with_base precision | real, fixed |
| E93 | with_base precision | real, fixed |
| D36 | with_base precision | duplicate of E93 |
| C3 | with_base precision (subnormals) + Zeros in base conversion (-0.0) | real, fixed |
| E38 | Zeros in base conversion | duplicate of C3 |
| E122 | Zeros in base conversion | real, fixed |
| E30 | with_base_and_precision to the same base does not round | real, fixed |
| D39 | Base conversion wrong for \|exponent\| > 38 | real, fixed |
| E22 | Base conversion wrong for \|exponent\| > 38 | duplicate of D39 |
| D31 | abs_cmp against UBig/IBig uses the signed order | real, fixed |
| E31 | abs_cmp against UBig/IBig uses the signed order | duplicate of D31 |
| D27 | NumOrd: zero ranks above small positive floats | real, fixed |
| E55 | NumOrd: zero ranks above small positive floats | duplicate of D27 |
| E175 | NumOrd against floats overflows for extreme exponents | real, fixed |
| E257 | Same-base NumOrd allocates the full power of the base | real, fixed |
| E255 | log2_bounds invalid for exponents above 2^24 | real, fixed |
| E72 | num-traits Signed::abs_sub | real, fixed |
| E242 | num-traits integer ToPrimitive panics on infinity | real, fixed |
| D25 | Uniform samples outside the range | real, fixed |
| E85 | Uniform samples outside the range | duplicate of D25 |
| E80 | Uniform over unlimited-precision bounds collapses | real, fixed |
| E176 | rand 0.9/0.10 Uniform::new on empty or reversed ranges | real, fixed |
| E200 | Uniform01::new_open(0) | real, fixed |
| E128 | from_parts_const precision | real, fixed |
| E222 | from_parts_const precision | real, fixed (same function, second defect) |
| E235 | Repr::new_const exponent overflow | real, fixed |
| E59 | error_bounds too wide in odd bases | real, fixed |
| E166 | error_bounds panics on unlimited precision under Down/Up | real, fixed |

## Changelog entries

`float/CHANGELOG.md`, under `## Unreleased`:

```markdown
### Fix
- `to_f32`/`to_f64` (on `FBig` and `Repr`) round correctly just below the smallest subnormal (values above half of it no longer become zero under round-to-nearest, and the exact half rounds to the subnormal under `HalfAway`) and just above `MAX` (values below `MAX + ½ulp` round to `MAX` instead of infinity under round-to-nearest).
- Negative zero converts to the integer 0: `FBig::to_int`/`Repr::to_int` return `Exact(0)`, the primitive integer `TryFrom` impls return `Ok(0)`, and `Repr::is_int` returns `true`.
- `FBig::with_base` computes the documented precision exactly (it was one digit short when `B^precision` is a power of the new base, e.g. 128 bits to base 16) and keeps at least one digit, so `with_base`, `to_decimal` and the num-traits `from_f32`/`from_f64` no longer panic for binary floats with 1 to 3 bits of precision.
- Base conversion (`with_base`, `with_base_and_precision`, `to_decimal`, `to_binary`) is correctly rounded, and reports exactness correctly, for every exponent; it used to return wrong digits once the exponent magnitude exceeded 38 (64-bit) at short target precisions.
- Base conversion maps zeros to zeros of the same sign in every base (negative zero used to panic or turn positive), rounds to the requested precision when the base is unchanged, and converts exactly between bases that are powers of a common root (e.g. 4 and 8).
- `AbsOrd::abs_cmp` between `FBig`/`Repr` and `UBig`/`IBig` compares magnitudes when either side is negative.
- `NumOrd` between `FBig`/`Repr` and `f32`/`f64`: a zero no longer ranks above positive floats below 1/2, and exponents beyond `isize::MAX / bit_len(base)` no longer overflow (panic in debug builds, wrong order in release builds).
- `NumOrd` between two floats of the same base no longer tries to allocate the full power of the base for large exponents.
- `EstimatedLog2::log2_bounds` for `FBig`/`Repr` stays valid for exponents beyond 2^24.
- num-traits: `Signed::abs_sub` returns the positive difference (zero when `self <= other`), and the integer `ToPrimitive` conversions return `None` for infinities instead of panicking.
- rand: `UniformFBig` (and rand's `Uniform`/`gen_range` for `FBig`/`DBig`) always samples inside the requested range; bounds of unlimited precision (such as `FBig::ZERO..FBig::ONE`) sample at the default precision of the builtin distributions instead of returning only the lower bound.
- rand 0.9/0.10: `Uniform::new`/`new_inclusive` for `FBig`/`DBig` return `Error::EmptyRange`/`Error::NonFinite` instead of accepting an empty range or panicking.
- `FBig::from_parts_const` infers the same precision as `from_parts` (trailing zero digits count, and significands at the top of the `DoubleWord` range get their last digit), and `Repr::new_const` saturates the exponent like `Repr::new`.
- `ErrorBounds::error_bounds`: the round-to-nearest bounds no longer exceed half an ulp in odd bases (which made transcendental functions misround there, e.g. `ln(1296)` in base 3), `mode::Down`/`mode::Up` return zero bounds for unlimited-precision values instead of panicking, `mode::HalfEven` includes the ties of floats with an even last digit (it was the other way round), and the bound facing zero is narrowed below powers of the base.

### Change
- `UniformFBig::new` panics for an empty range (`low >= high`; it used to accept `low == high` and return the excluded bound), `UniformFBig::new`/`new_inclusive` panic for infinite bounds, and the sampling precision is raised to the digit count of the bounds when the requested precision is smaller.
- `Uniform01::new_open` panics for a precision of 0 (it used to loop forever in base 2).
- `FBig::with_base`/`with_base_and_precision` on an unlimited-precision integer (non-negative exponent) convert losslessly to any base instead of panicking.
```

`rational/CHANGELOG.md`, under `## Unreleased`:

```markdown
### Fix
- `RBig::simplest_from_float` no longer panics for unlimited-precision floats under `mode::Down`/`mode::Up`, and returns a rational that rounds back to the float in odd bases.
```

## Cross-group notes

- Touched outside `float/`: `rational/src/third_party/dashu_float.rs` (`simplest_from_float`
  computes `f ∓ bound` with an unlimited-precision context instead of
  `with_precision(p + 1).unwrap()`, which would panic on the longer odd-base bounds) and
  `rational/tests/dashu_float.rs` (two tests). Part of the E59/E166 commit.
- `float/src/round.rs`/`ziv.rs` territory (float-arith?): the E59 commit changes
  `error_bounds` for every mode at powers of the base and the `HalfEven` tie inclusion, which
  the Ziv loops of `dashu-float` and `dashu-cmplx` consume. All workspace tests pass with it;
  the `fuzz/` differentials were not run.
- `float/src/log.rs` (`log2_bounds`), `float/src/cmp.rs`, `float/src/repr.rs`
  (`is_int`, `normalize_word_const`), `float/src/utils.rs` (`common_root`) and `float/Cargo.toml`
  (new `[[test]] num_traits`) are touched; other float groups may have edits nearby.
- The `to_f32`/`to_f64` reports (A13, D16, E19, E77) are **not** the `base/`
  `FloatEncoding::encode` root cause: the double rounding and the range check are in
  `float/src/convert.rs`. Nothing in `base/` was changed.
- Found, not fixed, not in this group: `RBig::to_float::<HalfAway, 10>(100)` of `355/113` (and
  `-355/113`) returns a significand ending in `…266` where the correctly rounded value ends in
  `…265` (the next digits are `486`; `Context::<HalfAway>::new(100).div` gets it right).
  Precisions 1, 2, 3, 20 and 50 are fine. `to_float` documents "guaranteed correct rounding".

## Reproduction

The new tests were run against an export of `a0fc535` with only the test files (and the
`[[test]] num_traits` stanza) overlaid; see the final report for the list of tests that fail
there.
