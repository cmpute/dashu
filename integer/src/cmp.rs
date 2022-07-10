//! Comparisons operators.

use crate::{
    arch::word::Word,
    ibig::IBig,
    repr::TypedReprRef::{self, *},
    sign::Sign::*,
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
            (RefSmall(dword), RefSmall(other_dword)) => dword.cmp(&other_dword),
            (RefSmall(_), RefLarge(_)) => Ordering::Less,
            (RefLarge(_), RefSmall(_)) => Ordering::Greater,
            (RefLarge(buffer), RefLarge(other_buffer)) => buffer
                .len()
                .cmp(&other_buffer.len())
                .then_with(|| cmp_same_len(buffer, other_buffer)),
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

/// Compare lhs with rhs as numbers.
pub fn cmp_same_len(lhs: &[Word], rhs: &[Word]) -> Ordering {
    debug_assert!(lhs.len() == rhs.len());
    lhs.iter().rev().cmp(rhs.iter().rev())
}
