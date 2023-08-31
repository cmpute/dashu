use crate::{rbig::RBig, repr::Repr, Relaxed};
use _num_modular::{FixedMersenneInt, ModularInteger};
use core::cmp::Ordering;
use dashu_base::{AbsCmp, EstimatedLog2, Sign};
use dashu_int::{IBig, UBig};
use num_order::{NumHash, NumOrd};

impl NumOrd<UBig> for RBig {
    fn num_cmp(&self, other: &UBig) -> Ordering {
        // case 1: compare sign
        if self.sign() == Sign::Negative {
            return Ordering::Less;
        }

        // case 2: compare log2 estimations
        let (self_lo, self_hi) = self.log2_bounds();
        let (other_lo, other_hi) = other.log2_bounds();
        if self_lo > other_hi {
            return Ordering::Greater;
        }
        if self_hi < other_lo {
            return Ordering::Less;
        }

        // case 3: compare the exact values
        self.numerator().abs_cmp(&(other * self.denominator()))
    }
    #[inline]
    fn num_partial_cmp(&self, other: &UBig) -> Option<Ordering> {
        Some(self.num_cmp(other))
    }
}

impl NumOrd<RBig> for UBig {
    #[inline]
    fn num_cmp(&self, other: &RBig) -> Ordering {
        other.num_cmp(self).reverse()
    }
    #[inline]
    fn num_partial_cmp(&self, other: &RBig) -> Option<Ordering> {
        Some(other.num_cmp(self).reverse())
    }
}

impl NumOrd<IBig> for RBig {
    fn num_cmp(&self, other: &IBig) -> Ordering {
        // case 1: compare sign
        match self.numerator().signum().cmp(&other.signum()) {
            Ordering::Greater => return Ordering::Greater,
            Ordering::Less => return Ordering::Less,
            _ => {}
        };
        let sign = self.sign();

        // case 2: compare log2 estimations
        let (self_lo, self_hi) = self.log2_bounds();
        let (other_lo, other_hi) = other.log2_bounds();
        if self_lo > other_hi {
            return match sign {
                Sign::Positive => Ordering::Greater,
                Sign::Negative => Ordering::Less,
            };
        }
        if self_hi < other_lo {
            return match sign {
                Sign::Positive => Ordering::Less,
                Sign::Negative => Ordering::Greater,
            };
        }

        // case 3: compare the exact values
        self.numerator().cmp(&(other * self.denominator()))
    }
    #[inline]
    fn num_partial_cmp(&self, other: &IBig) -> Option<Ordering> {
        Some(self.num_cmp(other))
    }
}

impl NumOrd<RBig> for IBig {
    #[inline]
    fn num_cmp(&self, other: &RBig) -> Ordering {
        other.num_cmp(self).reverse()
    }
    #[inline]
    fn num_partial_cmp(&self, other: &RBig) -> Option<Ordering> {
        Some(other.num_cmp(self).reverse())
    }
}

impl NumHash for Repr {
    fn num_hash<H: core::hash::Hasher>(&self, state: &mut H) {
        // 2^127 - 1 is used in the num-order crate
        type MInt = FixedMersenneInt<127, 1>;
        const M127: i128 = i128::MAX;
        const M127U: u128 = M127 as u128;
        const HASH_INF: i128 = i128::MAX;
        const HASH_NEGINF: i128 = i128::MIN + 1;

        let ub = (&self.denominator) % M127U; // denom is always positive in Ratio
        let binv = if ub != 0 {
            MInt::new(ub, &M127U).inv().unwrap()
        } else {
            // no modular inverse, use INF or NEGINF as the result
            return if self.numerator.sign() == Sign::Positive {
                HASH_INF.num_hash(state)
            } else {
                HASH_NEGINF.num_hash(state)
            };
        };

        let ua = (&self.numerator) % M127;
        let ua = binv.convert(ua.unsigned_abs());
        let ab = (ua * binv).residue() as i128;
        (self.numerator.sign() * ab).num_hash(state)
    }
}

impl NumHash for RBig {
    #[inline]
    fn num_hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.0.num_hash(state)
    }
}

impl NumHash for Relaxed {
    #[inline]
    fn num_hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.0.num_hash(state)
    }
}

#[cfg(feature = "dashu-float")]
mod with_float {
    use super::*;
    use dashu_float::{round::Round, FBig, Repr as FloatRepr};
    use dashu_int::Word;

    impl<const B: Word> NumOrd<FloatRepr<B>> for Repr {
        fn num_cmp(&self, other: &FloatRepr<B>) -> Ordering {
            // case 1: compare with inf
            if other.is_infinite() {
                return if other.exponent() > 0 {
                    Ordering::Less
                } else {
                    Ordering::Greater
                };
            }

            // case 2: compare sign
            match self.numerator.signum().cmp(&other.significand().signum()) {
                Ordering::Greater => return Ordering::Greater,
                Ordering::Less => return Ordering::Less,
                _ => {}
            };
            let sign = self.numerator.sign();

            // case 3: compare log2 estimations
            let (self_lo, self_hi) = self.log2_bounds();
            let (other_lo, other_hi) = other.log2_bounds();
            if self_lo > other_hi {
                return sign * Ordering::Greater;
            }
            if self_hi < other_lo {
                return sign * Ordering::Less;
            }

            // case 4: compare the exact values
            let (mut lhs, mut rhs) =
                (self.numerator.clone(), other.significand() * &self.denominator);
            if other.exponent() < 0 {
                let exp = -other.exponent() as usize;
                if B.is_power_of_two() {
                    lhs <<= exp * B.trailing_zeros() as usize;
                } else {
                    lhs *= UBig::from_word(B).pow(exp);
                }
            } else {
                let exp = other.exponent() as usize;
                if B.is_power_of_two() {
                    rhs <<= exp * B.trailing_zeros() as usize;
                } else {
                    rhs *= UBig::from_word(B).pow(exp);
                }
            }
            lhs.cmp(&rhs)
        }
        #[inline]
        fn num_partial_cmp(&self, other: &FloatRepr<B>) -> Option<Ordering> {
            Some(self.num_cmp(other))
        }
    }

    impl<R: Round, const B: Word> NumOrd<FBig<R, B>> for RBig {
        #[inline]
        fn num_cmp(&self, other: &FBig<R, B>) -> Ordering {
            self.0.num_cmp(other.repr())
        }
        #[inline]
        fn num_partial_cmp(&self, other: &FBig<R, B>) -> Option<Ordering> {
            self.0.num_partial_cmp(other.repr())
        }
    }

    impl<R: Round, const B: Word> NumOrd<FBig<R, B>> for Relaxed {
        #[inline]
        fn num_cmp(&self, other: &FBig<R, B>) -> Ordering {
            self.0.num_cmp(other.repr())
        }
        #[inline]
        fn num_partial_cmp(&self, other: &FBig<R, B>) -> Option<Ordering> {
            self.0.num_partial_cmp(other.repr())
        }
    }
}
