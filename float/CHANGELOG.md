# Changelog

## Unreleased

### Remove
- Public `Repr::from_str_native` / `FBig::from_str_native` methods (now crate-private). Use the `core::str::FromStr` impl (`s.parse()` / `FBig::from_str`) instead; its docs now carry the full parsing format specification.

### Change
- **(breaking)** `FBig` human-readable serde now pads the serialized string with trailing zeros so its significant-digit count equals the context precision, letting precision round-trip (previously it was lost). The binary format already preserved precision.
- (internal) The PostgreSQL `NUMERIC` conversion now extracts base-10000 digits via `UBig::to_digits` instead of a per-digit `div_rem` loop.
- (internal) Trig argument reduction (`reduce_to_quadrant`) now recovers the quadrant integer via `IBig::try_from` instead of `to_int()`, since the rounded value is already an exact integer.

### Fix
- `IBig::try_from(FBig)` and `UBig::try_from(FBig)` now accept IEEE-754 signed zero (`-0`), returning `Ok(0)` instead of `Err(LossOfPrecision)`. Signed zero carries its sign in a `-1` exponent sentinel rather than the significand, so its integer value is plain `0`.

### Add
- Hyperbolic functions `sinh`, `cosh`, `tanh` and their inverses `asinh`, `acosh`, `atanh` on
  `Context`/`FBig`/`CachedFBig`. Built from cancellation-free `exp_m1`/`ln_1p` formulas with
  IEEE-754 special-value handling: signed zeros (`sinh(±0)=±0`), infinities as values
  (`sinh(±∞)=±∞`, `cosh(±∞)=+∞`, `tanh(±∞)=±1`, `asinh(±∞)=±∞`, `acosh(+∞)=+∞`), and domain
  errors for `acosh(x<1)` and `atanh(|x|>1)` (`atanh(±1)=±∞`).
- `FpError` now carries `Overflow(Sign)` and `Underflow(Sign)` variants. Repr-level arithmetic
  functions (`mul_finite_reprs`, `repr_div`, `sqr`, `cubic`, `exp_internal`, `powi`) detect
  exponent overflow/underflow and return these errors. At the `FBig`/`CachedFBig` convenience
  layer they are converted to signed infinity or signed zero, matching IEEE 754 overflow/underflow
  semantics. The `Context` layer returns the raw error, giving callers the choice.
- `exp` and `exp_m1` now accept infinite input, returning the IEEE-correct result (`exp(+inf) = +inf`,
  `exp(-inf) = +0`, `exp_m1(+inf) = +inf`, `exp_m1(-inf) = -1`).
