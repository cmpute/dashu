//! Trait definitions for bitwise operations.
//!
//! Most traits are only implemented for unsigned integers yet.

use core::{cmp::Ordering, num::FpCategory};

use crate::{
    Approximation::{self, *},
    Sign,
};

/// Bit query for integers
///
/// # Examples
///
/// ```
/// # use dashu_base::BitTest;
/// // query a bit of the number
/// assert_eq!(0b10010.bit(1), true);
/// assert_eq!(0b10010.bit(3), false);
/// assert_eq!(0b10010.bit(100), false);
/// assert_eq!((-0b10010).bit(1), true);
/// assert_eq!((-0b10010).bit(3), true);
/// assert_eq!((-0b10010).bit(31), true); // sign bit of a negative i32 is set
/// assert_eq!((-0b10010).bit(100), true);
///
/// // query the bit length of the number
/// assert_eq!(0.bit_len(), 0);
/// assert_eq!(17.bit_len(), 5);
/// assert_eq!((-17).bit_len(), 5);
/// assert_eq!(0b101000000.bit_len(), 9);
/// ```
pub trait BitTest {
    /// Effective bit length of the binary representation.
    ///
    /// For 0, the length is 0.
    ///
    /// For positive numbers it is:
    /// * number of digits in base 2
    /// * the index of the top 1 bit plus one
    /// * the floored base-2 logarithm of the number plus one.
    ///
    /// For negative numbers it is:
    /// * number of digits in base 2 without the sign
    /// * the index of the top 0 bit plus one
    /// * the floored base-2 logarithm of the absolute value of the number plus one.
    fn bit_len(&self) -> usize;

    /// Returns true if the `n`-th bit is set in its two's complement binary representation, n starts from 0.
    fn bit(&self, n: usize) -> bool;
}

/// Functions related to the power of two.
///
/// # Examples
/// ```
/// use dashu_base::PowerOfTwo;
///
/// let n = 5u32;
/// assert!(!n.is_power_of_two());
/// assert_eq!(n.next_power_of_two(), 8);
/// ```
pub trait PowerOfTwo {
    /// Test if self is a power of two (`2^k`)
    fn is_power_of_two(&self) -> bool;
    /// Get the smallest power of two greater than or equal to self.
    fn next_power_of_two(self) -> Self;
}

macro_rules! impl_bit_ops_for_uint {
    ($($T:ty)*) => {$(
        impl BitTest for $T {
            #[inline]
            fn bit_len(&self) -> usize {
                (<$T>::BITS - self.leading_zeros()) as usize
            }
            #[inline]
            fn bit(&self, position: usize) -> bool {
                if position >= <$T>::BITS as usize {
                    return false;
                } else {
                    self & (1 << position) > 0
                }
            }
        }

        impl PowerOfTwo for $T {
            #[inline]
            fn is_power_of_two(&self) -> bool {
                <$T>::is_power_of_two(*self)
            }
            #[inline]
            fn next_power_of_two(self) -> $T {
                <$T>::next_power_of_two(self)
            }
        }
    )*}
}
impl_bit_ops_for_uint!(u8 u16 u32 u64 u128 usize);

macro_rules! impl_bit_ops_for_int {
    ($($T:ty)*) => {$(
        impl BitTest for $T {
            #[inline]
            fn bit_len(&self) -> usize {
                self.unsigned_abs().bit_len()
            }
            #[inline]
            fn bit(&self, position: usize) -> bool {
                if position >= <$T>::BITS as usize {
                    return self < &0;
                } else {
                    self & (1 << position) != 0
                }
            }
        }
    )*}
}
impl_bit_ops_for_int!(i8 i16 i32 i64 i128 isize);

/// Support encoding and decoding of floats into (mantissa, exponent) parts.
///
/// See the docs of each method for the details
///
/// # Examples
///
/// ```
/// # use dashu_base::{FloatEncoding, Approximation::*, Sign::*};
/// use core::num::FpCategory;
///
/// assert_eq!(0f64.decode(), Ok((0, -1074))); // exponent will not be reduced
/// assert_eq!(1f32.decode(), Ok((1 << 23, -23)));
/// assert_eq!(f32::INFINITY.decode(), Err(FpCategory::Infinite));
///
/// assert_eq!(f64::encode(0, 1), Exact(0f64));
/// assert_eq!(f32::encode(1, 0), Exact(1f32));
/// assert_eq!(f32::encode(i32::MAX, 100), Inexact(f32::INFINITY, Positive));
/// ```
pub trait FloatEncoding {
    /// The integer type used to represent the decoded mantissa.
    type Mantissa;
    /// The integer type used to represent the decoded binary exponent.
    type Exponent;

