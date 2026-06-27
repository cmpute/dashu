//! Parser for the `cbig!` literal macro. Accepts the algebraic `a+bi` form (reusing the runtime
//! `CBig::FromStr` grammar) or a `re, im` pair, e.g. `cbig!(11+100i)`, `cbig!(111)`, `cbig!(11, -100)`.

use super::float::{gen_binary_fbig_value, gen_binary_repr_const};
use core::str::FromStr;
use dashu_cmplx::CBig;
use dashu_float::FBig;
use proc_macro2::TokenStream;
use quote::quote;

fn panic_cbig_syntax() -> ! {
    panic!("Incorrect syntax, please refer to the docs for acceptable complex literal formats.")
}

/// Parse a base-2 `FBig` coefficient (the same grammar as `fbig!`).
fn parse_coeff(s: &str) -> FBig {
    FBig::from_str(s.trim()).unwrap_or_else(|_| panic_cbig_syntax())
}

pub fn parse_complex(static_: bool, embedded: bool, input: TokenStream) -> TokenStream {
    let value_str: String = input.into_iter().map(|tt| tt.to_string()).collect();
    let value_str = value_str.trim();

    // `re, im` pair (im is a plain real coefficient) vs the algebraic `a+bi` form.
    let z = if let Some((re_s, im_s)) = value_str.split_once(',') {
        CBig::from_parts(parse_coeff(re_s), parse_coeff(im_s))
    } else {
        CBig::from_str(value_str).unwrap_or_else(|_| panic_cbig_syntax())
    };
    let (re, im) = z.into_parts();

    let ns = if embedded {
        quote!(::dashu::complex)
    } else {
        quote!(::dashu_cmplx)
    };

    if static_ {
        // const construction: each Repr via from_static_words (or Repr::zero() for a zero coeff),
        // then from_repr_parts.
        let (re_repr, prec_re) = gen_binary_repr_const(embedded, &re);
        let (im_repr, prec_im) = gen_binary_repr_const(embedded, &im);
        let prec = prec_re.max(prec_im);
        quote! {{
            static VALUE: #ns::CBig = #ns::CBig::from_repr_parts(
                #re_repr,
                #im_repr,
                #ns::Context::new(#prec),
            );
            &VALUE
        }}
    } else {
        let re_tt = gen_binary_fbig_value(embedded, &re);
        let im_tt = gen_binary_fbig_value(embedded, &im);
        quote! { #ns::CBig::from_parts(#re_tt, #im_tt) }
    }
}
