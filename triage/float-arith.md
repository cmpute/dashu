# float-arith

34 reports, 21 clusters. All line numbers refer to the triaged commit `a0fc535`.

How things were verified: every new test was run against the fixed tree (debug, `--release`
and `--all-features`), and the new test files were also run against an untouched export of
`a0fc535`. There, every test listed below as a regression fails (the two `*_huge_exponent_gap`
tests do not finish within 40 s; they take 0.03 s after the fix), and the tests that only pin
intended behaviour (`test_sub_keeps_one_guard_digit`, `test_rem_euclid_rounds_to_precision`,
`test_ulp_neighbours`, `test_ulp_value`) pass on both trees.

## Clusters

### Exact cancellation builds its zero at the operands' exponent
Reports: B1, D18, E88, E204
Verdict: real, fixed
Root cause: `cancel_zero` (`float/src/add.rs:20`) built the result of `x - x` with
`Repr::new(0, exponent)`. `Repr::normalize` keeps a zero significand at exponent `-1`,
`isize::MAX` or `isize::MIN` as the special value it encodes, so operands at exponent `-1`
(0.5, 0.3, 1.5…) cancelled to `-0`, and operands at exponent `isize::MAX` (E204:
`from_parts(2, isize::MAX - 1)` normalizes to `1·2^isize::MAX`) cancelled to an "infinity" that
`repr_round` then rejected with the infinity panic.
Test: `float/tests/signed_zero.rs::test_exact_cancellation_ignores_operand_exponent`
Fix: `cancel_zero` builds `+0` / `-0` explicitly (`float/src/add.rs`).
Notes: E204's value `1·2^isize::MAX` is still not `is_infinite()`; a nonzero significand at a
sentinel exponent is what the library itself produces as the "largest finite" /
"smallest" saturation endpoints, so arithmetic has to accept it, and now does. `Repr::new`
itself still turns `(0, -1)` into `-0` (see cross-group notes).

### `Repr` `+` / `-` zero signs
Reports: E196
Verdict: real, fixed
Root cause: `impl Add/Sub for &Repr` (`float/src/repr_ops.rs:52-90`, `110-140`) returned the
other operand for a zero operand (so `(+0) + (-0)` was `-0` but `(-0) + (+0)` was `+0`, and
`(-0) - (-0)` was `-0`), and built an exact cancellation through `Repr::new(0, exponent)` like
the cluster above.
Test: `float/tests/signed_zero.rs::test_repr_add_sub_zero_signs`
Fix: zero ± zero follows IEEE 754 §6.3 for round-to-nearest (only `(-0) + (-0)` and
`(-0) - (+0)` are `-0`); a cancellation is `+0` (`exact_sum_repr`). Exponent distances use
`abs_diff`.

### `signum` precision
Reports: D30
Verdict: real, fixed
Root cause: `FBig::signum` (`float/src/sign.rs:60`) built its result with `Context::new(1)`
although it documents returning `ONE` / `ZERO` / `NEG_ONE` (unlimited precision), so
`x * x.signum()` on an unlimited-precision `x` was rounded to one digit.
Test: `float/tests/sign.rs::test_signum_is_exact_constant` (new file)
Fix: `Context::new(0)`; docs state the precision. `CachedFBig::signum` delegates, so it follows.

### `/` with a dividend wider than the result precision
Reports: E40
Verdict: real, fixed
Root cause: the `Div` operators called `Context::repr_div` (`float/src/div.rs:175`), whose
contract (`div.rs:484-492`) bounds the dividend by `divisor digits + precision`. An
unlimited-precision dividend mixed with a limited divisor breaks the bound: a
`debug_assert!` in debug builds, an unrounded / wrongly rounded quotient in release builds.
Test: `float/tests/div.rs::triage::test_div_unlimited_dividend` (against the exact
`helpers::quotient_oracle`)
Fix: the operators call `repr_div_any_width`, which `Context::div` already used.

### Empty `Product` of `CachedFBig`
Reports: E54
Verdict: real, fixed
Root cause: both `Product` impls returned `Self::default()` (zero) for an empty iterator
(`float/src/fbig_cached_ops.rs:590`, `605`).
Test: `float/tests/parity.rs::sum_product_parity` (extended)
Fix: return `Self::from(FBig::ONE)`.

