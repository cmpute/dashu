Dashu supports a complete set of conversions, including conversions among arbitrary precision types, and conversions between arbitrary precision types and primitive types.

Note that a general principle of implementations of `TryFrom` in `dashu` is that, `TryFrom` should succeed only when the conversion is lossless. Any precision loss during the conversion should cause the `TryFrom` to return an `Err`.

## Conversion among Types

Most of the time, you can use `From`/`Into`/`TryFrom`/`TryInto` to convert between these types. When the conversion is fallible, only `TryFrom` and `TryInto` will be implemented. Below is a table of conversions between arbitrary precision types using these traits, where the columns are source types, and rows are destination types.

| Dest\Src  | UBig | IBig    | FBig/DBig    | RBig        | CBig    |
|-----------|------|---------|--------------|-------------|---------|
| UBig      | \    | TryFrom | TryFrom      | TryFrom     | TryFrom |
| IBig      | From | \       | TryFrom      | TryFrom     | TryFrom |
| FBig/DBig | From | From    | \            | TryFrom[^a] | TryFrom |
| RBig      | From | From    | TryFrom[^a]  | \           | —       |
| CBig      | From | From    | From         | —           | \       |

> [^a]: To use the conversion between `RBig` and `FBig`, the optional feature `dashu-float` must be enabled for the `dashu-ratio` crate.

These conversions will only succeed when the conversion is exact (lossless) and in-range. For example, the conversion from a float number to an integer will fail, if the float number is infinite (will return `Err(ConversionError::OutOfBounds)`), or it has fractional parts (will return `Err(ConversionError::LossOfPrecision)`).

Nevertheless, there are other useful methods for **lossy** conversions:

| Src\Dest  | UBig              | IBig                                  | FBig/DBig         | RBig                         |
|-----------|-------------------|---------------------------------------|-------------------|------------------------------|
| UBig      | \                 | \                                     | \                 | \                            |
| IBig      | `.unsigned_abs()` | \                                     | \                 | \                            |
| FBig/DBig | \                 | `.to_int()`[^b]                       | ...[^c]           | `.simplest_from_float()`[^d] |
| RBig      | \                 | `.to_int()/.trunc()/.floor()/.ceil()` | `.to_float()`[^e] | \                            |

> - [^b] The methods `.ceil()`, `.floor()` and `.trunc()` of `FBig` doesn't return `IBig`, because when `FBig` is very large (with a high exponent), the `IBig` result can consume a great amount of memory, which is usually not a desirable behavior.
> - [^c] See the section *Conversion for FBig/DBig* below for this conversion.
> - [^d] See the section *Conversion from Floats to RBig* below for more approaches.
> - [^e] This method requires the `dashu-float` feature to be enabled for the crate `dashu-ratio`.

Another useful conversion is `UBig::as_ibig()`. Due to the fact that `UBig` and `IBig` has the same memory layout, A `UBig` can be directly used as an `IBig` through this method. Similarly, `RBig::as_relaxed()` can be helpful when you want to use an `RBig` instance as an `dashu::rational::Relaxed`. 

