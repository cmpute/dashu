# float-fmt: dashu-float text and serialization

29 reports, 16 distinct issues. All 16 are real and all are fixed; no report in the group was
wrong, intended behaviour, or unreproducible. Line numbers under "Root cause" refer to the
tested commit (`a0fc535`).

How each was verified: every regression test below was run against the unfixed source (it
fails) and against the fix (it passes), except where a cluster says otherwise.

## Clusters

### parse: zero with a scale

Reports: A3, D14, E10
Verdict: real, fixed
Root cause: `float/src/parse.rs:153` built every non-`-0` result with `Repr::new(sign * significand, exponent)`. A zero significand keeps the four sentinel exponents, so the scale of a zero literal leaked into the special-value encoding: `0e-1` became `-0` (exponent -1), `0e<isize::MAX>` became `+inf` and `0e<isize::MIN>` became `-inf`.
Test: `float/tests/io.rs::test_from_str_zero_with_scale`
Fix: a zero significand now goes through `Repr::zero_with_sign(sign)`, so only a leading `-` gives `-0` and the scale of a zero is ignored. `FromStr` docs say so.

### parse: second sign accepted

Reports: E153
Verdict: real, fixed
Root cause: `float/src/parse.rs:120` (and the integral-part calls next to it) hand each part to `UBig::from_str_radix`, which strips a leading `+` of its own. After the literal's sign was consumed, a second `+` was therefore accepted in the fractional part (`0.+1`, counted as a digit position, hence 0.01), and also in the integral part (`++1`, `-+1`, `0x+1`).
Test: `float/tests/io.rs::test_from_str_single_sign`
Fix: new private helper `parse_unsigned_digits` in `float/src/parse.rs` rejects a leading `+` (`InvalidDigit`); all five digit-run parses use it.
Notable: the report only mentions the fractional part; the integral part had the same hole.

### parse: "0x." accepted

Reports: E210
Verdict: real, fixed
Root cause: `float/src/parse.rs:83` only rejects a lone `.`; with the hex prefix the integral part is `0x`, and `parse.rs:94` treated the empty remainder as zero even when the fractional part was empty too.
Test: `float/tests/io.rs::test_from_str_prefix_without_digits`
Fix: `NoDigits` when both parts are empty after the prefix.

### parse: exponent at or beyond the isize limits

Reports: E211, E261
Verdict: real, fixed
Root cause: `float/src/parse.rs:129` computes `exponent -= fract_digits` unchecked (debug panic, release wrap to `isize::MAX`), and nothing rejects a nonzero significand at exponent `isize::MIN` / `isize::MAX`, which are reserved for the infinities (`x - x` then panics). `Repr::normalize` also saturates into `isize::MAX` when folding trailing zeros (`10e<MAX-1>`).
Test: `float/tests/io.rs::test_from_str_extreme_exponent`
Fix: checked subtraction, and a nonzero result whose normalized exponent is `isize::MIN` or `isize::MAX` is rejected. Both return `ParseError::InvalidDigit`, which is what an exponent too large for `isize` already returned. Documented on `FromStr`.
Notable: `ParseError` has no out-of-range variant; adding one is an API change in dashu-base, so the existing variant is reused.

### fmt: width one short for integers

Reports: B10, E109
Verdict: real, fixed
Root cause: `float/src/fmt.rs:137` (`if exp > 0`) counted a radix point for `exp == 0` when no precision is set, but an integer prints none, so the computed width was one too large and one fill character was dropped.
Test: `float/tests/io.rs::test_format_width_without_precision`, plus `test_format_width_is_exact`, an exhaustive sweep (values x width x precision x alignment, Display and scientific) asserting the padded length.
Fix: `exp >= 0`.

### fmt: extra digit when rounding carries (scientific)

Reports: B12, C1, E13
Verdict: real, fixed
Root cause: `float/src/fmt.rs:302-308` rounds the significand to `precision + 1` digits but does not handle the carry into one more digit (95 -> 10), so the extra digit was printed as a fractional digit (`1.0e2`).
Test: `float/tests/io.rs::test_scientific_format_carry`
Fix: after rounding, if the digit count exceeds the target, drop the last digit (exact, the value is a power of the base) and bump the exponent. Covers base 2/8/16 and the hex form of binary floats too.

### fmt: zero flag ignored with an alignment (scientific/binary/octal/hex)

