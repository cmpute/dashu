use core::ops::Neg;
use dashu_base::Sign;

use crate::{rbig::RBig, repr::Repr};

impl Neg for Repr {
    type Output = Repr;
    #[inline]
    fn neg(mut self) -> Self::Output {
        self.numerator = -self.numerator;
        self
    }
}

impl Neg for RBig {
    type Output = RBig;
    #[inline]
    fn neg(self) -> Self::Output {
        RBig(-self.0)
    }
}