Besides these methods designed for conversions, the constructors and destructors can also be used for the purpose of type conversion, especially from compound types to its parts. Please refer to the [Construction and Destruction](./construct.md#construct-from-parts) page for this approach.

## Conversion between Big Numbers and Primitives

All the numeric types in the `dashu` crates support conversion from and to primitive types.

To convert from primitive to big numbers:

| Dest\Src  | u* (e.g. u8) | i* (e.g. i8) | f* (e.g. f32) |
|-----------|--------------|--------------|---------------|
| UBig      | From         | TryFrom      | TryFrom       |
| IBig      | From         | From         | TryFrom       |
| FBig/DBig | From         | From         | TryFrom*      |
| RBig      | From         | From         | TryFrom       |
| CBig      | From         | From         | TryFrom*      |

> *: The conversion from `f32`/`f64` to `FBig`/`CBig` is **only defined in base 2**, because the conversion is almost always lossy when the base is not a power of two. To convert from `f32`/`f64` to big floats with other bases (such as `DBig` with base 10), the conversion can be achieved by converting to base 2 first, and then use the `.with_base()` method to convert to other bases. By this way, the rounding during the conversion can be explicitly selected.

To convert from big numbers to primitive numbers:

| Src\Dest  | u* (e.g. u8) | i* (e.g. i8) | f* (e.g. f32)                      |
|-----------|--------------|--------------|------------------------------------|
| UBig      | TryInto      | TryInto      | TryInto/`.to_f*()`                 |
| IBig      | TryInto      | TryInto      | TryInto/`.to_f*()`                 |
| FBig/DBig | TryInto      | TryInto      | TryInto/`.to_f*()`                 |
| RBig      | TryInto      | TryInto      | TryInto/`.to_f*()`/`.to_f*_fast()` |
| CBig      | TryInto      | TryInto      | TryInto                            |

In the table above, `.to_f*()` denotes `.to_f32()` and `.to_f64()`, similarly `.to_f*_fast()` denotes `.to_f32_fast()` and `.to_f64_fast()`. The *fast* methods don't guarantee correct rounding so that they can be faster. It's recommended to use the `.to_f*()` methods over the `TryFrom`/`TryInto` trait, because `.to_f*()` will not fail and it also returns the rounding direction during the conversion (i.e. the sign of the rounding error). (`CBig` has no `.to_f*()` methods — its only float-conversion path is `TryInto`, which is base-2.)

The conversions from and to primitive numbers are also implemented for the `dashu::float::Repr` type. Especially `.to_f32()` and `.to_f64()` are implemented which follows the default IEEE rounding mode.


### Conversion for FBig/DBig

Conversions involving `FBig`/`DBig` are richer than for the integer types, because a floating-point number carries three independent knobs: a **base**, a **precision** (a cap on the number of significant digits), and a **rounding mode**. Most conversions therefore come in two flavors — an infallible `From`/`Into` when no information is lost, and a fallible `TryFrom`/`TryInto` when exactness is required.

### Conversion to different base / precision / rounding mode

The base, precision, and rounding mode are changed independently:

- `with_rounding::<NewR>()` reinterprets the same value under a different rounding mode — the underlying representation is unchanged, only the context's rounding field moves, so no rounding occurs.
- `with_precision(p)` widens or shrinks the significand to `p` digits. Widening is always exact (`Approximation::Exact`); shrinking rounds per `R` and returns `Approximation::Inexact` carrying the rounding direction.

```rust
use dashu::base::Approximation::*;
use dashu::float::DBig;
use dashu::float::round::Rounding::*;

let a = DBig::from_str("2.345")?;
assert_eq!(a.precision(), 4);
assert_eq!(a.clone().with_precision(3), Inexact(DBig::from_str("2.35")?, AddOne));
assert_eq!(a.clone().with_precision(5), Exact(DBig::from_str("2.345")?));
```

- `with_base::<NewB>()` converts to a different base. The result precision is chosen so the significand cap is no larger than before — the largest integer $p'$ with $\mathrm{NewB}^{\,p'} \le B^{\,p}$. Conversion is exact when one base is a power of the other; otherwise it rounds per `R`. `with_base_and_precision::<NewB>(p)` lets you set the target precision explicitly.

For the common binary ↔ decimal hops, two shortcuts pick the rounding mode for you: `to_decimal()` is `with_rounding::<HalfAway>().with_base::<10>()` (yielding a `DBig`), and `to_binary()` is `with_rounding::<Zero>().with_base::<2>()`.

> These methods panic if the associated context has **unlimited precision** and the conversion cannot be done losslessly — set a precision first.

### Conversion to integers or primitive floats

Converting *into* `FBig` from `UBig`/`IBig` (or any primitive integer) infers the precision from the magnitude: the result precision equals the number of significant base-`B` digits of the integer.

Going the other way, `TryFrom<FBig> for IBig`/`UBig` succeeds only when the float is finite and exactly integer-valued — `ConversionError::OutOfBounds` for infinities, `LossOfPrecision` for a fractional part. For a rounding-aware path use `to_int()`, which always succeeds and reports the rounding direction:

```rust
use dashu::base::Approximation::*;
use dashu::float::DBig;
use dashu::float::round::Rounding::*;

assert_eq!(DBig::from_str("1234")?.to_int(), Exact(1234.into()));
assert_eq!(DBig::from_str("1.234")?.to_int(), Inexact(1.into(), NoOp));
```

