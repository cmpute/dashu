use core::ops::Mul;
use crate::{repr::FloatRepr, utils::mul_hi};

impl<const X: usize, const R: u8> Mul for &FloatRepr<X, R> {
    type Output = FloatRepr<X, R>;

    #[inline]
    fn mul(self, rhs: Self) -> Self::Output {
        let precision = self.precision.max(rhs.precision);
        let mantissa = mul_hi::<X>(&self.mantissa, &rhs.mantissa, precision + 1);
        let exponent = self.exponent + rhs.exponent;
        FloatRepr { mantissa, exponent, precision: precision + 1 }.with_precision(precision)
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
