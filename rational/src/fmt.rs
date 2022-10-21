use crate::{
    rbig::{RBig, Relaxed},
    repr::Repr,
};
use core::fmt;

impl fmt::Debug for Repr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_struct("Repr")
                .field("numerator", &format_args!("{:#?}", &self.numerator))
                .field("denominator", &format_args!("{:#?}", &self.denominator))
                .finish()
        } else {
            f.write_fmt(format_args!("{:?} / {:?}", &self.numerator, &self.denominator))
        }
    }
}

impl fmt::Debug for RBig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_struct("RBig")
                .field("numerator", &format_args!("{:#?}", self.numerator()))
                .field("denominator", &format_args!("{:#?}", self.denominator()))
                .finish()
        } else {
            f.write_fmt(format_args!("{:?} / {:?}", self.numerator(), self.denominator()))
        }
    }
}

impl fmt::Debug for Relaxed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_struct("Relaxed")
                .field("numerator", &format_args!("{:#?}", self.numerator()))
                .field("denominator", &format_args!("{:#?}", self.denominator()))
                .finish()
        } else {
            f.write_fmt(format_args!("{:?} / {:?}", self.numerator(), self.denominator()))
        }
    }
}
