//! Third-party trait implementations (feature-gated).

#[cfg(feature = "num-complex")]
mod num_complex;

#[cfg(feature = "num-order")]
mod num_order;

#[cfg(feature = "serde")]
mod serde;

#[cfg(feature = "zeroize")]
mod zeroize;

// Version-agnostic `UniformCBig` distribution + per-version `Distribution` glue (the `rand`
// feature aliases `rand_v08`; `rand_v09`/`rand_v010` are opt-in).
#[cfg(any(feature = "rand_v08", feature = "rand_v09", feature = "rand_v010"))]
pub mod rand;

#[cfg(feature = "rand_v08")]
mod rand_v08;

#[cfg(feature = "rand_v09")]
mod rand_v09;

#[cfg(feature = "rand_v010")]
mod rand_v010;

#[cfg(all(feature = "rkyv_v08", not(feature = "rkyv_v07")))]
mod rkyv_v08;
