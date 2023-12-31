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
//! Modular arithmetic is supported by the module [modular]. Some utilities for fast division is provided in the module [fast_div].
//!
//! To construct big integers from literals, please use the [`dashu-macro`](https://docs.rs/dashu-macros/latest/dashu_macros/)
//! crate for your convenience.
//!
//! # Examples
//!
//! ```
//! # use dashu_base::ParseError;
//! use dashu_int::{IBig, fast_div::ConstDivisor, UBig};
//!
//! let a = UBig::from(12345678u32);
//! let b = IBig::from(-0x10ff);
//! let c = IBig::from_str_radix("-azz", 36).unwrap();
//! let d: UBig = "15033211231241234523452345345787".parse()?;
//! let e = 2 * &b - 1;
//! let f = a * b.pow(10);
//!
//! assert_eq!(e, IBig::from(-0x21ff));
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
//! let ring = ConstDivisor::new(UBig::from(10000u32));
//! let x = ring.reduce(12345);
//! let y = ring.reduce(55443);
//! assert_eq!(format!("{}", x - y), "6902 (mod 10000)");
//! # Ok::<(), ParseError>(())
//! ```
//!
//! # Optional dependencies
//!
//! * `std` (*default*): for `std::error::Error` and some internal usages of `std` functions.
//! * `num-traits` (*default*): support traits from crate `num-traits`.
//! * `num-integer` (*default*): support traits from crate `num-integer`.
//! * `num-order` (*default*): support traits from crate `num-order`.
//! * `rand` (*default*): support random number generation based on crate `rand`.
//! * `serde`: support serialization and deserialization based on crate `serde`.
//! * `zeroize`: support traits from crate `zeroize`

#![cfg_attr(not(feature = "std"), no_std)]
// TODO: apply these attributes to all crates
// TODO: #![deny(missing_docs)]
// TODO: #![deny(clippy::allow_attributes_without_reason)]
#![deny(clippy::dbg_macro)]
#![deny(clippy::undocumented_unsafe_blocks)]
#![deny(clippy::let_underscore_must_use)]

extern crate alloc;

pub use crate::{ibig::IBig, ubig::UBig};
pub use dashu_base::Sign;

/// The primitive integer type used to construct the big integers, guaranteed to be
/// a rust built-in unsigned integer type.
///
/// The big integers is interally represented as an array of [Word]s, so convert
/// integers from and into [Word]s are efficient.
///
/// The size of a [Word] is usually the same as [usize], but it's not guaranteed.
/// It's dependent on the target architecture.
pub type Word = arch::word::Word;

/// The primitive integer type that has exactly double the size of [Word].
pub type DoubleWord = arch::word::DoubleWord;

mod add;
mod add_ops;
mod arch;
mod bits;
mod buffer;
mod cmp;
mod convert;
mod div;
mod div_const;
mod div_ops;
mod error;
pub mod fmt;
mod gcd;
mod gcd_ops;
mod helper_macros;
mod ibig;
mod iter;
mod log;
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
mod remove;
mod repr;
mod root;
mod root_ops;
mod shift;
mod shift_ops;
mod sign;
mod sqr;
mod third_party;
mod ubig;

// All the public items from third_party will be exposed
#[allow(unused_imports)]
pub use third_party::*;

// Re-export types for fast division
/// Prepared divisor types for fast division
pub mod fast_div {
    pub use super::div_const::ConstDivisor;
}
