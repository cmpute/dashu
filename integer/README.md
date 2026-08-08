# dashu-int

Arbitrary precision integer implementation, as a part of the `dashu` library for arbitrary-precision mathematics. See [Docs.rs](https://docs.rs/dashu-int/latest/dashu_int/) for the full documentation.

> The majority of the code is based on the [ibig crate](https://github.com/tczajka/ibig-rs). The modification notice based on the the original `ibig` repo is included in the [NOTICE](./NOTICE) file.

## Features

- Supports `no_std` and written in pure Rust.
- Support for both **unsigned** and **signed** big integers.
- Small integers are **inlined** on stack with specialized algorithms.
- **Efficient** implementation for basic arithmetic operations (`+`,`-`,`*`,`/`,`%`,`<<`,`>>`).
- Support **advanced** arithmetic operations including `pow`, `ilog`, `gcd`, `gcd_ext`.
- Bit operations for signed big integers follow the **2's complement rule**.
- **Efficient** implementation for modular arithmetics (e.g. modular powering and inverse).
- Efficient integer **parsing and printing** with base 2~36.
- **Developer friendly** debug printing for big integers.
- **Direct access** to underlying machine word array.

## Quick example

Construct integers with the compile-time literal macros (beyond `u128`, no precision
loss), mix them with built-in integers in arithmetic, and parse from any base:

```rust
use dashu_base::BitTest;
use dashu_int::{UBig, IBig};
use dashu_macros::{ubig, ibig};

// Compile-time literal beyond u128
let n = ubig!(0x5a4653ca_67376856_5b41f775_d6947d55_cf3813d1);
// Mixed-type arithmetic with primitive integers
let e = 2 * &IBig::from(-0x10ff) - 1;
// Parse a signed integer in an arbitrary base
let c = IBig::from_str_radix("-azz", 36).unwrap();

assert_eq!(e, IBig::from(-0x21ff));
assert_eq!(c.to_string(), "-14255");

// Bit operations follow the two's-complement rule
assert_eq!(ubig!(0xffff_ffff_ffff_ffff) >> 63, ubig!(1));
assert_eq!(ibig!(-1) >> 1, ibig!(-1)); // arithmetic shift

// bit_len reports the magnitude in bits
assert_eq!(n.bit_len(), 159);
```

For modular arithmetic, `MontgomeryRepr` reduces a modulus into Montgomery form so
multiplication, squaring, and exponentiation avoid the expensive division:

```rust
use dashu_int::{UBig, monty::MontgomeryRepr};

let p = UBig::from(2u8).pow(607) - UBig::ONE; // a Mersenne prime
let ring = MontgomeryRepr::new(p.clone());

// reduce values into Montgomery form, then multiply / square / pow
let a = ring.reduce(123u8);
assert_eq!(a.pow(&(p - UBig::ONE)), ring.reduce(1u8)); // Fermat: a^(p-1) = 1 (mod p)

// + - * stay in Montgomery form, avoiding the division
assert_eq!(ring.reduce(3u8) + ring.reduce(4u8), ring.reduce(7u8));
assert_eq!(ring.reduce(3u8) * ring.reduce(4u8), ring.reduce(12u8));

// inversion works too (3·3⁻¹ ≡ 1 mod p)
assert_eq!(ring.reduce(3u8).inv().unwrap() * ring.reduce(3u8), ring.reduce(1u8));
```

## Optional dependencies

* `std` (default): for `std::error::Error`.
* `num-traits` (default): integral traits.
* `rand` (default): random number generation.
* `serde`: serialization and deserialization.

## Performance

See the [built-in benchmark](../benchmark/).

## License

See the [top-level readme](../README.md).
