//! Comparisons operators.

use dashu_base::{AbsEq, AbsOrd};

use crate::{
    arch::word::Word,
    ibig::IBig,
    repr::TypedReprRef::{self, *},
    ubig::UBig,
    Sign::*,
};
use core::cmp::Ordering;

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

impl AbsEq for UBig {
    #[inline]
    fn abs_eq(&self, rhs: &Self) -> bool {
        self.eq(rhs)
    }
}
impl AbsEq for IBig {
    #[inline]
    fn abs_eq(&self, rhs: &Self) -> bool {
        self.0.as_sign_slice().1.eq(rhs.0.as_sign_slice().1)
    }
}
impl AbsEq<UBig> for IBig {
    #[inline]
    fn abs_eq(&self, rhs: &UBig) -> bool {
        self.0.as_sign_slice().1.eq(rhs.0.as_slice())
    }
}
impl AbsEq<IBig> for UBig {
    #[inline]
    fn abs_eq(&self, rhs: &IBig) -> bool {
        self.0.as_slice().eq(rhs.0.as_sign_slice().1)
    }
}

impl AbsOrd for UBig {
    #[inline]
    fn abs_cmp(&self, rhs: &Self) -> Ordering {
        self.0.as_typed().cmp(&rhs.0.as_typed())
    }
}
impl AbsOrd for IBig {
    #[inline]
    fn abs_cmp(&self, rhs: &Self) -> Ordering {
        self.0.as_sign_typed().1.cmp(&rhs.0.as_sign_typed().1)
    }
}
impl AbsOrd<UBig> for IBig {
    #[inline]
    fn abs_cmp(&self, rhs: &UBig) -> Ordering {
        self.0.as_sign_typed().1.cmp(&rhs.0.as_typed())
    }
}
impl AbsOrd<IBig> for UBig {
    #[inline]
    fn abs_cmp(&self, rhs: &IBig) -> Ordering {
        self.0.as_typed().cmp(&rhs.0.as_sign_typed().1)
    }
}