Reports: E194
Verdict: real, fixed
Root cause: `float/src/fmt.rs:361-371` computes the padding from the alignment only; the `0` flag was honoured just when no alignment was given (the `Display` path at `fmt.rs:150-153` already had the override).
Test: `float/tests/io.rs::test_scientific_format_zero_flag_alignment`
Fix: same `sign_aware_zero_pad` branch as the `Display` path.

### fmt: Display with a precision runs out of memory on tiny values

Reports: E263
Verdict: real, fixed
Root cause: `float/src/fmt.rs:90-93` splits the significand at `B^shift` where `shift = -(precision + exponent)`; for a hugely negative exponent that power is unbounded (and the cost grew with the exponent well before the OOM).
Test: `float/tests/io.rs::test_format_precision_tiny_value`
Fix: when `shift` exceeds the digit count, the value is a nonzero fraction below half a unit of the last printed place, so it is rounded from its sign alone via `Round::round_low_part`. Constant time; directed modes still round up (`0.001` under `Up`/`Away`).
Notable: the OOM itself was not re-run on the old code (it would try to allocate tens of GB); the test was run on the fix only, the report's output and the code are the evidence for the failure. The mirror case (a huge positive exponent, or no precision) still writes every digit, which is the value's real text and not a bug.

### serde: human-readable precision does not round trip

Reports: A6, D20, E91, E135
Verdict: real, fixed
Root cause: `float/src/third_party/serde.rs:44-53` pads the `Display` string so its digit count reaches the precision, but (a) cannot shrink it when the positional zeros alone exceed the precision (`10` or `1000` at precision 1, `0.5` at precision 1, `0.33` at precision 2: the parser counts the leading `0` and trailing zeros as digits), and (b) counts digits with `is_ascii_digit` (`serde.rs:46`), so digits `a`-`z` of bases above 10 are not counted and the padding is added on every round trip.
Test: `float/tests/serde.rs::test_serde_preserves_precision_exactly` (fixed cases with the expected JSON, plus a sweep over bases 10/2/16, exponents -7..=7 and precisions from the digit count upward and at 20/50/100/500)
Fix: `FBig::to_literal_with_precision` (private, in `serde.rs`) writes a literal with exactly `precision` digits: the padded positional form when it fits, otherwise the significand with an explicit scale (`1@3`, `5@-1`, `5.0@-2`), which `from_str` already parses in every base. The digit count is computed arithmetically, so nothing is formatted for huge exponents.
Notable: **wire-format change** for the values that did not round trip before (`"10"` at precision 1 is now `"1@1"`). Values that did round trip are written exactly as before. Old strings are still accepted.

### serde: unlimited precision becomes precision 1

Reports: E42
Verdict: real, fixed
Root cause: same place; a literal carries its precision as its digit count, and no literal has zero digits, so precision 0 (unlimited: `DBig::ZERO`, `DBig::ONE`, `from_parts(0, _)`) came back as the digit count. The same holds for a value with more digits than its precision (the guard digit of an inexact sum) and for an infinity with a nonzero precision.
Test: `float/tests/serde.rs::test_serde_precision_without_literal`
Fix: those numbers are written as the `(significand, exponent, precision)` struct in human-readable formats too (`{"significand":"1","exponent":0,"precision":0}`), and the human-readable `Deserialize` uses `deserialize_any` so it takes either a string or the struct.
Notable: **this is a wire-format decision and the one fix in this group I am least sure the maintainers will want**: `DBig::ONE` in JSON changes from a string to an object. It is in its own commit (`4f12dcc`) so it can be dropped; without it the behaviour is the pre-existing one for these values (string, precision becomes the digit count). `Repr` is unaffected (it has no precision). `complex/tests/serde.rs::cbig_serde_over_precision_value` documents the over-precision case as an accepted limitation of the string form for `CBig`, which has its own writer and was not changed.

### serde: serialized infinity cannot be read back

Reports: E73
Verdict: real, fixed
Root cause: `float/src/third_party/serde.rs:23,43` write `inf` / `-inf` via `Display`, but `visit_str` (`serde.rs:81,156`) only calls `from_str_native`, which rejects infinities by design.
Test: `float/tests/serde.rs::test_serde_infinity`
Fix: the `Repr` and `FBig` visitors accept exactly `inf` and `-inf`. `FromStr` is unchanged (still documented as not supporting infinities).

