# Group `rational` (38 reports)

Area: `dashu-ratio` (`rational/`). All 38 reports were reproduced or confirmed against the
code at `a0fc535`; 27 distinct clusters, all real (two of them documentation-only), all fixed.
Line numbers under "Root cause" refer to the tree at the base commit.

Verification: the new tests were also run against the unfixed sources (base tree `e9e3741`
with only `rational/tests/` and `rational/Cargo.toml` copied over, debug build). Every test
named under a cluster failed there, with these exceptions: `io.rs::test_from_str_expanded_zero_with_large_exponent`
and `simplify.rs::test_farey_neighbours_large_limit` did not finish within 60 s (they are the
hang / out-of-memory reports E178 and E64); `io.rs::test_rbig_in_expanded_alternate_roundtrip`,
`simplify.rs::test_simplest_from_float_roundtrip` and `div.rs::test_inv` are supporting
properties that already passed. With the fixes the whole `dashu-ratio` suite passes in debug
and release, with default features, `--no-default-features` and `--all-features`
(`num_order.rs` additionally needs `--features dashu-int/num-order`), and the workspace
tests, `cargo fmt --all -- --check` and the workspace clippy command are clean. MSRV 1.68
could not be checked by building (no such toolchain here, and 1.81 cannot resolve the
lockfile); the new code was only reviewed for newer APIs.

## Clusters

### parse-zero-denominator
Reports: A11, D41, E76
Verdict: real, fixed
Root cause: `Repr::from_str_radix` / `from_str_with_radix_prefix` (`rational/src/parse.rs:10`, `:31`) built the `Repr` from the parsed denominator without checking it for zero. `RBig` then held `n/0` (`reduce` divides by `gcd(n, 0) = n`, so `-7/0` became `-1/0`), and `Relaxed` panicked on `trailing_zeros().unwrap()` in `reduce2`.
Test: `rational/tests/io.rs::test_from_str_zero_denominator`
Fix: both parsers return `ParseError::InvalidSyntax` for a zero denominator (`rational/src/parse.rs`). No new `ParseError` variant was added (the enum is exhaustive, a new variant would be a breaking change in `dashu-base`); the method docs state the error.

### expanded-parse-inner-sign
Reports: E126
Verdict: real, fixed
Root cause: `Repr::from_str_expanded` handed the integer / fraction / repetend runs to `UBig::from_str_radix`, which accepts a leading `+`, and counted the `+` as a digit (`rational/src/parse.rs:126`), so `".+1"` was `1/100`.
Test: `rational/tests/io.rs::test_from_str_expanded_sign_inside_literal`
Fix: a run that starts with `+` or `-` is rejected with `InvalidDigit` (this also rejects `"++1"` / `"-+1"`, which were accepted before).

### expanded-parse-empty-repetend
Reports: E262
Verdict: real, fixed
Root cause: a repetend holding only `_` separators passed the `is_empty()` check but has zero digits, so the denominator `(B^0 - 1) * B^p` was 0 and `reduce` turned the whole literal into 0 (`rational/src/parse.rs:104-133`).
Test: `rational/tests/io.rs::test_from_str_expanded_separator_only_repetend`
Fix: a repetend without digits returns `InvalidSyntax`, like `()`.

### expanded-parse-zero-huge-exponent
Reports: E178
Verdict: real, fixed
Root cause: the scale factor `radix^|exponent|` was built before looking at the value (`rational/src/parse.rs:165`), so `"0e-41643224064"` ran out of memory.
Test: `rational/tests/io.rs::test_from_str_expanded_zero_with_large_exponent`
Fix: a zero value returns before the scale factor is built.
Notable: the report also lists `"1e-9223372036854775807"`: a nonzero value with such an exponent is not representable (its denominator would need ~2^63 digits) and still panics with the integer capacity assertion, like `UBig::pow` does. Left as is.

