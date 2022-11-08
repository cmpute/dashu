use super::{common::quote_sign, int::quote_ibig};
use core::str::FromStr;

use dashu_base::BitTest;
use dashu_float::{round::mode, DBig, FBig};
use dashu_int::{Sign, IBig};
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

    if mag.bit_len() <= 32 {
        // if the significand fits in a u32, generates const expression
        let sign = quote_sign(sign);
        let u: u32 = mag.try_into().unwrap();

        #[cfg(not(feature = "embedded"))]
        quote! { ::dashu_float::FBig::<::dashu_float::round::mode::Zero, 2>
            ::from_parts_const(#sign, #u as _, #exp, Some(#prec)) }
        #[cfg(feature = "embedded")]
        quote! { ::dashu::float::FBig::<::dashu::float::round::mode::Zero, 2>
            ::from_parts_const(#sign, #u as _, #exp, Some(#prec)) }
    } else {
        let signif_tt = quote_ibig(IBig::from_parts(sign, mag));

        #[cfg(not(feature = "embedded"))]
        quote! {{
            let repr = ::dashu_float::Repr::<2>::new(#signif_tt, #exp);
            let context = ::dashu_float::Context::<::dashu_float::round::mode::Zero>::new(#prec);
            ::dashu_float::FBig::from_repr(repr, context)
        }}
        #[cfg(feature = "embedded")]
        quote! {{
            let repr = ::dashu::float::Repr::<2>::new(#signif_tt, #exp);
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

    if signif.bit_len() <= 32 {
        // if the significand fits in a u32, generates const expression
        let (sign, mag) = signif.into_parts();
        let u: u32 = mag.try_into().unwrap();
        let sign = quote_sign(sign);
        #[cfg(not(feature = "embedded"))]
        quote! { ::dashu_float::DBig::from_parts_const(#sign, #u as _, #exp, Some(#prec)) }
        #[cfg(feature = "embedded")]
        quote! { ::dashu::float::DBig::from_parts_const(#sign, #u as _, #exp, Some(#prec)) }
    } else {
        let signif_tt = quote_ibig(signif);

        #[cfg(not(feature = "embedded"))]
        quote! {{
            let repr = ::dashu_float::Repr::<10>::new(#signif_tt, #exp);
            let context = ::dashu_float::Context::new(#prec);
            ::dashu_float::DBig::from_repr(repr, context)
        }}
        #[cfg(feature = "embedded")]
        quote! {{
            let repr = ::dashu::float::Repr::<10>::new(#signif_tt, #exp);
            let context = ::dashu::float::Context::new(#prec);
            ::dashu::float::DBig::from_repr(repr, context)
        }}
    }
}
