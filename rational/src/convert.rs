use core::cmp::Ordering;

use dashu_base::{
    Approximation::{self, *},
    BitTest, ConversionError, DivRem, FloatEncoding, Sign, UnsignedAbs,
};
use dashu_int::{IBig, UBig, Word};

use crate::{
    rbig::{RBig, Relaxed},
    repr::Repr,
};

impl From<UBig> for Repr {
    #[inline]
    fn from(v: UBig) -> Self {
        Repr {
            numerator: v.into(),
            denominator: UBig::ONE,
        }
    }
}

impl From<IBig> for Repr {
    #[inline]
    fn from(v: IBig) -> Self {
        Repr {
            numerator: v,
            denominator: UBig::ONE,
        }
    }
}

impl TryFrom<Repr> for UBig {
    type Error = ConversionError;
    #[inline]
    fn try_from(value: Repr) -> Result<Self, Self::Error> {
        // Integer iff the (reduced) denominator is 1; then the value is the numerator magnitude.
        if !value.denominator.is_one() {
            return Err(ConversionError::LossOfPrecision);
        }
        let (sign, mag) = value.numerator.into_parts();
        if sign == Sign::Negative {
            Err(ConversionError::OutOfBounds)
        } else {
            Ok(mag)
        }
    }
}

impl TryFrom<Repr> for IBig {
    type Error = ConversionError;
    #[inline]
    fn try_from(value: Repr) -> Result<Self, Self::Error> {
        if value.denominator.is_one() {
            Ok(value.numerator)
        } else {
            Err(ConversionError::LossOfPrecision)
        }
    }
}

impl Repr {
    /// Convert the value to an integer if it is one, without assuming that the numerator
    /// and the denominator are coprime (they might not be in a [Relaxed]).
    fn into_integer(self) -> Result<IBig, ConversionError> {
        if self.denominator.is_one() {
            return Ok(self.numerator);
        }
        let (q, r) = self.numerator.div_rem(&self.denominator);
        if r.is_zero() {
            Ok(q)
        } else {
            Err(ConversionError::LossOfPrecision)
        }
    }
}

macro_rules! forward_conversion_to_repr {
    ($from:ty => $t:ident) => {
        impl From<$from> for $t {
            #[inline]
            fn from(v: $from) -> Self {
                $t(Repr::from(v))
            }
        }
        impl TryFrom<$t> for $from {
            type Error = ConversionError;
            #[inline]
            fn try_from(value: $t) -> Result<Self, Self::Error> {
                Self::try_from(value.0)
            }
        }
    };
    // The fraction is not necessarily in the lowest terms, so the denominator doesn't
    // tell whether the value is an integer.
    ($from:ty => $t:ident, unreduced) => {
        impl From<$from> for $t {
            #[inline]
            fn from(v: $from) -> Self {
                $t(Repr::from(v))
            }
        }
        impl TryFrom<$t> for $from {
            type Error = ConversionError;
            #[inline]
            fn try_from(value: $t) -> Result<Self, Self::Error> {
                Self::try_from(Repr::from(value.0.into_integer()?))
            }
        }
    };
}
forward_conversion_to_repr!(UBig => RBig);
forward_conversion_to_repr!(IBig => RBig);
forward_conversion_to_repr!(UBig => Relaxed, unreduced);
forward_conversion_to_repr!(IBig => Relaxed, unreduced);

macro_rules! impl_conversion_for_prim_ints {
    ($($t:ty)*) => {$(
        impl From<$t> for Repr {
            #[inline]
            fn from(v: $t) -> Repr {
                Repr {
                    numerator: v.into(),
                    denominator: UBig::ONE
                }
            }
        }

        impl TryFrom<Repr> for $t {
            type Error = ConversionError;
            #[inline]
            fn try_from(value: Repr) -> Result<Self, Self::Error> {
                let int: IBig = value.try_into()?;
                int.try_into()
            }
        }

        forward_conversion_to_repr!($t => RBig);
        forward_conversion_to_repr!($t => Relaxed, unreduced);
    )*};
}
impl_conversion_for_prim_ints!(u8 u16 u32 u64 u128 usize i8 i16 i32 i64 i128 isize);

