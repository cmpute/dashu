# Printing

`UBig` and `IBig` support the full set of Rust standard formatter traits: `Display`, `Debug`, `Binary`, `Octal`, `LowerHex`, `UpperHex`. The float, rational, and complex types support `Display` and `Debug`, with extra radix/positional helpers described below. All of them honor the sign, width, fill, padding, and alignment options of `Formatter`.

## Integer Formatting

`Display` renders a `UBig`/`IBig` in decimal. The `Binary`, `Octal`, `LowerHex`, and `UpperHex` traits render in base 2/8/16, with the `#` flag adding the conventional `0b`/`0o`/`0x`/`0X` prefix. For any other radix, use `in_radix(r)` (base 2–36); its `#` flag uppercases digits above 9.

```rust
use dashu_int::UBig;

let n = UBig::from(255u8);
assert_eq!(format!("{}", n), "255");
assert_eq!(format!("{:#x}", n), "0xff");
assert_eq!(format!("{:#b}", n), "0b11111111");

assert_eq!(format!("{}", n.in_radix(16)), "ff");
assert_eq!(format!("{:#}", n.in_radix(16)), "FF");
```

## Debug Print

The `Debug` implementation uses a compact **head‥tail** format for large integers: it prints the most significant digits, a `..` separator, and the least significant digits, omitting the middle. For small integers that fit in a single `Word` or `DoubleWord` the full number is shown without truncation.

There are two forms, controlled by the formatter flags:

### Simple form (`{:?}`)

Shows the truncated head‥tail representation.

```rust
use dashu_int::{UBig, IBig};

// Small integers print in full
assert_eq!(format!("{:?}", UBig::from(12345u16)), "12345");
assert_eq!(format!("{:?}", IBig::from(-12345)), "-12345");

// Large integers show head..tail (example for 64-bit Word)
assert_eq!(
    format!("{:?}", UBig::ONE << 1000),
    "1071508607186267320..4386837205668069376"
);
assert_eq!(
    format!("{:?}", IBig::NEG_ONE << 1000),
    "-1071508607186267320..4386837205668069376"
);
```

The number of digits shown on each end depends on the `Word` size — on 64-bit targets it is 19 decimal digits at each end (one word's worth), on 32-bit targets it is 9 digits.

### Verbose form (`{:#?}`)

Adds `(digits: N, bits: M)` after the head‥tail representation, showing the total digit count and bit length.

```rust
use dashu_int::{UBig, Word};

let x = UBig::ONE << 1000;
if Word::BITS == 64 {
    assert_eq!(
        format!("{:#?}", x),
        "1071508607186267320..4386837205668069376 (digits: 302, bits: 1001)"
    );
}
```

## Float Formatting

`FBig`/`DBig` `Display` renders the significand with the radix point positioned by the exponent — the natural positional form, not scientific. The formatter precision option rounds to that many fractional digits.

```rust
use dashu_float::DBig;
use core::str::FromStr;

assert_eq!(format!("{}", DBig::from_str("12.34")?), "12.34");
assert_eq!(format!("{:.1}", DBig::from_str("12.34")?), "12.3");
```

For scientific notation use `LowerExp`/`UpperExp`: the exponent marker is `e`/`E` in base 10 and `@` in other bases. `Debug` prints `significand * base ^ exponent (prec: N)` (or a struct with `{:#?}`). Infinities render as `inf` / `-inf` under both `Display` and `Debug`.

```rust
use dashu_float::DBig;
use core::str::FromStr;

assert_eq!(format!("{:e}", DBig::from_str("1234.5")?), "1.2345e3");
assert_eq!(format!("{:E}", DBig::from_str("1234.5")?), "1.2345E3");
```

## Rational Formatting

`RBig`/`Relaxed` `Display` renders as `numerator/denominator`, or just the numerator when the denominator is `1`. The `Binary`/`Octal`/`LowerHex`/`UpperHex` traits and `in_radix(r)` format both parts in the given base.

```rust
use dashu_ratio::RBig;
use core::str::FromStr;

assert_eq!(format!("{}", RBig::from_str("22/7")?), "22/7");
assert_eq!(format!("{}", RBig::from_str("5/1")?), "5");
```

For the positional (decimal) expansion use `in_expanded()`. `{:.N}` prints exactly `N` fractional digits; the `#` flag detects the repeating part and parenthesizes it:

```rust
use dashu_ratio::RBig;

let x = RBig::from_parts(1.into(), 3u8.into());
assert_eq!(format!("{:.4}", x.in_expanded()), "0.3333");
assert_eq!(format!("{:#}", x.in_expanded()), "0.(3)");
```

## Complex Formatting

`CBig` `Display` uses the algebraic $a+bi$ notation: the imaginary term always carries an explicit sign, a unit coefficient is elided (`i`, not `1i`), and a zero imaginary part is omitted. `Debug` prints `re:<re> im:<im> (prec: <p>)`.

```rust
use dashu_cmplx::CBig;
use dashu_float::{FBig, round::mode::HalfAway};

type C = CBig<HalfAway, 10>;
type F = FBig<HalfAway, 10>;

assert_eq!(format!("{}", C::from_parts(F::from(1), F::from(2))), "1+2i");
assert_eq!(format!("{}", C::from_parts(F::from(-3), F::from(-4))), "-3-4i");
assert_eq!(format!("{}", C::from_parts(F::from(5), F::from(0))), "5");
assert_eq!(format!("{}", C::from_parts(F::from(0), F::from(1))), "i");
assert_eq!(format!("{}", C::from_parts(F::from(0), F::from(-1))), "-i");
```

The same algebraic grammar is accepted on input — see [Parsing](./parse.md).
