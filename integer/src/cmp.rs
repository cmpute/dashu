//! Comparisons operators.

use crate::{
    arch::word::Word,
    ibig::IBig,
    repr::TypedReprRef::{self, *},
    Sign::*,
    ubig::UBig,
};
use core::cmp::Ordering;

impl<'a> PartialOrd for TypedReprRef<'a> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<'a> Ord for TypedReprRef<'a> {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        match (*self, *other) {
            (RefSmall(dword0), RefSmall(dword1)) => dword0.cmp(&dword1),
            (RefSmall(_), RefLarge(_)) => Ordering::Less,
            (RefLarge(_), RefSmall(_)) => Ordering::Greater,
            (RefLarge(words0), RefLarge(words1)) => cmp_in_place(words0, words1),
        }
    }
}

impl Ord for UBig {
    #[inline]
    fn cmp(&self, other: &UBig) -> Ordering {
        self.repr().cmp(&other.repr())
    }
}

impl PartialOrd for UBig {
    #[inline]
    fn partial_cmp(&self, other: &UBig) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for IBig {
    #[inline]
    fn cmp(&self, other: &IBig) -> Ordering {
        let (lhs_sign, lhs_mag) = self.as_sign_repr();
        let (rhs_sign, rhs_mag) = other.as_sign_repr();
        match (lhs_sign, rhs_sign) {
            (Positive, Positive) => lhs_mag.cmp(&rhs_mag),
            (Positive, Negative) => Ordering::Greater,
            (Negative, Positive) => Ordering::Less,
            (Negative, Negative) => rhs_mag.cmp(&lhs_mag),
        }
    }
}

impl PartialOrd for IBig {
    #[inline]
    fn partial_cmp(&self, other: &IBig) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq<IBig> for UBig {
    #[inline]
    fn eq(&self, other: &IBig) -> bool {
        self.0 == other.0
    }
}

impl PartialEq<UBig> for IBig {
    #[inline]
    fn eq(&self, other: &UBig) -> bool {
        self.0 == other.0
    }
}

impl PartialOrd<IBig> for UBig {
    #[inline]
    fn partial_cmp(&self, other: &IBig) -> Option<Ordering> {
        let (rhs_sign, rhs_mag) = other.as_sign_repr();
        let ord = match rhs_sign {
            Positive => self.repr().cmp(&rhs_mag),
            Negative => Ordering::Greater,
        };
        Some(ord)
    }
}

impl PartialOrd<UBig> for IBig {
    #[inline]
    fn partial_cmp(&self, other: &UBig) -> Option<Ordering> {
        let (lhs_sign, lhs_mag) = self.as_sign_repr();
        let ord = match lhs_sign {
            Positive => lhs_mag.cmp(&other.repr()),
            Negative => Ordering::Less,
        };
        Some(ord)
    }
}

impl IBig {
    /// Check whether the magnitude of this number is equal the magnitude of the other number
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::IBig;
    /// assert!(IBig::from(2).abs_eq(&IBig::from(-2)));
    /// assert!(IBig::from(-3).abs_eq(&IBig::from(-3)));
    /// ```
    pub fn abs_eq(&self, other: &IBig) -> bool {
        self.0.as_sign_slice().1.eq(other.0.as_sign_slice().1)
    }

    /// Compare the magnitude of this number to the magnitude of the other number
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::IBig;
    /// assert!(IBig::from(2).abs_cmp(&IBig::from(-3)).is_le());
    /// assert!(IBig::from(-2).abs_cmp(&IBig::from(3)).is_le());
    /// ```
    pub fn abs_cmp(&self, other: &IBig) -> Ordering {
        self.0.as_sign_typed().1.cmp(&other.0.as_sign_typed().1)
    }
}

macro_rules! impl_cmp_with_primitive {
    ($big:ty, $prim:ty) => {
        impl PartialEq<$prim> for $big {
            #[inline]
            fn eq(&self, other: &$prim) -> bool {
                self == &<$big>::from(*other)
            }
        }

        impl PartialEq<$big> for $prim {
            #[inline]
            fn eq(&self, other: &$big) -> bool {
                other == &<$big>::from(*self)
            }
        }

        impl PartialOrd<$prim> for $big {
            #[inline]
            fn partial_cmp(&self, other: &$prim) -> Option<Ordering> {
                self.partial_cmp(&<$big>::from(*other))
            }
        }

        impl PartialOrd<$big> for $prim {
            #[inline]
            fn partial_cmp(&self, other: &$big) -> Option<Ordering> {
                <$big>::from(*self).partial_cmp(other)
            }
        }
    };
}
impl_cmp_with_primitive!(UBig, u8);
impl_cmp_with_primitive!(UBig, u16);
impl_cmp_with_primitive!(UBig, u32);
impl_cmp_with_primitive!(UBig, u64);
impl_cmp_with_primitive!(UBig, u128);
impl_cmp_with_primitive!(UBig, usize);
impl_cmp_with_primitive!(IBig, u8);
impl_cmp_with_primitive!(IBig, u16);
impl_cmp_with_primitive!(IBig, u32);
impl_cmp_with_primitive!(IBig, u64);
impl_cmp_with_primitive!(IBig, u128);
impl_cmp_with_primitive!(IBig, usize);
impl_cmp_with_primitive!(IBig, i8);
impl_cmp_with_primitive!(IBig, i16);
impl_cmp_with_primitive!(IBig, i32);
impl_cmp_with_primitive!(IBig, i64);
impl_cmp_with_primitive!(IBig, i128);
impl_cmp_with_primitive!(IBig, isize);

macro_rules! impl_cmp_ubig_with_signed_primitive {
    ($prim:ty) => {
        impl PartialEq<$prim> for UBig {
            #[inline]
            fn eq(&self, other: &$prim) -> bool {
                self == &IBig::from_signed(*other)
            }
        }

        impl PartialEq<UBig> for $prim {
            #[inline]
            fn eq(&self, other: &UBig) -> bool {
                other == &IBig::from_signed(*self)
            }
        }

        impl PartialOrd<$prim> for UBig {
            #[inline]
            fn partial_cmp(&self, other: &$prim) -> Option<Ordering> {
                self.partial_cmp(&IBig::from_signed(*other))
            }
        }

        impl PartialOrd<UBig> for $prim {
            #[inline]
            fn partial_cmp(&self, other: &UBig) -> Option<Ordering> {
                IBig::from_signed(*self).partial_cmp(other)
            }
        }
    };
}
impl_cmp_ubig_with_signed_primitive!(i8);
impl_cmp_ubig_with_signed_primitive!(i16);
impl_cmp_ubig_with_signed_primitive!(i32);
impl_cmp_ubig_with_signed_primitive!(i64);
impl_cmp_ubig_with_signed_primitive!(i128);
impl_cmp_ubig_with_signed_primitive!(isize);

/// Compare lhs with rhs of the same length as numbers.
#[inline]
pub fn cmp_same_len(lhs: &[Word], rhs: &[Word]) -> Ordering {
    debug_assert!(lhs.len() == rhs.len());
    lhs.iter().rev().cmp(rhs.iter().rev())
}

/// Compare lhs with rhs as numbers. The leading zero words of the input must be trimmed.
///
/// # Panics
///
/// Panic if lhs or rhs has leading zero words (including the case where lhs == 0 or rhs == 0)
pub fn cmp_in_place(lhs: &[Word], rhs: &[Word]) -> Ordering {
    debug_assert!(*lhs.last().unwrap() != 0 && *rhs.last().unwrap() != 0);
    lhs.len()
        .cmp(&rhs.len())
        .then_with(|| cmp_same_len(lhs, rhs))
}
