# dashu-cmplx

A big arbitrary precision complex number library for mathematics, implemented in pure Rust.

`dashu-cmplx` provides the arbitrary-precision complex number type [`CBig`], built on top of
[`dashu-float`](https://docs.rs/dashu-float)'s `FBig`. It is the Rust-native alternative to **GNU MPC**,
targeting MPC parity for the common functionalities (field arithmetic + elementary transcendentals +
abs/arg/conj/proj + I/O).

Each `CBig` is a pair of real parts (`re`, `im`) sharing one precision and one rounding mode, mirroring
`FBig`'s own `Repr`+`Context` layout. Rounding follows the C99 Annex G / Kahan branch-cut and signed-zero
model that `dashu-float` already implements for reals.

See the crate-level docs for details.

## Quick example

The algebraic `a+bi` form is both the input and output grammar, and the transcendentals
are correctly rounded:

```rust
use dashu_cmplx::CBig;
use dashu_float::round::mode::HalfAway;

type C = CBig<HalfAway, 10>;

// Algebraic form is both the input and output grammar
let z: C = "3+4i".parse().unwrap();
let w = C::I; // the imaginary unit

assert_eq!(format!("{}", &z + &w), "3+5i");
assert_eq!(format!("{}", &z * C::I), "-4+3i");
let _rotated = z.exp(); // correctly rounded

// abs / conj round out the surface
assert_eq!(z.abs().to_string(), "5"); // |3+4i| = 5
assert_eq!(z.conj().to_string(), "3-4i");

// decompose into real/imaginary parts
let (re, im) = z.into_parts();
assert_eq!(re.to_string(), "3");
assert_eq!(im.to_string(), "4");
```

## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](../LICENSE-APACHE) or https://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](../LICENSE-MIT) or https://opensource.org/licenses/MIT)

at your option.