### serde: struct form unreadable with owned keys (CBOR)

Reports: E102
Verdict: real, fixed
Root cause: `float/src/third_party/serde.rs:107,186` deserialize map keys as a borrowed `&str` (`map.next_key()?` matched against `&str` constants), which only formats that can lend the input provide. Same pattern at `rational/src/third_party/serde.rs:87` and `complex/src/third_party/serde.rs:128`.
Test: `float/tests/serde.rs::test_serde_struct_field_names`, `rational/tests/serde.rs::test_serde_struct_field_names`, `complex/tests/serde.rs::cbig_serde_struct_field_names` (serde_test tokens with borrowed, transient, owned and byte keys; no ciborium dependency added)
Fix: a `Field` enum deserialized through `deserialize_identifier` with a visitor (`FieldSeed`) in each of the three crates. Also fixed two wrong error payloads found on the way: `FBig`'s unknown-field error listed `Repr`'s fields, and a duplicate `denominator` was reported as `numerator`.
Notable: verified with serde_test only; ciborium itself was not run (it is not a dev-dependency).

### postgres: negative zero panics

Reports: A5, D35, E25
Verdict: real, fixed
Root cause: `float/src/third_party/postgres/mod.rs:155` shortcuts only `is_pos_zero()`; `-0` fell through to `leading_decimal_zeros` on a zero digit (`mod.rs:98`, `unreachable!`). For `Repr` (no precision) it encoded one explicit zero digit instead, which `numeric_to_repr` then hit the same `unreachable!` on.
Test: `float/tests/postgres_types.rs::test_negative_zero`
Fix: any zero significand encodes as the NUMERIC zero; decoding skips leading zero digits and treats all-zero digits as zero; `leading_decimal_zeros` no longer has unreachable arms.

### postgres: dscale drops the fraction of a value with a guard digit

Reports: D21, E44
Verdict: real, fixed
Root cause: `float/src/third_party/postgres/mod.rs:194` derives dscale from the context precision (`exp += digit_len - prec`). An inexact sum can hold `precision + 1` digits (10.9 at precision 2), which made dscale one too small (0), and the server truncates to dscale.
Test: `float/tests/postgres_types.rs::test_dscale_covers_fractional_digits`
Fix: dscale is the number of fractional digits of the value, plus trailing zeros only when the precision exceeds the digit count.
Notable: verified on the wire bytes (dscale 1, digits `[10, 9000]`). The reports' "server stores 10" was not re-run: no PostgreSQL server here, and `float/tests/postgres.rs` (live-server tests) was not run for the same reason.

### postgres: unknown sign word and other malformed input panic in from_sql

Reports: E186
Verdict: real, fixed
Root cause: `float/src/third_party/postgres/postgres_types.rs:37` (`_ => panic!()`), plus `:16` (`assert!(raw.len() >= 8)`) and an `unwrap` on an odd trailing byte.
Test: `float/tests/postgres_types.rs::test_from_sql_malformed`
Fix: `from_sql` returns `Err` for an unknown sign word, a truncated header, a payload that does not match the digit count, and a base-10000 digit outside `0..10000`.

### postgres: 1e32768 refused, precision above 32767 overflows

Reports: E192
Verdict: real, fixed
Root cause: `float/src/third_party/postgres/mod.rs:171-173` converts the base-10 exponent to `i16` before dividing by 4, although only the base-10000 weight has to fit; `mod.rs:194` does `digit_len as i16 - prec as i16` (debug overflow panic). On the decoding side `mod.rs:127` cast a possibly negative precision to `usize`.
Test: `float/tests/postgres_types.rs::test_large_weight_and_precision`
Fix: the arithmetic is done in `isize` with checked conversions at the end; the weight must fit `i16`, the fractional digits must fit dscale (`u16`), and the cosmetic zero padding implied by a large precision saturates at that limit. The decoded precision is never below the number of significant digits.
Notable: byte-for-byte round trip verified for both of the report's inputs.

### rkyv: infinities and large exponents truncated

