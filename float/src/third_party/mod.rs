//! Implementations for third party crates and traits

#[cfg(feature = "num-traits_v02")]
mod num_traits;

#[cfg(feature = "num-order")]
mod num_order;

#[cfg(feature = "rand_v08")]
pub mod rand_v08;
#[cfg(feature = "rand_v08")]
pub use rand_v08 as rand;

#[cfg(feature = "rand_v09")]
pub mod rand_v09;

#[cfg(feature = "rand_v010")]
pub mod rand_v010;

#[cfg(feature = "serde")]
mod serde;

#[cfg(feature = "zeroize")]
mod zeroize;

#[cfg(any(
    feature = "diesel_v1",
    feature = "diesel_v2",
    feature = "postgres-types"
))]
mod postgres;
