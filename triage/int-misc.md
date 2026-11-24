# Group `int-misc` — dashu-int non-kernel API and dashu-base

40 reports, 22 clusters. All 22 reproduced at `a0fc535`; all are real and fixed.
Line references are to the tree after the fixes.

## Clusters

### encode-rounding — `FloatEncoding::encode` misrounds / mis-tags / panics
Reports: D32, D42, E15, E33, E78, E137
Verdict: real, fixed
Root cause: `base/src/bit.rs` `encode` for f32/f64 had three independent defects in one routine:
(1) in the normal branch the sticky bit skipped one bit position (bit 6 for f32, bit 10 for f64 of the
normalized mantissa), so a mantissa whose only discarded bit was the one right below the round bit was
reported `Exact` and ties-plus-that-bit rounded down; (2) values in (2^-150, 2^-149) (f64: (2^-1075, 2^-1074))
were routed to the "underflow" branch and flushed to zero, and the subnormal branch derived its rounding bits
from a wrongly shifted mantissa; (3) that shift (`mantissa << (30 + shift)`) overflowed for `i32::MIN` /
`i64::MIN` at the smallest subnormal (debug panic).
Test: `base/tests/float_encoding.rs` (`test_encode_sticky_bits`, `test_encode_near_smallest_subnormal`,
`test_encode_min_mantissa`, `test_encode_f32_against_f64_oracle`, `test_encode_f64_matches_integer_cast`);
E33 additionally `integer/tests/convert.rs::test_to_float_sticky_bit_below_round_bit`.
Fix: both impls now share the `encode_float!` macro (`base/src/bit.rs:171`), which rounds once at the unit in
the last place of the result (clamped at the smallest subnormal) and compares the whole discarded part with
half an ulp. The old `round_to_even_adjustment` helper is gone.
Notes: E33 (`UBig::to_f64` above 2^128) and the `f32::try_from(RBig)` half of E15 are consequences of defect (1);
both were verified fixed by the `encode` change alone (`f32::try_from(RBig::from(33554433))` now returns
`Err(LossOfPrecision)`; checked with a throwaway test, no rational test added — see cross-group notes).
E137 is debug-only (release wrapped to the right answer). The f32 oracle test is exact (via f64); the f64 test
uses the `i64 as f64` cast as reference for exponent 0 only.

### abs-cmp-min — `AbsOrd::abs_cmp` on `iN::MIN`
Reports: E160
Verdict: real, fixed
Root cause: `base/src/sign.rs:293` compared `self.abs()` values; `iN::MIN.abs()` overflows (debug panic,
release wraps to MIN and compares as the smallest magnitude).
Test: `base/tests/sign.rs::test_abs_cmp_signed_min` (includes an exhaustive i8 check)
Fix: compare `unsigned_abs()`.

### abs-neg-zero — `Abs::abs` on `-0.0`
Reports: E209
Verdict: real, fixed
Root cause: `base/src/sign.rs:94` returned `self` when `self >= 0.`, which holds for `-0.0`.
Test: `base/tests/sign.rs::test_float_abs_negative_zero`
Fix: clear the sign bit (`from_bits(to_bits & !sign_mask)`), which is what `f64::abs` does. Side effect: a NaN
with the sign bit set now has it cleared too (still a NaN; previously returned unchanged).

### log2-est-u128 — `u128::log2_est` returns `inf`
Reports: E139
Verdict: real, fixed
Root cause: `base/src/math/log.rs:254` (std impl) computed `(*self as f32).log2()`; a `u128 >= 2^128 - 2^103`
rounds to 2^128, which is `inf` as f32.
Test: `base/tests/log.rs::test_u128_log2_est_near_max`
Fix: when the conversion overflows return `Self::BITS as f32` (the logarithm of the rounded value).

### log2-bounds-nostd — no-std float `log2_bounds` not a bracket
Reports: E243
Verdict: real, fixed
Root cause: `base/src/math/log.rs:302` (no-std float impl) added the exponent to both mantissa bounds with
round-to-nearest; for a tight mantissa bracket (mantissa 3) and a large exponent both ends rounded to the same
f32 above the true value.
Test: `base/tests/log.rs::test_float_log2_bounds_subnormal` (must be run with `--no-default-features` to cover
the affected impl; it also passes with std)
Fix: widen with `next_down` / `next_up` after the addition, unless the mantissa is a power of two (exact).
Notes: only reproduces without `std`.

