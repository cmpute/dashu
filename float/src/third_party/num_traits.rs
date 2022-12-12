//! Implement num-traits traits.

use crate::{fbig::FBig, round::Round};
use dashu_int::{IBig, Word};
use num_traits_v02 as num_traits;

impl<R: Round, const B: Word> num_traits::Zero for FBig<R, B> {
    #[inline]
    fn zero() -> Self {
        FBig::ZERO
    }
    #[inline]
    fn is_zero(&self) -> bool {
        self.repr.is_zero()
    }
}

impl<R: Round, const B: Word> num_traits::One for FBig<R, B> {
    #[inline]
    fn one() -> Self {
        FBig::ONE
    }
    #[inline]
    fn is_one(&self) -> bool {
        self.repr.is_one()
    }
}

macro_rules! impl_from_primitive_int {
    ($t:ty, $method:ident) => {
        #[inline]
        fn $method(n: $t) -> Option<Self> {
            Some(FBig::from(n))
        }
    };
}

impl<R: Round, const B: Word> num_traits::FromPrimitive for FBig<R, B> {
    impl_from_primitive_int!(i8, from_i8);
    impl_from_primitive_int!(i16, from_i16);
    impl_from_primitive_int!(i32, from_i32);
    impl_from_primitive_int!(i64, from_i64);
    impl_from_primitive_int!(i128, from_i128);
    impl_from_primitive_int!(isize, from_isize);
    impl_from_primitive_int!(u8, from_u8);
    impl_from_primitive_int!(u16, from_u16);
    impl_from_primitive_int!(u32, from_u32);
    impl_from_primitive_int!(u64, from_u64);
    impl_from_primitive_int!(u128, from_u128);
    impl_from_primitive_int!(usize, from_usize);

    #[inline]
    fn from_f32(f: f32) -> Option<Self> {
        match FBig::<R, 2>::try_from(f) {
            Ok(val) => Some(val.with_base::<B>().value()),
            Err(_) => None,
        }
    }
    #[inline]
    fn from_f64(f: f64) -> Option<Self> {
        match FBig::<R, 2>::try_from(f) {
            Ok(val) => Some(val.with_base::<B>().value()),
            Err(_) => None,
        }
    }
}

macro_rules! impl_to_primitive_int {
    ($t:ty, $method:ident) => {
        #[inline]
        fn $method(&self) -> Option<$t> {
            num_traits::ToPrimitive::$method(&self.to_int().value())
        }
    };
}

impl<R: Round, const B: Word> num_traits::ToPrimitive for FBig<R, B> {
    impl_to_primitive_int!(i8, to_i8);
    impl_to_primitive_int!(i16, to_i16);
    impl_to_primitive_int!(i32, to_i32);
    impl_to_primitive_int!(i64, to_i64);
    impl_to_primitive_int!(i128, to_i128);
    impl_to_primitive_int!(isize, to_isize);
    impl_to_primitive_int!(u8, to_u8);
    impl_to_primitive_int!(u16, to_u16);
    impl_to_primitive_int!(u32, to_u32);
    impl_to_primitive_int!(u64, to_u64);
    impl_to_primitive_int!(u128, to_u128);
    impl_to_primitive_int!(usize, to_usize);

    #[inline]
    fn to_f32(&self) -> Option<f32> {
        Some(self.to_f32().value())
    }
    #[inline]
    fn to_f64(&self) -> Option<f64> {
        Some(self.to_f64().value())
    }
}

impl<R: Round, const B: Word> num_traits::Pow<IBig> for FBig<R, B> {
    type Output = FBig<R, B>;

    fn pow(self, rhs: IBig) -> Self {
        self.powi(rhs)
    }
}
impl<R: Round, const B: Word> num_traits::Pow<&FBig<R, B>> for FBig<R, B> {
    type Output = FBig<R, B>;

    fn pow(self, rhs: &Self) -> Self {
        self.powf(rhs)
    }
}

// TODO: num_traits::{Num, Euclid, Signed} are not implemented for FBig, because we currently don't implement Rem for FBig.

#[cfg(test)]
mod tests {
    use super::num_traits::{FromPrimitive, One, Zero};
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