    /// Convert a float number `mantissa * 2^exponent` into `(mantissa, exponent)` parts faithfully.
    ///
    /// This method will not reduce the result (e.g. turn `2 * 2^-1` into `1 * 2^0`), and it
    /// will return [Err] when the float number is nan or infinite.
    fn decode(self) -> Result<(Self::Mantissa, Self::Exponent), FpCategory>;

    /// Convert `(mantissa, exponent)` to `mantissa * 2^exponent` faithfully.
    ///
    /// It won't generate `NaN` values. However if the actual value is out of the
    /// representation range, it might return an infinity or subnormal number.
    ///
    /// If any rounding happened during the conversion, it should follow the default
    /// behavior defined by IEEE 754 (round to nearest, ties to even)
    ///
    /// The returned approximation is exact if the input can be exactly representable by f32,
    /// otherwise the error field of the approximation contains the sign of `result - mantissa * 2^exp`.
    fn encode(mantissa: Self::Mantissa, exponent: Self::Exponent) -> Approximation<Self, Sign>
    where
        Self: Sized;
}

/// Shared implementation of [FloatEncoding::encode] for the primitive floats.
///
/// The magnitude is rounded once, directly at the position of the unit in the last place
/// of the result (which is clamped at the smallest subnormal), so that every discarded bit
/// takes part in the rounding decision.
macro_rules! encode_float {
    ($float:ty, $uint:ty, $mantissa:ident, $exponent:ident) => {{
        if $mantissa == 0 {
            return Exact(0.0);
        }

        const BITS: i32 = <$uint>::BITS as i32;
        const SIGNIFICAND_BITS: i32 = <$float>::MANTISSA_DIGITS as i32;
        // exponent of the least significant bit of subnormal numbers
        const MIN_ULP_EXP: i32 = <$float>::MIN_EXP - SIGNIFICAND_BITS;

        let negative = $mantissa < 0;
        let sign = Sign::from(negative);
        let sign_bit = (negative as $uint) << (BITS - 1);
        let man = $mantissa.unsigned_abs();

        // the magnitude lies in the range [2^(top_bit - 1), 2^top_bit)
        let top_bit = BITS - man.leading_zeros() as i32 + $exponent as i32;
        if top_bit > <$float>::MAX_EXP {
            return match sign {
                Sign::Positive => Inexact(<$float>::INFINITY, Sign::Positive),
                Sign::Negative => Inexact(<$float>::NEG_INFINITY, Sign::Negative),
            };
        }

        // the exponent of the unit in the last place of the result, and the number
        // of low bits of the mantissa that fall below it
        let ulp_exp = if top_bit - SIGNIFICAND_BITS > MIN_ULP_EXP {
            top_bit - SIGNIFICAND_BITS
        } else {
            MIN_ULP_EXP
        };
        let shift = ulp_exp - $exponent as i32;

        // split the mantissa into the kept part and the discarded part, the
        // latter is compared with half of the unit in the last place.
        let (kept, discarded) = if shift <= 0 {
            // at most SIGNIFICAND_BITS bits are kept, so the shift never overflows
            (man << (-shift) as u32, None)
        } else if shift > BITS {
            (0, Some(Ordering::Less))
        } else if shift == BITS {
            (0, Some(man.cmp(&(1 << (BITS - 1)))))
        } else {
            let low = man & ((1 << shift) - 1);
            let cmp = if low == 0 {
                None
            } else {
                Some(low.cmp(&(1 << (shift - 1))))
            };
            (man >> shift, cmp)
        };

        // The kept part has at most SIGNIFICAND_BITS bits. It is added to (rather than or-ed with) the
        // exponent field, so that the implicit top bit of a normal number increases the
        // exponent field from `ulp_exp - MIN_ULP_EXP` to its final value. For the same reason,
        // a carry from rounding up correctly moves to the next binade (or to infinity).
        let bits = sign_bit | ((((ulp_exp - MIN_ULP_EXP) as $uint) << (SIGNIFICAND_BITS - 1)) + kept);
        match discarded {
            None => Exact(<$float>::from_bits(bits)),
            Some(cmp) => {
                let round_up = match cmp {
                    Ordering::Less => false,
                    Ordering::Equal => kept & 1 == 1,
                    Ordering::Greater => true,
                };
                if round_up {
                    Inexact(<$float>::from_bits(bits + 1), Sign::Positive * sign)
                } else {
                    Inexact(<$float>::from_bits(bits), Sign::Negative * sign)
                }
            }
        }
    }};
}