### ibig-rem-unsigned — `IBig % uN` / `IBig::div_rem(uN)` panic on negative remainders
Reports: C2, D2, E1
Verdict: real, fixed (**breaking change of an operator output type**)
Root cause: `integer/src/div_ops.rs` implemented `Rem<uN>`, `DivRem<uN>` and `DivRemAssign<uN>` for `IBig`
with the remainder type `uN` via `try_into().unwrap()`; the remainder takes the sign of the dividend, so any
negative non-multiple panicked with `OutOfBounds`. The same macro gave `uN / IBig` the output `uN`, which
panicked the same way for a negative divisor with a nonzero quotient (not in the reports; same root cause).
Test: `integer/tests/div.rs::test_div_rem_ibig_unsigned`
Fix: new `impl_div_unsigned_primitive_with_ibig!` (`integer/src/div_ops.rs:296`): `IBig % uN`,
`IBig::div_rem(uN)`'s remainder, `IBig::div_rem_assign(uN)`'s remainder and `uN / IBig` now return `IBig`
(consistent with `IBig % UBig -> IBig`). Signed primitives are unchanged.
Notes: no unsigned type can hold the result, so there is no fix that keeps the old types; code that relied on
`let r: u32 = ibig % 10u32` now needs a conversion. Nothing else in the workspace (incl. python/benchmark/fuzz,
by grep) used these impls. If the maintainers prefer to keep the old signatures, the alternative is to document
the panic — this commit is self-contained and can be dropped.

### trailing-ones-large — `trailing_ones` wrong / panics beyond two words
Reports: B6, C5, D6, E4
Verdict: real, fixed
Root cause: `integer/src/bits.rs:601` `trailing_ones_large` started scanning at word 1 (so the low word was
assumed all ones) and indexed past the end when every word was `Word::MAX`.
Test: `integer/tests/bits.rs::test_trailing_ones_large`
Fix: scan from word 0 and return `len * WORD_BITS` when all words are ones.

### cbrt-negative — `IBig::cbrt` panics on negatives
Reports: D8, E5
Verdict: real, fixed
Root cause: `integer/src/root_ops.rs:109` copied the negative check of `sqrt`.
Test: `integer/tests/root.rs::test_cbrt_ibig`
Fix: delegate to `nth_root(3)`. This also fixes `num_integer::Roots::cbrt` for `IBig`.

### ibig-from-float-floor — `IBig::try_from(f32/f64)` floors negatives
Reports: B7, D10, E6
Verdict: real, fixed
Root cause: `integer/src/convert.rs:878` built a signed `IBig` mantissa and used `>>=`, which is a flooring
shift for negative `IBig`; positive values (and `UBig`) truncate.
Test: `integer/tests/convert.rs::test_ibig_from_float_truncates`
Fix: shift the magnitude and attach the sign afterwards (truncation towards zero for both signs). Also fixes
`num_traits::FromPrimitive::from_f32/from_f64` for `IBig`, which forward here.
Notes: the rounding mode was undocumented; truncation is what the positive branch, `UBig`, `as` casts and
num-traits' primitives do.

### from-chunks-alloc — `UBig::from_chunks` over-allocates 64x / OOM
Reports: E8
Verdict: real, fixed
Root cause: `integer/src/convert.rs:381` sized the result as `max_len + (n - 1) * chunk_bits + 1` *words*
(chunk_bits is a bit count).
Test: `integer/tests/from_chunks_alloc.rs::test_from_chunks_allocation_is_proportional_to_result` (own binary:
installs a counting global allocator); values in `integer/tests/convert.rs::test_from_chunks_overlapping`.
Fix: `max_len + (n - 1) * chunk_bits / WORD_BITS + 1` words, with a checked multiplication; leading zero chunks
are dropped first so they cannot inflate the allocation.

### from-chunks-zero — `UBig::from_chunks(_, 0)` does not panic
Reports: E214
Verdict: real, fixed
Root cause: same function had no `chunk_bits > 0` check (docs: "Panics if chunk_bits is zero"); the existing
`#[should_panic]` test passed only because of the `to_chunks(0)` call before it.
Test: `integer/tests/convert.rs::{test_from_chunks_zero_bits_empty, test_from_chunks_zero_bits_nonempty,
test_to_chunks_zero_bits}`
Fix: `assert!(chunk_bits > 0)` at the top of `Repr::from_chunks`.