macro_rules! impl_conversion_from_float {
    ($t:ty) => {
        impl TryFrom<$t> for Repr {
            type Error = ConversionError;

            fn try_from(value: $t) -> Result<Self, Self::Error> {
                // shortcut to prevent issues in counting leading zeros
                if value == 0. {
                    return Ok(Repr::zero());
                }

                match value.decode() {
                    Ok((man, exp)) => {
                        // here we don't remove the common factor 2, because we need exact
                        // exponent value in some cases (like approx_f32 and approx_f64)
                        let repr = if exp >= 0 {
                            Repr {
                                numerator: IBig::from(man) << exp as usize,
                                denominator: UBig::ONE,
                            }
                        } else {
                            let mut denominator = UBig::ZERO;
                            denominator.set_bit((-exp) as _);
                            Repr {
                                numerator: IBig::from(man),
                                denominator,
                            }
                        };
                        Ok(repr)
                    }
                    Err(_) => Err(ConversionError::OutOfBounds),
                }
            }
        }

        impl TryFrom<$t> for RBig {
            type Error = ConversionError;
            #[inline]
            fn try_from(value: $t) -> Result<Self, Self::Error> {
                Repr::try_from(value).map(|repr| RBig(repr.reduce2()))
            }
        }
        impl TryFrom<$t> for Relaxed {
            type Error = ConversionError;
            #[inline]
            fn try_from(value: $t) -> Result<Self, Self::Error> {
                Repr::try_from(value).map(|repr| Relaxed(repr.reduce2()))
            }
        }
    };
}
impl_conversion_from_float!(f32);
impl_conversion_from_float!(f64);

macro_rules! impl_conversion_to_float {
    ($t:ty, $to_float:ident) => {
        impl TryFrom<RBig> for $t {
            type Error = ConversionError;

            /// Convert RBig to primitive floats. It returns [Ok] only if
            /// the conversion can be done losslessly
            #[inline]
            fn try_from(value: RBig) -> Result<Self, Self::Error> {
                <$t>::try_from(value.0)
            }
        }

        impl TryFrom<Relaxed> for $t {
            type Error = ConversionError;

            /// Convert Relaxed to primitive floats. It returns [Ok] only if
            /// the conversion can be done losslessly
            #[inline]
            fn try_from(value: Relaxed) -> Result<Self, Self::Error> {
                <$t>::try_from(value.0)
            }
        }

        impl TryFrom<Repr> for $t {
            type Error = ConversionError;

            fn try_from(value: Repr) -> Result<Self, Self::Error> {
                // the correctly rounded conversion tells whether the value is representable
                match value.$to_float() {
                    Exact(v) => Ok(v),
                    Inexact(v, _) => {
                        if v.is_infinite() {
                            Err(ConversionError::OutOfBounds)
                        } else {
                            Err(ConversionError::LossOfPrecision)
                        }
                    }
                }
            }
        }
    };
}
impl_conversion_to_float!(f32, to_f32);
impl_conversion_to_float!(f64, to_f64);

/// The `n` most significant bits of the nonzero number made of `words` (least significant
/// word first, `bit_len` bits in total). Lower bits are truncated; a number shorter than `n`
/// bits is padded with zeros on the right, so the result is always in `[2^(n-1), 2^n)`.
fn leading_bits(words: &[Word], bit_len: usize, n: u32) -> u128 {
    debug_assert!(bit_len > 0 && n > 0 && n <= u128::BITS);

    let mut acc: u128 = 0;
    let mut have: u32 = 0; // number of bits collected so far
    let mut word_bits = ((bit_len - 1) % Word::BITS as usize) as u32 + 1; // for the top word
    for &word in words.iter().rev() {
        let take = word_bits.min(n - have);
        if take == 0 {
            break;
        }
        acc = (acc << take) | (word >> (word_bits - take)) as u128;
        have += take;
        word_bits = Word::BITS;
    }
    acc << (n - have)
}

/// Compute `floor(log2(|numerator/denominator|))`, given the unsigned
/// numerator magnitude and `exp = numerator.bit_len() - denominator.bit_len()`.
///
/// Since `|numerator/denominator| ∈ [2^(exp-1), 2^(exp+1))`, the result is
/// either `exp` or `exp - 1`; this disambiguates with a single comparison.
fn log2_floor_abs(numerator: &UBig, denominator: &UBig, exp: isize) -> isize {
    let ge_power = if exp >= 0 {
        numerator >= &(denominator << exp as usize)
    } else {
        &(numerator << (-exp) as usize) >= denominator
    };
    if ge_power {
        exp
    } else {
        exp - 1
    }
}

