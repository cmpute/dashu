# Bit Manipulation

`UBig` and `IBig` support the bitwise operators `&` (and), `|` (or), `^` (xor), and `!` (not). On `UBig`, `!` is an *infinite-width* complement — every bit above the highest set bit is treated as `1`, so `!n` is generally a very large number. On `IBig`, `!` follows the two's-complement rule.

```rust
use dashu_int::UBig;

let a = UBig::from(0b1100u8);
let b = UBig::from(0b1010u8);
assert_eq!(format!("{:b}", &a & &b), "1000");
assert_eq!(format!("{:b}", &a | &b), "1110");
```

## Bit testing and length

The `BitTest` trait (from `dashu-base`) tests and measures individual bits: `.bit(n)` returns the `n`-th bit, and `.bit_len()` returns the position of the highest set bit plus one. `set_bit(n)` / `clear_bit(n)` mutate a `UBig` in place, and `trailing_zeros()` counts the low-order zero bits.

## Shifts

`<<` and `>>` shift by a `usize`. Left shifts grow the number; right shifts shrink it and are equivalent to floor-division by a power of two.

## Using `UBig` as a bit vector

Because a `UBig` has unbounded width, it works naturally as an arbitrarily large bit set: set bit `i` with `set_bit(i)`, test it with `bit(i)`, and read the extent with `bit_len()`.

```rust
use dashu_base::BitTest;
use dashu_int::UBig;

let mut bits = UBig::ZERO;
bits.set_bit(0);
bits.set_bit(100);
assert!(bits.bit(0) && bits.bit(100));
assert!(!bits.bit(1));
assert_eq!(bits.bit_len(), 101);
```
