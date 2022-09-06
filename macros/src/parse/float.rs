use super::int::{quote_sign, quote_words};
use core::str::FromStr;

use dashu_float::{round::mode, DBig, FBig};
use dashu_int::{DoubleWord, Sign, Word};
use proc_macro2::TokenStream;
use quote::quote;

fn panic_fbig_syntax() -> ! {
    panic!("Incorrect syntax, please refer to the docs for acceptable float literal formats.")
}

// XXX: support arbitrary base?
pub fn parse_binary_float(input: TokenStream) -> TokenStream {
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
    let value_str = match value_str.strip_prefix('_') {
        Some(s) => s,
        None => &value_str,
    };

    // generate expressions
    let (man, exp) = FBig::<mode::Zero, 2>::from_str(value_str)
        .unwrap_or_else(|_| panic_fbig_syntax())
        .into_parts();
    assert!(man.sign() == Sign::Positive);
    let sign = quote_sign(sign);
    let words = man.as_sign_words().1;
    let n_words = words.len();
    if n_words <= 2 {
        // the number is small enough to fit a double word, generates const expression
        let low = *words.get(0).unwrap_or(&0);
        let high = *words.get(1).unwrap_or(&0);
        let dword = low as DoubleWord | (high as DoubleWord) << Word::BITS;
        quote! { ::dashu_float::FBig::<::dashu_float::round::mode::Zero, 2>::from_parts_const(#sign, #dword, #exp) }
    } else {
        // the number contains more than two words, convert to array of words
        let words_tt = quote_words(words);
        quote! {{
            const WORDS: [::dashu_int::Word; #n_words] = #words_tt;
            let man = ::dashu_int::IBig::from_parts(#sign, ::dashu_int::UBig::from_words(&WORDS));
            ::dashu_float::FBig::<::dashu_float::round::mode::Zero, 2>::from_parts(man, #exp)
        }}
    }
}

pub fn parse_decimal_float(input: TokenStream) -> TokenStream {
    let mut value_str = String::new();
    input
        .into_iter()
        .for_each(|tt| value_str.push_str(&tt.to_string()));

    let f = DBig::from_str(&value_str).unwrap_or_else(|_| panic_fbig_syntax());
    let (man, exp) = f.into_parts();
    let (sign, words) = man.as_sign_words();
    let sign = quote_sign(sign);
    let n_words = words.len();
    if n_words <= 2 {
        // the number is small enough to fit a double word, generates const expression
        let low = *words.get(0).unwrap_or(&0);
        let high = *words.get(1).unwrap_or(&0);
        let dword = low as DoubleWord | (high as DoubleWord) << Word::BITS;
        quote! { ::dashu_float::DBig::from_parts_const(#sign, #dword, #exp) }
    } else {
        // the number contains more than two words, convert to array of words
        let words_tt = quote_words(words);
        quote! {{
            const WORDS: [::dashu_int::Word; #n_words] = #words_tt;
            let man = ::dashu_int::IBig::from_parts(#sign, ::dashu_int::UBig::from_words(&WORDS));
            ::dashu_float::DBig::from_parts(man, #exp)
        }}
    }
}
