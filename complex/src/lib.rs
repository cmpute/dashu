// Copyright (c) 2022 Jacob Zhong
//
// Licensed under either of
//
// * Apache License, Version 2.0
//   (LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0)
// * MIT license
//   (LICENSE-MIT or https://opensource.org/licenses/MIT)
//
// at your option.
//
// Unless you explicitly state otherwise, any contribution intentionally submitted
// for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
// dual licensed as above, without any additional terms or conditions.

//! A big arbitrary precision complex number library.
//!
//! The library provides the type [`CBig`]: an arbitrary-precision complex number built on top of
//! [`dashu_float`]'s [`FBig`]. Each [`CBig`] stores a real and an imaginary part ([`Repr`]) over a
//! single shared precision and rounding mode, mirroring [`FBig`]'s `Repr`+`Context` layout. It
//! targets parity with GNU MPC for the common functionalities (field arithmetic + elementary
//! transcendentals + abs/arg/conj/proj + I/O).
//!
//! Rounding follows the C99 Annex G / Kahan branch-cut and signed-zero model that `dashu-float`
//! already implements for reals. There is **no NaN**: C99 NaN-producing cases are mapped to
//! [`FpError`] at the [`Context`] layer (and panics at the convenience layer), exactly mirroring
//! how `FBig` behaves.
//!
//! # Two-layer API
//!
//! Like `FBig`, operations come in two layers:
//! * **Context layer** — [`Context`] methods return a [`CfpResult`] (`Result<CRounded<CBig>, FpError>`)
//!   carrying per-axis inexactness `(Rounding, Rounding)`.
//! * **Convenience layer** — [`CBig`] methods and operators unwrap to a plain [`CBig`], panicking on
//!   `Indeterminate` / `OutOfDomain` / `InfiniteInput` and saturating `Overflow`/`Underflow`.
//!
//! # Examples
//!
//! ```
//! use dashu_cmplx::CBig;
//! use dashu_float::{FBig, round::mode::HalfAway};
//!
//! type C = CBig<HalfAway, 10>; // base-10 so values render as decimals
//! let z = C::from_parts(FBig::from(3), FBig::from(4));
//! let w = C::I;
//! let sum = &z + &w; // (3+4i) + i = 3+5i
//! assert_eq!(sum.re().significand(), &3.into());
//! assert_eq!(sum.imag().significand(), &5.into());
//!
//! // algebraic display
//! assert_eq!(format!("{}", sum), "3+5i");
//! ```
//!
//! # Optional dependencies
//!
//! * `std` (*default*): enable `std` for dependencies.
//! * `num-order` (*default*): `NumOrd`/`NumHash` for `CBig`.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

mod add;
mod cbig;
mod cmp;
mod context;
mod convert;
mod div;
mod fmt;
mod helper_macros;
mod misc;
mod mul;
mod parse;
mod sub;
mod third_party;

// All the public items from third_party will be exposed
#[allow(unused_imports)]
pub use third_party::*;

pub use cbig::CBig;
pub use context::{CRounded, CfpResult, Context};

// Rounding machinery and the float primitives CBig is built on are reused from dashu-float
// unchanged (they appear in this crate's public signatures).
pub use dashu_float::round; // → dashu_cmplx::round::{mode, Round, Rounding}
pub use dashu_float::round::{Round, Rounding};
pub use dashu_float::{ConstCache, FBig, FpError, Repr};

#[doc(hidden)]
pub use dashu_int::Word; // for the cbig! literal macro (M6)