To a primitive float, `to_f32()` / `to_f64()` return `Rounded<f32>` / `Rounded<f64>` carrying the rounding direction; they never fail (at the edges of the format the result depends on the rounding mode — see [range boundaries](#range-boundaries-overflow-underflow-and-the-rounding-mode) below; infinities map to infinities). The reverse — `TryFrom<f32>`/`TryFrom<f64> for FBig` — is **base-2 only** (it is almost always lossy in any other base); to reach a non-binary `FBig`, convert to base 2 first and then call `with_base()`. NaN is rejected with `ConversionError::OutOfBounds`.

### Range boundaries: overflow, underflow, and the rounding mode

`to_f32()` / `to_f64()` round under the context's mode all the way to the edge of the format, so when the true value falls outside the finite range the rounding mode decides the *endpoint*:

- **Overflow** (value beyond `f32::MAX` / `f64::MAX`): the outward modes (toward `±∞`, away from zero) reach `±∞`; toward-zero, toward the opposite infinity, and nearest saturate to the largest *finite* value (`±MAX`).
- **Underflow** (value below the smallest subnormal — `2⁻¹⁴⁹` for f32, `2⁻¹⁰⁷⁴` for f64): the outward modes reach the smallest subnormal of that sign; the rest round to signed `±0`.

```rust
use dashu::integer::IBig;
use dashu::float::{FBig, round::mode::{Zero, Up}};

// 3·2^127 overflows f32::MAX: toward-zero saturates to f32::MAX, toward +∞ reaches +∞.
let big = FBig::<Zero, 2>::from_parts(IBig::from(3), 127);
assert_eq!(big.to_f32().value(), f32::MAX);
assert!(big.with_rounding::<Up>().to_f32().value().is_infinite());

// 2^-160 is below the smallest f32 subnormal: toward +∞ reaches it, toward-zero gives +0.
let tiny = FBig::<Zero, 2>::from_parts(IBig::from(1), -160);
assert_eq!(tiny.with_rounding::<Up>().to_f32().value(), f32::from_bits(1));
assert_eq!(tiny.to_f32().value(), 0.0);
```

`TryFrom<FBig> for f32` / `f64`, by contrast, is the *fallible* path: it returns `Err` for **any** inexact conversion (in-range rounding included) — so prefer `to_f32()` / `to_f64()` when you want the value. Its error variant is classified by the *input magnitude*, not the mode-aware result: a value beyond `±MAX` is `Err(ConversionError::OutOfBounds)` under **every** rounding mode (whether that mode would saturate to `±MAX` or overflow to `±∞`), while an in-range value that merely loses precision is `Err(LossOfPrecision)`. Thus `Err(OutOfBounds)` reliably means "beyond the finite range", independent of the rounding mode:

```rust
use dashu::base::ConversionError;
use dashu::integer::IBig;
use dashu::float::{FBig, round::mode::{Zero, Up}};

let big = FBig::<Zero, 2>::from_parts(IBig::from(3), 127);
// Beyond f32::MAX is out of range under every mode — the mode only picks
// the endpoint that to_f32() would return (±MAX vs ±∞).
assert_eq!(f32::try_from(big.clone().with_rounding::<Up>()), Err(ConversionError::OutOfBounds));
assert_eq!(f32::try_from(big), Err(ConversionError::OutOfBounds));
```

### Conversion to RBig

With the optional `dashu-float` feature enabled on `dashu-ratio`, `TryFrom<FBig> for RBig` succeeds only when the float is exactly rational-representable, and `RBig::to_float()` is the rounding-aware path in the other direction.

For approximating a float by a *simple* rational (the smallest numerator/denominator within a tolerance), use `simplest_from_f32` / `simplest_from_f64`, which search the float's own rounding interval. On `RBig`, `simplest_in(lower, upper)` finds the simplest rational inside an explicit interval, and `nearest(&limit)` / `next_up(&limit)` / `next_down(&limit)` find the rational(s) closest to a value with a denominator bounded by `limit` (Farey-sequence neighbors). `is_simpler_than(other)` compares two `RBig`s directly by the same simplicity ordering.