### expanded-odd-radix-rounding
Reports: A10, B9, E138
Verdict: real, fixed
Root cause: `round_and_carry` (`rational/src/fmt/expanded.rs:193-199`) rounded on one guard digit with `extra * 2 >= radix`. In an odd radix the half-way point is `0.hhh…` with `h = (radix-1)/2`, so a guard digit `h` followed by anything larger is above half but was rounded down. Both the fixed and the scientific format used it.
Test: `rational/tests/io.rs::test_rbig_in_expanded_rounding_odd_radix` (fixed cases from all three reports plus an exhaustive half-ulp check over small fractions, radices 3–35, fixed and scientific)
Fix: the digit extraction returns the exact remainder after the requested digits and rounding is decided by `2 * rem >= den` (`rounds_up`); the scientific format scales the value so that the integer quotient holds the `prec + 1` significant digits and rounds on its remainder (`rational/src/fmt/expanded.rs`). Even radices print exactly what they printed before (round half up).

### expanded-alternate-long-terminating
Reports: A2, D11, E24
Verdict: real, fixed
Root cause: with `{:#}` a terminating expansion took the rounded branch and was cut to the default precision of 64 digits (`rational/src/fmt/expanded.rs:220-262`), contradicting the round-trip guarantee documented on `from_str_expanded`.
Test: `rational/tests/io.rs::test_rbig_in_expanded_alternate_long_terminating`, `::test_rbig_in_expanded_alternate_roundtrip`
Fix: with the alternate flag a finite expansion is printed in full, however long (finiteness is decided on the denominator when the per-digit loop exhausts its budget), still padded with zeros up to the precision.
Notable (behaviour change): this also applies with an explicit precision, e.g. `{:#.2}` of 1/8 is now `0.125` (was `0.13`). The precision was already ignored for repeating values in this mode (`{:#.2}` of 5/12 is `0.41(6)`), so `#` now consistently means "exact". Without `#` nothing changes.

### expanded-repetend-budget-edge
Reports: E240
Verdict: real, fixed
Root cause: the detection loop tested `frac_digits.len() < max_digits` before checking whether the current remainder had been seen (`rational/src/fmt/expanded.rs:64`), so a prefix + period of exactly the budget (128 by default, N+1 with `{:#.N}`) was missed.
Test: `rational/tests/io.rs::test_rbig_in_expanded_repetend_filling_budget`
Fix: `expand_exact` checks termination and the cycle before the budget.

### expanded-zero-exponent-marker
Reports: E17
Verdict: real, fixed
Root cause: the zero shortcut of `fmt_scientific` wrote the raw `e`/`E` (`rational/src/fmt/expanded.rs:305`) instead of the radix dependent marker.
Test: `rational/tests/io.rs::test_rbig_in_expanded_scientific_zero_marker`
Fix: zero uses `@` for every radix other than 10, like nonzero values.

### display-flags
Reports: E111
Verdict: real, fixed
Root cause: `Display` (and `Binary`/`Octal`/`LowerHex`/`UpperHex`) for `Repr` forwarded the caller's formatter to the numerator and to the denominator separately (`rational/src/fmt/mod.rs:19-28`, `:65-77`), so `+` and the width applied twice. `InRadix` instead dropped all flags.
Test: `rational/tests/io.rs::test_format_flags_apply_to_whole_fraction`
Fix: `fmt_fraction` writes one sign and pads the fraction as one item (`Formatter::pad_integral`; no buffer is allocated unless a width is given). The alternate flag (radix prefix / upper case) still applies to both components. `in_radix` now honours `+` and the width as well.

### float-try-from-large-integer
Reports: D22, E16
Verdict: real, fixed
Root cause: `f32/f64::try_from(RBig)` converted the numerator to the mantissa type of `FloatEncoding` with `try_into().unwrap()` (`rational/src/convert.rs:188`), which fails from 2^31 / 2^63 on although the bound check above it allows up to 2^128 / 2^1024.
Test: `rational/tests/convert.rs::test_float_try_from_large_values`
Fix: the lossless conversion is derived from the correctly rounded `to_f32` / `to_f64` (`Exact` → `Ok`, infinite → `OutOfBounds`, otherwise `LossOfPrecision`). `TryFrom<Relaxed>` no longer needs to canonicalize.

