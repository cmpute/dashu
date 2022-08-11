use crate::{fbig::FBig, round::Round, repr::Repr};
use core::ops::{Mul, Neg};
use dashu_base::Abs;
use dashu_int::{Sign, Word};

impl<const B: Word> Neg for Repr<B> {
    type Output = Self;
    #[inline]
    fn neg(mut self) -> Self::Output {
        self.significand = -self.significand;
        self
    }
}

impl<const B: Word, R: Round> Neg for FBig<B, R> {
    type Output = Self;
    #[inline]
    fn neg(mut self) -> Self::Output {
        self.repr.significand = -self.repr.significand;
        self
    }
}

impl<const B: Word, R: Round> Neg for &FBig<B, R> {
    type Output = FBig<B, R>;
    #[inline]
    fn neg(self) -> Self::Output {
        self.clone().neg()
    }
}

impl<const B: Word, R: Round> Abs for FBig<B, R> {
    type Output = Self;
    fn abs(mut self) -> Self::Output {
        self.repr.significand = self.repr.significand.abs();
        self
    }
}

impl<const B: Word, R: Round> Mul<FBig<B, R>> for Sign {
    type Output = FBig<B, R>;
    #[inline]
    fn mul(self, mut rhs: FBig<B, R>) -> Self::Output {
        rhs.repr.significand = rhs.repr.significand * self;
        rhs
    }
}

// TODO(next): implement all variants with sign
// TODO(next): implement MulAssign for int and float
