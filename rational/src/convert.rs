use core::cmp::Ordering;

use dashu_base::{Approximation::{*, self}, Sign, FloatEncoding, UnsignedAbs, DivRem};
use dashu_int::{error::OutOfBoundsError, IBig, UBig};

use crate::{rbig::{RBig, Relaxed}, repr::Repr};

impl From<UBig> for RBig {
    #[inline]
    fn from(v: UBig) -> Self {
        RBig::from_parts(v.into(), UBig::ONE)
    }
}

impl From<IBig> for RBig {
    #[inline]
    fn from(v: IBig) -> Self {
        RBig::from_parts(v, UBig::ONE)
    }
}

impl TryFrom<RBig> for IBig {
    type Error = OutOfBoundsError; // TODO(v0.3): change to PrecisionLossError
    #[inline]
    fn try_from(value: RBig) -> Result<Self, Self::Error> {
        if value.0.denominator.is_one() {
            Ok(value.0.numerator)
        } else {
            Err(OutOfBoundsError)
        }
    }
}

impl From<u8> for RBig {
    #[inline]
    fn from(v: u8) -> RBig {
        RBig::from_parts(v.into(), UBig::ONE)
    }
}

impl TryFrom<RBig> for u8 {
    type Error = OutOfBoundsError;
    #[inline]
    fn try_from(value: RBig) -> Result<Self, Self::Error> {
        let int: IBig = value.try_into()?;
        int.try_into()
    }
}

macro_rules! impl_from_float_for_repr {
    ($t:ty) => {
        impl TryFrom<$t> for Repr {
            type Error = OutOfBoundsError;

            fn try_from(value: $t) -> Result<Self, Self::Error> {
                // shortcut to prevent issues in counting leading zeros
                if value == 0. {
                    return Ok(Repr::zero())
                }

                match value.decode() {
                    Ok((mut man, mut exp)) => {
                        let shift = man.trailing_zeros();
                        man >>= shift;
                        exp += shift as i16;

                        let repr = if exp >= 0 {
                            Repr { numerator: IBig::from(man) << exp as usize, denominator: UBig::ONE }
                        } else {
                            let mut denominator = UBig::ZERO;
                            denominator.set_bit((-exp) as _);
                            Repr { numerator: IBig::from(man), denominator }
                        };
                        Ok(repr)
                    },
                    Err(_) => Err(OutOfBoundsError)
                }
            }
        }
 
        impl TryFrom<$t> for RBig {
            type Error = OutOfBoundsError;
            #[inline]
            fn try_from(value: $t) -> Result<Self, Self::Error> {
                Repr::try_from(value).map(|repr| RBig(repr))
            }
        }
        impl TryFrom<$t> for Relaxed {
            type Error = OutOfBoundsError;
            #[inline]
            fn try_from(value: $t) -> Result<Self, Self::Error> {
                Repr::try_from(value).map(|repr| Relaxed(repr))
            }
        }
    };
}
impl_from_float_for_repr!(f32);
impl_from_float_for_repr!(f64);

impl Repr {
    /// Convert the rational number to [f32] without guaranteed correct rounding.
    fn to_f32_fast(&self) -> f32 {
        // shortcut
        if self.numerator.is_zero() {
            return 0.;
        }

        // to get enough precision (24 bits), we need to do a 48 by 24 bit division
        let sign = self.numerator.sign();
        let num_bits = self.numerator.abs_bit_len();
        let den_bits = self.denominator.bit_len();

        let num_shift = num_bits as isize - 48;
        let num48: i64 = if num_shift >= 0 {
            (&self.numerator) >> num_shift as usize
        } else {
            (&self.numerator) << (-num_shift) as usize
        }.try_into().unwrap();

        let den_shift = den_bits as isize - 24;
        let den24: u32 = if den_shift >= 0 {
            (&self.denominator) >> den_shift as usize
        } else {
            (&self.denominator) << (-den_shift) as usize
        }.try_into().unwrap();

        // determine the exponent
        let exponent = num_shift - den_shift;
        if exponent >= 128 { // max f32 = 2^128 * (1 - 2^-24)
            sign * f32::INFINITY
        }  else if exponent < -149 - 25 { // min f32 = 2^-149, quotient has at most 25 bits
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
        let num_bits = self.numerator.abs_bit_len();
        let den_bits = self.denominator.bit_len();

        let num_shift = num_bits as isize - 106;
        let num106: i128 = if num_shift >= 0 {
            (&self.numerator) >> num_shift as usize
        } else {
            (&self.numerator) << (-num_shift) as usize
        }.try_into().unwrap();

        let den_shift = den_bits as isize - 53;
        let den53: u64 = if den_shift >= 0 {
            (&self.denominator) >> den_shift as usize
        } else {
            (&self.denominator) << (-den_shift) as usize
        }.try_into().unwrap();

        // determine the exponent
        let exponent = num_shift - den_shift;
        if exponent >= 1024 { // max f64 = 2^1024 × (1 − 2^−53)
            sign * f64::INFINITY
        }  else if exponent < -1074 - 54 { // min f64 = 2^-1074, quotient has at most 54 bits
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
        let num_bits = self.numerator.abs_bit_len();
        let den_bits = self.denominator.bit_len();

        let shift = num_bits as isize - den_bits as isize - 24; // i.e. exponent
        let (num, den) = if shift >= 0 {
            (self.numerator.clone(), (&self.denominator) << shift as usize)
        } else {
            ((&self.numerator) << (-shift) as usize, self.denominator.clone())
        };

        // then construct the 
        if shift >= 128 { // max f32 = 2^128 * (1 - 2^-24)
            Inexact(sign * f32::INFINITY, sign)
        } else if shift < -149 - 25 { // min f32 = 2^-149, quotient has at most 25 bits
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
            }.and_then(|man| f32::encode(sign * man as i32, shift as i16))
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
        let num_bits = self.numerator.abs_bit_len();
        let den_bits = self.denominator.bit_len();

        let shift = num_bits as isize - den_bits as isize - 53; // i.e. exponent
        let (num, den) = if shift >= 0 {
            (self.numerator.clone(), (&self.denominator) << shift as usize)
        } else {
            ((&self.numerator) << (-shift) as usize, self.denominator.clone())
        };

        // then construct the 
        if shift >= 1024 { // max f64 = 2^1024 × (1 − 2^−53)
            Inexact(sign * f64::INFINITY, sign)
        } else if shift < -1074 - 53 { // min f64 = 2^-1074, quotient has at most 53 bits
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
            }.and_then(|man| f64::encode(sign * man as i64, shift as i16))
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

    #[inline]
    pub fn to_f32(&self) -> Approximation<f32, Sign> {
        self.0.to_f32()
    }
    #[inline]
    pub fn to_f64(&self) -> Approximation<f64, Sign> {
        self.0.to_f64()
    }
}
