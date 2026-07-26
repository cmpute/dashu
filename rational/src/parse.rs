use crate::{
    rbig::{RBig, Relaxed},
    repr::Repr,
};
use core::{num::IntErrorKind, str::FromStr};
use dashu_base::{ParseError, Sign};
use dashu_int::{IBig, UBig};

impl Repr {
    fn from_str_radix(src: &str, radix: u32) -> Result<Self, ParseError> {
        if let Some(slash) = src.find('/') {
            if src[slash + 1..].contains('/') {
                return Err(ParseError::InvalidSyntax);
            }
            let num = IBig::from_str_radix(&src[..slash], radix)?;
            let den = IBig::from_str_radix(&src[slash + 1..], radix)?;
            let (sign, den) = den.into_parts();
            Ok(Repr {
                numerator: num * sign,
                denominator: den,
            })
        } else {
            let n = IBig::from_str_radix(src, radix)?;
            Ok(Repr {
                numerator: n,
                denominator: UBig::ONE,
            })
        }
    }

    pub fn from_str_with_radix_prefix(src: &str) -> Result<(Self, u8), ParseError> {
        if let Some(slash) = src.find('/') {
            if src[slash + 1..].contains('/') {
                return Err(ParseError::InvalidSyntax);
            }
            // first parse the numerator part
            let (num, num_radix) = IBig::from_str_with_radix_prefix(&src[..slash])?;
            let (den, den_radix) = IBig::from_str_with_radix_default(&src[slash + 1..], num_radix)?;
            let (den_sign, den) = den.into_parts();

            if num_radix != den_radix {
                return Err(ParseError::InconsistentRadix);
            }
            Ok((
                Repr {
                    numerator: num * den_sign,
                    denominator: den,
                },
                num_radix,
            ))
        } else {
            let (n, radix) = IBig::from_str_with_radix_prefix(src)?;
            Ok((
                Repr {
                    numerator: n,
                    denominator: UBig::ONE,
                },
                radix,
            ))
        }
    }

    /// Parse a base-10 decimal literal (with optional exponent and repeating part) into
    /// an (unreduced) [Repr]. See [RBig::from_str_decimal] for the accepted syntax.
    fn from_str_decimal(src: &str) -> Result<Self, ParseError> {
        // parse and remove the sign
        let (sign, src) = match src.strip_prefix('-') {
            Some(rest) => (Sign::Negative, rest),
            None => (Sign::Positive, src.strip_prefix('+').unwrap_or(src)),
        };

        // locate the scale marker (last e/E/@) and parse the trailing exponent
        let (scale, body) = match src.rfind(['e', 'E', '@']) {
            Some(pos) => {
                let scale = match src[pos + 1..].parse::<isize>() {
                    Ok(v) => v,
                    Err(e) => match e.kind() {
                        IntErrorKind::Empty => return Err(ParseError::NoDigits),
                        _ => return Err(ParseError::InvalidDigit),
                    },
                };
                (scale, &src[..pos])
            }
            None => (0isize, src),
        };

        // split off the repeating block, if any: it must be `(digits)` at the very end
        let (main, repetend) = match body.find('(') {
            None => (body, None),
            Some(open) => {
                let close = match body.rfind(')') {
                    Some(c) if c + 1 == body.len() => c,
                    _ => return Err(ParseError::InvalidSyntax),
                };
                let rep_str = &body[open + 1..close];
                if rep_str.is_empty() {
                    return Err(ParseError::InvalidSyntax);
                }
                // the repeating block must follow a decimal point
                let main = &body[..open];
                if !main.contains('.') {
                    return Err(ParseError::InvalidSyntax);
                }
                (main, Some(rep_str))
            }
        };

        // split main into int[.fract] parts
        let (int_str, fract_str) = match main.find('.') {
            Some(dot) => (&main[..dot], &main[dot + 1..]),
            None => (main, ""),
        };

        // count written digits (underscores are separators, not digits) for the NoDigits guard
        let int_digits = int_str.len() - int_str.matches('_').count();
        let fract_digits = fract_str.len() - fract_str.matches('_').count();
        let rep_digits = repetend.map_or(0, |s| s.len() - s.matches('_').count());
        if int_digits + fract_digits + rep_digits == 0 {
            return Err(ParseError::NoDigits);
        }

        let int_val: UBig = if int_str.is_empty() {
            UBig::ZERO
        } else {
            UBig::from_str_radix(int_str, 10)?
        };
        let fract_val: UBig = if fract_str.is_empty() {
            UBig::ZERO
        } else {
            UBig::from_str_radix(fract_str, 10)?
        };

        // assemble the value as numerator/denominator.
        // terminating:   a.b = (a*10^p + b) / 10^p
        // repeating a.b(c) = a + (b*(10^q-1) + c) / ((10^q-1) * 10^p)
        let (num, den) = if let Some(rep_str) = repetend {
            let rep_val = UBig::from_str_radix(rep_str, 10)?;
            let ten_p = UBig::from(10u8).pow(fract_digits);
            let unit = UBig::from(10u8).pow(rep_digits) - UBig::ONE; // 10^q - 1
            let fract_den = &unit * &ten_p; // (10^q-1)*10^p
            let fract_num = fract_val * &unit + rep_val; // b*(10^q-1) + c
            let num = int_val * &fract_den + fract_num; // a*fract_den + fract_num
            (num, fract_den)
        } else {
            let den = UBig::from(10u8).pow(fract_digits);
            let num = int_val * &den + fract_val;
            (num, den)
        };

        // apply the exponent scale
        let abs_scale = scale.unsigned_abs();
        let scale_factor = UBig::from(10u8).pow(abs_scale);
        let (num, den) = if scale >= 0 {
            (num * scale_factor, den)
        } else {
            (num, den * scale_factor)
        };

        Ok(Repr {
            numerator: sign * num,
            denominator: den,
        })
    }
}