### to-float-fast-accuracy
Reports: E143, E233
Verdict: real, fixed
Root cause: `to_f32_fast` / `to_f64_fast` truncated the denominator to the mantissa width (24 / 53 bits, `rational/src/convert.rs:290`, `:339`); that alone is a relative error of up to one ulp, and the 25/54-bit quotient was then rounded twice (here and in `encode`). The documented bound is one unit in the last place.
Test: `rational/tests/convert.rs::test_to_float_fast_accuracy` (report inputs, a fixed-seed pseudo random sweep over operand sizes 1–400 bits, and the overflow / subnormal boundaries)
Fix: divide the 64 (128) leading bits of the numerator by the 32 (64) leading bits of the denominator — still one primitive division — and round the quotient once to the mantissa width. The operand truncation now costs less than 1/64 ulp, so the result is the correctly rounded one except right next to a rounding boundary. The leading bits are read from the words directly (no big shift, no allocation).

### relaxed-to-integer
Reports: E68
Verdict: real, fixed
Root cause: `TryFrom<Repr>` for integers tests `denominator.is_one()` (`rational/src/convert.rs:35-61`), valid only for a reduced fraction; `Relaxed` keeps common odd factors.
Test: `rational/tests/convert.rs::test_relaxed_to_integers`
Fix: the `Relaxed` conversions divide when the denominator is not one and succeed if the remainder is zero (`Repr::into_integer`). `RBig` conversions are unchanged.

### to-float-double-rounding
Reports: A12, D1, E35
Verdict: real, fixed
Root cause: `Repr::to_float` (`rational/src/third_party/dashu_float.rs:126-160`) produced a quotient of `precision` or `precision + 1` digits, rounded it to an integer with `R::round_ratio`, and then rounded that integer again to the precision in `Context::convert_int` — a double rounding (2.49 → 2.5 → 3).
Test: `rational/tests/dashu_float.rs::test_to_float_rounds_once`, `::test_to_float_correctly_rounded` (exact rational oracle, all six modes, bases 2/10 plus odd bases, binary precisions 20/50/100/500 and small ones)
Fix: when the quotient has `precision + 1` digits its last digit is folded into the remainder before the single rounding step.

### numord-zero-vs-float
Reports: D43, E32
Verdict: real, fixed
Root cause: `NumOrd<f32/f64> for Repr` estimates `log2` from bit lengths (`rational/src/third_party/num_order.rs:257`); a zero numerator has bit length 0, read as magnitude ~2^-1, so zero compared `Greater` than positive floats below 1/4.
Test: `rational/tests/num_order.rs::test_ord_zero_with_float`
Fix: a zero rational is decided by the sign of the float before the magnitude estimate.

### inv-zero
Reports: E28
Verdict: real, fixed
Root cause: `Inverse for Repr` swapped numerator and denominator unconditionally (`rational/src/div.rs:278`), producing `1/0`.
Test: `rational/tests/div.rs::test_inv`, `::test_inv_zero_rbig`, `::test_inv_zero_rbig_ref`, `::test_inv_zero_relaxed`, `::test_inv_zero_relaxed_ref`
Fix: panics with the usual division-by-zero message.

### log2-bounds
Reports: E260
Verdict: real, fixed
Root cause: `Repr::log2_bounds` subtracted the f32 bounds of numerator and denominator with round-to-nearest (`rational/src/repr.rs:144`), which can move a bound across the true value.
Test: `rational/tests/log.rs::test_log2_bounds_contain_log2` (new file)
Fix: the subtraction's rounding error is computed exactly (error-free transformation) and the bound steps outwards with `dashu_base::utils::next_down/next_up` only when it is inexact, so exact cases (`1/1024` → `(-10, -10)`) stay exact.

### num-traits-abs-sub
Reports: E101
Verdict: real, fixed
Root cause: `Signed::abs_sub` returned `|a - b|` (`rational/src/third_party/num_traits.rs:47`); num-traits defines it as the positive difference `max(a - b, 0)`.
Test: `rational/tests/num_traits.rs::signed_abs_sub_is_positive_difference`
Fix: returns zero when `self <= other`.

