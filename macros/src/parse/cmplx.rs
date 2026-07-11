//! Parser for the `cbig!` literal macro. Accepts the algebraic `a+bi` form (reusing the runtime
//! `CBig::FromStr` grammar) or a `re, im` pair, e.g. `cbig!(11+100i)`, `cbig!(111)`, `cbig!(11, -100)`.

use super::common::quote_sign;
use super::float::{gen_binary_fbig_value, gen_binary_repr_const};
use core::str::FromStr;
use dashu_base::{BitTest, Sign};
use dashu_cmplx::CBig;
use dashu_float::FBig;
use proc_macro2::TokenStream;
use quote::quote;

fn panic_cbig_syntax() -> ! {
    panic!("Incorrect syntax, please refer to the docs for acceptable complex literal formats.")
}

/// Extract `(sign, significand, exponent)` from a base-2 `FBig` coefficient when its significand
/// fits in a `u32`, so the coefficient can be reconstructed via the const-friendly
/// `CBig::from_parts_const`. Returns `None` for non-finite or too-large coefficients, in which
/// case the caller falls back to the runtime heap path (`CBig::from_parts`).
fn small_coeff(f: &FBig) -> Option<(Sign, u32, isize)> {
    if f.repr().is_infinite() {
        return None;
    }
    let (signif, exp) = f.clone().into_repr().into_parts();
    let (sign, mag) = signif.into_parts();
    if mag.bit_len() <= 32 {
        let u: u32 = mag.try_into().unwrap();
        Some((sign, u, exp))
    } else {
        None
    }
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
    let ns_f = if embedded {
        quote!(::dashu::float)
    } else {
        quote!(::dashu_float)
    };

    if static_ {
        // const construction: each Repr via from_static_words (or Repr::zero() for a zero coeff),
        // then new.
        let (re_repr, prec_re) = gen_binary_repr_const(embedded, &re);
        let (im_repr, prec_im) = gen_binary_repr_const(embedded, &im);
        let prec = prec_re.max(prec_im);
        quote! {{
            static VALUE: #ns::CBig = #ns::CBig::new(
                #re_repr,
                #im_repr,
                #ns::Context::new(#prec),
            );
            &VALUE
        }}
    } else {
        // When both coefficients fit in a DoubleWord, emit const-friendly construction
        // (CBig::from_parts_const, which builds inline Reprs without heap arithmetic) so that
        // `cbig!` works in const position for small literals — matching `fbig!`. Otherwise fall
        // back to the runtime heap path through CBig::from_parts.
        match (small_coeff(&re), small_coeff(&im)) {
            (Some((re_sign, re_u, re_exp)), Some((im_sign, im_u, im_exp))) => {
                let prec = re.precision().max(im.precision());
                let re_sign = quote_sign(embedded, re_sign);
                let im_sign = quote_sign(embedded, im_sign);
                quote! {
                    #ns::CBig::<#ns_f::round::mode::Zero, 2>::from_parts_const(
                        (#re_sign, #re_u as _, #re_exp),
                        (#im_sign, #im_u as _, #im_exp),
                        #prec,
                    )
                }
            }
            _ => {
                let re_tt = gen_binary_fbig_value(embedded, &re);
                let im_tt = gen_binary_fbig_value(embedded, &im);
                quote! { #ns::CBig::from_parts(#re_tt, #im_tt) }
            }
        }
    }
}
