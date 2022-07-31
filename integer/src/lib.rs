// Copyright (c) 2022 Jacob Zhong
//
// Licensed under either of
//
// * Apache License, Version 2.0
//   (LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0)
// * MIT license
//   (LICENSE-MIT or https://opensource.org/licenses/MIT)
//
// at your option.
//
// Unless you explicitly state otherwise, any contribution intentionally submitted
// for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
// dual licensed as above, without any additional terms or conditions.
//
// This crate is a for of the `ibig` crate. The original LICENSE is included in the
// [NOTICE.md](../NOTICE.md)

//! A big integer library with good performance.
//!
//! The library implements efficient large integer arithmetic in pure Rust.
//!
//! The two main integer types are [UBig] (for unsigned integers) and [IBig] (for signed integers).
//!
//! Modular arithmetic is supported by the module [modular].
//! 
//! To construct big integers from literals conveniently, please use the `dashu-macro` crate.
//!
//! # Examples
//!
//! ```
//! # use dashu_int::error::ParseError;
//! use dashu_int::{IBig, modular::ModuloRing, UBig};
//!
//! let a = UBig::from(12345678u32);
//! let b = UBig::from(0x10ffu16);
//! let c = IBig::from_str_radix("-azz", 36).unwrap();
//! let d: UBig = "15033211231241234523452345345787".parse()?;
//! let e = 2u8 * &b + 1u8;
//! let f = a * b.pow(10);
//!
//! assert_eq!(e, 0x21ff); // direct comparison with primitive integers
//! assert_eq!(c.to_string(), "-14255");
//! assert_eq!(
//!     f.in_radix(16).to_string(),
//!     "1589bda8effbfc495d8d73c83d8b27f94954e"
//! );
//! assert_eq!(
//!     format!("hello {:#x}", d % 0xabcd_1234_1341_3245_1345u128),
//!     "hello 0x1a7e7c487267d2658a93"
//! );
//!
//! let ring = ModuloRing::new(UBig::from(10000u32));
//! let x = ring.convert(12345);
//! let y = ring.convert(55443);
//! assert_eq!(format!("{}", x - y), "6902 (mod 10000)");
//! # Ok::<(), ParseError>(())
//! ```
//!
//! # Optional dependencies
//!
//! * `std` (default): for `std::error::Error`.
//! * `num-traits` (default): integral traits.
//! * `rand` (default): random number generation.
//! * `serde`: serialization and deserialization.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub use crate::{ibig::IBig, sign::Sign, ubig::UBig};

/// The primitive integer type used to construct the big integers.
/// 
/// The big integers is interally represented as an array of [Word]s, so convert
/// integers from and into [Word]s are efficient.
///
/// The size of a [Word] is usually the same as [usize], but it's not guaranteed.
/// It's dependent on the target architecture.
pub type Word = arch::word::Word;

mod add;
mod add_ops;
mod arch;
mod bits;
mod buffer;
mod cmp;
mod convert;
mod div;
mod div_ops;
pub mod error;
pub mod fast_div;
pub mod fmt;
mod gcd;
mod gcd_ops;
mod helper_macros;
mod ibig;
mod math;
mod memory;
pub mod modular;
mod mul;
mod mul_ops;
pub mod ops;
mod parse;
mod pow;
mod primitive;
mod radix;
mod repr;
mod shift;
mod shift_ops;
mod sign;
mod ubig;

#[cfg(feature = "rand")]
pub mod rand;

#[cfg(feature = "num-traits")]
mod num_traits;

#[cfg(feature = "serde")]
mod serde;
