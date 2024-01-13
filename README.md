# dashu

[![Crate](https://img.shields.io/crates/v/dashu.svg)](https://crates.io/crates/dashu)
[![Docs](https://docs.rs/dashu/badge.svg)](https://docs.rs/dashu)
[![Tests](https://github.com/cmpute/dashu/actions/workflows/tests.yml/badge.svg)](https://github.com/cmpute/dashu/actions)
[![MSRV 1.61](https://img.shields.io/badge/rustc-1.61%2B-informational.svg)](#dashu)
[![License](https://img.shields.io/crates/l/dashu)](#license)
<!-- [![Book](https://img.shields.io/badge/book-master-yellow.svg)]() -->

A library set of arbitrary precision numbers (aka. big numbers) implemented in Rust. It's intended to be a Rust native alternative to GNU GMP + MPFR (+ MPC in future). It features:
- Pure rust, full `no_std` support.
- Focus on ergonomics & readability, and then efficiency.
- Optimized speed and memory usage.
- Current MSRV is 1.61.

## Sub-crates

- [`dashu-base`](./base): Common trait definitions
- [`dashu-int`](./integer): Arbitrary precision integers
- [`dashu-float`](./float): Arbitrary precision floating point numbers
- [`dashu-ratio`](./rational): Arbitrary precision rational numbers
- [`dashu-macros`](./macros): Macros for creating big numbers

`dashu` is a meta crate that re-exports all the types from these sub-crates. Please see the README.md in each subdirectory for crate-specific introduction.

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
