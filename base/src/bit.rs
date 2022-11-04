//! Trait definitions for bitwise operations.
//!
//! Most traits are only implemented for unsigned integers yet.

use core::num::FpCategory;

use crate::{
    Approximation::{self, *},
    Sign::{self, *},
};

/// Common bit operations for integers
// TODO(v0.3): add doc tests
pub trait BitTest {
    /// Get the minimum required number of bits to represent this integer
    fn bit_len(&self) -> usize;

    /// Get the n-th bit of the integer
    fn bit(&self, n: usize) -> bool;

    /// Get the number of trailing zeros in the integer
    fn trailing_zeros(&self) -> Option<usize>;

    // TODO(v0.3): add trailing_ones
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

macro_rules! impl_bit_ops_prim {
    ($($T:ty)*) => {$(
        impl BitTest for $T {
            #[inline]
            fn bit_len(&self) -> usize {
                (<$T>::BITS - self.leading_zeros()) as usize
            }
            #[inline]
            fn bit(&self, position: usize) -> bool {
                self & (1 << position) > 0
            }
            #[inline]
            fn trailing_zeros(&self) -> Option<usize> {
                if *self == 0 {
                    None
                } else {
                    Some(<$T>::trailing_zeros(*self) as usize)
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
impl_bit_ops_prim!(u8 u16 u32 u64 u128 usize);

pub trait FloatEncoding {
    type Mantissa;
    type Exponent;

    /// This method should return [Err] when the float number is nan or infinite
    fn decode(self) -> Result<(Self::Mantissa, Self::Exponent), FpCategory>;

    /// This method converts (mantissa, exponent) to mantissa * 2^exponent faithfully.
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

/// Round to even floating point adjustment, based on the bottom
/// bit of mantissa and additional 2 bits (i.e. 3 bits in units of ULP/4).
#[inline]
fn round_to_even_adjustment(bits: u8) -> bool {
    bits >= 0b110 || bits == 0b011
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
        if mantissa == 0 {
            return Exact(0f32);
        }

        // clear sign
        let sign = (mantissa < 0) as u32;
        let mut mantissa = mantissa.unsigned_abs();

        let zeros = mantissa.leading_zeros();
        let topbit = (u32::BITS - zeros) as i16 + exponent;

        if topbit > 128 {
            // overflow
            return if sign == 0 {
                Inexact(f32::INFINITY, Sign::Positive)
            } else {
                Inexact(f32::NEG_INFINITY, Sign::Negative)
            };
        } else if topbit < -125 - 23 {
            // underflow
            return if sign == 0 {
                Inexact(0f32, Sign::Negative)
            } else {
                Inexact(-0f32, Sign::Positive)
            };
        };

        let bits; // bit representation
        let round_bits; // for rounding
        if topbit <= -125 {
            // subnormal float
            // (this branch includes 1e-125, the smallest positive normal f32)

            // first remove the exponent
            let shift = exponent + 126 + 23;
            if shift >= 0 {
                round_bits = 0; // not rounding is required
                mantissa <<= shift as u32;
            } else {
                let shifted = mantissa << (30 + shift) as u32;
                round_bits = (shifted >> 28 & 0b110) as u8 | ((shifted & 0xfffffff) != 0) as u8;
                mantissa >>= (-shift) as u32;
            }

            // then compose the bit representation of f32
            bits = (sign << 31) | mantissa;
        } else {
            // normal float
            // first normalize the mantissa (and remove the top bit)
            if mantissa == 1 {
                mantissa = 0; // shl will overflow
            } else {
                mantissa <<= zeros + 1;
            }

            // then calculate the exponent (bias is 127)
            let exponent = (exponent + 127 + u32::BITS as i16) as u32 - zeros - 1;

            // then compose the bit representation of f32
            bits = (sign << 31) | (exponent << 23) | (mantissa >> 9);

            // get the low bit of mantissa and two extra bits, and adding round-to-even adjustment
            round_bits = ((mantissa >> 7) & 0b110) as u8 | ((mantissa & 0x7f) != 0) as u8;
        };

        if round_bits & 0b11 == 0 {
            // If two extra bits are all zeros, then the float is exact
            Exact(f32::from_bits(bits))
        } else {
            let sign = Sign::from(sign > 0);
            if round_to_even_adjustment(round_bits) {
                // If the mantissa overflows, this correctly increases the exponent and sets the mantissa to 0.
                // If the exponent overflows, we correctly get the representation of infinity.
                Inexact(f32::from_bits(bits + 1), Positive * sign)
            } else {
                Inexact(f32::from_bits(bits), Negative * sign)
            }
        }
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
        if mantissa == 0 {
            return Exact(0f64);
        }

        // clear sign
        let sign = (mantissa < 0) as u64;
        let mut mantissa = mantissa.unsigned_abs();

        let zeros = mantissa.leading_zeros();
        let topbit = (u64::BITS - zeros) as i16 + exponent;

        if topbit > 1024 {
            // overflow
            return if sign == 0 {
                Inexact(f64::INFINITY, Sign::Positive)
            } else {
                Inexact(f64::NEG_INFINITY, Sign::Negative)
            };
        } else if topbit < -1022 - 52 {
            // underflow
            return if sign == 0 {
                Inexact(0f64, Sign::Negative)
            } else {
                Inexact(-0f64, Sign::Positive)
            };
        };

        let bits; // bit representation
        let round_bits; // for rounding
        if topbit <= -1022 {
            // subnormal float
            // (this branch includes 1e-1022, the smallest positive normal f32)

            // first remove the exponent
            let shift = exponent + 1022 + 52;
            if shift >= 0 {
                round_bits = 0; // not rounding is required
                mantissa <<= shift as u32;
            } else {
                let shifted = mantissa << (62 + shift) as u64;
                round_bits =
                    (shifted >> 60 & 0b110) as u8 | ((shifted & 0xfffffffffffffff) != 0) as u8;
                mantissa >>= (-shift) as u32;
            }

            // then compose the bit representation of f64
            bits = (sign << 63) | mantissa;
        } else {
            // normal float
            // first normalize the mantissa (and remove the top bit)
            if mantissa == 1 {
                mantissa = 0; // shl will overflow
            } else {
                mantissa <<= zeros + 1;
            }

            // then calculate the exponent (bias is 1023)
            let exponent = (exponent + 1023 + u64::BITS as i16) as u64 - zeros as u64 - 1;

            // then compose the bit representation of f64
            bits = (sign << 63) | (exponent << 52) | (mantissa >> 12);

            // get the low bit of mantissa and two extra bits, and adding round-to-even adjustment
            round_bits = ((mantissa >> 10) & 0b110) as u8 | ((mantissa & 0x3ff) != 0) as u8;
        };

        if round_bits & 0b11 == 0 {
            // If two extra bits are all zeros, then the float is exact
            Exact(f64::from_bits(bits))
        } else {
            let sign = Sign::from(sign > 0);
            if round_to_even_adjustment(round_bits) {
                // If the mantissa overflows, this correctly increases the exponent and sets the mantissa to 0.
                // If the exponent overflows, we correctly get the representation of infinity.
                Inexact(f64::from_bits(bits + 1), Positive * sign)
            } else {
                Inexact(f64::from_bits(bits), Negative * sign)
            }
        }
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
            3.14159265,
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
            3.141592653979323,
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
