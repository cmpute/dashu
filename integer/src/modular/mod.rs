//! Modular arithmetic.
//!
//! Modular arithmetic is performed on [Reduced] values attached to a [ConstDivisor][crate::div_const::ConstDivisor].
//!
//! Trying to mix different [ConstDivisor][crate::fast_div::ConstDivisor] instances (even with the same modulus!) will cause a panic.
//!
//! # Examples
//!
//! ```
//! use dashu_int::{fast_div::ConstDivisor, UBig};
//!
//! let ring = ConstDivisor::new(UBig::from(10000u32));
//! let x = ring.reduce(12345);
//! let y = ring.reduce(55443);
//! assert_eq!(format!("{}", x - y), "6902 (mod 10000)");
//! ```

pub use convert::IntoRing;
pub use repr::Reduced;

mod add;
pub(crate) mod convert;
mod div;
mod fmt;
mod mul;
mod pow;
mod reducer;
pub(crate) mod repr;

// TODO: Also support Montgomery form reductions, use relaxed form described
//       in https://cetinkayakoc.net/docs/j56.pdf and https://eprint.iacr.org/2011/239.pdf
