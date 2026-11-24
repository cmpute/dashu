use crate::{
    fbig::FBig,
    repr::{Context, Repr, Word},
    round::Round,
};
use core::{num::IntErrorKind, str::FromStr};
use dashu_base::{ParseError, Sign};
use dashu_int::{
    fmt::{MAX_RADIX, MIN_RADIX},
    UBig,
};

/// Parse an unsigned run of digits (digit separators `_` allowed) in the given radix.
///
/// The sign of a literal is consumed once, up front, by [`Repr::from_str_native`]; the integer
/// parser underneath accepts a leading `+` of its own, which would otherwise let a second sign
/// slip through in the integral part (`++1`, `0x+1`) or in the fractional part (`0.+1`).
fn parse_unsigned_digits(src: &str, radix: u32) -> Result<UBig, ParseError> {
    if src.starts_with('+') {
        return Err(ParseError::InvalidDigit);
    }
    UBig::from_str_radix(src, radix)
}

impl<const B: Word> Repr<B> {
    /// Convert a string in the native base (i.e. radix `B`) to [Repr].
    ///
    /// Upon success, this method returns an [Repr] and the number of digits (in radix `B`)
    /// contained in the string.
    ///
    /// This method is the underlying implementation of the [`FromStr`] impl on
    /// [`FBig`]; see those docs for the accepted formats.
    pub(crate) fn from_str_native(mut src: &str) -> Result<(Self, usize), ParseError> {
        assert!(MIN_RADIX as Word <= B && B <= MAX_RADIX as Word);

        // B is guaranteed to be in 2..=36 by the assert above; the cast to u32
        // is needed because `from_str_radix` takes a u32 radix. On 32-bit Word
        // targets the cast is a no-op.
        #[allow(clippy::unnecessary_cast)]
        let radix: u32 = B as u32;

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

        // determine the position of scale markers
        let has_prefix = src.starts_with("0x") || src.starts_with("0X");
        let scale_pos = match B {
            10 => src.rfind(['e', 'E', '@']),
            2 => {
                if has_prefix {
                    src.rfind(['p', 'P', '@'])
                } else {
                    src.rfind(['b', 'B', '@'])
                }
            }
            8 => src.rfind(['o', 'O', '@']),
            16 => src.rfind(['h', 'H', '@']),
            _ => src.rfind('@'),
        };

        // parse scale and remove the scale part from the str
        let (scale, pmarker) = if let Some(pos) = scale_pos {
            let value = match src[pos + 1..].parse::<isize>() {
                Err(e) => match e.kind() {
                    IntErrorKind::Empty => return Err(ParseError::NoDigits),
                    _ => return Err(ParseError::InvalidDigit),
                },
                Ok(v) => v,
            };
            let use_p = if B == 2 {
                src.as_bytes().get(pos) == Some(&b'p') || src.as_bytes().get(pos) == Some(&b'P')
            } else {
                false
            };
            src = &src[..pos];
            (value, use_p)
        } else {
            (0, false)
        };

        // parse the body of the float number
        let mut exponent = scale;
        let ndigits;
        let significand = if let Some(dot) = src.find('.') {
            // check whether both integral part and fractional part are empty
            if src.len() == 1 {
                return Err(ParseError::NoDigits);
            }

            // parse integral part
            let (int, int_digits, base) = if dot != 0 {
                let int_str = &src[..dot];
                if B == 2 && has_prefix {
                    // only base 2 float is allowed using prefix
                    let int_str = &int_str[2..];
                    let digits = 4 * (int_str.len() - int_str.matches('_').count());
                    if int_str.is_empty() {
                        // the prefix alone carries no digit: `0x.` omits both parts
                        if dot + 1 == src.len() {
                            return Err(ParseError::NoDigits);
                        }
                        (UBig::ZERO, digits, 16)
                    } else {
                        (parse_unsigned_digits(int_str, 16)?, digits, 16)
                    }
                } else if B == 2 && pmarker && !has_prefix {
                    return Err(ParseError::UnsupportedRadix);
                } else {
                    let digits = int_str.len() - int_str.matches('_').count();
                    (parse_unsigned_digits(int_str, radix)?, digits, radix)
                }
            } else {
                if pmarker {
                    // prefix is required for using `p` as scale marker
                    return Err(ParseError::UnsupportedRadix);
                }
                (UBig::ZERO, 0, radix)
            };

            // parse fractional part
            src = &src[dot + 1..];
            let (fract, fract_digits) = if !src.is_empty() {
                let mut digits = src.len() - src.matches('_').count();
                if B == 2 && base == 16 {
                    digits *= 4;
                }
                (parse_unsigned_digits(src, base)?, digits)
            } else {
                (UBig::ZERO, 0)
            };
            ndigits = int_digits + fract_digits;

            if fract.is_zero() {
                int
            } else {
                // `cc - len(bbb)` can leave the exponent range when `cc` is near `isize::MIN`
                exponent = isize::try_from(fract_digits)
                    .ok()
                    .and_then(|n| exponent.checked_sub(n))
                    .ok_or(ParseError::InvalidDigit)?;
                int * UBig::from_word(B).pow(fract_digits) + fract
            }
        } else {
            let has_prefix = src.starts_with("0x") || src.starts_with("0X");
            if B == 2 && has_prefix {
                src = &src[2..];
                ndigits = 4 * (src.len() - src.matches('_').count());
                parse_unsigned_digits(src, 16)?
            } else if B == 2 && pmarker && !has_prefix {
                return Err(ParseError::UnsupportedRadix);
            } else {
                ndigits = src.len() - src.matches('_').count();
                parse_unsigned_digits(src, radix)?
            }
        };

        // A zero significand carries its sign in the exponent sentinel (`-0` is exponent -1), and
        // `sign * significand` cannot express it — negating an `IBig` zero yields zero. Without
        // this, `-0`, `-0.0` and `-0e0` all parsed as `+0` and the value did not survive its own
        // `Display` round-trip.
        //
        // For the same reason the scale of a zero must be dropped rather than stored: `0e-1`
        // would otherwise land on the `-0` sentinel and `0e<isize::MAX>` on the infinity one.
        let repr = if significand.is_zero() {
            Repr::zero_with_sign(sign)
        } else {
            let repr = Repr::new(sign * significand, exponent);
            // The extreme exponents are reserved (a nonzero significand there is neither a
            // finite number nor an infinity), and normalization saturates into them when
            // folding trailing zeros overflows; such a literal is out of range.
            if repr.exponent == isize::MAX || repr.exponent == isize::MIN {
                return Err(ParseError::InvalidDigit);
            }
            repr
        };
        Ok((repr, ndigits))
    }
}

