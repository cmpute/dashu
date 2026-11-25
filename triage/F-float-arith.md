# F-float-arith

10 reports of run F (against `8137b6e`), 10 clusters: 9 real and fixed, 1 not a bug.
File and line references are to `8137b6e` unless they say "base" (`a0fc535`).

How things were verified:

- Every report's reproducer was built and run against the base checkout (`a0fc535`, debug;
  release too for F13, F36, F75, F88) and against this tree before and after the fix.
- The new tests were also run against an export of `master` (`b0705ed`, the tree the reports
  were filed against plus the notes): there, exactly the tests named below as regressions
  fail, `test_quantize_zero_result_sign` fails on the one expectation changed for F13, and
  the pin `test_guard_digit_is_rounded_by_the_next_operation` passes.
  `test_rem_zero_dividend_huge_divisor_exponent` does not finish there: it was killed at
  25 GB of memory (it takes milliseconds after the fix).
- After the fixes: `cargo test -p dashu-float --all-features` (debug), the affected test files
  in `--release`, `cargo test --workspace --exclude dashu-python`,
  `cargo test -p dashu-cmplx -p dashu-ratio -p dashu-macros -p dashu --all-features` (after
  the `Repr::new` change), `cargo fmt --all -- --check`,
  `cargo clippy --all-features --all-targets --workspace --exclude dashu-python -- -D warnings`,
  `cargo check -p dashu-float --no-default-features` and `cargo doc -p dashu-float` are clean.

## Clusters

