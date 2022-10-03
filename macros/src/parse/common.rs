use dashu_int::{DoubleWord, Sign, Word};
use proc_macro2::{Delimiter, Group, Literal, Punct, Spacing, TokenStream, TokenTree};
use quote::quote;

pub fn get_dword_from_words(words: &[Word]) -> Option<DoubleWord> {
    match *words {
        [] => Some(0),
        [word] => Some(word as DoubleWord),
        [low, high] => Some(low as DoubleWord | (high as DoubleWord) << Word::BITS),
        _ => None,
    }
}

pub fn quote_words(words: &[Word]) -> Group {
    let words_stream: TokenStream = words
        .iter()
        .flat_map(|&w| {
            [
                // currently Word bits <= 64
                TokenTree::Literal(Literal::u64_unsuffixed(w as u64)),
                TokenTree::Punct(Punct::new(',', Spacing::Alone)),
            ]
        })
        .collect();
    Group::new(Delimiter::Bracket, words_stream)
}

pub fn quote_sign(sign: Sign) -> TokenStream {
    #[cfg(not(feature = "embedded"))]
    match sign {
        Sign::Positive => quote! { ::dashu_base::Sign::Positive },
        Sign::Negative => quote! { ::dashu_base::Sign::Negative },
    }
    #[cfg(feature = "embedded")]
    match sign {
        Sign::Positive => quote! { ::dashu::base::Sign::Positive },
        Sign::Negative => quote! { ::dashu::base::Sign::Negative },
    }
}
