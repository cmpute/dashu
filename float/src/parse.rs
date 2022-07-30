use crate::{repr::FloatRepr, round::Round};
use core::str::FromStr;
use core::{marker::PhantomData, num::IntErrorKind};
use dashu_int::{
    error::ParseError,
    fmt::{MAX_RADIX, MIN_RADIX},
    IBig, Sign, UBig,
};

impl<const X: usize, R: Round> FromStr for FloatRepr<X, R> {
    type Err = ParseError;

    /// Convert a string in a given base to [FloatRepr].
    ///
    /// `src` may contain an optional `+` prefix.
    /// Digits 10-35 are represented by `a-z` or `A-Z`.
    ///
    /// The valid representations include
    /// 1. `aaa` or `aaa.`
    ///     * `aaa` is represented in native radix `X` without radix prefixes.
    /// 1. `aaa.bbb` = `aaabbb / radix ^ len(bbb)`
    ///     * `aaa` and `bbb` are represented in native radix `X` without radix prefixes.
    ///     * `len(bbb)` represents the number of digits in `bbb`, e.g `len(bbb)` is 3. (Same below)
    /// 1. `aaa.bbb@cc` = `aaabbb / radix ^ len(bbb) * radix ^ cc`
    ///     * `aaa` and `bbb` are represented in native radix `X`
    ///     * Refernce: [GMP: IO of floats](https://gmplib.org/manual/I_002fO-of-Floats)
    /// 1. `aaa.bbbEcc` = `aaabbb / radix ^ len(bbb) * 10 ^ cc`
    ///     * `E` could be lower case, radix `X` must be 10
    ///     * `aaa` and `bbb` are all represented in decimal
    /// 1. `0xaaa` or `0xaaa`
    /// 1. `0xaaa.bbb` = `aaabbb / radix ^ len(bbb)`
    /// 1. `0xaaa.bbbPcc` = `aaabbb / radix ^ len(bbb) * 2 ^ cc`
    ///     * `P` could be lower case, radix `X` must be 2 (not 16!)
    ///     * `aaa` and `bbb` are represented in hexadecimal
    ///     * Reference: [C++ langauge specs](https://en.cppreference.com/w/cpp/language/floating_literal)
    /// 1. `aaa.bbbBcc` = `aaabbb / radix ^ len(bbb) * 2 ^ cc`
    /// 1. `aaa.bbbOcc` = `aaabbb / radix ^ len(bbb) * 8 ^ cc`
    /// 1. `aaa.bbbHcc` = `aaabbb / radix ^ len(bbb) * 16 ^ cc`
    ///     * `B`/`O`/`H` could be lower case, and radix `X` must be consistent with the marker.
    ///     * `aaa` and `bbb` are represented in binary/octal/hexadecimal correspondingly without prefix.
    ///     * Reference: [Wikipedia: Scientific Notation](https://en.wikipedia.org/wiki/Scientific_notation#Other_bases)
    ///
    /// Literal `aaa` and `cc` above can be signed, but `bbb` must be unsigned.
    /// All `cc` are represented in decimal. Either `aaa` or `bbb` can be omitted
    /// when its value is zero, but they are not allowed to be omitted at the same time.
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
            let use_p = if X == 2 {
                src.bytes().nth(pos) == Some(b'p') || src.bytes().nth(pos) == Some(b'P')
            } else {
                false
            };
            src = &src[..pos];
            (value, use_p)
        } else {
            (0, false)
        };

        // parse and remove the sign
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

        // parse the body of the float number
        let mut exponent = scale;
        let ndigits;
        let mantissa = if let Some(dot) = src.find('.') {
            // check whether both integral part and fractional part are empty
            if src.len() == 1 {
                return Err(ParseError::NoDigits);
            }

            // parse integral part
            let (trunc, trunc_digits, base) = if dot != 0 {
                let trunc_str = &src[..dot];
                let has_prefix = trunc_str.starts_with("0x") || trunc_str.starts_with("0X");
                if X == 2 && has_prefix {
                    // only hexadecimal is allowed with prefix
                    let trunc_str = &trunc_str[2..];
                    let digits = 4 * (trunc_str.len() - trunc_str.matches('_').count());
                    if trunc_str.len() == 0 {
                        (UBig::ZERO, digits, 16)
                    } else {
                        (UBig::from_str_radix(&trunc_str, 16)?, digits, 16)
                    }
                } else if X == 2 && pmarker && !has_prefix {
                    return Err(ParseError::UnsupportedRadix);
                } else {
                    let digits = trunc_str.len() - trunc_str.matches('_').count();
                    (
                        UBig::from_str_radix(&src[..dot], X as u32)?,
                        digits,
                        X as u32,
                    )
                }
            } else {
                if pmarker {
                    // prefix is required for using `p` as scale marker
                    return Err(ParseError::UnsupportedRadix);
                }
                (UBig::ZERO, 0, X as u32)
            };

            // parse fractional part
            src = &src[dot + 1..];
            let (fract, fract_digits) = if !src.is_empty() {
                let mut digits = src.len() - src.matches('_').count();
                if X == 2 && base == 16 {
                    digits *= 4;
                }
                (UBig::from_str_radix(src, base)?, digits)
            } else {
                (UBig::ZERO, 0)
            };
            ndigits = trunc_digits + fract_digits;

            if fract.is_zero() {
                trunc
            } else {
                exponent -= fract_digits as isize;
                trunc * UBig::from(X).pow(fract_digits) + fract
            }
        } else {
            let has_prefix = src.starts_with("0x") || src.starts_with("0X");
            if X == 2 && has_prefix {
                src = &src[2..];
                ndigits = 4 * (src.len() - src.matches('_').count());
                UBig::from_str_radix(src, 16)?
            } else if X == 2 && pmarker && !has_prefix {
                return Err(ParseError::UnsupportedRadix);
            } else {
                ndigits = src.len() - src.matches('_').count();
                UBig::from_str_radix(&src, X as u32)?
            }
        };

        let (mantissa, exponent) = Self::normalize(sign * mantissa, exponent);
        Ok(Self {
            mantissa,
            exponent,
            precision: ndigits,
            _marker: PhantomData,
        })
    }
}
