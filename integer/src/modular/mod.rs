//! Modular arithmetic.
//!
//! Modular arithmetic is performed on [Modulo] values attached to a [ModuloRing].
//!
//! Trying to mix different rings (even with the same modulus!) will cause a panic.
//!
//! # Examples
//!
//! ```
//! use dashu_int::{modular::ModuloRing, UBig};
//!
//! let ring = ModuloRing::new(UBig::from(10000u32));
//! let x = ring.convert(12345);
//! let y = ring.convert(55443);
//! assert_eq!(format!("{}", x - y), "6902 (mod 10000)");
//! ```

pub use convert::IntoModulo;
pub use modulo::Modulo;
pub use modulo_ring::ModuloRing;

mod add;
pub(crate) mod convert;
mod eq;
mod fmt;
mod inv;
pub(crate) mod modulo;
pub(crate) mod modulo_ring;
mod mul;
mod pow;