Reports: E20
Verdict: real, fixed
Root cause: `float/src/repr.rs:57`, `exponent: isize` under the rkyv derive. rkyv archives `isize` with a fixed 32-bit width (`size_32` / `pointer_width_32`) by an `as` cast, so `isize::MAX` (infinity) became -1 (`-0`), `isize::MIN` became 0, and exponents beyond i32 wrapped. rkyv 0.7 does the same (not in the report, reproduced).
Test: `float/src/third_party/rkyv_v08.rs::tests::{special_values_roundtrip, exponent_range}` (rkyv 0.8 is not a dev-dependency, its tests live in the unit-test module as before); `float/tests/rkyv.rs::{special_values_rkyv_roundtrip, exponent_range_rkyv_roundtrip, exponent_out_of_range_rkyv}` for rkyv 0.7.
Fix: the exponent field is archived through a wrapper (`third_party::rkyv_v08::Exponent`, `third_party::rkyv_v07::Exponent`) that keeps the archived layout, stores the two infinity markers as the largest/smallest archived values, and refuses any other exponent not strictly between them: an error with rkyv 0.8, a panic with rkyv 0.7 (its field wrappers cannot return an error).
Notable: layout-compatible with existing archives of ordinary values. On 64-bit targets finite exponents of exactly `i32::MAX` / `i32::MIN` or beyond can no longer be archived (they were silently corrupted before). rkyv 0.8 `Serialize` for `Repr`/`FBig` now needs `S::Error: rancor::Source`, which every stock serializer satisfies. The precision (`usize`, archived as u32) has the same truncation for precisions above 2^32 digits; left alone.

## Per-report table

| Report | Cluster | Verdict |
|---|---|---|
| A3 | parse: zero with a scale | real, fixed |
| A5 | postgres: negative zero panics | real, fixed |
| A6 | serde: human-readable precision does not round trip | real, fixed |
| B10 | fmt: width one short for integers | real, fixed |
| B12 | fmt: extra digit when rounding carries | real, fixed |
| C1 | fmt: extra digit when rounding carries | duplicate of B12 |
| D14 | parse: zero with a scale | duplicate of A3 |
| D20 | serde: human-readable precision does not round trip | duplicate of A6 |
| D21 | postgres: dscale drops the fraction | real, fixed |
| D35 | postgres: negative zero panics | duplicate of A5 |
| E10 | parse: zero with a scale | duplicate of A3 |
| E13 | fmt: extra digit when rounding carries | duplicate of B12 |
| E20 | rkyv: infinities and large exponents truncated | real, fixed |
| E25 | postgres: negative zero panics | duplicate of A5 |
| E42 | serde: unlimited precision becomes precision 1 | real, fixed |
| E44 | postgres: dscale drops the fraction | duplicate of D21 |
| E73 | serde: serialized infinity cannot be read back | real, fixed |
| E91 | serde: human-readable precision does not round trip | duplicate of A6 (the base-16 digit-counting half) |
| E102 | serde: struct form unreadable with owned keys | real, fixed |
| E109 | fmt: width one short for integers | duplicate of B10 |
| E135 | serde: human-readable precision does not round trip | duplicate of A6 |
| E153 | parse: second sign accepted | real, fixed |
| E186 | postgres: malformed input panics in from_sql | real, fixed |
| E192 | postgres: 1e32768 refused, precision above 32767 overflows | real, fixed |
| E194 | fmt: zero flag ignored with an alignment | real, fixed |
| E210 | parse: "0x." accepted | real, fixed |
| E211 | parse: exponent at or beyond the isize limits | real, fixed |
| E261 | parse: exponent at or beyond the isize limits | duplicate of E211 |
| E263 | fmt: Display with a precision runs out of memory | real, fixed |

## Changelog entries

### `float/CHANGELOG.md`