/// Round `|numerator/denominator| * 2^(-shift)` to the nearest integer
/// (ties to even), where `numerator` is the unsigned magnitude and `sign`
/// is the sign of the original rational.
fn rounded_abs_mantissa(
    numerator: UBig,
    denominator: &UBig,
    sign: Sign,
    shift: isize,
) -> Approximation<UBig, Sign> {
    let (num, den) = if shift >= 0 {
        (numerator, denominator << shift as usize)
    } else {
        (numerator << (-shift) as usize, denominator.clone())
    };
    let (man, r) = num.div_rem(&den);

    if r.is_zero() {
        Exact(man)
    } else {
        let half = (r << 1).cmp(&den);
        if half == Ordering::Greater || (half == Ordering::Equal && man.bit(0)) {
            Inexact(man + UBig::ONE, sign)
        } else {
            Inexact(man, -sign)
        }
    }
}

/// Implement the conversion to a primitive float without guaranteed correct rounding.
///
/// The quotient is computed from the `2 * denominator_bits` leading bits of the numerator
/// and the `denominator_bits` leading bits of the denominator by a single primitive division.
/// Truncating the operands perturbs the quotient by a relative error below
/// `2^(2 - denominator_bits)`, which is a small fraction of a unit in the last place of the
/// float, and the quotient is then rounded once. The result is therefore the correctly
/// rounded one unless the value is very close to a rounding boundary, in which case it can
/// be the float on the other side of that boundary (one unit in the last place away).
macro_rules! impl_to_float_fast {
    ($method:ident -> $f:ty, mantissa: $m:ty, denominator_bits: $den_bits:literal) => {
        fn $method(&self) -> $f {
            // shortcut
            if self.numerator.is_zero() {
                return 0.;
            }

            const MANTISSA_BITS: u32 = <$f>::MANTISSA_DIGITS;
            const DEN_BITS: u32 = $den_bits;
            const NUM_BITS: u32 = 2 * DEN_BITS;

            // |value| ~ num / den * 2^exponent
            let (sign, num_words) = self.numerator.as_sign_words();
            let num_len = self.numerator.bit_len();
            let den_len = self.denominator.bit_len();
            let num = leading_bits(num_words, num_len, NUM_BITS);
            let den = leading_bits(self.denominator.as_words(), den_len, DEN_BITS);
            let exponent =
                (num_len as isize - NUM_BITS as isize) - (den_len as isize - DEN_BITS as isize);

            // the quotient has DEN_BITS or DEN_BITS + 1 bits
            let (q, r) = (num / den, num % den);

            // round it to the width of the mantissa (to nearest, ties to even)
            let dropped = (u128::BITS - q.leading_zeros()) - MANTISSA_BITS;
            let mut man = q >> dropped;
            let tail = q & ((1u128 << dropped) - 1);
            let half = 1u128 << (dropped - 1);
            if tail > half || (tail == half && (r != 0 || man & 1 == 1)) {
                man += 1;
            }
            let exponent = exponent + dropped as isize;

            // now |value| ~ man * 2^exponent with 2^(MANTISSA_BITS - 1) <= man <= 2^MANTISSA_BITS
            if exponent > <$f>::MAX_EXP as isize - MANTISSA_BITS as isize {
                // |value| >= 2^MAX_EXP, which is beyond the largest finite float
                sign * <$f>::INFINITY
            } else if exponent < <$f>::MIN_EXP as isize - 2 * MANTISSA_BITS as isize - 1 {
                // |value| < 2^(MIN_EXP - MANTISSA_BITS - 1), half of the smallest subnormal
                sign * (0 as $f)
            } else {
                <$f>::encode(sign * man as $m, exponent as i16).value()
            }
        }
    };
}

impl Repr {
    impl_to_float_fast!(to_f32_fast -> f32, mantissa: i32, denominator_bits: 32);
    impl_to_float_fast!(to_f64_fast -> f64, mantissa: i64, denominator_bits: 64);

