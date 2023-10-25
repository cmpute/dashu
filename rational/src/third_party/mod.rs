//! Implementations for third party crates and traits

#[cfg(feature = "dashu-float")]
mod dashu_float;

#[cfg(feature = "rand_v08")]
pub mod rand;

#[cfg(feature = "num-traits_v02")]
mod num_traits;

#[cfg(feature = "num-order")]
mod num_order;

#[cfg(feature = "serde")]
mod serde;

#[cfg(feature = "zeroize")]
mod zeroize;
