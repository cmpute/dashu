# dashu-int

Arbitrary precision integer implementation as a part of the `dashu` library. See [Docs.rs](https://docs.rs/dashu-int/latest/dashu_int/) for the full documentation.

> The majority of the code is based on the [ibig crate](https://github.com/tczajka/ibig-rs). The modification notice based on the the original `ibig` repo is included in the [NOTICE](./NOTICE) file.

## Features

- Support for both **unsigned** and **signed** big integers.
- Small integers are **inlined** on stack with specialized algorithms.
- **Efficient** implementation for basic arithmetic operations (`+`,`-`,`*`,`/`,`%`,`<<`,`>>`).
- Support **advanced** arithmetic operations including `pow`, `ilog`, `gcd`, `gcd_ext`.
- Bit operations for signed big integers follow the **2's complement rule**.
- **Efficient** implementation for modular arithmetics (e.g. modular powering and inverse).
- Efficient integer **parsing and printing** with base 2~36.
- **Developer friendly** debug printing for big integers.
- **Direct access** to underlying machine word array.

## Optional dependencies

* `std` (default): for `std::error::Error`.
* `num-traits` (default): integral traits.
* `rand` (default): random number generation.
* `serde`: serialization and deserialization.

## Performance

See the [built-in benchmark](../benchmark/).

## License

See the [top-level readme](../README.md).
