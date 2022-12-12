use crate::{
    rbig::{RBig, Relaxed},
    repr::Repr,
};
use core::str::FromStr;
use dashu_base::{ParseError, Sign};
use dashu_int::{IBig, UBig};

impl Repr {
    fn from_str_radix(src: &str, radix: u32) -> Result<Self, ParseError> {
        if let Some(slash) = src.find('/') {
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

    pub fn from_str_with_radix_prefix(mut src: &str) -> Result<(Self, u32), ParseError> {
        if let Some(slash) = src.find('/') {
            // first parse the numerator part
            let (num, num_radix) = IBig::from_str_with_radix_prefix(&src[..slash])?;
            src = &src[slash + 1..];

            // then strip the prefixes of the denominator
            // TODO(v0.4): use the updated IBig::from_str_with_radix_prefix so we don't have to duplicate the code
            let sign = match src.strip_prefix('-') {
                Some(s) => {
                    src = s;
                    Sign::Negative
                }
                None => {
                    src = src.strip_prefix('+').unwrap_or(src);
                    Sign::Positive
                }
            };

            let (den, den_radix) = if let Some(bin) = src.strip_prefix("0b") {
                (UBig::from_str_radix(bin, 2)?, 2)
            } else if let Some(oct) = src.strip_prefix("0o") {
                (UBig::from_str_radix(oct, 8)?, 8)
            } else if let Some(hex) = src.strip_prefix("0x") {
                (UBig::from_str_radix(hex, 16)?, 16)
            } else {
                (UBig::from_str_radix(src, num_radix)?, num_radix)
            };

            if num_radix != den_radix {
                // TODO(v0.4): add a separate error for this
                return Err(ParseError::UnsupportedRadix);
            }
            Ok((
                Repr {
                    numerator: num * sign,
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
    /// parsed integer and radix.
    ///
    /// `src` may contain an '+' or `-` prefix before the radix prefix.
    ///
    /// Allowed prefixes: `0b` for binary, `0o` for octal, `0x` for hexadecimal.
    ///
    /// If the radix prefixes for the numerator and the denominator are not the same,
    /// then a ParseError will be returned. The radix prefix for the denominator can be
    /// omitted, and the radix for the numerator will used in the conversion.
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
    pub fn from_str_with_radix_prefix(src: &str) -> Result<(Self, u32), ParseError> {
        Repr::from_str_with_radix_prefix(src).map(|(repr, radix)| (Self(repr.reduce()), radix))
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
    pub fn from_str_with_radix_prefix(src: &str) -> Result<(Self, u32), ParseError> {
        Repr::from_str_with_radix_prefix(src).map(|(repr, radix)| (Self(repr.reduce2()), radix))
    }
}

impl FromStr for Relaxed {
    type Err = ParseError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, ParseError> {
        Self::from_str_radix(s, 10)
    }
}
