use crate::{
    rbig::{RBig, Relaxed},
    repr::Repr,
};
use dashu_int::UBig;
use zeroize::Zeroize;

impl Zeroize for Repr {
    #[inline]
    fn zeroize(&mut self) {
        self.numerator.zeroize();
        self.denominator.zeroize();
        // the buffers are wiped, now restore the invariant of a nonzero denominator:
        // the value becomes zero (0/1)
        self.denominator = UBig::ONE;
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
