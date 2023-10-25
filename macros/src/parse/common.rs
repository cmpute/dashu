use dashu_base::Sign;
use proc_macro2::{Delimiter, Group, Literal, Punct, Spacing, TokenStream, TokenTree};
use quote::quote;

pub fn quote_bytes(bytes: &[u8]) -> Group {
    let words_stream: TokenStream = bytes
        .iter()
        .flat_map(|&b| {
            [
                TokenTree::Literal(Literal::u8_unsuffixed(b)),
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
