//! Helper macros for constructing numbers
//!
//! These macros are the simpler versions of the ones in `dashu-macros`, meant
//! to be used in testing only.
//!
//! These macros rely on string parsing, so do not use
//! these macros when testing string parsing!

#[macro_export]
macro_rules! ubig {
    ($val:tt) => {{
        const STR: &::core::primitive::str = ::core::stringify!($val);
        ::core::result::Result::expect(
            ::dashu_int::UBig::from_str_with_radix_prefix(STR),
            "invalid number",
        )
        .0
    }};
    ($val:tt base $radix:literal) => {{
        const STR: &::core::primitive::str = ::core::stringify!($val);
        let s =
            ::core::option::Option::unwrap_or(::core::primitive::str::strip_prefix(STR, "_"), STR);
        ::core::result::Result::expect(
            ::dashu_int::UBig::from_str_radix(s, $radix),
            "invalid number",
        )
    }};
}

#[macro_export]
macro_rules! ibig {
    (- $val:tt) => {
        - <::dashu_int::IBig as ::core::convert::From<::dashu_int::UBig>>::from($crate::ubig!($val))
    };
    (- $val:tt base $radix:literal) => {
        - <::dashu_int::IBig as ::core::convert::From<::dashu_int::UBig>>::from(
            $crate::ubig!($val base $radix)
        )
    };
    ($val:tt) => {
        <::dashu_int::IBig as ::core::convert::From<::dashu_int::UBig>>::from($crate::ubig!($val))
    };
    ($val:tt base $radix:literal) => {
        <::dashu_int::IBig as ::core::convert::From<::dashu_int::UBig>>::from(
            $crate::ubig!($val base $radix)
        )
    };
}