impl RBig {
    /// Convert a string in a given base to [RBig].
    ///
    /// The numerator and the denominator are separated by `/`.
    /// `src` may contain an optional `+` prefix.
    /// Digits 10-35 are represented by `a-z` or `A-Z`.
    ///
    /// # Examples
    /// ```
    /// # use dashu_base::ParseError;
    /// # use dashu_ratio::RBig;
    /// assert_eq!(
    ///     RBig::from_str_radix("+7ab/-sse", 32)?,
    ///     RBig::from_parts((-7499).into(), 29582u16.into())
    /// );
    /// # Ok::<(), ParseError>(())
    /// ```
    #[inline]
    pub fn from_str_radix(src: &str, radix: u32) -> Result<Self, ParseError> {
        Repr::from_str_radix(src, radix).map(|repr| RBig(repr.reduce()))
    }

    /// Convert a string with optional radix prefixes to [RBig], return the
    /// parsed integer and radix. If no prefix is present, then the default radix 10
    /// will be used for parsing.
    ///
    /// `src` may contain an '+' or `-` prefix before the radix prefix of both the
    /// numerator and denominator.
    ///
    /// Allowed prefixes: `0b` for binary, `0o` for octal, `0x` for hexadecimal.
    ///
    /// If the radix prefixes for the numerator and the denominator are not the same,
    /// then a ParseError will be returned. The radix prefix for the denominator can be
    /// omitted, and the radix for the numerator will used for parsing.
    ///
    /// # Examples
    /// ```
    /// # use dashu_base::ParseError;
    /// # use dashu_ratio::RBig;
    /// assert_eq!(RBig::from_str_with_radix_prefix("+0o17/25")?,
    ///     (RBig::from_parts(0o17.into(), 0o25u8.into()), 8));
    /// assert_eq!(RBig::from_str_with_radix_prefix("-0x1f/-0x1e")?,
    ///     (RBig::from_parts(0x1f.into(), 0x1eu8.into()), 16));
    /// # Ok::<(), ParseError>(())
    /// ```
    #[inline]
    pub fn from_str_with_radix_prefix(src: &str) -> Result<(Self, u8), ParseError> {
        Repr::from_str_with_radix_prefix(src).map(|(repr, radix)| (Self(repr.reduce()), radix))
    }

