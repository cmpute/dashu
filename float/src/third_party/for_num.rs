use num_traits::{FromPrimitive, One, ToPrimitive, Zero};

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

impl<R: Round, const B: Word> FromPrimitive for FBig<R, B> {
    #[inline]
    fn from_isize(n: isize) -> Option<Self> {
        Some(FBig::from(n))
    }
    #[inline]
    fn from_i8(n: i8) -> Option<Self> {
        Some(FBig::from(n))
    }
    #[inline]
    fn from_i16(n: i16) -> Option<Self> {
        Some(FBig::from(n))
    }
    #[inline]
    fn from_i32(n: i32) -> Option<Self> {
        Some(FBig::from(n))
    }
    #[inline]
    fn from_i64(n: i64) -> Option<Self> {
        Some(FBig::from(n))
    }
    #[inline]
    fn from_i128(n: i128) -> Option<Self> {
        Some(FBig::from(n))
    }
    #[inline]
    fn from_usize(n: usize) -> Option<Self> {
        Some(FBig::from(n))
    }
    #[inline]
    fn from_u8(n: u8) -> Option<Self> {
        Some(FBig::from(n))
    }
    #[inline]
    fn from_u16(n: u16) -> Option<Self> {
        Some(FBig::from(n))
    }
    #[inline]
    fn from_u32(n: u32) -> Option<Self> {
        Some(FBig::from(n))
    }
    #[inline]
    fn from_u64(n: u64) -> Option<Self> {
        Some(FBig::from(n))
    }
    #[inline]
    fn from_u128(n: u128) -> Option<Self> {
        Some(FBig::from(n))
    }
    // #[inline]
    // fn from_f32(n: f32) -> Option<Self> {
    //     FBig::to_f32()
    // }
    // #[inline]
    // fn from_f64(n: f64) -> Option<Self> {
    //     FBig::try_from(n).ok()
    // }
}



#[cfg(test)]
mod tests {
    use std::ops::Neg;
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

    #[test]
    fn test_from() {
        assert_eq!(DBig::from_usize(1), Some(DBig::one()));
        assert_eq!(DBig::from_isize(-1), Some(DBig::one().neg()));
    }
}
