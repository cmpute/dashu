//! Third-party trait implementations (feature-gated).

#[cfg(feature = "num-order")]
mod num_order;

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
