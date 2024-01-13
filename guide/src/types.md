# Numeric Types

In `dashu` crates, there are standalone types for each kind of numbers with arbitrary precision, as listed below:

- `dashu_int::UBig` (alias `dashu::Natural`) represents unsigned integers (i.e. natural numbers).
- `dashu_int::IBig` (alias `dashu::Integer`) represents (signed) integers.
- `dashu_float::FBig` (alias `dashu::Real`) represents real numbers with floating point representation (`signficand * base ^ exponent`)
- `dashu_float::DBig` (alias `dashu::Decimal`) is a specialization of `FBig` with `base = 10`.
- `dashu_ratio::RBig` (alias `dashu::Rational`) represents rational numbers. It has a variant `dashu_ratio::Relaxed`, which also represents a rational number, but it doesn't enforce that the number is in the canonicalized form.

Common operations are implemented for all these numeric types, please refer to the other sections or the API docs for the usages.

## Word

A `dashu_int::Word` is an unsigned integer representing a native machine word. The size of a `Word` usually depends on the platform, for example, the `Word` is `u32` on 32-bit platforms. However, the behavior can be overriden by setting the `force_bits` config flag (e.g. add `--cfg force_bits="32"` to the environment variable `RUSTFLAGS`). Since this type is not consistant across platforms, be careful to use it when writing portable programs.

Moreover, there is another type `DoubleWord` representing an integer type with double the size of a `Word`. It's the maximum integer type that can fit in a `UBig` instance without heap allocation. It's also involved in some const constructors.

## Sign

A `dashu_base::Sign` is a **binary** enum to represent the sign of numbers. Due to effciency and clarity, the number zero will be categorized as `Sign::Positive`, even though it's mathematically unsigned. (Imagine if you store the sign in a ternary format, every number instance will have to pay an extra bit to store the sign, and extra branches to do operations.) To get a ternary representation, it's recommended to use the `.signum()` methods on the numeric types.

Convenient utilities related to the sign are provided with this enum. For example, you can get the sign of any primitive numbers or big numbers through the `dashu_base::Signed` trait, you can also multiply the sign by another sign. You can even multiply the sign with `core::cmp::Ordering`, this is very handy when you want to flip a comparison result based on the sign of operands, and this is widely used in the comparison implementations in `dashu`.

## Layout of `UBig`

The most fundamental type of the `dashu` libraries is the natural number `UBig`. The underlying representation of an `UBig` number is an array of `Word`s. What's special about `dashu` is that when it contains only one or two words, the words will be inlined and no heap allocation will happen. Furthermore, an `UBig` usually only occupies a stack space of 3 words when it's inlined (see the code for the details if you are interested). Thanks to special memory optimization in `dashu`, an `Option<UBig>` and even a `Option<IBig>` will also take only 3 words.

> Currently the memory layout of an `UBig` instance is not finalized, so don't rely on this by now. Besides, there will be no compatiblity guarantee for the memory layout between different versions. The memory layout will probably be stablized in a `v1.0.0` release.

## Layout of `FBig`

The layout of `FBig` (and `DBig`) is a little different from other types. An `FBig` instance contains a number representation `dashu_float::Repr` and a context `dashu_float::Context`. The context will be copied every time a new `FBig` is created based on it. The context currently contains the rounding information and the precision associated with this number. In future, it might also contains an `Rc` pointing to a cache for math constants (such as π). Therefore, if you don't want to store the additional context information, you can just store the `Repr` part of the `FBig`. The later operations on the `Repr` can be called with the associated methods of the `Context`, which all takes the reference to a `Repr` instance. However, this could lead to a little overhead in some cases.

# Auxiliary Types

Besides the numeric types defined in separate crates, there are some auxiliary types defined in the crate **dashu-base**.

## Sign

In `dashu`, the sign of the numbers are represented as an enum `dashu_base::Sign`. It only has two variants: `Positive` and `Negative`. Zero is considered as `Positive`. A `Sign` can be converted from a boolean value using `::from()`, where `true` is mapped to `Negative`.

To get the sign of a number, usually there is a `.sign()` method for the numeric types. For primitive integers, the sign can be retrieved with the `dashu_base::Signed` trait.

The type `Sign` also supports some operations, namely `Neg` and `Mul`. The sign can be flipped using `Neg` and it can be multiplied with another `Sign` or other numeric types to their signs.

## Approximation

The enum `Approximation` is another commonly used type in `dashu`. It's used when an operation can return inexact values (such as rounding and number conversion). The enum has two variants: `Exact` and `Inexact`, the latter one contains a error term for representing the sign or magnitude of the error caused by inexact operations.

When you have an `Approximation` instance, call `.value()`, `.value_ref()` or `unwrap()` to get the operation result, and call `.error()` to get the error term. This struct also support method to work in functional programming style, such as `.map()` and `.and_then()`.
