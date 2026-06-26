# Standard Format API

`UBig` and `IBig` support the full set of Rust standard formatter traits:
[`Display`], [`Debug`], [`Binary`], [`Octal`], [`LowerHex`], [`UpperHex`].
All of them support the sign, width, fill, padding, and alignment options of
[`Formatter`]. For custom radices use [`InRadix`] (see below).

TODO: describe the `in_radix` API

## Debug Print

The [`Debug`] implementation uses a compact **head‥tail** format for large
integers: it prints the most significant digits, a `..` separator, and the
least significant digits, omitting the middle.  For small integers that fit in
a single [`Word`] or [`DoubleWord`] the full number is shown without
truncation.

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

The number of digits shown on each end depends on the [`Word`] size —
on 64-bit targets it is 19 decimal digits at each end (one word's worth),
on 32-bit targets it is 9 digits.

### Verbose form (`{:#?}`)

Adds `(digits: N, bits: M)` after the head‥tail representation, showing the
total digit count and bit length.

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

## Rational Number Formatting

TODO: rational numbers have both in_radix and in_expanded functions, other than normal traits
