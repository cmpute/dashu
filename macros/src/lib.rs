use proc_macro::TokenStream;
use quote::quote;

mod parse;

#[proc_macro]
pub fn ubig(input: TokenStream) -> TokenStream {
    parse::int::parse_integer::<false>(proc_macro2::TokenStream::from(input)).into()
}

#[proc_macro]
pub fn ibig(input: TokenStream) -> TokenStream {
    parse::int::parse_integer::<true>(proc_macro2::TokenStream::from(input)).into()
}

#[proc_macro]
pub fn fbig(input: TokenStream) -> TokenStream {
    for token in proc_macro2::TokenStream::from(input) {
        dbg!(&token);
    }
    quote! { 1 }.into()
}
