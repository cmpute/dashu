# Basic Arithmetics

The standard arithmetic operators are implemented for all numeric types, for both owned and borrowed operands. The behavior of division and remainder differs by type.

## Integer Arithmetic

`UBig` and `IBig` support `+`, `-`, `*`, `/`, and `%`. Integer division rounds toward zero, and the remainder takes the sign of the dividend (the C/Rust convention). For Euclidean division (non-negative remainder) use the `DivRemEuclid` / `RemEuclid` traits from `dashu-base`; `DivRem` returns both quotient and remainder at once.

```rust
use dashu_int::IBig;

let b = IBig::from(-0x10ff);
let e = 2 * &b - 1; // mixes naturally with primitives
assert_eq!(e, IBig::from(-0x21ff));
```

## Float Arithmetic

`FBig`/`DBig` support `+`, `-`, `*`, `/` between values of the **same base and rounding mode** (mixed bases are a compile error by design). The result precision is `max(lhs.precision, rhs.precision)`, and each operation reports its inexactness through the two-layer API described in [Exponential and Logarithm](./exp_log.md). Infinities are terminal: `1/0` and `ln(0)` produce `±∞`, but feeding an infinity back into arithmetic is an error (`FpError::InfiniteInput`).

## Rational Arithmetic

`RBig` supports `+`, `-`, `*`, `/`. Division by zero panics. `Relaxed` performs the same operations without auto-reducing to lowest terms (faster for a chain of operations); call `canonicalize()` to reduce when needed.

## Complex Arithmetic

`CBig` supports the field operations `+`, `-`, `*`, `/`, plus `sqr` and `inv` (multiplicative inverse). Multiplication and division by a real `FBig` are also available as mixed-type operators. Multiplication and division use Smith's method with a guard digit and re-round, giving the same near-correctly-rounded guarantee as `dashu-float`'s transcendentals.

```rust
use dashu_cmplx::CBig;
use dashu_float::{FBig, round::mode::HalfAway};

type C = CBig<HalfAway, 10>;
let z = C::from_parts(FBig::from(3), FBig::from(4));
let sum = &z + &C::I; // (3+4i) + i = 3+5i
assert_eq!(sum.im().significand(), &5.into());
```

## Mixed-type arithmetic

There are **no implicit mixed-type operators** between different big-number kinds (e.g. `UBig + FBig` does not compile) — convert explicitly first (see [Conversion](../convert.md)).
