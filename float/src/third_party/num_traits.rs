//! Implement num-traits traits.

use crate::{fbig::FBig, round::Round};
use dashu_int::Word;
use num_traits::{FromPrimitive, One, ToPrimitive, Zero};

impl<R: Round, const B: Word> Zero for FBig<R, B> {
    #[inline]
    fn zero() -> Self {
        FBig::ZERO
    }

    #[inline]
    fn is_zero(&self) -> bool {
        self.repr.is_zero()
    }
}

impl<R: Round, const B: Word> One for FBig<R, B> {
    #[inline]
    fn one() -> Self {
        FBig::ONE
    }
    #[inline]
    fn is_one(&self) -> bool {
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

    #[track_caller]
    fn from_f32(_: f32) -> Option<Self> {
        // TODO: implement this
        panic!("Unsupported BASE `{B}`")
    }

    #[track_caller]
    fn from_f64(_: f64) -> Option<Self> {
        // TODO: implement this
        panic!("Unsupported BASE `{B}`")
    }
}

impl<R: Round, const B: Word> ToPrimitive for FBig<R, B> {
    #[inline]
    fn to_isize(&self) -> Option<isize> {
        self.to_int().value().to_isize()
    }
    #[inline]
    fn to_i8(&self) -> Option<i8> {
        self.to_int().value().to_i8()
    }
    #[inline]
    fn to_i16(&self) -> Option<i16> {
        self.to_int().value().to_i16()
    }
    #[inline]
    fn to_i32(&self) -> Option<i32> {
        self.to_int().value().to_i32()
    }
    #[inline]
    fn to_i64(&self) -> Option<i64> {
        self.to_int().value().to_i64()
    }
    #[inline]
    fn to_i128(&self) -> Option<i128> {
        self.to_int().value().to_i128()
    }
    #[inline]
    fn to_usize(&self) -> Option<usize> {
        self.to_int().value().to_usize()
    }
    #[inline]
    fn to_u8(&self) -> Option<u8> {
        self.to_int().value().to_u8()
    }
    #[inline]
    fn to_u16(&self) -> Option<u16> {
        self.to_int().value().to_u16()
    }
    #[inline]
    fn to_u32(&self) -> Option<u32> {
        self.to_int().value().to_u32()
    }
    #[inline]
    fn to_u64(&self) -> Option<u64> {
        self.to_int().value().to_u64()
    }
    #[inline]
    fn to_u128(&self) -> Option<u128> {
        self.to_int().value().to_u128()
    }
    fn to_f32(&self) -> Option<f32> {
        // TODO: implement this
        todo!()
    }
    fn to_f64(&self) -> Option<f64> {
        // TODO: implement this
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DBig;

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
        assert_eq!(DBig::from_isize(-1), Some(-DBig::one()));
    }
}