### num-traits-pow-usize
Reports: E208
Verdict: real, fixed
Root cause: `Pow<usize>` cast the exponent to `isize` (`rational/src/third_party/num_traits.rs:84`, `:91`); above `isize::MAX` it became negative and took the reciprocal path, panicking for a zero base.
Test: `rational/tests/num_traits.rs::pow_usize_beyond_isize_max`
Fix: `Repr::pow_unsigned(usize)`, used by `Pow<usize>` and by the non-negative branch of `pow`.

### zeroize
Reports: E116
Verdict: real, fixed
Root cause: `Zeroize for Repr` zeroized the denominator and left it at 0 (`rational/src/third_party/zeroize.rs:11`).
Test: `rational/tests/zeroize.rs::test_zeroize_gives_zero` (new file, `required-features = ["zeroize"]` in `rational/Cargo.toml`)
Fix: after wiping both buffers the denominator is set to one, so the value is a valid zero.

### simplicity-order
Reports: E47
Verdict: real, fixed
Root cause: `RBig::is_simpler_than` and-ed the three criteria (`rational/src/simplify.rs:120-124`) instead of applying them lexicographically as documented, so it was false whenever the denominators were equal or the signs did not differ in the right way (`1` vs `3/2`, `1` vs `-1`). `simplest_from_f32/f64/float` use it to consider included interval endpoints, which were therefore never chosen.
Test: `rational/tests/simplify.rs::test_is_simpler_than`, `rational/tests/dashu_float.rs::test_simplest_from_float_included_endpoint`
Fix: lexicographic comparison (denominator, numerator magnitude, sign). The `simplest_in` docs spell the order out.

### simplest-in-zero-endpoint
Reports: E74
Verdict: real, fixed
Root cause: `Repr::simplest_in` returned 0 whenever the signs of the endpoints differed (`rational/src/simplify.rs:15`); zero has a positive sign, so `(-1, 0)` returned the endpoint 0.
Test: `rational/tests/simplify.rs::test_simplest_in_zero_endpoint`, `::test_simplest_in_against_search` (all intervals with endpoints of denominator ≤ 9 in [-2, 2] against an enumeration)
Fix: a zero endpoint takes the sign of the other endpoint.

### simplest-from-f64-interval
Reports: E121
Verdict: real, fixed
Root cause: `impl_simplest_from_float!` built the interval as `(2n ± 1)/(2d)` from the exact fraction `n/d` of the float (`rational/src/simplify.rs:80-100`). That is ±1/2 ulp only when the float has a negative exponent; for integers from 2^53 (2^24) on it is ±1/2 instead of ±ulp/2. It also ignored that the gap below a power of two is half as wide.
Test: `rational/tests/simplify.rs::test_simplest_from_float_interval`, `::test_simplest_from_float_roundtrip` (the result converts back to the float with `to_f64`/`to_f32`; edge floats and a fixed-seed sweep of bit patterns)
Fix: the interval is built from the decoded mantissa and exponent, with the narrower lower side at powers of two (except at the least normal number).
Notable: the report expects `2^54 + 7` for `2^54 + 8`; the correct answer is `2^54 + 6`, the included midpoint (the mantissa of `2^54 + 8` is even, so the tie rounds to it; the test asserts `18014398509481990u64 as f64 == f`). Likewise `2^25 + 6` for the f32 case.

### simplest-from-float-power-of-base
Reports: E14
Verdict: real, fixed
Root cause: `RBig::simplest_from_float` took the rounding interval from `R::error_bounds(f)` (`rational/src/third_party/dashu_float.rs:206`), which is symmetric (`float/src/round.rs`): it does not account for the floats below a power of the base being `B` times denser, so results fell outside the true interval (10 at one digit → 6). In addition `mode::HalfEven::error_bounds` decides tie inclusion on bit 0 of the *stored* (trailing-zero stripped) significand (`float/src/round.rs:445`), which is not the parity of the precision-wide significand (1536 at 2 bits → 1280, which rounds to 1024).
Test: `rational/tests/dashu_float.rs::test_simplest_from_float_power_of_base`, `::test_simplest_from_float_roundtrip` (round trip through `to_float` for all modes, bases 2/10/3/7, precisions 1–3 with an exhaustive minimality check, and 20/50/100/500)
Fix: `simplest_from_float` no longer calls `error_bounds`. It locates each end of the rounding interval by rounding probe points (1/4, 1/2, 3/4 of the gap to the adjacent float, with the gap towards zero divided by `B` at a power of the base) with the now correct `to_float`, so the result rounds back to `f` under every mode by construction. Zero of either sign gives 0 and unlimited precision values are converted exactly.
Notable: this makes the function independent of the `ErrorBounds` bugs tracked by the float groups (E59 odd bases, E166 panic for `Up`/`Down` at unlimited precision — `simplest_from_float(&FBig::<Up, 2>::ONE)` now returns 1). The `R: ErrorBounds` bound is kept for API compatibility. `float/` was not touched; see cross-group notes.

