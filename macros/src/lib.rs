// Copyright (c) 2022 Jacob Zhong
//
// Licensed under either of
//
// * Apache License, Version 2.0
//   (LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0)
// * MIT license
//   (LICENSE-MIT or https://opensource.org/licenses/MIT)
//
// at your option.
//
// Unless you explicitly state otherwise, any contribution intentionally submitted
// for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
// dual licensed as above, without any additional terms or conditions.

//! A macro library for creating big numbers from literals.
//!
//! See the documentation of each macro for the usage.
//!
//! # Dependency requirement
//!
//! Due the fact that the macros expand to plain tokens, and proc macro crates can't
//! re-export types, it's required to add explicit dependency to the underlying crates
//! when using the macros. Specifically, you need to add the following crates as dependencies
//! to your `Cargo.toml`:
//! * For [ubig!]/[static_ubig!] and [ibig!]/[static_ibig!]: `dashu-int`
//! * For [fbig!] and [dbig!]: `dashu-int`, `dashu-float`
//! * For [rbig!]: `dashu-int`, `dashu-ratio`
//!
//! If you are using these macros from the `dashu` crate, then it's not necessary to
//! explicitly adding these dependencies, because the related types are re-exported
//! by the `dashu` crate.

use proc_macro::TokenStream;

mod parse;

#[proc_macro]
#[doc = include_str!("../docs/ubig.md")]
pub fn ubig(input: TokenStream) -> TokenStream {
    parse::int::parse_integer(false, false, false, input.into()).into()
}

#[proc_macro]
#[doc = include_str!("../docs/static_ubig.md")]
pub fn static_ubig(input: TokenStream) -> TokenStream {
    parse::int::parse_integer(false, true, false, input.into()).into()
}

#[doc(hidden)]
#[proc_macro]
pub fn ubig_embedded(input: TokenStream) -> TokenStream {
    parse::int::parse_integer(false, false, true, input.into()).into()
}

#[doc(hidden)]
#[proc_macro]
#[doc = include_str!("../docs/static_ubig.md")]
pub fn static_ubig_embedded(input: TokenStream) -> TokenStream {
    parse::int::parse_integer(false, true, true, input.into()).into()
}

#[proc_macro]
#[doc = include_str!("../docs/ibig.md")]
pub fn ibig(input: TokenStream) -> TokenStream {
    parse::int::parse_integer(true, false, false, input.into()).into()
}

#[proc_macro]
#[doc = include_str!("../docs/static_ibig.md")]
pub fn static_ibig(input: TokenStream) -> TokenStream {
    parse::int::parse_integer(true, true, false, input.into()).into()
}

#[doc(hidden)]
#[proc_macro]
pub fn ibig_embedded(input: TokenStream) -> TokenStream {
    parse::int::parse_integer(true, false, true, input.into()).into()
}

#[doc(hidden)]
#[proc_macro]
#[doc = include_str!("../docs/static_ibig.md")]
pub fn static_ibig_embedded(input: TokenStream) -> TokenStream {
    parse::int::parse_integer(true, true, true, input.into()).into()
}

#[proc_macro]
#[doc = include_str!("../docs/fbig.md")]
pub fn fbig(input: TokenStream) -> TokenStream {
    parse::float::parse_binary_float(false, false, input.into()).into()
}

#[proc_macro]
#[doc = include_str!("../docs/static_fbig.md")]
pub fn static_fbig(input: TokenStream) -> TokenStream {
    parse::float::parse_binary_float(true, false, input.into()).into()
}

#[doc(hidden)]
#[proc_macro]
pub fn fbig_embedded(input: TokenStream) -> TokenStream {
    parse::float::parse_binary_float(false, true, input.into()).into()
}

#[doc(hidden)]
#[proc_macro]
pub fn static_fbig_embedded(input: TokenStream) -> TokenStream {
    parse::float::parse_binary_float(true, true, input.into()).into()
}

#[proc_macro]
#[doc = include_str!("../docs/dbig.md")]
pub fn dbig(input: TokenStream) -> TokenStream {
    parse::float::parse_decimal_float(false, false, input.into()).into()
}

#[proc_macro]
#[doc = include_str!("../docs/static_dbig.md")]
pub fn static_dbig(input: TokenStream) -> TokenStream {
    parse::float::parse_decimal_float(true, false, input.into()).into()
}

#[doc(hidden)]
#[proc_macro]
pub fn dbig_embedded(input: TokenStream) -> TokenStream {
    parse::float::parse_decimal_float(false, true, input.into()).into()
}

#[doc(hidden)]
#[proc_macro]
pub fn static_dbig_embedded(input: TokenStream) -> TokenStream {
    parse::float::parse_decimal_float(true, true, input.into()).into()
}

#[proc_macro]
#[doc = include_str!("../docs/rbig.md")]
pub fn rbig(input: TokenStream) -> TokenStream {
    parse::ratio::parse_ratio(false, input.into()).into()
}

#[doc(hidden)]
#[proc_macro]
pub fn rbig_embedded(input: TokenStream) -> TokenStream {
    parse::ratio::parse_ratio(true, input.into()).into()
}

// TODO(v0.5): add static_ubig!, static_ibig!, static_fbig!, static_dbig! (and their embedded versions)
//             rbig won't be supported because gcd cannot be done in const). These methods are designed
//             for big numbers, so the word array should be declared as static. Besides, they should wrap
//             the declaration to make sure the result number is immutable static.
