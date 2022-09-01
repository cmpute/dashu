use core::ops::{Shl, ShlAssign, Shr, ShrAssign};
use crate::{round::{Round}, repr::Word, fbig::FBig};

impl<const B: Word, R: Round> Shl<usize> for FBig<B, R> {
    type Output = Self;
    #[inline]
    fn shl(mut self, rhs: usize) -> Self::Output {
        self.repr.exponent += rhs as isize;
        self
    }
}

impl<const B: Word, R: Round> ShlAssign<usize> for FBig<B, R> {
    #[inline]
    fn shl_assign(&mut self, rhs: usize) {
        self.repr.exponent += rhs as isize;
    }
}

impl<const B: Word, R: Round> Shr<usize> for FBig<B, R> {
    type Output = Self;
    #[inline]
    fn shr(mut self, rhs: usize) -> Self::Output {
        self.repr.exponent -= rhs as isize;
        self
    }
}

impl<const B: Word, R: Round> ShrAssign<usize> for FBig<B, R> {
    #[inline]
    fn shr_assign(&mut self, rhs: usize) {
        self.repr.exponent -= rhs as isize;
    }
}