### farey-linear-time
Reports: E64
Verdict: real, fixed
Root cause: `RBig::farey_neighbors` (`rational/src/simplify.rs:218-250`) walked the Stern–Brocot tree one mediant per iteration; for a value close to a fraction with a small denominator that is `O(limit)` iterations (`ZERO.next_down(1e9)`: 19 s in release).
Test: `rational/tests/simplify.rs::test_farey_neighbours_large_limit` (limits up to 10^100; would not terminate before the fix), `::test_farey_neighbours_against_search` (all limits ≤ 9 against an enumeration)
Fix: `farey_bounds` takes each run of same-side steps at once; the run lengths are the quotients of the Euclidean algorithm on the two cross differences, so the cost is logarithmic in the limit. It starts from `floor(x)` and `floor(x) + 1`, so the `trunc`/`fract` split is gone.

### farey-limit-one-assertion
Reports: E79
Verdict: real, fixed (debug-assertion only; release builds returned the right value)
Root cause: for a value already in the sequence, `next_up`/`next_down` searched for `x ± 1/limit²` (`rational/src/simplify.rs:321`, `:354`); with an integer and `limit = 1` that target has denominator 1, violating `debug_assert!(x.denominator() > limit)` at `:219`.
Test: `rational/tests/simplify.rs::test_farey_neighbours_of_integers`
Fix: `farey_bounds` returns the greatest element `<= x` and the least element `> x` directly (when the left bound reaches `x` the right bound runs to the largest denominator), so no perturbed target is needed. `next_down` is the mirror image of `next_up`.

### nearest-docs
Reports: E213
Verdict: docs
Root cause: the doc of `RBig::nearest` said the error is "less than `1/(2*limit)`" (it is reached at midpoints) and described the returned sign as that of `self - nearest` while the code, the doc example and the existing tests use `nearest - self`.
Test: `rational/tests/simplify.rs::test_farey_neighbours_of_integers` (midpoint cases), `::test_farey_neighbours_against_search` (inclusive bounds, tie goes to the lesser candidate)
Fix: docs corrected ("at most", sign of `self.nearest() - self`, the tie rule, `# Panics`).

### next-up-down-docs
Reports: E219
Verdict: docs
Root cause: the docs of `next_up` / `next_down` said "denominator less than `limit`" and "error less than `1/limit`"; both bounds are inclusive.
Test: `rational/tests/simplify.rs::test_farey_neighbours_of_integers`, `::test_farey_neighbours_against_search`
Fix: docs corrected, with an integer example.

## Per-report table

