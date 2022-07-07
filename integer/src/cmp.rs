//! Comparisons operators.

use crate::{
    arch::word::Word,
    ibig::IBig,
    repr::TypedReprRef::{self, *},
    sign::Sign::*,
    ubig::UBig,
};
use core::cmp::Ordering;

/// Compare two `Repr`s
fn repr_cmp(lhs: TypedReprRef, rhs: TypedReprRef) -> Ordering {
    match (lhs, rhs) {
        (RefSmall(dword), RefSmall(other_dword)) => dword.cmp(&other_dword),
        (RefSmall(_), RefLarge(_)) => Ordering::Less,
        (RefLarge(_), RefSmall(_)) => Ordering::Greater,
        (RefLarge(buffer), RefLarge(other_buffer)) => buffer
            .len()
            .cmp(&other_buffer.len())
            .then_with(|| cmp_same_len(buffer, other_buffer)),
    }
}

impl Ord for UBig {
    #[inline]
    fn cmp(&self, other: &UBig) -> Ordering {
        repr_cmp(self.repr(), other.repr())
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
            (Positive, Positive) => repr_cmp(lhs_mag, rhs_mag),
            (Positive, Negative) => Ordering::Greater,
            (Negative, Positive) => Ordering::Less,
            (Negative, Negative) => repr_cmp(rhs_mag, lhs_mag),
        }
    }
}

impl PartialOrd for IBig {
    #[inline]
    fn partial_cmp(&self, other: &IBig) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Compare lhs with rhs as numbers.
pub(crate) fn cmp_same_len(lhs: &[Word], rhs: &[Word]) -> Ordering {
    debug_assert!(lhs.len() == rhs.len());
    lhs.iter().rev().cmp(rhs.iter().rev())
}

// TODO: implement cmp with primitive integers and eliminate UBig::from(xu8) in the code
// TODO: implement cmp between IBig and UBig
