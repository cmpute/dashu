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

//! A macro library for creating big numbers from literals.
//!
//! See the documentation of each macro for the usage.
//!
//! # Dependency requirement
//!
//! Due the fact that the macros expand to plain tokens, and proc macro crates can't
//! re-export types, it's required to add explicit dependency to the underlying crates
//! when using the macros. Specifically, you need to add the following crates as dependencies
//! to your `Cargo.toml`:
//! * For [ubig!] and [ibig!]: `dashu-int`
//! * For [fbig!] and [dbig!]: `dashu-int`, `dashu-float`
//! * For [rbig!]: `dashu-int`, `dashu-ratio`
//!
//! If you are using these macros from the `dashu` crate, then it's not necessary to
//! explicitly adding these dependencies, because the related types are re-exported
//! by the `dashu` crate.

use proc_macro::TokenStream;

mod parse;

/// Create an arbitrary precision unsigned integer ([dashu_int::UBig])
///
/// Usually just pass use a numeric literal. This works for bases 2, 8, 10 or 16 using standard
/// prefixes:
/// ```
/// # use dashu_macros::ubig;
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
/// # use dashu_macros::ubig;
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
/// # use dashu_macros::ubig;
/// use dashu_int::UBig;
///
/// const A: UBig = ubig!(123);
/// const B: UBig = ubig!(0x123);
/// const C: UBig = ubig!(0xffff_ffff);
/// ```
#[proc_macro]
pub fn ubig(input: TokenStream) -> TokenStream {
    parse::int::parse_integer::<false>(input.into()).into()
}

/// Create an arbitrary precision signed integer ([dashu_int::IBig])
///
/// Usually just pass use a numeric literal. This works for bases 2, 8, 10 or 16 using standard
/// prefixes:
/// ```
/// # use dashu_macros::ibig;
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
/// # use dashu_macros::ibig;
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
/// # use dashu_macros::ibig;
/// use dashu_int::IBig;
///
/// const A: IBig = ibig!(-123);
/// const B: IBig = ibig!(0x123);
/// const C: IBig = ibig!(-0xffff_ffff);
/// ```
#[proc_macro]
pub fn ibig(input: TokenStream) -> TokenStream {
    parse::int::parse_integer::<true>(input.into()).into()
}

/// Create an arbitrary precision float number ([dashu_float::FBig]) with base 2 rounding towards zero.
///
/// This macro only accepts binary or hexadecimal literals. It doesn't allow decimal literals because
/// the conversion is not always lossless. Therefore if you want to create an [FBig][dashu_float::FBig]
/// instance with decimal literals, use the [dbig!] macro and then change the radix with
/// [with_radix][dashu_float::FBig::with_base].
///
/// ```
/// # use dashu_macros::fbig;
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
/// # use dashu_macros::fbig;
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
/// # use dashu_macros::fbig;
/// use dashu_float::FBig;
///
/// const A: FBig = fbig!(-1001.10);
/// const B: FBig = fbig!(0x123);
/// const C: FBig = fbig!(-0xffff_ffffp-127);
/// ```
#[proc_macro]
pub fn fbig(input: TokenStream) -> TokenStream {
    parse::float::parse_binary_float(input.into()).into()
}

/// Create an arbitrary precision float number ([dashu_float::DBig]) with base 10 rounding to the nearest.
///
/// ```
/// # use dashu_macros::dbig;
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
/// # use dashu_macros::dbig;
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
/// # use dashu_macros::dbig;
/// use dashu_float::DBig;
///
/// const A: DBig = dbig!(-1.201);
/// const B: DBig = dbig!(1234_5678e-100);
/// const C: DBig = dbig!(-1e100000);
/// ```
#[proc_macro]
pub fn dbig(input: TokenStream) -> TokenStream {
    parse::float::parse_decimal_float(input.into()).into()
}

/// Create an arbitrary precision rational number ([dashu_ratio::RBig] or [dashu_ratio::Relaxed]).
///
/// ```
/// # use dashu_macros::rbig;
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
/// # use dashu_macros::rbig;
/// use dashu_ratio::{RBig, Relaxed};
///
/// const A: RBig = rbig!(-1/2);
/// const B: Relaxed = rbig!(~3355/15);
/// ```
#[proc_macro]
pub fn rbig(input: TokenStream) -> TokenStream {
    parse::ratio::parse_ratio(input.into()).into()
}