    /// Convert a base-10 decimal string to [RBig].
    ///
    /// Accepts fixed-point (`1.5`, `-.25`, `3.`), scientific (`1.234e2`, `1.5E-3`,
    /// `1.5@2`), and repeating-decimal notation where the repeating block is enclosed
    /// in parentheses after the decimal point (`0.1(6)` = 1/6, `0.(3)` = 1/3, `1.1(6)` =
    /// 7/6). Underscore separators are allowed in any digit run.
    ///
    /// At least one digit is required overall. The accepted forms are a superset of what
    /// [`RBig::in_expanded`] emits with radix 10, so this method is the inverse of the
    /// expanded/decimal printer:
    /// - terminating decimals round-trip through `format!("{:.N}", x.in_expanded(10))`
    ///   for any `N` at least as large as the number of fractional digits;
    /// - every rational, including repeating decimals, round-trips exactly through
    ///   `format!("{:#}", x.in_expanded(10))` (use `{:#.N}` with a larger `N` when the
    ///   repetend is longer than the printer's default detection budget).
    ///
    /// This method only parses base-10 decimals. For `numerator/denominator` notation
    /// use [`RBig::from_str`] / [`RBig::from_str_radix`]; for other bases use
    /// [`RBig::from_str_with_radix_prefix`].
    ///
    /// # Errors
    ///
    /// Returns [`ParseError::NoDigits`] if the input contains no digits, and
    /// [`ParseError::InvalidDigit`] for malformed input (an invalid digit, an unclosed
    /// or empty repeating group, a repeating group without a preceding `.`, or an
    /// invalid exponent).
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_base::ParseError;
    /// # use core::str::FromStr;
    /// # use dashu_ratio::RBig;
    /// assert_eq!(RBig::from_str_decimal("1.5")?, RBig::from_str("3/2")?);
    /// assert_eq!(RBig::from_str_decimal("1.234e2")?, RBig::from_str("617/5")?); // = 123.4
    /// assert_eq!(RBig::from_str_decimal("0.1(6)")?, RBig::from_str("1/6")?);   // repeating
    /// # Ok::<(), ParseError>(())
    /// ```
    #[inline]
    pub fn from_str_decimal(src: &str) -> Result<Self, ParseError> {
        Repr::from_str_decimal(src).map(|repr| RBig(repr.reduce()))
    }
}

impl FromStr for RBig {
    type Err = ParseError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, ParseError> {
        Self::from_str_radix(s, 10)
    }
}

impl Relaxed {
    /// Convert a string in a given base to [Relaxed].
    ///
    /// See [RBig::from_str_radix] for details.
    #[inline]
    pub fn from_str_radix(src: &str, radix: u32) -> Result<Self, ParseError> {
        Repr::from_str_radix(src, radix).map(|repr| Relaxed(repr.reduce2()))
    }

    /// Convert a string with optional radix prefixes to [RBig], return the
    /// parsed integer and radix.
    ///
    /// See [RBig::from_str_with_radix_prefix] for details.
    #[inline]
    pub fn from_str_with_radix_prefix(src: &str) -> Result<(Self, u8), ParseError> {
        Repr::from_str_with_radix_prefix(src).map(|(repr, radix)| (Self(repr.reduce2()), radix))
    }

    /// Convert a base-10 decimal string to [Relaxed].
    ///
    /// See [`RBig::from_str_decimal`] for the accepted syntax and round-trip properties.
    /// The result is only reduced by powers of 2 (per [Relaxed]'s invariant), so the
    /// denominator may retain factors of 5; e.g. `Relaxed::from_str_decimal("0.5")`
    /// holds `5/10`, which compares equal to `1/2`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_base::ParseError;
    /// # use core::str::FromStr;
    /// # use dashu_ratio::Relaxed;
    /// let r = Relaxed::from_str_decimal("0.5")?;
    /// assert_eq!(r, Relaxed::from_str("1/2")?);
    /// # Ok::<(), ParseError>(())
    /// ```
    #[inline]
    pub fn from_str_decimal(src: &str) -> Result<Self, ParseError> {
        Repr::from_str_decimal(src).map(|repr| Relaxed(repr.reduce2()))
    }
}

