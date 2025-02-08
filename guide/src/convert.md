All the numeric types in `dashu` supports convenient conversions to other numeric types.

# Conversion between `dashu` Types

The following table listed how to convert between the types. `.xxx` means it's a method of the source type, `::xxx` means it's a (static) method of the target type.

<!-- TODO: add links to the API doc for each conversion method -->

| From | UBig                             | IBig                             | FBig                                    | DBig                                    | RBig                       |
|-------------|----------------------------------|----------------------------------|-----------------------------------------|-----------------------------------------|----------------------------|
| To UBig     | \                                | `::try_from()`; [`.unsigned_abs()`](https://docs.rs/dashu-int/latest/dashu_int/struct.IBig.html#method.unsigned_abs) | [`.to_int()`](https://docs.rs/dashu-float/latest/dashu_float/struct.FBig.html#method.to_int); [`.ceil()`](https://docs.rs/dashu-float/latest/dashu_float/struct.FBig.html#method.ceil); [`.floor()`](https://docs.rs/dashu-float/latest/dashu_float/struct.FBig.html#method.floor) | [`.to_int()`](https://docs.rs/dashu-float/latest/dashu_float/struct.FBig.html#method.to_int); [`.ceil()`](https://docs.rs/dashu-float/latest/dashu_float/struct.FBig.html#method.ceil); [`.floor()`](https://docs.rs/dashu-float/latest/dashu_float/struct.FBig.html#method.floor)      | `::try_from()`; `.to_int()` |
| To IBig     | `::from()`                        | \                                | [`.to_int()`](https://docs.rs/dashu-float/latest/dashu_float/struct.FBig.html#method.to_int); [`.ceil()`](https://docs.rs/dashu-float/latest/dashu_float/struct.FBig.html#method.ceil); [`.floor()`](https://docs.rs/dashu-float/latest/dashu_float/struct.FBig.html#method.floor) | [`.to_int()`](https://docs.rs/dashu-float/latest/dashu_float/struct.FBig.html#method.to_int); [`.ceil()`](https://docs.rs/dashu-float/latest/dashu_float/struct.FBig.html#method.ceil); [`.floor()`](https://docs.rs/dashu-float/latest/dashu_float/struct.FBig.html#method.floor)| `::try_from()`; `.to_int()` |
| To FBig     | `::from()`; [`Repr::convert_int()`](https://docs.rs/dashu-float/latest/dashu_float/struct.Context.html#method.convert_int) | `::from()`; [`Repr::convert_int()`](https://docs.rs/dashu-float/latest/dashu_float/struct.Context.html#method.convert_int) | \                                       | `.to_binary()`; `.with_base()`          |                            |
| To DBig     | `::from()`; [`Repr::convert_int()`](https://docs.rs/dashu-float/latest/dashu_float/struct.Context.html#method.convert_int) | `::from()`; [`Repr::convert_int()`](https://docs.rs/dashu-float/latest/dashu_float/struct.Context.html#method.convert_int) | `.to_decimal()`; `.with_base()`         | \                                       |                            |
| To RBig     | `::from()`                        | `::from()`                        | `::try_from()`; `::from_simplest_fbig()` | `::try_from()`; `::from_simplest_fbig()` | \                          |

Besides these conversion functions, a obvious way is to use the constructor and deconstructor. But when you still want to retain the source numbers, you might want to use the functions above.

# Conversion between Primitives

(Table 1 From uxx, ixx, fxx To XBig; Table 2 To uxx, ixx, fxx From XBig)

| From | unsigned integers `uxx` | signed integers `ixx` | floating point `fxx`
|-------------|----------------------------------|----------------------------------|-----------------------------------------|-----------------------------------------|----------------------------|
| To UBig| ...

# Conversion to FBig/DBig

Another useful conversion is `UBig::as_ibig()`. Due to the fact that `UBig` and `IBig` has the same memory layout, A `UBig` can be directed used as an `IBig` through this method. Similarly, `RBig::as_relaxed()` can be helpful when you want to use an `RBig` instance as an `dashu_ratio::Relaxed`. 

Aside from these methods designed for conversions, the constructors and destructors can also be used for the purpose of type conversion, especially from compound types to its parts. Please refer to the [Construction and Destruction](./construct.md#Construct_from_Parts) page for this approach.

# Conversion between Big Numbers and Primitives

All the numeric types in the `dashu` crates support conversion from and to primitive types.

To convert from primitive to big numbers:

| Dest\Src  | u* (e.g. u8) | i* (e.g. i8) | f* (e.g. f32) |
|-----------|--------------|--------------|---------------|
| UBig      | From         | TryFrom      | TryFrom       |
| IBig      | From         | From         | TryFrom       |
| FBig/DBig | From         | From         | TryFrom[^f]   |
| RBig      | From         | From         | TryFrom       |

> [^f]: The conversion from `f32`/`f64` to `FBig` is **only defined in base 2**, because the conversion is almost always lossy when the base is not a power of two. To convert from `f32`/`f64` to big floats with other bases (such as `DBig` with base 10), the conversion can be achieved by converting to base 2 first, and then use the `.with_base()` method to convert to other bases. By this way, the rounding during the conversion can be explicitly selected.

To convert from big numbers to primitive numbers:

| Src\Dest  | u* (e.g. u8) | i* (e.g. i8) | f* (e.g. f32)                      |
|-----------|--------------|--------------|------------------------------------|
| UBig      | TryInto      | TryInto      | TryInto/`.to_f*()`                 |
| IBig      | TryInto      | TryInto      | TryInto/`.to_f*()`                 |
| FBig/DBig | TryInto      | TryInto      | TryInto/`.to_f*()`                 |
| RBig      | TryInto      | TryInto      | TryInto/`.to_f*()`/`.to_f*_fast()` |

In the table above, `.to_f*()` denotes `.to_f32()` and `.to_f64()`, similarly `.to_f*_fast()` denotes `.to_f32_fast()` and `.to_f64_fast()`. The *fast* methods don't guarantee corrent rounding so that they can be faster. It's recommended to use the `.to_f*()` methods over the `TryFrom`/`TryInto` trait, because `.to_f*()` will not fail and it also returns the rounding direction during the conversion (i.e. the sign of the rounding error).

The conversions from and to primitive numbers are also implemented for the `dashu_float::Repr` type. Especially `.to_f32()` and `.to_f64()` are implemented which follows the default IEEE rounding mode.


# Conversion for FBig/DBig

The conversion methods of integers and rational are simple and intuitive. However, the conversion can get complex for floating point numbers. The float type in `dashu` supports arbitrary base and rounding mode, which results in even more conversion cases.

The most useful conversions for the float types may be `.to_binary()` and `.to_decimal()`. The former method converts the number to base-2, and you can round it or operate it before you finally convert it to the native float types `f32`/`f64`. It's useful because the conversions from/to `f32`/`f64` are only defined for base-2. The latter method converts the number to base-10, and then you can print the number in decimal digits. It's useful because the float numbers can be only printed in the **native base**. These two methods are essentially the special case of the `.with_base()` method introduced below. For more information about printing. please refer to the [Printing](./io.print.md) page.

## Conversion to different base / precision / rounding mode

To convert the float numbers to different base, precision or rounding mode, there are methods to support each of the functionalities respectively.

Specifically, the related methods of `FBig` includes:
- `.with_precision()`: Change the precision of the float number
- `.with_rounding()`: Change the rounding mode of the float number
- `.with_base()`: Change the base (radix) of the float number. This operation involves heavy computation, and the precision of the output is automatically determined.
- `.with_base_and_precision()`: Similar to `with_base()`, but you can decide the output precision before the conversion.

For how the precision is determined with `.with_base()` and `.with_base_and_precision()`, please refer to [the API docs](https://docs.rs/dashu-float/latest/dashu_float/struct.FBig.html#method.with_base) for details. 

# Conversion to third-party types

(list what types we support, and link to specific pages)
TODO: convert from UBig/IBig to FBig, the precision will be inferred. convert from FBig to UBig/IBig will round. Convert from/to native floats only with base 2, precision will be automatically determined.

## Conversion to RBig

TODO: `simplest_in()`, `simplest_from_*()`, `.nearest_in()`, `next_up()`, `next_down()`, etc.
