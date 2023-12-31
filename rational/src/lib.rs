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

//! A big rational library with good performance.
//!
//! The library implements efficient arithmetic and conversion functions in pure Rust.
//!
//! The two main rational types are [RBig] and [Relaxed]. Both of them represent the
//! rational number as a pair of integers (numerator and denominator) and their APIs
//! are mostly the same. However only with [RBig], the numerator and denominator are
//! reduced so that they don't have common divisors other than one. Therefore, [Relaxed]
//! sometimes can be much faster if you don't care about a reduced representation of
//! the rational number. However, benchmarking is always recommended before choosing
//! which representation to use.
//!
//! To construct big rationals from literals, please use the [`dashu-macro`](https://docs.rs/dashu-macros/latest/dashu_macros/)
//! crate for your convenience.
//!
//! # Examples
//!
//! ```
//! # use dashu_base::ParseError;
//! use dashu_int::{IBig, UBig};
//! use dashu_ratio::{RBig, Relaxed};
//!
//! let a = RBig::from_parts((-12).into(), 34u8.into());
//! let b = RBig::from_str_radix("-azz/ep", 36).unwrap();
//! let c = RBig::try_from(3.1415926f32).unwrap(); // c = 6588397 / 2097152 (lossless)
//! let c2 = RBig::simplest_from_f32(3.1415926).unwrap(); // c2 = 51808 / 16491
//! assert_eq!(c2.numerator(), &IBig::from(51808));
//!
//! assert_eq!(c.to_string(), "6588397/2097152");
//! let d = RBig::simplest_from_f32(22./7.).unwrap();
//! assert_eq!(d.to_string(), "22/7"); // round trip to the original literal
//!
//! // for Relaxed, only the common divisor 2 is removed
//! let e: Relaxed = "-3228/1224".parse()?; // d = -807 / 306
//! assert_eq!(e.numerator(), &IBig::from(-807));
//! let f: RBig = e.clone().canonicalize(); // e = -269 / 102
//! assert_eq!(f.numerator(), &IBig::from(-269));
//! # Ok::<(), ParseError>(())
//! ```

#![cfg_attr(not(feature = "std"), no_std)]

mod add;
mod cmp;
mod convert;
mod div;
mod error;
mod fmt;
mod helper_macros;
mod mul;
pub mod ops;
mod parse;
mod rbig;
mod repr;
mod round;
mod sign;
mod simplify;
mod third_party;

// All the public items from third_party will be exposed
#[allow(unused_imports)]
pub use third_party::*;

pub use rbig::{RBig, Relaxed};
