//! Implementations for third party traits

#[cfg(feature = "rand")]
pub mod rand;

#[cfg(feature = "num-traits")]
mod num_traits;

#[cfg(feature = "serde")]
mod serde;
