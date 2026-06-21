//! Implementations for third party crates and traits

#[cfg(feature = "num-integer_v01")]
mod num_integer;

#[cfg(feature = "num-order")]
mod num_order;

#[cfg(feature = "num-traits_v02")]
mod num_traits;

// Version-agnostic distributions + sampling algorithms (the `dashu_int::rand` path).
// The per-version rand trait impls live in `rand_v08` / `rand_v09` / `rand_v010`.
#[cfg(any(feature = "rand_v08", feature = "rand_v09", feature = "rand_v010"))]
pub mod rand;

#[cfg(feature = "rand_v08")]
mod rand_v08;

#[cfg(feature = "rand_v09")]
mod rand_v09;

#[cfg(feature = "rand_v010")]
mod rand_v010;

#[cfg(feature = "serde")]
mod serde;

#[cfg(feature = "zeroize")]
mod zeroize;
