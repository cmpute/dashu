#![allow(deprecated)] // TODO(v0.5): remove after the implementations for AbsEq are removed.

use crate::{repr::Repr, RBig, Relaxed};
use core::{
    cmp::Ordering,
    hash::{Hash, Hasher},
};
use dashu_base::{
    AbsEq, AbsOrd, BitTest, EstimatedLog2,
    Sign::{self, *},
};
use dashu_int::{IBig, UBig};

/// Check whether a == b. `ABS` determine whether the signs are ignored during comparison
fn repr_eq<const ABS: bool>(a: &Repr, b: &Repr) -> bool {
    // for relaxed representation, we have to compare it's actual value
    if !ABS && a.numerator.sign() != b.numerator.sign() {
        return false;
    }
    if a.numerator.is_zero() {
        return b.numerator.is_zero();
    }

    let n1d2_bits = a.numerator.bit_len() as isize + b.denominator.bit_len() as isize;
    let n2d1_bits = b.numerator.bit_len() as isize + a.denominator.bit_len() as isize;
    if n1d2_bits.abs_diff(n2d1_bits) > 1 {
        return false;
    }

    // do the final product after filtering out simple cases
    let lhs = &a.numerator * &b.denominator;
    let rhs = &b.numerator * &a.denominator;
    lhs.abs_eq(&rhs)
}

impl PartialEq for Repr {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        repr_eq::<false>(self, other)
    }
}
impl Eq for Repr {}

impl AbsEq for Repr {
    #[inline]
    fn abs_eq(&self, other: &Self) -> bool {
        repr_eq::<true>(self, other)
    }
}

impl AbsEq for Relaxed {
    #[inline]
    fn abs_eq(&self, other: &Self) -> bool {
        self.0.abs_eq(&other.0)
    }
}

impl PartialEq for RBig {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        // representation of RBig is canonicalized, so it suffices to compare the components
        self.0.numerator == other.0.numerator && self.0.denominator == other.0.denominator
    }
}
impl Eq for RBig {}

impl AbsEq for RBig {
    #[inline]
    fn abs_eq(&self, other: &Self) -> bool {
        // representation of RBig is canonicalized, so it suffices to compare the components
        self.0.numerator.abs_eq(&other.0.numerator) && self.0.denominator == other.0.denominator
    }
}

// Hash is only implemented for RBig but not for Relaxed, because the representation
// is not unique for Relaxed.
impl Hash for RBig {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.numerator.hash(state);
        self.0.denominator.hash(state);
    }
}

fn repr_cmp<const ABS: bool>(lhs: &Repr, rhs: &Repr) -> Ordering {
    // step1: compare sign
    let negative = if ABS {
        false
    } else {
        match (lhs.numerator.sign(), rhs.numerator.sign()) {
            (Positive, Positive) => false,
            (Positive, Negative) => return Ordering::Greater,
            (Negative, Positive) => return Ordering::Less,
            (Negative, Negative) => true,
        }
    };

    // step2: if both numbers are integers or one of them is zero
    if lhs.denominator.is_one() && rhs.denominator.is_one() {
        return if ABS {
            lhs.numerator.abs_cmp(&rhs.numerator)
        } else {
            lhs.numerator.cmp(&rhs.numerator)
        };
    }
    match (lhs.numerator.is_zero(), rhs.numerator.is_zero()) {
        (true, true) => return Ordering::Equal,
        (true, false) => return Ordering::Less, // `b` must be strictly positive
        (false, true) => return Ordering::Greater, // `a` must be strictly positive
        _ => {}
    };

    // step3: test bit size
    let lhs_bits = lhs.numerator.bit_len() as isize - lhs.denominator.bit_len() as isize;
    let rhs_bits = rhs.numerator.bit_len() as isize - rhs.denominator.bit_len() as isize;
    if lhs_bits > rhs_bits + 1 {
        return match negative {
            false => Ordering::Greater,
            true => Ordering::Less,
        };
    } else if rhs_bits < lhs_bits - 1 {
        return match negative {
            false => Ordering::Less,
            true => Ordering::Greater,
        };
    }

    // step4: finally do multiplication test
    let n1d2 = (&lhs.numerator) * (&rhs.denominator);
    let n2d1 = (&rhs.numerator) * (&lhs.denominator);
    if ABS {
        n1d2.abs_cmp(&n2d1)
    } else {
        n1d2.cmp(&n2d1)
    }
}

