//! Implement supports for PostgreSQL related crates.

#[cfg(feature = "diesel_v1")]
mod diesel_v1;

#[cfg(feature = "diesel_v2")]
mod diesel_v2;

#[cfg(feature = "postgres-types")]
mod postgres_types;

use dashu_base::{ConversionError, EstimatedLog2, Sign};
use dashu_int::UBig;

use crate::{
    fbig::FBig,
    repr::{Context, Repr},
    round::Round,
};
use alloc::vec::Vec;

/// Represents the NUMERIC value in PostgreSQL, closely mirroring the PG wire protocol without NaN.
///
/// Note that the NUMERIC type in PostgreSQL is actually a fixed point representation. All the digits
/// are counting from the decimal point. Therefore be careful about it, for example, `1e3` in dashu
/// has 1 digit precision, but it has 4 digits precision in PostgreSQL (because 1000 has four digits).
///
/// Reference: <https://github.com/postgres/postgres/blob/master/src/backend/utils/adt/numeric.c#L253>
///
/// # Representation Examples
///
/// |     value | weight | scale | digits              |
/// |----------:|:------:|:-----:|:--------------------|
/// | 123456780 |   2    |   0   | `[1, 2345, 6780]`   |
/// |  12345678 |   1    |   0   | `[1234, 5678]`      |
/// | 1234567.8 |   1    |   1   | `[123, 4567, 8000]` |
/// | 123456.78 |   1    |   2   | `[12, 3456, 7800]`  |
/// | 12345.678 |   1    |   3   | `[1, 2345, 6780]`   |
/// | 1234.5678 |   0    |   4   | `[1234, 5678]`      |
///
/// basically, `value = digits * NBASE ^ (weight + 1 - len(digits))`
#[derive(Debug, PartialEq)]
pub(in crate::third_party::postgres) struct Numeric {
    // The sign and infinity flags are both stored in the `sign` field in a PG numeric
    pub sign: Sign,
    pub is_inf: bool,

    /// The exponent of the first digit in base NBASE (= 10000)
    pub weight: i16,

    /// Number of digits in base 10 after the decimal point
    pub dscale: u16,

    /// The actual digit, should be an iterator
    pub digits: Vec<i16>,
}

impl Numeric {
    const fn infinity() -> Self {
        Numeric {
            sign: Sign::Positive,
            is_inf: true,
            weight: 0,
            dscale: 0,
            digits: Vec::new(),
        }
    }

    const fn neg_infinity() -> Self {
        Numeric {
            sign: Sign::Negative,
            is_inf: true,
            weight: 0,
            dscale: 0,
            digits: Vec::new(),
        }
    }

    const fn zero() -> Self {
        Numeric {
            sign: Sign::Positive,
            is_inf: false,
            weight: 0,
            dscale: 0,
            digits: Vec::new(),
        }
    }
}

/// The largest `dscale` the server accepts. The wire format has 16 bits for it, but the server
/// keeps 14 (its `NUMERIC_DSCALE_MASK` is `0x3FFF`) and refuses a value received with a larger
/// one ("invalid scale in external numeric value").
const MAX_DSCALE: usize = 0x3FFF;

/// The number of decimal digits before the decimal point that the format can hold: the weight
/// of the leading base-10000 digit is an `i16`.
const MAX_INTEGER_DIGITS: usize = (i16::MAX as usize + 1) * 4;

/// Number of decimal digits that the leading base-10000 digit lacks to be a full four-digit
/// group. The digit is expected to be in `1..=9999`; anything else is counted as a full group.
#[inline]
const fn leading_decimal_zeros(n: &i16) -> usize {
    match n {
        100..=999 => 1,
        10..=99 => 2,
        1..=9 => 3,
        _ => 0,
    }
}

