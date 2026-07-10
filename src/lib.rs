//! The meta crate that re-exports all `dashu` numeric types.

#![cfg_attr(not(feature = "std"), no_std)]
#![doc(
    html_logo_url = "data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iNTAwIiBoZWlnaHQ9IjUwMCIgdmlld0JveD0iMCAwIDUwMCA1MDAiIHhtbG5zPSJodHRwOi8vd3d3LnczLm9yZy8yMDAwL3N2ZyI+PHJlY3Qgd2lkdGg9IjUwMCIgaGVpZ2h0PSI1MDAiIGZpbGw9IiNGNUYxRTYiLz48Y2lyY2xlIGN4PSIyNTAiIGN5PSIxNzAiIHI9IjEzMCIgZmlsbD0iI0YyRTVDOCIgb3BhY2l0eT0iMC43Ii8+PGNpcmNsZSBjeD0iMjUwIiBjeT0iMTgwIiByPSIxMDAiIGZpbGw9IiNFRURGQTkiIG9wYWNpdHk9IjAuNSIvPjxwYXRoIGQ9Ik0yNTAgMTE1QzMxMCAxMTUgMzU1IDE1MCAzNjUgMTk1QzQwNSAyMDUgNDQwIDI0NSA0NDAgMjk1QzQ0MCAzNDUgMzk1IDM3MCAzMzUgMzcwTDE2NSAzNzBDMTA1IDM3MCA2MCAzNDUgNjAgMjk1QzYwIDI0NSA5NSAyMDUgMTM1IDE5NUMxNDUgMTUwIDE5MCAxMTUgMjUwIDExNVoiIGZpbGw9IiMzRDVBNDUiLz48ZyBzdHJva2U9IiMzRDVBNDUiIHN0cm9rZS13aWR0aD0iNiIgc3Ryb2tlLWxpbmVjYXA9InJvdW5kIiBmaWxsPSJub25lIj48bGluZSB4MT0iMTIwIiB5MT0iMzYwIiB4Mj0iMTIwIiB5Mj0iNDIwIi8+PGxpbmUgeDE9IjE1MCIgeTE9IjM3MCIgeDI9IjE1MCIgeTI9IjQ1MCIvPjxsaW5lIHgxPSIxNzUiIHkxPSIzNzAiIHgyPSIxNzUiIHkyPSI0MjUiLz48bGluZSB4MT0iMjAwIiB5MT0iMzcwIiB4Mj0iMjAwIiB5Mj0iNDM1Ii8+PGxpbmUgeDE9IjMwMCIgeTE9IjM3MCIgeDI9IjMwMCIgeTI9IjQzNSIvPjxsaW5lIHgxPSIzMjUiIHkxPSIzNzAiIHgyPSIzMjUiIHkyPSI0MjUiLz48bGluZSB4MT0iMzUwIiB5MT0iMzcwIiB4Mj0iMzUwIiB5Mj0iNDUwIi8+PGxpbmUgeDE9IjM4MCIgeTE9IjM2MCIgeDI9IjM4MCIgeTI9IjQyMCIvPjwvZz48ZyBzdHJva2U9IiNDODVBM0UiIHN0cm9rZS13aWR0aD0iOCIgc3Ryb2tlLWxpbmVjYXA9InJvdW5kIiBmaWxsPSJub25lIj48cGF0aCBkPSJNMjMyIDQzNUwyMzIgMzQwUTIzMCAzMTAgMjEwIDI5NUwxOTggMjg3Ii8+PHBhdGggZD0iTTI2OCA0MzVMMjY4IDM0MFEyNzAgMzEwIDI5MCAyOTVMMzAyIDI4NyIvPjxsaW5lIHgxPSIyMTgiIHkxPSI0MzUiIHgyPSIyODIiIHkyPSI0MzUiLz48L2c+PC9zdmc+Cg=="
)]
#![deny(missing_docs)]
#![deny(clippy::dbg_macro)]
#![deny(clippy::undocumented_unsafe_blocks)]
#![deny(clippy::let_underscore_must_use)]

/// Defintions of common traits
pub mod base {
    pub use dashu_base::*;
}

/// Arbitrary precision integer number
pub mod integer {
    pub use dashu_int::*;
}

/// Arbitrary precision floating point number
pub mod float {
    pub use dashu_float::*;
}

/// Arbitrary precision rational number
pub mod rational {
    pub use dashu_ratio::*;
}

/// Arbitrary precision complex number
pub mod complex {
    pub use dashu_cmplx::*;
}

#[doc(hidden)]
pub use dashu_macros as __dashu_macros;