- `ConstCache` now also caches the base-free `√10005` isqrt used by π (`ConstCache::pi`), so a
  repeat π call at the same or lower precision reuses it instead of recomputing. The isqrt is held
  as a base-free integer (`floor(√10005 · 2^bits)`, computed via Karatsuba `UBig::sqrt`) and folded
  into the π integer ratio, so no cross-base conversion is needed. `ConstCache::total_words()`
  counts it; `total_terms()` is unchanged (the isqrt isn't a series). Measured warm-π speedup:
  ~20× at 500 digits, ~1.3× at 5000.
- IEEE-754 signed zero (`-0`): operations now produce the sign of zero mandated by the standard
  (e.g. `1 / -inf = -0`, `sqrt(-0) = -0`, `ceil(-0) = -0`, cancellation under round-toward-negative).
  `+0` and `-0` compare equal; `-0.0` round-trips through `f32`/`f64`.
- `FpError` (`InfiniteInput`, `OutOfDomain`, `Indeterminate`) and `FpResult<T> = Result<Rounded<T>, FpError>`.
  Infinite *outputs* are returned as values inside `Ok` (`1/0 → +inf`, `ln(0) → -inf`, `exp(huge) → +inf`,
  `tan(π/2) → ±inf`); infinite *inputs* are `Err(InfiniteInput)` (making infinities terminal, which
  structurally avoids the NaN-producing indeterminate forms); domain errors (`0/0`, `sqrt(-x)`, `ln(-x)`,
  `asin(|x|>1)`, `pow(neg, non-int)`) are `Err`. The `FBig`/`CachedFBig` convenience layers panic on error.

### Change
- **Breaking (encoding):** infinities are re-encoded with sentinel exponents `isize::MAX`/`isize::MIN`
  (was `1`/`-1`), and `-0` is encoded at exponent `-1`. `normalize()` preserves these special values
  instead of clobbering them; `Repr`'s `PartialEq`/`Eq` are now manual so `+0 == -0`.
- **Breaking (result model):** `Context` arithmetic/transcendental/trig methods now return
  `FpResult<FBig<R, B>>` (= `Result<Rounded<FBig<R, B>>, FpError>`) instead of `Rounded<FBig<R, B>>`
  (arithmetic) / `FpResult<B>` (the old trig enum). The old `FpResult` enum is **removed** (replaced by
  the type alias). `FBig::tan`/`asin`/`acos`/`atan2` now return `Self` (panic on error) instead of the
  enum, matching the other trig methods.
- `atan2(±finite, +inf)` now returns the signed zero of `y` (now that signed zero is supported).
- `powf(±0, y)` returns the *positive* result (`+0` for `y > 0`, `+inf` for `y < 0`) — matching the
  common float-pow convention (a float exponent doesn't track parity). Use `powi` for the sign-correct
  result (`pow(-0, odd) = -0`).
- Removed the unused `panic_overflow`/`panic_underflow`/`panic_infinite`/`panic_power_negative_base`/
  `panic_root_negative` helpers (their conditions are now `FpError`s).

### Fix
- `exp(huge)` / `exp_m1(huge)` now return `+inf` (or `0` / `-1` for huge negative arguments) instead of
  panicking when the scaled exponent overflows `isize`; `powi` likewise returns `±inf`/`0` on
  astronomically large results.
- `exp` / `exp_m1` at high precision (≳ a few thousand digits) returned values wrong in the low bits.
  The series working precision was sized `p + O(log p)`, but the final `Bⁿ` powering amplifies the
  series' relative error by `Bⁿ`, so it must carry `≈ n ≈ √p` extra digits (cf. MPFR's
  `q = precy + 2·K + …`, `K ≈ √precy`). The working precision is now `p + 2n` (`n = 2^⌊log₂ p / 2⌋`)
  and the final powering runs at that same precision instead of `2p`. Verified correct against an
  independent pure-Taylor reference up to 8192 bits / 4000 digits; also faster (roughly half the
  multiply cost at large `p`).
- The `FBig` `+`/`-` operators now produce `-0` on exact cancellation under round-toward-negative
  (`Down`), matching `Context::add`/`sub` (previously the equal-exponent fast path yielded `+0`).
- `ShrAssign` (`>>=`) for `FBig` previously subtracted the shift amount twice; it now shifts exactly once.
- Trig functions (`sin`/`cos`/`tan`/`sin_cos`/`asin`/`acos`/`atan`/`atan2`) panicked on tiny negative
  inputs (e.g. `sin(-1e-30)`): `round()` of a value in `(-1, 0)` yields signed zero, whose exponent
  sentinel `IBig::try_from` rejected, hitting an `unreachable!` during argument reduction. The quadrant
  integer is now extracted via `to_int`, which tolerates the signed-zero encoding. Found by the new
  `trig_prop` property tests.

### Add
- Add the `ConstCache` type and the `CachedFBig` wrapper. `ConstCache` caches exact binary-splitting tree state for mathematical constants (π, ln2, ln10, ln(B)) so that repeated calls at increasing precision *extend* prior work instead of recomputing from scratch. `CachedFBig` is an `FBig` carrying a shared `Rc<RefCell<ConstCache>>` handle: its transcendental operations (`ln`, `exp`, `sin`/`cos`/…, `pi`, base conversion) thread that handle through the `Context` methods, reusing/extending the cached state. `Context` and `FBig` stay `Copy` + `Send` + `Sync` + `no_std` (so `static_fbig!`/`static_dbig!` keep working); only `CachedFBig` is `!Send + !Sync` (sharing state via `Rc<RefCell<..>>`). Because `Context` accepts `Option<&mut ConstCache>`, users can build `Arc<Mutex<ConstCache>>`-based variants too.
- Add the `ConstCache` type and the `CachedFBig` wrapper. `ConstCache` caches exact binary-splitting tree state for mathematical constants (π, ln2, ln10, ln(B)) so that repeated calls at increasing precision *extend* prior work instead of recomputing from scratch. `CachedFBig` is an `FBig` carrying a shared `Rc<RefCell<ConstCache>>` handle: its transcendental operations (`ln`, `exp`, `sin`/`cos`/…, `pi`, base conversion) thread that handle through the `Context` methods, reusing/extending the cached state. `Context` and `FBig` stay `Copy` + `Send` + `Sync` + `no_std` (so `static_fbig!`/`static_dbig!` keep working); only `CachedFBig` is `!Send + !Sync` (sharing state via `Rc<RefCell<..>>`). Because `Context` accepts `Option<&mut ConstCache>`, users can build `Arc<Mutex<ConstCache>>`-based variants too.
- Mixed operators for `CachedFBig`: it now supports binary operations with `FBig` and with all primitive integer types (`u8`–`usize`, `i8`–`isize`, `UBig`, `IBig`), in both directions. The cache handle from the `CachedFBig` operand is preserved. `From<FBig> for CachedFBig` and `From<CachedFBig> for FBig` are implemented for ergonomic conversion.
- `CachedFBig::cache()` provides read-only access to the shared `ConstCache`, with `ConstCache::total_terms()` and `total_words()` for cache size inspection, and `CachedFBig::clear_cache()` / `ConstCache::clear()` to free cached memory.

### Change
- **Breaking (low-level `Context` API):** the `Context` constant-source methods (`ln`, `ln_1p`, `exp`, `exp_m1`, `powf`, `pi`, `sin`, `cos`, `sin_cos`, `tan`, `asin`, `acos`, `atan`, `atan2`, and the internal `ln2`/`ln10`/`ln_base`/`convert_base`) now take an additional `cache: Option<&mut ConstCache>` parameter, threading an optional shared cache. The high-level `FBig` API is unchanged (it passes `None`).
- Removed the `MathCache` type (subsumed by `ConstCache`, which is now public with `&mut self` methods).
- `Context::iacoth` (used internally by `ln`) now evaluates the series with binary splitting instead of an iterative loop, reusing the shared `iacoth_bs` helper. This keeps `Q` at O(p) digits and improves high-precision performance; behavior is unchanged (pinned by existing fixtures).
- `iacoth_bs` now skips its first several leaves via a compile-time constant basecase (the `L(6)`/`L(9)`/`L(99)` initial blocks). The precomputed `(P, Q, T)` values are kept within `u32` so the constants are portable across `Word = u16`/`u32`/`u64` (the `DoubleWord` constructor is `const` on every configuration).

### Fix

- Replace `f64::ceil()` in `ConstCache`'s precision/bit helpers with a `no_std`-safe integer ceiling (`ceil_usize`). `f64::ceil` is `std`-only on the crate's MSRV and broke the workspace `--all-features --tests` build, where `dashu-float` is compiled without `std` as a dependency of `dashu-ratio`.

## 0.4.5

### Add
- Add `FBig::quantize(exp)` to round to the nearest multiple of `BASE^exp` (the dashu analog of Python's `Decimal.quantize()`), returning `Rounded<Self>` with the result precision set so that `ulp()` equals `BASE^exp`.
- Implement the cubic root (`CubicRoot` for `FBig`, `Context::cbrt`) and the general nth root (`FBig::nth_root`, `Context::nth_root`) with correct rounding, built on top of `UBig::nth_root`.
- Implement trigonometric functions (`sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `atan2`, `sin_cos`) for `FBig` and `Context<R>` ([#60](https://github.com/cmpute/dashu/pull/60)).
- Add π constant computation (`FBig::pi()` and `Context::pi()`) using the Chudnovsky algorithm with binary splitting ([#60](https://github.com/cmpute/dashu/pull/60)).
- Add `FpResult` enum to handle non-finite math operation results (NaN, Infinite, Overflow, Underflow) without panicking ([#60](https://github.com/cmpute/dashu/pull/60)).
- Add `panic_nan`, `panic_overflow`, `panic_underflow`, and `panic_infinite` helpers to the `error` module.
- Optional `rand_v09` (rand 0.9, MSRV 1.63) and `rand_v010` (rand 0.10, MSRV 1.85) features mirroring `rand_v08`. The default `rand` feature still maps to `rand_v08`.
- The random-float distributions (`Uniform01`, `UniformFBig`) and their sampling now live once in the version-agnostic `dashu_float::rand` module. The per-version modules are now private trait bindings.

### Fix
- Fix rounding issues in `to_f32()` and `to_f64()` ([#53](https://github.com/cmpute/dashu/issues/53), [#56](https://github.com/cmpute/dashu/issues/56)).
- Fix several rounding bugs in `FBig`/`Context` addition and subtraction: severe-cancellation collapse, spurious-ULP errors from negligible operands, the window-edge boundary, and `Context::sub` with a zero left operand under directed rounding modes.
- Fix `FBig::fract()` inflating context precision and `split_at_point_internal` using an incorrect fractional scale for values smaller than one.

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
