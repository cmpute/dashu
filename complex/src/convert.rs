//! Conversions between [`CBig`], [`FBig`], and integers.
//!
//! The into-`CBig` direction is lossless through [`From`] (a real [`FBig`], or a `UBig`/`IBig` →
//! the real part, with imaginary `+0`). The out-of-`CBig` direction is lossy through [`TryFrom`],
//! composing `CBig → FBig → IBig` — exactly the [`From`]/[`TryFrom`] split `FBig` uses.

use crate::cbig::CBig;
use dashu_base::ConversionError;
use dashu_float::round::Round;
use dashu_float::{FBig, Repr};
use dashu_int::{IBig, UBig, Word};

impl<R: Round, const B: Word> From<FBig<R, B>> for CBig<R, B> {
    /// Embed a real [`FBig`] as a complex number with imaginary part `+0`.
    #[inline]
    fn from(re: FBig<R, B>) -> Self {
        let fctx = re.context();
        Self {
            re: re.into_repr(),
            im: Repr::zero(),
            context: crate::repr::Context(fctx),
        }
    }
}

impl<R: Round, const B: Word> From<UBig> for CBig<R, B> {
    /// Embed an unsigned integer as a complex number (exact, unlimited precision) with imaginary `+0`.
    #[inline]
    fn from(v: UBig) -> Self {
        FBig::from(v).into()
    }
}

impl<R: Round, const B: Word> From<IBig> for CBig<R, B> {
    /// Embed a signed integer as a complex number (exact, unlimited precision) with imaginary `+0`.
    #[inline]
    fn from(v: IBig) -> Self {
        FBig::from(v).into()
    }
}

impl<R: Round, const B: Word> TryFrom<CBig<R, B>> for FBig<R, B> {
    type Error = ConversionError;

    /// Extract the real part, succeeding only when the imaginary part is zero (purely real; both
    /// `±0` count as zero). This is the guarded "is this complex actually real?" check — distinct
    /// from [`CBig::re`] / [`CBig::into_parts`], which return the real part unconditionally.
    #[inline]
    fn try_from(z: CBig<R, B>) -> Result<Self, Self::Error> {
        if z.im.is_pos_zero() || z.im.is_neg_zero() {
            Ok(FBig::from_repr(z.re, z.context.float()))
        } else {
            Err(ConversionError::LossOfPrecision)
        }
    }
}

impl<R: Round, const B: Word> TryFrom<CBig<R, B>> for IBig {
    type Error = ConversionError;

    /// Extract an integer, succeeding only when the number is purely real, finite, and
    /// integer-valued. Composes [`CBig`] → [`FBig`] → [`IBig`]; for a rounding-aware path use
    /// [`FBig::to_int`] on the real part ([`CBig::re`]).
    #[inline]
    fn try_from(z: CBig<R, B>) -> Result<Self, Self::Error> {
        let re: FBig<R, B> = FBig::try_from(z)?;
        IBig::try_from(re)
    }
}

impl<R: Round, const B: Word> TryFrom<CBig<R, B>> for UBig {
    type Error = ConversionError;

    /// Extract an unsigned integer, succeeding only when the number is purely real, finite,
    /// integer-valued, and non-negative. Composes [`CBig`] → [`FBig`] → [`UBig`].
    #[inline]
    fn try_from(z: CBig<R, B>) -> Result<Self, Self::Error> {
        let re: FBig<R, B> = FBig::try_from(z)?;
        UBig::try_from(re)
    }
}

// Conversions between `CBig` and the integer primitives (both directions), composing through
// `FBig`. Integers embed as purely-real complex numbers; extraction succeeds only when the value
// is purely real, finite, in range, and integer-valued.
macro_rules! impl_cbig_int_conv {
    ($($t:ty)*) => {$(
        impl<R: Round, const B: Word> From<$t> for CBig<R, B> {
            #[inline]
            fn from(v: $t) -> Self {
                FBig::from(v).into()
            }
        }

        impl<R: Round, const B: Word> TryFrom<CBig<R, B>> for $t {
            type Error = ConversionError;

            #[inline]
            fn try_from(z: CBig<R, B>) -> Result<Self, Self::Error> {
                let re: FBig<R, B> = FBig::try_from(z)?;
                re.try_into()
            }
        }
    )*};
}
impl_cbig_int_conv!(u8 u16 u32 u64 u128 usize i8 i16 i32 i64 i128 isize);

