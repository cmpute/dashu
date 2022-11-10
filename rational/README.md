# dashu-ratio

Arbitrary precision rational implementation as a part of the `dashu` library. See [Docs.rs](https://docs.rs/dashu-ratio/latest/dashu_ratio/) for the full documentation.

## Features

- Supports `no_std` and written in pure Rust.
- Support a **relaxed** verion of rational numbers for **fast computation**.
- Support for **Diophantine Approximation** of floating point numbers.
- Rational numbers with small numerators and denominators are **inlined** on stack.
- Efficient integer **parsing and printing** with base 2~36.
- **Developer friendly** debug printing for float numbers.

## Optional dependencies

* `std` (default): enable `std` support for dependencies.

## Performance

Relevant benchmark will be implemented in the [built-in benchmark](../benchmark/).

## License

See the [top-level readme](../README.md).
