//! This crate contains general trait definitions and some commonly used structs and enums.

#![cfg_attr(not(feature = "std"), no_std)]

pub mod bit;
pub mod number;
pub mod ring;

pub use bit::*;
pub use number::*;
pub use ring::*;
