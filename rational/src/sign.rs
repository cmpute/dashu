use core::ops::{Mul, Neg};
use dashu_base::Sign;

use crate::{
    rbig::{RBig, Relaxed},
    repr::Repr,
};

impl Repr {
    #[inline]
    fn neg(mut self) -> Repr {
        self.numerator = -self.numerator;
        self
    }
}

impl Neg for RBig {
    type Output = RBig;
    #[inline]
    fn neg(self) -> Self::Output {
        RBig(self.0.neg())
    }
}

impl Neg for &RBig {
    type Output = RBig;
    #[inline]
    fn neg(self) -> Self::Output {
        RBig(self.0.clone().neg())
    }
}

impl Neg for Relaxed {
    type Output = Relaxed;
    #[inline]
    fn neg(self) -> Self::Output {
        Relaxed(self.0.neg())
    }
}

impl Neg for &Relaxed {
    type Output = Relaxed;
    #[inline]
    fn neg(self) -> Self::Output {
        Relaxed(self.0.clone().neg())
    }
}

impl Mul<Sign> for RBig {
    type Output = RBig;
    #[inline]
    fn mul(mut self, rhs: Sign) -> RBig {
        self.0.numerator *= rhs;
        self
    }
}

impl Mul<Sign> for Relaxed {
    type Output = Relaxed;
    #[inline]
    fn mul(mut self, rhs: Sign) -> Self::Output {
        self.0.numerator *= rhs;
        self
    }
}
