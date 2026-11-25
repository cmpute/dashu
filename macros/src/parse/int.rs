use dashu_base::{BitTest, ParseError};
use dashu_int::{IBig, Sign, UBig};
use proc_macro2::{TokenStream, TokenTree};
use quote::quote;

use super::common::{quote_bytes, quote_sign, quote_words, unwrap_with_error_msg};

pub fn parse_integer(
    signed: bool,
    static_: bool,
    embedded: bool,
    input: TokenStream,
) -> TokenStream {
    let (sign, big) = unwrap_with_error_msg(parse_integer_with_error(signed, input));

    let ns = if embedded {
        quote!(::dashu::integer)
    } else {
        quote!(::dashu_int)
    };

    // if the integer fits in a u32, generates const expression
    if big.bit_len() <= 32 && !static_ {
        let u: u32 = big.try_into().unwrap();
        let sign = quote_sign(embedded, sign);

        return if signed {
            quote! { #ns::IBig::from_parts_const(#sign, #u as _) }
        } else {
            quote! { #ns::UBig::from_dword(#u as _) }
        };
    }

    match (signed, static_) {
        (false, false) => quote_ubig(embedded, big),
        (true, false) => quote_ibig(embedded, IBig::from_parts(sign, big)),
        (false, true) => {
            let data_defs = quote_words(&big.to_le_bytes(), embedded);
            quote! {{
                static DATA: &'static [#ns::Word] = #data_defs;
                static VALUE: #ns::UBig = unsafe { #ns::UBig::from_static_words(DATA) };
                &VALUE
            }}
        }
        (true, true) => {
            let data_defs = quote_words(&big.to_le_bytes(), embedded);
            let sign = quote_sign(embedded, sign);
            quote! {{
                static DATA: &'static [#ns::Word] = #data_defs;
                static VALUE: #ns::IBig = unsafe { #ns::IBig::from_static_words(#sign, DATA) };
                &VALUE
            }}
        }
    }
}

fn parse_integer_with_error(signed: bool, input: TokenStream) -> Result<(Sign, UBig), ParseError> {
    let mut val: Option<_> = None;
    let mut sign: Option<Sign> = None;
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
                } else if base.is_none() && !base_marked && ident == "base" {
                    base_marked = true
                } else {
                    return Err(ParseError::InvalidDigit);
                }
            }
            TokenTree::Punct(punct) => {
                // at most one sign, in front of the digits and only for signed integers
                if !signed || val.is_some() || sign.is_some() {
                    return Err(ParseError::InvalidDigit);
                }
                sign = Some(match punct.as_char() {
                    '-' => Sign::Negative,
                    '+' => Sign::Positive,
                    _ => return Err(ParseError::InvalidDigit),
                });
            }
            _ => return Err(ParseError::InvalidDigit),
        }
    }

    // forward the literal to proper format
    let val = val.ok_or(ParseError::NoDigits)?;
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

    Ok((sign.unwrap_or(Sign::Positive), big))
}

/// Generate tokens for creating a [UBig] instance (non-const)
pub fn quote_ubig(embedded: bool, int: UBig) -> TokenStream {
    debug_assert!(int.bit_len() > 32);
    let bytes = int.to_le_bytes();
    let len = bytes.len();
    let bytes_tt = quote_bytes(&bytes);

    let ns = if embedded {
        quote!(::dashu::integer)
    } else {
        quote!(::dashu_int)
    };
    quote! {{
        const BYTES: [u8; #len] = #bytes_tt;
        #ns::UBig::from_le_bytes(&BYTES)
    }}
}

/// Generate tokens for creating a [IBig] instance (non-const)
pub fn quote_ibig(embedded: bool, int: IBig) -> TokenStream {
    debug_assert!(int.bit_len() > 32);
    let (sign, mag) = int.into_parts();
    let sign = quote_sign(embedded, sign);
    let mag_tt = quote_ubig(embedded, mag);

    let ns: TokenStream = if embedded {
        quote!(::dashu::integer)
    } else {
        quote!(::dashu_int)
    };
    quote! { #ns::IBig::from_parts(#sign, #mag_tt) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(signed: bool, text: &str) -> Result<(Sign, UBig), ParseError> {
        parse_integer_with_error(signed, text.parse().unwrap())
    }

    #[test]
    fn test_single_sign_only() {
        assert_eq!(parse(true, "-5"), Ok((Sign::Negative, UBig::from(5u8))));
        assert_eq!(parse(true, "+5"), Ok((Sign::Positive, UBig::from(5u8))));
        assert_eq!(parse(true, "- a base 16"), Ok((Sign::Negative, UBig::from(10u8))));

        // repeated or mixed signs are rejected, as by the runtime parsers
        for text in ["--5", "- - 5", "+-5", "-+5", "++5", "---5", "5-", "-"] {
            assert!(parse(true, text).is_err(), "{:?} was accepted", text);
        }
        // unsigned literals take no sign at all
        for text in ["-5", "+5", ""] {
            assert!(parse(false, text).is_err(), "{:?} was accepted", text);
        }
    }

    #[test]
    fn test_single_base_marker_only() {
        assert_eq!(parse(false, "5 base 36"), Ok((Sign::Positive, UBig::from(5u8))));
        assert_eq!(parse(false, "z base 36"), Ok((Sign::Positive, UBig::from(35u8))));
        assert_eq!(parse(true, "-z base 36"), Ok((Sign::Negative, UBig::from(35u8))));
        // `base` itself is a valid digit string in a large enough base
        assert_eq!(parse(false, "base base 36"), Ok((Sign::Positive, UBig::from(527198u32))));

        // a repeated, dangling or misplaced marker is rejected, as by `rbig!`
        for text in [
            "5 base base 36",
            "z base base 36",
            "base base base 36",
            "5 base",
            "5 base base",
            "5 base 36 base",
            "5 base 36 36",
            "5 36",
            "5 base x",
            "base 36",
        ] {
            assert!(parse(false, text).is_err(), "{:?} was accepted", text);
            assert!(parse(true, text).is_err(), "{:?} was accepted", text);
        }
        assert!(parse(true, "-5 base base 36").is_err());
    }
}
