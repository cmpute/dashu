//! Macros for big integer literals.

/// Create a [UBig](crate::UBig) value.
///
/// Usually just pass use a numeric literal. This works for bases 2, 8, 10 or 16 using standard
/// prefixes:
/// ```
/// # use dashu_int::ubig;
/// let a = ubig!(100);
/// let b = ubig!(0b101);
/// let c = ubig!(0o202);
/// let d = ubig!(0x2ff);
///
/// let f = ubig!(314159265358979323846264338327950288419716939937);
/// let g = ubig!(0x1234567890123456789012345678901234567890123456789);
/// ```
///
/// For an arbitrary base, add `base N`:
/// ```
/// # use dashu_int::ubig;
/// let e = ubig!(a3gp1 base 32);
/// ```
///
/// This macro also supports using underscore as separaters in the string just like primitives integers,
/// this could be useful to increase the readability.
/// ```
/// # use dashu_int::ubig;
/// let f = ubig!(3141_5926_5358_9793_2384_6264_3383_2795_0288_4197);
/// let g = ubig!(3141592653589793238462643383279502884197);
/// assert_eq!(f, g);
/// let h = ubig!(0x1234_5678_9abc_def0);
/// let i = ubig!(0x123456789abcdef0);
/// assert_eq!(h, i);
/// ```
///
/// If the sequence of digits is not a valid Rust literal or identifier, put an underscore before
/// the digits. This may be necessary when the first digit is decimal, but not all digits are decimal.
/// ```
/// # use dashu_int::ubig;
/// let h = ubig!(_0b102 base 32);
/// let i = ubig!(b102 base 32);
/// assert_eq!(h, i);
/// let j = ubig!(_100ef base 32);
/// ```
#[macro_export]
macro_rules! ubig {
    ($val:tt) => {{
        const STR: &::core::primitive::str = ::core::stringify!($val);
        const PRIM: ::core::option::Option<::core::primitive::u128> =
            $crate::parse::parse_int_from_const_str_with_prefix(STR.as_bytes());

        if let ::core::option::Option::Some(prim) = PRIM {
            <$crate::UBig as ::core::convert::From<::core::primitive::u128>>::from(prim)
        } else {
            ::core::result::Result::expect(
                $crate::UBig::from_str_with_radix_prefix(STR),
                "invalid number",
            )
        }
    }};
    ($val:tt base $radix:literal) => {{
        const STR: &::core::primitive::str = ::core::stringify!($val);
        const PRIM: ::core::option::Option<::core::primitive::u128> =
            $crate::parse::parse_int_from_const_str::<$radix>(STR.as_bytes());

        if let ::core::option::Option::Some(prim) = PRIM {
            <$crate::UBig as ::core::convert::From<::core::primitive::u128>>::from(prim)
        } else {
            let s = ::core::option::Option::unwrap_or(
                ::core::primitive::str::strip_prefix(STR, "_"),
                STR,
            );
            ::core::result::Result::expect(
                $crate::UBig::from_str_radix(s, $radix),
                "invalid number",
            )
        }
    }};
}

/// Create an [IBig](crate::IBig) value.
///
/// Usually just pass use a numeric literal. This works for bases 2, 8, 10 or 16 using standard
/// prefixes:
/// ```
/// # use dashu_int::ibig;
/// let a = ibig!(100);
/// let b = ibig!(0b101);
/// let c = ibig!(0o202);
/// let d = ibig!(0x2ff);
///
/// let f = ibig!(-314159265358979323846264338327950288419716939937);
/// let g = ibig!(-0x1234567890123456789012345678901234567890123456789);
/// ```
///
/// For an arbitrary base, add `base N`:
/// ```
/// # use dashu_int::ibig;
/// let e = ibig!(-a3gp1 base 32);
/// ```
///
/// This macro also supports using underscore as separaters in the string just like primitives integers,
/// this could be useful to increase the readability.
/// ```
/// # use dashu_int::ibig;
/// let f = ibig!(-3141_5926_5358_9793_2384_6264_3383_2795_0288_4197);
/// let g = ibig!(-3141592653589793238462643383279502884197);
/// assert_eq!(f, g);
/// let h = ibig!(-0x1234_5678_9abc_def0);
/// let i = ibig!(-0x123456789abcdef0);
/// assert_eq!(h, i);
/// ```
///
/// If the sequence of digits is not a valid Rust literal or identifier, put an underscore before
/// the digits. This may be necessary when the first digit is decimal, but not all digits are decimal.
/// ```
/// # use dashu_int::ibig;
/// let g = ibig!(_0b102 base 32);
/// let h = ibig!(b102 base 32);
/// assert_eq!(g, h);
/// let i = ibig!(-_100ef base 32);
/// ```
///
#[macro_export]
macro_rules! ibig {
    (- $val:tt) => {
        - <$crate::IBig as ::core::convert::From<$crate::UBig>>::from($crate::ubig!($val))
    };
    (- $val:tt base $radix:literal) => {
        - <$crate::IBig as ::core::convert::From<$crate::UBig>>::from(
            $crate::ubig!($val base $radix)
        )
    };
    ($val:tt) => {
        <$crate::IBig as ::core::convert::From<$crate::UBig>>::from($crate::ubig!($val))
    };
    ($val:tt base $radix:literal) => {
        <$crate::IBig as ::core::convert::From<$crate::UBig>>::from(
            $crate::ubig!($val base $radix)
        )
    };
}
