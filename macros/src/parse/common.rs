use dashu_base::{ParseError, Sign};
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

pub fn quote_sign(embedded: bool, sign: Sign) -> TokenStream {
    if !embedded {
        match sign {
            Sign::Positive => quote! { ::dashu_base::Sign::Positive },
            Sign::Negative => quote! { ::dashu_base::Sign::Negative },
        }
    } else {
        match sign {
            Sign::Positive => quote! { ::dashu::base::Sign::Positive },
            Sign::Negative => quote! { ::dashu::base::Sign::Negative },
        }
    }
}

pub fn print_error_msg(error_type: ParseError) -> ! {
    match error_type {
        ParseError::NoDigits => panic!("Missing digits in the number components!"),
        ParseError::InvalidDigit => panic!("Invalid digits or syntax in the literal! Please refer to the documentation of this macro."),
        ParseError::UnsupportedRadix => panic!("The given radix is invalid or unsupported!"),
        ParseError::InconsistentRadix => panic!("Radix of different components are different!"),
    }
}
