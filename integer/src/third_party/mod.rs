//! Implementations for third party traits

#[cfg(feature = "num-integer")]
mod num_integer;

#[cfg(feature = "num-order")]
mod num_order;

#[cfg(feature = "num-traits")]
mod num_traits;

#[cfg(feature = "rand")]
pub mod rand;

#[cfg(feature = "serde")]
mod serde;

// TODO(next): support zeroize::Zeroize, quickcheck::Arbitrary
