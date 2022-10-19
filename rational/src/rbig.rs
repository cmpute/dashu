use dashu_int::{IBig, UBig};

use crate::repr::Repr;

#[repr(transparent)]
pub struct RBig(pub(crate) Repr);

#[repr(transparent)]
pub struct Relaxed(pub(crate) Repr); // the result is not always normalized

impl RBig {
    #[inline]
    pub fn into_parts(self) -> (IBig, UBig) {
        (self.0.numerator, self.0.denominator)
    }
}

impl Relaxed {
    #[inline]
    pub fn into_parts(self) -> (IBig, UBig) {
        (self.0.numerator, self.0.denominator)
    }
}
