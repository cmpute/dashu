# Changelog

## Unreleased

### Add
- `Repr::new_const`: a `const`-evaluable, normalized `Repr` constructor from a `DoubleWord`
  significand (the `const` counterpart of `Repr::new`). `FBig::from_parts_const` now delegates to
  it, and the complex literal macro uses it.
- `Repr::is_int` is now `const` (it only inspects the exponent and the infinity sentinel).
- **Exact `Add`/`Sub`/`Mul` operators for `Repr`** (in a new `repr_ops` module). A `Repr` carries no
  precision limit, so add/sub/mul on it are lossless — these are the shared primitives the crate uses
  for exact intermediates (the Ziv containment test, the correctly-rounded `Sum`, and the `FBig`
  multiply path). `Mul` saturates exponent overflow/underflow to the signed infinity/zero sentinels,
  so the operator is infallible; the internal `make_mul_repr`/`unwrap_mul_repr!` helpers are removed
  in favor of the operator. No behavior change to `FBig` arithmetic.
- **Guaranteed-correct rounding for `exp`, `exp_m1`, `ln`, `ln_1p`** via a Ziv retry loop. A generic
  `Context::ziv` driver rounds the working-precision approximation to the target precision and
  verifies, against the `ErrorBounds` rounding preimage, that the approximation's provable error
  interval lies entirely inside one rounding bin; if not, it retries with more guard digits. The
  loop provably terminates and preserves the `Exact`/`Inexact` flag. Series evaluation is factored
  into near-correct `ln_compute`/`exp_compute` cores (`R: Round`) that the Ziv wrapper certifies.
- **Tightened `exp` guard digits** (now a performance knob, since Ziv — not the guard count —
  guarantees correctness): the `Bⁿ`-powering guard is halved from `2n` to `n` and the series guard
  drops its conservative `+ 2`.
- **Guaranteed-correct rounding for most remaining transcendentals** via the Ziv loop: the
  trigonometric family (`sin`, `cos`, `sin_cos`, `tan`, `asin`, `acos`, `atan`, `atan2`), the
  hyperbolic family (`sinh`, `cosh`, `sinh_cosh`, `tanh`, `asinh`, `acosh`, `atanh`), and `hypot`.
  The trig series (`sin`/`cos`/`atan`)
  are factored into near-correct `_compute` cores like `exp_compute`; the composition-based functions
  treat the now-Ziv-correct `exp`/`ln`/`atan` as black boxes and count only their arithmetic. The
  trig argument reduction folds a `|k|·ulp(π/2)` reduction-error term into the radius so the
  containment test stays sound for huge `|x|`. `ziv_pair` certifies both halves of `sin_cos`/
  `sinh_cosh`.
- **Guaranteed-correct rounding for `powf` (non-integer exponent)** via the Ziv loop. `x^y =
  exp(y·ln x)`, and `exp` amplifies the rounding of `y·ln x` by the result magnitude — so the
  radius is `result.ulp()·(|y·ln x|+1)·(B+8)` taken at the *working* precision: it shrinks as
  `B^{-guard}`, so the containment test converges (a radius computed at unlimited precision would
  be constant across retries and never settle for a value near a rounding boundary). An
  integer-valued exponent now delegates to `powi` (binary exponentiation), which also admits a
  negative base — its sign fixed by the exponent's parity — so `powf(-x, n)` is in domain for
  integer `n`. That integer-exponent path stays within 1 ulp (near-correct), matching `powi`.

### Change
- **(breaking, bound)** `Context::exp`/`exp_m1`/`ln`/`ln_1p` (and `powf`, the hyperbolic family,
  and the base-conversion path that derives from them) now require `R: ErrorBounds` rather than
  `R: Round` — the Ziv containment test needs the rounding preimage that `ErrorBounds` provides.
  All six built-in modes satisfy `ErrorBounds`; only custom non-`ErrorBounds` `Round` modes are
  affected (custom modes are already discouraged). `CachedFBig`/`CachedCBig` forwarders for these
  methods carry the same bound. Arithmetic, roots, trigonometric, and `powi` remain `R: Round`.