    /// Convert the rational number to [f32] with guaranteed correct rounding.
    fn to_f32(&self) -> Approximation<f32, Sign> {
        // shortcut
        if self.numerator.is_zero() {
            return Exact(0.);
        }

        let sign = self.numerator.sign();
        let numerator = (&self.numerator).unsigned_abs();
        // The bit-length difference bounds the binary exponent:
        // `top_exp = floor(log2(|value|))` is either `exp` or `exp - 1`.
        // Fast-path the definite overflow/underflow range from this O(1) bound
        // so the comparison (and its shift) inside `log2_floor_abs` stays bounded.
        let exp = numerator.bit_len() as isize - self.denominator.bit_len() as isize;
        if exp >= 129 {
            // top_exp >= 128, so |value| >= 2^128 > f32::MAX
            return Inexact(sign * f32::INFINITY, sign);
        } else if exp <= -151 {
            // top_exp <= -151 < -150, so |value| < 2^-150, which rounds to zero
            return Inexact(sign * 0f32, -sign);
        }

        let top_exp = log2_floor_abs(&numerator, &self.denominator, exp);
        if top_exp >= 128 {
            // max f32 = 2^128 * (1 - 2^-24)
            Inexact(sign * f32::INFINITY, sign)
        } else if top_exp < -150 {
            // values < 2^-150 round to zero; 2^-150 is the half-way tie, rounded to even (zero)
            Inexact(sign * 0f32, -sign)
        } else {
            // scale |value| by 2^(-shift) into [2^23, 2^24) so the rounded mantissa
            // fits f32's 24-bit significand exactly (no second rounding in encode).
            // Clamp to the subnormal quantization 2^-149 for tiny magnitudes.
            let shift = (top_exp - 23).max(-149);
            rounded_abs_mantissa(numerator, &self.denominator, sign, shift).and_then(|man| {
                let man: u32 = man.try_into().unwrap();
                if man == 0 {
                    // encode(0, _) yields +0; preserve the sign of an underflowed zero
                    Exact(sign * 0f32)
                } else {
                    f32::encode(sign * man as i32, shift as i16)
                }
            })
        }
    }

    fn to_f64(&self) -> Approximation<f64, Sign> {
        // shortcut
        if self.numerator.is_zero() {
            return Exact(0.);
        }

        let sign = self.numerator.sign();
        let numerator = (&self.numerator).unsigned_abs();
        // See `to_f32` for the rationale on fast-pathing via the bit-length bound.
        let exp = numerator.bit_len() as isize - self.denominator.bit_len() as isize;
        if exp >= 1025 {
            // top_exp >= 1024, so |value| >= 2^1024 > f64::MAX
            return Inexact(sign * f64::INFINITY, sign);
        } else if exp <= -1076 {
            // top_exp <= -1076 < -1075, so |value| < 2^-1075, which rounds to zero
            return Inexact(sign * 0f64, -sign);
        }

        let top_exp = log2_floor_abs(&numerator, &self.denominator, exp);
        if top_exp >= 1024 {
            // max f64 = 2^1024 × (1 − 2^−53)
            Inexact(sign * f64::INFINITY, sign)
        } else if top_exp < -1075 {
            // values < 2^-1075 round to zero; 2^-1075 is the half-way tie, rounded to even (zero)
            Inexact(sign * 0f64, -sign)
        } else {
            // scale |value| into [2^52, 2^53) for an exact 53-bit mantissa (no second rounding)
            let shift = (top_exp - 52).max(-1074);
            rounded_abs_mantissa(numerator, &self.denominator, sign, shift).and_then(|man| {
                let man: u64 = man.try_into().unwrap();
                if man == 0 {
                    Exact(sign * 0f64)
                } else {
                    f64::encode(sign * man as i64, shift as i16)
                }
            })
        }
    }
}

impl RBig {
    /// Convert the rational number to a [f32].
    ///
    /// The rounding follows the default IEEE 754 behavior (rounds to nearest,
    /// ties to even).
    ///
    /// The rounding will be correct at most of the time, but in rare cases (when the
    /// number is very close to the midpoint of two adjacent floats) the result can be
    /// the other one of these two floats, that is off by one unit in the last place.
    /// Use [RBig::to_f32] for ensured correct rounding.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_ratio::RBig;
    /// assert_eq!(RBig::ONE.to_f32_fast(), 1f32);
    ///
    /// let r = RBig::from_parts(22.into(), 7u8.into());
    /// assert_eq!(r.to_f32_fast(), 22./7.)
    /// ```
    #[inline]
    pub fn to_f32_fast(&self) -> f32 {
        self.0.to_f32_fast()
    }

    /// Convert the rational number to a [f64].
    ///
    /// The rounding follows the default IEEE 754 behavior (rounds to nearest,
    /// ties to even).
    ///
    /// The rounding will be correct at most of the time, but in rare cases (when the
    /// number is very close to the midpoint of two adjacent floats) the result can be
    /// the other one of these two floats, that is off by one unit in the last place.
    /// Use [RBig::to_f64] for ensured correct rounding.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_ratio::RBig;
    /// assert_eq!(RBig::ONE.to_f64_fast(), 1f64);
    ///
    /// let r = RBig::from_parts(22.into(), 7u8.into());
    /// assert_eq!(r.to_f64_fast(), 22./7.)
    /// ```
    #[inline]
    pub fn to_f64_fast(&self) -> f64 {
        self.0.to_f64_fast()
    }