impl PartialOrd for Repr {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Repr {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        repr_cmp::<false>(self, other)
    }
}

impl AbsOrd for Repr {
    #[inline]
    fn abs_cmp(&self, other: &Self) -> Ordering {
        repr_cmp::<true>(self, other)
    }
}

macro_rules! forward_abs_ord_both_to_repr {
    ($t1:ty, $t2:ty) => {
        impl AbsOrd<$t2> for $t1 {
            #[inline]
            fn abs_cmp(&self, other: &$t2) -> Ordering {
                repr_cmp::<true>(&self.0, &other.0)
            }
        }
    };
}
forward_abs_ord_both_to_repr!(RBig, RBig);
forward_abs_ord_both_to_repr!(RBig, Relaxed);
forward_abs_ord_both_to_repr!(Relaxed, RBig);
forward_abs_ord_both_to_repr!(Relaxed, Relaxed);

macro_rules! forward_abs_ord_to_repr {
    ($R:ty, $T:ty) => {
        impl AbsOrd<$T> for $R {
            #[inline]
            fn abs_cmp(&self, other: &$T) -> Ordering {
                self.0.abs_cmp(other)
            }
        }
        impl AbsOrd<$R> for $T {
            #[inline]
            fn abs_cmp(&self, other: &$R) -> Ordering {
                other.0.abs_cmp(self).reverse()
            }
        }
    };
}
// forward_abs_ord_to_repr!(RBig, IBig);
// forward_abs_ord_to_repr!(Relaxed, IBig);

pub(crate) fn repr_cmp_ubig<const ABS: bool>(lhs: &Repr, rhs: &UBig) -> Ordering {
    // case 1: compare sign
    if !ABS && lhs.numerator.sign() == Sign::Negative {
        return Ordering::Less;
    }

    // case 2: compare log2 estimations
    let (lhs_lo, lhs_hi) = lhs.log2_bounds();
    let (rhs_lo, rhs_hi) = rhs.log2_bounds();
    if lhs_lo > rhs_hi {
        return Ordering::Greater;
    }
    if lhs_hi < rhs_lo {
        return Ordering::Less;
    }

    // case 3: compare the exact values
    lhs.numerator.abs_cmp(&(rhs * &lhs.denominator))
}

impl AbsOrd<UBig> for Repr {
    #[inline]
    fn abs_cmp(&self, other: &UBig) -> Ordering {
        repr_cmp_ubig::<true>(self, other)
    }
}
forward_abs_ord_to_repr!(RBig, UBig);
forward_abs_ord_to_repr!(Relaxed, UBig);

pub(crate) fn repr_cmp_ibig<const ABS: bool>(lhs: &Repr, rhs: &IBig) -> Ordering {
    // case 1: compare sign
    let sign = if ABS {
        Sign::Positive
    } else {
        match lhs.numerator.signum().cmp(&rhs.signum()) {
            Ordering::Greater => return Ordering::Greater,
            Ordering::Less => return Ordering::Less,
            _ => {}
        };
        lhs.numerator.sign()
    };

    // case 2: compare log2 estimations
    let (lhs_lo, lhs_hi) = lhs.log2_bounds();
    let (rhs_lo, rhs_hi) = rhs.log2_bounds();
    if lhs_lo > rhs_hi {
        return sign * Ordering::Greater;
    }
    if lhs_hi < rhs_lo {
        return sign * Ordering::Less;
    }

    // case 3: compare the exact values
    if ABS {
        lhs.numerator.abs_cmp(&(rhs * &lhs.denominator))
    } else {
        lhs.numerator.cmp(&(rhs * &lhs.denominator))
    }
}

