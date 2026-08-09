The standard arithmetic operators are implemented for all numeric types, for both owned and borrowed operands. The behavior of division and remainder differs by type.

## Integer Arithmetic

`UBig` and `IBig` support `+`, `-`, `*`, `/`, and `%`. Integer division rounds toward zero, and the remainder takes the sign of the dividend (the C/Rust convention). For Euclidean division (non-negative remainder) use the `DivRemEuclid` / `RemEuclid` traits from `dashu-base`; `DivRem` returns both quotient and remainder at once.

```rust
use dashu::integer::IBig;

let b = IBig::from(-0x10ff);
let e = 2 * &b - 1; // mixes naturally with primitives
assert_eq!(e, IBig::from(-0x21ff));
```

## Float Arithmetic

`FBig`/`DBig` support `+`, `-`, `*`, `/` between values of the **same base and rounding mode** (mixed bases are a compile error by design). The result precision is `max(lhs.precision, rhs.precision)`, and each operation reports its inexactness through the two-layer API described in [Power, Exponential and Logarithm](./pow_exp_log.md). Infinities are terminal: `1/0` and `ln(0)` produce `±∞`, but feeding an infinity back into arithmetic is an error (`FpError::InfiniteInput`).

Beyond the four field operations, floats can be decomposed and rounded with `trunc()`,
`ceil()`, `floor()`, `round()` (nearest, ties away from zero), and `fract()` (the fractional
part); `split_at_point()` returns both halves as `(integral, fractional)` in one call, and
`quantize(exp)` rounds to the nearest multiple of `BASE^exp` (the dashu analog of Python's
`Decimal.quantize`, returning the rounding direction). Each of these adjusts the result
precision to the digits that remain, so the value is exact for its stored digits. The unit in
the last place is `ulp()`, with a cheaper lower bound `ulp_lb()` that is useful as a
negligibility threshold in iterative algorithms.

```rust
use core::str::FromStr;
use dashu::float::DBig;

let a = DBig::from_str("1.234")?;
assert_eq!(a.round().to_string(), "1");
assert_eq!(a.fract().to_string(), "0.234");
let (int, frac) = a.clone().split_at_point();
assert_eq!(int.to_string(), "1");
assert_eq!(frac.to_string(), "0.234");
```

## Rational Arithmetic

`RBig` supports `+`, `-`, `*`, `/`. Division by zero panics. `Relaxed` performs the same operations without auto-reducing to lowest terms (faster for a chain of operations); call `canonicalize()` to reduce when needed. `RBig` also has `fract()` for the fractional part, `is_int()` to test whether the value is an integer, and `relax()` — the consuming variant of `as_relaxed()` (see [Conversion](../convert.md)) that hands the value to a `Relaxed` without copying.

## Complex Arithmetic

`CBig` supports the field operations `+`, `-`, `*`, `/`, plus `sqr` and `inv` (multiplicative inverse). Multiplication and division by a real `FBig` are also available as mixed-type operators. Complex multiplication and division are near-correctly rounded via a guard-digit re-round (division uses Smith's overflow-safe method). `conj()` gives the complex conjugate and `mul_i(negative)` multiplies by `+i` or `-i` — an exact rotation that swaps the real and imaginary parts. The components are exposed as raw representations through `re()` / `im()` (borrowing `&Repr`), or as owned `FBig`s through `into_parts()` (see [Construction and Destruction](../construct.md)).

```rust
use dashu::complex::CBig;
use dashu::float::{FBig, round::mode::HalfAway};

type C = CBig<HalfAway, 10>;
let z = C::from_parts(FBig::from(3), FBig::from(4));
let sum = &z + &C::I; // (3+4i) + i = 3+5i
assert_eq!(sum.im().significand(), &5.into());
```

## Aggregation: `Sum` and `Product`

All numeric types implement `core::iter::Sum` and `Product` over iterators of `T` or `&T`. For `FBig`/`DBig`, `Sum` is correctly-rounded — addends are accumulated exactly and rounded once (MPFR `mpfr_sum`), not folded with `+`, which rounds at every step and can drop small addends; the result precision is `max` over the addends'. `Product` is a plain fold on every type. For `FBig`/`DBig`/`CBig` the iterator must yield the big type itself (convert primitives first), whereas `UBig`/`IBig`/`RBig` accept primitive elements directly.

```rust
use dashu::float::DBig;

let vals = [dbig!(1), dbig!(1e-20), dbig!(-1)];
let folded = vals.iter().cloned().fold(DBig::ZERO, |a, b| a + b); // 0 — 1e-20 rounded away
let summed: DBig = vals.iter().sum();                             // 1e-20 — Sum is exact
```

## Mixed-type arithmetic

There are **no implicit mixed-type operators** between different big-number kinds (e.g. `UBig + FBig` does not compile) — convert explicitly first (see [Conversion](../convert.md)).
