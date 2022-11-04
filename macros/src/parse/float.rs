use super::common::{quote_bytes, quote_sign};
use core::str::FromStr;

use dashu_float::{round::mode, DBig, FBig};
use dashu_int::Sign;
use proc_macro2::TokenStream;
use quote::quote;

fn panic_fbig_syntax() -> ! {
    panic!("Incorrect syntax, please refer to the docs for acceptable float literal formats.")
}

pub fn parse_binary_float(input: TokenStream) -> TokenStream {
    let mut value_str = String::new();
    input
        .into_iter()
        .for_each(|tt| value_str.push_str(&tt.to_string()));

    // parse and remove the sign
    let mut value_str = value_str.as_str();
    let sign = match value_str.strip_prefix('-') {
        Some(s) => {
            value_str = s;
            Sign::Negative
        }
        None => {
            value_str = value_str.strip_prefix('+').unwrap_or(value_str);
            Sign::Positive
        }
    };

    // allow one underscore prefix
    let value_str = value_str.strip_prefix('_').unwrap_or(value_str);

    // generate expressions
    let f = FBig::<mode::Zero, 2>::from_str(value_str).unwrap_or_else(|_| panic_fbig_syntax());
    let prec = f.precision();
    let (signif, exp) = f.into_repr().into_parts();
    assert!(signif.sign() == Sign::Positive);
    let mag = signif.into_parts().1;
    let sign = quote_sign(sign);

    if mag.bit_len() <= 32 {
        // if the significand fits in a u32, generates const expression
        let u: u32 = mag.try_into().unwrap();
        #[cfg(not(feature = "embedded"))]
        quote! { ::dashu_float::FBig::<::dashu_float::round::mode::Zero, 2>::from_parts_const(#sign, #u as _, #exp, Some(#prec)) }
        #[cfg(feature = "embedded")]
        quote! { ::dashu::float::FBig::<::dashu::float::round::mode::Zero, 2>::from_parts_const(#sign, #u as _, #exp, Some(#prec)) }
    } else {
        // otherwise, convert to a byte array
        let bytes = mag.to_le_bytes();
        let len = bytes.len();
        let bytes_tt = quote_bytes(&bytes);
        #[cfg(not(feature = "embedded"))]
        quote! {{
            const BYTES: [u8; #len] = #bytes_tt;
            let signif = ::dashu_int::IBig::from_parts(#sign, ::dashu_int::UBig::from_le_bytes(&BYTES));
            let repr = ::dashu_float::Repr::<2>::new(signif, #exp);
            let context = ::dashu_float::Context::<::dashu_float::round::mode::Zero>::new(#prec);
            ::dashu_float::FBig::from_repr(repr, context)
        }}
        #[cfg(feature = "embedded")]
        quote! {{
            const BYTES: [u8; #len] = #bytes_tt;
            let signif = ::dashu::integer::IBig::from_parts(#sign, ::dashu::integer::UBig::from_le_bytes(&BYTES));
            let repr = ::dashu::float::Repr::<2>::new(signif, #exp);
            let context = ::dashu::float::Context::<::dashu::float::round::mode::Zero>::new(#prec);
            ::dashu::float::FBig::from_repr(repr, context)
        }}
    }
}

pub fn parse_decimal_float(input: TokenStream) -> TokenStream {
    let mut value_str = String::new();
    input
        .into_iter()
        .for_each(|tt| value_str.push_str(&tt.to_string()));

    let f = DBig::from_str(&value_str).unwrap_or_else(|_| panic_fbig_syntax());
    let prec = f.precision();
    let (signif, exp) = f.into_repr().into_parts();
    let (sign, mag) = signif.into_parts();
    let sign = quote_sign(sign);

    if mag.bit_len() <= 32 {
        // if the significand fits in a u32, generates const expression
        let u: u32 = mag.try_into().unwrap();
        #[cfg(not(feature = "embedded"))]
        quote! { ::dashu_float::DBig::from_parts_const(#sign, #u as _, #exp, Some(#prec)) }
        #[cfg(feature = "embedded")]
        quote! { ::dashu::float::DBig::from_parts_const(#sign, #u as _, #exp, Some(#prec)) }
    } else {
        // otherwise, convert to a byte array
        let bytes = mag.to_le_bytes();
        let len = bytes.len();
        let bytes_tt = quote_bytes(&bytes);
        #[cfg(not(feature = "embedded"))]
        quote! {{
            const BYTES: [u8; #len] = #bytes_tt;
            let signif = ::dashu_int::IBig::from_parts(#sign, ::dashu_int::UBig::from_le_bytes(&BYTES));
            let repr = ::dashu_float::Repr::<10>::new(signif, #exp);
            let context = ::dashu_float::Context::new(#prec);
            ::dashu_float::DBig::from_repr(repr, context)
        }}
        #[cfg(feature = "embedded")]
        quote! {{
            const BYTES: [u8; #len] = #bytes_tt;
            let signif = ::dashu::integer::IBig::from_parts(#sign, ::dashu::integer::UBig::from_le_bytes(&BYTES));
            let repr = ::dashu::float::Repr::<10>::new(signif, #exp);
            let context = ::dashu::float::Context::new(#prec);
            ::dashu::float::DBig::from_repr(repr, context)
        }}
    }
}