### Fix
- **Directed-rounding hang on exact results** (`acos(1)`, `acos(-1)`, `asin(0)`): under a directed
  mode (Down/Up/Zero) a transcendental whose true value is exactly representable carries a positive
  radius that can't be certified against the value's one-sided rounding preimage, so the Ziv
  containment test infinite-retried (and the retry eventually tripped a `dashu-int` NTT assertion).
  `acos` now short-circuits `|x| = 1` to the exact `0` / `π`, and `asin` short-circuits `x = 0` to
  `±0` — matching the existing zero/`|x|=1` special cases in the rest of the inverse trig/hyperbolic
  family. (`hypot` of an exact Pythagorean triple, e.g. `hypot(3,4) = 5`, still hits this under
  directed rounding — it has no clean single special case; the underlying `dashu-int` NTT crash is
  tracked separately.)

## 0.5.0

### Add
- **IEEE-754 signed zero (`-0`)**: operations now produce the sign of zero mandated by the standard
  (e.g. `1 / -inf = -0`, `sqrt(-0) = -0`, `ceil(-0) = -0`, cancellation under round-toward-negative).
  `+0` and `-0` compare equal; `-0.0` round-trips through `f32`/`f64`.
- **New error model**: `FpError` (`InfiniteInput`, `OutOfDomain`, `Indeterminate`, and new
  `Overflow(Sign)`/`Underflow(Sign)`) with `FpResult<T> = Result<Rounded<T>, FpError>`. Infinite
  *outputs* are values inside `Ok` (`1/0 → +inf`, `ln(0) → -inf`, `exp(huge) → +inf`); infinite
  *inputs* are `Err(InfiniteInput)` (structurally avoiding NaN-producing indeterminate forms); domain
  errors (`0/0`, `sqrt(-x)`, `ln(-x)`, `asin(|x|>1)`) are `Err`. The `FBig`/`CachedFBig` convenience
  layers panic on error and saturate `Overflow`/`Underflow` to signed infinity/zero.
- **`ConstCache` + `CachedFBig`**: `ConstCache` caches exact binary-splitting tree state for constants
  (π, ln2, ln10, ln(B) — including the base-free `√10005` isqrt that feeds π) so repeated calls at
  increasing precision *extend* prior work instead of recomputing. `CachedFBig` is an `FBig` carrying a
  shared `Rc<RefCell<ConstCache>>` handle; its transcendentals (`ln`, `exp`, `sin`/`cos`/…, `pi`, base
  conversion) thread that handle through `Context`. `Context`/`FBig` stay `Copy` + `Send` + `Sync` +
  `no_std`; only `CachedFBig` is `!Send + !Sync`. `CachedFBig::cache()`/`clear_cache()` and
  `ConstCache::total_terms()`/`total_words()` inspect/free cached memory.
- **Hyperbolic functions** `sinh`/`cosh`/`tanh`/`asinh`/`acosh`/`atanh` on `Context`/`FBig`/`CachedFBig`,
  built from cancellation-free `exp_m1`/`ln_1p` formulas with IEEE special-value handling.
- **`FBig::hypot`** / `Context::hypot`: overflow/underflow-safe `sqrt(a² + b²)` via the scaled
  sum-of-squares (the larger operand is never squared).
- **`FBig::sinh_cosh`** / `Context::sinh_cosh`: combined `sinh`+`cosh` sharing the `exp_m1(±x)` work.
- `exp`/`exp_m1` now accept infinite input (`exp(+inf) = +inf`, `exp(-inf) = +0`, `exp_m1(-inf) = -1`).
- `CachedFBig` now mirrors `FBig`'s full trait surface — formatting, ordering, conversions, shift and
  root/euclid ops, `Sum`/`Product` — plus the reference-operand variants of its binary operators and
  mixed ops with `FBig` and the integer primitives. Third-party traits (serde/num-traits/num-order/
  rand/zeroize/postgres) are reached via `.as_fbig()`.
- `Repr::num_hash_residue` (behind `num-order`), exposed so composite types can combine their parts'
  residues algebraically.

### Change
- **(breaking)** `Repr::is_zero` is renamed to `Repr::is_pos_zero` (it tests only `+0`); use
  `significand().is_zero()` to detect either signed zero. `num_traits::Zero::is_zero` for `FBig` now
  returns `true` for either.
