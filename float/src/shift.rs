use crate::{fbig::FBig, repr::Word, round::Round, error::check_inf};
use core::ops::{Shl, ShlAssign, Shr, ShrAssign};

impl<R: Round, const B: Word> Shl<isize> for FBig<R, B> {
    type Output = Self;
    #[inline]
    fn shl(mut self, rhs: isize) -> Self::Output {
        check_inf(&self.repr);
        if !self.repr.is_zero() {
            self.repr.exponent += rhs;            
        }
        self
    }
}

impl<R: Round, const B: Word> ShlAssign<isize> for FBig<R, B> {
    #[inline]
    fn shl_assign(&mut self, rhs: isize) {
        check_inf(&self.repr);
        if !self.repr.is_zero() {
            self.repr.exponent += rhs;
        }
    }
}

impl<R: Round, const B: Word> Shr<isize> for FBig<R, B> {
    type Output = Self;
    #[inline]
    fn shr(mut self, rhs: isize) -> Self::Output {
        check_inf(&self.repr);
        if !self.repr.is_zero() {
            self.repr.exponent -= rhs;
        }
        self
    }
}

impl<R: Round, const B: Word> ShrAssign<isize> for FBig<R, B> {
    #[inline]
    fn shr_assign(&mut self, rhs: isize) {
        check_inf(&self.repr);
        if !self.repr.is_zero() {
            self.repr.exponent -= rhs;
        }
        self.repr.exponent -= rhs;
    }
}
