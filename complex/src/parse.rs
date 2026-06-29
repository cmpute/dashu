//! [`FromStr`] for [`CBig`] — the algebraic `a+bi` grammar that [`Display`](crate::CBig) emits.
//!
//! Accepts an optional real term and an optional `"<sign><coeff>i"` imaginary term (at least one
//! required): `"5"`, `"-7i"`, `"i"`, `"-i"`, `"1+2i"`, `"-3-4i"`. Each coefficient parses via
//! [`FBig`]'s `FromStr` (so `inf`/`-inf` are accepted). The MPC parenthesized `"(re im)"` form and
//! anything else malformed yield a [`ParseError`].

use crate::cbig::CBig;
use core::str::FromStr;
use dashu_base::ParseError;
use dashu_float::round::Round;
use dashu_float::{FBig, Repr};
use dashu_int::Word;

impl<R: Round, const B: Word> FromStr for CBig<R, B> {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, ParseError> {
        let s = s.trim();
        if s.is_empty() {
            return Err(ParseError::NoDigits);
        }
        // Reject the MPC parenthesized form "(re im)" outright.
        if s.contains('(') || s.contains(')') {
            return Err(ParseError::InvalidDigit);
        }

        // The only valid 'i' is the trailing imaginary-unit marker.
        let i_count = s.bytes().filter(|&c| c == b'i').count();
        if i_count > 1 {
            return Err(ParseError::InvalidDigit);
        }

        if i_count == 0 {
            // pure real term
            let re = FBig::<R, B>::from_str(s)?;
            return Ok(CBig::from(re));
        }

        // exactly one 'i', and it must be the final character
        if !s.ends_with('i') {
            return Err(ParseError::InvalidDigit);
        }
        let prefix = &s[..s.len() - 1];

        // Split prefix into the real term and the imaginary coefficient. The imaginary coefficient
        // starts at the last '+' / '-' that is *not* at the leading position; if there is none, the
        // whole prefix (if any) is the imaginary coefficient and the real term is empty.
        let split = prefix.rfind(['+', '-']).filter(|&pos| pos > 0);
        let (real_str, imag_str) = match split {
            Some(pos) => (&prefix[..pos], &prefix[pos..]),
            None => ("", prefix),
        };

        let im = match imag_str {
            "" | "+" => FBig::<R, B>::ONE,
            "-" => FBig::<R, B>::NEG_ONE,
            other => FBig::<R, B>::from_str(other)?,
        };

        let re = if real_str.is_empty() {
            // implicit real zero carries the imaginary part's precision
            FBig::from_repr(Repr::zero(), im.context())
        } else {
            FBig::<R, B>::from_str(real_str)?
        };

        Ok(CBig::from_parts(re, im))
    }
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
        assert!(z.im().is_zero());
        assert_eq!(z.re().significand(), &5.into());
    }

    #[test]
    fn pure_imaginary_unit() {
        let z: C = "i".parse().unwrap();
        assert!(z.re().is_zero());
        assert_eq!(z.im().significand(), &1.into());

        let z: C = "-i".parse().unwrap();
        assert_eq!(z.im().significand(), &(-1i32).into());
    }

    #[test]
    fn malformed_rejected() {
        assert!("(1 2)".parse::<C>().is_err());
        assert!("".parse::<C>().is_err());
        assert!("1+2".parse::<C>().is_err()); // 'i' required for an imaginary term
        assert!("ii".parse::<C>().is_err());
        assert!("1+2ii".parse::<C>().is_err());
        assert!("i5".parse::<C>().is_err()); // 'i' must be trailing
    }
}
