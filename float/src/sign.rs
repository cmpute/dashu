use crate::{repr::FloatRepr, round::Round};
use core::ops::{Mul, Neg};
use dashu_base::Abs;
use dashu_int::Sign;

impl<const X: usize, R: Round> Neg for FloatRepr<X, R> {
    type Output = Self;
    #[inline]
    fn neg(mut self) -> Self::Output {
        self.mantissa = -self.mantissa;
        self
    }
}

impl<const X: usize, R: Round> Neg for &FloatRepr<X, R> {
    type Output = FloatRepr<X, R>;
    #[inline]
    fn neg(self) -> Self::Output {
        self.clone().neg()
    }
}

impl<const X: usize, R: Round> Abs for FloatRepr<X, R> {
    type Output = Self;
    fn abs(mut self) -> Self::Output {
        self.mantissa = self.mantissa.abs();
        self
    }
}

impl<const X: usize, R: Round> Mul<FloatRepr<X, R>> for Sign {
    type Output = FloatRepr<X, R>;
    #[inline]
    fn mul(self, mut rhs: FloatRepr<X, R>) -> Self::Output {
        rhs.mantissa = rhs.mantissa * self;
        rhs
    }
}

// TODO: implement all variants with sign
// TODO: implement MulAssign for int and float
