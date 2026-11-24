# Changelog

## Unreleased

### Change
- With the alternate flag (`{:#}`), `in_expanded` now always prints a finite expansion in full, also when it is longer than the precision (`{:#.2}` of 1/8 is `0.125`, it was rounded to `0.13`). The precision only pads the output and sets the repetend detection budget in this mode.
- `Display`, `Binary`, `Octal`, `LowerHex`, `UpperHex` and `in_radix` of a fraction apply the `+` flag, the width, the fill and the alignment to the fraction as a whole (`{:+}` of 1/2 is `+1/2`, it was `+1/+2`).
- `RBig::simplest_from_float` determines the rounding interval from the rounding of `to_float` itself instead of `ErrorBounds::error_bounds`; numbers with unlimited precision are converted exactly.

### Fix
- `RBig::simplest_from_float` no longer panics for unlimited-precision floats under `mode::Down`/`mode::Up`, and returns a rational that rounds back to the float in odd bases.
- serde: the struct form of `RBig` / `Relaxed` can be deserialized from formats that provide
  owned or transient field names (e.g. CBOR).
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

## 0.6.0

### Change
- **(breaking) `RBig::pow` / `Relaxed::pow` now take an `isize` exponent** (was `usize`). Negative
  exponents reciprocate first (`x.pow(-n) == 1/x^n`); a zero base with a negative exponent panics.
  Non-negative exponents behave as before; call sites passing a `usize` variable need an `as isize`
  cast.
- Removed the `rustversion` dependency; the unversioned `rand` / `rkyv` feature aliases now select the
  newest versions (`rand_v010` / `rkyv_v08`).

### Add
- rkyv 0.7 / 0.8 support (`rkyv_v07`/`rkyv_v08`) — derive-based archive.

### Fix
- `from_str` / `from_str_expanded` reject malformed input (multiple `/` separators, a bad repetend)
  with `ParseError::InvalidSyntax` instead of `InvalidDigit`.

## 0.5.1

### Add
- `RBig::from_str_expanded` / `Relaxed::from_str_expanded`: parse a positional-expansion
  string in any base 2–36 into a rational — the full inverse of `in_expanded(radix)`. Accepts
  fixed-point (`1.5`, `-.25`, `3.`), scientific, and repeating notation with the repetend
  parenthesized after the radix point (`0.1(6)` = 1/6, `0.(3)` = 1/3). The scientific marker is
  `e`/`E` for base 10 and `@` for every other base (`e`/`E` are themselves digits once the base
  reaches 15). Underscore separators are allowed in any digit run, and digits above 9 are
  case-insensitive `a`–`z`/`A`–`Z`. Terminating expansions round-trip through
  `format!("{:.N}", x.in_expanded(radix))`, and every rational (including repeating expansions)
  round-trips exactly through `format!("{:#}", x.in_expanded(radix))`. Structurally malformed
  input (an unclosed or empty repetend, or a repetend with no preceding radix point) returns
  `ParseError::InvalidDigit`; `radix` outside 2–36 panics, matching `in_expanded`.
- `RBig::from_str_decimal` / `Relaxed::from_str_decimal` are retained as convenience aliases
  for `from_str_expanded(src, 10)`.
- `num_traits::Pow<isize>` is now implemented for `RBig` and `Relaxed` (and their
  references). Negative exponents reciprocate first, so e.g. `(2/3).pow(-1isize) == 3/2`;
  a zero base raised to a negative power panics with divide-by-zero. The native
  `RBig::pow` / `Relaxed::pow` keep an unsigned (`usize`) exponent for now — widening them
  to `isize` is a planned v1.0 breaking change (see V1-ROADMAP).

### Fix
- Constructing or reducing an `RBig` with two large, similarly-sized operands no longer aborts with
  "internal error: not enough memory allocated". The underlying integer GCD scratchpad is now sized
  for the worst-case division reachable during reduction (see the `dashu-int` changelog).

## 0.5.0

### Fix
- `UBig::try_from(RBig)` (and `TryFrom<Repr> for UBig`) now tests the reduced denominator (a value is
  an integer iff its denominator is 1) instead of the numerator magnitude. Previously it returned
  `Ok(1)` for any value with numerator ±1 (e.g. `1/2` → `Ok(1)`) and `Err(LossOfPrecision)` for every
  integer whose magnitude wasn't 1 (e.g. `5` → `Err`); it now converts integers correctly and rejects
  non-integers.
- Fixed a broken intra-doc link to `Display` in `InRadix`'s docs (`core::fmt::Display`), surfaced by
  `cargo doc -D warnings`.
- (internal) The `in_expanded` formatting unit tests failed to compile under `no_std` (`cargo test --no-default-features`) because the `format!` macro was not imported; the test module now imports `alloc::format`.