impl FromStr for Relaxed {
    type Err = ParseError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, ParseError> {
        Self::from_str_radix(s, 10)
    }
}

#[cfg(test)]
mod tests {
    use crate::{RBig, Relaxed};
    use alloc::format;
    use core::str::FromStr;
    use dashu_base::ParseError;

    #[test]
    fn test_from_str_decimal_basic() {
        assert_eq!(RBig::from_str_decimal("0").unwrap(), RBig::ZERO);
        assert_eq!(RBig::from_str_decimal("42").unwrap(), RBig::from_str("42").unwrap());
        assert_eq!(RBig::from_str_decimal("-42").unwrap(), RBig::from_str("-42").unwrap());
        assert_eq!(RBig::from_str_decimal("+42").unwrap(), RBig::from_str("42").unwrap());
        assert_eq!(RBig::from_str_decimal("1.5").unwrap(), RBig::from_str("3/2").unwrap());
        assert_eq!(RBig::from_str_decimal("-0.125").unwrap(), RBig::from_str("-1/8").unwrap());
        assert_eq!(RBig::from_str_decimal(".25").unwrap(), RBig::from_str("1/4").unwrap());
        assert_eq!(RBig::from_str_decimal("3.").unwrap(), RBig::from_str("3").unwrap());
        assert_eq!(RBig::from_str_decimal("1_000.5").unwrap(), RBig::from_str("2001/2").unwrap());
    }

    #[test]
    fn test_from_str_decimal_scientific() {
        assert_eq!(RBig::from_str_decimal("1.234e2").unwrap(), RBig::from_str("617/5").unwrap());
        assert_eq!(
            RBig::from_str_decimal("3.3333e-1").unwrap(),
            RBig::from_str("33333/100000").unwrap()
        );
        assert_eq!(RBig::from_str_decimal("1.234E2").unwrap(), RBig::from_str("617/5").unwrap());
        assert_eq!(RBig::from_str_decimal("1.234@2").unwrap(), RBig::from_str("617/5").unwrap());
        assert_eq!(RBig::from_str_decimal("1.5@-1").unwrap(), RBig::from_str("3/20").unwrap());
        assert_eq!(
            RBig::from_str_decimal("-12_34.56_78e9").unwrap(),
            RBig::from_str("-1234567800000").unwrap()
        );
    }

    #[test]
    fn test_from_str_decimal_repetend() {
        assert_eq!(RBig::from_str_decimal("0.(3)").unwrap(), RBig::from_str("1/3").unwrap());
        assert_eq!(RBig::from_str_decimal("0.1(6)").unwrap(), RBig::from_str("1/6").unwrap());
        assert_eq!(RBig::from_str_decimal("1.1(6)").unwrap(), RBig::from_str("7/6").unwrap());
        assert_eq!(RBig::from_str_decimal("0.(142857)").unwrap(), RBig::from_str("1/7").unwrap());
        assert_eq!(RBig::from_str_decimal("-3.(6)").unwrap(), RBig::from_str("-11/3").unwrap());
        assert_eq!(RBig::from_str_decimal("3.(6)").unwrap(), RBig::from_str("11/3").unwrap());
        // exponent applied to a repeating literal
        assert_eq!(RBig::from_str_decimal("1.1(6)e1").unwrap(), RBig::from_str("35/3").unwrap());
    }

