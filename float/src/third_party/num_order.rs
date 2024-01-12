use _num_modular::{FixedMersenneInt, ModularAbs, ModularInteger};
use core::cmp::Ordering;
use dashu_base::{BitTest, EstimatedLog2, FloatEncoding, Sign, Signed};
use dashu_int::{IBig, UBig, Word};
use num_order::{NumHash, NumOrd};

use crate::{
    cmp::{repr_cmp_ibig, repr_cmp_ubig},
    fbig::FBig,
    repr::Repr,
    round::Round,
    utils::shl_digits_in_place,
};

impl<const B1: Word, const B2: Word> NumOrd<Repr<B2>> for Repr<B1> {
    fn num_cmp(&self, other: &Repr<B2>) -> Ordering {
        // case 1: compare with inf
        match (self.is_infinite(), other.is_infinite()) {
            (true, true) => return self.exponent.cmp(&other.exponent),
            (false, true) => {
                return match other.exponent >= 0 {
                    true => Ordering::Less,
                    false => Ordering::Greater,
                }
            }
            (true, false) => {
                return match self.exponent >= 0 {
                    true => Ordering::Greater,
                    false => Ordering::Less,
                }
            }
            _ => {}
        };

        // case 2: compare sign
        let sign = match (self.significand.sign(), other.significand.sign()) {
            (Sign::Positive, Sign::Positive) => Sign::Positive,
            (Sign::Positive, Sign::Negative) => return Ordering::Greater,
            (Sign::Negative, Sign::Positive) => return Ordering::Less,
            (Sign::Negative, Sign::Negative) => Sign::Negative,
        };

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
        let (mut lhs, mut rhs) = (self.significand.clone(), other.significand.clone());
        if self.exponent < 0 {
            shl_digits_in_place::<B1>(&mut rhs, (-self.exponent) as usize);
        } else {
            shl_digits_in_place::<B1>(&mut lhs, self.exponent as usize);
        }
        if other.exponent < 0 {
            shl_digits_in_place::<B2>(&mut lhs, (-other.exponent) as usize);
        } else {
            shl_digits_in_place::<B2>(&mut rhs, other.exponent as usize);
        }
        lhs.cmp(&rhs)
    }
    #[inline]
    fn num_partial_cmp(&self, other: &Repr<B2>) -> Option<Ordering> {
        Some(self.num_cmp(other))
    }
}

impl<R1: Round, R2: Round, const B1: Word, const B2: Word> NumOrd<FBig<R2, B2>> for FBig<R1, B1> {
    #[inline]
    fn num_cmp(&self, other: &FBig<R2, B2>) -> Ordering {
        self.repr.num_cmp(&other.repr)
    }
    #[inline]
    fn num_partial_cmp(&self, other: &FBig<R2, B2>) -> Option<Ordering> {
        self.repr.num_partial_cmp(&other.repr)
    }
}

macro_rules! impl_num_ord_with_method {
    ($T:ty, $method:ident) => {
        impl<const B: Word> NumOrd<$T> for Repr<B> {
            #[inline]
            fn num_cmp(&self, other: &$T) -> Ordering {
                $method::<B, false>(self, other)
            }
            #[inline]
            fn num_partial_cmp(&self, other: &$T) -> Option<Ordering> {
                Some($method::<B, false>(self, other))
            }
        }
        impl<const B: Word> NumOrd<Repr<B>> for $T {
            #[inline]
            fn num_cmp(&self, other: &Repr<B>) -> Ordering {
                $method::<B, false>(other, self).reverse()
            }
            #[inline]
            fn num_partial_cmp(&self, other: &Repr<B>) -> Option<Ordering> {
                Some($method::<B, false>(other, self).reverse())
            }
        }
    };
}
impl_num_ord_with_method!(UBig, repr_cmp_ubig);
impl_num_ord_with_method!(IBig, repr_cmp_ibig);