- **(breaking)** `Sum` for `FBig` is now correctly-rounded: addends are accumulated exactly and the
  total rounded once (MPFR `mpfr_sum` semantics). The generic `Sum<T>`/`Product<T>` impls are replaced
  by concrete `Sum`/`Sum<&FBig>`/`Product`/`Product<&FBig>`; cross-type sums (e.g. `Sum<u8>`) require
  converting the elements first.
- **(breaking)** `FBig` human-readable serde now pads the serialized string to the context precision's
  digit count so precision round-trips (the binary format already preserved it).
- **(breaking, encoding)** infinities are re-encoded with sentinel exponents `isize::MAX`/`isize::MIN`
  and `-0` at exponent `-1`; `normalize()` preserves these, and `Repr`'s `PartialEq`/`Eq` are manual
  so `+0 == -0`.
- **(breaking, result model)** `Context` arithmetic/transcendental/trig methods now return
  `FpResult<FBig<R, B>>` instead of `Rounded<FBig<R, B>>` (arithmetic) / the old trig `FpResult` enum.
  `FBig::tan`/`asin`/`acos`/`atan2` now return `Self` (panic on error), matching the other trig methods.
- **(breaking, low-level)** `Context` constant-source methods take an additional
  `cache: Option<&mut ConstCache>` parameter; the high-level `FBig` API is unchanged (passes `None`).
- `atan2(±finite, +inf)` returns the signed zero of `y`; `powf(±0, y)` returns the *positive* result
  (`+0` for `y > 0`, `+inf` for `y < 0`) — use `powi` for the sign-correct `pow(-0, odd) = -0`.

### Remove
- Public `Repr::from_str_native` / `FBig::from_str_native` (now crate-private — use `s.parse()`).
- The old `FpResult` enum and the `MathCache` type (subsumed by the public `ConstCache`).
- The `panic_overflow`/`panic_underflow`/`panic_infinite`/`panic_power_negative_base`/`panic_root_negative`
  helpers (their conditions are now `FpError`s).

### Fix
- Signed-zero correctness: `exp_m1(-0) = -0`; `powf(base, -0) = 1`; `quantize(-0)` preserves the sign;
  `+`/`-` produce `-0` on exact cancellation under `Down`; `Sum` cancellation to zero yields `+0` (or
  `-0` only under roundTowardNegative); `IBig`/`UBig::try_from(FBig)` accept `-0`.
- `NumOrd` against a primitive `0.0`/`-0.0` now reports either signed zero as `Equal`.
- `error_bounds` honors the `ErrorBounds` contract for unlimited precision (`Away` returns
  `(0, 0, true, true)`), and `HalfEven` gives `-0` the one-sided preimage (matching `Zero`/`HalfAway`).
- `Context::asin`/`acos` no longer panic on `±1` under `Down` (the `1 - x²` → `-0` → `sqrt(-0)` path).
- `exp`/`exp_m1`/`powi` return `±inf`/`0` on astronomically large results instead of panicking.
- `exp`/`exp_m1` at high precision (≳ a few thousand digits) were wrong in the low bits — the series
  working precision now carries `≈ √p` extra guard digits to absorb the `Bⁿ` final-powering error
  amplification (cf. MPFR's `K ≈ √precy`).
- `ShrAssign` (`>>=`) previously subtracted the shift twice.
- Trig functions no longer panic on tiny negative inputs (`sin(-1e-30)`): the signed-zero encoding no
  longer trips argument reduction.
- Broken intra-doc links surfaced by `cargo doc -D warnings`; `f64::ceil` in `ConstCache` replaced
  with a `no_std`-safe integer ceiling.
- `FBig::from_repr`'s debug assertion now accepts the documented single guard digit.

### Improve
- Documented the `math::trig` module and enabled `#![deny(missing_docs)]` together with
  `clippy::dbg_macro`, `clippy::undocumented_unsafe_blocks`, and `clippy::let_underscore_must_use`
  as crate-level denies.
- Migrated the verbose `FBig` type prose out of the rustdoc and into the user guide, leaving a concise
  summary with guide links; the runnable `# Examples` are kept verbatim.
- (internal) `Context::iacoth` and the `ConstCache` π path use binary splitting; the PostgreSQL
  `NUMERIC` conversion and trig argument reduction use `UBig::to_digits` / `IBig::try_from`.

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
