use crate::{repr::Repr, RBig, Relaxed};
use core::{
    cmp::Ordering,
    hash::{Hash, Hasher},
};
use dashu_base::{BitTest, Sign::*};

impl PartialEq for Repr {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        // for relaxed representation, we have to compare it's actual value
        if self.numerator.sign() != other.numerator.sign() {
            return false;
        }
        if self.numerator.is_zero() {
            return other.numerator.is_zero();
        }

        let n1d2_bits = self.numerator.bit_len() as isize + other.denominator.bit_len() as isize;
        let n2d1_bits = other.numerator.bit_len() as isize + self.denominator.bit_len() as isize;
        if n1d2_bits.abs_diff(n2d1_bits) > 1 {
            return false;
        }

        // do the final product after filtering out simple cases
        (&self.numerator) * (&other.denominator) == (&other.numerator) * (&self.denominator)
    }
}
impl Eq for Repr {}

impl PartialEq for RBig {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        // representation of RBig is canonicalized, so it suffices to compare the components
        self.0.numerator == other.0.numerator && self.0.denominator == other.0.denominator
    }
}
impl Eq for RBig {}

// Hash is only implemented for RBig but not for Relaxed, because the representation
// is not unique for Relaxed.
impl Hash for RBig {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.numerator.hash(state);
        self.0.denominator.hash(state);
    }
}

impl PartialOrd for Repr {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Repr {
    fn cmp(&self, other: &Self) -> Ordering {
        // step1: compare sign
        let negative = match (self.numerator.sign(), other.numerator.sign()) {
            (Positive, Positive) => false,
            (Positive, Negative) => return Ordering::Greater,
            (Negative, Positive) => return Ordering::Less,
            (Negative, Negative) => true,
        };

        // step2: if both numbers are integers or one of them is zero
        if self.denominator.is_one() && other.denominator.is_one() {
            return self.numerator.cmp(&other.numerator);
        }
        match (self.numerator.is_zero(), other.numerator.is_zero()) {
            (true, true) => return Ordering::Equal,
            (true, false) => return Ordering::Less, // `other` must be strictly positive
            (false, true) => return Ordering::Greater, // `self` must be strictly positive
            _ => {}
        };

        // step3: test bit size
        let n1d2_bits = self.numerator.bit_len() as isize + other.denominator.bit_len() as isize;
        let n2d1_bits = other.numerator.bit_len() as isize + self.denominator.bit_len() as isize;
        if n1d2_bits > n2d1_bits + 1 {
            return if negative {
                Ordering::Less
            } else {
                Ordering::Greater
            };
        } else if n1d2_bits < n2d1_bits - 1 {
            return if negative {
                Ordering::Greater
            } else {
                Ordering::Less
            };
        }

        // step4: finally do multiplication test
        let n1d2 = (&self.numerator) * (&other.denominator);
        let n2d1 = (&other.numerator) * (&self.denominator);
        n1d2.cmp(&n2d1)
    }
}

// TODO(v0.4): deprecate comparison between RBig and Relaxed, use num-order instead
impl PartialEq<RBig> for Relaxed {
    #[inline]
    fn eq(&self, other: &RBig) -> bool {
        self.0.eq(&other.0)
    }
}
impl PartialOrd<RBig> for Relaxed {
    #[inline]
    fn partial_cmp(&self, other: &RBig) -> Option<Ordering> {
        self.0.partial_cmp(&other.0)
    }
}
impl PartialEq<Relaxed> for RBig {
    #[inline]
    fn eq(&self, other: &Relaxed) -> bool {
        self.0.eq(&other.0)
    }
}
impl PartialOrd<Relaxed> for RBig {
    #[inline]
    fn partial_cmp(&self, other: &Relaxed) -> Option<Ordering> {
        self.0.partial_cmp(&other.0)
    }
}
