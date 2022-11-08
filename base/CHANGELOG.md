# Changelog

## 0.3.0 (WIP)

### Add

- Add trait `AbsCmp` and `AbsEq`
- Add trait `FloatEncoding` and implement it for `f32` and `f64`
- Add trait `Signed` and implement it for all signed primitive types
- Add conversion between `Sign` and `bool`
- Implement `Abs` for `f32` and `f64`
- Add types `error::{ConversionError, ParseError}` (originates from `dashu-int`)
- Add trait `SquareRoot`, `SquareRootRem`, `CubicRoot`, `CubicRootRem`

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
