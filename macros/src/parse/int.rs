use dashu_int::UBig;
use proc_macro2::{Delimiter, Group, Literal, Punct, Spacing, TokenStream, TokenTree};
use quote::quote;

fn panic_ubig_syntax() -> ! {
    panic!("Incorrect syntax, the correct syntax is like ubig!(1230) or ubig(1230 base 4)")
}

fn panic_ibig_syntax() -> ! {
    panic!("Incorrect syntax, the correct syntax is like ibig!(-1230) or ibig(-1230 base 4)")
}

fn panic_ubig_no_sign() -> ! {
    panic!("`ubig!` expression shouldn't contain a sign.")
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
                } else if base.is_none() && ident.to_string() == "base" {
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
            let b = u32::from_str_radix(&b, 10).unwrap();
            UBig::from_str_radix(&val, b)
                .expect(&format!("Some digits are not valid under base {}", b))
        }
        None => UBig::from_str_with_radix_prefix(&val).expect("Some digits are not valid").0,
    };

    // generate output tokens
    let n_words = big.as_words().len();
    let sign = if neg {
        quote! { ::dashu_int::Sign::Negative }
    } else {
        quote! { ::dashu_int::Sign::Positive }
    };
    let tokens = if n_words <= 2 {
        // the number is small enough to fit a double word,
        // in this case the output is guaranteed to be a const expression
        let words = big.as_words();
        let low = *words.get(0).unwrap_or(&0);
        let high = *words.get(1).unwrap_or(&0);
        if SIGNED {
            quote! { ::dashu_int::IBig::from_sign_dword(#sign, #low, #high) }
        } else {
            quote! { ::dashu_int::UBig::from_dword(#low, #high) }
        }
    } else {
        // the number contains more than two words, then the words will be stored
        // in an static array and converted to integer later.

        // generate an array of words
        let words_stream: TokenStream = big
            .as_words()
            .iter()
            .map(|&w| {
                [
                    TokenTree::Literal(Literal::u64_unsuffixed(w)),
                    TokenTree::Punct(Punct::new(',', Spacing::Alone)),
                ]
            })
            .flatten()
            .collect();
        let words = Group::new(Delimiter::Bracket, words_stream);

        // generate the actual expression
        if SIGNED {
            quote! {{
                const WORDS: [::dashu_int::Word; #n_words] = #words;
                ::dashu_int::IBig::from_sign_words(#sign, &WORDS)
            }}
        } else {
            quote! {{
                const WORDS: [::dashu_int::Word; #n_words] = #words;
                ::dashu_int::UBig::from_words(&WORDS)
            }}
        }
    };

    tokens
}
