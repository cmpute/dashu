use dashu_int::{IBig, Sign, UBig};
use proc_macro2::{TokenStream, TokenTree};
use quote::quote;

use super::common::{get_dword_from_words, quote_sign, quote_words};

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

    // generate output tokens
    if SIGNED {
        let sign = if neg { Sign::Negative } else { Sign::Positive };
        quote_ibig(IBig::from_parts(sign, big))
    } else {
        quote_ubig(big)
    }
}

pub fn quote_ubig(int: UBig) -> TokenStream {
    let words = int.as_words();
    if let Some(dword) = get_dword_from_words(words) {
        quote! { ::dashu_int::UBig::from_dword(#dword) }
    } else {
        // the number contains more than two words, convert to array of words
        let n_words = words.len();
        let words_tt = quote_words(words);
        quote! {{
            const WORDS: [::dashu_int::Word; #n_words] = #words_tt;
            ::dashu_int::UBig::from_words(&WORDS)
        }}
    }
}

pub fn quote_ibig(int: IBig) -> TokenStream {
    let (sign, mag) = int.into_parts();
    let sign = quote_sign(sign);
    let words = mag.as_words();
    if let Some(dword) = get_dword_from_words(words) {
        quote! { ::dashu_int::IBig::from_parts_const(#sign, #dword) }
    } else {
        // the number contains more than two words, convert to array of words
        let n_words = words.len();
        let words_tt = quote_words(words);
        quote! {{
            const WORDS: [::dashu_int::Word; #n_words] = #words_tt;
            ::dashu_int::IBig::from_parts(#sign, ::dashu_int::UBig::from_words(&WORDS))
        }}
    }
}