impl<R: Round, const B: Word> FromStr for FBig<R, B> {
    type Err = ParseError;

    /// Convert a string in the native base (i.e. radix `B`) to [`FBig`].
    ///
    /// If parsing succeeds, the result is **losslessly** parsed from the input
    /// string, and its precision equals the number of significant digits in the
    /// input.
    ///
    /// **Note**: Infinites are **intentionally not supported** by this function.
    ///
    /// # Format
    ///
    /// The valid representations include
    /// 1. `aaa` or `aaa.`
    ///     * `aaa` is represented in native base `B` without base prefixes.
    /// 1. `aaa.bbb` = `aaabbb / base ^ len(bbb)`
    ///     * `aaa` and `bbb` are represented in native base `B` without base prefixes.
    ///     * `len(bbb)` represents the number of digits in `bbb`, e.g `len(bbb)` is 3. (Same below)
    /// 1. `aaa.bbb@cc` = `aaabbb * base ^ (cc - len(bbb))`
    ///     * `aaa` and `bbb` are represented in native base `B`
    ///     * This is consistent with the representation used by [GNU GMP](https://gmplib.org/manual/I_002fO-of-Floats).
    /// 1. `aaa.bbbEcc` = `aaabbb * 10 ^ (cc - len(bbb))`
    ///     * `E` could be lower case, base `B` must be 10
    ///     * `aaa` and `bbb` are all represented in decimal
    /// 1. `0xaaa` or `0xaaa`
    /// 1. `0xaaa.bbb` = `0xaaabbb / 16 ^ len(bbb)`
    /// 1. `0xaaa.bbbPcc` = `0xaaabbb / 16 ^ len(bbb) * 2 ^ cc`
    ///     * `P` could be lower case, base `B` must be 2 (not 16!)
    ///     * `aaa` and `bbb` are represented in hexadecimal
    ///     * This is consistent with the [C++ hexadecimal literals](https://en.cppreference.com/w/cpp/language/floating_literal).
    /// 1. `aaa.bbbBcc` = `aaabbb * 2 ^ (cc - len(bbb))`
    /// 1. `aaa.bbbOcc` = `aaabbb * 8 ^ (cc - len(bbb))`
    /// 1. `aaa.bbbHcc` = `aaabbb * 16 ^ (cc - len(bbb))`
    ///     * `B`/`O`/`H` could be lower case, and base `B` must be consistent with the marker.
    ///     * `aaa` and `bbb` are represented in binary/octal/hexadecimal correspondingly without prefix.
    ///     * This is consistent with some scientific notations described in [Wikipedia](https://en.wikipedia.org/wiki/Scientific_notation#Other_bases).
    ///
    /// Digits 10-35 are represented by `a-z` or `A-Z`.
    ///
    /// Literal `aaa` and `cc` above can be signed, but `bbb` must be unsigned.
    /// All `cc` are represented in decimal. Either `aaa` or `bbb` can be omitted
    /// when its value is zero, but they are not allowed to be omitted at the same time.
    ///
    /// A zero is `+0` or `-0` according to the sign of `aaa` alone; its scale `cc` is ignored.
    /// For a nonzero number the resulting exponent must lie strictly between [`isize::MIN`]
    /// and [`isize::MAX`]; a literal whose scale `cc` does not fit, or whose exponent leaves
    /// that range, is rejected with [`ParseError::InvalidDigit`].
    ///
    /// # Precision
    ///
    /// The precision of the parsed number is determined by the number of digits that are presented
    /// in the input string. For example, the numbers parsed from `12.34` or `1.234e-1` will have a
    /// precision of 4, while the ones parsed from `12.34000` or `00012.34` will have a precision of 7.
    ///
    /// # Panics
    ///
    /// Panics if the base `B` is not between [MIN_RADIX] and [MAX_RADIX] inclusive.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// # use core::str::FromStr;
    /// let a = DBig::from_str("-1.23400e-3")?;
    /// let b = DBig::from_str("-123.4@-05")?;
    /// assert_eq!(a, b);
    /// assert_eq!(a.precision(), 6);
    /// assert_eq!(b.precision(), 4);
    ///
    /// assert!(DBig::from_str("-0x1.234p-3").is_err());
    /// assert!(DBig::from_str("-1.234H-3").is_err());
    /// # Ok::<(), ParseError>(())
    /// ```
    #[inline]
    fn from_str(s: &str) -> Result<Self, ParseError> {
        let (repr, ndigits) = Repr::from_str_native(s)?;
        Ok(Self {
            repr,
            context: Context::new(ndigits),
        })
    }
}