// returns the representation and the precision
fn numeric_to_repr(num: Numeric) -> (Repr<10>, usize) {
    // shortcut for infinities and zeros
    if num.is_inf {
        return match num.sign {
            Sign::Positive => (Repr::<10>::infinity(), 0),
            Sign::Negative => (Repr::<10>::neg_infinity(), 0),
        };
    }

    // The server strips leading zero digits, but other producers might not (and a zero can
    // be written with explicit zero digits), so do not rely on the first digit being nonzero.
    let leading = num.digits.iter().take_while(|d| **d == 0).count();
    let nonzero = &num.digits[leading..];
    if nonzero.is_empty() {
        // NUMERIC has no negative zero, and a zero has no significant digits
        return (Repr::zero(), 0);
    }

    // calculate the significand
    const NBASE: UBig = UBig::from_word(10000);
    let mut signif = UBig::ZERO;
    for d in nonzero.iter() {
        signif *= NBASE;
        signif += *d as u16;
    }

    let exp = (num.weight as isize + 1 - num.digits.len() as isize) * 4;
    let repr = Repr::new(num.sign * signif, exp);

    // The precision counts the decimal digits from the leading one down to `dscale` places
    // after the decimal point. It is never less than the number of significant digits, even
    // if `dscale` claims fewer fractional digits than there are.
    let digit_len = (nonzero.len() * 4 - leading_decimal_zeros(&nonzero[0])) as isize;
    let significant = digit_len - (repr.exponent - exp);
    let precision = (digit_len + exp + num.dscale as isize).max(significant) as usize;

    (repr, precision)
}

impl From<Numeric> for Repr<10> {
    #[inline]
    fn from(num: Numeric) -> Self {
        numeric_to_repr(num).0
    }
}

impl<R: Round> From<Numeric> for FBig<R, 10> {
    #[inline]
    fn from(num: Numeric) -> Self {
        let (repr, precision) = numeric_to_repr(num);
        let context = Context::new(precision);
        Self::from_repr(repr, context)
    }
}

fn repr_to_numeric(repr: &Repr<10>, precision: Option<usize>) -> Result<Numeric, ConversionError> {
    // shortcut for infinities and zeros
    if repr.is_infinite() {
        return Ok(match repr.sign() {
            Sign::Positive => Numeric::infinity(),
            Sign::Negative => Numeric::neg_infinity(),
        });
    } else if repr.significand.is_zero() {
        // NUMERIC has no negative zero: both zeros are stored as zero
        return Ok(Numeric::zero());
    }

    // Check the range before the significand is expanded into digits. Every digit after the
    // decimal point has to be covered by `dscale`, and the leading digit by the weight; a
    // number that passes has at most `MAX_INTEGER_DIGITS + MAX_DSCALE` decimal digits, which is
    // well within the 16-bit count of base-10000 digits. (The magnitude test is a cheap filter
    // on the logarithm; the weight is checked exactly below.)
    let required_digits = repr
        .exponent
        .checked_neg()
        .ok_or(ConversionError::OutOfBounds)?;
    if required_digits > MAX_DSCALE as isize {
        return Err(ConversionError::OutOfBounds);
    }
    let log10_lb = repr.log2_bounds().0 * core::f32::consts::LOG10_2;
    if log10_lb > (MAX_INTEGER_DIGITS + 1) as f32 {
        return Err(ConversionError::OutOfBounds);
    }
    let digit_len_est = repr.significand.log2_bounds().1 * core::f32::consts::LOG10_2;
    let mut digits = Vec::with_capacity(digit_len_est as usize / 4 + 2);

    // destruct the repr
    let Repr {
        significand,
        exponent,
    } = repr.clone();
    let (sign, mut signif) = significand.into_parts();

    // represent the exponent in base NBASE = 10000
    let exp_rem = exponent.rem_euclid(4);
    if exp_rem != 0 {
        signif *= 10u16.pow(exp_rem as u32);
    };
    // exponent of the last digit, which is a multiple of 4
    let exp = exponent
        .checked_sub(exp_rem)
        .ok_or(ConversionError::OutOfBounds)?;
    let weight = exp / 4; // exponent in base NBASE

    // calculate the actual digits (base-NBASE = 10000, most significant first)
    digits.extend(signif.to_digits(10000).into_iter().map(|d| d as i16));

    // Calculate the number of digits after the decimal point. The server truncates the value
    // to `dscale` fractional digits, so it must cover every fractional digit of the value. A
    // precision beyond the significant digits adds trailing zeros, and a precision below them
    // (a guard digit) must not take any digit away.
    let fract_digits = match precision {
        Some(prec) if prec > 0 => {
            let leading_zeros = leading_decimal_zeros(digits.first().unwrap());
            let digit_len = digits.len() * 4 - leading_zeros - exp_rem as usize;
            let fract_digits = exponent.checked_neg().ok_or(ConversionError::OutOfBounds)?;
            match prec.checked_sub(digit_len) {
                // the padding is cosmetic, so it stops at the largest representable scale
                Some(padding) => fract_digits.saturating_add_unsigned(padding),
                None => fract_digits,
            }
        }
        _ => exp.checked_neg().ok_or(ConversionError::OutOfBounds)?,
    };
    let dscale = fract_digits.clamp(0, MAX_DSCALE as isize) as u16;

    let weight = weight
        .checked_add(digits.len() as isize - 1)
        .and_then(|w| i16::try_from(w).ok())
        .ok_or(ConversionError::OutOfBounds)?;

    Ok(Numeric {
        sign,
        is_inf: false,
        weight,
        dscale,
        digits,
    })
}

