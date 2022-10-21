use dashu_int::{IBig, UBig};

use crate::{error::panic_divide_by_0, repr::Repr};

#[repr(transparent)]
pub struct RBig(pub(crate) Repr);

#[repr(transparent)]
pub struct Relaxed(pub(crate) Repr); // the result is not always normalized

impl RBig {
    #[inline]
    pub fn from_parts(numerator: IBig, denominator: UBig) -> Self {
        if denominator.is_zero() {
            panic_divide_by_0()
        }
        let mut repr = Repr {
            numerator,
            denominator,
        };
        repr.reduce();
        Self(repr)
    }
    #[inline]
    pub fn into_parts(self) -> (IBig, UBig) {
        (self.0.numerator, self.0.denominator)
    }
    #[inline]
    pub fn numerator(&self) -> &IBig {
        &self.0.numerator
    }
    #[inline]
    pub fn denominator(&self) -> &UBig {
        &self.0.denominator
    }
}

impl Relaxed {
    #[inline]
    pub fn from_parts(numerator: IBig, denominator: UBig) -> Self {
        if denominator.is_zero() {
            panic_divide_by_0();
        }

        let mut repr = Repr {
            numerator,
            denominator,
        };
        repr.reduce2();
        Self(repr)
    }
    #[inline]
    pub fn into_parts(self) -> (IBig, UBig) {
        (self.0.numerator, self.0.denominator)
    }
    #[inline]
    pub fn numerator(&self) -> &IBig {
        &self.0.numerator
    }
    #[inline]
    pub fn denominator(&self) -> &UBig {
        &self.0.denominator
    }
}
