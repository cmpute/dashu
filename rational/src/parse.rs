
use dashu_int::{IBig, UBig, error::ParseError};
use core::str::FromStr;
use crate::rbig::{RBig, Relaxed};

macro_rules! impl_from_str {
    ($t:ty) => {
        impl $t {
            pub fn from_str_radix(src: &str, radix: u32) -> Result<Self, ParseError> {
                if let Some(slash) = src.find('/') {
                    let num = IBig::from_str_radix(&src[..slash], radix)?;
                    let den = IBig::from_str_radix(&src[slash + 1..], radix)?;
                    let (sign, den) = den.into_parts();
                    Ok(Self::from_parts(num * sign, den))
                } else {
                    let n = IBig::from_str_radix(src, radix)?;
                    Ok(Self::from_parts(n, UBig::ONE))
                }
            }
        }

        impl FromStr for $t {
            type Err = ParseError;

            #[inline]
            fn from_str(s: &str) -> Result<Self, ParseError> {
                Self::from_str_radix(s, 10)
            }
        }
    };
}
impl_from_str!(RBig);
impl_from_str!(Relaxed);
