use super::{
    common::{quote_sign, unwrap_with_error_msg, quote_words},
    int::{quote_ibig, quote_ubig},
};

use dashu_base::{BitTest, ParseError, Sign};
use dashu_int::{IBig, UBig};
use dashu_ratio::{RBig, Relaxed};
use proc_macro2::{TokenStream, TokenTree};
use quote::quote;

pub fn parse_ratio(embedded: bool, input: TokenStream) -> TokenStream {
    let (num, den, relaxed) = unwrap_with_error_msg(parse_ratio_with_error(input));

    let (ns, int_ns) = if embedded {
        (quote!(::dashu::rational), quote!(::dashu::integer))
    } else {
        (quote!(::dashu_ratio), quote!(::dashu_int))
    };
    let type_tt = if relaxed {
        quote!( #ns::Relaxed )
    } else {
        quote!( #ns::RBig )
    };

    // if the numerator and denominator fit in a u32, generates const expression
    if num.bit_len() <= 32 && den.bit_len() <= 32 {
        let (sign, num) = num.into_parts();
        let sign = quote_sign(embedded, sign);
        let num: u32 = num.try_into().unwrap();
        let den: u32 = den.try_into().unwrap();
        return quote! { #type_tt::from_parts_const(#sign, #num as _, #den as _) };
    }

    let num_tt = if num.bit_len() <= 32 {
        let (sign, num) = num.into_parts();
        let sign = quote_sign(embedded, sign);
        let u: u32 = num.try_into().unwrap();
        quote!( #int_ns::IBig::from_parts_const(#sign, #u as _) )
    } else {
        quote_ibig(embedded, num)
    };
    let den_tt = if den.bit_len() <= 32 {
        let u: u32 = den.try_into().unwrap();
        quote!( #int_ns::UBig::from_dword(#u as _) )
    } else {
        quote_ubig(embedded, den)
    };
    quote! { #type_tt::from_parts(#num_tt, #den_tt) }
}

pub fn parse_static_ratio(embedded: bool, input: TokenStream) -> TokenStream {
    let (num, den, relaxed) = unwrap_with_error_msg(parse_ratio_with_error(input));
    
    let ns = if embedded {
        quote!(::dashu::rational)
    } else {
        quote!(::dashu_ratio)
    };
    let type_tt = if relaxed {
        quote!( #ns::Relaxed )
    } else {
        quote!( #ns::RBig )
    };

    // if the numerator and denominator fit in a u32, generates const expression
    if num.bit_len() <= 32 && den.bit_len() <= 32 {
        let (sign, num) = num.into_parts();
        let sign = quote_sign(embedded, sign);
        let num: u32 = num.try_into().unwrap();
        let den: u32 = den.try_into().unwrap();
        let value_tt = quote! { #type_tt::from_parts_const(#sign, #num as _, #den as _) };
        return quote! {{ static VALUE: #type_tt = #value_tt; &VALUE }};
    }

    let (sign, num) = num.into_parts();
    let num_data_defs = quote_words(&num.to_le_bytes(), embedded);
    let den_data_defs = quote_words(&den.to_le_bytes(), embedded);
    let sign = quote_sign(embedded, sign);
    if relaxed {
        quote! {{
            static NUM_DATA: &'static [#ns::Word] = #num_data_defs;
            static DEN_DATA: &'static [#ns::Word] = #den_data_defs;
            static VALUE: #type_tt = unsafe {
                #ns::Relaxed::from_static_words(#sign, NUM_DATA, DEN_DATA)
            };
            &VALUE
        }}
    } else {
        quote! {{
            static NUM_DATA: &'static [#ns::Word] = #num_data_defs;
            static DEN_DATA: &'static [#ns::Word] = #den_data_defs;

            // here transmuting is safe because
            // 1) RBig and Relaxed has the same inner representation
            // 2) The numerator and denominator are reduced during parsing
            static VALUE: #type_tt = unsafe { core::mem::transmute(
                #ns::Relaxed::from_static_words(#sign, NUM_DATA, DEN_DATA)
            )};
            &VALUE
        }}
    }
}

fn parse_ratio_with_error(input: TokenStream) -> Result<(IBig, UBig, bool), ParseError> {
    let mut num_val: Option<_> = None;
    let mut num_neg = false;
    let mut den_val: Option<_> = None;
    let mut den_neg = false;
    let mut den_marked = false;
    let mut relaxed = false;
    let mut base_marked = false;
    let mut base: Option<_> = None;

    // parse tokens
    for token in input {
        match token {
            TokenTree::Literal(lit) => {
                if num_val.is_none() {
                    num_val = Some(lit.to_string());
                } else if den_val.is_none() {
                    den_val = Some(lit.to_string());
                } else if base.is_none() && base_marked {
                    base = Some(lit.to_string());
                } else {
                    return Err(ParseError::InvalidDigit);
                }
            }
            TokenTree::Ident(ident) => {
                if num_val.is_none() {
                    num_val = Some(ident.to_string())
                } else if den_val.is_none() {
                    den_val = Some(ident.to_string());
                } else if base.is_none() && ident == "base" {
                    base_marked = true
                } else {
                    return Err(ParseError::InvalidDigit);
                }
            }
            TokenTree::Punct(punct) => {
                if punct.as_char() == '/' {
                    if !den_marked && !base_marked {
                        den_marked = true;
                    } else {
                        return Err(ParseError::InvalidDigit);
                    }
                } else if punct.as_char() == '~' {
                    if num_val.is_none() && den_val.is_none() {
                        relaxed = true;
                    } else {
                        return Err(ParseError::InvalidDigit);
                    }
                } else if num_val.is_none() {
                    if punct.as_char() == '-' {
                        num_neg = true;
                    } else if punct.as_char() != '+' {
                        return Err(ParseError::InvalidDigit);
                    }
                } else if den_val.is_none() {
                    if punct.as_char() == '-' {
                        den_neg = true;
                    } else if punct.as_char() != '+' {
                        return Err(ParseError::InvalidDigit);
                    }
                } else {
                    return Err(ParseError::InvalidDigit);
                }
            }
            _ => return Err(ParseError::InvalidDigit),
        }
    }

    // generate expressions
    let num_val = num_val.ok_or(ParseError::NoDigits)?;
    let (num, den) = match base {
        Some(b) => {
            let b = b.parse::<u32>().or(Err(ParseError::UnsupportedRadix))?;
            (
                UBig::from_str_radix(&num_val, b)?,
                den_val.map_or(Ok(UBig::ONE), |val| UBig::from_str_radix(&val, b))?,
            )
        }
        None => {
            if base_marked {
                return Err(ParseError::UnsupportedRadix);
            } else {
                let (num_val, num_base) = UBig::from_str_with_radix_prefix(&num_val)?;
                if let Some(val) = den_val {
                    let (den_val, den_base) = UBig::from_str_with_radix_default(&val, num_base)?;
                    if num_base != den_base {
                        return Err(ParseError::InconsistentRadix);
                    }
                    (num_val, den_val)
                } else {
                    (num_val, UBig::ONE)
                }
            }
        }
    };

    // calculate components
    let (num, den) = if relaxed {
        Relaxed::from_parts_signed(Sign::from(num_neg) * num, Sign::from(den_neg) * den)
            .into_parts()
    } else {
        RBig::from_parts_signed(Sign::from(num_neg) * num, Sign::from(den_neg) * den).into_parts()
    };
    Ok((num, den, relaxed))
}