| Report | Cluster | Verdict |
|---|---|---|
| A2 | expanded-alternate-long-terminating | real, fixed |
| A10 | expanded-odd-radix-rounding | real, fixed |
| A11 | parse-zero-denominator | real, fixed |
| A12 | to-float-double-rounding | real, fixed |
| B9 | expanded-odd-radix-rounding | duplicate of A10 (real, fixed) |
| D1 | to-float-double-rounding | duplicate of A12 (real, fixed) |
| D11 | expanded-alternate-long-terminating | duplicate of A2 (real, fixed) |
| D22 | float-try-from-large-integer | real, fixed |
| D41 | parse-zero-denominator | duplicate of A11 (real, fixed) |
| D43 | numord-zero-vs-float | real, fixed |
| E14 | simplest-from-float-power-of-base | real, fixed |
| E16 | float-try-from-large-integer | duplicate of D22 (real, fixed) |
| E17 | expanded-zero-exponent-marker | real, fixed |
| E24 | expanded-alternate-long-terminating | duplicate of A2 (real, fixed) |
| E28 | inv-zero | real, fixed |
| E32 | numord-zero-vs-float | duplicate of D43 (real, fixed) |
| E35 | to-float-double-rounding | duplicate of A12 (real, fixed) |
| E47 | simplicity-order | real, fixed |
| E64 | farey-linear-time | real, fixed |
| E68 | relaxed-to-integer | real, fixed |
| E74 | simplest-in-zero-endpoint | real, fixed |
| E76 | parse-zero-denominator | duplicate of A11 (real, fixed) |
| E79 | farey-limit-one-assertion | real, fixed (debug builds only) |
| E101 | num-traits-abs-sub | real, fixed |
| E111 | display-flags | real, fixed |
| E116 | zeroize | real, fixed |
| E121 | simplest-from-f64-interval | real, fixed (expected value in the report is off by one) |
| E126 | expanded-parse-inner-sign | real, fixed |
| E138 | expanded-odd-radix-rounding | duplicate of A10 (real, fixed) |
| E143 | to-float-fast-accuracy | real, fixed |
| E178 | expanded-parse-zero-huge-exponent | real, fixed (zero literals; huge nonzero values still exceed capacity) |
| E208 | num-traits-pow-usize | real, fixed |
| E213 | nearest-docs | docs |
| E219 | next-up-down-docs | docs |
| E233 | to-float-fast-accuracy | duplicate of E143 (real, fixed) |
| E240 | expanded-repetend-budget-edge | real, fixed |
| E260 | log2-bounds | real, fixed |
| E262 | expanded-parse-empty-repetend | real, fixed |

## Changelog entries

`rational/CHANGELOG.md`, under `## Unreleased`:

