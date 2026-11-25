# F run — group `float-io`

dashu-float parsing, formatting, serialization (serde / postgres / rkyv), conversion,
comparison, random sampling and small accessors. 17 reports, 17 distinct clusters (no two
reports share a root cause). All 17 were reproduced at `b0705ed`; all are fixed, one of them
(F25) only in part because the other part is not a bug.

"Base" below is `a0fc535`. Base behaviour was established by running the same reproducer
against the base checkout from a throwaway crate (debug build unless stated), and the origin of
the faulty code with `git log -S` / `git log -- <file>` over `a0fc535..8137b6e`.

A note that several clusters depend on: a nonzero significand at the exponent `isize::MAX` (or
`isize::MIN`) is **not** an infinity, and the library produces such values itself —
`FBig::from_parts(2, isize::MAX - 1)` normalizes onto it, and `Context::unwrap_fp` returns
`(B^p − 1)·B^isize::MAX` as the "largest finite" endpoint of an overflow under the inward
rounding modes (see the earlier `float-arith.md`, "`+` / `-` leave an overflowed exponent").
The arithmetic kernels treat a *result* at that exponent as an overflow, and `FromStr` refuses
a literal with it. I kept all three conventions and made the code in this group consistent with
them; I did not try to forbid the values.

## Clusters

### to_f32 / to_f64: base conversion overflows the binary exponent
Reports: F7
Verdict: real, fixed
Root cause: `Context::convert_to_f64` / `convert_to_f32` (`float/src/convert.rs:645`, `:699` at `b0705ed`)
round the base-2 image of the value with `repr_round`. When the binary exponent does not fit an
`isize` (`3e18·log2(10)`, or `4·(isize::MAX / 2)` for base 16), `convert_base` reports
`Inexact(±inf)`; `convert_base_odd` passed it through (its zero-significand arm also matches an
infinity), the range check did not see it (`log2_bounds` of an infinity was `-inf`, see F77),
and `repr_round` hit `assert_finite`.
Base behaviour: verified by running. `FBig::<HalfEven, 16>::from_parts(1, isize::MAX / 2).to_f64()`
panics identically ("arithmetic operations with the infinity are not allowed!"). The decimal
inputs (`1e(1e18)`, `1e(3e18)`) panic with "out of memory" instead: the base evaluated
`10^exponent` on the way.
Provenance: mixture. For the base-16 input: **pre-existing**. For the decimal inputs:
**masked** — the unguarded infinity is base code, but at the base a decimal input of this size
died earlier in the base conversion (the cluster "Base conversion wrong for |exponent| > 38",
D39 / E22, fixed by `94e6641`, whose new `convert_base_far` returns the saturated infinity that
exposes it).
Test: `float/tests/convert.rs::test_to_float_exponent_beyond_binary_range`
Fix: `convert_to_f64` / `convert_to_f32` return `Err(Overflow)` when the base-2 image is an
infinity, so `to_f32` / `to_f64` saturate by rounding mode (`±inf`, or `±MAX` under an inward
mode) like every other out-of-range value.

### `{:e}`: printed exponent overflows `isize`
Reports: F11
Verdict: real, fixed
Root cause: `fmt_round_scientific` (`float/src/fmt.rs:358` at `b0705ed`) computed the exponent of the leading
digit as `exp + len − 1` in `isize`; for `123e<isize::MAX − 1>` that is `isize::MAX + 1` (debug
panic "attempt to add with overflow"; a release build wraps and prints `1.23e-9223372036854775808`).
The same sum is used after rounding to a precision (`self.exponent - diff + 1`).
Base behaviour: verified by running (debug): the same panic at the same expression for
`12e9223372036854775806` and `123e9223372036854775806`.
Provenance: **pre-existing**.
Test: `float/tests/io.rs::test_scientific_format_extreme_exponent`
Fix: the scientific formatter tracks the exponent as `i128`. Covers `{:e}`/`{:E}`, the
precision/carry path, and the `b`/`o`/`x` forms. The release-mode output was not re-run; the
arithmetic no longer has a narrow intermediate.