impl AbsOrd<IBig> for Repr {
    #[inline]
    fn abs_cmp(&self, other: &IBig) -> Ordering {
        repr_cmp_ibig::<true>(self, other)
    }
}
forward_abs_ord_to_repr!(RBig, IBig);
forward_abs_ord_to_repr!(Relaxed, IBig);

#[cfg(feature = "dashu-float")]
pub(crate) mod with_float {
    use super::*;
    use dashu_float::{round::Round, FBig, Repr as FloatRepr};
    use dashu_int::Word;

    pub(crate) fn repr_cmp_fbig<const B: Word, const ABS: bool>(
        lhs: &Repr,
        rhs: &FloatRepr<B>,
    ) -> Ordering {
        // case 1: compare with inf
        if rhs.is_infinite() {
            return match ABS || rhs.exponent() > 0 {
                true => Ordering::Less,
                false => Ordering::Greater,
            };
        }

        // case 2: compare sign
        let sign = if ABS {
            Sign::Positive
        } else {
            match lhs.numerator.signum().cmp(&rhs.significand().signum()) {
                Ordering::Greater => return Ordering::Greater,
                Ordering::Less => return Ordering::Less,
                _ => {}
            };
            lhs.numerator.sign()
        };

        // case 3: compare log2 estimations
        let (lhs_lo, lhs_hi) = lhs.log2_bounds();
        let (rhs_lo, rhs_hi) = rhs.log2_bounds();
        if lhs_lo > rhs_hi {
            return sign * Ordering::Greater;
        }
        if lhs_hi < rhs_lo {
            return sign * Ordering::Less;
        }

        let rhs_exp = rhs.exponent();

        // case 4: compare the exact values
        let (mut lhs, mut rhs) = (lhs.numerator.clone(), rhs.significand() * &lhs.denominator);
        if rhs_exp < 0 {
            let exp = -rhs_exp as usize;
            if B.is_power_of_two() {
                lhs <<= exp * B.trailing_zeros() as usize;
            } else {
                lhs *= UBig::from_word(B).pow(exp);
            }
        } else {
            let exp = rhs_exp as usize;
            if B.is_power_of_two() {
                rhs <<= exp * B.trailing_zeros() as usize;
            } else {
                rhs *= UBig::from_word(B).pow(exp);
            }
        }

        if ABS {
            lhs.abs_cmp(&rhs)
        } else {
            lhs.cmp(&rhs)
        }
    }

    impl<R: Round, const B: Word> AbsOrd<FBig<R, B>> for RBig {
        #[inline]
        fn abs_cmp(&self, other: &FBig<R, B>) -> Ordering {
            repr_cmp_fbig::<B, true>(&self.0, other.repr())
        }
    }

    impl<R: Round, const B: Word> AbsOrd<FBig<R, B>> for Relaxed {
        #[inline]
        fn abs_cmp(&self, other: &FBig<R, B>) -> Ordering {
            repr_cmp_fbig::<B, true>(&self.0, other.repr())
        }
    }

    impl<R: Round, const B: Word> AbsOrd<RBig> for FBig<R, B> {
        #[inline]
        fn abs_cmp(&self, other: &RBig) -> Ordering {
            repr_cmp_fbig::<B, true>(&other.0, self.repr()).reverse()
        }
    }

    impl<R: Round, const B: Word> AbsOrd<Relaxed> for FBig<R, B> {
        #[inline]
        fn abs_cmp(&self, other: &Relaxed) -> Ordering {
            repr_cmp_fbig::<B, true>(&other.0, self.repr()).reverse()
        }
    }
}
