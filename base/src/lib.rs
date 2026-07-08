//! This crate contains general trait definitions and some commonly used structs and enums.

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(missing_docs)]
#![deny(clippy::dbg_macro)]
#![deny(clippy::undocumented_unsafe_blocks)]
#![deny(clippy::let_underscore_must_use)]

pub mod approx;
pub mod bit;
pub mod error;
pub mod math;
pub mod ring;
pub mod sign;

/// Some useful utility functions that are also used internally in this crate.
pub mod utils {
    pub use super::math::log::{next_down, next_up};
}

pub use approx::*;
pub use bit::*;
pub use error::*;
pub use math::*;
pub use ring::*;
pub use sign::*;
