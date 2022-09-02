use crate::{fbig::FBig, repr::Word, round::Round};
use core::ops::{Shl, ShlAssign, Shr, ShrAssign};

impl<R: Round, const B: Word> Shl<usize> for FBig<R, B> {
    type Output = Self;
    #[inline]
    fn shl(mut self, rhs: usize) -> Self::Output {
        self.repr.exponent += rhs as isize;
        self
    }
}

impl<R: Round, const B: Word> ShlAssign<usize> for FBig<R, B> {
    #[inline]
    fn shl_assign(&mut self, rhs: usize) {
        self.repr.exponent += rhs as isize;
    }
}

impl<R: Round, const B: Word> Shr<usize> for FBig<R, B> {
    type Output = Self;
    #[inline]
    fn shr(mut self, rhs: usize) -> Self::Output {
        self.repr.exponent -= rhs as isize;
        self
    }
}

impl<R: Round, const B: Word> ShrAssign<usize> for FBig<R, B> {
    #[inline]
    fn shr_assign(&mut self, rhs: usize) {
        self.repr.exponent -= rhs as isize;
    }
}
