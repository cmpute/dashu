use super::{
    common::quote_sign,
    int::{quote_ibig, quote_ubig},
};
use core::str::FromStr;

use dashu_base::BitTest;
use dashu_ratio::{RBig, Relaxed};
use proc_macro2::TokenStream;
use quote::quote;

fn panic_rbig_syntax() -> ! {
    panic!("Incorrect syntax, please refer to the docs for acceptable rational formats.")
}

pub fn parse_ratio(embedded: bool, input: TokenStream) -> TokenStream {
    // TODO: support parsing from non-trivial base and with optional prefix and add docs examples
    let mut value_str = String::new();
    input
        .into_iter()
        .for_each(|tt| value_str.push_str(&tt.to_string()));

    // parse and remove the relaxed prefix and the sign
    let mut value_str = value_str.as_str();
    let relaxed = match value_str.strip_prefix('~') {
        Some(s) => {
            value_str = s;
            true
        }
        None => false,
    };

    // generate expressions
    let (num, den) = if relaxed {
        Relaxed::from_str(value_str)
            .unwrap_or_else(|_| panic_rbig_syntax())
            .into_parts()
    } else {
        RBig::from_str(value_str)
            .unwrap_or_else(|_| panic_rbig_syntax())
            .into_parts()
    };

    if num.bit_len() <= 32 && den.bit_len() <= 32 {
        // if the numerator and denominator fit in a u32, generates const expression
        let (sign, num) = num.into_parts();
        let sign = quote_sign(embedded, sign);
        let num: u32 = num.try_into().unwrap();
        let den: u32 = den.try_into().unwrap();

        if relaxed {
            if !embedded {
                quote! { ::dashu_ratio::Relaxed::from_parts_const(#sign, #num as _, #den as _) }
            } else {
                quote! { ::dashu::rational::Relaxed::from_parts_const(#sign, #num as _, #den as _) }
            }
        } else if !embedded {
            quote! { ::dashu_ratio::RBig::from_parts_const(#sign, #num as _, #den as _) }
        } else {
            quote! { ::dashu::rational::RBig::from_parts_const(#sign, #num as _, #den as _) }
        }
    } else {
        let (num_tt, den_tt) = (quote_ibig(embedded, num), quote_ubig(embedded, den));

        if relaxed {
            if !embedded {
                quote! { ::dashu_ratio::Relaxed::from_parts(#num_tt, #den_tt) }
            } else {
                quote! { ::dashu::rational::Relaxed::from_parts(#num_tt, #den_tt) }
            }
        } else if !embedded {
            quote! { ::dashu_ratio::RBig::from_parts(#num_tt, #den_tt) }
        } else {
            quote! { ::dashu::rational::RBig::from_parts(#num_tt, #den_tt) }
        }
    }
}