### `{:.0}` of `-0` prints `-1` under `Up` / `Away`
Reports: F15
Verdict: real, fixed
Root cause: `fmt_round` (`float/src/fmt.rs:92-103` at `b0705ed`). `-0` is stored with the exponent `-1`, so with
a precision of 0 the "tiny value" branch (`shift > digits_ub()`, i.e. `1 > 0`) took it for a
nonzero negative fraction and rounded it from its sign alone: `-1` under the modes that round a
negative fraction away from zero.
Base behaviour: verified by running: prints `-0` for all four modes.
Provenance: **regression**, commit `c90d537` ("float: fix Display/scientific width, carry digit,
zero flag and tiny-value rounding", the fix for E263), which added that branch.
Test: `float/tests/io.rs::test_format_negative_zero_precision` (6 modes × bases 2 and 10, plus
bases 3 and 16, with width / `0` / `+`, and the scientific forms)
Fix: a zero is never rounded by the formatter (the precision only pads it).

### postgres: more than 16383 significant digits refused
Reports: F17
Verdict: real, fixed
Root cause: `repr_to_numeric` (`float/src/third_party/postgres/mod.rs:173-177`) compared an
estimate of the **decimal** digit count with `u16::MAX / 4`, i.e. the limit of the base-10000
digit count divided by four instead of multiplied, and returned `LossOfPrecision`.
Base behaviour: verified by running: identical (`Ok` at 16383 digits, `Err("number can't be
converted losslessly")` from 16384 on).
Provenance: **pre-existing**.
Test: `float/tests/postgres_types.rs::test_many_significant_digits`
Fix: the limit is what the format holds. The scale is checked first, a cheap filter on the
magnitude (`log10 > 131072 + 1`) refuses numbers that cannot have a valid weight before they
are expanded into digits, and the weight is checked exactly afterwards as before. A number that
passes has at most 36864 base-10000 digits, so the 16-bit count cannot overflow. A too-long
integer is now `OutOfBounds` rather than `LossOfPrecision`.
Notable: verified for up to 131072 digits in the scratch reproducer; the committed test stops at
17000 digits because the round trip of the largest sizes takes seconds in a debug build.

### postgres: `dscale` above the server's limit
Reports: F18
Verdict: real, fixed
Root cause: `MAX_DSCALE` (`float/src/third_party/postgres/mod.rs:91`) was `u16::MAX`. The wire
field is 16 bits, but the server keeps 14: `numeric_recv` rejects a value whose `dscale` is not
covered by `NUMERIC_DSCALE_MASK` (`0x3FFF`).
Base behaviour: verified by running: the same bytes for the reported inputs (`5` at precision
16385 → dscale 16384, `1e-16384` → dscale 16384). The base refused `1e-65535`, which `8137b6e`
accepted with dscale 65535.
Provenance: **pre-existing** for the reported inputs. The earlier fix for E192 (`bad3e07`) moved
the cap to `u16::MAX` and added the two test expectations (dscale 39999 and 65535) that this
changes.
Test: `float/tests/postgres_types.rs::test_large_weight_and_precision` (updated)
Fix: `MAX_DSCALE = 0x3FFF`: the cosmetic padding of a wide precision stops there, and a value
with more fractional digits is `OutOfBounds`.
Notable: **not verified against a server.** The limit is taken from the PostgreSQL source as the
report quotes it (and as I remember it); no database was available. `from_sql` still decodes a
larger `dscale` from another producer.

### serde: human-readable `Repr` writes one character per unit of the exponent
Reports: F23
Verdict: real, fixed
Root cause: `impl Serialize for Repr` (`float/src/third_party/serde.rs:103`) used
`collect_str(self)`, i.e. the positional `Display` form.
Base behaviour: verified by running: identical for `Repr` (1003 / 100003 bytes for `1e1000` /
`1e100000`), and the base wrote `FBig` the same way.
Provenance: **pre-existing**. (`fbc65cf` gave `FBig` the literal with a scale as part of the
precision round trip, A6 / D20 / E91 / E135; `Repr` was not touched.)
Test: `float/tests/serde.rs::test_repr_serde_large_exponent`
Fix: `Repr::to_literal`: the positional form while it adds at most 32 zeros, `signif@exp`
beyond. The deserializer already read that form.
Notable: output change — `Repr` values whose positional form has more than 32 padding zeros
serialize differently (`"1@300"` instead of 301 characters). Both forms are read.

### rkyv: precision above `u32::MAX` truncated
Reports: F24
Verdict: real, fixed
Root cause: `Context::precision` (`float/src/repr.rs:113`) was archived by the derive as a plain
`usize`, which is 32 bits wide in the archive (`size_32` / `pointer_width_32`) and truncates.
Base behaviour: verified by running (feature `rkyv_v07`): identical (`2^32` → 0, `2^32 + 5` → 5).
Provenance: **pre-existing**. (The earlier E20 fix wrapped the exponent only.)
Test: `float/tests/rkyv.rs::precision_range_rkyv_roundtrip`, `::precision_out_of_range_rkyv`,
`::precision_wrapping_to_unlimited_rkyv` (rkyv 0.7); `float/src/third_party/rkyv_v08.rs`
`tests::precision_range` (rkyv 0.8, in-source like the other 0.8 tests)
Fix: a `Precision` field wrapper next to `Exponent` in both rkyv modules: a precision that does
not fit is refused — an error with rkyv 0.8, a panic with rkyv 0.7 (which gives a field wrapper
no error channel; same convention as the exponent). The archived layout is unchanged.
Notable: rkyv 0.8 had the same defect; it was not in the report and was not run at the base.

### serde: struct form accepts more digits than the precision; reserved exponents
Reports: F25
Verdict: real, fixed (significand wider than `precision + 1`); not a bug (nonzero significand at
exponent `isize::MAX`)
Root cause: `FBigVisitor::visit_map` / `visit_seq` (`float/src/third_party/serde.rs:283-335` at `b0705ed`)
built the `FBig` from the three fields without the check `FBig::from_repr` documents, so
`{"significand":"123456","exponent":0,"precision":1}` gave a value that changes when zero is
added to it.
The other half of the report — the struct form accepts `5·10^isize::MAX` while the literal
`"5e9223372036854775807"` is refused — is intended: such a value exists (see the note at the
top), no literal can denote it, and the struct form is how it round-trips (F27). It is finite
(`is_finite()`), and `a * 2 = inf` is the overflow it should be.
Base behaviour: verified by running: the base read no struct form from a human-readable format
at all (`invalid type: map`), and it *accepted* the literal `"5e9223372036854775807"`.
Provenance: **new-code** — the human-readable struct form was added by `622dbbf` / `fbc65cf`.
(The same missing check sits in the struct form of the binary formats, which is base code; that
path was not run at the base.)
Test: `float/tests/serde.rs::test_serde_struct_significand_fits_precision`,
`::test_serde_reserved_exponent` (pins the intended half)
Fix: the struct form is rejected ("the significand has more digits than the precision allows")
when the normalized significand has more than `precision + 1` digits; unlimited precision,
infinities and the one guard digit the serializer itself writes are accepted.

### serde: a value at a reserved exponent is written as a literal that cannot be read
Reports: F27
Verdict: real, fixed
Root cause: `FBig::to_literal_with_precision` (`float/src/third_party/serde.rs:44`) wrote
`1.0@9223372036854775807` for `from_parts(2, isize::MAX − 1)`, and the parser refuses that
exponent for a nonzero number.
Base behaviour: verified by running: the base does not return for this input — it wrote the
positional form, 2^63 zeros (the run was killed by its 400 s timeout at this step).
Provenance: **new-code** — the literal with a scale is from `fbc65cf`, and the parser's refusal
of the reserved exponents from `59a79d4` (E211 / E261). Neither existed at the base.
Test: `float/tests/serde.rs::test_serde_reserved_exponent`
Fix: a nonzero number at a reserved exponent has no literal: `FBig` falls back to the struct
form it already uses for the other values without a literal, and so does `Repr`, whose
human-readable deserializer now reads the struct form too (`deserialize_any`).
Notable: of the report's two acceptable outcomes ("`from_parts` never builds it" or "it reads
back") this is the second; the first would need `from_parts` to panic or change its value.

### parse: trailing fractional zeros push the exponent below `isize::MIN`
Reports: F37
Verdict: real, fixed
Root cause: `Repr::from_str_native` (`float/src/parse.rs:146-149`) computed `cc − len(bbb)` with
`checked_sub` and rejected the literal when it left `isize`, before the trailing zeros of the
significand were folded back into the exponent.
Base behaviour: verified by running (debug): `".100e-9223372036854775806"` and
`"0.1000e-9223372036854775805"` panic with "attempt to subtract with overflow"; the other four
literals of the report parse.
Provenance: **incomplete-fix** — "parse: exponent at or beyond the isize limits" (E211 / E261,
`59a79d4`) replaced the unchecked subtraction by a checked one that rejects, instead of
carrying the exponent until it is normalized.
Test: `float/tests/io.rs::test_from_str_extreme_exponent` (extended, with the `b` / `p` / `@`
markers)
Fix: the exponent is an `i128` until `Repr::new_wide` normalizes and range-checks it.
Notable: the report says all six of its literals denote `1e-9223372036854775807`; the last one,
`0.1000e-9223372036854775805`, is `1e-9223372036854775806`. It was wrongly rejected all the same.

### fmt: infinities ignore width, fill, alignment and `+`
Reports: F49
Verdict: real, fixed
Root cause: `Repr::write_if_infinite` (`float/src/fmt.rs:71`) writes the bare string, and both
formatters returned through it.
Base behaviour: verified by running: identical.
Provenance: **pre-existing**.
Test: `float/tests/io.rs::test_format_infinity_flags` (compared against `f64` for 20 specs)
Fix: `Repr::fmt_if_infinite` pads like the finite path (sign, then zeros under `0`, otherwise
fill by alignment, right-aligned by default) and writes `+inf` under `+`; the precision is
ignored. `Debug` is unchanged. Both guides say so now.

### NumOrd between different bases builds the powers of the bases
Reports: F50
Verdict: real, fixed
Root cause: `impl NumOrd<Repr<B2>> for Repr<B1>` (`float/src/third_party/num_order.rs:63-75`):
when the `f32` logarithm estimates do not separate the two numbers it scaled both by
`B1^|e1|` and `B2^|e2|`.
Base behaviour: verified by running (release): the same timings (`n = 10^6`: 7.8 ms base,
8.3 ms at `b0705ed`; `n = 3·10^7`: 436 ms / 460 ms).
Provenance: **pre-existing**.
Test: `float/tests/num_order.rs::test_ord_cross_base_large_exponent` (the two binary neighbours
of decimal numbers at 20 / 50 / 100 / 500 bits, exponents up to 4·10^12),
`::test_ord_cross_base_equal_values`, `::test_ord_cross_base_matches_scaled_integers` (sweep
against the scaled-integer oracle)
Fix: `abs_cmp_cross_base`. Up to 4096 bits of powers: scaled integers as before. Bases that are
powers of one integer (2/8, 4/8): exact, through the difference of the exponents. Independent
bases: enclosures of `ln|s| + e·ln B` from the `Down` / `Up` contexts at a doubling precision.
Two such numbers can be equal only if `max|e| ≤ 64·max(bits of the significands)` (argument in
the code comment); only then is the loop cut off, falling back to the scaled integers.
`1e-1000000000` vs `2^-3321928094` now takes about 0.2 ms.
Notable: this calls `Context::ln` from the comparison (the logarithm group's code, unchanged).
The equality bound is my own argument, not a library fact — worth a second pair of eyes.

### UniformFBig returns `-0`
Reports: F71
Verdict: real, fixed
Root cause: `UniformFBig::sample_fbig` (`float/src/third_party/rand.rs`) evaluates in
`Context<Down>`, where an exact cancellation (`low + unit·width = 0`) is `-0`; and `with_sampler`
computed the width `0 − 0` in that context, `-0`, for the point range `[0, 0]`.
Base behaviour: verified by running: a range that straddles zero gives `-0` at the base too
(45 of 100 for `[-1, 1)` at precision 1); `new_inclusive(0, 0, 5)` gives `+0` at the base.
Provenance: mixture. Straddling ranges: **pre-existing**. `new_inclusive(0, 0)`: **regression**,
commit `1812267` ("float: keep uniform samples inside their range…", D25 / E85), which made the
width an exact `Down` difference.
Test: `float/tests/random.rs::test_uniform_zero_sample_is_positive`
Fix: a zero sample is `+0`.

### UniformFBig cost grows with the exponent gap of the bounds
Reports: F81
Verdict: real, fixed
Root cause: same function: the sample was the *exact* `low + unit·(high − low)`, rounded
afterwards. The exact width of `[2^-g, 1)` has `g` digits.
Base behaviour: verified by running (release): 100 samples of `[2^-10^7, 1)` take 13 µs and of
`[10^-10^5, 1)` 40 µs — no dependence on the gap. (The base rounded the width to the precision,
which is what let samples leave the range, D25.)
Provenance: **regression**, commit `1812267`.
Test: `float/tests/random.rs::test_uniform_distant_exponents` (gaps up to `isize::MAX / 2`, which
no exact evaluation could finish), `::test_uniform_matches_exact_formula` (the samples are
unchanged: same random stream, exact reference, 20 / 50 / 100 / 500 bits and bases 2 / 3 / 10)
Fix: the same value as `low·(1 − unit) + high·unit`: two exact short products and one correctly
rounded (`Down`) addition. The struct's `scale` / `offset` fields became `low` / `high`
(`pub(crate)`).

### log2 of an infinity is `-inf`
Reports: F77
Verdict: real, fixed
Root cause: `impl EstimatedLog2 for Repr` (`float/src/log.rs:34`) returns the zero answer for
every zero significand, which an infinity has too; `log2_est` computed `-inf + exponent·log B`.
Base behaviour: verified by running: identical.
Provenance: **pre-existing**.
Test: `float/tests/log.rs::test_log2_of_infinity`
Fix: `(+inf, +inf)` / `+inf` for `±inf` (`FBig`, `Repr`; `CachedFBig` delegates).

### with_precision turns a finite value into an infinity under an inward mode
Reports: F90
Verdict: real, fixed
Root cause: `Context::repr_round` → `rounded_to_repr` (`float/src/repr.rs:141`) canonicalizes
every rounded result at the exponent `isize::MAX` to an infinity, for the arithmetic kernels to
report as an overflow. `with_precision` (`float/src/convert.rs:255`) has no error channel and
returned it as the value, whatever the rounding mode.
Base behaviour: verified by running: identical (`Inexact(inf, NoOp)` under `Down` / `Zero`).
Provenance: **pre-existing**.
Test: `float/tests/convert.rs::test_precision_change_near_max_exponent` (6 modes, bases 2 / 10 /
16, precisions 1 / 20 / 50 / 100 / 500, both signs)
Fix: `Context::repr_round_in_range` (convert.rs) for `with_precision` and the same-base
`with_base_and_precision`: a result at or beyond `isize::MAX` saturates by
`overflow_repr_endpoint`, like an overflowing operation — the infinity under the modes that
round to nearest or away from zero on that side (unchanged), but under an inward mode the
rounded value itself while it still has an exponent, and the largest finite number of the
precision beyond that.
Notable: the report expects `1·2^(isize::MAX − 1)` as "the largest finite value"; the library's
largest finite value at precision 1 is `1·2^isize::MAX` (`unwrap_fp`), which is what is returned
and is still `≤ x`. Under an inward mode the rounded value is kept rather than replaced by the
endpoint when its exponent is exactly `isize::MAX`, because the endpoint `(B^p − 1)·B^isize::MAX`
can lie beyond the input (`5·2^(isize::MAX−2)` at two bits rounds down to `1·2^isize::MAX`; the
endpoint would be `3·2^isize::MAX`). The arithmetic has that flaw — verified by running:
`Context::<Down>::new(2).mul(5·2^(isize::MAX−2), 1)` is `Err(Overflow)`, and `unwrap_fp` turns
it into `3·2^isize::MAX`, above the exact product. It belongs to the arithmetic group and is not
changed here. The reported flag is the endpoint's (`AddOne` for `+inf`).

### ulp saturates half way through its exponent sum
Reports: F92
Verdict: real, fixed
Root cause: `FBig::ulp` / `ulp_lb` (`float/src/fbig.rs:338-343`, `:381-387`):
`exponent.saturating_add(digits).saturating_sub(precision)` clips at `isize::MAX` before the
precision is taken off.
Base behaviour: verified by running: identical (`10^(isize::MAX − 25)` instead of
`10^(isize::MAX − 13)`).
Provenance: **pre-existing**.
Test: `float/tests/ulp.rs::test_ulp_near_exponent_limits`
Fix: the sum is formed in `i128` and saturated once (`ulp_exponent`). An ulp below the range
still saturates at `isize::MIN`, as the existing unit test expects.

## Per-report table

| Report | Cluster | Verdict | Provenance |
|---|---|---|---|
| F7 | to_f32 / to_f64: base conversion overflows the binary exponent | real, fixed | mixture: pre-existing (base 16), masked (base 10, by D39 / E22) |
| F11 | `{:e}`: printed exponent overflows `isize` | real, fixed | pre-existing |
| F15 | `{:.0}` of `-0` prints `-1` under `Up` / `Away` | real, fixed | regression (`c90d537`) |
| F17 | postgres: more than 16383 significant digits refused | real, fixed | pre-existing |
| F18 | postgres: `dscale` above the server's limit | real, fixed | pre-existing |
| F23 | serde: human-readable `Repr` writes one character per unit of the exponent | real, fixed | pre-existing |
| F24 | rkyv: precision above `u32::MAX` truncated | real, fixed | pre-existing |
| F25 | serde: struct form accepts more digits than the precision; reserved exponents | real, fixed (digits) / not a bug (reserved exponent) | new-code (`622dbbf`, `fbc65cf`) |
| F27 | serde: a value at a reserved exponent is written as a literal that cannot be read | real, fixed | new-code (`fbc65cf`, `59a79d4`) |
| F37 | parse: trailing fractional zeros push the exponent below `isize::MIN` | real, fixed | incomplete-fix (E211 / E261) |
| F49 | fmt: infinities ignore width, fill, alignment and `+` | real, fixed | pre-existing |
| F50 | NumOrd between different bases builds the powers of the bases | real, fixed | pre-existing |
| F71 | UniformFBig returns `-0` | real, fixed | mixture: pre-existing (straddling range), regression (`[0, 0]`, `1812267`) |
| F77 | log2 of an infinity is `-inf` | real, fixed | pre-existing |
| F81 | UniformFBig cost grows with the exponent gap of the bounds | real, fixed | regression (`1812267`) |
| F90 | with_precision turns a finite value into an infinity under an inward mode | real, fixed | pre-existing |
| F92 | ulp saturates half way through its exponent sum | real, fixed | pre-existing |

Counts: 17 real (16 fixed in full, F25 fixed for the half that is a bug). Provenance:
pre-existing 10 (F11, F17, F18, F23, F24, F49, F50, F77, F90, F92), regression 2 (F15, F81),
new-code 2 (F25, F27), incomplete-fix 1 (F37), mixture 2 (F7: pre-existing + masked; F71:
pre-existing + regression).

## Changelog entries

`float/CHANGELOG.md`, under `## Unreleased`:

### Fix
- `to_f32` / `to_f64` no longer panic for a decimal or hexadecimal number whose exponent does not fit an `isize` once scaled to base 2 (`1e3000000000000000000`); they saturate like other out-of-range values.
- The scientific formats (`{:e}`, `{:E}`, `{:b}`, `{:o}`, `{:x}`) print the right exponent for a multi-digit significand next to `isize::MAX` instead of overflowing (a panic in debug builds, a negative exponent in release builds).
- `{:.0}` prints `-0` for a negative zero under the `Up` and `Away` rounding modes (it printed `-1`).
- `FBig::from_str` accepts a literal whose fractional trailing zeros take `cc - len(bbb)` below `isize::MIN` while the value itself is in range (`0.100e-9223372036854775806`).
- `EstimatedLog2::log2_bounds` / `log2_est` return `+inf` for the infinities (they returned the `-inf` of a zero).
- `FBig::ulp` / `ulp_lb` no longer return a too small value for a multi-digit significand next to the largest exponent.
- `FBig::with_precision` (and `with_base_and_precision` to the same base) no longer returns an infinity under a rounding mode that rounds toward zero on that side: a result that overflows saturates the way an overflowing operation does.
- serde: a nonzero number at a reserved exponent (`FBig::from_parts(2, isize::MAX - 1)`, or the saturated result of an overflow under an inward rounding mode) round-trips through the human-readable formats; it was written as a literal that the parser refuses.
- serde: the struct form of `FBig` is rejected when the significand has more digits than `precision + 1`.
- rkyv: a precision above `u32::MAX` is refused instead of being truncated to its low 32 bits (an error with rkyv 0.8, a panic with rkyv 0.7).
- postgres: `ToSql` no longer refuses numbers with more than 16383 significant digits; a number too large for the format is `ConversionError::OutOfBounds`.
- postgres: the encoded `dscale` never exceeds 16383, the largest scale the server accepts.
- rand: `UniformFBig` (and rand's `Uniform` / `gen_range`) never returns a negative zero.
- rand: the cost of a `UniformFBig` sample no longer grows with the distance between the exponents of the bounds.
- num-order: `NumOrd` between floats of different bases no longer takes time and memory proportional to the exponents.

### Change
- Infinities honor the width, fill, alignment, `+` and `0` flags in `Display` and the scientific formats (`{:+8}` gives `    +inf`).
- serde: the human-readable form of a `Repr` with a large exponent is a literal with a scale (`"1@1000"`) instead of the positional form; a nonzero number at a reserved exponent is written as a struct, for `Repr` as well as `FBig`. Both forms are read.
- postgres: a number that needs more than 16383 fractional digits is `ConversionError::OutOfBounds` (the limit was 65535), and the padding implied by a larger precision stops at 16383.

## Cross-group notes

Files outside this group's natural area, or shared:

- `float/src/repr.rs` (shared): two small edits. `Context::precision` gained the rkyv `with`
  attributes (F24), and one sentence of the `rounded_to_repr` doc comment that described
  `with_precision` was removed (F90). No code change to `repr_round` / `rounded_to_repr`.
- `float/src/log.rs` (exp/log group): the two `EstimatedLog2 for Repr` methods only (F77). No
  internal caller passes an infinity and relies on `-inf`; the float, complex and rational
  suites pass.
- `float/src/fbig.rs`: `ulp` / `ulp_lb` and a private helper (F92).
- `float/src/third_party/num_order.rs` now calls `Context::ln`, `mul`, `add`, `sub` (F50).
- `guide/src/io/print.md` and `guide-zh/src/io/print.md`: one sentence each (F49).
- `float/tests/postgres_types.rs`: two existing expectations from the E192 fix changed (dscale
  39999 and 65535 are now 16383), and `1e-65535` moved to the rejected list (F18).

For other groups:

- float-arith: `Context::unwrap_fp` returns `(B^p − 1)·B^isize::MAX` for an `Overflow` under an
  inward mode even when the correctly rounded result was representable at the exponent
  `isize::MAX` with a smaller significand, which is beyond the true value (see F90). Not
  changed here.
- The quadratic digit loop in `numeric_to_repr` (postgres `from_sql`) and `to_digits(10000)`
  make a 131072-digit NUMERIC take seconds in a debug build. Not a report; left alone.
- `fmt_round` (positional `Display`) of a value next to `isize::MAX` still computes
  `exp + len − 1` in `isize` when a width is given; reaching it means printing about 2^63 zeros,
  so it was left alone.

Not run: the `fuzz/` differentials, the Dylint house lints, `float/tests/postgres.rs` (needs a
database), the Python bindings, and an MSRV (1.68) build — the oldest installed toolchain is
1.81. The new code uses `i128`, `Ord::clamp`, `isize::unsigned_abs` and `usize::BITS`, all
stable before 1.68.