impl FloatEncoding for f32 {
    type Mantissa = i32;
    type Exponent = i16;

    #[inline]
    fn decode(self) -> Result<(i32, i16), FpCategory> {
        let bits: u32 = self.to_bits();
        let sign_bit = bits >> 31;
        let mantissa_bits = bits & 0x7fffff;

        // deal with inf/nan values
        let mut exponent = ((bits >> 23) & 0xff) as i16;
        if exponent == 0xff {
            return if mantissa_bits != 0 {
                Err(FpCategory::Nan)
            } else {
                Err(FpCategory::Infinite)
            };
        }

        // then parse values
        let mantissa = if exponent == 0 {
            // subnormal
            exponent = -126 - 23;
            mantissa_bits
        } else {
            // normal
            exponent -= 127 + 23; // bias + mantissa shift
            mantissa_bits | 0x800000
        } as i32;

        let sign = Sign::from(sign_bit > 0);
        Ok((mantissa * sign, exponent))
    }

    #[inline]
    fn encode(mantissa: i32, exponent: i16) -> Approximation<Self, Sign> {
        encode_float!(f32, u32, mantissa, exponent)
    }
}

impl FloatEncoding for f64 {
    type Mantissa = i64;
    type Exponent = i16;

    #[inline]
    fn decode(self) -> Result<(i64, i16), FpCategory> {
        let bits: u64 = self.to_bits();
        let sign_bit = bits >> 63;
        let mantissa_bits = bits & 0xfffffffffffff;

        // deal with inf/nan values
        let mut exponent = ((bits >> 52) & 0x7ff) as i16;
        if exponent == 0x7ff {
            return if mantissa_bits != 0 {
                Err(FpCategory::Nan)
            } else {
                Err(FpCategory::Infinite)
            };
        }

        // then parse values
        let mantissa = if exponent == 0 {
            // subnormal
            exponent = -1022 - 52;
            mantissa_bits
        } else {
            // normal
            exponent -= 1023 + 52; // bias + mantissa shift
            mantissa_bits | 0x10000000000000
        } as i64;

        if sign_bit == 0 {
            Ok((mantissa, exponent))
        } else {
            Ok((-mantissa, exponent))
        }
    }