### to-f32-exact-32bit — `to_f32` reports `Exact` for `u64::MAX` on 32-bit words
Reports: E9
Verdict: real, fixed
Root cause: `integer/src/convert.rs:1143` `to_f32_small` round-tripped through the saturating cast
`f as DoubleWord`; when the double word is u64 (32-bit words) or u32 (16-bit words) the value can round up to
2^BITS, which saturates back to `DoubleWord::MAX` and compares equal. (`to_f64_small` already had this guard.)
Test: `integer/tests/convert.rs::test_to_f32_rounding_up_to_power_of_two`
Fix: treat `f >= 2^BITS` as rounded up (2^BITS is `inf` for the u128 double word, subsuming the old check).
Notes: only reproduces with `RUSTFLAGS='--cfg force_bits="32"'` (or `"16"`); verified failing before and
passing after under that cfg, and the whole dashu-int test suite passes under both cfgs.

### ilog-zero — `ilog` of zero returns a value
Reports: E11
Verdict: real, fixed
Root cause: `integer/src/log.rs:96` `log` only reached the zero check in `log_dword`; the base-2 and
power-of-two shortcuts computed `bit_len() - 1` (debug overflow panic, release `usize::MAX` / OOM) and the
large-base arm returned `(0, 1)`.
Test: `integer/tests/log.rs::{test_log_of_zero, test_log_of_zero_base_2, test_log_of_zero_base_16,
test_log_of_zero_large_base}`
Fix: reject zero at the top of `log`.

### parse-underscore-only — strings of only underscores parse as 0
Reports: E189
Verdict: real, fixed
Root cause: `integer/src/parse/mod.rs:109` only rejected the empty string; the digit loops skip `_`.
Test: `integer/tests/io.rs::test_from_str_underscores_only`
Fix: return `NoDigits` when the (sign- and prefix-stripped) string contains nothing but underscores.

### parse-default-radix — `from_str_with_radix_default` panics on bad radix
Reports: E198
Verdict: real, fixed
Root cause: `integer/src/parse/mod.rs:89` passed `default_radix` on unvalidated (debug assert / division by
zero / assertion in release).
Test: `integer/tests/io.rs::test_from_str_with_radix_default_unsupported_radix`
Fix: return `Err(UnsupportedRadix)` up front (also when the string carries its own `0x`/`0o`/`0b` prefix).

### num-integer-div-floor — `IBig::div_floor` returns 0 instead of -1
Reports: A1, B3, D7, E39
Verdict: real, fixed
Root cause: `integer/src/third_party/num_integer.rs:88` adjusted when the truncated quotient was negative; it
is zero (non-negative) when `|a| < |b|`.
Test: `integer/tests/num_integer.rs::ibig_floor_division_small_quotient` (sweep against `i64`)
Fix: adjust when the remainder is nonzero and the operand signs differ. `div_mod_floor` / `div_ceil` (default
methods) follow.

### num-integer-lcm-sign — `IBig::lcm` negative for mixed signs
Reports: E65
Verdict: real, fixed
Root cause: `integer/src/third_party/num_integer.rs:137` computed `a / gcd * b` without taking the magnitude.
Test: `integer/tests/num_integer.rs::ibig_lcm_is_non_negative`
Fix: take `Abs::abs` of the product.

### num-integer-gcd-zero — `gcd(0, 0)` panics through `num_integer::Integer`
Reports: E151
Verdict: real, fixed
Root cause: `num_integer.rs:37,54,128,146` forwarded to dashu's `Gcd`/`ExtendedGcd`, which document a panic
for (0, 0); the num-integer trait defines gcd(0, 0) = 0 (and `lcm` in the same impl already handled (0, 0)).
Test: `integer/tests/num_integer.rs::gcd_of_two_zeros_is_zero`
Fix: return zero (and Bézout coefficients (0, 0)) for (0, 0) in the trait impls only; the inherent
`Gcd::gcd` keeps its documented panic.

### num-integer-feature — `num-integer` feature does not compile alone
Reports: E184
Verdict: real, fixed
Root cause: `integer/Cargo.toml`: `num_integer::Integer` requires `num_traits::Num`, but the implicit
`num-integer_v01` feature did not enable `num-traits_v02`.
Test: CI step `cargo check -p dashu-int --no-default-features --features num-integer` in
`.github/workflows/build.yml` (a build configuration cannot be a unit test).
Fix: explicit feature `num-integer_v01 = ["dep:num-integer_v01", "num-traits_v02"]`. Also corrected the feature
list in `integer/src/lib.rs` (it claimed num-traits/num-integer/rand were default features).

### num-order-float — `NumOrd` vs small and infinite floats
Reports: D15, E34, E37
Verdict: real, fixed
Root cause: `integer/src/third_party/num_order.rs`: (a) for a float with magnitude below 0.5
(`other_bits < 0`) both impls answered "integer is greater" without checking for zero (`:177`, `:245`);
(b) the IBig impl returned `-sign * Less` for infinities, inverting both same-sign cases (`:233`).
Test: `integer/tests/num_order.rs::{test_ord_with_small_float, test_ord_with_infinite_float,
test_ord_with_float_matches_exact_conversion}`
Fix: for `other_bits <= 0` the integer is greater in magnitude iff it is nonzero; infinity uses
`sign * Less`.

