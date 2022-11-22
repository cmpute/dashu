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

/// Create an arbitrary precision unsigned integer ([dashu::integer::UBig])
///
/// Usually just pass use a numeric literal. This works for bases 2, 8, 10 or 16 using standard
/// prefixes:
/// ```
/// use dashu::ubig;
/// let a = ubig!(100);
/// let b = ubig!(0b101);
/// let c = ubig!(0o202);
/// let d = ubig!(0x2ff);
/// let e = ubig!(314159265358979323846264338327950288419716939937);
///
/// // underscores can be used to separate digits
/// let f = ubig!(0x5a4653ca_67376856_5b41f775_d6947d55_cf3813d1);
/// ```
///
/// For an arbitrary base, add `base N`:
/// ```
/// use dashu::ubig;
/// let g = ubig!(a3gp1 base 32);
///
/// // it might be necessary to put a underscore to prevent
/// // Rust from recognizing some digits as prefix or exponent
/// let h = ubig!(_100ef base 32);
/// let i = ubig!(_0b102 base 32);
/// let j = ubig!(b102 base 32);
/// assert_eq!(i, j);
/// ```
///
/// For numbers that are small enough (fits in a [u32]), the literal can
/// be assigned to a constant.
///
/// ```
/// use dashu::{integer::UBig, ubig};
///
/// const A: UBig = ubig!(123);
/// const B: UBig = ubig!(0x123);
/// const C: UBig = ubig!(0xffff_ffff);
/// ```
#[macro_export]
macro_rules! ubig {
    ($($t:tt)+) => {
        $crate::__dashu_macros::ubig_embedded!($($t)+)
    }
}

/// Create an arbitrary precision signed integer ([dashu::integer::IBig])
///
/// Usually just pass use a numeric literal. This works for bases 2, 8, 10 or 16 using standard
/// prefixes:
/// ```
/// use dashu::ibig;
/// let a = ibig!(-100);
/// let b = ibig!(0b101);
/// let c = ibig!(-0o202);
/// let d = ibig!(0x2ff);
/// let e = ibig!(314159265358979323846264338327950288419716939937);
///
/// // underscores can be used to separate digits
/// let f = ibig!(-0x5a4653ca_67376856_5b41f775_d6947d55_cf3813d1);
/// ```
///
/// For an arbitrary base, add `base N`:
/// ```
/// # use dashu::ibig;
/// let g = ibig!(-a3gp1 base 32);
///
/// // it might be necessary to put a underscore to prevent
/// // Rust from recognizing some digits as prefix or exponent
/// let h = ibig!(-_100ef base 32);
/// let i = ibig!(_0b102 base 32);
/// let j = ibig!(b102 base 32);
/// assert_eq!(i, j);
/// ```
///
/// For numbers that are small enough (fits in a [u32]), the literal can
/// be assigned to a constant.
///
/// ```
/// use dashu::{ibig, integer::IBig};
///
/// const A: IBig = ibig!(-123);
/// const B: IBig = ibig!(0x123);
/// const C: IBig = ibig!(-0xffff_ffff);
/// ```
#[macro_export]
macro_rules! ibig {
    ($($t:tt)+) => {
        $crate::__dashu_macros::ibig_embedded!($($t)+)
    }
}

