Dashu supports a complete set of conversions, including conversions among arbitrary precision types, and conversions between arbitrary precision types and primitive types.

Note that a general principle of implementations of `TryFrom` in `dashu` is that, `TryFrom` should succeed only when the conversion is lossless. Any precision loss during the conversion should cause the `TryFrom` to return an `Err`.

# Conversion among Types

Most of the time, you can use `From`/`Into`/`TryFrom`/`TryInto` to convert between these types. When the conversion is fallible, only `TryFrom` and `TryInto` will be implemented. Below is a table of conversions between arbitrary precision types using these traits, where the columns are source types, and rows are destination types.

| Dest\Src  | UBig | IBig    | FBig/DBig    | RBig        |
|-----------|------|---------|--------------|-------------|
| UBig      | \    | TryFrom | TryFrom      | TryFrom     |
| IBig      | From | \       | TryFrom      | TryFrom     |
| FBig/DBig | From | From    | \            | TryFrom[^a] |
| RBig      | From | From    | TryFrom[^a]  | \           |

> [^a]: To use the conversion between `RBig` and `FBig`, the optional feature `dashu-float` must be enabled for the `dashu-ratio` crate.

These conversions will only succeed when the conversion is exact (lossless) and in-range. For example, the conversion from a float number to an integer will fail, if the float number is infinite (will return `Err(ConversionError::OutOfBounds)`), or it has fractional parts (will return `Err(ConversionError::LossOfPrecision)`).

Nevertheless, there are other useful methods for **lossy** conversions:

| Src\Dest  | UBig              | IBig                                  | FBig/DBig         | RBig                         |
|-----------|-------------------|---------------------------------------|-------------------|------------------------------|
| UBig      | \                 | \                                     | \                 | \                            |
| IBig      | `.unsigned_abs()` | \                                     | \                 | \                            |
| FBig/DBig | \                 | `.to_int()`[^b]                       | ...[^c]           | `.simplest_from_float()`[^d] |
| RBig      | \                 | `.to_int()/.trunc()/.floor()/.ceil()` | `.to_float()`[^e] | \                            |

> - [^b] The methods `.ceil()`, `.floor()` and `.trunc()` of `FBig` doesn't return `IBig`, because when `FBig` is very large (with a high exponent), the `IBig` result can consume a great amout of memory, which is usually not a desirable behavior.
> - [^c] See the section *Conversion for FBig/DBig* below for this conversion.
> - [^d] See the section *Conversion from Floats to RBig* below for more approaches.
> - [^e] This method requires the `dashu-float` feature to be enabled for the crate `dashu-ratio`.

Another useful conversion is `UBig::as_ibig()`. Due to the fact that `UBig` and `IBig` has the same memory layout, A `UBig` can be directed used as an `IBig` through this method. Similarly, `RBig::as_relaxed()` can be helpful when you want to use an `RBig` instance as an `dashu_ratio::Relaxed`. 

Besides these methods designed for conversions, the constructors and destructors can also be used for the purpose of type conversion, especially from compound types to its parts. Please refer to the [Construction and Destruction](./construct.md#Construct_from_Parts) page for this approach.


## Conversion for FBig/DBig

TODO: `with_rounding`, `with_precision`, `with_base`, `to_binary`, `to_decimal`, etc.
(how precision is determined)

## Conversion from Floats to RBig

TODO: `simplest_in()`, `simplest_from_*()`, `.nearest_in()`, `next_up()`, `next_down()`, etc.

# Conversion between Big Numbers and Primitives

All the numeric types in the `dashu` crates support conversion from and to primitive types.

To convert from primitive to big numbers:

| Dest\Src  | u* (e.g. u8) | i* (e.g. i8) | f* (e.g. f32) |
|-----------|--------------|--------------|---------------|
| UBig      | From         | TryFrom      | TryFrom       |
| IBig      | From         | From         | TryFrom       |
| FBig/DBig | From         | From         | TryFrom*      |
| RBig      | From         | From         | TryFrom       |

> *: The conversion from `f32`/`f64` to `FBig` is **only defined in base 2**, because the conversion is almost always lossy when the base is not a power of two. To convert from `f32`/`f64` to big floats with other bases (such as `DBig` with base 10), the conversion can be achieved by converting to base 2 first, and then use the `.with_base()` method to convert to other bases. By this way, the rounding during the conversion can be explicitly selected.

To convert from big numbers to primitive numbers:

| Src\Dest  | u* (e.g. u8) | i* (e.g. i8) | f* (e.g. f32)                      |
|-----------|--------------|--------------|------------------------------------|
| UBig      | TryInto      | TryInto      | TryInto/`.to_f*()`                 |
| IBig      | TryInto      | TryInto      | TryInto/`.to_f*()`                 |
| FBig/DBig | TryInto      | TryInto      | TryInto/`.to_f*()`                 |
| RBig      | TryInto      | TryInto      | TryInto/`.to_f*()`/`.to_f*_fast()` |

In the table above, `.to_f*()` denotes `.to_f32()` and `.to_f64()`, similarly `.to_f*_fast()` denotes `.to_f32_fast()` and `.to_f64_fast()`. The *fast* methods don't guarantee corrent rounding so that they can be faster. It's recommended to use the `.to_f*()` methods over the `TryFrom`/`TryInto` trait, because `.to_f*()` will not fail and it also returns the rounding direction during the conversion (i.e. the sign of the rounding error).