// Conversions between `CBig` and `f32`/`f64` (both directions, base 2 only). NaN is rejected on
// input; infinities are preserved. Extraction succeeds only when purely real and exactly
// representable.
macro_rules! impl_cbig_float_conv {
    ($($t:ty)*) => {$(
        impl<R: Round> TryFrom<$t> for CBig<R, 2> {
            type Error = ConversionError;

            #[inline]
            fn try_from(f: $t) -> Result<Self, Self::Error> {
                Ok(CBig::from(FBig::try_from(f)?))
            }
        }

        impl<R: Round> TryFrom<CBig<R, 2>> for $t {
            type Error = ConversionError;

            #[inline]
            fn try_from(z: CBig<R, 2>) -> Result<Self, Self::Error> {
                let re: FBig<R, 2> = FBig::try_from(z)?;
                re.try_into()
            }
        }
    )*};
}
impl_cbig_float_conv!(f32 f64);

#[cfg(test)]
mod tests {
    use super::*;
    use dashu_float::round::mode;

    type C = CBig<mode::HalfAway, 10>;
    type F = FBig<mode::HalfAway, 10>;

    #[test]
    fn from_fbig_is_purely_real() {
        let z = C::from(F::from(7));
        assert!(z.im().is_pos_zero() || z.im().is_neg_zero());
        assert_eq!(z.re().significand(), &7.into());
    }

    #[test]
    fn from_integers() {
        let z: C = UBig::from(5u32).into();
        assert_eq!(z.re().significand(), &5.into());
        let z: C = IBig::from(-3).into();
        assert_eq!(z.re().significand(), &(-3i32).into());
    }

    #[test]
    fn try_from_fbig_ok_iff_purely_real() {
        let z = C::from_parts(7.into(), 0.into());
        let re: F = F::try_from(z).unwrap();
        assert_eq!(re.repr().significand(), &7.into());

        let z = C::from_parts(3.into(), 4.into());
        assert_eq!(F::try_from(z), Err(ConversionError::LossOfPrecision));
    }

    #[test]
    fn try_from_ibig_composes() {
        let z: C = IBig::from(9).into();
        let i: IBig = IBig::try_from(z).unwrap();
        assert_eq!(i, 9.into());

        // fractional real part → LossOfPrecision
        let z = C::from(F::from_parts(123.into(), -2)); // 1.23
        assert_eq!(IBig::try_from(z), Err(ConversionError::LossOfPrecision));

        // nonzero imaginary → LossOfPrecision
        let z = C::from_parts(9.into(), 1.into());
        assert_eq!(IBig::try_from(z), Err(ConversionError::LossOfPrecision));
    }

    #[test]
    fn try_from_ubig_composes() {
        let z: C = IBig::from(9).into();
        let u: UBig = UBig::try_from(z).unwrap();
        assert_eq!(u, UBig::from(9u8));

        // negative real part → OutOfBounds
        let z: C = IBig::from(-9).into();
        assert_eq!(UBig::try_from(z), Err(ConversionError::OutOfBounds));

        // fractional real part → LossOfPrecision
        let z = C::from(F::from_parts(123.into(), -2)); // 1.23
        assert_eq!(UBig::try_from(z), Err(ConversionError::LossOfPrecision));

        // nonzero imaginary → LossOfPrecision
        let z = C::from_parts(9.into(), 1.into());
        assert_eq!(UBig::try_from(z), Err(ConversionError::LossOfPrecision));
    }

    #[test]
    fn primitive_conversions() {
        // integers embed as purely-real complex numbers (any base)
        let z: C = 7u8.into();
        assert_eq!(z.re().significand(), &7.into());
        let z: C = (-3i8).into();
        assert_eq!(z.re().significand(), &(-3i32).into());

        // floats embed into a base-2 CBig; NaN is rejected
        let z = CBig::<mode::HalfAway, 2>::try_from(2.5f64).unwrap();
        assert_eq!(z.re().significand(), &5.into()); // 2.5 = 5 * 2^-1
        assert!(CBig::<mode::HalfAway, 2>::try_from(f32::NAN).is_err());

        // CBig -> integer primitive (fails on negative / out-of-range / fractional / nonzero-imag)
        assert_eq!(u8::try_from(C::from(9u8)), Ok(9u8));
        assert_eq!(u8::try_from(C::from(-9i8)), Err(ConversionError::OutOfBounds));
        assert_eq!(u8::try_from(C::from(300u16)), Err(ConversionError::OutOfBounds));
        assert_eq!(i8::try_from(C::from(-9i8)), Ok(-9i8));
        assert_eq!(
            i8::try_from(C::from(F::from_parts(123.into(), -2))), // 1.23
            Err(ConversionError::LossOfPrecision)
        );

        // CBig -> float primitive (base-2 only)
        let z = CBig::<mode::HalfAway, 2>::try_from(2.5f64).unwrap();
        assert_eq!(f64::try_from(z), Ok(2.5));
    }
}
