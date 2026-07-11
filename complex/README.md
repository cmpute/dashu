# dashu-cmplx

A big arbitrary precision complex number library, implemented in pure Rust.

`dashu-cmplx` provides the arbitrary-precision complex number type [`CBig`], built on top of
[`dashu-float`](https://docs.rs/dashu-float)'s `FBig`. It is the Rust-native alternative to **GNU MPC**,
targeting MPC parity for the common functionalities (field arithmetic + elementary transcendentals +
abs/arg/conj/proj + I/O).

Each `CBig` is a pair of real parts (`re`, `im`) sharing one precision and one rounding mode, mirroring
`FBig`'s own `Repr`+`Context` layout. Rounding follows the C99 Annex G / Kahan branch-cut and signed-zero
model that `dashu-float` already implements for reals.

See the crate-level docs for details.

## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](../LICENSE-APACHE) or https://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](../LICENSE-MIT) or https://opensource.org/licenses/MIT)

at your option.
