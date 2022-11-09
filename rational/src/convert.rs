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
        let (sign, mag) = value.numerator.into_parts();
        if sign == Sign::Negative {
            Err(ConversionError::OutOfBounds)
        } else if mag.is_one() {
            Ok(mag)
        } else {
            Err(ConversionError::LossOfPrecision)
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

        // to get enough precision, shift such that numerator has
        // 24 bits more than the denominator
        let sign = self.numerator.sign();
        let num_bits = self.numerator.bit_len();
        let den_bits = self.denominator.bit_len();

        let shift = num_bits as isize - den_bits as isize - 24; // i.e. exponent
        let (num, den) = if shift >= 0 {
            (self.numerator.clone(), (&self.denominator) << shift as usize)
        } else {
            ((&self.numerator) << (-shift) as usize, self.denominator.clone())
        };

        // then construct the
        if shift >= 128 {
            // max f32 = 2^128 * (1 - 2^-24)
            Inexact(sign * f32::INFINITY, sign)
        } else if shift < -149 - 25 {
            // min f32 = 2^-149, quotient has at most 25 bits
            Inexact(sign * 0f32, -sign)
        } else {
            let (man, r) = num.unsigned_abs().div_rem(&den);
            let man: u32 = man.try_into().unwrap();

            // round to nearest, ties to even
            if r.is_zero() {
                Exact(man)
            } else {
                let half = (r << 1).cmp(&den);
                if half == Ordering::Greater || (half == Ordering::Equal && man & 1 > 0) {
                    Inexact(man + 1, sign)
                } else {
                    Inexact(man, -sign)
                }
            }
            .and_then(|man| f32::encode(sign * man as i32, shift as i16))
        }
    }

    fn to_f64(&self) -> Approximation<f64, Sign> {
        // shortcut
        if self.numerator.is_zero() {
            return Exact(0.);
        }

        // to get enough precision, shift such that numerator has
        // 53 bits more than the denominator
        let sign = self.numerator.sign();
        let num_bits = self.numerator.bit_len();
        let den_bits = self.denominator.bit_len();

        let shift = num_bits as isize - den_bits as isize - 53; // i.e. exponent
        let (num, den) = if shift >= 0 {
            (self.numerator.clone(), (&self.denominator) << shift as usize)
        } else {
            ((&self.numerator) << (-shift) as usize, self.denominator.clone())
        };

        // then construct the
        if shift >= 1024 {
            // max f64 = 2^1024 × (1 − 2^−53)
            Inexact(sign * f64::INFINITY, sign)
        } else if shift < -1074 - 53 {
            // min f64 = 2^-1074, quotient has at most 53 bits
            Inexact(sign * 0f64, -sign)
        } else {
            let (man, r) = num.unsigned_abs().div_rem(&den);
            let man: u64 = man.try_into().unwrap();

            // round to nearest, ties to even
            if r.is_zero() {
                Exact(man)
            } else {
                let half = (r << 1).cmp(&den);
                if half == Ordering::Greater || (half == Ordering::Equal && man & 1 > 0) {
                    Inexact(man + 1, sign)
                } else {
                    Inexact(man, -sign)
                }
            }
            .and_then(|man| f64::encode(sign * man as i64, shift as i16))
        }
    }
}

impl RBig {
    /// Convert the rational number to [f32].
    ///
    /// The rounding will be correct at most of the time, but in rare cases
    /// the mantissa can be off by one bit.
    #[inline]
    pub fn to_f32_fast(&self) -> f32 {
        self.0.to_f32_fast()
    }
    #[inline]
    pub fn to_f64_fast(&self) -> f64 {
        self.0.to_f64_fast()
    }

    /// Convert the rational number to [f32] with guaranteed correct rounding.
    #[inline]
    pub fn to_f32(&self) -> Approximation<f32, Sign> {
        self.0.to_f32()
    }
    #[inline]
    pub fn to_f64(&self) -> Approximation<f64, Sign> {
        self.0.to_f64()
    }
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
    /// Convert the rational number to [f32].
    ///
    /// See [RBig::to_f32_fast] for details.
    #[inline]
    pub fn to_f32_fast(&self) -> f32 {
        self.0.to_f32_fast()
    }
    /// Convert the rational number to [f64].
    ///
    /// See [RBig::to_f64_fast] for details.
    #[inline]
    pub fn to_f64_fast(&self) -> f64 {
        self.0.to_f64_fast()
    }

    /// Convert the rational number to [f32] with guaranteed correct rounding.
    ///
    /// See [RBig::to_f32] for details.
    #[inline]
    pub fn to_f32(&self) -> Approximation<f32, Sign> {
        self.0.to_f32()
    }
    /// Convert the rational number to [f64] with guaranteed correct rounding.
    ///
    /// See [RBig::to_f64] for details.
    #[inline]
    pub fn to_f64(&self) -> Approximation<f64, Sign> {
        self.0.to_f64()
    }
    /// Convert the rational number to [IBig].
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
