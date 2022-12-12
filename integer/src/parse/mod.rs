//! Integer parsing helpers.

use crate::{
    ibig::IBig,
    radix::{self, is_radix_valid, Digit},
    ubig::UBig,
    Sign::*,
};
use core::str::FromStr;
use dashu_base::ParseError;

mod non_power_two;
mod power_two;

impl FromStr for UBig {
    type Err = ParseError;
    #[inline]
    fn from_str(s: &str) -> Result<UBig, ParseError> {
        UBig::from_str_radix(s, 10)
    }
}

impl FromStr for IBig {
    type Err = ParseError;
    #[inline]
    fn from_str(s: &str) -> Result<IBig, ParseError> {
        IBig::from_str_radix(s, 10)
    }
}

impl UBig {
    /// Convert a string in a given base to [UBig].
    ///
    /// `src` may contain an optional `+` prefix.
    /// Digits 10-35 are represented by `a-z` or `A-Z`.
    ///
    /// # Examples
    /// ```
    /// # use dashu_base::ParseError;
    /// # use dashu_int::UBig;
    /// assert_eq!(UBig::from_str_radix("+7ab", 32)?, UBig::from(7499u16));
    /// # Ok::<(), ParseError>(())
    /// ```
    pub fn from_str_radix(src: &str, radix: u32) -> Result<UBig, ParseError> {
        if !is_radix_valid(radix) {
            return Err(ParseError::UnsupportedRadix);
        }
        let src = src.strip_prefix('+').unwrap_or(src);
        UBig::from_str_radix_no_sign(src, radix)
    }

    /// Convert a string with an optional radix prefix to [UBig], returns the
    /// parsed integer and radix.
    ///
    /// `src` may contain an optional `+` before the radix prefix.
    ///
    /// Allowed prefixes: `0b` for binary, `0o` for octal, `0x` for hexadecimal.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_base::ParseError;
    /// # use dashu_int::UBig;
    /// assert_eq!(UBig::from_str_with_radix_prefix("+0o17")?, (UBig::from(0o17u8), 8));
    /// assert_eq!(UBig::from_str_with_radix_prefix("0x1f")?.0, UBig::from(0x1fu8));
    /// # Ok::<(), ParseError>(())
    /// ```
    #[inline]
    pub fn from_str_with_radix_prefix(src: &str) -> Result<(UBig, Digit), ParseError> {
        // TODO(v0.4): add an argment to specify the default radix instead of always using 10
        let src = src.strip_prefix('+').unwrap_or(src);
        UBig::from_str_with_radix_prefix_no_sign(src)
    }

    /// Convert an unsigned string with an optional radix prefix to [UBig].
    fn from_str_with_radix_prefix_no_sign(src: &str) -> Result<(UBig, Digit), ParseError> {
        if let Some(bin) = src.strip_prefix("0b") {
            UBig::from_str_radix_no_sign(bin, 2).map(|v| (v, 2))
        } else if let Some(oct) = src.strip_prefix("0o") {
            UBig::from_str_radix_no_sign(oct, 8).map(|v| (v, 8))
        } else if let Some(hex) = src.strip_prefix("0x") {
            UBig::from_str_radix_no_sign(hex, 16).map(|v| (v, 16))
        } else {
            UBig::from_str_radix_no_sign(src, 10).map(|v| (v, 10))
        }
    }

    /// Convert an unsigned string to [UBig].
    fn from_str_radix_no_sign(mut src: &str, radix: Digit) -> Result<UBig, ParseError> {
        debug_assert!(radix::is_radix_valid(radix));
        if src.is_empty() {
            return Err(ParseError::NoDigits);
        }

        while let Some(src2) = src.strip_prefix('0') {
            src = src2;
        }

        if radix.is_power_of_two() {
            power_two::parse(src, radix)
        } else {
            non_power_two::parse(src, radix)
        }
    }
}

impl IBig {
    /// Convert a string in a given base to [IBig].
    ///
    /// The string may contain a `+` or `-` prefix.
    /// Digits 10-35 are represented by `a-z` or `A-Z`.
    ///
    /// # Examples
    /// ```
    /// # use dashu_base::ParseError;
    /// # use dashu_int::IBig;
    /// assert_eq!(IBig::from_str_radix("-7ab", 32)?, IBig::from(-7499));
    /// # Ok::<(), ParseError>(())
    /// ```
    pub fn from_str_radix(mut src: &str, radix: u32) -> Result<IBig, ParseError> {
        if !is_radix_valid(radix) {
            return Err(ParseError::UnsupportedRadix);
        }

        let sign = match src.strip_prefix('-') {
            Some(s) => {
                src = s;
                Negative
            }
            None => {
                src = src.strip_prefix('+').unwrap_or(src);
                Positive
            }
        };
        let mag = UBig::from_str_radix_no_sign(src, radix)?;
        Ok(IBig(mag.0.with_sign(sign)))
    }

    /// Convert a string with an optional radix prefix to [IBig], return the
    /// parsed integer and radix.
    ///
    /// `src` may contain an '+' or `-` prefix before the radix prefix.
    ///
    /// Allowed prefixes: `0b` for binary, `0o` for octal, `0x` for hexadecimal.
    ///
    /// # Examples
    /// ```
    /// # use dashu_base::ParseError;
    /// # use dashu_int::IBig;
    /// assert_eq!(IBig::from_str_with_radix_prefix("+0o17")?, (IBig::from(0o17), 8));
    /// assert_eq!(IBig::from_str_with_radix_prefix("-0x1f")?.0, IBig::from(-0x1f));
    /// # Ok::<(), ParseError>(())
    /// ```
    pub fn from_str_with_radix_prefix(mut src: &str) -> Result<(IBig, Digit), ParseError> {
        let sign = match src.strip_prefix('-') {
            Some(s) => {
                src = s;
                Negative
            }
            None => {
                src = src.strip_prefix('+').unwrap_or(src);
                Positive
            }
        };
        let (mag, radix) = UBig::from_str_with_radix_prefix_no_sign(src)?;
        Ok((IBig(mag.0.with_sign(sign)), radix))
    }
}
