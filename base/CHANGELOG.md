# Changelog

## Unreleased

### Add
- `Approximation::value_with_exact()` — the value together with an `is_exact` flag (the error `E`
  is discarded). Used by dashu-float's Ball exact-tracking ops (`mul_tracking`/`add_tracking`/
  `sqrt_tracking`/`scale_int_tracking`) and `hypot` to report a zero radius for an all-exact chain.

## 0.6.0-rc.1

### Add
- `ParseError::InvalidSyntax` variant for structurally malformed input (e.g. an unclosed
  repeating group in a decimal literal, or multiple `/` separators in a rational). This is a
  breaking change for code that exhaustively matches `ParseError` without a wildcard arm.
  (Re-introduced after being held back from 0.5.1 to keep that release break-free.)

## 0.5.1

### Fix
- `BitTest::bit` on signed integers now reports the sign-bit position correctly. The masked
  value `self & (1 << position)` was compared with `> 0`, which is always false at the sign-bit
  position (`BITS-1`) because `1 << (BITS-1)` is `iN::MIN` (negative); it now compares with
  `!= 0`, so e.g. `(-18i32).bit(31)` correctly returns `true`. (Found by the `fuzz/tests/base`
  differential.)
- `FloatEncoding::encode` no longer produces `NaN` for very large positive exponents. The
  overflow-routing bound `top_bit = (BITS - leading_zeros) + exponent` was computed in `i16`,
  which wrapped for huge exponents and misrouted them away from the overflow branch (e.g.
  `f64::encode(256, 32759)` returned `NaN`); it is now computed in `i32` and correctly returns
  `Inexact(±INFINITY, …)`. (Found by the `fuzz/tests/base` differential.)

## 0.5.0

### Remove
- `AbsEq` trait (folded into `AbsOrd`; use `.abs_cmp(..).is_eq()`).

### Improve
- Documented all previously-undocumented public items (associated types/methods on the arithmetic
  traits `DivRem`/`DivRemEuclid`/`Gcd`/`ExtendedGcd`/`SquareRootRem`/`CubicRootRem`, `Abs`/
  `UnsignedAbs`/`AbsOrd`/`Signed`/`Inverse`/`SquareRoot`/`CubicRoot`, the `Approximation` methods,
  the `FloatEncoding` associated types, and the `Sign` variants) and enabled `#![deny(missing_docs)]`
  together with `clippy::dbg_macro`, `clippy::undocumented_unsafe_blocks`, and
  `clippy::let_underscore_must_use` as crate-level denies.

## 0.4.3

### Add
- `Sign::as_sign_str` helper for rendering a sign as `+`/`-`/`` (empty), used by the formatting traits.
- Fast path for integer `sqrt` using native `f64::sqrt` on small inputs (requires `std` feature).

## 0.4.2

- Fix `log2_bounds` to return strictly enclosing bounds.
- Bump MSRV from 1.61 to 1.68.

## 0.4.1

- Mark `AbsEq` as deprecated.
- Re-implement functions `next_up` and `next_down`, and expose them through the `utils` module.

## 0.4.0

### Add

- Add `is_positive()` and `is_negative()` to the `Signed` trait.

### Change

- `SquareRoot` and `CubicRoot` are moved to `dashu_base::math`.
- `AbsCmp` is renamed to `AbsOrd`.
- `FBig::square` and `Context::square` are renamed to `sqr`.

## 0.3.1

- Add trait `Inverse`.
- Implement `AbsCmp` and `AbsEq` for primitive types.

## 0.3.0

### Add

- Add trait `AbsCmp` and `AbsEq`
- Add trait `FloatEncoding` and implement it for `f32` and `f64`
- Add trait `Signed` and implement it for all signed primitive types
- Add conversion between `Sign` and `bool`
- Implement `Abs` for `f32` and `f64`
- Add types `error::{ConversionError, ParseError}` (originates from `dashu-int`)
- Add trait `SquareRoot`, `SquareRootRem`, `CubicRoot`, `CubicRootRem`
- Implement `EstimatedLog2` for `f32`, `f64` and signed integers

### Change

- `trailing_zeros` has been removed from the `BitTest` trait
- The definition of `BitTest::bit_len` has changed, and `BitTest` is now implemented for signed integers.

### Remove

- `Root` and `RootRem` are removed (use `SquareRoot`, `SquareRootRem`, etc. instead)

## 0.2.1

- Implement `RootRem` for `u8`, `u16`, `u32`
- Add trait `Root` and implement it for `u8`, `u16`, `u32`, `u64`, `u128`

## 0.2.0

- Add traits `Approximation`, `Sign` and `EstimatedLog2`.

## 0.1.1

- Fix the bug of the GCD algorithm.

## 0.1.0 (Initial release)

- including several common trait definitions.