```markdown
### Fix
- `FBig::from_str`: a zero literal ignores its scale. `0e-1` parsed as `-0` and
  `0e<isize::MAX>` / `0e<isize::MIN>` as the infinities; only a leading `-` gives `-0` now.
- `FBig::from_str`: only one sign is accepted, in front of the integral part. `0.+1` parsed
  as 0.01, and `++1`, `-+1`, `0x+1` were accepted.
- `FBig::from_str`: `0x.` (a hex prefix and a dot without any digit) is rejected with
  `ParseError::NoDigits`.
- `FBig::from_str`: a nonzero literal whose exponent overflows or lands on the reserved
  exponents `isize::MIN` / `isize::MAX` is rejected with `ParseError::InvalidDigit`. It used to
  panic in debug builds, wrap around in release builds, or return a value on which arithmetic
  panics.
- `Display` with a width and no precision padded integers one character short
  (`format!("{:6}", 123)` gave 5 characters).
- Scientific formatting (`{:e}`, `{:b}`, `{:o}`, `{:x}`) with a precision printed one
  fractional digit too many when rounding carried into a new leading digit (`95` at `{:.0e}`
  gave `1.0e2`).
- Scientific formatting ignored the `0` flag when an alignment was given (`{:<08e}`).
- `Display` with a precision no longer runs out of memory (or takes time proportional to the
  exponent) for values with a very large negative exponent.
- serde: the human-readable string now has exactly as many digits as the precision, so the
  precision round-trips also when the positional form has more digits than the precision
  (`10` at precision 1, `0.5` at precision 1) and for digits above 9 in bases above 10. Such
  values are written with an explicit scale (`"1@1"`, `"5@-1"`).
- serde: the `inf` / `-inf` strings written for infinities can be deserialized.
- serde: the struct form can be deserialized from formats that provide owned or transient
  field names (e.g. CBOR).
- postgres: negative zero is stored as zero instead of panicking in `to_sql` / `from_sql`.
- postgres: `dscale` always covers the fractional digits of the value. A result carrying a
  guard digit (`11 - 0.1` at precision 2) was sent with `dscale = 0`, so the server stored 10.
- postgres: `to_sql` accepts every value whose weight fits the NUMERIC format (`1e32768` was
  refused) and no longer overflows for precisions above 32767.
- postgres-types: `from_sql` returns an error instead of panicking on an unknown sign word, a
  truncated value or an out-of-range digit.
- rkyv (0.7 and 0.8): infinities round-trip. `INFINITY` used to come back as `-0` and
  `NEG_INFINITY` as `+0`. An exponent that does not fit the 32-bit archived exponent is now
  refused (an error with rkyv 0.8, a panic with rkyv 0.7) instead of being truncated.

### Change
- **(breaking, encoding)** serde: in human-readable formats, an `FBig` whose precision cannot
  be expressed by a literal (unlimited precision, more significant digits than the precision,
  or an infinity with a nonzero precision) is serialized as the
  `(significand, exponent, precision)` struct instead of a string, so that the precision
  round-trips (`DBig::ONE` came back with precision 1). The deserializer accepts both forms.
```

### `rational/CHANGELOG.md`

```markdown
### Fix
- serde: the struct form of `RBig` / `Relaxed` can be deserialized from formats that provide
  owned or transient field names (e.g. CBOR).
```

### `complex/CHANGELOG.md`

```markdown
### Fix
- serde: the struct form of `CBig` can be deserialized from formats that provide owned or
  transient field names (e.g. CBOR).
```

## Cross-group notes

- Touched outside `float/`: `rational/src/third_party/serde.rs` + `rational/tests/serde.rs`
  and `complex/src/third_party/serde.rs` + `complex/tests/serde.rs` (commit `6d22e27`), for the
  rational half of E102 and the identical pattern in `CBig`.
- Touched in `float/` outside text/serialization code proper: `float/src/repr.rs` (two
  `cfg_attr` lines on the `exponent` field for the rkyv wrapper), `float/Cargo.toml` (new
  `[[test]] postgres_types`), new files `float/src/third_party/rkyv_v07.rs` and
  `float/tests/postgres_types.rs`.
- E261's symptom ("`x - x` panics with 'arithmetic operations with the infinity are not
  allowed'" on a nonzero significand at exponent `isize::MIN`/`MAX`) is fixed here at the
  parser. The arithmetic side (how `FBig` treats such a non-canonical `Repr` built some other
  way, e.g. `from_parts`) belongs to the float arithmetic groups.
- `complex`'s human-readable serde has its own writer and still has the limitations fixed here
  for `FBig` (unlimited precision and over-precision parts do not round trip; see
  `complex/tests/serde.rs::cbig_serde_over_precision_value`). Not in this group's reports.
- Pre-existing, not caused by this work: `float/tests/rkyv.rs` and `complex/tests/rkyv.rs` are
  written against rkyv 0.7 but declare `required-features = ["rkyv"]` (= rkyv 0.8), so
  `cargo test --features rkyv --test rkyv` does not compile; they only build when `rkyv_v07`
  is also on (as with `--all-features`).