### abs-sub — `num_traits::Signed::abs_sub` returns `|a - b|`
Reports: A8, E57
Verdict: real, fixed (IBig only)
Root cause: `integer/src/third_party/num_traits.rs:144` returned `Abs::abs(self - other)`; the trait defines
the positive difference `max(self - other, 0)`.
Test: `integer/tests/num_traits.rs::test_abs_sub` (needs `--features num-traits`)
Fix: return zero when `self <= other`.
Notes: A8 also covers `RBig`/`Relaxed` (`rational/src/third_party/num_traits.rs:46`) and `FBig`
(`float/src/third_party/num_traits.rs:168`), which have the same defect and are **not** fixed here (reports
E101 and E72 in the rational and float groups).

## Per-report table

| Report | Cluster | Verdict |
|---|---|---|
| A1 | num-integer-div-floor | real, fixed |
| A8 | abs-sub | real, fixed (IBig part; RBig/FBig parts belong to E101/E72) |
| B3 | num-integer-div-floor | duplicate of A1 |
| B6 | trailing-ones-large | real, fixed |
| B7 | ibig-from-float-floor | real, fixed |
| C2 | ibig-rem-unsigned | real, fixed |
| C5 | trailing-ones-large | duplicate of B6 |
| D2 | ibig-rem-unsigned | duplicate of C2 |
| D6 | trailing-ones-large | duplicate of B6 |
| D7 | num-integer-div-floor | duplicate of A1 |
| D8 | cbrt-negative | real, fixed |
| D10 | ibig-from-float-floor | duplicate of B7 |
| D15 | num-order-float | real, fixed |
| D32 | encode-rounding | real, fixed |
| D42 | encode-rounding | duplicate of D32 (subnormal + MIN-mantissa defects of the same routine) |
| E1 | ibig-rem-unsigned | duplicate of C2 |
| E4 | trailing-ones-large | duplicate of B6 |
| E5 | cbrt-negative | duplicate of D8 |
| E6 | ibig-from-float-floor | duplicate of B7 |
| E8 | from-chunks-alloc | real, fixed |
| E9 | to-f32-exact-32bit | real, fixed |
| E11 | ilog-zero | real, fixed |
| E15 | encode-rounding | duplicate of D32 |
| E33 | encode-rounding | duplicate of D32 (seen through `UBig::to_f64`) |
| E34 | num-order-float | duplicate of D15 |
| E37 | num-order-float | duplicate of D15 |
| E39 | num-integer-div-floor | duplicate of A1 |
| E57 | abs-sub | duplicate of A8 |
| E65 | num-integer-lcm-sign | real, fixed |
| E78 | encode-rounding | duplicate of D32 |
| E137 | encode-rounding | duplicate of D32 (debug-only panic) |
| E139 | log2-est-u128 | real, fixed |
| E151 | num-integer-gcd-zero | real, fixed |
| E160 | abs-cmp-min | real, fixed |
| E184 | num-integer-feature | real, fixed |
| E189 | parse-underscore-only | real, fixed |
| E198 | parse-default-radix | real, fixed |
| E209 | abs-neg-zero | real, fixed |
| E214 | from-chunks-zero | real, fixed |
| E243 | log2-bounds-nostd | real, fixed |

## Changelog entries

`base/CHANGELOG.md`, under `## Unreleased`:

```markdown
### Fix
- `FloatEncoding::encode` for `f32`/`f64` is now correctly rounded (to nearest, ties to even) for every input: a mantissa wider than the float whose only discarded bit lies right below the rounding bit is no longer reported as `Exact` or rounded down, values between half of the smallest subnormal and the smallest subnormal are no longer flushed to zero, results in the subnormal range are rounded and tagged correctly, and `encode(i32::MIN, -180)` / `encode(i64::MIN, -1137)` no longer overflow a shift.
- `AbsOrd::abs_cmp` on signed primitive integers no longer panics (debug) or returns the wrong order (release) when an operand is the minimum value of the type.
- `Abs::abs` on `f32`/`f64` returns `+0.0` for `-0.0`.
- `EstimatedLog2::log2_est` on `u128` no longer returns infinity for values that round to 2^128 as `f32`.
- Without the `std` feature, `EstimatedLog2::log2_bounds` on `f32`/`f64` always brackets the exact logarithm (it could return two equal bounds above it, e.g. for `f32::from_bits(3)`).
```

