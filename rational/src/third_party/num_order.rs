use crate::{
    cmp::{repr_cmp_ibig, repr_cmp_ubig},
    rbig::RBig,
    repr::Repr,
    Relaxed,
};
use _num_modular::{FixedMersenneInt, ModularInteger};
use core::cmp::Ordering;
use dashu_base::{BitTest, FloatEncoding, Sign, Signed};
use dashu_int::{IBig, UBig};
use num_order::{NumHash, NumOrd};

macro_rules! impl_ord_between_ratio {
    ($t1:ty, $t2:ty) => {
        impl NumOrd<$t2> for $t1 {
            #[inline]
            fn num_eq(&self, other: &$t2) -> bool {
                self.0.eq(&other.0)
            }
            #[inline]
            fn num_partial_cmp(&self, other: &$t2) -> Option<Ordering> {
                Some(self.0.cmp(&other.0))
            }
            #[inline]
            fn num_cmp(&self, other: &$t2) -> Ordering {
                self.0.cmp(&other.0)
            }
        }
    };
}
impl_ord_between_ratio!(RBig, Relaxed);
impl_ord_between_ratio!(Relaxed, RBig);

impl NumOrd<UBig> for Repr {
    #[inline]
    fn num_cmp(&self, other: &UBig) -> Ordering {
        repr_cmp_ubig::<false>(self, other)
    }
    #[inline]
    fn num_partial_cmp(&self, other: &UBig) -> Option<Ordering> {
        Some(repr_cmp_ubig::<false>(self, other))
    }
}

impl NumOrd<IBig> for Repr {
    #[inline]
    fn num_cmp(&self, other: &IBig) -> Ordering {
        repr_cmp_ibig::<false>(self, other)
    }
    #[inline]
    fn num_partial_cmp(&self, other: &IBig) -> Option<Ordering> {
        Some(repr_cmp_ibig::<false>(self, other))
    }
}