macro_rules! forward_num_ord_to_repr {
    ($t:ty) => {
        impl<R: Round, const B: Word> NumOrd<$t> for FBig<R, B> {
            #[inline]
            fn num_cmp(&self, other: &$t) -> Ordering {
                self.repr.num_cmp(other)
            }
            #[inline]
            fn num_partial_cmp(&self, other: &$t) -> Option<Ordering> {
                self.repr.num_partial_cmp(other)
            }
        }

        impl<R: Round, const B: Word> NumOrd<FBig<R, B>> for $t {
            #[inline]
            fn num_cmp(&self, other: &FBig<R, B>) -> Ordering {
                self.num_cmp(&other.repr)
            }
            #[inline]
            fn num_partial_cmp(&self, other: &FBig<R, B>) -> Option<Ordering> {
                self.num_partial_cmp(&other.repr)
            }
        }
    };
}
forward_num_ord_to_repr!(UBig);
forward_num_ord_to_repr!(IBig);

macro_rules! impl_num_ord_fbig_unsigned {
    ($($t:ty)*) => {$(
        impl<const B: Word> NumOrd<$t> for Repr<B> {
            #[inline]
            fn num_partial_cmp(&self, other: &$t) -> Option<Ordering> {
                Some(repr_cmp_ubig::<B, false>(self, &UBig::from(*other)))
            }
        }
        impl<const B: Word> NumOrd<Repr<B>> for $t {
            #[inline]
            fn num_partial_cmp(&self, other: &Repr<B>) -> Option<Ordering> {
                Some(repr_cmp_ubig::<B, false>(other, &UBig::from(*self)).reverse())
            }
        }
        impl<R: Round, const B: Word> NumOrd<$t> for FBig<R, B> {
            #[inline]
            fn num_partial_cmp(&self, other: &$t) -> Option<Ordering> {
                Some(repr_cmp_ubig::<B, false>(&self.repr, &UBig::from(*other)))
            }
        }
        impl<R: Round, const B: Word> NumOrd<FBig<R, B>> for $t {
            #[inline]
            fn num_partial_cmp(&self, other: &FBig<R, B>) -> Option<Ordering> {
                Some(repr_cmp_ubig::<B, false>(&other.repr, &UBig::from(*self)).reverse())
            }
        }
    )*};
}
impl_num_ord_fbig_unsigned!(u8 u16 u32 u64 u128 usize);

macro_rules! impl_num_ord_with_signed {
    ($($t:ty)*) => {$(
        impl<const B: Word> NumOrd<$t> for Repr<B> {
            #[inline]
            fn num_partial_cmp(&self, other: &$t) -> Option<Ordering> {
                Some(repr_cmp_ibig::<B, false>(self, &IBig::from(*other)))
            }
        }
        impl<const B: Word> NumOrd<Repr<B>> for $t {
            #[inline]
            fn num_partial_cmp(&self, other: &Repr<B>) -> Option<Ordering> {
                Some(repr_cmp_ibig::<B, false>(other, &IBig::from(*self)).reverse())
            }
        }
        impl<R: Round, const B: Word> NumOrd<$t> for FBig<R, B> {
            #[inline]
            fn num_partial_cmp(&self, other: &$t) -> Option<Ordering> {
                Some(repr_cmp_ibig::<B, false>(&self.repr, &IBig::from(*other)))
            }
        }
        impl<R: Round, const B: Word> NumOrd<FBig<R, B>> for $t {
            #[inline]
            fn num_partial_cmp(&self, other: &FBig<R, B>) -> Option<Ordering> {
                Some(repr_cmp_ibig::<B, false>(&other.repr, &IBig::from(*self)).reverse())
            }
        }
    )*};
}
impl_num_ord_with_signed!(i8 i16 i32 i64 i128 isize);

