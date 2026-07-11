use core::cmp::Ordering;

use dashu_base::{
    Approximation::{self, *},
    BitTest, ConversionError, DivRem, FloatEncoding, PowerOfTwo, Sign, UnsignedAbs,
};
use dashu_int::{IBig, UBig};

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
}
forward_conversion_to_repr!(UBig => RBig);
forward_conversion_to_repr!(IBig => RBig);
forward_conversion_to_repr!(UBig => Relaxed);
forward_conversion_to_repr!(IBig => Relaxed);

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
        forward_conversion_to_repr!($t => Relaxed);
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
    ($t:ty [$lb:literal, $ub:literal]) => {
        impl TryFrom<RBig> for $t {
            type Error = ConversionError;

            /// Convert RBig to primitive floats. It returns [Ok] only if
            /// the conversion can be done losslessly
            fn try_from(value: RBig) -> Result<Self, Self::Error> {
                if value.0.numerator.is_zero() {
                    Ok(0.)
                } else if value.0.denominator.is_power_of_two() {
                    // conversion is exact only if the denominator is a power of two
                    let num_bits = value.0.numerator.bit_len();
                    let den_bits = value.0.denominator.trailing_zeros().unwrap();
                    let top_bit = num_bits as isize - den_bits as isize;
                    if top_bit > $ub {
                        // see to_f32::encode for explanation of the bounds
                        Err(ConversionError::OutOfBounds)
                    } else if top_bit < $lb {
                        Err(ConversionError::LossOfPrecision)
                    } else {
                        match <$t>::encode(
                            value.0.numerator.try_into().unwrap(),
                            -(den_bits as i16),
                        ) {
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
                } else {
                    Err(ConversionError::LossOfPrecision)
                }
            }
        }

        impl TryFrom<Relaxed> for $t {
            type Error = ConversionError;

            #[inline]
            fn try_from(value: Relaxed) -> Result<Self, Self::Error> {
                // convert to RBig to eliminate cofactors
                <$t>::try_from(value.canonicalize())
            }
        }
    };
}
impl_conversion_to_float!(f32 [-149, 128]); // see f32::encode for explanation of the bounds
impl_conversion_to_float!(f64 [-1074, 1024]); // see f32::encode for explanation of the bounds

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

impl Repr {
    /// Convert the rational number to [f32] without guaranteed correct rounding.
    fn to_f32_fast(&self) -> f32 {
        // shortcut
        if self.numerator.is_zero() {
            return 0.;
        }

        // to get enough precision (24 bits), we need to do a 48 by 24 bit division
        let sign = self.numerator.sign();
        let num_bits = self.numerator.bit_len();
        let den_bits = self.denominator.bit_len();

        let num_shift = num_bits as isize - 48;
        let num48: i64 = if num_shift >= 0 {
            (&self.numerator) >> num_shift as usize
        } else {
            (&self.numerator) << (-num_shift) as usize
        }
        .try_into()
        .unwrap();

        let den_shift = den_bits as isize - 24;
        let den24: u32 = if den_shift >= 0 {
            (&self.denominator) >> den_shift as usize
        } else {
            (&self.denominator) << (-den_shift) as usize
        }
        .try_into()
        .unwrap();

        // determine the exponent
        let exponent = num_shift - den_shift;
        if exponent >= 128 {
            // max f32 = 2^128 * (1 - 2^-24)
            sign * f32::INFINITY
        } else if exponent < -149 - 25 {
            // min f32 = 2^-149, quotient has at most 25 bits
            sign * 0f32
        } else {
            let (mut man, r) = num48.unsigned_abs().div_rem(den24 as u64);

            // round to nearest, ties to even
            let half = (r as u32 * 2).cmp(&den24);
            if half == Ordering::Greater || (half == Ordering::Equal && man & 1 > 0) {
                man += 1;
            }
            f32::encode(sign * man as i32, exponent as i16).value()
        }
    }

    fn to_f64_fast(&self) -> f64 {
        // shortcut
        if self.numerator.is_zero() {
            return 0.;
        }

        // to get enough precision (53 bits), we need to do a 106 by 53 bit division
        let sign = self.numerator.sign();
        let num_bits = self.numerator.bit_len();
        let den_bits = self.denominator.bit_len();

        let num_shift = num_bits as isize - 106;
        let num106: i128 = if num_shift >= 0 {
            (&self.numerator) >> num_shift as usize
        } else {
            (&self.numerator) << (-num_shift) as usize
        }
        .try_into()
        .unwrap();

        let den_shift = den_bits as isize - 53;
        let den53: u64 = if den_shift >= 0 {
            (&self.denominator) >> den_shift as usize
        } else {
            (&self.denominator) << (-den_shift) as usize
        }
        .try_into()
        .unwrap();

        // determine the exponent
        let exponent = num_shift - den_shift;
        if exponent >= 1024 {
            // max f64 = 2^1024 × (1 − 2^−53)
            sign * f64::INFINITY
        } else if exponent < -1074 - 54 {
            // min f64 = 2^-1074, quotient has at most 54 bits
            sign * 0f64
        } else {
            let (mut man, r) = num106.unsigned_abs().div_rem(den53 as u128);

            // round to nearest, ties to even
            let half = (r as u64 * 2).cmp(&den53);
            if half == Ordering::Greater || (half == Ordering::Equal && man & 1 > 0) {
                man += 1;
            }
            f64::encode(sign * man as i64, exponent as i16).value()
        }
    }

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
    /// The rounding will be correct at most of the time, but in rare cases the
    /// mantissa can be off by one bit. Use [RBig::to_f32] for ensured correct
    /// rounding.
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
    /// The rounding will be correct at most of the time, but in rare cases the
    /// mantissa can be off by one bit. Use [RBig::to_f64] for ensured correct
    /// rounding.
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