### Zero remainder sign
Reports: E90
Verdict: real, fixed
Root cause: `repr_rem` (`float/src/div.rs:890-896`) gave a zero remainder the sign of the
dividend only when the dividend was itself `-0`; an exact multiple (`-6 % 3`) came out `+0`.
Test: `float/tests/div.rs::triage::test_rem_zero_takes_dividend_sign`
Fix: the zero takes `lhs.sign()` (and `+0` for the Euclidean remainder).

### Remainder cost grows with the exponent gap
Reports: E149, E150
Verdict: real, fixed
Root cause: (E149) for base 2 the "dividend above divisor" branch of `repr_rem` computed
`(UBig::ONE << shift).into_ring(..)` (`float/src/div.rs:846-848`), materializing the aligned
dividend the surrounding comment says it avoids. (E150) `rem_euclid` / `div_rem_euclid`
(`div.rs:216-226`, `258-268`) went through `align_as_int`, which shifts one significand across
the whole exponent gap.
Test: `float/tests/div.rs::triage::test_rem_huge_exponent_gap`, plus the bounded proptest
`triage::prop_rem_matches_integer_arithmetic` (`%`, `rem_euclid`, `div_euclid`,
`div_rem_euclid` against exact integer arithmetic, all six modes, bases 2/3/10, precisions
0/20/50/100/500)
Fix: one kernel `repr_rem_with` serves `%` and the Euclidean remainder. Base 2 uses the modular
power beyond a few words of shift. The "divisor above dividend" branch no longer builds
`|rhs|·B^gap` when the dividend lies below the divisor's lowest digit: the dividend is its own
truncated remainder, and the stepped remainder `lhs ∓ rhs` is rounded through a new sticky
rounding helper (`Context::sticky_threshold` / `repr_round_sticky` / `repr_round_sum_exact` in
`float/src/add.rs`). `div_euclid` returns `0` / `∓1` directly in that situation.
Notes: the same blow-up existed in the direction the reports did not exercise (small dividend,
huge divisor exponent); it is fixed too. `div_euclid`'s quotient is inherently as wide as the
gap when the dividend is the large operand. `rem_euclid` / `div_euclid` now panic on an
infinite operand like `%` does (they used to treat the infinity as zero).

