//! Montgomery modular arithmetic.
//!
//! Modular arithmetic in [Montgomery form], an alternative to the Barrett-style reduction used by
//! the [`modular`](crate::modular) module. A modular multiplication is an ordinary multiplication
//! followed by a Montgomery reduction (REDC) instead of a division, so it is faster than Barrett
//! whenever the REDC is cheaper than the division.
//!
//! The operand product reuses the crate's fast multiplication (Karatsuba / Toom-3 / NTT); the
//! reduction is a word-by-word REDC using the double-word "addmul_2" kernel. Each result is fully
//! reduced to the canonical range `[0, m)`.
//!
//! # When to use Montgomery vs Barrett
//!
//! Montgomery multiplication, squaring and exponentiation beat [`modular::Reduced`] across roughly
//! the 256–4096-bit range, where the word-by-word REDC is cheaper than Barrett's division. Beyond
//! ~8 kbits the two are comparable, as the O(n²) REDC meets Barrett's sub-quadratic
//! divide-and-conquer division.
//!
//! **For inverse-heavy workloads, prefer [`modular::Reduced`].** Computing a Montgomery inverse
//! requires exiting Montgomery form (a REDC), running the extended GCD, then re-entering (a
//! multiplication plus another REDC); Barrett keeps values in plain form and inverts directly, so
//! [`Reduced::inv`](crate::modular::Reduced::inv) is substantially faster. (The GCD itself is the
//! same sub-quadratic Lehmer algorithm in both backends — the difference is the conversion
//! round-trip.)
//!
//! References (also cited by the sibling `modular` module):
//! * Yanik, Savaş, Koç. *Incomplete Reduction in Modular Arithmetic.*
//!   <https://cetinkayakoc.net/docs/j56.pdf>
//! * Gueron. *Efficient Software Implementations of Modular Exponentiation.*
//!   <https://eprint.iacr.org/2011/239.pdf>
//!
//! [Montgomery form]: https://en.wikipedia.org/wiki/Montgomery_modular_multiplication
//! [`modular::Reduced`]: crate::modular::Reduced
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
