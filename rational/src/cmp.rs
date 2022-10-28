use crate::rbig::Relaxed;

impl PartialEq for Relaxed {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        // for relaxed representation, we have to compare it's actual value
        // representation of RBig is canonicalized, so it suffices to compare the components
        self.numerator() * other.denominator() == other.numerator() * self.denominator()
    }
}
impl Eq for Relaxed {}
