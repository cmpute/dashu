use crate::{
    rbig::{RBig, Relaxed},
    repr::Repr,
};
use core::fmt::{self, Write};

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
            fmt::Display::fmt(&self.numerator, f)
        } else {
            fmt::Display::fmt(&self.numerator, f)?;
            f.write_char('/')?;
            fmt::Display::fmt(&self.denominator, f)
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