`integer/CHANGELOG.md`, under `## Unreleased`:

```markdown
### Change
- (breaking) `IBig % uN`, the remainder of `IBig::div_rem(uN)` / `IBig::div_rem_assign(uN)` and `uN / IBig` (for an unsigned primitive `uN`) now return an `IBig` instead of `uN`. The result takes its sign from the `IBig` operand, so these operations used to panic whenever it was negative (e.g. `IBig::from(-1) % 2u32`).
- The `num-integer` (`num-integer_v01`) feature now enables `num-traits` (`num-traits_v02`); it failed to compile when enabled on its own.

### Fix
- `UBig::trailing_ones` / `IBig::trailing_ones` returned wrong counts for values larger than two words, and panicked when every word was all ones.
- `CubicRoot::cbrt` (and `num_integer::Roots::cbrt`) on a negative `IBig` returns the negative root instead of panicking.
- `IBig::try_from(f32/f64)` (and `num_traits::FromPrimitive::from_f32/from_f64`) truncates towards zero for negative values as well; it used to round them towards negative infinity (`-0.5` gave `-1`).
- `UBig::from_chunks` no longer allocates `chunk_bits` words per chunk (about 64 times the size of the result, running out of memory for large `chunk_bits`), and now panics when `chunk_bits` is zero as documented.
- `UBig::to_f32` / `IBig::to_f32` no longer report `Exact` for a value that rounds up to 2^32 (16-bit words) or 2^64 (32-bit words), such as `u64::MAX`.
- `UBig::to_f64` / `IBig::to_f64` (and `to_f32`) are correctly rounded and tagged for multi-word values whose only bit below the rounding bit is the next one down (through the `FloatEncoding::encode` fix in `dashu-base`).
- `UBig::ilog` / `IBig::ilog` of zero now panic for every base; they returned a value for power-of-two bases (release builds) and for bases of 2^128 and above.
- Parsing rejects strings without any digit other than underscore separators (`"_"`, `"-_"`, `"0x_"`) with `ParseError::NoDigits` instead of returning zero.
- `UBig::from_str_with_radix_default` / `IBig::from_str_with_radix_default` return `ParseError::UnsupportedRadix` for a default radix outside 2..=36 instead of panicking.
- `num_integer::Integer` for `IBig`: `div_floor` (and `div_mod_floor`, `div_ceil`) returned 0 instead of -1 when the operands have different signs and `|a| < |b|`; `lcm` (and `gcd_lcm`) returned a negative value for operands of different signs.
- `num_integer::Integer::{gcd, gcd_lcm, extended_gcd}` for `UBig` and `IBig` return zero for `(0, 0)` instead of panicking.
- `num_order::NumOrd` between `UBig`/`IBig` and `f32`/`f64`: zero compared as greater than positive floats below 0.5, and `IBig` values compared as greater than `+inf` and less than `-inf`.
- `num_traits::Signed::abs_sub` for `IBig` returns `max(a - b, 0)` instead of `|a - b|`.
```

## Cross-group notes

- **Files outside `integer/` and `base/`:** `.github/workflows/build.yml` (one added step:
  `cargo check -p dashu-int --no-default-features --features num-integer`, skipped on the 1.68 matrix entry).
  New test files: `base/tests/{float_encoding,sign,log}.rs` (dashu-base had no `tests/` directory),
  `integer/tests/{num_traits,from_chunks_alloc}.rs`.
- **`base/src/bit.rs` `encode` is shared by float/rational conversions.** The rewrite changes results only where
  the old one was wrong, but other groups' reports about `to_f32`/`to_f64`/`f32::try_from` on `FBig`/`RBig`
  with mis-tagged exactness or wrong subnormals may be this root cause (E15's `f32::try_from(RBig::from(33554433))`
  is; verified fixed, no test added in `rational/`). Full workspace tests pass with the change.
- **`abs_sub` (A8):** the `RBig`/`Relaxed` and `FBig` impls are still `|a - b|`
  (`rational/src/third_party/num_traits.rs:46`, `float/src/third_party/num_traits.rs:168`) — left to the
  rational (E101) and float (E72) groups.
- **Breaking change to reconcile:** `IBig % uN`, `IBig::div_rem(uN)`, `IBig::div_rem_assign(uN)` and `uN / IBig`
  now return `IBig` (commit "Return IBig from IBig % uN, ..."). Any other group's new tests that use these
  operators with the old `uN` result type will need `IBig` on the right-hand side of their assertions.
- `IBig::cbrt` now accepts negatives; `CubicRootRem` is still not implemented for `IBig` (not reported).
