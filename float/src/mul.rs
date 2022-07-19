use core::ops::Mul;
use crate::{repr::FloatRepr, utils::{shr_rem_radix_in_place, round_with_fract, get_precision}};

impl<const X: usize, const R: u8> Mul for &FloatRepr<X, R> {
    type Output = FloatRepr<X, R>;

    #[inline]
    fn mul(self, rhs: Self) -> Self::Output {
        let precision = self.precision.max(rhs.precision);
        let exponent = self.exponent + rhs.exponent;
        let mut mantissa = &self.mantissa * &rhs.mantissa;
        let actual_prec = get_precision::<X>(&mantissa);
        if actual_prec > precision {
            let shift = actual_prec - precision;
            let low_digits = shr_rem_radix_in_place::<X>(&mut mantissa, shift);
            mantissa += round_with_fract::<X, R>(&mantissa, low_digits, shift);
            let (mantissa, exponent) = Self::Output::normalize(mantissa, exponent);
            FloatRepr { mantissa, exponent, precision }
        } else {
            FloatRepr { mantissa, exponent, precision }
        }
    }
}

impl<const X: usize, const R: u8> Mul for FloatRepr<X, R> {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: Self) -> Self::Output {
        (&self).mul(&rhs)
    }
}
impl<const X: usize, const R: u8> Mul<FloatRepr<X, R>> for &FloatRepr<X, R> {
    type Output = FloatRepr<X, R>;
    #[inline]
    fn mul(self, rhs: FloatRepr<X, R>) -> Self::Output {
        self.mul(&rhs)
    }
}
