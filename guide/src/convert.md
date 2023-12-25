Dashu supports a complete set of conversions, including conversions among arbitrary precision types, and conversions between arbitrary precision types and primitive types.

# Conversion among Types

Most of the time, you can use `From`/`Into`/`TryFrom`/`TryInto` to convert between these types. When the conversion is fallible, only `TryFrom` and `TryInto` will be implemented. Below is a table of conversions between arbitrary precision types using these traits, where the columns are source types, and rows are destination types.

| Dest\Src  | UBig      | IBig            | FBig/DBig        | RBig             |
|-----------|-----------|-----------------|------------------|------------------|
| UBig      | \         | TryFrom/TryInto | TryFrom/TryInto  | TryFrom/TryInto  |
| IBig      | From/Into | \               | TryFrom/TryInto  | TryFrom/TryInto  |
| FBig/DBig | From/Into | From/Into       | \                | TryFrom/TryInto* |
| RBig      | From/Into | From/Into       | TryFrom/TryInto* | \                |

> *: To use the conversion between `RBig` and `FBig`, the optional feature `dashu-float` must be enabled for the `dashu-ratio` crate.

These conversions will only succeed when the conversion is exact (lossless) and in-range. For example the conversion from a float number to an integer will fail, if the float number is infinite (will return `Err(ConversionError::OutOfBounds)`), or it has fractional parts (will return `Err(ConversionError::LossOfPrecision)`).

Nevertheless, there are other useful methods for lossy conversions:
- `IBig` to `UBig`: `.unsigned_abs()` (from `dashu_base::UnsignedAbs`)
- `FBig`/`DBig` to `IBig`: `.to_int()`
- `RBig` to `IBig`: `.to_int()`, `.trunc()`, `.ceil()`, `.floor()`
- `RBig` to `FBig`/`FBig`: `.to_float()`
> The methods `.ceil()`, `.floor()` and `.trunc()` of `FBig` doesn't return `IBig`, because when `FBig` is very large (with a high exponent), the `IBig` result can consume a great amout of memory, which might not be a desirable behavior.

Another useful conversion that is worth mentioning is `UBig::as_ibig()`. Due to the fact that `UBig` and `IBig` has the same memory layout, A `UBig` can be directed used as an `IBig` through this method. Similarly, `RBig::as_relaxed()` can be helpful when you want to use an `RBig` instance as an `dashu_ratio::Relaxed`. 

Besides these methods designed for conversions, the constructors and destructors can also be used for the purpose of type conversion, especially from compound types to its parts. Please refer to the [Construction and Destruction](./construct.md#Construct_from_Parts) page for this approach.

# Conversion between Primitives



# Conversion for FBig/DBig

(how precision is determined)

# Conversion between FBig/RBig
