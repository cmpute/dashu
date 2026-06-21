# Changelog

## Unreleased

### Add
- Add `FBig::quantize(exp)` to round to the nearest multiple of `BASE^exp` (the dashu analog of Python's `Decimal.quantize()`), returning `Rounded<Self>` with the result precision set so that `ulp()` equals `BASE^exp`.
- Implement the cubic root (`CubicRoot` for `FBig`, `Context::cbrt`) and the general nth root (`FBig::nth_root`, `Context::nth_root`) with correct rounding, built on top of `UBig::nth_root`.
- Implement trigonometric functions (`sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `atan2`, `sin_cos`) for `FBig` and `Context<R>` ([#60](https://github.com/cmpute/dashu/pull/60)).
- Add π constant computation (`FBig::pi()` and `Context::pi()`) using the Chudnovsky algorithm with binary splitting ([#60](https://github.com/cmpute/dashu/pull/60)).
- Add `FpResult` enum to handle non-finite math operation results (NaN, Infinite, Overflow, Underflow) without panicking ([#60](https://github.com/cmpute/dashu/pull/60)).
- Add `panic_nan`, `panic_overflow`, `panic_underflow`, and `panic_infinite` helpers to the `error` module.
- Optional `rand_v09` (rand 0.9, MSRV 1.63) and `rand_v010` (rand 0.10, MSRV 1.85) features mirroring `rand_v08`, exposing the random-float distributions (`Uniform01`, `UniformFBig`) under those rand versions. The default `rand` feature remains `rand_v08`.
- The `rand` distributions and their sampling algorithms now live once in the version-agnostic `dashu_float::rand` module. The `rand_v08` / `rand_v09` / `rand_v010` modules are now private per-version trait bindings; access the distributions through `dashu_float::rand`.

### Fix
- Fix rounding issues in `to_32()` and `to_f64()` (fixes [#53](https://github.com/cmpute/dashu/issues/53) and [#56](https://github.com/cmpute/dashu/issues/56)).
- Fix `FBig::fract()` inflating context precision for values smaller than one.
- Fix `split_at_point_internal` using incorrect fractional scale for numbers smaller than one, causing incorrect rounding results.
- Fix severe-cancellation errors in `FBig`/`Context` subtraction (and effective subtraction via addition): when the smaller operand reached into the larger operand's significant digits, the limited-precision alignment path could collapse a genuinely small difference to the wrong value (e.g. `1.00 - 0.99999999` at precision 3 returned `0` instead of `1e-8`). Such cases now form the exact difference at full operand width and round once.
- Fix a spurious-ULP rounding error in `FBig`/`Context` addition and subtraction when the larger operand had fewer digits than the context precision and the smaller operand was negligible (e.g. `1 + 2^-100` at precision 10 returned `513·2^-9` instead of `1` under base-2 round-to-nearest). The negligible-operand sticky is now positioned at the operand's real magnitude instead of at `precision - digits`, so it can no longer land on a rounding tie.
- Fix severe-cancellation errors at the window-edge boundary in `FBig`/`Context` subtraction: the guard fired only on strict overlap, missing the exact-edge case where the smaller operand's top digit lands on the precision-window edge, so a subtraction could still collapse (e.g. `0.5 - 0.4375 = 0.0625` at precision 1 returned `0`). The guard now fires on `>=`, i.e. whenever the smaller operand reaches the window; mild borrows there now return the exact `precision+1`-digit value (the guard digit), consistent with the documented add/sub precision behavior.
- Fix `Context::sub` mis-rounding when the left operand is zero under the asymmetric directed rounding modes (`Up` = toward +∞, `Down` = toward −∞). The zero-left path rounded the right operand and then negated, but `round(-x) != -round(x)` for those modes, so `0 - b` could land one ULP off (truncated toward zero instead of rounded away). The negated operand is now rounded directly.

## 0.4.4

- Bump MSRV from 1.61 to 1.68.

## 0.4.3

- Mark `FBig::from_str_native` as deprecated.
- Implement `TryFrom<Repr>` and `TryFrom<FBig>` for primitive integers.
- Implement `TryFrom<Repr<2>>` and `TryFrom<FBig<_, 2>>` for primitive floats.
- Implement `From<UBig>` and `From<IBig>` for `Repr`.
- Implement `core::fmt::{Binary, Oct, LowerExp, UpperExp, LowerHex, UpperHex}` for `Repr`, `FBig` (some are limited to certain bases).

## 0.4.2

- Add `Repr::from_static_words` to support the `static_fbig!` and `static_dbig!` macros.
- Add `FBig::from_repr_const` to support create an `FBig` instance from repr in const context.
- Add conversion from `f32`/`f64` to `Repr<2>`.
- Implement `NumOrd` between `FBig` and primitive integers / floats. 
- Implement `AbsOrd` between `FBig` and `UBig`/`IBig`.
- Now the `Debug` output of `FBig` values will not contains the rounding mode information (when alternative flag is not set).

## 0.4.1

- Fix the termination criteria for `ln` and `exp` series ([#44](https://github.com/cmpute/dashu/issues/44)).
- Fix `powf` panicking when base is 0.

## 0.4.0

### Add

- Implement `num-order::NumOrd` between `FBig` and `UBig`/`IBig` and between `FBig` with different bases.
- Implement `num-order::NumHash` for `FBig` and `Repr`.
- Add `ErrorBounds` trait that calculate the rounding range for a floating point number.

### Change

- Now feature `num-traits` and `rand` are not enabled by default, feature `num-order` is enabled instead.
- The type of `Repr::BASE` is changed from `IBig` to `UBig`
- `UBig::square` and `IBig::square` are renamed to `sqr`.
- The implementation of square root is now implemented by the `dashu_base::SquareRoot` trait instead of a standalone method of `FBig`.
- The rounding behaviors of `FBig::to_decimal` and `FBig::to_binary` are changed for better ergonomics.
- The rounding behaviors of `FBig::to_f32` and `FBig::to_f64` now follow the mode specified by the type argument.

## 0.3.2

- The default precision for float numbers from `from_parts`/`From<UBig>`/`From<IBig>` are now based on the actual digits on the integers, rather than the digits after simplification. (#28)

## 0.3.1

- Implement `num_traits::{Zero, One, FromPrimitive, ToPrimitive, Num, Signed, Euclid, Pow}` for `FBig` (#19)
- Implement `rand::distributions::uniform::UniformSampler` for `FBig` through `crate::rand::UniformFBig`
- Implement `rand::distributions::{Open01, OpenClosed01, Standard}` for `FBig`
- Implement `dashu_base::Inverse` for `FBig`
- Implement `rand::distributions::uniform::SampleUniform` for `FBig`.
- Implement `serde::{Serialize, Deserialize}` for `FBig` and `Repr`
- Implement `Rem` trait for `FBig`
- Add support of random floating point numbers generation through `crate::rand::Uniform01` and `crate::rand::UniformFBig`.
- Add support for serialization from/to PostgreSQL arguments through `diesel::{deserialize::FromSql, serialize::ToSql}` and `postgres_types::{FromSql, ToSql}`.
- Add `from_str_native()` for `Repr`
- Add `to_f32()`, `to_f64()` for `Repr`, and these two methods supports all bases for both `Repr` and `FBig`.
- Add `to_int()` for `Repr`, which is equivalent to `FBig::trunc()`
- Add `TryFrom<FBig>` for `UBig` and `IBig`
- Add `round()` for `FBig`
- Add `rand_v08` and `num-traits_v02` feature flags to prevent breaking changes due to dependency updates in future 
- Re-export operation traits through the `ops` module.

## 0.3.0

### Add

- Conversion from FBig to `f32`/`f64` support subnormal values now.
- Add a `split_at_point()` function to `FBig`

## 0.2.1

- Implement `core::iter::{Sum, Product}` for `FBig`
- Implement `powf`, `sqrt` for `FBig`

## 0.2.0 (Initial release)

- Support basic arithmetic operations (`add`/`sub`/`mul`/`div`/`exp`/`ln`) and base conversion.

# Todo

## Roadmap to next version
- Support generating base math constants (E, Pi, SQRT2, etc.)
- Support operations with inf
- Create operations benchmark
- Benchmark against crates: rug, twofloat, num-bigfloat, rust_decimal, bigdecimal, scientific
- Implement more formatting traits
- Other math functions: sin/cos/tan/etc.
