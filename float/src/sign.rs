use crate::repr::FloatRepr;
use core::ops::Neg;
use dashu_base::Abs;

impl<const X: usize, const R: u8> Neg for FloatRepr<X, R> {
    type Output = Self;
    fn neg(mut self) -> Self::Output {
        self.mantissa = -self.mantissa;
        self
    }
}

impl<const X: usize, const R: u8> Neg for &FloatRepr<X, R> {
    type Output = FloatRepr<X, R>;
    fn neg(self) -> Self::Output {
        self.clone().neg()
    }
}

impl<const X: usize, const R: u8> Abs for FloatRepr<X, R> {
    type Output = Self;
    fn abs(mut self) -> Self::Output {
        self.mantissa = self.mantissa.abs();
        self
    }
}