### `Repr::new` / `from_parts` read a zero significand as a special value
Reports: F9
Verdict: real, fixed
Root cause: `Repr::new` (`float/src/repr.rs:613`) only calls `normalize`, which deliberately
keeps a zero significand at exponent `-1`, `isize::MAX` or `isize::MIN` as the special value
that pair encodes. So `Repr::new(0, -1)` / `FBig::from_parts(0, -1)` is `-0` (reciprocal
`-inf`) and `from_parts(0, isize::MAX)` is `+inf`, while `new_const` / `from_parts_const`
give `+0`.
Base behaviour: identical output at `a0fc535` (run).
Provenance: **incomplete-fix** of B1 / D18 / E88 / E204 ("Exact cancellation builds its zero
at the operands' exponent", commit `6af48e7`) and of float-fmt's A3 / D14 / E10. Those fixed
the callers (`cancel_zero`, `exact_sum_repr`, `quantize`, the parser) and the earlier notes
record that `Repr::new` itself was left alone; the report is the pipeline re-filing B1 against
the constructor. The defect is base code.
Test: `float/tests/convert.rs::test_from_parts_zero_significand`
Fix: `Repr::new` returns `+0` for a zero significand whatever the exponent
(`float/src/repr.rs`); `FBig::from_parts` / `CachedFBig::from_parts` inherit it. The
deserializers, which do need to rebuild a special value from its encoded pair, use the new
crate-private `Repr::from_encoded_parts` (`float/src/third_party/serde.rs`, four call
sites). Docs of `Repr::new`, `Repr::into_parts` and `FBig::from_parts` say so.
Notes: this is a public behaviour change: `Repr::new(sig, exp)` no longer rebuilds `-0` /
`±inf` from `into_parts()` output; those have their own constructors. Nothing in the
workspace relied on it except serde (the rkyv derives write the fields directly; the Python
bindings pass user input straight to `from_parts`). `from_parts(0, e)` has precision 1 and
`from_parts_const(+, 0, e, None)` precision 0, as before (not reported, left alone).

### `NumHash` negates an `isize::MIN` exponent
Reports: F36
Verdict: real, fixed
Root cause: `Repr::num_hash_residue` computed `-self.exponent as u128`
(`float/src/third_party/num_order.rs:338`), which overflows for `isize::MIN`; in base 2 the
`absm` helper of the modular-arithmetic dependency negates the same value. A nonzero
significand at exponent `isize::MIN` is what the outward underflow endpoint is, so it is a
value the library produces.
Base behaviour: identical (debug: panic at `num_order.rs:317`; release: a hash), run for
base 10.
Provenance: **pre-existing**.
Test: `float/tests/num_order.rs::test_hash` (new block at the end: the hash must not panic and
must agree with the same value written in base 100 / base 4)
Fix: `unsigned_abs` for the exponent, and `rem_euclid(127)` in place of `absm` for base 2.
Notes: the panic is debug-only, as reported. In release the base-10 hash was computed from
the wrapped exponent: the reproducer prints `0x4b5476b5c631ff3f` on the unfixed tree and
`0x01a3ad50695305b2` after the fix, and only the latter agrees with the same value written
in base 100 — so release builds returned a hash that broke the `NumHash` contract. Not the
same defect as F9, although the report cross-references it (`from_parts` accepts the
exponent in both).

### `quantize` carries onto the sentinel exponent
Reports: F13
Verdict: real, fixed
Root cause: `quantize` built its result with `Repr::new(signif, exp)`
(`float/src/round_ops.rs:409`), whose normalization saturates at `isize::MAX`: a carry to
`10 · B^isize::MAX` came back as `1 · B^isize::MAX` (a factor `B` too small), and a carry to
`B^isize::MAX` as a nonzero significand at the sentinel exponent, without any range check.
Base behaviour: release build: the same two wrong values (run). Debug build: panics with
"attempt to add with overflow" at base `float/src/round_ops.rs:368`, in the precision
arithmetic `repr.exponent + digits - exp` (run).
Provenance: **pre-existing** in release builds; in debug builds **masked** by that overflow
panic, which the E193 fix (`a0bef9f`, unsigned exponent distances) removed.
Test: `float/tests/round.rs::triage::test_quantize_overflow_saturates`
Fix: the result goes through `Repr::new_wide`; an exponent at or beyond `isize::MAX` is an
overflow resolved by `overflow_repr_endpoint` at the result's precision, i.e. exactly what
`<<` and `*` do (`±inf` for the nearest and outward modes, the largest finite value for the
inward ones).
Notes: one existing expectation changed: `triage::test_quantize_zero_result_sign` pinned
`tiny.quantize(isize::MAX)` under `mode::Away` to `1 · B^isize::MAX`; it is now the infinity.
Under an inward mode a multi-digit multiple at exponent `isize::MAX` (e.g. `123e(MAX-1)`
quantized to `12 · 10^MAX` under `Zero`) becomes the endpoint `99 · 10^MAX`, which is above
the exact value. That is the same hole at the top of the exponent range that `+` has (and
E145 at the bottom): the library treats every nonzero significand at exponent `isize::MAX`
as "overflowed".

### `fma` reports the range error of the product
Reports: F16
Verdict: real, fixed
Root cause: `Context::fma` formed `a·b` with the saturating `Repr` product and returned its
`Underflow` / `Overflow` before looking at `c` (`float/src/mul.rs:618-625`). A product below
the exponent range next to an ordinary `c` only decides the rounding direction; `FBig::fma`
turned the error into `0` for `3 + 2^(isize::MIN - 2)`.
Base behaviour: identical (run).
Provenance: **pre-existing** (the E98 / E158 fixes in `3bfafe1` touched `fma`'s zero-product
branch and the product range of `mul`, not this).
Test: `float/tests/mul.rs::triage::test_fma_product_outside_exponent_range` (against
`add` / `sub` of the smallest value for a distant `c`, and against the same operation with
all exponents shifted into range for a `c` near the product; six modes, bases 2 / 3 / 10,
precisions 20 / 50 / 100 / 500)
Fix: the product keeps an `i128` exponent. When it is out of range, `fma_wide`
(`float/src/mul.rs`) forms the sum in a shifted exponent frame — replacing an operand that
lies more than `isize::MAX / 2` digits below the other by one unit of its sign — and checks
the range on the rounded result. The in-range path is unchanged.
Notes: this also fixes the mirror case the report did not exercise: a product above the range
that `c` cancels (`B^MAX - (B-1)·B^(MAX-1)`) was `Overflow`. At unlimited precision the exact
sum is as wide as the distance between `c` and the product, so a distant out-of-range product
still returns its range error (documented); the report's own case at precision 0 is
`Err(Underflow)`. `complex`'s `fma` has its own code and was not examined.

### Integer operands round an unlimited-precision float
Reports: F41
Verdict: real, fixed
Root cause: the `FBig op integer` operators convert the integer with `FBig::from`
(`float/src/helper_macros.rs`), whose precision is the integer's digit count, and
`Context::max` lets "unlimited" (0) yield to any limited precision. So next to an
unlimited-precision float the integer's digit count became the result precision:
`12.34 + 1u8 = 10`, `x + 0u8 != x`, `12.34 * 3u8 = 40`.
Base behaviour: identical for `+`, `-`, `*` and their assign / reversed forms (run). `x / 4u8`
with an unlimited `x` panicked at base (see notes).
Provenance: **pre-existing**.
Test: `float/tests/add.rs::triage::test_integer_operand_keeps_unlimited_precision`
(primitives, `UBig`, `IBig`; `+ - *`, reversed and assign forms; `CachedFBig`; and that the
limited-precision results are unchanged at 20 / 50 / 100 / 500 digits)
Fix: `FBig::integer_operand` (`float/src/fbig.rs`, crate-private) makes the converted integer
unlimited when the float operand is; the three macros in `helper_macros.rs` and their
`CachedFBig` mirrors in `fbig_cached_ops.rs` use it.
Notes: only the unlimited case changes. With a limited-precision float the result precision is
still `max(precision, digits of the integer)` as before (`1.2 + 123456u32` has precision 6).
`FBig(prec 0) + FBig(prec 1)` between two floats is unchanged too: that rule is how exact
constants adopt the other operand's precision. Division shares the macros:
`unlimited / 4u8` used to round to one digit (`12.34 / 4u8 = 3`, since `9c73e01`; a panic at
base) and now panics with "precision cannot be 0", exactly like
`unlimited / FBig(4, prec 0)` — a quotient has no unlimited-precision form. That is a
behaviour change worth a maintainer's eye.

### Guard digit rounded by an identity operation
Reports: F42
Verdict: not a bug (one doc sentence corrected)
Root cause: none. `101 - 0.2` at precision 3 is `100.8` with the intentional add/sub guard
digit (E173, `AGENTS.md`, the user guide: "before the next operation rounds it back").
`x + 0`, `x - 0`, a one-element `Sum` and `Product` are operations and round it to `101`.
Base behaviour: identical (run).
Provenance: **pre-existing** behaviour, by design.
Test: `float/tests/add.rs::triage::test_guard_digit_is_rounded_by_the_next_operation` (pins the
reported inputs, and that a value that fits its precision is left alone)
Fix: documentation only. The `Repr` docs (`float/src/repr.rs`, sentence added by `3894c78`
for E173) said `with_precision` rounds the guard digit away; it only does for a smaller
precision (`x.with_precision(3)` is `Exact(x)`), which the test also pins.

### `Context::rem` panics on a zero divisor
Reports: F47
Verdict: real, fixed
Root cause: `Context::rem` (`float/src/div.rs:1061`) only checked for infinities; a zero
divisor reached the integer division in the kernel, which panics ("divisor must not be 0"),
although the function returns an `FpResult`.
Base behaviour: identical (run).
Provenance: **pre-existing**.
Test: `float/tests/div.rs::triage::test_rem_by_zero_is_an_error`
Fix: `Context::rem` returns `Err(FpError::Indeterminate)` for a zero divisor (documented, also
on the variant). The `%`, `rem_euclid` and `div_euclid` operators have no error channel and
still panic with the same message, now from an explicit check in `repr_rem_with` /
`div_euclid` (`error::panic_divide_by_0`), and the `%` docs list it under `# Panics`.
Notes: `Indeterminate` was chosen over a new variant: it is the one that maps to "the result
is NaN", and IEEE 754 makes `x rem 0` invalid for every `x`. `Context::div(1, 0)` remains
`Ok(inf)`.

### `Sum` does not saturate like `+`
Reports: F75
Verdict: real, fixed
Root cause: `precise_sum` (`float/src/iter.rs:109`, `121`) wrapped the rounded sum with
`FBig::new` directly, without `finish_rounded` / `unwrap_fp`. A sum reaching exponent
`isize::MAX` was returned as a nonzero significand at the sentinel exponent (where `+` gives
`inf`), and a sum that `repr_round` saturated was returned as `inf` (where `+` gives the
largest finite value under `mode::Zero`).
Base behaviour: `Sum` returns the same values as now-unfixed; at base `+` returned them too,
so the two agreed (run, debug and release).
Provenance: **incomplete-fix** of E146 ("`+` / `-` leave an overflowed exponent", commit
`3bfafe1`): that made the eight `Add` / `Sub` operators saturate and left `Sum`, which is
documented to match chained `+`. The `Sum` values themselves are base behaviour; the
disagreement with `+` that the report observes dates from `3bfafe1`.
Test: `float/tests/iter.rs::triage::test_sum_exponent_overflow_matches_add`
Fix: the partial sum in `precise_sum` is a significand with an `i128` exponent, rounded by
`repr_round_wide` and resolved by `unwrap_fp`.
Notes: because the exponent is wide, partial sums may leave the range and come back
(`[a; 23] + [-a; 22]` at exponent `isize::MAX - 1` is `a`); with the saturating `Repr`
addition that would have been wrong. `CachedFBig` and `CBig` sums delegate here.

### Shift by zero at a sentinel exponent
Reports: F88
Verdict: real, fixed
Root cause: `scale_exponent` (`float/src/shift.rs:16-27`) tests the *resulting* exponent
against the sentinels, also when the shift is zero and the value already sits there:
`1·10^isize::MAX << 0` was `inf` (`9·10^isize::MAX` under `mode::Zero`),
`1·10^isize::MIN >> 0` was `0`.
Base behaviour: all seven lines of the reproducer are unchanged values at `a0fc535` (run,
debug and release): the base shift only added to the exponent.
Provenance: **regression**, commit `3bfafe1` (the E207 fix "Shifts to a sentinel exponent").
Test: `float/tests/shift.rs::test_shift_by_zero_is_identity`
Fix: a zero shift returns the value unchanged.
Notes: the report's third line, `(x >> 1) << 1 != x` for `x = 1·10^isize::MAX`, is not
fixed and not a bug under the model E207 introduced: a nonzero shift onto a sentinel exponent
is an overflow, as multiplying by the base is. The test pins it.

### Remainder of a zero dividend with a huge divisor exponent
Reports: F94
Verdict: real, fixed
Root cause: in `repr_rem_with` the gap-independent branch for "dividend below the divisor's
lowest digit" was guarded by `!lhs_signif.is_zero()` (`float/src/div.rs:918`), so a zero
dividend fell through to `split_digits(0, shift)`, which materializes `B^shift` for the
divisor's whole exponent. `div_euclid` had the same guard in front of `align_as_int`.
Base behaviour: `rem(±0, 3·2^(isize::MAX/2))`, `0 % …`, `0.rem_euclid(…)` and
`0.div_euclid(…)` all panic with "out of memory" at `a0fc535` too (run with a reduced
reproducer; the report's own program dies earlier at base, on `rem(1, …)`).
Provenance: **incomplete-fix** of E149 / E150 ("Remainder cost grows with the exponent gap",
commit `9c73e01`), which made the remainder independent of the gap for nonzero dividends in
both directions and excluded zero. The defect is base code.
Test: `float/tests/div.rs::triage::test_rem_zero_dividend_huge_divisor_exponent`
Fix: a zero dividend returns its own (signed) zero at the top of `repr_rem_with`, and
`div_euclid` returns `0`, before any alignment.
Notes: committed together with F47 (same kernel lines).

## Per-report table

| Report | Cluster | Verdict | Provenance |
|---|---|---|---|
| F9 | `Repr::new` / `from_parts` read a zero significand as a special value | real, fixed | incomplete-fix (B1) |
| F13 | `quantize` carries onto the sentinel exponent | real, fixed | pre-existing (release); masked in debug by the E193 overflow panic |
| F16 | `fma` reports the range error of the product | real, fixed | pre-existing |
| F36 | `NumHash` negates an `isize::MIN` exponent | real, fixed | pre-existing |
| F41 | Integer operands round an unlimited-precision float | real, fixed | pre-existing |
| F42 | Guard digit rounded by an identity operation | not a bug | pre-existing behaviour (by design) |
| F47 | `Context::rem` panics on a zero divisor | real, fixed | pre-existing |
| F75 | `Sum` does not saturate like `+` | real, fixed | incomplete-fix (E146, `3bfafe1`) |
| F88 | Shift by zero at a sentinel exponent | real, fixed | regression (`3bfafe1`, E207) |
| F94 | Remainder of a zero dividend with a huge divisor exponent | real, fixed | incomplete-fix (E149 / E150, `9c73e01`) |

Counts: 9 real, fixed; 1 not a bug. Provenance: 1 regression, 3 incomplete-fix, 5
pre-existing (one of them the not-a-bug), 1 pre-existing / masked mixture; no new-code.

## Changelog entries

`float/CHANGELOG.md`, under `## Unreleased`:

```markdown
### Fix
- Fix `Repr::new` and `FBig::from_parts` reading a zero significand at exponent `-1`, `isize::MAX` or `isize::MIN` as `-0` / `+inf` / `-inf`: zero is `+0` at every exponent.
- Fix a debug-build overflow panic in `NumHash` for a value with exponent `isize::MIN`.
- Fix `quantize` returning a nonzero significand at the sentinel exponent `isize::MAX` (a factor of the base too small when the carry went beyond it): a result outside the exponent range now saturates like an overflowing multiplication.
- Fix `Context::fma` / `FBig::fma` reporting the `Underflow` / `Overflow` of the product `a·b` when `c + a·b` is representable (`fma` returned `0` for `3 + 2^(isize::MIN - 2)`): the exponent range is checked on the rounded result.
- Fix `+`, `-`, `*` between an unlimited-precision `FBig` (or `CachedFBig`) and an integer rounding the result to the number of digits of the integer (`12.34 + 1u8` was `10`): the result stays exact.
- Fix `Context::rem` panicking on a zero divisor: it returns `Err(FpError::Indeterminate)`.
- Fix `%`, `rem_euclid`, `div_euclid` and `Context::rem` running out of memory for a zero dividend and a divisor with a huge exponent.
- Fix `Sum` returning a nonzero significand at the sentinel exponent, or the wrong saturation endpoint, when the sum overflows the exponent range: it now saturates like `+`, and partial sums may leave the range.
- Fix `<< 0` / `>> 0` changing a value whose exponent is `isize::MAX` or `isize::MIN` (to an infinity, the largest finite value, or zero).

### Change
- `Repr::new(significand, exponent)` no longer rebuilds `-0` or an infinity from the parts returned by `Repr::into_parts`; use `Repr::neg_zero`, `Repr::infinity` and `Repr::neg_infinity`.
- Dividing an unlimited-precision `FBig` by an integer (or an integer by it) panics like any other unlimited-precision division, instead of rounding the quotient to the number of digits of the integer.
- Document that `%` panics on a zero divisor, and that the guard digit of a subtraction result is rounded away by the next operation (also `x + 0` or a one-element `Sum`), but by `with_precision` only for a smaller precision.
```

## Cross-group notes

- `float/src/repr.rs` (shared): `Repr::new` returns `+0` for a zero significand; new
  crate-private `Repr::from_encoded_parts` (the old behaviour of `new`); doc changes on
  `Repr::new`, `Repr::into_parts` and the guard-digit paragraph of the `Repr` docs. Any
  other group that calls `Repr::new` expecting `(0, -1)` / `(0, isize::MAX)` to come back as
  `-0` / `inf` must switch to `from_encoded_parts` or the dedicated constructors; no such
  caller exists in this tree apart from serde.
- `float/src/third_party/serde.rs` (float I/O area): the four `Repr::new(significand,
  exponent)` calls in the deserializers became `Repr::from_encoded_parts`. No format change.
- `float/src/third_party/num_order.rs` (float conversion area): two lines in
  `num_hash_residue`, and the `ModularAbs` import dropped.
- `float/src/error.rs`: new `panic_divide_by_0`; one sentence added to
  `FpError::Indeterminate`.
- `float/src/fbig.rs`, `float/src/helper_macros.rs`, `float/src/fbig_cached_ops.rs`:
  `integer_operand` and its use in the primitive-operand macros, which also generate the
  `Mul` / `Div` operators (and `Rem` for `CachedFBig`).
- `float/src/round.rs` and `float/src/ziv.rs` were not touched.
- Existing test expectation changed: `float/tests/round.rs::triage::test_quantize_zero_result_sign`
  (`mode::Away`, quantum `isize::MAX`, see F13).
- F36 names F9's location but is a separate defect; no report in this group duplicates
  another.
- The exponent-range model stays as E145 left it: a nonzero significand at `isize::MAX` /
  `isize::MIN` is at once "what an overflow endpoint looks like" and "out of range". F13,
  F75 and F88 were fixed inside that model (consistent with `+`, `*`, `<<`), not by
  redesigning it.
- Not run: the `fuzz/` differentials, the Dylint house lints, and an MSRV (1.68) build (the
  oldest installed toolchain, 1.81, cannot resolve the dependency graph). The new code uses
  `i128` arithmetic, `unsigned_abs`, `rem_euclid`, `abs_diff` and `isize::try_from` only.
