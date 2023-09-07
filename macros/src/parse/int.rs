use dashu_base::{BitTest, ParseError};
use dashu_int::{IBig, Sign, UBig};
use proc_macro2::{TokenStream, TokenTree};
use quote::quote;

use super::common::{print_error_msg, quote_bytes, quote_sign};

pub fn parse_integer<const SIGNED: bool>(embedded: bool, input: TokenStream) -> TokenStream {
    match parse_integer_with_error::<SIGNED>(input) {
        Ok((sign, big)) => {
            // if the integer fits in a u32, generates const expression
            if big.bit_len() <= 32 {
                let u: u32 = big.try_into().unwrap();
                let sign = quote_sign(embedded, sign);
                if SIGNED {
                    if !embedded {
                        quote! { ::dashu_int::IBig::from_parts_const(#sign, #u as _) }
                    } else {
                        quote! { ::dashu::integer::IBig::from_parts_const(#sign, #u as _) }
                    }
                } else if !embedded {
                    quote! { ::dashu_int::UBig::from_dword(#u as _) }
                } else {
                    quote! { ::dashu::integer::UBig::from_dword(#u as _) }
                }
            } else if SIGNED {
                quote_ibig(embedded, IBig::from_parts(sign, big))
            } else {
                quote_ubig(embedded, big)
            }
        }
        Err(e) => print_error_msg(e),
    }
}

fn parse_integer_with_error<const SIGNED: bool>(
    input: TokenStream,
) -> Result<(Sign, UBig), ParseError> {
    let mut val: Option<_> = None;
    let mut neg = false;
    let mut base_marked = false;
    let mut base: Option<_> = None;

    // parse tokens
    for token in input {
        match token {
            TokenTree::Literal(lit) => {
                if val.is_none() {
                    val = Some(lit.to_string());
                } else if base.is_none() && base_marked {
                    base = Some(lit.to_string());
                } else {
                    return Err(ParseError::InvalidDigit);
                }
            }
            TokenTree::Ident(ident) => {
                if val.is_none() {
                    val = Some(ident.to_string()) // this accepts numbers starting with non-base 10 digits
                } else if base.is_none() && ident == "base" {
                    base_marked = true
                } else {
                    return Err(ParseError::InvalidDigit);
                }
            }
            TokenTree::Punct(punct) => {
                if val.is_none() && punct.as_char() == '-' {
                    if SIGNED {
                        neg = true;
                    } else {
                        return Err(ParseError::InvalidDigit);
                    }
                } else if val.is_none() && punct.as_char() == '+' {
                    if !SIGNED {
                        return Err(ParseError::InvalidDigit);
                    }
                } else {
                    return Err(ParseError::InvalidDigit);
                }
            }
            _ => return Err(ParseError::InvalidDigit),
        }
    }

    // forward the literal to proper format
    let val = val.unwrap();
    let big = match base {
        Some(b) => {
            let b = b.parse::<u32>().or(Err(ParseError::UnsupportedRadix))?;
            UBig::from_str_radix(&val, b)?
        }
        None => {
            if base_marked {
                return Err(ParseError::UnsupportedRadix);
            } else {
                UBig::from_str_with_radix_prefix(&val)?.0
            }
        }
    };

    Ok((Sign::from(neg), big))
}

/// Generate tokens for creating a [UBig] instance (non-const)
pub fn quote_ubig(embedded: bool, int: UBig) -> TokenStream {
    debug_assert!(int.bit_len() > 32);
    let bytes = int.to_le_bytes();
    let len = bytes.len();
    let bytes_tt = quote_bytes(&bytes);

    if !embedded {
        quote! {{
            const BYTES: [u8; #len] = #bytes_tt;
            ::dashu_int::UBig::from_le_bytes(&BYTES)
        }}
    } else {
        quote! {{
            const BYTES: [u8; #len] = #bytes_tt;
            ::dashu::integer::UBig::from_le_bytes(&BYTES)
        }}
    }
}

/// Generate tokens for creating a [IBig] instance (non-const)
pub fn quote_ibig(embedded: bool, int: IBig) -> TokenStream {
    debug_assert!(int.bit_len() > 32);
    let (sign, mag) = int.into_parts();
    let sign = quote_sign(embedded, sign);
    let mag_tt = quote_ubig(embedded, mag);

    if !embedded {
        quote! { ::dashu_int::IBig::from_parts(#sign, #mag_tt) }
    } else {
        quote! { ::dashu::integer::IBig::from_parts(#sign, #mag_tt) }
    }
}
