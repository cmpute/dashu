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

/// Create a FBig instance with base 2 from literal, note that this macro doesn't support the float point.
#[macro_export]
macro_rules! fbig {
    ($val:tt) => {{
        const STR: &::core::primitive::str = ::core::stringify!($val);
        let stripped = STR.strip_prefix('_').unwrap_or(STR);
        ::core::result::Result::expect(
            <::dashu_float::FBig as ::core::str::FromStr>::from_str(stripped),
            "invalid number",
        )
    }};
    ($val:tt - $exp:literal) => {{
        const VAL_STR: &::core::primitive::str = ::core::stringify!($val);
        const EXP_STR: &::core::primitive::str = ::core::stringify!($exp);
        let stripped = VAL_STR.strip_prefix('_').unwrap_or(VAL_STR);
        let concat = [stripped, &"-", EXP_STR].concat();
        ::core::result::Result::expect(
            <::dashu_float::FBig as ::core::str::FromStr>::from_str(&concat),
            "invalid number",
        )
    }};
    (-$val:tt) => {{
        const STR: &::core::primitive::str = ::core::stringify!($val);
        let stripped = STR.strip_prefix('_').unwrap_or(STR);
        -::core::result::Result::expect(
            <::dashu_float::FBig as ::core::str::FromStr>::from_str(stripped),
            "invalid number",
        )
    }};
    (-$val:tt - $exp:literal) => {{
        const VAL_STR: &::core::primitive::str = ::core::stringify!($val);
        const EXP_STR: &::core::primitive::str = ::core::stringify!($exp);
        let stripped = VAL_STR.strip_prefix('_').unwrap_or(VAL_STR);
        let concat = [stripped, &"-", EXP_STR].concat();
        -::core::result::Result::expect(
            <::dashu_float::FBig as ::core::str::FromStr>::from_str(&concat),
            "invalid number",
        )
    }};
}

/// Create a DBig instance from literal, note that this macro doesn't support the float point.
#[macro_export]
macro_rules! dbig {
    ($val:tt) => {{
        const STR: &::core::primitive::str = ::core::stringify!($val);
        ::core::result::Result::expect(
            <::dashu_float::DBig as ::core::str::FromStr>::from_str(STR),
            "invalid number",
        )
    }};
    ($val:tt - $exp:literal) => {{
        const VAL_STR: &::core::primitive::str = ::core::stringify!($val);
        const EXP_STR: &::core::primitive::str = ::core::stringify!($exp);
        let concat = [VAL_STR, &"-", EXP_STR].concat();
        ::core::result::Result::expect(
            <::dashu_float::DBig as ::core::str::FromStr>::from_str(&concat),
            "invalid number",
        )
    }};
    (-$val:tt) => {{
        const STR: &::core::primitive::str = ::core::stringify!($val);
        -::core::result::Result::expect(
            <::dashu_float::DBig as ::core::str::FromStr>::from_str(STR),
            "invalid number",
        )
    }};
    (-$val:tt - $exp:literal) => {{
        const VAL_STR: &::core::primitive::str = ::core::stringify!($val);
        const EXP_STR: &::core::primitive::str = ::core::stringify!($exp);
        let concat = [VAL_STR, &"-", EXP_STR].concat();
        -::core::result::Result::expect(
            <::dashu_float::DBig as ::core::str::FromStr>::from_str(&concat),
            "invalid number",
        )
    }};
}
