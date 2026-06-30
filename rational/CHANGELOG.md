# Changelog

## Unreleased

### Fix
- Fixed a broken intra-doc link to `Display` in `InRadix`'s docs (`core::fmt::Display`), surfaced by
  `cargo doc -D warnings`.
- (internal) The `in_expanded` formatting unit tests failed to compile under `no_std` (`cargo test --no-default-features`) because the `format!` macro was not imported; the test module now imports `alloc::format`.

### Change
- **(breaking)** `From<Repr> for FBig`, `From<RBig> for FBig`, and `From<Relaxed> for FBig` are now `TryFrom`. The conversion succeeds only when the value is exactly representable in the target base (every prime factor of the reduced denominator divides the base), otherwise it returns `ConversionError::LossOfPrecision`. For a correctly-rounded conversion use `RBig::to_float` / `Relaxed::to_float`.
- `in_radix`/`from_str_with_radix_prefix` now use `u8` for the radix (was `u32`), matching `dashu-int`.
- (internal) Expanded-position formatting is faster: the integer part uses `dashu_int::UBig::to_digits`, and the non-repetend fractional path now extracts digits in batches through a precomputed `dashu_int::fast_div::ConstDivisor` (one big-int division per word of digits, instead of one division per digit).

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