### `rem_euclid` can return `|rhs|`
Reports: E58
Verdict: not a bug
Root cause: none — the exact Euclidean remainder of `-1` by `1e2` is `99`, and both operands
have precision 1, so the result is rounded to one digit (`1e2` under `HalfAway`, `9e1` under
`Zero`). This is the documented rule for `%` ("the mode also rounds the remainder value itself
down to the context precision") and the same limitation Rust documents for `f64::rem_euclid`.
With the divisor at precision 3 the result is `99`.
Test: `float/tests/div.rs::triage::test_rem_euclid_rounds_to_precision` (pins the behaviour)
Fix: documentation only — the `RemEuclid` impl now states the rounding and its consequence.

### Round-to-integer operations drop the sign of a zero result
Reports: E197, E168, E206
Verdict: real, fixed
Root cause: `ceil` returned `Self::ZERO` / `Repr::new(hi + rounding, 0)` for a negative value
above `-1` (`float/src/round_ops.rs:186-195`); `split_at_point` returned `Self::ZERO` for the
zero part and built the integral part with `Repr::new(hi, 0)` (`round_ops.rs:108-121`), while
`trunc` / `fract` / `round` use `sign_kept_int`.
Test: `float/tests/round.rs::triage::test_round_to_integer_keeps_sign_of_zero`,
`triage::test_split_at_point_matches_trunc_fract`
Fix: `ceil` and `split_at_point` use `sign_kept_int`; `split_at_point` is now literally
`(trunc(), fract())`, precisions included.

### `split_at_point` on `+inf`
Reports: E212
Verdict: real, fixed
Root cause: no finiteness check; `+inf` has exponent `isize::MAX >= 0` and took the "integer"
shortcut (`float/src/round_ops.rs:108`), while `-inf` happened to panic further down.
Test: `float/tests/round.rs::triage::test_split_at_point_matches_trunc_fract`
Fix: `assert_finite` first (documented under `# Panics`).

### `fract` changes the precision of values below one
Reports: E187
Verdict: real, fixed
Root cause: the "below one" shortcut of `fract` / `trunc` relies on `Repr::smaller_than_one`,
a cheap estimate with false negatives for values in roughly `[B^-2, 1)`; those fell through to
the split path, which sets the precision to the number of fractional digits
(`float/src/round_ops.rs:149-155`).
Test: `float/tests/round.rs::triage::test_fract_below_one_is_identity`
Fix: after the split, a zero integral part takes the same result as the shortcut (in `fract`,
`trunc` and `split_at_point`). The docs of `fract` spell out the precision rule.

### `quantize`: sign of a zero result, extreme quantum exponents
Reports: E60, E193
Verdict: real, fixed
Root cause: (E60) the rounded result was built with `Repr::new(hi + adjust, exp)`
(`float/src/round_ops.rs:362`), which is `+0` for a zero significand — or `-0` / an infinity
when `exp` happens to be `-1` / `isize::MAX`. (E193) the precision and shift were computed in
`isize` (`round_ops.rs:352`, `359`), overflowing for a quantum near `isize::MIN`.
Test: `float/tests/round.rs::triage::test_quantize_zero_result_sign`,
`triage::test_quantize_extreme_exponents`
Fix: a zero result is `Repr::zero_with_sign(self.sign())`; distances use `abs_diff` with a
saturating precision; a quantum far above the value no longer splits by `BASE^shift` (which
would have materialized `5^shift` in base 10).
Notes: E193 is debug-only (release wrapped to the right value).

### `fma` with a zero product
Reports: E98
Verdict: real, fixed
Root cause: `Context::fma` returned `c` rounded whenever `a·b` was a zero
(`float/src/mul.rs:615-617`), ignoring the product's sign and the rounding mode.
Test: `float/tests/mul.rs::triage::test_fma_zero_product_sign` (against `add`/`sub` of the
signed zero product, all zero-sign combinations)
Fix: zero product + zero `c` goes through `zero_sum` (now `pub(crate)`); a zero `c` with a
nonzero product is `Context::mul` on the signed operands.

### Exact product tagged `Inexact(NoOp)`
Reports: E115
Verdict: real, fixed
Root cause: the short-product classifier `round_high_product` (`float/src/mul.rs:83-146`)
always returned `Inexact`, also for an exact window with zero discarded digits.
Test: `float/tests/mul.rs::triage::test_mul_exact_product_of_wide_operands_is_exact`
Fix: return `Exact` in that case.

### Rounding flag after a partial cancellation
Reports: E136
Verdict: real, fixed
Root cause: `repr_round_sum` (`float/src/add.rs:253`) reported the adjustment `round_fract`
returns relative to the intermediate significand. After an effective subtraction the low part
has the opposite sign, so that significand is not the truncation of the exact value: `Away`
reported `NoOp` for a result it rounded away from zero, and `Zero` reported `SubOne`.
Test: `float/tests/add.rs::triage::test_add_sub_rounding_flag_direction` (the flag must match
the direction of the result relative to the exact sum, all modes)
Fix: the reported flag is shifted by the unit that truncation toward zero removes; the value is
unchanged.
Notes: this changes flags that existing tests pinned. Eleven expectations in
`float/tests/add.rs` (`test_add_binary`, `test_add_decimal`, `test_sub_binary`,
`test_sub_decimal`) were updated, e.g. `0x100 + (-0x1p-10)` under `Zero` is now `NoOp` (was
`SubOne`) and `100 - 1e-10` under `HalfAway` is now `AddOne` (was `NoOp`). The new values are
the ones `with_precision` / `Context::mul` report for the same exact value, and the ones the
table in `round::mode` allows (`Zero` only ever lists `NoOp`). This is a judgement call worth a
maintainer's eye.

### Exponent range judged before rounding
Reports: E158, E237
Verdict: real, fixed
Root cause: `Context::mul` / `sqr` / `cubic` checked the exact product's exponent before
rounding (`float/src/mul.rs:442-452`, `497-498`, `549-550`) and the division kernel computed
`lhs.exponent - rhs.exponent` with `checked_sub` (`float/src/div.rs:677-689`). A result whose
unrounded exponent leaves the range while the rounded, normalized one does not was reported as
`Underflow` / `Overflow` — depending, for products, on whether the wide short-product path or
the exact path ran.
Test: `float/tests/mul.rs::triage::test_mul_exponent_range_is_checked_after_rounding`,
`float/tests/div.rs::triage_range::test_div_exponent_range_is_checked_after_rounding`
Fix: new `Repr::new_wide` / `Context::repr_round_wide` (`float/src/repr.rs`) carry the exponent
as `i128` and check the range on the rounded, normalized result; the division kernel tracks its
exponent the same way.
Notes: two existing tests asserted the old errors for results that are representable and were
rewritten: `float/tests/high.rs::test_mul_exponent_overflow` (a product of about
`2^(isize::MIN + 8187)` was expected to be `Underflow`, and `*` returned 0 for it) and
`test_div_exponent_overflow` (a quotient with a 2400-bit significand at exponent
`isize::MAX - 2393` was expected to be `Overflow`). Both now check the value against the same
operation at ordinary exponents, plus a genuinely out-of-range case. E158 asked for "all `Ok` or
all `Err`"; it is all `Ok`.

### `+` / `-` leave an overflowed exponent
Reports: E146
Verdict: real, fixed
Root cause: the `Add` / `Sub` operators wrapped the kernel result directly
(`float/src/add.rs:56` and the seven siblings) instead of going through
`finish_rounded` + `unwrap_fp` as `Context::add` callers and `*` do.
Test: `float/tests/add.rs::triage::test_add_sub_exponent_overflow_saturates`
Fix: the eight operator impls call `context.unwrap_fp(context.finish_rounded(sum))`.
Notes: under the inward modes the overflow endpoint is the largest finite value
`(B^p − 1)·B^isize::MAX`, which again has a nonzero significand at the sentinel exponent — by
design of `unwrap_fp`. The reported `HalfEven` case now gives `±inf`.

### Exponent gap beyond `isize::MAX` in add/sub
Reports: E205
Verdict: real, fixed
Root cause: `(lhs.exponent - rhs.exponent) as usize` in `repr_add_large_small` /
`repr_add_small_large` (`float/src/add.rs:271`, `389`).
Test: `float/tests/add.rs::triage::test_add_sub_exponent_gap_beyond_isize`
Fix: `abs_diff`, with saturating digit-position bookkeeping.
Notes: debug-only (release wrapped to the right value).

### Shifts to a sentinel exponent
Reports: E207
Verdict: real, fixed
Root cause: `<<` / `>>` added the amount to the exponent unchecked (`float/src/shift.rs:10`),
so the result could land on a sentinel exponent (and `x - x` on it panicked, see the first
cluster) or overflow `isize`.
Test: `float/tests/shift.rs::test_shift_exponent_saturation`
Fix: a shift is treated as the exact multiplication by `B^n` it is: an exponent at or beyond
the sentinels overflows / underflows through `unwrap_fp`.
Notes: the infinite overflow endpoint no longer asserts a limited precision
(`overflow_repr_endpoint`, `float/src/exp.rs`), so `DBig::ONE << isize::MAX` is `inf`. Under an
inward mode at unlimited precision an overflow still panics ("precision cannot be 0"), as `*`
already did — there is no largest finite value to return.

### Directed underflow endpoint below the exact value
Reports: E145
Verdict: real, not fixed
Root cause: a nonzero result at exponent `isize::MIN` is reported as `Underflow`
(`Repr::check_finite_exponent`, `float/src/repr.rs:396`), and the outward endpoint
`underflow_repr_endpoint` (`float/src/exp.rs:298`) is `1·B^isize::MIN`. Results such as
`3·2^isize::MIN` are larger than that, so `mode::Up` returns less than the exact value. The
same holds for any result in `(B^isize::MIN, B^(isize::MIN + p))`.
Test: `float/tests/mul.rs::triage::test_directed_rounding_at_minimum_exponent`
(`#[ignore = "E145: needs a redesign of the exponent-range / underflow-endpoint model"]`)
Fix: none. A sound fix needs either gradual underflow (re-rounding to fewer digits at the
lowest exponent) or making `isize::MIN` an ordinary exponent, in every kernel that uses
saturating exponent arithmetic as its overflow signal, plus new endpoints — a design change
across the crate (and across the float-exp group's reports E241 / E258).
Notes: the inputs are at the very edge of the exponent range.

### Guard digit after a subtraction
Reports: E173
Verdict: not a bug
Root cause: none — an effective subtraction is rounded to `precision + 1` digits on purpose
(`rnd_precision` in `float/src/add.rs`; AGENTS.md, the `Repr` docs, and the existing
`test_sub_decimal` "the output significand can have one more digit than the precision").
`1 - 20` at precision 1 is therefore the exact two-digit `-19`.
Test: `float/tests/add.rs::triage::test_sub_keeps_one_guard_digit` (pins the reported inputs)
Fix: documentation only — the `Repr` docs said "inexact addition or subtraction"; they now say
which operations keep the guard digit and that the result is `Exact` when `p + 1` digits hold
it.

### `Sum`: sign of an all-`+0` sum under `Down`
Reports: E182
Verdict: real, fixed
Root cause: `precise_sum` (`float/src/iter.rs:45-51`) made every zero sum `-0` under
roundTowardNegative, including a sum of `+0` addends.
Test: `float/tests/iter.rs::triage::test_sum_zero_sign_matches_chained_add`
Fix: `-0` when all addends are `-0`, or under roundTowardNegative unless all addends are `+0`.

### `Sum` cost grows with the exponent gap
Reports: E169
Verdict: real, fixed
Root cause: `precise_sum` accumulated all addends exactly (`float/src/iter.rs:38`), so the
accumulator spanned the full exponent range of the inputs.
Test: `float/tests/iter.rs::triage::test_sum_huge_exponent_gap`, plus the bounded proptest
`triage::prop_sum_matches_exact_fold` (sum against the unlimited-precision fold rounded once,
with cancelling terms, six modes, bases 2/3/10, precisions 20/50/100/500)
Fix: addends are sorted by magnitude and accumulated exactly from the top; once the rest lies
below the partial sum's sticky threshold it only contributes the sign of its exact sum
(`tail_sign`), through the same sticky rounding helper as the remainder. An unlimited-precision
sum still materializes the span, as its result must.

### `CachedFBig::ulp` reimplements `FBig::ulp`
Reports: E179
Verdict: real, fixed
Root cause: `CachedFBig::ulp` (`float/src/fbig_cached.rs:198-207`) recomputed the exponent with
unchecked arithmetic and without the infinity shortcut of `FBig::ulp`.
Test: `float/tests/ulp.rs::test_cached_ulp_matches_fbig` (new file)
Fix: delegate to `FBig::ulp`, like `ulp_lb` already did.
Notes: the infinity panic is in both profiles; the overflow panics are debug-only (release
returned a wrapped exponent).

### `ulp` of `-0`
Reports: E253
Verdict: real, fixed
Root cause: `ulp` / `ulp_lb` used the stored exponent (`float/src/fbig.rs:319-327`, `358-366`),
which for `-0` is the sentinel `-1`.
Test: `float/tests/ulp.rs::test_ulp_of_signed_zeros`
Fix: a zero counts as exponent 0 whatever its sign.

### `ulp` documentation
Reports: E248
Verdict: docs
Root cause: the docs claimed `x + ulp` is always the first float greater than `x`; toward zero
from a power of the base the spacing is `ulp / B`.
Test: `float/tests/ulp.rs::test_ulp_neighbours`, `test_ulp_value` (pin the documented meaning)
Fix: the `FBig::ulp` docs describe the value (unit in the last place) and when `x ± ulp` is the
neighbour.

## Per-report table

| Report | Cluster | Verdict |
|---|---|---|
| B1 | Exact cancellation builds its zero at the operands' exponent | real, fixed |
| D18 | Exact cancellation builds its zero at the operands' exponent | duplicate of B1 (real, fixed) |
| D30 | `signum` precision | real, fixed |
| E40 | `/` with a dividend wider than the result precision | real, fixed |
| E54 | Empty `Product` of `CachedFBig` | real, fixed |
| E58 | `rem_euclid` can return `\|rhs\|` | not a bug |
| E60 | `quantize`: sign of a zero result, extreme quantum exponents | real, fixed |
| E88 | Exact cancellation builds its zero at the operands' exponent | duplicate of B1 (real, fixed) |
| E90 | Zero remainder sign | real, fixed |
| E98 | `fma` with a zero product | real, fixed |
| E115 | Exact product tagged `Inexact(NoOp)` | real, fixed |
| E136 | Rounding flag after a partial cancellation | real, fixed |
| E145 | Directed underflow endpoint below the exact value | real, not fixed |
| E146 | `+` / `-` leave an overflowed exponent | real, fixed |
| E149 | Remainder cost grows with the exponent gap | real, fixed |
| E150 | Remainder cost grows with the exponent gap | real, fixed |
| E158 | Exponent range judged before rounding | real, fixed |
| E168 | Round-to-integer operations drop the sign of a zero result | real, fixed |
| E169 | `Sum` cost grows with the exponent gap | real, fixed |
| E173 | Guard digit after a subtraction | not a bug |
| E179 | `CachedFBig::ulp` reimplements `FBig::ulp` | real, fixed |
| E182 | `Sum`: sign of an all-`+0` sum under `Down` | real, fixed |
| E187 | `fract` changes the precision of values below one | real, fixed |
| E193 | `quantize`: sign of a zero result, extreme quantum exponents | real, fixed |
| E196 | `Repr` `+` / `-` zero signs | real, fixed |
| E197 | Round-to-integer operations drop the sign of a zero result | real, fixed |
| E204 | Exact cancellation builds its zero at the operands' exponent | real, fixed (same root cause as B1) |
| E205 | Exponent gap beyond `isize::MAX` in add/sub | real, fixed |
| E206 | Round-to-integer operations drop the sign of a zero result | real, fixed |
| E207 | Shifts to a sentinel exponent | real, fixed |
| E212 | `split_at_point` on `+inf` | real, fixed |
| E237 | Exponent range judged before rounding | real, fixed |
| E248 | `ulp` documentation | docs |
| E253 | `ulp` of `-0` | real, fixed |

## Changelog entries

`float/CHANGELOG.md`, under `## Unreleased`:

```markdown
### Fix
- Fix `x - x` and `x + (-x)` returning `-0` (outside `mode::Down`) when the operands have exponent `-1` (e.g. `0.5 - 0.5`, `0.3 - 0.3`), and panicking when the operands sit at exponent `isize::MAX`.
- Fix the exact `Repr` `+` / `-` operators: a sum of zeros is `-0` only for `(-0) + (-0)` / `(-0) - (+0)` regardless of operand order, and an exact cancellation is always `+0`.
- Fix `FBig::signum` returning a precision-1 value instead of the exact `ONE` / `ZERO` / `NEG_ONE`, which rounded `x * x.signum()` to one digit for unlimited-precision `x`.
- Fix `FBig / FBig` with an unlimited-precision (or otherwise over-wide) dividend: it failed a debug assertion in debug builds and returned an unrounded or wrongly rounded quotient in release builds.
- Fix `Product` of an empty iterator of `CachedFBig` returning `0` instead of `1`.
- Fix `%` / `Context::rem` returning `+0` instead of `-0` when a negative dividend is an exact multiple of the divisor.
- Fix `%`, `rem_euclid` and `div_rem_euclid` taking time and memory proportional to the exponent gap between the operands (base-2 `%` ran out of memory for a large gap); `rem_euclid` / `div_euclid` now panic on an infinite operand like `%`.
- Fix `ceil` and `split_at_point` returning `+0` instead of `-0` for the zero integral / fractional part of a negative number; `split_at_point` now matches `(trunc(), fract())` exactly and panics on `+inf` as it did on `-inf`.
- Fix `fract()` (and the precision of `trunc()`) for values below one that the magnitude estimate did not recognize: `fract()` reset the precision of values in `[0.01, 1)` to their number of fractional digits.
- Fix `quantize` returning `+0` for a negative value that rounds to zero, and overflowing (debug builds) for a quantum exponent near `isize::MIN`.
- Fix `Context::fma` ignoring the sign of a zero product: `fma(+0, 1, -0)` is now `+0` (outside `mode::Down`), matching `add` / `sub`.
- Fix `Context::mul` tagging an exactly representable product of wide operands as `Inexact(_, NoOp)`.
- Fix the `Rounding` flag of an inexact `add` / `sub` whose operands partly cancel: it is now relative to the truncated exact value like every other operation (e.g. `mode::Away` reported `NoOp` for `4 - 1` rounded to `4`).
- Fix `Context::mul` / `sqr` / `cubic` / `div` (and the operators) reporting `Underflow` / `Overflow` from the operand exponents when the rounded result is representable, e.g. `2 / (3·2^(isize::MIN+1))`, or narrow operands whose exponents sum to `isize::MIN`.
- Fix `FBig + FBig` / `FBig - FBig` returning a nonzero significand at the sentinel exponent instead of saturating like the other operators when the sum overflows the exponent range.
- Fix an overflow panic (debug builds) in `add` / `sub` when the operand exponents are more than `isize::MAX` apart.
- Fix `<<` / `>>` moving a value onto (or past) the sentinel exponents: an out-of-range shift now overflows / underflows like a multiplication by a power of the base.
- Fix `Sum` returning `-0` for a sum of `+0` addends under `mode::Down`.
- Fix `Sum` taking time and memory proportional to the exponent gaps between the addends at limited precision.
- Fix `CachedFBig::ulp` panicking on infinities and (debug builds) on exponents near the ends of the `isize` range; it now delegates to `FBig::ulp`.
- Fix `ulp()` / `ulp_lb()` (and the error bounds) of `-0` being one digit smaller than for `+0`.

### Change
- An overflow that saturates to `±inf` (nearest and outward modes) no longer panics at unlimited precision; only the inward modes, which need the largest finite value, still require a limited precision.
- Document that `rem_euclid` rounds its result to the operands' precision (and can therefore return `|rhs|`), when a subtraction result carries the guard digit, the precision rule of `fract()`, and what `ulp()` means next to a power of the base.
```

## Cross-group notes

- `float/src/repr.rs`: added `Repr::new_wide` and `Context::repr_round_wide` (wide-exponent
  constructors used by mul/div), and reworded the guard-digit paragraph of the `Repr` docs.
  `Repr::new` / `normalize` are untouched: `Repr::new(0, -1)` is still `-0` and
  `Repr::new(0, isize::MAX)` still `+inf`. That is the shared root of this group's first two
  clusters and of float-fmt's D14 (`"0e-1"` parsing as `-0`). If another group makes `Repr::new`
  canonicalize a zero significand to `+0`, the explicit zero constructions added here
  (`cancel_zero`, `exact_sum_repr`, `quantize`) become redundant but stay correct.
- `float/src/exp.rs`: `overflow_repr_endpoint` only asserts a limited precision in the branch
  that needs the largest finite value (float-exp's area; three lines).
- `float/src/fbig_cached.rs`, `float/src/fbig_cached_ops.rs`: `CachedFBig::ulp` delegation and
  the empty-`Product` identity. No API-surface change; the cached wrappers of `rem_euclid`,
  `signum`, `split_at_point`, shifts etc. delegate and inherit the fixes.
- `float/src/add.rs` gained three `pub(crate)` helpers (`sticky_threshold`,
  `repr_round_sticky`, `repr_round_sum_exact`) shared by the remainder and `Sum`.
- Existing tests whose expectations changed: `float/tests/add.rs` (rounding flags, see E136)
  and `float/tests/high.rs` (`test_mul_exponent_overflow`, `test_div_exponent_overflow`, see
  E158 / E237).
- `float/tests/div.rs` now imports `tests/helpers.rs` (`#[allow(dead_code)] mod helpers;`) for
  the exact quotient oracle.
- E204 (`from_parts(2, isize::MAX - 1)` normalizing onto the sentinel exponent) and
  float-convert's E235 (`Repr::new_const` vs `Repr::new` saturation) concern the same
  constructors; only the arithmetic panic was fixed here.
- E145 is tied to the underflow endpoint that float-exp's E241 / E258 also touch.
- Not run: the `fuzz/` differentials (MPFR/MPC), the Dylint house lints, and an MSRV (1.68)
  build — no such toolchain is installed; the new code uses `abs_diff`, `unsigned_abs`,
  `usize::BITS` and `i128` arithmetic only.