macro_rules! forward_num_ord_to_repr {
    ($R:ty, $T:ty) => {
        impl NumOrd<$T> for $R {
            #[inline]
            fn num_cmp(&self, other: &$T) -> Ordering {
                self.0.num_cmp(other)
            }
            #[inline]
            fn num_partial_cmp(&self, other: &$T) -> Option<Ordering> {
                self.0.num_partial_cmp(other)
            }
        }
        impl NumOrd<$R> for $T {
            #[inline]
            fn num_cmp(&self, other: &$R) -> Ordering {
                other.0.num_cmp(self).reverse()
            }
            #[inline]
            fn num_partial_cmp(&self, other: &$R) -> Option<Ordering> {
                other.0.num_partial_cmp(self).map(|ord| ord.reverse())
            }
        }
    };
}
forward_num_ord_to_repr!(RBig, UBig);
forward_num_ord_to_repr!(Relaxed, UBig);
forward_num_ord_to_repr!(RBig, IBig);
forward_num_ord_to_repr!(Relaxed, IBig);

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
            return if self.numerator.is_positive() {
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
    use crate::cmp::with_float::repr_cmp_fbig;
    use dashu_float::{round::Round, FBig, Repr as FloatRepr};
    use dashu_int::Word;

    impl<const B: Word> NumOrd<FloatRepr<B>> for Repr {
        #[inline]
        fn num_cmp(&self, other: &FloatRepr<B>) -> Ordering {
            repr_cmp_fbig::<B, false>(self, other)
        }
        #[inline]
        fn num_partial_cmp(&self, other: &FloatRepr<B>) -> Option<Ordering> {
            Some(repr_cmp_fbig::<B, false>(self, other))
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

    impl<R: Round, const B: Word> NumOrd<RBig> for FBig<R, B> {
        #[inline]
        fn num_cmp(&self, other: &RBig) -> Ordering {
            other.0.num_cmp(self.repr()).reverse()
        }
        #[inline]
        fn num_partial_cmp(&self, other: &RBig) -> Option<Ordering> {
            Some(self.num_cmp(other))
        }
    }

    impl<R: Round, const B: Word> NumOrd<Relaxed> for FBig<R, B> {
        #[inline]
        fn num_cmp(&self, other: &Relaxed) -> Ordering {
            other.0.num_cmp(self.repr()).reverse()
        }
        #[inline]
        fn num_partial_cmp(&self, other: &Relaxed) -> Option<Ordering> {
            Some(self.num_cmp(other))
        }
    }
}

macro_rules! impl_num_ord_with_unsigned {
    ($($t:ty)*) => {$(
        impl NumOrd<$t> for Repr {
            #[inline]
            fn num_cmp(&self, other: &$t) -> Ordering {
                repr_cmp_ubig::<false>(self, &UBig::from(*other))
            }
            #[inline]
            fn num_partial_cmp(&self, other: &$t) -> Option<Ordering> {
                Some(repr_cmp_ubig::<false>(self, &UBig::from(*other)))
            }
        }
        forward_num_ord_to_repr!(RBig, $t);
        forward_num_ord_to_repr!(Relaxed, $t);
    )*};
}
impl_num_ord_with_unsigned!(u8 u16 u32 u64 u128 usize);

macro_rules! impl_num_ord_with_signed {
    ($($t:ty)*) => {$(
        impl NumOrd<$t> for Repr {
            #[inline]
            fn num_cmp(&self, other: &$t) -> Ordering {
                repr_cmp_ibig::<false>(self, &IBig::from(*other))
            }
            #[inline]
            fn num_partial_cmp(&self, other: &$t) -> Option<Ordering> {
                Some(repr_cmp_ibig::<false>(self, &IBig::from(*other)))
            }
        }
        forward_num_ord_to_repr!(RBig, $t);
        forward_num_ord_to_repr!(Relaxed, $t);
    )*};
}
impl_num_ord_with_signed!(i8 i16 i32 i64 i128 isize);

macro_rules! impl_num_ord_with_float {
    ($($t:ty)*) => {$(
        impl NumOrd<$t> for Repr {
            fn num_partial_cmp(&self, other: &$t) -> Option<Ordering> {
                // step 1: compare with nan/inf/0
                if other.is_nan() {
                    return None;
                } else if other.is_infinite() {
                    return match other.sign() {
                        Sign::Positive => Some(Ordering::Less),
                        Sign::Negative => Some(Ordering::Greater),
                    };
                } else if *other == 0. {
                    return match self.numerator.is_zero() {
                        true => Some(Ordering::Equal),
                        false => Some(self.numerator.sign() * Ordering::Greater)
                    };
                }

                // step 2: compare sign
                let sign = match (self.numerator.sign(), other.sign()) {
                    (Sign::Positive, Sign::Positive) => Sign::Positive,
                    (Sign::Positive, Sign::Negative) => return Some(Ordering::Greater),
                    (Sign::Negative, Sign::Positive) => return Some(Ordering::Less),
                    (Sign::Negative, Sign::Negative) => Sign::Negative,
                };

                // step 3: test if the number is bigger than the max float value
                // Here we don't use EstimatedLog2, since a direct comparison is not that expensive.
                // We just need a quick way to determine if one number is much larger than the other.
                // The bit length (essentially ⌊log2(x)⌋ + 1) is used instead here.
                let self_log2 = self.numerator.bit_len() as isize - self.denominator.bit_len() as isize;
                let (self_log2_lb, self_log2_ub) = (self_log2 - 1, self_log2 + 1);
                if self_log2_lb > (<$t>::MANTISSA_DIGITS as isize + <$t>::MAX_EXP as isize) {
                    return Some(sign * Ordering::Greater);
                }

                // step 4: decode the float and compare the bits
                let (other_man, other_exp) = other.decode().unwrap();
                let other_log2 = other_man.bit_len() as isize + other_exp as isize - 1;
                if self_log2_lb > other_log2 {
                    return Some(sign * Ordering::Greater);
                } else if self_log2_ub < other_log2 {
                    return Some(sign * Ordering::Less);
                }

                // step 5: compare the exact values
                let (other_man, other_exp) = other.decode().unwrap();
                let (mut lhs, mut rhs) = (self.numerator.clone(), IBig::from(other_man) * &self.denominator);
                if other_exp < 0 {
                    lhs <<= -other_exp as usize;
                } else {
                    rhs <<= other_exp as usize;
                }

                Some(lhs.cmp(&rhs))
            }
        }
    )*};
}
impl_num_ord_with_float!(f32 f64);
forward_num_ord_to_repr!(RBig, f32);
forward_num_ord_to_repr!(Relaxed, f32);
forward_num_ord_to_repr!(RBig, f64);
forward_num_ord_to_repr!(Relaxed, f64);
