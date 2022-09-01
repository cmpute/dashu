//! - Rounding is ensured in type level
//! - Precision is stored inside the numbers
//! - The higher precision will be used if two oprands have different precision
//! - Conversion from f32 and f64 is only implemented for BinaryRepr
//! - Conversion from and to str is limited to native radix. To print or parse with different
//!   radix, use FloatRepr::with_radix() to convert. (printing with certain radices is permitted,
//!   but need to specify explicitly, to print decimal numbers, one can use scientific representation
//!   or use the alternate flag)

#![cfg_attr(not(feature = "std"), no_std)]

// TODO: reference crates: twofloat, num-bigfloat, rust_decimal, bigdecimal, scientific
// TODO: algorithm ref
//   - https://www.researchgate.net/project/Arbitrary-precision-Arithmetic-package
//   - https://www.mpfr.org/algorithms.pdf
//   - Handbook of Floating-Point arithmetic
//   - https://hal.archives-ouvertes.fr/hal-01227877/file/2015-FixFloat.pdf

mod add;
mod cmp;
mod convert;
mod div;
mod error;
mod exp;
mod fbig;
mod fmt;
mod ibig_ext;
mod log;
mod mul;
mod parse;
mod repr;
pub mod round;
mod shift;
mod sign;
mod utils;

pub use fbig::FBig;
pub use repr::Context;

/// Multi-precision float number with decimal exponent and [HalfAway][round::mode::HalfAway] rounding mode
pub type DBig = FBig<10, round::mode::HalfAway>;
