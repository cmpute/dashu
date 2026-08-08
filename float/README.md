# dashu-float

Arbitrary precision floating point number implementation, as a part of the `dashu` library for arbitrary-precision mathematics. See [Docs.rs](https://docs.rs/dashu-float/latest/dashu_float/) for the full documentation.

# Features

- Supports `no_std` and written in pure Rust.
- Support **arbitrary base** and **arbitrary rounding mode**.
- Support efficient **base conversion**.
- Small float numbers are **inlined** on stack.
- Efficient float number **parsing and printing** with base 2~36.
- Supports the **hexadecimal float format** used by C++.
- **Developer friendly** debug printing for float numbers.

## Quick example

Binary and decimal float literals, correctly-rounded transcendentals, and the
`Context` layer for explicit precision control:

```rust
use dashu::{fbig, dbig};
use dashu_float::{Context, round::mode::HalfEven};

// Binary and decimal float literals — hex floats, precision from digit count
let x = fbig!(0x1.fffp-1); // ≈ 0.9998
let pi = dbig!(3.1415926535897932384626);

// Correctly-rounded transcendentals (Ziv engine)
let e = x.exp(); // exp(0.9998…), binary
let _log_pi = pi.ln();

// Context layer: IEEE 754 binary64 precision, HalfEven rounding
let ctx = Context::<HalfEven>::new(53);
let e53 = ctx.exp(&x.repr(), None).unwrap().value();
assert!(e53 > e); // higher precision, larger value

// set the precision explicitly
let x50 = x.with_precision(50).value();
assert_eq!(x50.precision(), 50);
```

## Optional dependencies

* `std` (default): enable `std` support for dependencies.

## Performance

Relevant benchmark will be implemented in the [built-in benchmark](../benchmark/).

## License

See the [top-level readme](../README.md).

