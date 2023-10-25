use crate::{
    rbig::{RBig, Relaxed},
    repr::Repr,
};
use zeroize::Zeroize;

impl Zeroize for Repr {
    #[inline]
    fn zeroize(&mut self) {
        self.numerator.zeroize();
        self.denominator.zeroize();
    }
}

impl Zeroize for RBig {
    #[inline]
    fn zeroize(&mut self) {
        self.0.zeroize()
    }
}

impl Zeroize for Relaxed {
    #[inline]
    fn zeroize(&mut self) {
        self.0.zeroize()
    }
}