    #[test]
    fn test_from_str_decimal_roundtrip_plain() {
        // terminating decimals round-trip through {:.N} for large enough N
        let cases = [
            "0",
            "1",
            "1/2",
            "1/4",
            "1/5",
            "1/8",
            "3/8",
            "7/8",
            "1/40",
            "1/250",
            "314159265358979323/1000000000000000000",
        ];
        for s in cases {
            let x = RBig::from_str(s).unwrap();
            let printed = format!("{:.20}", x.in_expanded(10));
            assert_eq!(
                RBig::from_str_decimal(&printed).unwrap(),
                x,
                "round-trip failed for {s} via '{printed}'"
            );
        }
    }

    #[test]
    fn test_from_str_decimal_roundtrip_scientific() {
        let x = RBig::from_str("1/8").unwrap();
        assert_eq!(RBig::from_str_decimal(&format!("{:.6e}", x.in_expanded(10))).unwrap(), x);
        assert_eq!(RBig::from_str_decimal(&format!("{:.6E}", x.in_expanded(10))).unwrap(), x);
    }

    #[test]
    fn test_from_str_decimal_roundtrip_repetend() {
        // short periods: default {:#} detects them and round-trips exactly
        let short = ["1/3", "1/6", "1/7", "7/6", "22/7"];
        for s in short {
            let x = RBig::from_str(s).unwrap();
            let printed = format!("{:#}", x.in_expanded(10));
            assert!(printed.contains('('), "expected a repetend for {s}, got '{printed}'");
            assert_eq!(
                RBig::from_str_decimal(&printed).unwrap(),
                x,
                "repetend round-trip failed for {s} via '{printed}'"
            );
        }
        // a long-period fraction: raise the printer's detection budget with {:#.N}
        let x = RBig::from_str("1/149").unwrap();
        let printed = format!("{:#.256}", x.in_expanded(10));
        assert!(printed.contains('('), "expected a repetend for 1/149, got '{printed}'");
        assert_eq!(RBig::from_str_decimal(&printed).unwrap(), x);
    }

    #[test]
    fn test_from_str_decimal_relaxed() {
        // Relaxed only strips powers of 2, so 0.5 stays 5/10 — compare by value
        let r = Relaxed::from_str_decimal("0.5").unwrap();
        assert_eq!(r, Relaxed::from_str("1/2").unwrap());
        assert_eq!(r, Relaxed::from_str("5/10").unwrap());
        // repeating decimal
        assert_eq!(Relaxed::from_str_decimal("0.1(6)").unwrap(), Relaxed::from_str("1/6").unwrap());
    }

    #[test]
    fn test_from_str_decimal_errors() {
        let no_digits = ["", ".", ".e", "-", "-.", "e", "+", "1e"];
        let invalid_syntax = [
            "(6)",  // no decimal point before the repeating group
            "1(6)", // no decimal point before the repeating group
            "1.(6", // unclosed repeating group
            "1.()", // empty repeating group
        ];
        let invalid_digit = [
            "abc",    // non-decimal character
            "0x1",    // radix prefix is not valid in base 10
            "1.2.3",  // second decimal point
            "1e2(6)", // malformed exponent
            "1ee2",   // bad exponent
        ];
        for s in no_digits {
            assert!(
                matches!(RBig::from_str_decimal(s), Err(ParseError::NoDigits)),
                "expected NoDigits for {s:?}"
            );
        }
        for s in invalid_syntax {
            assert!(
                matches!(RBig::from_str_decimal(s), Err(ParseError::InvalidSyntax)),
                "expected InvalidSyntax for {s:?}"
            );
        }
        for s in invalid_digit {
            assert!(
                matches!(RBig::from_str_decimal(s), Err(ParseError::InvalidDigit)),
                "expected InvalidDigit for {s:?}"
            );
        }
    }

    #[test]
    fn test_from_str_radix_multiple_separators() {
        // multiple `/` separators are structurally malformed
        assert!(matches!(RBig::from_str_radix("1/2/3", 10), Err(ParseError::InvalidSyntax)));
        assert!(matches!(RBig::from_str("1/2/3"), Err(ParseError::InvalidSyntax)));
        assert!(matches!(
            RBig::from_str_with_radix_prefix("0x1/2/3"),
            Err(ParseError::InvalidSyntax)
        ));
    }
}