### Change
- **(breaking)** `From<Repr> for FBig`, `From<RBig> for FBig`, and `From<Relaxed> for FBig` are now `TryFrom`. The conversion succeeds only when the value is exactly representable in the target base (every prime factor of the reduced denominator divides the base), otherwise it returns `ConversionError::LossOfPrecision`. For a correctly-rounded conversion use `RBig::to_float` / `Relaxed::to_float`.
- `in_radix`/`from_str_with_radix_prefix` now use `u8` for the radix (was `u32`), matching `dashu-int`.
- (internal) Expanded-position formatting is faster: the integer part uses `dashu_int::UBig::to_digits`, and the non-repetend fractional path now extracts digits in batches through a precomputed `dashu_int::fast_div::ConstDivisor` (one big-int division per word of digits, instead of one division per digit).

### Improve
- Documented the `fmt` and `fmt::expanded` modules and enabled `#![deny(missing_docs)]` together
  with `clippy::dbg_macro`, `clippy::undocumented_unsafe_blocks`, and `clippy::let_underscore_must_use`
  as crate-level denies.
- Trimmed the `RBig` rustdoc to a concise summary with a pointer to the user guide (and fixed an
  "an rational" → "a rational" typo).

## 0.4.4

### Fix

- Fix `RBig::to_f32` and `RBig::to_f64` to avoid double rounding when converting rationals to native floats.

## 0.4.3

### Add

- Implement `Binary`, `Octal`, `LowerHex`, and `UpperHex` formatting for `RBig` and `Relaxed`.
- Add `RBig::in_radix` and `Relaxed::in_radix` methods for formatting in arbitrary radices (2-36).
- Add `RBig::in_expanded` and `Relaxed::in_expanded` methods for printing the positional expansion of a rational number, with support for precision control, repetend display (`#` flag), and scientific notation (`e`/`E`).
- Optional `rand_v09` (rand 0.9, MSRV 1.63) and `rand_v010` (rand 0.10, MSRV 1.85) features mirroring `rand_v08`. The default `rand` feature still maps to `rand_v08`.
- The random-rational distributions (`Uniform01`, `UniformRBig`) and their sampling now live once in the version-agnostic `dashu_rational::rand` module. The per-version modules are now private trait bindings.

## 0.4.2

- Implement `Div<RBig>` and `Div<Relaxed>` for `UBig`/`IBig`.
- Bugfix of division between big rational and big integers.
- Bump MSRV from 1.61 to 1.68.

## 0.4.1

- Implement `AbsOrd` between `RBig`/`Relaxed` and `UBig`/`IBig`/`FBig`.
- Implement `NumOrd` between `RBig`/`Relaxed` and primitive integers / floats.
- Add `RBig::as_relaxed` to use an `RBig` as an `Relaxed` instance.
- Method `RBig::to_float` now will enforce the precision of the output even for zero ([#45](https://github.com/cmpute/dashu/issues/45)).

## 0.4.0

### Add

- Add `is_int` to `RBig`
- Implement `num-order::NumOrd` between `RBig`/`Relaxed` and `UBig`/`IBig`/`FBig`.
- Implement `num-order::NumHash` for `RBig` and `Relaxed`
- Implement `simpliest_from_float` for `RBig`.

### Change

- Now feature `num-traits` and `rand` are not enabled by default, feature `num-order` is enabled instead.
- Fix the bug in `is_one` of `RBig` and `Relaxed`.
- `RBig::square` and `Relaxed::square` are renamed to `sqr`

### Remove

- `PartialOrd` is no longer implemented between `RBig` and `Relaxed`. Please use `num-order::NumOrd` instead.

## 0.3.2

Fix the Bug in multiplication between `RBig` and `IBig`.

## 0.3.1

- Implement `Sum` and `Product` traits for `RBig` and `Relaxed`.
- Implement `Rem` trait for `RBig` and `Relaxed`.
- Implement `dashu_base::{Abs, Inverse, DivEuclid, RemEuclid, DivRemEuclid, EstimatedLog2}` traits for `RBig` and `Relaxed`.
- Implement `rand::distributions::uniform::SampleUniform` for `RBig`.
- Implement `serde::{Serialize, Deserialize}` for `RBig` and `Relaxed`
- Implement `num_traits::{Zero, One, Num, Signed, FromPrimitive, ToPrimitive, Pow, Euclid}` for `RBig` and `Relaxed`
- Add `cubic()`, `pow()` for `RBig` and `Relaxed`.
- Add `round()` for `RBig` and `Relaxed`.
- Add support of random rational numbers generation through `Uniform01` and `UniformRBig`.
- Add `rand_v08` and `num-traits_v02` feature flags to prevent breaking changes due to dependency updates in future 
- Fix the bug in number comparison.
- Re-export operation traits through the `ops` module.

## 0.3.0 (Initial release)

- Support basic arithmetic operations and numeric conversion.
- Support Diophatine approximations.
