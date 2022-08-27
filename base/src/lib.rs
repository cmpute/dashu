//! This crate contains general trait definitions and some commonly used structs and enums.

#![cfg_attr(not(feature = "std"), no_std)]

pub mod bit;
pub mod sign;
pub mod ring;
pub mod approx;
pub mod math;

pub use bit::*;
pub use sign::*;
pub use ring::*;
pub use approx::*;
pub use math::*;
