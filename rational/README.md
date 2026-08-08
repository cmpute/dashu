# dashu-ratio

Arbitrary precision rational implementation, as a part of the `dashu` library for arbitrary-precision mathematics. See [Docs.rs](https://docs.rs/dashu-ratio/latest/dashu_ratio/) for the full documentation.

## Features

- Supports `no_std` and written in pure Rust.
- Support a **relaxed** verion of rational numbers for **fast computation**.
- Support for **Diophantine Approximation** of floating point numbers.
- Rational numbers with small numerators and denominators are **inlined** on stack.
- Efficient integer **parsing and printing** with base 2~36.
- **Developer friendly** debug printing for float numbers.

## Quick example

Exact rational arithmetic, recovering the human-intended fraction from a float, and
the `Relaxed` form that skips auto-reduction until you ask for it:

```rust
use dashu::rbig;
use dashu_ratio::{RBig, Relaxed};

// Compile-time rational literal
let exact = rbig!(22 / 7);
// Recover the human-intended rational from a float
let r = RBig::simplest_from_f32(22. / 7.).unwrap();
assert_eq!(r, exact);

// Relaxed: skip auto-reduction for speed, canonicalize at the end
let relaxed: Relaxed = rbig!(~108 / 72);  // no auto-reduction yet
let reduced: RBig = relaxed.canonicalize();
assert_eq!(reduced.numerator(), &3u8.into()); // 108/72 = 3/2

// Exact arithmetic; Display prints numerator/denominator
let sum = rbig!(1 / 2) + rbig!(1 / 3);
assert_eq!(sum.to_string(), "5/6");

// parse rationals from strings too
let _parsed: RBig = "-22/7".parse().unwrap();
```

## Optional dependencies

* `std` (default): enable `std` support for dependencies.

## Performance

Relevant benchmark will be implemented in the [built-in benchmark](../benchmark/).

## License

See the [top-level readme](../README.md).
