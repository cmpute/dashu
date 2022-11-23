use num_traits::{One, Zero};

use dashu_int::Word;

use crate::round::Round;
use crate::FBig;

impl<R: Round, const B: Word> Zero for FBig<R, B> {
    #[inline]
    fn zero() -> Self {
        FBig::from(0)
    }

    #[inline]
    fn is_zero(&self) -> bool {
        self.repr.is_zero()
    }
}

impl<R: Round, const B: Word> One for FBig<R, B> {
    #[inline]
    fn one() -> Self {
        FBig::from(1)
    }
    #[inline]
    fn is_one(&self) -> bool
    where
        Self: PartialEq,
    {
        self.repr.is_one()
    }
}

#[cfg(test)]
mod tests {
    use crate::DBig;

    use super::*;

    #[test]
    fn test_01() {
        assert_eq!(DBig::from(0), DBig::zero());
        assert_eq!(DBig::from(1), DBig::one());

        assert!(DBig::from(0).is_zero());
        assert!(!DBig::from(0).is_one());
        assert!(!DBig::from(1).is_zero());
        assert!(DBig::from(1).is_one());
    }
}