    /// Convert the rational number to a [f32] with guaranteed correct rounding.
    ///
    /// The rounding follows the default IEEE 754 behavior (rounds to nearest,
    /// ties to even).
    ///
    /// Because of the guaranteed rounding, it might take a long time to convert
    /// when the numerator and denominator are large. In this case [RBig::to_f32_fast]
    /// can be used if the correct rounding is not required.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_base::{Approximation::*, Sign::*};
    /// # use dashu_ratio::RBig;
    /// assert_eq!(RBig::ONE.to_f32(), Exact(1f32));
    ///
    /// let r = RBig::from_parts(22.into(), 7u8.into());
    /// // f32 representation of 22/7 is smaller than the actual 22/7
    /// assert_eq!(r.to_f32(), Inexact(22./7., Negative));
    /// ```
    #[inline]
    pub fn to_f32(&self) -> Approximation<f32, Sign> {
        self.0.to_f32()
    }

    /// Convert the rational number to a [f64] with guaranteed correct rounding.
    ///
    /// The rounding follows the default IEEE 754 behavior (rounds to nearest,
    /// ties to even).
    ///
    /// Because of the guaranteed rounding, it might take a long time to convert
    /// when the numerator and denominator are large. In this case [RBig::to_f64_fast]
    /// can be used if the correct rounding is not required.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_base::{Approximation::*, Sign::*};
    /// # use dashu_ratio::RBig;
    /// assert_eq!(RBig::ONE.to_f64(), Exact(1f64));
    ///
    /// let r = RBig::from_parts(22.into(), 7u8.into());
    /// // f64 representation of 22/7 is smaller than the actual 22/7
    /// assert_eq!(r.to_f64(), Inexact(22./7., Negative));
    /// ```
    #[inline]
    pub fn to_f64(&self) -> Approximation<f64, Sign> {
        self.0.to_f64()
    }

    /// Convert the rational number to an [IBig].
    ///
    /// The conversion rounds toward zero. It's equivalent to [RBig::trunc],
    /// but it returns the fractional part if the rational number is not an integer.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_base::Approximation::*;
    /// # use dashu_int::{IBig, UBig};
    /// # use dashu_ratio::RBig;
    /// let a = RBig::from_parts(22.into(), UBig::ONE);
    /// assert_eq!(a.to_int(), Exact(IBig::from(22)));
    ///
    /// let b = RBig::from_parts(22.into(), 7u8.into());
    /// assert_eq!(b.to_int(), Inexact(
    ///     IBig::from(3), RBig::from_parts(1.into(), 7u8.into())
    /// ));
    /// ```
    #[inline]
    pub fn to_int(&self) -> Approximation<IBig, Self> {
        let (trunc, fract) = self.clone().split_at_point();
        if fract.is_zero() {
            Approximation::Exact(trunc)
        } else {
            Approximation::Inexact(trunc, fract)
        }
    }
}

impl Relaxed {
    /// Convert the rational number to a [f32].
    ///
    /// See [RBig::to_f32_fast] for details.
    #[inline]
    pub fn to_f32_fast(&self) -> f32 {
        self.0.to_f32_fast()
    }
    /// Convert the rational number to a [f64].
    ///
    /// See [RBig::to_f64_fast] for details.
    #[inline]
    pub fn to_f64_fast(&self) -> f64 {
        self.0.to_f64_fast()
    }

