use core::str::FromStr;
use core::num::IntErrorKind;
use dashu_int::{IBig, error::ParseError};
use crate::{repr::FloatRepr, utils::get_precision};

impl<const X: usize, const R: u8> FromStr for FloatRepr<X, R> {
    type Err = ParseError;

    /// Convert a string in a given base to [FloatRepr].
    ///
    /// `src` may contain an optional `+` prefix.
    /// Digits 10-35 are represented by `a-z` or `A-Z`.
    /// 
    /// The valid representations include
    /// 1. `xxx.yyy` = `xxxyyy / radix ^ len(yyy)` (in this case 3)
    /// 2. `xxx.yyyEzz` = `xxxyyy / radix ^ len(yyy) * 10 ^ zz`
    ///   > `E` could be lower case, Radix must be 10, `zz` is represented in decimal
    /// 3. `xxx.yyyPzz` = `xxxyyy / radix ^ len(yyy) * 2 ^ zz`
    ///   > `P` could be lower case, Radix must be 16, `zz` is represented in decimal
    ///
    /// # Panics
    ///
    /// Panics if `Radix` is not supported by [ibig]. (currently only 2 ~ 36 is supported)
    ///
    fn from_str(mut src: &str) -> Result<Self, ParseError> {
        assert!(X >= 2 && X <= 36, "radix is not supported");

        // determine the position of scale markers
        let e_pos = match (src.rfind('E'), src.rfind('e')) {
            (Some(_), Some(_)) => return Err(ParseError::InvalidDigit),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None
        };

        let p_pos = match (src.rfind('P'), src.rfind('p')) {
            (Some(_), Some(_)) => return Err(ParseError::InvalidDigit),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None
        };

        let scale_pos = match (e_pos, p_pos) {
            (Some(_), Some(_)) => return Err(ParseError::InvalidDigit),
            (Some(e), None) => {
                if X != 10 {
                    return Err(ParseError::InvalidDigit);
                }
                Some(e)
            },
            (None, Some(p)) => {
                if X != 16 {
                    return Err(ParseError::InvalidDigit);
                }
                Some(p)
            },
            (None, None) => None
        };

        // parse scale and remove the scale part from the str
        let scale = if let Some(pos) = scale_pos {
            let value = match isize::from_str_radix(&src[pos+1..], X as u32) {
                Err(e) => match e.kind() {
                    IntErrorKind::Empty => return Err(ParseError::NoDigits),
                    _ => return Err(ParseError::InvalidDigit),
                },
                Ok(v) => v
            };
            src = &src[..pos];
            Some(value)
        } else {
            None
        };

        // parse the body of the float number
        let result = match (src.find('.'), scale) {
            (None, None) => {
                let mantissa = IBig::from_str_radix(&src, X as u32)?;
                Self::from_parts(mantissa, 0)
            }
            (Some(dot), None) => {
                let trunc = IBig::from_str_radix(&src[..dot], X as u32)?;
                let fract = IBig::from_str_radix(&src[dot+1..], X as u32)?;
                
                let fract_digits = get_precision::<X>(&fract);
                let mantissa = trunc * IBig::from(X).pow(fract_digits) + fract;
                Self::from_parts(mantissa, -(fract_digits as isize))
            },
            (None, Some(s)) => {
                let mantissa = IBig::from_str_radix(&src, X as u32)?;
                Self::from_parts(mantissa, s)
            },
            (Some(dot), Some(s)) => {
                let trunc = IBig::from_str_radix(&src[..dot], X as u32)?;
                let fract = IBig::from_str_radix(&src[dot+1..], X as u32)?;

                let fract_digits = get_precision::<X>(&fract);
                let mantissa = trunc * IBig::from(X).pow(fract_digits) + fract;
                let exponent = s - fract_digits as isize;
                Self::from_parts(mantissa, exponent)
            },
        };

        Ok(result)
    }
}
