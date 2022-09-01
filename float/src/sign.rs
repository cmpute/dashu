use crate::{fbig::FBig, repr::{Repr, Word, Context}, round::Round};
use core::ops::{Mul, Neg, MulAssign};
use dashu_base::Abs;
use dashu_int::{Sign, IBig};

impl<const B: Word, R: Round> FBig<B, R> {
    pub const fn signum(&self) -> Self {
        let significand = if self.repr.significand.is_zero() && self.repr.exponent != 0 {
            if self.repr.exponent > 0 { IBig::ONE } else { IBig::NEG_ONE }
        } else {
            self.repr.significand.signum()
        };
        let repr = Repr {
            significand,
            exponent: 0,
        };
        Self::new_raw(repr, Context::new(1))
    }
}

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

impl<const B: Word, R: Round> Mul<Sign> for FBig<B, R> {
    type Output = FBig<B, R>;
    #[inline]
    fn mul(mut self, rhs: Sign) -> Self::Output {
        self.repr.significand *= rhs;
        self
    }
}

impl<const B: Word, R: Round> MulAssign<Sign> for FBig<B, R> {
    #[inline]
    fn mul_assign(&mut self, rhs: Sign) {
        self.repr.significand *= rhs;
    }
}
