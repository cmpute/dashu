# dashu-float

Arbitrary precision floating point number implementation as a part of the `dashu` library. See [Docs.rs](https://docs.rs/dashu-float/latest/dashu_float/) for the full documentation.

# Features

- Supports `no_std` and written in pure Rust.
- Support **arbitrary base** and **arbitrary rounding mode**.
- Support efficient **base conversion**.
- Small float numbers are **inlined** on stack.
- Efficient float number **parsing and printing** with base 2~36.
- Supports the **hexadecimal float format** used by C++.
- **Developer friendly** debug printing for float numbers.

## Optional dependencies

* `std` (default): enable `std` support for dependencies.

## Performance

Relevant benchmark will be implemented in the [built-in benchmark](../benchmark/).

## License

See the [top-level readme](../README.md).

