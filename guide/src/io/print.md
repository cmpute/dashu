`UBig` and `IBig` support the full set of Rust standard formatter traits: `Display`, `Debug`, `Binary`, `Octal`, `LowerHex`, `UpperHex`. The float, rational, and complex types support `Display` and `Debug`, with extra radix/positional helpers described below. All of them honor the sign, width, fill, padding, and alignment options of `Formatter`.

## Integer Formatting

`Display` renders a `UBig`/`IBig` in decimal. The `Binary`, `Octal`, `LowerHex`, and `UpperHex` traits render in base 2/8/16, with the `#` flag adding the conventional `0b`/`0o`/`0x`/`0X` prefix. For any other radix, use `in_radix(r)` (base 2–36); its `#` flag uppercases digits above 9.

```rust
use dashu::integer::UBig;

let n = UBig::from(255u8);
assert_eq!(format!("{}", n), "255");
assert_eq!(format!("{:#x}", n), "0xff");
assert_eq!(format!("{:#b}", n), "0b11111111");

assert_eq!(format!("{}", n.in_radix(16)), "ff");
assert_eq!(format!("{:#}", n.in_radix(16)), "FF");
```

## Float Formatting

`FBig`/`DBig` `Display` renders the significand with the radix point positioned by the exponent — the natural positional form, not scientific. The formatter precision option rounds to that many fractional digits.

```rust
use core::str::FromStr;
use dashu::float::DBig;

assert_eq!(format!("{}", DBig::from_str("12.34")?), "12.34");
assert_eq!(format!("{:.1}", DBig::from_str("12.34")?), "12.3");
```

For scientific notation use `LowerExp`/`UpperExp`: the exponent marker is `e`/`E` in base 10 and `@` in other bases. Infinities render as `inf` / `-inf` under both `Display` and `Debug`.

```rust
use core::str::FromStr;
use dashu::float::DBig;

assert_eq!(format!("{:e}", DBig::from_str("1234.5")?), "1.2345e3");
assert_eq!(format!("{:E}", DBig::from_str("1234.5")?), "1.2345E3");
```

## Rational Formatting

`RBig`/`Relaxed` `Display` renders as `numerator/denominator`, or just the numerator when the denominator is `1`. The `Binary`/`Octal`/`LowerHex`/`UpperHex` traits and `in_radix(r)` format both parts in the given base.

```rust
use core::str::FromStr;
use dashu::rational::RBig;

assert_eq!(format!("{}", RBig::from_str("22/7")?), "22/7");
assert_eq!(format!("{}", RBig::from_str("5/1")?), "5");
```

For the positional (decimal) expansion use `in_expanded()`. `{:.N}` prints exactly `N` fractional digits; the `#` flag detects the repeating part and parenthesizes it:

```rust
use dashu::rational::RBig;

let x = RBig::from_parts(1.into(), 3u8.into());
assert_eq!(format!("{:.4}", x.in_expanded()), "0.3333");
assert_eq!(format!("{:#}", x.in_expanded()), "0.(3)");
```

## Complex Formatting

`CBig` `Display` uses the algebraic $a+bi$ notation: the imaginary term always carries an explicit sign, a unit coefficient is elided (`i`, not `1i`), and a zero imaginary part is omitted.

```rust
use dashu::complex::CBig;
use dashu::float::{FBig, round::mode::HalfAway};

type C = CBig<HalfAway, 10>;
type F = FBig<HalfAway, 10>;

assert_eq!(format!("{}", C::from_parts(F::from(1), F::from(2))), "1+2i");
assert_eq!(format!("{}", C::from_parts(F::from(-3), F::from(-4))), "-3-4i");
assert_eq!(format!("{}", C::from_parts(F::from(5), F::from(0))), "5");
assert_eq!(format!("{}", C::from_parts(F::from(0), F::from(1))), "i");
assert_eq!(format!("{}", C::from_parts(F::from(0), F::from(-1))), "-i");
```

The same algebraic grammar is accepted on input — see [Parsing](./parse.md).

## Debug Print

`Debug` output is meant for quick inspection (it is **not** a stable serialization format — see [Serialization](./serialize.md)). Large integers use a compact **head‥tail** format — the most-significant digits, a `..` separator, then the least-significant digits, with the middle omitted — while small integers print in full. Each numeric type has its own `Debug` shape:

```rust
use core::str::FromStr;
use dashu::complex::CBig;
use dashu::float::{CachedFBig, Context, DBig, FBig, Repr, round::mode::HalfAway};
use dashu::integer::{IBig, UBig};
use dashu::rational::RBig;

// UBig / IBig — head..tail for large values, full for small
assert_eq!(format!("{:?}", UBig::from(12345u16)), "12345");
assert_eq!(format!("{:?}", IBig::from(-12345)), "-12345");
assert_eq!(
    format!("{:?}", UBig::ONE << 1000),
    "1071508607186267320..4386837205668069376"
);

// FBig / DBig — significand * base ^ exponent (prec: N)
let f: FBig = FBig::from(3u8); // FBig<Zero, 2>
assert_eq!(format!("{:?}", f), "3 * 2 ^ 0 (prec: 2)");
assert_eq!(format!("{:?}", DBig::from_str("12.34")?), "1234 * 10 ^ -2 (prec: 4)");

// CachedFBig — a struct exposing the repr and precision
let c = CachedFBig::<HalfAway, 10>::with_cache(Repr::new(1234.into(), -3), Context::new(50));
assert_eq!(format!("{:?}", c), "CachedFBig { repr: 1234 * 10 ^ -3, precision: 50 }");

// RBig — numerator / denominator
assert_eq!(format!("{:?}", RBig::from_parts(1.into(), 3u8.into())), "1 / 3");

// CBig — re:<re> im:<im> (prec: N)
type F = FBig<HalfAway, 10>;
assert_eq!(
    format!("{:?}", CBig::<HalfAway, 10>::from_parts(F::from(3), F::from(4))),
    "re:3 im:4 (prec: 1)"
);
```

The head‥tail digit count depends on the `Word` size — 19 decimal digits per end on 64-bit targets, 9 on 32-bit. The verbose form `{:#?}` adds detail; for integers it appends the digit count and bit length:

```rust
use dashu::integer::{UBig, Word};

let x = UBig::ONE << 1000;
if Word::BITS == 64 {
    assert_eq!(
        format!("{:#?}", x),
        "1071508607186267320..4386837205668069376 (digits: 302, bits: 1001)"
    );
}
```
