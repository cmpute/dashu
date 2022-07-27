use crate::{repr::FloatRepr, utils::get_precision};
use core::num::IntErrorKind;
use core::str::FromStr;
use dashu_int::{error::ParseError, IBig, fmt::{MIN_RADIX, MAX_RADIX}, UBig, Sign};

impl<const X: usize, const R: u8> FromStr for FloatRepr<X, R> {
    type Err = ParseError;

    /// Convert a string in a given base to [FloatRepr].
    ///
    /// `src` may contain an optional `+` prefix.
    /// Digits 10-35 are represented by `a-z` or `A-Z`.
    ///
    /// The valid representations include
    /// 1. `xxx` or `xxx.`
    ///     * Integer representation with optional `0b`/`0o`/`0x` prefix
    /// 1. `xxx.yyy` = `xxxyyy / radix ^ len(yyy)`
    ///     * `xxx` and `yyy` are represented in native radix `X`
    ///     * `len(yyy)` represents the number of digits in `yyy`, e.g `len(yyy)` is 3. (Same below)
    /// 1. `xxx.yyy@zz` = `xxxyyy / radix ^ len(yyy) * radix ^ zz`
    ///     * `xxx` and `yyy` are represented in native radix `X`
    ///     * Refernce: [GMP: IO of floats](https://gmplib.org/manual/I_002fO-of-Floats)
    /// 1. `xxx.yyyEzz` = `xxxyyy / radix ^ len(yyy) * 10 ^ zz`
    ///     * `E` could be lower case, radix `X` must be 10
    ///     * `xxx` and `yyy` are all represented in decimal
    /// 1. `xxx.yyyPzz` = `xxxyyy / radix ^ len(yyy) * 2 ^ zz`
    ///     * `P` could be lower case, radix `X` must be 2
    ///     * `xxx` and `yyy` are represented in binary/octal/hexadecimal with proper `0b`/`0o`/`0x` prefix.
    ///     * Reference: [C++ langauge specs](https://en.cppreference.com/w/cpp/language/floating_literal)
    /// 1. `xxx.yyyBzz` = `xxxyyy / radix ^ len(yyy) * 2 ^ zz`
    /// 1. `xxx.yyyOzz` = `xxxyyy / radix ^ len(yyy) * 8 ^ zz`
    /// 1. `xxx.yyyHzz` = `xxxyyy / radix ^ len(yyy) * 16 ^ zz`
    ///     * `B`/`O`/`H` could be lower case, and radix `X` must be consistent with the marker.
    ///     * `xxx` and `yyy` are represented in binary/octal/hexadecimal correspondingly without prefix.
    ///     * Reference: [Wikipedia: Scientific Notation](https://en.wikipedia.org/wiki/Scientific_notation#Other_bases)
    /// 
    /// Literal `xxx` and `zz` above can be signed. All `zz` are represented in decimal.
    /// Either `xxx` or `yyy` can be omitted when its value is zero, but they are not
    /// allowed to be omitted at the same time.
    ///
    /// # Panics
    /// 
    /// Panics if the radix `X` is not between [MIN_RADIX] and [MAX_RADIX] inclusive
    /// 
    fn from_str(mut src: &str) -> Result<Self, ParseError> {
        assert!(MIN_RADIX as usize <= X && X <= MAX_RADIX as usize);

        // determine the position of scale markers
        let scale_pos = match X {
            10 => src.rfind(&['e', 'E', '@']),
            2 => src.rfind(&['b', 'B', 'p', 'P', '@']),
            8 => src.rfind(&['o', 'O', '@']),
            16 => src.rfind(&['h', 'H', '@']),
            _ => src.rfind('@'),
        };

        // parse scale and remove the scale part from the str
        let (scale, pmarker) = if let Some(pos) = scale_pos {
            let value = match isize::from_str_radix(&src[pos + 1..], 10) {
                Err(e) => match e.kind() {
                    IntErrorKind::Empty => return Err(ParseError::NoDigits),
                    _ => return Err(ParseError::InvalidDigit),
                },
                Ok(v) => v,
            };
            src = &src[..pos];
            (value, src.chars().nth(pos) == Some('p') || src.chars().nth(pos) == Some('P')) 
        } else {
            (0, false)
        };

        // parse the body of the float number
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
        let mut exponent = scale;
        let mantissa = if let Some(dot) = src.find('.') {
            // check whether both integral part and fractional part are empty
            if src.len() == 1 {
                return Err(ParseError::NoDigits);
            }

            // parse integral part
            let (trunc, base) = if dot != 0 {
                if pmarker {
                    UBig::from_str_with_radix_prefix(&src[..dot])?
                } else {
                    (UBig::from_str_radix(&src[..dot], X as u32)?, X as u32)
                }
            } else {
                if pmarker {
                    // prefix is required for using `p` as scale marker
                    return Err(ParseError::UnsupportedRadix);
                }
                (UBig::zero(), X as u32)
            };

            // parse fractional part
            src = &src[dot + 1..];
            let fract = if !src.is_empty() {
                UBig::from_str_radix(src, base)?
            } else {
                UBig::zero()
            };
            let fract_digits = src.len() - src.matches('_').count();
            exponent -= fract_digits as isize;

            trunc * UBig::from(X).pow(fract_digits) + fract
        } else {
            if pmarker {
                UBig::from_str_with_radix_prefix(&src)?.0
            } else {
                UBig::from_str_radix(&src, X as u32)?
            }
        };

        Ok(Self::from_parts(sign * mantissa, exponent))
    }
}