    #[inline]
    fn encode(mantissa: i64, exponent: i16) -> Approximation<Self, Sign> {
        encode_float!(f64, u64, mantissa, exponent)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_float_encoding() {
        // special values
        assert_eq!(f32::INFINITY.decode(), Err(FpCategory::Infinite));
        assert_eq!(f32::NEG_INFINITY.decode(), Err(FpCategory::Infinite));
        assert_eq!(f32::NAN.decode(), Err(FpCategory::Nan));
        assert_eq!(f64::INFINITY.decode(), Err(FpCategory::Infinite));
        assert_eq!(f64::NEG_INFINITY.decode(), Err(FpCategory::Infinite));
        assert_eq!(f64::NAN.decode(), Err(FpCategory::Nan));

        // round trip test
        let f32_cases = [
            0.,
            -1.,
            1.,
            f32::MIN,
            f32::MAX,
            f32::MIN_POSITIVE,
            -f32::MIN_POSITIVE,
            f32::EPSILON,
            f32::from_bits(0x1),      // smallest f32
            f32::from_bits(0x7ff),    // some subnormal value
            f32::from_bits(0x7fffff), // largest subnormal number
            f32::from_bits(0x800000), // smallest normal number
            -123.4567,
            core::f32::consts::PI,
        ];
        for f in f32_cases {
            let (man, exp) = f.decode().unwrap();
            assert_eq!(f32::encode(man, exp), Exact(f));
        }

        let f64_cases = [
            0.,
            -1.,
            1.,
            f64::MIN,
            f64::MAX,
            f64::MIN_POSITIVE,
            -f64::MIN_POSITIVE,
            f64::EPSILON,
            f64::from_bits(0x1),              // smallest f64
            f64::from_bits(0x7fffff),         // largest subnormal number
            f64::from_bits(0xfffffffffffff),  // some subnormal value
            f64::from_bits(0x10000000000000), // smallest normal number
            -123456.789012345,
            core::f64::consts::PI,
        ];
        for f in f64_cases {
            let (man, exp) = f.decode().unwrap();
            assert_eq!(f64::encode(man, exp), Exact(f));
        }

        // test out of ranges
        assert_eq!(f32::encode(1, 128), Inexact(f32::INFINITY, Sign::Positive));
        assert_eq!(f32::encode(-1, 128), Inexact(f32::NEG_INFINITY, Sign::Negative));
        assert_eq!(f32::encode(1, -150), Inexact(0f32, Sign::Negative));
        assert_eq!(f32::encode(-1, -150), Inexact(-0f32, Sign::Positive));
        assert_eq!(f64::encode(1, 1024), Inexact(f64::INFINITY, Sign::Positive));
        assert_eq!(f64::encode(-1, 1024), Inexact(f64::NEG_INFINITY, Sign::Negative));
        assert_eq!(f64::encode(1, -1075), Inexact(0f64, Sign::Negative));
        assert_eq!(f64::encode(-1, -1075), Inexact(-0f64, Sign::Positive));

        // regression: a very large positive exponent must overflow to infinity rather than
        // produce NaN. The `top_bit = (BITS - zeros) + exponent` bound is computed in i32
        // precisely so that huge exponents route to the overflow branch (an earlier i16
        // computation wrapped and misrouted them, e.g. encode(256, 32759) -> NaN).
        assert_eq!(f32::encode(256, 32759), Inexact(f32::INFINITY, Sign::Positive));
        assert_eq!(f64::encode(256, 32759), Inexact(f64::INFINITY, Sign::Positive));

        // test rounding
        assert_eq!(f32::encode(3, -150), Inexact(f32::from_bits(0x00000002), Sign::Positive));
        assert_eq!(f32::encode(-5, -150), Inexact(f32::from_bits(0x80000002), Sign::Positive));
        assert_eq!(f32::encode(i32::MAX, 50), Inexact(f32::from_bits(0x68000000), Sign::Positive));
        assert_eq!(
            f32::encode(i32::MAX, -150),
            Inexact(f32::from_bits(0x04000000), Sign::Positive)
        );
        assert_eq!(
            f32::encode(i32::MAX, -160),
            Inexact(f32::from_bits(0x00100000), Sign::Positive)
        );
        assert_eq!(
            f32::encode(i32::MAX, -170),
            Inexact(f32::from_bits(0x00000400), Sign::Positive)
        );
        assert_eq!(
            f64::encode(3, -1075),
            Inexact(f64::from_bits(0x0000000000000002), Sign::Positive)
        );
        assert_eq!(
            f64::encode(-5, -1075),
            Inexact(f64::from_bits(0x8000000000000002), Sign::Positive)
        );
        assert_eq!(
            f64::encode(i64::MAX, 500),
            Inexact(f64::from_bits(0x6320000000000000), Sign::Positive)
        );
        assert_eq!(
            f64::encode(i64::MAX, -1075),
            Inexact(f64::from_bits(0x00b0000000000000), Sign::Positive)
        );
        assert_eq!(
            f64::encode(i64::MAX, -1095),
            Inexact(f64::from_bits(0x0000040000000000), Sign::Positive)
        );
        assert_eq!(f64::encode(i64::MAX, -1115), Inexact(f64::from_bits(0x400000), Sign::Positive));

        // other cases
        assert_eq!(f32::encode(1, 0), Exact(1f32));
        assert_eq!(f64::encode(1, 0), Exact(1f64));
        assert_eq!(f32::encode(0x1000000, -173), Exact(f32::from_bits(0x1)));
        assert_eq!(f64::encode(0x40000000000000, -1128), Exact(f64::from_bits(0x1)));
    }
}
