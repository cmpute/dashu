use super::{
    common::{quote_sign, quote_words, unwrap_with_error_msg},
    int::quote_ibig,
};
use core::str::FromStr;

use dashu_base::{BitTest, Signed};
use dashu_float::{DBig, FBig};
use dashu_int::{IBig, Sign};
use proc_macro2::TokenStream;
use quote::quote;

fn panic_fbig_syntax() -> ! {
    panic!("Incorrect syntax, please refer to the docs for acceptable float literal formats.")
}

pub fn parse_binary_float(static_: bool, embedded: bool, input: TokenStream) -> TokenStream {
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
    type FBin = FBig; // use the default generic arguments
    let f = unwrap_with_error_msg(FBin::from_str(value_str));
    let prec = f.precision();
    let (signif, exp) = f.into_repr().into_parts();
    assert!(signif.is_positive());
    let mag = signif.into_parts().1;

    let ns = if embedded {
        quote!(::dashu::float)
    } else {
        quote!(::dashu_float)
    };
    let repr_tt = quote!( #ns::Repr::<2> );
    let type_tt = quote!( #ns::FBig::<#ns::round::mode::Zero, 2> );

    if mag.bit_len() <= 32 {
        // if the significand fits in a u32, generates const expression
        let sign = quote_sign(embedded, sign);
        let u: u32 = mag.try_into().unwrap();

        let value_tt = quote!( #type_tt::from_parts_const(#sign, #u as _, #exp, Some(#prec)) );
        return if static_ {
            quote! {{ static VALUE: #type_tt = #value_tt; &VALUE }}
        } else {
            value_tt
        };
    }

    if static_ {
        let data_defs = quote_words(&mag.to_le_bytes(), embedded);
        let sign = quote_sign(embedded, sign);
        quote! {{
            static DATA: &'static [#ns::Word] = #data_defs;
            static VALUE: #type_tt = unsafe {
                #type_tt::from_repr_const(#repr_tt::from_static_words(#sign, DATA, #exp))
            };
            &VALUE
        }}
    } else {
        let signif_tt = quote_ibig(embedded, IBig::from_parts(sign, mag));
        quote! {{
            let repr = #repr_tt::new(#signif_tt, #exp);
            let context = #ns::Context::<#ns::round::mode::Zero>::new(#prec);
            #ns::FBig::from_repr(repr, context)
        }}
    }
}

pub fn parse_decimal_float(static_: bool, embedded: bool, input: TokenStream) -> TokenStream {
    let mut value_str = String::new();
    input
        .into_iter()
        .for_each(|tt| value_str.push_str(&tt.to_string()));

    let f = DBig::from_str(&value_str).unwrap_or_else(|_| panic_fbig_syntax());
    let prec = f.precision();
    let (signif, exp) = f.into_repr().into_parts();
    let (sign, mag) = signif.into_parts();

    let ns = if embedded {
        quote!(::dashu::float)
    } else {
        quote!(::dashu_float)
    };

    // if the significand fits in a u32, generates const expression
    if mag.bit_len() <= 32 {
        let u: u32 = mag.try_into().unwrap();
        let sign = quote_sign(embedded, sign);

        let value_tt = quote!( #ns::DBig::from_parts_const(#sign, #u as _, #exp, Some(#prec)) );
        return if static_ {
            quote! {{ static VALUE: #ns::DBig = #value_tt; &VALUE }}
        } else {
            value_tt
        };
    }

    if static_ {
        let bytes = mag.to_le_bytes();
        let data_defs = quote_words(&bytes, embedded);
        let sign = quote_sign(embedded, sign);
        quote! {{
            static DATA: &'static [#ns::Word] = #data_defs;
            static VALUE: #ns::DBig = unsafe {
                #ns::DBig::from_repr_const(#ns::Repr::<10>::from_static_words(#sign, DATA, #exp))
            };
            &VALUE
        }}
    } else {
        let signif_tt = quote_ibig(embedded, IBig::from_parts(sign, mag));
        quote! {{
            let repr = #ns::Repr::<10>::new(#signif_tt, #exp);
            let context = #ns::Context::new(#prec);
            #ns::DBig::from_repr(repr, context)
        }}
    }
}
