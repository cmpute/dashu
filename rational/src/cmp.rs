use crate::rbig::{RBig, Relaxed};

impl PartialEq for RBig {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        // representation of RBig is canonicalized, so it suffices to compare the components
        self.numerator() == other.numerator() && self.denominator() == other.denominator()
    }
}
impl Eq for RBig {}

impl PartialEq for Relaxed {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        // for relaxed representation, we have to compare it's actual value
        // representation of RBig is canonicalized, so it suffices to compare the components
        self.numerator() * other.denominator() == other.numerator() * self.denominator()
    }
}
impl Eq for Relaxed {}