#[macro_export]
#[doc = include_str!("macro-docs/ubig.md")]
macro_rules! ubig {
    ($($t:tt)+) => {
        $crate::__dashu_macros::ubig_embedded!($($t)+)
    }
}

#[macro_export]
#[rustversion::since(1.64)]
#[doc = include_str!("macro-docs/static_ubig.md")]
macro_rules! static_ubig {
    ($($t:tt)+) => {
        $crate::__dashu_macros::static_ubig_embedded!($($t)+)
    }
}

#[macro_export]
#[doc = include_str!("macro-docs/ibig.md")]
macro_rules! ibig {
    ($($t:tt)+) => {
        $crate::__dashu_macros::ibig_embedded!($($t)+)
    }
}

#[macro_export]
#[rustversion::since(1.64)]
#[doc = include_str!("macro-docs/static_ibig.md")]
macro_rules! static_ibig {
    ($($t:tt)+) => {
        $crate::__dashu_macros::static_ibig_embedded!($($t)+)
    }
}

#[macro_export]
#[doc = include_str!("macro-docs/fbig.md")]
macro_rules! fbig {
    ($($t:tt)+) => {
        $crate::__dashu_macros::fbig_embedded!($($t)+)
    }
}

#[macro_export]
#[rustversion::since(1.64)]
#[doc = include_str!("macro-docs/static_fbig.md")]
macro_rules! static_fbig {
    ($($t:tt)+) => {
        $crate::__dashu_macros::static_fbig_embedded!($($t)+)
    }
}

#[macro_export]
#[doc = include_str!("macro-docs/dbig.md")]
macro_rules! dbig {
    ($($t:tt)+) => {
        $crate::__dashu_macros::dbig_embedded!($($t)+)
    }
}

#[macro_export]
#[rustversion::since(1.64)]
#[doc = include_str!("macro-docs/static_dbig.md")]
macro_rules! static_dbig {
    ($($t:tt)+) => {
        $crate::__dashu_macros::static_dbig_embedded!($($t)+)
    }
}

#[macro_export]
#[doc = include_str!("macro-docs/rbig.md")]
macro_rules! rbig {
    ($($t:tt)+) => {
        $crate::__dashu_macros::rbig_embedded!($($t)+)
    }
}

#[macro_export]
#[rustversion::since(1.64)]
#[doc = include_str!("macro-docs/static_rbig.md")]
macro_rules! static_rbig {
    ($($t:tt)+) => {
        $crate::__dashu_macros::static_rbig_embedded!($($t)+)
    }
}

#[macro_export]
#[doc = include_str!("macro-docs/cbig.md")]
macro_rules! cbig {
    ($($t:tt)+) => {
        $crate::__dashu_macros::cbig_embedded!($($t)+)
    }
}

#[macro_export]
#[rustversion::since(1.64)]
#[doc = include_str!("macro-docs/static_cbig.md")]
macro_rules! static_cbig {
    ($($t:tt)+) => {
        $crate::__dashu_macros::static_cbig_embedded!($($t)+)
    }
}

/// A verbose alias for [UBig][dashu_int::UBig]
pub type Natural = dashu_int::UBig;

/// A verbose alias for [IBig][dashu_int::IBig]
pub type Integer = dashu_int::IBig;

/// A verbose alias for [FBig][dashu_float::FBig] (base 2, rounding towards zero)
pub type Real = dashu_float::FBig;

/// A verbose alias for [CachedFBig][dashu_float::CachedFBig] (base 2, rounding towards zero) — the
/// cached, faster variant of [`Real`] for transcendental-heavy code. `!Send + !Sync`.
pub type FastReal = dashu_float::CachedFBig;

/// A verbose alias for [DBig][dashu_float::DBig] (base 10, rounding to the nearest)
pub type Decimal = dashu_float::DBig;

/// A verbose alias for the base-10 [CachedFBig][dashu_float::CachedFBig] (rounding to nearest) — the
/// cached, faster variant of [`Decimal`]. `!Send + !Sync`.
pub type FastDecimal = dashu_float::CachedFBig<dashu_float::round::mode::HalfAway, 10>;

/// A verbose alias for [RBig][dashu_ratio::RBig]
pub type Rational = dashu_ratio::RBig;

/// A verbose alias for [CBig][dashu_cmplx::CBig] (base 2, rounding towards zero)
pub type Complex = dashu_cmplx::CBig;

/// A verbose alias for [CachedCBig][dashu_cmplx::CachedCBig] (base 2, rounding towards zero) — the
/// cached, faster variant of [`Complex`] for transcendental-heavy code. `!Send + !Sync`.
pub type FastComplex = dashu_cmplx::CachedCBig;
