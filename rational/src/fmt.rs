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

impl fmt::Display for Repr {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.denominator.is_one() {
            f.write_fmt(format_args!("{:?}", &self.numerator))
        } else {
            f.write_fmt(format_args!("{:?}/{:?}", &self.numerator, &self.denominator))
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

impl fmt::Display for RBig {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
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

impl fmt::Display for Relaxed {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

// TODO: support formatting as decimal (see below)
/*
 * format!("{}", rbig!(1/3)) -> 1/3
 * format!("{:.4}", rbig!(1/3)) -> 1.3333
 * format!("{:#.4}", rbig!(1/3)) -> 1.(3)
 * format!("{:e}", rbig!(1/3)) -> 1.3e0
 * format!("{:.4e}", rbig!(1/3)) -> 1.3333e0
 */
