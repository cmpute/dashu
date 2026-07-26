Every numeric type implements `FromStr`, so values can be built with `"...".parse()?` or `T::from_str(...)`. Underscore separators are allowed in all numeric literals.

## Parsing Integers

`UBig::from_str` / `IBig::from_str` accept an optional sign followed by decimal digits. For other bases use `from_str_radix(s, radix)` (radix 2–36); it recognizes a `0x`/`0o`/`0b` prefix independently of the `radix` argument.

```rust
use dashu::integer::{UBig, IBig};
use core::str::FromStr;

assert_eq!(UBig::from_str("12345")?, UBig::from(12345u16));
assert_eq!(IBig::from_str_radix("-1aff", 16)?, IBig::from(-0x1aff));
```

## Parsing Floats

`FBig`/`DBig` `FromStr` reads the significand in the value's native base, with the exponent in one of these forms:

| Form | Meaning | Base |
|------|---------|------|
| `aaa` / `aaa.` / `aaa.bbb` | fixed point | any |
| `aaa.bbb@cc` | significand × base^cc | any |
| `aaa.bbbEcc` / `aaa.bbbecc` | significand × 10^cc | 10 |
| `0xaaa.bbbPcc` | hex significand × 2^cc | 2 |

Precision is inferred from the number of significant digits presented. String `inf`/`NaN` literals are **not** accepted — construct infinities from the `INFINITY` constant instead.

```rust
use dashu::float::DBig;
use core::str::FromStr;

assert_eq!(format!("{:e}", DBig::from_str("6.022e23")?), "6.022e23");
assert_eq!(DBig::from_str("-0.0123456789")?.to_string(), "-0.0123456789");
```

## Parsing Rationals

`RBig::from_str` accepts `numerator/denominator`, or just a numerator (denominator defaults to 1). `from_str_radix` parses both parts in the given base; a `0x`/`0o`/`0b` prefix must be consistent between them.

```rust
use dashu::rational::RBig;
use core::str::FromStr;

assert_eq!(RBig::from_str("22/7")?.to_string(), "22/7");
```

For decimal literals, `RBig::from_str_decimal` / `Relaxed::from_str_decimal` accept fixed-point (`1.5`, `-.25`), scientific (`1.234e2`, `1.5@2`), and **repeating-decimal** notation with the repetend parenthesized (`0.1(6)` = 1/6, `0.(3)` = 1/3). It is the exact inverse of `in_expanded(10)` (see [Printing](./print.md)): every rational round-trips through `{:#}`, and terminating decimals round-trip through `{:.N}`.

```rust
use dashu::rational::RBig;
use core::str::FromStr;

let x = RBig::from_str_decimal("0.1(6)")?; // 1/6
assert_eq!(x, RBig::from_str("1/6")?);
// every rational round-trips through the repetend printer
assert_eq!(RBig::from_str_decimal(&format!("{:#}", x.in_expanded(10)))?, x);
```

## Parsing Complex

`CBig::FromStr` accepts the same algebraic $a+bi$ grammar that `Display` emits: an optional real term plus an optional signed imaginary term (at least one required); a unit coefficient may be omitted (`i`, `-i`). The MPC-style parenthesized form `(re im)` is **not** accepted.

```rust
use dashu::complex::CBig;
use dashu::float::round::mode::HalfAway;
use core::str::FromStr;

type C = CBig<HalfAway, 10>;
assert_eq!(C::from_str("1+2i")?.to_string(), "1+2i");
assert_eq!(C::from_str("-i")?.to_string(), "-i");
```
