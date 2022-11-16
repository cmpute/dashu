There are multiple ways to construct and deconstruct the numeric types, which are listed below. These constructors are used for directly compose the numbers from its components. To construct from alternative representations, please refer to the [Input and Output](./io/index.md) and [Conversion](convert.md) sections.

# Constants

For all the numeric types, there are several constants associated with the type. You can use them to construct an instance, or directly use them with binary operators. These constants includes:

- `UBig`: `::ZERO`, `::ONE`
- `IBig`: `::ZERO`, `::ONE`, `::NEG_ONE`
- `FBig`/`DBig`: `::ZERO`, `::ONE`, `::NEG_ONE`, `::INFINITY`, `::NEG_INFINITY`
- `RBig`: `::ZERO`, `::ONE`, `::NEG_ONE`

# Raw Constructor for `UBig`

For `UBig`, it can be constructed from a slice of [`Word`](./types.md#word)s, using the `::from_words()` method. The words must be arranged in little-endian order, i.e. the first word should represent the least significant part of the number. If then integer you want to construct is small, then you can also use the `::from_word()` and `::from_dword()` methods, which can be called from a `const` context.

To deconstruct a `UBig`, currently we don't support taking the ownership of the words stored in a `UBig`. You can only access them using the `.as_words()` method, which returns a reference to the words. In future, when the memory layout of the `UBig` is stablized, it's possible to add a deconstructor that giving the ownership of the word to prevent unnecessary copying.

# Construct from Parts

For other numeric types, they are usually composed by several parts. And you can construct them using the `::from_parts()` and `::from_parts_const()` method. The latter one can be called from a `const` context, but the size of the components is limited when using `::from_parts_const()`.

The components of different types are listed below:

- For `::from_parts()`
  - `IBig` = sign: `Sign` + magnitude: `UBig`
  - `FBig`/`DBig` = significand: `IBig` + exponent: `isize`
  - `RBig` = numerator: `IBig` + denominator: `UBig`
- For `::from_parts_const()`
  - `IBig` = sign: `Sign` + magnitude: `DoubleWord`
  - `FBig`/`DBig` = sign: `Sign` + significand: `DoubleWord` + exponent: `isize`
  - `RBig` = sign: `Sign` + numerator: `DoubleWord` + denomiator: `DoubleWord`

It's worth noting that, the constructors for `FBig` and `DBig` also determines the precision of the result floating numbers. A float number created from `::from_parts()` will have a precision of the digits in the magnitude (in the given radix). A float number created from `::from_parts_const()` will have a precision either inferred from the magnitude (same as `::from_parts()`) or from the argument `min_precision` of the method.

To deconstruct these numeric types, use the `::into_parts()` functions to get the components without copying. However for `FBig`/`DBig`, you might need to use the `.into_repr()` to get the underlying representation `Repr`, and then use the `.into_parts()` method of `Repr` to get the magnitude and mantissa.

`from_parts` and `into_parts` (`into_repr` for `FBig`)

# `dashu-macros`