impl TryFrom<&Repr<10>> for Numeric {
    type Error = ConversionError;

    #[inline]
    fn try_from(value: &Repr<10>) -> Result<Self, Self::Error> {
        repr_to_numeric(value, None)
    }
}

impl<R: Round> TryFrom<&FBig<R, 10>> for Numeric {
    type Error = ConversionError;

    #[inline]
    fn try_from(value: &FBig<R, 10>) -> Result<Self, Self::Error> {
        repr_to_numeric(&value.repr, Some(value.context.precision))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DBig;
    use core::str::FromStr;

    #[test]
    fn test_conversion_between_dbig_and_numeric() {
        let decimal = DBig::ZERO;
        let expected = Numeric {
            sign: Sign::Positive,
            is_inf: false,
            weight: 0,
            dscale: 0,
            digits: vec![],
        };
        assert_eq!(expected, (&decimal).try_into().unwrap());
        let parsed: DBig = expected.into();
        assert_eq!(parsed, decimal);
        assert_eq!(parsed.precision(), 0);

        let decimal = DBig::ONE;
        let expected = Numeric {
            sign: Sign::Positive,
            is_inf: false,
            weight: 0,
            dscale: 0,
            digits: vec![1],
        };
        assert_eq!(expected, (&decimal).try_into().unwrap());
        let parsed: DBig = expected.into();
        assert_eq!(parsed, decimal);
        assert_eq!(parsed.precision(), 1);

        let decimal = DBig::NEG_ONE;
        let expected = Numeric {
            sign: Sign::Negative,
            is_inf: false,
            weight: 0,
            dscale: 0,
            digits: vec![1],
        };
        assert_eq!(expected, (&decimal).try_into().unwrap());
        let parsed: DBig = expected.into();
        assert_eq!(parsed, decimal);
        assert_eq!(parsed.precision(), 1);

        let decimal = DBig::from_str("1e4").unwrap();
        let expected = Numeric {
            sign: Sign::Positive,
            is_inf: false,
            weight: 1, // 1e4 = 1 * NBASE
            dscale: 0, // 1e4 has no digits after decimal points
            digits: vec![1],
        };
        assert_eq!(expected, (&decimal).try_into().unwrap());
        let parsed: DBig = expected.into();
        assert_eq!(parsed, decimal);
        assert_eq!(parsed.precision(), 5); // integers always has full precision

        let decimal = DBig::from_str("-10000.00").unwrap();
        let expected = Numeric {
            sign: Sign::Negative,
            is_inf: false,
            weight: 1, // 10000.00 = 1 * NBASE
            dscale: 2, // 10000.00 has 2 digits after decimal points
            digits: vec![1],
        };
        assert_eq!(expected, (&decimal).try_into().unwrap());
        let parsed: DBig = expected.into();
        assert_eq!(parsed, decimal);
        assert_eq!(parsed.precision(), 7);

        let decimal = DBig::from_str("1e6").unwrap();
        let expected = Numeric {
            sign: Sign::Positive,
            is_inf: false,
            weight: 1, // 1e6 = 100 * NBASE
            dscale: 0, // 1e6 has no digits after decimal points
            digits: vec![100],
        };
        assert_eq!(expected, (&decimal).try_into().unwrap());
        let parsed: DBig = expected.into();
        assert_eq!(parsed, decimal);
        assert_eq!(parsed.precision(), 7); // integers always has full precision

        let decimal = DBig::from_str("-1000000.0000").unwrap();
        let expected = Numeric {
            sign: Sign::Negative,
            is_inf: false,
            weight: 1, // 1000000.0000 = 100 * NBASE
            dscale: 4, // 1000000.0000 has 4 digits after decimal points
            digits: vec![100],
        };
        assert_eq!(expected, (&decimal).try_into().unwrap());
        let parsed: DBig = expected.into();
        assert_eq!(parsed, decimal);
        assert_eq!(parsed.precision(), 11);

        let decimal = DBig::from_str("1e-4").unwrap();
        let expected = Numeric {
            sign: Sign::Positive,
            is_inf: false,
            weight: -1, // 1e-4 = 1 * NBASE ^ -1
            dscale: 4,  // 1e-4 = 0.0001 has 4 digits after decimal points
            digits: vec![1],
        };
        assert_eq!(expected, (&decimal).try_into().unwrap());
        let parsed: DBig = expected.into();
        assert_eq!(parsed, decimal);
        assert_eq!(parsed.precision(), 1);

        // 000.0001 has a precision of 7 digits, so it's considered as "1.000000e-4"
        let decimal = DBig::from_str("-000.0001").unwrap();
        let expected = Numeric {
            sign: Sign::Negative,
            is_inf: false,
            weight: -1, // 1.000000e-4 = 1 * NBASE ^ -1
            dscale: 10, // 1.000000e-4 = 0.0001000000 has 10 digits after decimal points
            digits: vec![1],
        };
        assert_eq!(expected, (&decimal).try_into().unwrap());
        let parsed: DBig = expected.into();
        assert_eq!(parsed, decimal);
        assert_eq!(parsed.precision(), 7);

        let decimal = DBig::from_str("1e-6").unwrap();
        let expected = Numeric {
            sign: Sign::Positive,
            is_inf: false,
            weight: -2, // 1e-6 = 100 * NBASE ^ -2
            dscale: 6,  // 1e-6 = 0.000001 has 6 digits after decimal points
            digits: vec![100],
        };
        assert_eq!(expected, (&decimal).try_into().unwrap());
        let parsed: DBig = expected.into();
        assert_eq!(parsed, decimal);
        assert_eq!(parsed.precision(), 1);

        // 00000.000001 has a precision of 11 digits, so it's considered as "1.0000000000e-6"
        let decimal = DBig::from_str("-00000.000001").unwrap();
        let expected = Numeric {
            sign: Sign::Negative,
            is_inf: false,
            weight: -2, // 1.0000000000e-6 = 100 * NBASE ^ -2
            dscale: 16, // 0.0000010000000000 has 16 digits after decimal points
            digits: vec![100],
        };
        assert_eq!(expected, (&decimal).try_into().unwrap());
        let parsed: DBig = expected.into();
        assert_eq!(parsed, decimal);
        assert_eq!(parsed.precision(), 11);
    }
}
