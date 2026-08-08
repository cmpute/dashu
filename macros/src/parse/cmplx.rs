//! Parser for the `cbig!` literal macro. Accepts the algebraic `a+bi` form (the same grammar as
//! the runtime `CBig::FromStr`) or a `re, im` pair, e.g. `cbig!(3+4i)`, `cbig!(3)`, `cbig!(3, 4)`.
//! Coefficients are **decimal by default** and may use the `0x` / `0b` / `0o` prefixes for other
//! bases, matching the `ubig!` / `ibig!` macros.

use super::common::quote_sign;
use super::float::{gen_binary_fbig_value, gen_binary_repr_const};
use core::str::FromStr;
use dashu_base::{BitTest, Sign};
use dashu_float::round::mode::Zero;
use dashu_float::{FBig, Repr};
use proc_macro2::TokenStream;
use quote::quote;

fn panic_cbig_syntax() -> ! {
    panic!("Incorrect syntax, please refer to the docs for acceptable complex literal formats.")
}

/// Extract `(sign, significand, exponent)` from a base-2 `FBig` coefficient when its significand
/// fits in a `u32`, so the coefficient can be reconstructed via the const-friendly
/// `CBig::from_parts_const`. Returns `None` for non-finite or too-large coefficients, in which
/// case the caller falls back to the runtime heap path (`CBig::from_parts`).
fn small_coeff(f: &FBig) -> Option<(Sign, u32, isize)> {
    if f.repr().is_infinite() {
        return None;
    }
    let (signif, exp) = f.clone().into_repr().into_parts();
    let (sign, mag) = signif.into_parts();
    if mag.bit_len() <= 32 {
        let u: u32 = mag.try_into().unwrap();
        Some((sign, u, exp))
    } else {
        None
    }
}

/// Parse a single coefficient in its detected base, returning a base-2 `FBig` (the base the
/// emitted `CBig` uses). `0x`/`0X` selects hexadecimal, `0b`/`0B` binary, `0o`/`0O` octal, and
/// unprefixed literals are decimal — consistent with `ubig!` / `ibig!`.
///
/// Hexadecimal and binary are parsed natively by the base-2 [`FBig`] `FromStr` (which also
/// supports the C++ `0x…p…` hex-float syntax); octal and decimal are parsed at their own base and
/// then converted to base 2, so integer coefficients convert exactly.
fn parse_coeff(s: &str) -> FBig {
    let s = s.trim();
    // split off the sign (kept so the parser applies it) and a leading `_` escape (see `fbig!`);
    // trim the rest so `"+ 4"` (spaces around the algebraic sign) still parses
    let (sign, rest) = match s.strip_prefix('-') {
        Some(r) => ("-", r.trim()),
        None => match s.strip_prefix('+') {
            Some(r) => ("+", r.trim()),
            None => ("", s),
        },
    };
    let digits = rest.strip_prefix('_').unwrap_or(rest);

    if digits.starts_with("0x") || digits.starts_with("0X") {
        FBig::from_str(&format!("{sign}{digits}")).unwrap_or_else(|_| panic_cbig_syntax())
    } else if let Some(body) = digits
        .strip_prefix("0b")
        .or_else(|| digits.strip_prefix("0B"))
    {
        FBig::from_str(&format!("{sign}{body}")).unwrap_or_else(|_| panic_cbig_syntax())
    } else if let Some(body) = digits
        .strip_prefix("0o")
        .or_else(|| digits.strip_prefix("0O"))
    {
        FBig::<Zero, 8>::from_str(&format!("{sign}{body}"))
            .unwrap_or_else(|_| panic_cbig_syntax())
            .with_base::<2>()
            .value()
    } else {
        FBig::<Zero, 10>::from_str(&format!("{sign}{digits}"))
            .unwrap_or_else(|_| panic_cbig_syntax())
            .with_base::<2>()
            .value()
    }
}

/// Parse the algebraic `a+bi` form into a `(re, im)` pair of base-2 `FBig` coefficients. Mirrors
/// the splitting done by the runtime `CBig::from_str` (`complex/src/parse.rs`).
fn parse_algebraic(s: &str) -> (FBig, FBig) {
    let s = s.trim();
    if !s.contains('i') {
        let re = parse_coeff(s);
        let im = FBig::from_repr(Repr::zero(), re.context());
        return (re, im);
    }
    // exactly one trailing `i`
    if s.bytes().filter(|&c| c == b'i').count() > 1 || !s.ends_with('i') {
        panic_cbig_syntax();
    }
    let prefix = &s[..s.len() - 1];
    let split = prefix.rfind(['+', '-']).filter(|&pos| pos > 0);
    let (real_str, imag_str) = match split {
        Some(pos) => (&prefix[..pos], &prefix[pos..]),
        None => ("", prefix),
    };
    let imag_str = imag_str.trim();

    let im = match imag_str {
        "" | "+" => FBig::ONE,
        "-" => FBig::NEG_ONE,
        other => parse_coeff(other),
    };
    let re = if real_str.is_empty() {
        FBig::from_repr(Repr::zero(), im.context())
    } else {
        parse_coeff(real_str)
    };
    (re, im)
}

pub fn parse_complex(static_: bool, embedded: bool, input: TokenStream) -> TokenStream {
    let value_str: String = input.into_iter().map(|tt| tt.to_string()).collect();
    let value_str = value_str.trim();

    // `re, im` pair (each is a plain real coefficient) vs the algebraic `a+bi` form.
    let (re, im) = if let Some((re_s, im_s)) = value_str.split_once(',') {
        (parse_coeff(re_s), parse_coeff(im_s))
    } else {
        parse_algebraic(value_str)
    };

    let ns = if embedded {
        quote!(::dashu::complex)
    } else {
        quote!(::dashu_cmplx)
    };
    let ns_f = if embedded {
        quote!(::dashu::float)
    } else {
        quote!(::dashu_float)
    };

    if static_ {
        // const construction: each Repr via from_static_words (or Repr::zero() for a zero coeff),
        // then new.
        let (re_repr, prec_re) = gen_binary_repr_const(embedded, &re);
        let (im_repr, prec_im) = gen_binary_repr_const(embedded, &im);
        let prec = prec_re.max(prec_im);
        quote! {{
            static VALUE: #ns::CBig = #ns::CBig::new(
                #re_repr,
                #im_repr,
                #ns::Context::new(#prec),
            );
            &VALUE
        }}
    } else {
        // When both coefficients fit in a DoubleWord, emit const-friendly construction
        // (CBig::from_parts_const, which builds inline Reprs without heap arithmetic) so that
        // `cbig!` works in const position for small literals — matching `fbig!`. Otherwise fall
        // back to the runtime heap path through CBig::from_parts.
        match (small_coeff(&re), small_coeff(&im)) {
            (Some((re_sign, re_u, re_exp)), Some((im_sign, im_u, im_exp))) => {
                let prec = re.precision().max(im.precision());
                let re_sign = quote_sign(embedded, re_sign);
                let im_sign = quote_sign(embedded, im_sign);
                quote! {
                    #ns::CBig::<#ns_f::round::mode::Zero, 2>::from_parts_const(
                        (#re_sign, #re_u as _, #re_exp),
                        (#im_sign, #im_u as _, #im_exp),
                        #prec,
                    )
                }
            }
            _ => {
                let re_tt = gen_binary_fbig_value(embedded, &re);
                let im_tt = gen_binary_fbig_value(embedded, &im);
                quote! { #ns::CBig::from_parts(#re_tt, #im_tt) }
            }
        }
    }
}
