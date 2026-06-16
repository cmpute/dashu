//! Montgomery modular arithmetic.
//!
//! This module provides modular arithmetic in [Montgomery form], which replaces the
//! division step of modular reduction with an extra multiplication. For large moduli
//! this is faster than the Barrett-style reduction used by the [`modular`](crate::modular)
//! module, because the crate's fast multiplication algorithms (Karatsuba, Toom-3, NTT)
//! are reused for both the operand product and the Montgomery correction term.
//!
//! The reduction uses word-aligned REDC with the relaxed "Almost-Montgomery" output
//! range `[0, 2m)` (see the papers referenced below); each operation is fully reduced
//! to the canonical range `[0, m)`.
//!
//! References (also cited by the sibling `modular` module):
//! * Yanik, Savaş, Koç. *Incomplete Reduction in Modular Arithmetic.*
//!   <https://cetinkayakoc.net/docs/j56.pdf>
//! * Gueron. *Efficient Software Implementations of Modular Exponentiation.*
//!   <https://eprint.iacr.org/2011/239.pdf>
//!
//! [Montgomery form]: https://en.wikipedia.org/wiki/Montgomery_modular_multiplication
//!
//! # Examples
//!
//! ```
//! # use dashu_int::{monty::MontgomeryRepr, UBig};
//! // A Mersenne prime.
//! let p = UBig::from(2u8).pow(607) - UBig::ONE;
//! let ring = MontgomeryRepr::new(p.clone());
//! // Fermat's little theorem: a^(p-1) = 1 (mod p)
//! let a = ring.reduce(123);
//! assert_eq!(a.pow(&(p - UBig::ONE)), ring.reduce(1));
//! ```

pub use convert::IntoMontgomeryRing;
pub use repr::{Montgomery, MontgomeryRepr};

mod add;
pub(crate) mod convert;
mod div;
mod fmt;
mod mul;
mod pow;
pub(crate) mod repr;
