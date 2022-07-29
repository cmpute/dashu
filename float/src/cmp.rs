use crate::repr::FloatRepr;

impl<const X: usize, const R: u8> PartialEq for FloatRepr<X, R> {
    fn eq(&self, other: &Self) -> bool {
        self.mantissa == other.mantissa && self.exponent == other.exponent
    }

    fn ne(&self, other: &Self) -> bool {
        self.mantissa != other.mantissa || self.exponent != other.exponent
    }
}
impl<const X: usize, const R: u8> Eq for FloatRepr<X, R> {}
