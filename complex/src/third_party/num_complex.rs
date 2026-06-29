//! Conversions between [`CBig`] and `num-complex`'s `Complex<f32>`/`Complex<f64>` (behind the
//! `num-complex` feature).
//!
//! Mirroring `dashu-float`'s primitive-float conversions, both directions are restricted to base 2
//! and compose through [`FBig`]: a `Complex<f64>` splits into two `f64`s, each lifted to an exact
//! base-2 [`FBig`] (NaN → [`ConversionError::OutOfBounds`]; infinities and signed zeros preserved);
//! a base-2 [`CBig`] rounds each part back to `f32`/`f64`, erroring on overflow or inexactness. The
//! pair is therefore the exact-into / rounding-out-of split that [`FBig`] uses for `f64`.

use crate::cbig::CBig;
use dashu_base::ConversionError;
use dashu_float::round::Round;
use dashu_float::FBig;
use num_complex_v04::{Complex32, Complex64};

macro_rules! impl_complex_conversions {
    ($cx:ty, $f:ty) => {
        impl<R: Round> TryFrom<$cx> for CBig<R, 2> {
            type Error = ConversionError;

            /// Lift a primitive-float `Complex` to an exact base-2 [`CBig`]. A `NaN` part is
            /// unmappable (`CBig` has no NaN) and yields [`ConversionError::OutOfBounds`];
            /// infinities and signed zeros are preserved, exactly as for `FBig: TryFrom<f64>`.
            #[inline]
            fn try_from(c: $cx) -> Result<Self, Self::Error> {
                let re = FBig::try_from(c.re)?;
                let im = FBig::try_from(c.im)?;
                Ok(CBig::from_parts(re, im))
            }
        }

        impl<R: Round> TryFrom<CBig<R, 2>> for $cx {
            type Error = ConversionError;

            /// Round a base-2 [`CBig`] back to a primitive-float `Complex`, composing
            /// [`CBig`] → [`FBig`] → `f32`/`f64` per part. Errors on overflow or inexactness, and
            /// also when a part is already infinite — mirroring `FBig: TryFrom<FBig> for f64`.
            #[inline]
            fn try_from(z: CBig<R, 2>) -> Result<Self, Self::Error> {
                let fctx = z.context.float();
                let re = <$f>::try_from(FBig::from_repr(z.re, fctx))?;
                let im = <$f>::try_from(FBig::from_repr(z.im, fctx))?;
                Ok(<$cx>::new(re, im))
            }
        }
    };
}

impl_complex_conversions!(Complex32, f32);
impl_complex_conversions!(Complex64, f64);

#[cfg(test)]
mod tests {
    use super::*;
    use dashu_float::round::mode;

    type C2 = CBig<mode::Zero, 2>;

    #[test]
    fn f64_roundtrip() {
        for (re, im) in [
            (3.0_f64, 4.0),
            (1.0, 0.0),
            (0.0, 1.0),
            (-2.0, 0.5),
            (0.0, 0.0),
            (1.5, -2.25),
        ] {
            let c = Complex64::new(re, im);
            let z = C2::try_from(c).unwrap();
            let back: Complex64 = z.try_into().unwrap();
            assert_eq!(back, c, "roundtrip failed for {re}+{im}i");
        }
    }

    #[test]
    fn f32_roundtrip() {
        for (re, im) in [(3.0_f32, 4.0), (-1.5_f32, 0.25)] {
            let c = Complex32::new(re, im);
            let z = C2::try_from(c).unwrap();
            let back: Complex32 = z.try_into().unwrap();
            assert_eq!(back, c);
        }
    }

    #[test]
    fn nan_is_out_of_bounds() {
        assert_eq!(C2::try_from(Complex64::new(f64::NAN, 0.0)), Err(ConversionError::OutOfBounds));
        assert_eq!(C2::try_from(Complex64::new(0.0, f64::NAN)), Err(ConversionError::OutOfBounds));
    }

    #[test]
    fn infinities_preserved_on_lift() {
        let z = C2::try_from(Complex64::new(f64::INFINITY, f64::NEG_INFINITY)).unwrap();
        assert!(z.re().is_infinite());
        assert!(z.im().is_infinite());
        // an infinite part can't round-trip back to f64 (mirrors FBig)
        assert_eq!(Complex64::try_from(z), Err(ConversionError::LossOfPrecision));
    }

    #[test]
    fn signed_zero_preserved() {
        let z = C2::try_from(Complex64::new(-0.0, 0.0)).unwrap();
        assert!(z.re().is_neg_zero());
        assert!(z.im().is_zero());
    }

    #[test]
    fn high_precision_rounds_inexactly() {
        // 2^53 + 1 needs 54 mantissa bits, so it can't convert back to f64 exactly
        let big = FBig::<mode::Zero, 2>::from_parts(((1u64 << 53) + 1).into(), 0);
        let z = C2::from_parts(big, FBig::from(0));
        assert_eq!(Complex64::try_from(z), Err(ConversionError::LossOfPrecision));
    }
}
