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

//! A big float library supporting arbitrary precision, arbitrary base and arbitrary rounding mode.
//!
//! The library implements efficient large floating point arithmetic in pure Rust.
//!
//! The main type is [FBig] representing the arbitrary precision floating point numbers, the [DBig] type
//! is an alias supporting decimal floating point numbers.
//!
//! To construct big floats from literals, please use the [`dashu-macro`](https://docs.rs/dashu-macros/latest/dashu_macros/)
//! crate for your convenience.
//!
//! # Examples
//!
//! ```
//! # use dashu_base::ParseError;
//! use core::convert::TryFrom;
//! use dashu_float::DBig;
//!
//! // due to the limit of rust generics, the default float type
//! // need to be instantiate explicitly
//! type FBig = dashu_float::FBig;
//!
//! let a = FBig::try_from(-12.34_f32).unwrap();
//! let b = DBig::from_str_native("6.022e23")?;
//! let c = DBig::from_parts(271828.into(), -5);
//! let d: DBig = "-0.0123456789".parse()?;
//! let e = 2 * b.ln() + DBig::ONE;
//! let f = &c * d.powi(10.into()) / 7;
//!
//! assert_eq!(a.precision(), 24); // IEEE 754 single has 24 significant bits
//! assert_eq!(b.precision(), 4); // 4 decimal digits
//!
//! assert!(b > c); // comparison is limited in the same base
//! assert!(a.to_decimal().value() < d);
//! assert_eq!(c.to_string(), "2.71828");
//!
//! // use associated functions of the context to get full result
//! use dashu_base::Approximation::*;
//! use dashu_float::{Context, round::{mode::HalfAway, Rounding::*}};
//! let ctxt = Context::<HalfAway>::new(6);
//! assert_eq!(ctxt.exp(DBig::ONE.repr()), Inexact(c, NoOp));
//! # Ok::<(), ParseError>(())
//! ```
//!
//! # Optional dependencies
//!
//! * `std` (*default*): enable `std` for dependencies.

#![cfg_attr(not(feature = "std"), no_std)]

mod add;
mod cmp;
mod convert;
mod div;
mod error;
mod exp;
mod fbig;
mod fmt;
mod helper_macros;
mod iter;
mod log;
mod mul;
pub mod ops;
mod parse;
mod repr;
mod root;
pub mod round;
mod round_ops;
mod shift;
mod sign;
mod third_party;
mod utils;

// All the public items from third_party will be exposed
#[allow(unused_imports)]
pub use third_party::*;

pub use fbig::FBig;
pub use repr::{Context, Repr};

/// Multi-precision float number with decimal exponent and [HalfAway][round::mode::HalfAway] rounding mode
pub type DBig = FBig<round::mode::HalfAway, 10>;

// TODO: allow operations with inf, but only panic when the result is nan (inf - inf and inf / inf)
//       for division with zero (and other functions that has different limits at zero),
//       we might forbidden it because we don't want to support negative zero in this library.
