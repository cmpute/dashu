use crate::{
    rbig::{RBig, Relaxed},
    repr::Repr,
};
use core::str::FromStr;
use dashu_base::ParseError;
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

    pub fn from_str_with_radix_prefix(src: &str) -> Result<(Self, u32), ParseError> {
        if let Some(slash) = src.find('/') {
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
