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
        if z.im.is_zero() || z.im.is_neg_zero() {
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

#[cfg(test)]
mod tests {
    use super::*;
    use dashu_float::round::mode;

    type C = CBig<mode::HalfAway, 10>;
    type F = FBig<mode::HalfAway, 10>;

    #[test]
    fn from_fbig_is_purely_real() {
        let z = C::from(F::from(7));
        assert!(z.im().is_zero() || z.im().is_neg_zero());
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
}
