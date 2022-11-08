use dashu_int::{IBig, Sign, UBig};
use proc_macro2::{TokenStream, TokenTree};
use quote::quote;

use super::common::{quote_bytes, quote_sign};

fn panic_ubig_syntax() -> ! {
    panic!("Incorrect syntax, the correct syntax is like ubig!(1230) or ubig(1230 base 4)")
}

fn panic_ibig_syntax() -> ! {
    panic!("Incorrect syntax, the correct syntax is like ibig!(-1230) or ibig(-1230 base 4)")
}

fn panic_ubig_no_sign() -> ! {
    panic!("`ubig!` expression shouldn't contain a sign.")
}

fn panic_base_invalid() -> ! {
    panic!("Empty or invalid base literal")
}

pub fn parse_integer<const SIGNED: bool>(input: TokenStream) -> TokenStream {
    let mut val: Option<_> = None;
    let mut neg = false;
    let mut base_marked: bool = false;
    let mut base: Option<_> = None;

    let panic_syntax = if SIGNED {
        panic_ibig_syntax
    } else {
        panic_ubig_syntax
    };

    // parse tokens
    for token in input {
        match token {
            TokenTree::Literal(lit) => {
                if val.is_none() {
                    val = Some(lit.to_string());
                } else if base.is_none() && base_marked {
                    base = Some(lit.to_string());
                } else {
                    panic_syntax();
                }
            }
            TokenTree::Ident(ident) => {
                if val.is_none() {
                    val = Some(ident.to_string())
                } else if base.is_none() && ident == "base" {
                    base_marked = true
                } else {
                    panic_syntax();
                }
            }
            TokenTree::Punct(punct) => {
                if val.is_none() && punct.as_char() == '-' {
                    if SIGNED {
                        neg = true;
                    } else {
                        panic_ubig_no_sign()
                    }
                } else if val.is_none() && punct.as_char() == '+' {
                    if !SIGNED {
                        panic_ubig_no_sign()
                    }
                } else {
                    panic_syntax()
                }
            }
            _ => panic_syntax(),
        }
    }

    // forward the literal to proper format
    let val = val.unwrap();
    let big = match base {
        Some(b) => {
            let b = b.parse::<u32>().unwrap();
            match UBig::from_str_radix(&val, b) {
                Ok(v) => v,
                Err(_) => panic_base_invalid(),
            }
        }
        None => {
            if base_marked {
                panic_base_invalid()
            } else {
                UBig::from_str_with_radix_prefix(&val)
                    .expect("Some digits are not valid")
                    .0
            }
        }
    };

    // if the integer fits in a u32, generates const expression
    let sign = if neg { Sign::Negative } else { Sign::Positive };
    if big.bit_len() <= 32 {
        let u: u32 = big.try_into().unwrap();
        let sign = quote_sign(sign);
        if SIGNED {
            #[cfg(not(feature = "embedded"))]
            quote! { ::dashu_int::IBig::from_parts_const(#sign, #u as _) }
            #[cfg(feature = "embedded")]
            quote! { ::dashu::integer::IBig::from_parts_const(#sign, #u as _) }
        } else {
            #[cfg(not(feature = "embedded"))]
            quote! { ::dashu_int::UBig::from_dword(#u as _) }
            #[cfg(feature = "embedded")]
            quote! { ::dashu::integer::UBig::from_dword(#u as _) }
        }
    } else {
        if SIGNED {
            quote_ibig(IBig::from_parts(sign, big))
        } else {
            quote_ubig(big)
        }
    }
}

// TODO(v0.3): only inline u32 ints, (then this will be platform agnostic), parse the big integer from bytes
//             instead of words?

/// Generate tokens for creating a [UBig] instance (non-const)
pub fn quote_ubig(int: UBig) -> TokenStream {
    debug_assert!(int.bit_len() > 32);
    let bytes = int.to_le_bytes();
    let len = bytes.len();
    let bytes_tt = quote_bytes(&bytes);
    #[cfg(not(feature = "embedded"))]
    quote! {{
        const BYTES: [u8; #len] = #bytes_tt;
        ::dashu_int::UBig::from_le_bytes(&BYTES)
    }}
    #[cfg(feature = "embedded")]
    quote! {{
        const BYTES: [u8; #len] = #bytes_tt;
        ::dashu::integer::UBig::from_le_bytes(&BYTES)
    }}
}

/// Generate tokens for creating a [IBig] instance (non-const)
pub fn quote_ibig(int: IBig) -> TokenStream {
    debug_assert!(int.bit_len() > 32);
    let (sign, mag) = int.into_parts();
    let sign = quote_sign(sign);
    let mag_tt = quote_ubig(mag);

    #[cfg(not(feature = "embedded"))]
    quote! { ::dashu_int::IBig::from_parts(#sign, #mag_tt) }
    #[cfg(feature = "embedded")]
    quote! { ::dashu::int::IBig::from_parts(#sign, #mag_tt) }
}
