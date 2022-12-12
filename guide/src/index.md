# The user guide for `dashu`

Welcome to the `dashu` user guide! `dashu` is a library set of arbitrary precision numbers (aka. big numbers) implemented in pure Rust with `no_std` support.

The book is a companion to [`dashu`'s API docs](https://docs.rs/dashu/latest/dashu/). It contains a more concise overview of all the functionalities equipped with `dashu` and some examples.

Please choose from the chapters on the left to jump to individual topics.

# Philosophy

The `dashu` crates are designed to be **ergonomic**, **readable** and **efficient**. (ordered descendingly in importance)

## Ergonomics

Our first goal for the crates is to create numeric types that are **easy to use**. The ergonomics of `dashu` is currently reflected by the following features:

1. Binary operators are supported between the big number types and primitive types. For example, an unsigned big integer (of type `UBig`) modulo `u64` will return a `u64` value.
2. The big float type has the rounding mode embedded in the generic parameter, so that you don't need to specify the rounding mode wherever you want to do arithmetic operations.
3. Several macros are provided for creating the big numbers from literals, and we designed the structure of the numeric types so that they can be constructed in `const` context.

> When we focus on ergonomics, we also care about **correctness**, so we don't provide functionalities that are handy in some cases, but could lead to confusion.
> 
> For example, we support binary operations between `IBig` and unsigned primitive integers (e.g. `u64`), but we don't support those between `UBig` and signed primitive integers (such as `i64`).

## Readability

We want the library to be a great reference, like TomMath, for somebody that also want to implement algorithms for arbitrary precision numbers. Therefore we will try to make the internal functions well-documented, and explain the algorithms through the comments in the code.

We also use `clippy` lints, such as `#![deny(clippy::undocumented_unsafe_blocks)]`, to make the code idiomatic and robust.

## Efficiency

`dashu` is currently maintained by only a few people that are not the best experts in arbitrary precision computing. We are not pushing the crates to be the most performant one among all the libraries written in various languages.
> If you want the fastest crate for arbitrary precision library, you should better take a look at bindings to the [GMP library](https://gmplib.org), such as [rug](https://lib.rs/crates/rug).

However, we will still put efforts into optimizing the algorithms used in the crates and make it more performant enough for most users. [Our benchmarks](./benchmark.md) show that the `dashu` crates are **among the fastest** implementations of the arbitrary precision types in Rust.

# License

Licensed under either [Apache License, Version 2.0](https://www.apache.org/licenses/LICENSE-2.0) or [MIT license](https://opensource.org/licenses/MIT) at your option.
