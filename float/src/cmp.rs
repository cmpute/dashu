use crate::{repr::FloatRepr, round::Round};

impl<const X: usize, R: Round> PartialEq for FloatRepr<X, R> {
    fn eq(&self, other: &Self) -> bool {
        self.mantissa == other.mantissa && self.exponent == other.exponent
    }

    fn ne(&self, other: &Self) -> bool {
        self.mantissa != other.mantissa || self.exponent != other.exponent
    }
}
impl<const X: usize, R: Round> Eq for FloatRepr<X, R> {}
