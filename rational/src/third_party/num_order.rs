use crate::{
    cmp::{repr_cmp_ibig, repr_cmp_ubig},
    rbig::RBig,
    repr::Repr,
    Relaxed,
};
use _num_modular::{FixedMersenneInt, ModularInteger};
use core::cmp::Ordering;
use dashu_base::Signed;
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
                Some(self.0.num_cmp(other))
            }
        }
        impl NumOrd<$R> for $T {
            #[inline]
            fn num_cmp(&self, other: &$R) -> Ordering {
                other.0.num_cmp(self).reverse()
            }
            #[inline]
            fn num_partial_cmp(&self, other: &$R) -> Option<Ordering> {
                Some(other.0.num_cmp(self).reverse())
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

// TODO(next): implement NumOrd between RBig and primitives