    /// Convert the rational number to a [f32] with guaranteed correct rounding.
    ///
    /// See [RBig::to_f32] for details.
    #[inline]
    pub fn to_f32(&self) -> Approximation<f32, Sign> {
        self.0.to_f32()
    }
    /// Convert the rational number to a [f64] with guaranteed correct rounding.
    ///
    /// See [RBig::to_f64] for details.
    #[inline]
    pub fn to_f64(&self) -> Approximation<f64, Sign> {
        self.0.to_f64()
    }
    /// Convert the rational number to am [IBig].
    ///
    /// See [RBig::to_int] for details.
    #[inline]
    pub fn to_int(&self) -> Approximation<IBig, Self> {
        let (trunc, fract) = self.clone().split_at_point();
        if fract.is_zero() {
            Approximation::Exact(trunc)
        } else {
            Approximation::Inexact(trunc, fract)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dashu_base::Sign::{Negative, Positive};

    #[test]
    fn test_ubig_try_from_repr() {
        // Integer values (reduced denominator == 1) convert to their magnitude.
        assert_eq!(UBig::try_from(RBig::from_parts(5.into(), UBig::ONE)), Ok(UBig::from(5u8)));
        assert_eq!(UBig::try_from(RBig::ZERO), Ok(UBig::ZERO));
        // A proper fraction is not an integer — must not silently succeed as the numerator.
        assert_eq!(
            UBig::try_from(RBig::from_parts(1.into(), 2u8.into())),
            Err(ConversionError::LossOfPrecision)
        );
        // A negative integer is out of the unsigned range.
        assert_eq!(
            UBig::try_from(RBig::from_parts((-3).into(), UBig::ONE)),
            Err(ConversionError::OutOfBounds)
        );
    }

    #[test]
    fn test_rbig_to_f64_without_double_rounding() {
        let input =
            RBig::from_parts((-10534148920556696739i128).into(), 73786976294838206464u128.into());

        assert_eq!(input.to_f64(), Inexact(f64::from_bits(0xbfc2_461a_1430_9b17), Negative));
        assert_eq!((-input).to_f64(), Inexact(f64::from_bits(0x3fc2_461a_1430_9b17), Positive));
    }

    #[test]
    fn test_rbig_to_float_midpoints_tie_to_even() {
        assert_eq!(
            RBig::from_parts(((1u64 << 24) + 1).into(), (1u64 << 24).into()).to_f32(),
            Inexact(1.0, Negative)
        );
        assert_eq!(
            RBig::from_parts(((1u64 << 24) + 3).into(), (1u64 << 24).into()).to_f32(),
            Inexact(f32::from_bits(0x3f80_0002), Positive)
        );

        assert_eq!(
            RBig::from_parts(((1u64 << 53) + 1).into(), (1u64 << 53).into()).to_f64(),
            Inexact(1.0, Negative)
        );
        assert_eq!(
            RBig::from_parts(((1u64 << 53) + 3).into(), (1u64 << 53).into()).to_f64(),
            Inexact(f64::from_bits(0x3ff0_0000_0000_0002), Positive)
        );
    }

    #[test]
    fn test_rbig_to_float_around_midpoints() {
        assert_eq!(
            RBig::from_parts(((1u64 << 25) + 1).into(), (1u64 << 25).into()).to_f32(),
            Inexact(1.0, Negative)
        );
        assert_eq!(
            RBig::from_parts(((1u64 << 25) + 3).into(), (1u64 << 25).into()).to_f32(),
            Inexact(f32::from_bits(0x3f80_0001), Positive)
        );

        assert_eq!(
            RBig::from_parts(((1u128 << 54) + 1).into(), (1u128 << 54).into()).to_f64(),
            Inexact(1.0, Negative)
        );
        assert_eq!(
            RBig::from_parts(((1u128 << 54) + 3).into(), (1u128 << 54).into()).to_f64(),
            Inexact(f64::from_bits(0x3ff0_0000_0000_0001), Positive)
        );
    }

    #[test]
    fn test_rbig_to_float_subnormal_rounding_boundaries() {
        assert_eq!(RBig::from_parts(1.into(), UBig::ONE << 150).to_f32(), Inexact(0.0, Negative));
        assert_eq!(
            RBig::from_parts((-1).into(), UBig::ONE << 150).to_f32(),
            Inexact(-0.0, Positive)
        );
        assert_eq!(
            RBig::from_parts(3.into(), UBig::ONE << 151).to_f32(),
            Inexact(f32::from_bits(1), Positive)
        );
        assert_eq!(
            RBig::from_parts(3.into(), UBig::ONE << 150).to_f32(),
            Inexact(f32::from_bits(2), Positive)
        );

        assert_eq!(RBig::from_parts(1.into(), UBig::ONE << 1075).to_f64(), Inexact(0.0, Negative));
        assert_eq!(
            RBig::from_parts((-1).into(), UBig::ONE << 1075).to_f64(),
            Inexact(-0.0, Positive)
        );
        assert_eq!(
            RBig::from_parts(3.into(), UBig::ONE << 1076).to_f64(),
            Inexact(f64::from_bits(1), Positive)
        );
        assert_eq!(
            RBig::from_parts(3.into(), UBig::ONE << 1075).to_f64(),
            Inexact(f64::from_bits(2), Positive)
        );
    }
}
