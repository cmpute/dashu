//! A big float library supporting arbitrary precision, arbitrary base and arbitrary rounding mode.
//! 
// TODO(v0.2): crate level docs

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
mod log;
mod mul;
mod parse;
mod repr;
pub mod round;
mod shift;
mod sign;
mod utils;

pub use fbig::FBig;
pub use repr::{Context, Repr};

/// Multi-precision float number with decimal exponent and [HalfAway][round::mode::HalfAway] rounding mode
pub type DBig = FBig<round::mode::HalfAway, 10>;
