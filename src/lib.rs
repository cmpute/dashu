//! The meta crate that re-exports all `dashu` numeric types.

#![cfg_attr(not(feature = "std"), no_std)]

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

#[doc(hidden)]
pub use dashu_macros as __dashu_macros;

#[macro_export]
#[doc = include_str!("../macros/docs/ubig.md")]
macro_rules! ubig {
    ($($t:tt)+) => {
        $crate::__dashu_macros::ubig_embedded!($($t)+)
    }
}

#[macro_export]
#[rustversion::since(1.64)]
#[doc = include_str!("../macros/docs/static_ubig.md")]
macro_rules! static_ubig {
    ($($t:tt)+) => {
        $crate::__dashu_macros::static_ubig_embedded!($($t)+)
    }
}

#[macro_export]
#[doc = include_str!("../macros/docs/ibig.md")]
macro_rules! ibig {
    ($($t:tt)+) => {
        $crate::__dashu_macros::ibig_embedded!($($t)+)
    }
}

#[macro_export]
#[rustversion::since(1.64)]
#[doc = include_str!("../macros/docs/static_ibig.md")]
macro_rules! static_ibig {
    ($($t:tt)+) => {
        $crate::__dashu_macros::static_ibig_embedded!($($t)+)
    }
}

#[macro_export]
#[doc = include_str!("../macros/docs/fbig.md")]
macro_rules! fbig {
    ($($t:tt)+) => {
        $crate::__dashu_macros::fbig_embedded!($($t)+)
    }
}

#[macro_export]
#[rustversion::since(1.64)]
#[doc = include_str!("../macros/docs/static_fbig.md")]
macro_rules! static_fbig {
    ($($t:tt)+) => {
        $crate::__dashu_macros::static_fbig_embedded!($($t)+)
    }
}

#[macro_export]
#[doc = include_str!("../macros/docs/dbig.md")]
macro_rules! dbig {
    ($($t:tt)+) => {
        $crate::__dashu_macros::dbig_embedded!($($t)+)
    }
}

#[macro_export]
#[rustversion::since(1.64)]
#[doc = include_str!("../macros/docs/static_dbig.md")]
macro_rules! static_dbig {
    ($($t:tt)+) => {
        $crate::__dashu_macros::static_dbig_embedded!($($t)+)
    }
}

#[macro_export]
#[doc = include_str!("../macros/docs/rbig.md")]
macro_rules! rbig {
    ($($t:tt)+) => {
        $crate::__dashu_macros::rbig_embedded!($($t)+)
    }
}

/// A verbose alias for [UBig][dashu_int::UBig]
pub type Natural = dashu_int::UBig;

/// A verbose alias for [IBig][dashu_int::IBig]
pub type Integer = dashu_int::IBig;

/// A verbose alias for [FBig][dashu_float::FBig] (base 2, rounding towards zero)
pub type Real = dashu_float::FBig;

/// A verbose alias for [DBig][dashu_float::DBig] (base 10, rounding to the nearest)
pub type Decimal = dashu_float::DBig;

/// A verbose alias for [RBig][dashu_ratio::RBig]
pub type Rational = dashu_ratio::RBig;
