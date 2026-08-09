`UBig` and `IBig` support the bitwise operators `&` (and), `|` (or), and `^` (xor). The unary complement `!` is implemented **only on `IBig`**, following the two's-complement rule (`!x == -x - 1`); `UBig` does not implement `!`.

```rust
use dashu::integer::UBig;

let a = UBig::from(0b1100u8);
let b = UBig::from(0b1010u8);
assert_eq!(format!("{:b}", &a & &b), "1000");
assert_eq!(format!("{:b}", &a | &b), "1110");
```

## Bit testing and length

The `BitTest` trait (from `dashu-base`) tests and measures individual bits: `.bit(n)` returns the `n`-th bit, and `.bit_len()` returns the position of the highest set bit plus one. `set_bit(n)` / `clear_bit(n)` mutate a `UBig` in place, and `trailing_zeros()` counts the low-order zero bits.

For counting and slicing, `UBig` also provides `count_ones()`, `count_zeros()` (the zeros *after*
the leading bit, `None` for zero), `trailing_ones()`, `clear_high_bits(n)` (keep only the lowest
`n` bits), and `split_bits(n)` (splits into `(low, high)` at bit `n`). Whether a value is a power
of two — or the next one up — is given by the `PowerOfTwo` trait: `is_power_of_two()` and
`next_power_of_two()`.

```rust
use dashu::base::PowerOfTwo;
use dashu::integer::UBig;

let x = UBig::from(0b10100011u8);
assert_eq!(x.count_ones(), 4);
assert_eq!(x.count_zeros(), Some(4));
let mut y = x.clone();
y.clear_high_bits(4); // keep the lowest 4 bits
assert_eq!(y, UBig::from(0b0011u8));
assert!(UBig::from(1024u32).is_power_of_two());
assert_eq!(UBig::from(1023u32).next_power_of_two(), UBig::from(1024u32));
```

## Shifts

`<<` and `>>` shift by a `usize`. Left shifts grow the number; right shifts shrink it and are equivalent to floor-division by a power of two.

## Using `UBig` as a bit vector

Because a `UBig` has unbounded width, it works naturally as an arbitrarily large bit set: set bit `i` with `set_bit(i)`, test it with `bit(i)`, and read the extent with `bit_len()`.

```rust
use dashu::base::BitTest;
use dashu::integer::UBig;

let mut bits = UBig::ZERO;
bits.set_bit(0);
bits.set_bit(100);
assert!(bits.bit(0) && bits.bit(100));
assert!(!bits.bit(1));
assert_eq!(bits.bit_len(), 101);
```
