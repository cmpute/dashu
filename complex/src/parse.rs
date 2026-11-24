//! [`FromStr`] for [`CBig`] — the algebraic `a+bi` grammar that [`Display`](crate::CBig) emits.
//!
//! Accepts an optional real term and an optional `"<sign><coeff>i"` imaginary term (at least one
//! required): `"5"`, `"-7i"`, `"i"`, `"-i"`, `"1+2i"`, `"-3-4i"`, `"1+2e-3i"`. Each coefficient
//! parses via [`FBig`]'s `FromStr` (so signed exponents are fine), and additionally `inf` /
//! `-inf` are accepted as coefficients, as `Display` writes them. The MPC parenthesized
//! `"(re im)"` form and anything else malformed yield a [`ParseError`].
//!
//! In bases above 18 the letter `i` is a digit, so there the imaginary term is marked by an
//! explicit `*i` suffix instead (`"1+2i*i"`, `"1*i"`), exactly as `Display` writes it; a string
//! without that suffix is a real number.

use crate::cbig::CBig;
use crate::fmt::i_is_digit;
use core::str::FromStr;
use dashu_base::ParseError;
use dashu_float::round::Round;
use dashu_float::{FBig, Repr};
use dashu_int::Word;

/// Parse one coefficient: [`FBig`]'s grammar, plus the `inf` / `-inf` that `Display` writes for
/// an infinite part. (In bases above 23 the letters of `inf` are all digits, so there the string
/// is a number, as it is for `FBig`.)
fn parse_coefficient<R: Round, const B: Word>(s: &str) -> Result<FBig<R, B>, ParseError> {
    if B <= 23 {
        match s {
            "inf" | "+inf" => return Ok(FBig::INFINITY),
            "-inf" => return Ok(FBig::NEG_INFINITY),
            _ => {}
        }
    }
    FBig::from_str(s)
}

/// Parse the coefficient of the imaginary term (the text before the imaginary-unit marker): a
/// bare sign (or nothing) is the unit coefficient `±1`.
fn parse_imaginary<R: Round, const B: Word>(s: &str) -> Result<FBig<R, B>, ParseError> {
    match s {
        "" | "+" => Ok(FBig::ONE),
        "-" => Ok(FBig::NEG_ONE),
        _ => {
            // exactly one sign may lead the coefficient
            let mut bytes = s.bytes();
            if let (Some(b'+' | b'-'), Some(b'+' | b'-')) = (bytes.next(), bytes.next()) {
                return Err(ParseError::InvalidSyntax);
            }
            parse_coefficient(s)
        }
    }
}

impl<R: Round, const B: Word> FromStr for CBig<R, B> {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, ParseError> {
        let s = s.trim();
        if s.is_empty() {
            return Err(ParseError::NoDigits);
        }
        // Reject the MPC parenthesized form "(re im)" outright.
        if s.contains('(') || s.contains(')') {
            return Err(ParseError::InvalidSyntax);
        }

        let marker = if i_is_digit::<B>() { "*i" } else { "i" };
        let prefix = match s.strip_suffix(marker) {
            Some(prefix) => prefix,
            None => {
                // pure real term
                if has_stray_i::<B>(s) {
                    return Err(ParseError::InvalidSyntax); // 'i' not trailing
                }
                return Ok(CBig::from(parse_coefficient::<R, B>(s)?));
            }
        };
        // The only valid 'i' is the trailing imaginary-unit marker (besides the one in `inf`).
        if has_stray_i::<B>(prefix) {
            return Err(ParseError::InvalidSyntax);
        }

        // Split the prefix into the real term and the imaginary coefficient at a '+' / '-' that
        // is not at the leading position. A sign can also belong to an exponent (`1+2e-3i`,
        // `1e-5i`), so take the rightmost split at which both sides are well-formed numbers; an
        // exponent's sign is never such a split, as the text before it ends in the scale marker.
        for (pos, _) in prefix.rmatch_indices(['+', '-']) {
            if pos == 0 {
                continue;
            }
            let re = parse_coefficient::<R, B>(&prefix[..pos]);
            let im = parse_imaginary::<R, B>(&prefix[pos..]);
            if let (Ok(re), Ok(im)) = (re, im) {
                return Ok(CBig::from_parts(re, im));
            }
        }

        // No split: the whole prefix is the imaginary coefficient, and the implicit real zero
        // carries the imaginary part's precision.
        let im = parse_imaginary::<R, B>(prefix)?;
        let re = FBig::from_repr(Repr::zero(), im.context());
        Ok(CBig::from_parts(re, im))
    }
}

/// Whether `s` contains an `i` that can be neither a digit nor part of an `inf` coefficient.
fn has_stray_i<const B: Word>(s: &str) -> bool {
    !i_is_digit::<B>() && s.split("inf").any(|rest| rest.contains('i'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::format;
    use dashu_float::round::mode;

    type C = CBig<mode::HalfAway, 10>;

    fn parse_ok(s: &str) -> C {
        s.parse()
            .unwrap_or_else(|e| panic!("failed to parse {s:?}: {e:?}"))
    }

    #[test]
    fn roundtrip_display_fromstr() {
        let cases = [
            "0", "5", "-7i", "i", "-i", "1+2i", "-3-4i", "1+i", "2-i", "4i",
        ];
        for s in cases {
            let z: C = parse_ok(s);
            assert_eq!(format!("{}", z), s, "roundtrip failed for {s:?}");
        }
    }

    #[test]
    fn pure_real() {
        let z: C = "5".parse().unwrap();
        assert!(z.im().is_pos_zero());
        assert_eq!(z.re().significand(), &5.into());
    }

    #[test]
    fn pure_imaginary_unit() {
        let z: C = "i".parse().unwrap();
        assert!(z.re().is_pos_zero());
        assert_eq!(z.im().significand(), &1.into());

        let z: C = "-i".parse().unwrap();
        assert_eq!(z.im().significand(), &(-1i32).into());
    }

    #[test]
    fn malformed_rejected() {
        use dashu_base::ParseError;
        // structurally malformed inputs return InvalidSyntax
        assert_eq!("(1 2)".parse::<C>().unwrap_err(), ParseError::InvalidSyntax); // MPC form
        assert_eq!("ii".parse::<C>().unwrap_err(), ParseError::InvalidSyntax); // multiple 'i'
        assert_eq!("1+2ii".parse::<C>().unwrap_err(), ParseError::InvalidSyntax); // multiple 'i'
        assert_eq!("i5".parse::<C>().unwrap_err(), ParseError::InvalidSyntax); // 'i' not trailing
                                                                               // other malformations still yield a ParseError
        assert_eq!("".parse::<C>().unwrap_err(), ParseError::NoDigits);
        assert!("1+2".parse::<C>().is_err()); // 'i' required for an imaginary term
    }
}