/// Create an arbitrary precision float number ([dashu::float::FBig]) with base 2 rounding towards zero.
///
/// This macro only accepts binary or hexadecimal literals. It doesn't allow decimal literals because
/// the conversion is not always lossless. Therefore if you want to create an [FBig][dashu_float::FBig]
/// instance with decimal literals, use the [dbig!] macro and then change the radix with
/// [with_radix][dashu_float::FBig::with_base].
///
/// ```
/// use dashu::fbig;
/// let a = fbig!(11.001); // digits in base 2, equal to 3.125 in decimal
/// let b = fbig!(1.101B-3); // exponent in base 2 can be specified using `Bxx`
/// let c = fbig!(-0x1a7f); // digits in base 16
/// let d = fbig!(0x03.efp-2); // equal to 0.9833984375 in decimal
///
/// // underscores can be used to separate digits
/// let e = fbig!(0xa54653ca_67376856_5b41f775.f00c1782_d6947d55p-33);
///
/// // Due to the limitation of Rust literal syntax, the hexadecimal literal
/// // with floating point requires an underscore prefix if the first digit is
/// // not a decimal digit.
/// let f = fbig!(-_0xae.1f);
/// let g = fbig!(-0xae1fp-8);
/// assert_eq!(f, g);
/// let h = fbig!(-0x12._34);
/// let i = fbig!(-_0x12.34);
/// assert_eq!(h, i);
/// ```
///
/// The generated float has precision determined by length of digits in the input literal.
///
/// ```
/// use dashu::fbig;
/// let a = fbig!(11.001); // 5 binary digits
/// assert_eq!(a.precision(), 5);
///
/// let b = fbig!(0x0003.ef00p-2); // 8 hexadecimal digits = 32 binary digits
/// assert_eq!(b.precision(), 32);
/// assert_eq!(b.digits(), 10); // 0x3ef only has 10 effective bits
/// ```
///
/// For numbers that are small enough (significand fits in a [u32]),
/// the literal can be assigned to a constant.
///
/// ```
/// use dashu::{fbig, float::FBig};
///
/// const A: FBig = fbig!(-1001.10);
/// const B: FBig = fbig!(0x123);
/// const C: FBig = fbig!(-0xffff_ffffp-127);
/// ```
#[macro_export]
macro_rules! fbig {
    ($($t:tt)+) => {
        $crate::__dashu_macros::fbig_embedded!($($t)+)
    }
}

/// Create an arbitrary precision float number ([dashu::float::DBig]) with base 10 rounding to the nearest.
///
/// ```
/// use dashu::dbig;
/// let a = dbig!(12.001);
/// let b = dbig!(7.42e-3); // exponent in base 2 can be specified using `Bxx`
///
/// // underscores can be used to separate digits
/// let c = dbig!(3.141_592_653_589_793_238);
/// ```
///
/// The generated float has precision determined by length of digits in the input literal.
///
/// ```
/// use dashu::dbig;
/// let a = dbig!(12.001); // 5 decimal digits
/// assert_eq!(a.precision(), 5);
///
/// let b = dbig!(003.1200e-2); // 7 decimal digits
/// assert_eq!(b.precision(), 7);
/// assert_eq!(b.digits(), 3); // 312 only has 3 effective digits
/// ```
///
/// For numbers whose significands are small enough (fit in a [u32]),
/// the literal can be assigned to a constant.
///
/// ```
/// use dashu::{dbig, float::DBig};
///
/// const A: DBig = dbig!(-1.201);
/// const B: DBig = dbig!(1234_5678e-100);
/// const C: DBig = dbig!(-1e100000);
/// ```
#[macro_export]
macro_rules! dbig {
    ($($t:tt)+) => {
        $crate::__dashu_macros::dbig_embedded!($($t)+)
    }
}

/// Create an arbitrary precision rational number ([dashu::rational::RBig] or [dashu::rational::Relaxed]).
///
/// ```
/// # use dashu::rbig;
/// let a = rbig!(22/7);
/// let b = rbig!(~-1/13); // use `~` to create a relaxed rational number
///
/// // underscores can be used to separate digits
/// let c = rbig!(107_241/35_291);
/// ```
///
/// For numbers whose the numerator and denominator are small enough (fit in [u32]),
/// the literal can be assigned to a constant.
///
/// ```
/// use dashu::{rational::{RBig, Relaxed}, rbig};
///
/// const A: RBig = rbig!(-1/2);
/// const B: Relaxed = rbig!(~3355/15);
/// ```
#[macro_export]
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
