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

    /// Parse a positional-expansion literal in the given `radix` (with optional exponent
    /// and repeating part) into an (unreduced) [Repr]. See [RBig::from_str_expanded] for
    /// the accepted syntax.
    fn from_str_expanded(src: &str, radix: u8) -> Result<Self, ParseError> {
        assert!((2..=36).contains(&radix), "radix must be between 2 and 36");

        // parse and remove the sign
        let (sign, src) = match src.strip_prefix('-') {
            Some(rest) => (Sign::Negative, rest),
            None => (Sign::Positive, src.strip_prefix('+').unwrap_or(src)),
        };

        // locate the scale marker and parse the trailing exponent. For radix 10 the
        // printer emits `e`/`E`; for every other radix it emits `@` exclusively — and
        // `e`/`E` are themselves valid digits once radix >= 15, so they must not be
        // treated as a marker there. `@` is never a digit, so it is always safe.
        let marker_pos = if radix == 10 {
            src.rfind(['e', 'E', '@'])
        } else {
            src.rfind('@')
        };
        let (scale, body) = match marker_pos {
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

        let base = UBig::from(radix);
        let int_val: UBig = if int_str.is_empty() {
            UBig::ZERO
        } else {
            UBig::from_str_radix(int_str, radix as u32)?
        };
        let fract_val: UBig = if fract_str.is_empty() {
            UBig::ZERO
        } else {
            UBig::from_str_radix(fract_str, radix as u32)?
        };

        // assemble the value as numerator/denominator (B = radix, p = #frac digits,
        // q = #repetend digits):
        //   terminating a.b    = (a*B^p + b) / B^p
        //   repeating  a.b(c)  = a + (b*(B^q-1) + c) / ((B^q-1) * B^p)
        let (num, den) = if let Some(rep_str) = repetend {
            let rep_val = UBig::from_str_radix(rep_str, radix as u32)?;
            let b_p = base.pow(fract_digits);
            let unit = base.pow(rep_digits) - UBig::ONE; // B^q - 1
            let fract_den = &unit * &b_p; // (B^q-1)*B^p
            let fract_num = fract_val * &unit + rep_val; // b*(B^q-1) + c
            let num = int_val * &fract_den + fract_num; // a*fract_den + fract_num
            (num, fract_den)
        } else {
            let den = base.pow(fract_digits);
            let num = int_val * &den + fract_val;
            (num, den)
        };

        // apply the exponent scale
        let abs_scale = scale.unsigned_abs();
        let scale_factor = base.pow(abs_scale);
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

    /// Convert a positional-expansion string in the given `radix` (2–36) to [RBig].
    ///
    /// Accepts fixed-point (`1.5`, `-.25`, `3.`), scientific, and repeating notation where
    /// the repeating block (repetend) is enclosed in parentheses after the radix point
    /// (`0.1(6)` = 1/6, `0.(3)` = 1/3, `1.1(6)` = 7/6). Underscore separators are allowed
    /// in any digit run. Digits above 9 are `a`–`z` or `A`–`Z` (case-insensitive).
    ///
    /// The scientific-notation marker follows what [`RBig::in_expanded`] emits: `e`/`E`
    /// (or `@`) for radix 10, and `@` only for every other radix — `e`/`E` are valid
    /// digits once `radix >= 15`, so they are never treated as a marker there. The
    /// exponent itself is a signed decimal integer.
    ///
    /// Unlike `FBig` (and the C hex-float syntax it accepts), the `p`/`P` binary-exponent
    /// marker is **not** recognized — `0x1.23p4` is not a valid literal here. `p`/`P` are
    /// simply digits (valid once `radix >= 26`); use `@` for a scaled expansion in any
    /// non-decimal base.
    ///
    /// At least one digit is required overall. This method is the inverse of
    /// [`RBig::in_expanded`]:
    /// - terminating expansions round-trip through `format!("{:.N}", x.in_expanded(radix))`
    ///   for any `N` at least as large as the number of fractional digits;
    /// - every rational, including repeating expansions, round-trips exactly through
    ///   `format!("{:#}", x.in_expanded(radix))` (use `{:#.N}` with a larger `N` when the
    ///   repetend is longer than the printer's default detection budget).
    ///
    /// For base-10 literals specifically, see the [`RBig::from_str_decimal`] alias. For
    /// `numerator/denominator` notation use [`RBig::from_str`] / [`RBig::from_str_radix`].
    ///
    /// # Panics
    ///
    /// Panics if `radix` is not in `2..=36`.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError::NoDigits`] if the input contains no digits, and
    /// [`ParseError::InvalidDigit`] for an invalid digit or exponent. An unclosed or empty
    /// repetend, or a repetend with no preceding radix point, returns
    /// [`ParseError::InvalidSyntax`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_base::ParseError;
    /// # use core::str::FromStr;
    /// # use dashu_ratio::RBig;
    /// assert_eq!(RBig::from_str_expanded("1.5", 10)?, RBig::from_str("3/2")?);
    /// // binary: 0.1 = 1/2, repeating 0.(01) = 1/3
    /// assert_eq!(RBig::from_str_expanded("0.1", 2)?, RBig::from_str("1/2")?);
    /// assert_eq!(RBig::from_str_expanded("0.(01)", 2)?, RBig::from_str("1/3")?);
    /// // hexadecimal: ff = 255, and 0.1@1 ( = 1/16 × 16) = 1
    /// assert_eq!(RBig::from_str_expanded("ff", 16)?, RBig::from_str("255")?);
    /// assert_eq!(RBig::from_str_expanded("0.1@1", 16)?, RBig::ONE);
    /// # Ok::<(), ParseError>(())
    /// ```
    #[inline]
    pub fn from_str_expanded(src: &str, radix: u8) -> Result<Self, ParseError> {
        Repr::from_str_expanded(src, radix).map(|repr| RBig(repr.reduce()))
    }

    /// Convert a base-10 decimal string to [RBig].
    ///
    /// This is a convenience alias for [`RBig::from_str_expanded`] with `radix = 10`;
    /// see that method for the full grammar and round-trip properties.
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
        Self::from_str_expanded(src, 10)
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

    /// Convert a positional-expansion string in the given `radix` (2–36) to [Relaxed].
    ///
    /// See [`RBig::from_str_expanded`] for the accepted syntax. The result is only reduced
    /// by powers of 2 (per [Relaxed]'s invariant), so the denominator may retain factors
    /// that share no power of 2 with the radix; e.g. `Relaxed::from_str_expanded("0.5", 10)`
    /// holds `5/10`, which compares equal to `1/2`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_base::ParseError;
    /// # use core::str::FromStr;
    /// # use dashu_ratio::Relaxed;
    /// let r = Relaxed::from_str_expanded("0.5", 10)?;
    /// assert_eq!(r, Relaxed::from_str("1/2")?);
    /// # Ok::<(), ParseError>(())
    /// ```
    #[inline]
    pub fn from_str_expanded(src: &str, radix: u8) -> Result<Self, ParseError> {
        Repr::from_str_expanded(src, radix).map(|repr| Relaxed(repr.reduce2()))
    }

    /// Convert a base-10 decimal string to [Relaxed].
    ///
    /// This is a convenience alias for [`Relaxed::from_str_expanded`] with `radix = 10`.
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
        Self::from_str_expanded(src, 10)
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

    #[test]
    fn test_from_str_expanded_basic() {
        // base 10 still works through the generalized entry point
        assert_eq!(RBig::from_str_expanded("1.5", 10).unwrap(), RBig::from_str("3/2").unwrap());
        // base 2
        assert_eq!(RBig::from_str_expanded("0.1", 2).unwrap(), RBig::from_str("1/2").unwrap());
        assert_eq!(RBig::from_str_expanded("0.01", 2).unwrap(), RBig::from_str("1/4").unwrap());
        assert_eq!(RBig::from_str_expanded("101", 2).unwrap(), RBig::from_str("5").unwrap());
        assert_eq!(RBig::from_str_expanded("0.101", 2).unwrap(), RBig::from_str("5/8").unwrap());
        // base 8
        assert_eq!(RBig::from_str_expanded("0.1", 8).unwrap(), RBig::from_str("1/8").unwrap());
        assert_eq!(RBig::from_str_expanded("10", 8).unwrap(), RBig::from_str("8").unwrap());
        // base 16 (case-insensitive digits)
        assert_eq!(RBig::from_str_expanded("ff", 16).unwrap(), RBig::from_str("255").unwrap());
        assert_eq!(RBig::from_str_expanded("FF", 16).unwrap(), RBig::from_str("255").unwrap());
        assert_eq!(RBig::from_str_expanded("0.8", 16).unwrap(), RBig::from_str("1/2").unwrap());
        assert_eq!(RBig::from_str_expanded("-a", 16).unwrap(), RBig::from_str("-10").unwrap());
        // base 36
        assert_eq!(RBig::from_str_expanded("z", 36).unwrap(), RBig::from_str("35").unwrap());
        // underscore separators
        assert_eq!(RBig::from_str_expanded("1_0", 2).unwrap(), RBig::from_str("2").unwrap());
    }

    #[test]
    fn test_from_str_expanded_repetend() {
        // base 2: 0.(01) = 1/3, 0.(1) = 1, 0.0(01) = 1/6
        assert_eq!(RBig::from_str_expanded("0.(01)", 2).unwrap(), RBig::from_str("1/3").unwrap());
        assert_eq!(RBig::from_str_expanded("0.(1)", 2).unwrap(), RBig::from_str("1").unwrap());
        assert_eq!(RBig::from_str_expanded("0.0(01)", 2).unwrap(), RBig::from_str("1/6").unwrap());
        // base 8: 0.(1) = 1/7, 0.(142857)_10 analog -> 0.(3) = 3/7
        assert_eq!(RBig::from_str_expanded("0.(1)", 8).unwrap(), RBig::from_str("1/7").unwrap());
        assert_eq!(RBig::from_str_expanded("0.(3)", 8).unwrap(), RBig::from_str("3/7").unwrap());
        // base 16: 0.(3) = 3/15 = 1/5
        assert_eq!(RBig::from_str_expanded("0.(3)", 16).unwrap(), RBig::from_str("1/5").unwrap());
        // mixed: base 2 0.1(01) = 1/2 + (1/3)/2 = 2/3
        assert_eq!(RBig::from_str_expanded("0.1(01)", 2).unwrap(), RBig::from_str("2/3").unwrap());
    }

    #[test]
    fn test_from_str_expanded_scientific() {
        // `@` is the marker for non-decimal radices
        assert_eq!(RBig::from_str_expanded("1.0@2", 2).unwrap(), RBig::from_str("4").unwrap());
        assert_eq!(RBig::from_str_expanded("1.0@-2", 2).unwrap(), RBig::from_str("1/4").unwrap());
        assert_eq!(RBig::from_str_expanded("1.0@2", 16).unwrap(), RBig::from_str("256").unwrap());
        assert_eq!(RBig::from_str_expanded("0.1@1", 16).unwrap(), RBig::ONE); // 1/16 × 16
                                                                              // `@` is also accepted for radix 10 (alias path)
        assert_eq!(RBig::from_str_expanded("1.5@2", 10).unwrap(), RBig::from_str("150").unwrap());
    }

    #[test]
    fn test_from_str_expanded_e_is_digit_above_14() {
        // For radix >= 15, `e`/`E` are valid digits and must NOT be read as an exponent
        // marker — only `@` is. 0.e (base 16) = 14/16 = 7/8.
        assert_eq!(RBig::from_str_expanded("0.e", 16).unwrap(), RBig::from_str("7/8").unwrap());
        assert_eq!(RBig::from_str_expanded("0.E", 16).unwrap(), RBig::from_str("7/8").unwrap());
        assert_eq!(RBig::from_str_expanded("e", 16).unwrap(), RBig::from_str("14").unwrap());
        assert_eq!(RBig::from_str_expanded("ee", 16).unwrap(), RBig::from_str("238").unwrap());
        // `1.0e2` in base 16 is a plain fixed-point literal (fract = "0e2" = 226), not
        // scientific: value = (1*16^3 + 226)/16^3 = 4322/4096 = 2161/2048.
        assert_eq!(
            RBig::from_str_expanded("1.0e2", 16).unwrap(),
            RBig::from_str("2161/2048").unwrap()
        );
    }

    #[test]
    fn test_from_str_expanded_roundtrip() {
        // terminating expansions round-trip through {:.N} in any radix
        let terminating: &[(u8, &str)] = &[
            (2, "1/2"),
            (2, "5/8"),
            (8, "1/8"),
            (8, "3/4"),
            (16, "1/4"),
            (16, "3/16"),
            (36, "1/36"),
        ];
        for &(radix, s) in terminating {
            let x = RBig::from_str(s).unwrap();
            let printed = format!("{:.12}", x.in_expanded(radix));
            assert_eq!(
                RBig::from_str_expanded(&printed, radix).unwrap(),
                x,
                "terminating round-trip failed for {s} base {radix} via '{printed}'"
            );
        }

        // repeating expansions round-trip exactly through {:#}
        let repeating: &[(u8, &str)] =
            &[(2, "1/3"), (2, "1/6"), (8, "1/7"), (16, "1/5"), (16, "1/3")];
        for &(radix, s) in repeating {
            let x = RBig::from_str(s).unwrap();
            let printed = format!("{:#}", x.in_expanded(radix));
            assert!(
                printed.contains('('),
                "expected a repetend for {s} base {radix}, got '{printed}'"
            );
            assert_eq!(
                RBig::from_str_expanded(&printed, radix).unwrap(),
                x,
                "repetend round-trip failed for {s} base {radix} via '{printed}'"
            );
        }

        // long-period fraction in base 16: raise the printer's detection budget with {:#.N}
        let x = RBig::from_str("1/17").unwrap();
        let printed = format!("{:#.128}", x.in_expanded(16));
        assert!(printed.contains('('), "expected a repetend for 1/17 base 16, got '{printed}'");
        assert_eq!(RBig::from_str_expanded(&printed, 16).unwrap(), x);
    }

    #[test]
    fn test_from_str_expanded_relaxed() {
        // Relaxed only strips powers of 2: 0.5 (base 10) holds 5/10 (the factor 5 is
        // retained), which compares equal to 1/2. (A power-of-2 base like 16 would fully
        // reduce, so it doesn't demonstrate this.)
        let r = Relaxed::from_str_expanded("0.5", 10).unwrap();
        assert_eq!(r, Relaxed::from_str("1/2").unwrap());
        assert_eq!(r, Relaxed::from_str("5/10").unwrap());
        // base-2 repeating
        assert_eq!(
            Relaxed::from_str_expanded("0.(01)", 2).unwrap(),
            Relaxed::from_str("1/3").unwrap()
        );
    }

    #[test]
    fn test_from_str_expanded_errors() {
        // NoDigits
        for s in ["", ".", "-", "0.@"] {
            assert!(
                matches!(RBig::from_str_expanded(s, 16), Err(ParseError::NoDigits)),
                "expected NoDigits for {s:?} base 16"
            );
        }
        // InvalidSyntax: unclosed / empty repetend, or repetend with no radix point
        for s in ["1.(6", "1.()", "(6)", "1(6)"] {
            assert!(
                matches!(RBig::from_str_expanded(s, 16), Err(ParseError::InvalidSyntax)),
                "expected InvalidSyntax for {s:?} base 16"
            );
        }
        // InvalidDigit: out-of-range digit for the radix, or a bad exponent
        for s in ["2", "0.2"] {
            assert!(
                matches!(RBig::from_str_expanded(s, 2), Err(ParseError::InvalidDigit)),
                "expected InvalidDigit for {s:?} base 2"
            );
        }
        assert!(matches!(RBig::from_str_expanded("0.g", 16), Err(ParseError::InvalidDigit)));
        assert!(matches!(
            RBig::from_str_expanded("1.0@", 16),
            Err(ParseError::NoDigits) // empty exponent after marker
        ));
    }

    #[test]
    #[should_panic(expected = "radix must be between 2 and 36")]
    fn test_from_str_expanded_radix_0_panics() {
        let _r = RBig::from_str_expanded("0", 0);
    }

    #[test]
    #[should_panic(expected = "radix must be between 2 and 36")]
    fn test_from_str_expanded_radix_37_panics() {
        let _r = RBig::from_str_expanded("0", 37);
    }
}
