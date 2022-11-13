# Numeric Types

In `dashu` crates, there are standalone types for each kind of numbers with arbitrary precision, as listed below:

- `dashu_int::UBig` (alias `dashu::Natural`) represents unsigned integers (i.e. natural numbers).
- `dashu_int::IBig` (alias `dashu::Integer`) represents (signed) integers.
- `dashu_float::FBig` (alias `dashu::Real`) represents real numbers with floating point representation (`signficand * base ^ exponent`)
- `dashu_float::DBig` (alias `dashu::Decimal`) is a special case of `FBig` with `base = 10`.
- `dashu_ratio::RBig` (alias `dashu::Rational`) represents rational numbers.

Common operations are implemented for all these numeric types, please refer to the other sections or the API docs for the usages.

# Auxiliary Types

Besides the numeric types defined in separate crates, there are some auxiliary types defined in the crate **dashu-base**.

## Sign

In `dashu`, the sign of the numbers are represented as an enum `dashu_base::Sign`. It only has two variants: `Positive` and `Negative`. Zero is considered as `Positive`. A `Sign` can be converted from a boolean value using `::from()`, where `true` is mapped to `Negative`.

To get the sign of a number, usually there is a `.sign()` method for the numeric types. For primitive integers, the sign can be retrieved with the `dashu_base::Signed` trait.

The type `Sign` also supports some operations, namely `Neg` and `Mul`. The sign can be flipped using `Neg` and it can be multiplied with another `Sign` or other numeric types to their signs.

## Approximation

The enum `Approximation` is another commonly used type in `dashu`. It's used when an operation can return inexact values (such as rounding and number conversion). The enum has two variants: `Exact` and `Inexact`. 

# Memory Layout

...