macro_rules! impl_num_ord_with_float {
    ($($t:ty)*) => {$(
        impl<const B: Word> NumOrd<$t> for Repr<B> {
            #[inline]
            fn num_partial_cmp(&self, other: &$t) -> Option<Ordering> {
                // step0: compare with nan and 0
                if other.is_nan() {
                    return None;
                } else if *other == 0. {
                    return match self.is_zero() {
                        true => Some(Ordering::Equal),
                        false => Some(self.sign() * Ordering::Greater)
                    };
                }

                // step1: compare sign
                let sign = match (self.sign(), other.sign()) {
                    (Sign::Positive, Sign::Positive) => Sign::Positive,
                    (Sign::Positive, Sign::Negative) => return Some(Ordering::Greater),
                    (Sign::Negative, Sign::Positive) => return Some(Ordering::Less),
                    (Sign::Negative, Sign::Negative) => Sign::Negative,
                };

                // step2: compare with inf
                match (self.is_infinite(), other.is_infinite()) {
                    (true, true) => return Some(Ordering::Equal),
                    (false, true) => return Some(sign * Ordering::Less),
                    (true, false) => return Some(sign * Ordering::Greater),
                    _ => {}
                };

                // step3: test if the number is bigger than the max float value
                // Here we don't use EstimatedLog2, since a direct comparison is not that expensive.
                // We just need a quick way to determine if one number is much larger than the other.
                // The bit length (essentially ⌊log2(x)⌋ + 1) is used instead here.
                let self_signif_log2 = self.significand.bit_len() as isize;
                let self_log2 = self_signif_log2 + B.bit_len() as isize * self.exponent;
                let (self_log2_lb, self_log2_ub) = if self.exponent >= 0 {
                    (self_log2 - self.exponent, self_log2)
                } else {
                    (self_log2, self_log2 - self.exponent)
                };
                if self_log2_lb > (<$t>::MANTISSA_DIGITS as isize + <$t>::MAX_EXP as isize) {
                    return Some(sign * Ordering::Greater);
                }

                // step4: decode the float and compare the bits
                let (other_signif, other_exp) = other.decode().unwrap();
                let other_log2 = other_signif.bit_len() as isize + other_exp as isize;
                if self_log2_lb > other_log2 {
                    return Some(sign * Ordering::Greater);
                } else if self_log2_ub < other_log2 {
                    return Some(sign * Ordering::Less);
                }

                // step5: do the final comparison
                let (mut lhs, mut rhs) = (self.significand.clone(), IBig::from(other_signif));
                if self.exponent < 0 {
                    shl_digits_in_place::<B>(&mut rhs, (-self.exponent) as usize);
                } else {
                    shl_digits_in_place::<B>(&mut lhs, self.exponent as usize);
                }
                if other_exp < 0 {
                    lhs <<= (-other_exp) as usize;
                } else {
                    rhs <<= other_exp as usize;
                }
                Some(lhs.cmp(&rhs))
            }
        }

        impl<const B: Word> NumOrd<Repr<B>> for $t {
            #[inline]
            fn num_partial_cmp(&self, other: &Repr<B>) -> Option<Ordering> {
                other.num_partial_cmp(self).map(|ord| ord.reverse())
            }
        }
    )*};
}
impl_num_ord_with_float!(f32 f64);
forward_num_ord_to_repr!(f32);
forward_num_ord_to_repr!(f64);

impl<const B: Word> NumHash for Repr<B> {
    fn num_hash<H: core::hash::Hasher>(&self, state: &mut H) {
        // 2^127 - 1 is used in the num-order crate
        type MInt = FixedMersenneInt<127, 1>;
        const M127: i128 = i128::MAX;
        const M127U: u128 = M127 as u128;

        let signif_residue = &self.significand % M127;
        let signif_hash = MInt::new(signif_residue.unsigned_abs(), &M127U);
        let exp_hash = if B == 2 {
            signif_hash.convert(1 << self.exponent.absm(&127))
        } else if self.exponent < 0 {
            // since a Word is at most 64 bits right now, B is always less than M127
            signif_hash
                .convert(B as u128)
                .pow(&(-self.exponent as u128))
                .inv()
                .unwrap()
        } else {
            signif_hash.convert(B as u128).pow(&(self.exponent as u128))
        };

        let mut hash = (signif_hash * exp_hash).residue() as i128;
        if signif_residue < 0 {
            hash = -hash;
        }

        hash.num_hash(state)
    }
}

impl<R: Round, const B: Word> NumHash for FBig<R, B> {
    #[inline]
    fn num_hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.repr.num_hash(state)
    }
}