```markdown
### Change
- With the alternate flag (`{:#}`), `in_expanded` now always prints a finite expansion in full, also when it is longer than the precision (`{:#.2}` of 1/8 is `0.125`, it was rounded to `0.13`). The precision only pads the output and sets the repetend detection budget in this mode.
- `Display`, `Binary`, `Octal`, `LowerHex`, `UpperHex` and `in_radix` of a fraction apply the `+` flag, the width, the fill and the alignment to the fraction as a whole (`{:+}` of 1/2 is `+1/2`, it was `+1/+2`).
- `RBig::simplest_from_float` determines the rounding interval from the rounding of `to_float` itself instead of `ErrorBounds::error_bounds`; numbers with unlimited precision are converted exactly.

### Fix
- `from_str`, `from_str_radix` and `from_str_with_radix_prefix` of `RBig` / `Relaxed` return `ParseError::InvalidSyntax` for a zero denominator (`"1/0"`) instead of an invalid `RBig` or a panic.
- `from_str_expanded` / `from_str_decimal` reject a sign after the radix point or inside the repetend (`".+1"` was read as 1/100), reject a repetend that holds only `_` separators (`"0.1(_)"` was read as 0), and no longer run out of memory on a zero literal with a huge exponent (`"0e-41643224064"`).
- `in_expanded` rounds correctly in odd radices (`{:.4}` of 1/7 in radix 3 is `0.0110`, it was `0.0102`), in both the fixed and the scientific format.
- `{:#}` of `in_expanded` detects a repetend whose non-repeating prefix and period fill the detection budget exactly, and prints finite expansions longer than 64 digits in full, so that the output reads back with `from_str_expanded`.
- The scientific format of `in_expanded` uses the `@` exponent marker for zero in non-decimal radices (`0@0`, it was `0e0`).
- `f32::try_from` / `f64::try_from` of `RBig` and `Relaxed` no longer panic for integers from 2^31 / 2^63 on.
- `to_f32_fast` / `to_f64_fast` are within one unit in the last place of the correctly rounded result, as documented (they could be 2 ulps off).
- Integer conversions (`IBig::try_from`, `UBig::try_from`, `i64::try_from`, …) of a `Relaxed` with an integer value succeed when the fraction is not in the lowest terms (`Relaxed::from_parts(3, 3)`).
- `RBig::to_float` / `Relaxed::to_float` round only once (2.49 at one decimal digit under `HalfAway` gave 3).
- `NumOrd` between a zero `RBig` / `Relaxed` and a positive `f32` / `f64` below 1/4 returns `Less` (it returned `Greater`).
- `Inverse::inv` of a zero `RBig` / `Relaxed` panics instead of returning `1/0`.
- `EstimatedLog2::log2_bounds` of `RBig` / `Relaxed` always contains the logarithm (the bounds of 3/1024 excluded it).
- `num_traits::Signed::abs_sub` returns the positive difference (zero when `self <= other`) instead of `|self - other|`.
- `num_traits::Pow<usize>` no longer treats exponents above `isize::MAX` as negative.
- `Zeroize` leaves a valid zero (`0/1`) instead of `0/0`.
- `RBig::is_simpler_than` applies its criteria (denominator, numerator, sign) in order, instead of requiring all of them at once.
- `RBig::simplest_in` no longer returns zero when zero is an endpoint of the interval (`simplest_in(-1, 0)` is `-1/2`).
- `RBig::simplest_from_f32` / `simplest_from_f64` use the correct rounding interval for floats with an ulp above 1 and at powers of two, and return an included endpoint when it is the simplest (`simplest_from_f64(2^54 + 8)` is `2^54 + 6`).
- `RBig::simplest_from_float` returns a number that rounds back to the float when its magnitude is a power of the base, and returns an included endpoint when it is the simplest.
- `RBig::nearest`, `next_up` and `next_down` run in time logarithmic in the limit (they took time proportional to it for values close to a simple fraction), and `next_up` / `next_down` no longer trip a debug assertion for an integer with `limit = 1`.
- Docs of `RBig::nearest`, `next_up` and `next_down`: the error bounds are inclusive, the denominator is at most `limit`, and the sign returned by `nearest` is that of `nearest - self`.
```

## Cross-group notes

- Files touched outside `rational/`: `guide/src/io/print.md` and `guide-zh/src/io/print.md` (one sentence each on the exact `{:#}` output). Nothing in `base/`, `integer/` or `float/` was modified.
- `rational/Cargo.toml` gained a `[[test]] name = "zeroize"` entry; `rational/tests/log.rs` and `rational/tests/zeroize.rs` are new files.
- E14 (float-convert area, not fixed here): `ErrorBounds::error_bounds` in `float/src/round.rs` is still wider than the true rounding preimage on the side towards zero when the magnitude is a power of the base (all modes with a nonzero bound on that side), and `mode::HalfEven::error_bounds` computes tie inclusion as `significand.bit(0)` of the stored, trailing-zero-stripped significand: inclusive for an odd full-width significand (wrong: ties round away from it) and, in base 2, effectively always inclusive. `Context`'s Ziv loop (`float/src/ziv.rs::contained`) relies on these bounds, so this is probably the root cause of float reports such as E45 ("results just below 0.9995") and related to E59 / E166. `simplest_from_float` was the only user in `rational/` and no longer depends on it, so whatever the float groups do to `error_bounds` cannot conflict with or double-correct this group's fix.
- `FloatEncoding::encode` (`base/src/bit.rs`, int-misc): not touched. `to_f32`/`to_f64` hand it a mantissa that already fits, and the rewritten `to_f*_fast` now round to the mantissa width themselves, so outside the subnormal range they no longer depend on its rounding. From reading the f32 normal branch (not verified by a test here), its sticky bit appears to skip bit 7 of the shifted mantissa (`round_bits = ((mantissa >> 7) & 0b110) | ((mantissa & 0x7f) != 0)`); int-misc is presumably fixing that.
- `UBig::from_str_radix("_", …)` and a leading `+` are accepted by the integer parser (int-misc area); `from_str_expanded` now guards against both itself.
- `ParseError` (in `base/`) has no dedicated variant for a zero denominator; `InvalidSyntax` is used. If a variant is added centrally, `rational/src/parse.rs` and `rational/tests/io.rs::test_from_str_zero_denominator` are the two places to update.
