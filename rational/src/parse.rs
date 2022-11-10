use crate::{rbig::{RBig, Relaxed}, repr::Repr};
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
                denominator: den
            })
        } else {
            let n = IBig::from_str_radix(src, radix)?;
            Ok(Repr {
                numerator: n,
                denominator: UBig::ONE
            })
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
        Repr::from_str_radix(src, radix).map(|v| RBig(v.reduce()))
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
        Repr::from_str_radix(src, radix).map(|v| Relaxed(v.reduce2()))
    }
}

impl FromStr for Relaxed {
    type Err = ParseError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, ParseError> {
        Self::from_str_radix(s, 10)
    }
}
