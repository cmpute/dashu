# dashu

[English](README.md) | [简体中文](README-zh.md)

<img src="guide/src/assets/dashu-banner.png" alt="dashu">

[![Crate](https://img.shields.io/crates/v/dashu.svg)](https://crates.io/crates/dashu)
[![Docs](https://docs.rs/dashu/badge.svg)](https://docs.rs/dashu)
[![Tests](https://github.com/cmpute/dashu/actions/workflows/tests.yml/badge.svg)](https://github.com/cmpute/dashu/actions)
[![MSRV 1.68](https://img.shields.io/badge/rustc-1.68%2B-informational.svg)](#dashu)
[![License](https://img.shields.io/crates/l/dashu)](#license)
[![Book](https://img.shields.io/badge/book-user_guide-yellow.svg)](https://zyxin.xyz/dashu/)

A library set of arbitrary precision numbers (aka. big numbers) for mathematics and numerics, implemented in Rust. It's a Rust native alternative to GNU GMP + MPFR + MPC. It features:
- Pure rust, full `no_std` support.
- Focus on ergonomics & readability, and then efficiency.
- Optimized speed and memory usage.
- Current MSRV is 1.68. The MSRV covers the default build (no optional features); optional
  features may require a newer Rust version (e.g. `rkyv_v08` needs Rust ≥ 1.81).

## Sub-crates

- [`dashu-base`](./base): Common trait definitions
- [`dashu-int`](./integer): Arbitrary precision integers
- [`dashu-float`](./float): Arbitrary precision floating point numbers
- [`dashu-ratio`](./rational): Arbitrary precision rational numbers
- [`dashu-cmplx`](./complex): Arbitrary precision complex numbers
- [`dashu-macros`](./macros): Macros for creating big numbers

`dashu` is a meta crate that re-exports all the types from these sub-crates. Please see the README.md in each subdirectory for crate-specific introduction.

## Examples

### Construction & literal macros

Construct numbers with the compile-time literal macros (no precision loss, any size) and
the meta-crate type aliases — readable names for every number domain:

```rust
use dashu::{ubig, ibig, fbig, dbig, rbig, cbig};
use dashu::{Natural, Integer, Real, Decimal, Rational, Complex};

// Compile-time literals — zero precision loss, any size
let n: Natural = ubig!(0x5a4653ca_67376856_5b41f775_d6947d55_cf3813d1);
let e: Real = fbig!(0x1.ffffp1023);
let pi: Decimal = dbig!(3.1415926535897932384626);
let r: Rational = rbig!(22 / 7);
let z: Complex = cbig!(3 + 4i); // complex, decimal by default

// Meta-crate type aliases cover every number domain
let _neg: Integer = ibig!(-0x10ff);
let _prod = &n * &_neg; // Natural × Integer → Integer

// Explicit radix and base prefixes work too
let _hex = ubig!(dead_beef base 16);
let _bin = ibig!(-0b1111);

// Associated constants are available on every type
let _one: Natural = Natural::ONE;
let _unit = Complex::I; // the imaginary unit
```

### String conversion

Parse from strings in any base, then format back with the full `std::fmt` mini-language —
hexadecimal, scientific, positional expansion, and more:

```rust
use dashu::{ubig, Integer, Decimal, Rational};
use core::str::FromStr;

// Parse from strings — any base, scientific notation, rational form
let a = Integer::from_str_radix("1a2b3c", 16).unwrap();
let b: Decimal = "3.1415926535897932384626".parse().unwrap();
let c: Rational = "22/7".parse().unwrap();

// Full std::fmt mini-language for every type
assert_eq!(format!("{:#x}", a), "0x1a2b3c");
assert_eq!(format!("{:e}", b), "3.1415926535897932384626e0");
assert_eq!(format!("{:#}", c.in_expanded(10)), "3.(142857)");

// in_radix formats in any base; {:.N} rounds the fractional digits
assert_eq!(format!("{}", a.in_radix(32)), "1kaps");
assert_eq!(format!("{:.3}", c.in_expanded(10)), "3.143");

// Debug prints a compact head‥tail form for large values
assert_eq!(
    format!("{:?}", ubig!(1) << 1000),
    "1071508607186267320..4386837205668069376"
);
```

### Serialization

Enable the `serde` feature for compact binary encoding (e.g. `postcard`) and
human-readable JSON round-trips with precision preserved:

```rust
// requires: features = ["serde"]

use dashu::{Integer, Rational, Real};

// Binary format: compact little-endian byte encoding
let a = Integer::from(12345u32);
let bytes = postcard::to_stdvec(&a).unwrap();
let b: Integer = postcard::from_bytes(&bytes).unwrap();
assert_eq!(a, b);

// Human-readable: precision-preserving string format
let r = Rational::from_parts(22u8.into(), 7u8.into());
let json = serde_json::to_string(&r).unwrap();
assert_eq!(json, r#""22/7""#);

// JSON round-trips preserve the value exactly
let rt: Rational = serde_json::from_str(&json).unwrap();
assert_eq!(rt, r);

// Floats serialize losslessly too, with precision preserved
let x: Real = "0x1.8p0".parse().unwrap(); // 1.5
let json2 = serde_json::to_string(&x).unwrap();
let _rt: Real = serde_json::from_str(&json2).unwrap();
```

### Random numbers

Enable the `rand` feature for uniform sampling — integers of a given bit-length,
floats in `[0, 1)` at a chosen precision, and more:

```rust
// requires: features = ["rand"]

use dashu::{Natural, Integer, Real};
use dashu::base::BitTest;
use dashu::integer::rand::UniformBits;
use dashu::float::rand::Uniform01;
use rand::RngExt;

let mut rng = rand::rng();

// Uniform random integers with a bit-length limit
let a: Natural = rng.sample(UniformBits::new(256));
let b: Integer = rng.sample(UniformBits::new(64));
assert!(a.bit_len() <= 256 && b.bit_len() <= 64);

// Uniform floats in [0, 1) at chosen precisions
let x: Real = rng.sample(Uniform01::new(53)); // binary64
let _y: Real = rng.sample(Uniform01::new(200)); // higher precision
assert!(x >= Real::ZERO && x < Real::ONE);

// sample_iter streams an unbounded sequence
let _stream: Vec<Natural> = rng.sample_iter(UniformBits::new(8)).take(3).collect();
```

### Type conversion

Convert freely between number domains — binary to decimal floats, floats to rationals
(recovering the human-intended fraction), floats to integers with a rounding direction:

```rust
use dashu::float::round::mode::Zero;
use dashu::{Real, Decimal, Rational, Integer};

// Base conversion: binary (Real) ↔ decimal (Decimal)
let x: Decimal = "3.141592653589793".parse().unwrap();
let y = x.to_binary().value(); // Decimal → binary float
let _back = y.to_decimal().value(); // and back

// Float → rational: recover the fraction the programmer meant
let r = Rational::simplest_from_f64(0.1).unwrap();
assert_eq!(r, Rational::from_parts(1u8.into(), 10u8.into()));

// Float → integer with a rounding direction
let floor: Integer = y.to_int().value(); // truncates toward zero
assert_eq!(floor, Integer::from(3u8));

// Exact rational → float at a chosen precision
let third = Rational::from_parts(1u8.into(), 3u8.into());
let _f = third.to_float::<Zero, 2>(50).value(); // 1/3 in base 2
```

## Python package

[`dashu-python`](./python) is a user-friendly test field for the core dashu
functionalities: through the [`dashu-rs`](https://pypi.org/project/dashu-rs/)
package on PyPI, users can try dashu's arbitrary-precision integers, rationals,
floats, and complex numbers from Python to get an idea of what dashu is capable
of — no Rust toolchain needed. Beyond exploring dashu, it also stands on its own
as a standalone arbitrary-precision number package for the Python ecosystem.

## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](../LICENSE-APACHE) or https://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](../LICENSE-MIT) or https://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
