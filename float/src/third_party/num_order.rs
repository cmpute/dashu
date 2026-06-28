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

impl<const B: Word> Repr<B> {
    /// The numeric-hash residue (mod 2¹²⁷−1) used by [`NumHash`]:
    /// `sgn(significand) · (|significand| mod M127) · (B^exponent mod M127)`.
    ///
    /// Special values: `+0` → `0`, `-0` → `0`, `+∞` → `HASH_INF` (= `M127`), `-∞` → `HASH_NEGINF`
    /// (= `-M127`), matching num-order's `f64::fhash`. The subsequent `i128::num_hash` maps both
    /// `HASH_INF` and `HASH_NEGINF` back to `0`, so the *final* hash of ±∞ is `0` — but the
    /// *residue* distinguishes them so that composite types (e.g. `CBig`) combine them algebraically
    /// the same way num-order's `Complex<f64>` does.
    pub fn num_hash_residue(&self) -> i128 {
        // 2^127 - 1 is used in the num-order crate
        type MInt = FixedMersenneInt<127, 1>;
        const M127: i128 = i128::MAX;
        const M127U: u128 = M127 as u128;

        if self.significand.is_zero() {
            // Distinguish infinities (sentinel exponents) from signed zero.
            return match self.exponent {
                isize::MAX => M127,          // +∞  → HASH_INF
                isize::MIN => i128::MIN + 1, // -∞  → HASH_NEGINF (= -M127)
                _ => 0,                      // ±0
            };
        }

        let signif_residue = &self.significand % M127;
        let signif_hash = MInt::new(signif_residue.unsigned_abs(), &M127U);
        let exp_hash = if B == 2 {
            signif_hash.convert(1 << self.exponent.absm(&127))
        } else if self.exponent < 0 {
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
        hash
    }
}

impl<const B: Word> NumHash for Repr<B> {
    #[inline]
    fn num_hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.num_hash_residue().num_hash(state)
    }
}

impl<R: Round, const B: Word> NumHash for FBig<R, B> {
    #[inline]
    fn num_hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.repr.num_hash(state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DBig;
    use core::cmp::Ordering;
    use num_order::{NumHash, NumOrd};

    /// Default binary FBig (Zero rounding, base 2).
    type FBin = FBig;

    /// Hash a `NumHash` value to u64 for comparison.
    fn num_hash<T: NumHash>(value: &T) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::Hasher;
        let mut hasher = DefaultHasher::new();
        value.num_hash(&mut hasher);
        hasher.finish()
    }

    /// Capture the i128 residue a `NumHash` impl writes (the `i128` NumHash writes its value via
    /// `Hasher::write_i128`), so the *field element* can be compared directly.
    fn residue<T: NumHash>(value: &T) -> i128 {
        struct Collector(i128);
        impl core::hash::Hasher for Collector {
            fn write_i128(&mut self, v: i128) {
                self.0 = v;
            }
            fn write(&mut self, _: &[u8]) {}
            fn finish(&self) -> u64 {
                0
            }
        }
        let mut c = Collector(0);
        value.num_hash(&mut c);
        c.0
    }

    // The base-2 Repr residue must equal num-order's f64 `fhash` for the same finite value — this
    // is what lets dashu-cmplx's CBig reuse Repr residues and stay in sync with num-order's
    // Complex<f64> hashing.
    #[test]
    fn test_fbig_num_hash_matches_f64() {
        for v in [
            1.0_f64,
            2.0,
            3.0,
            0.5,
            0.25,
            -0.75,
            100.0,
            1e-10,
            1e20,
            123.456,
            1.0 / 3.0,
            f64::INFINITY,
            f64::NEG_INFINITY,
            -0.0,
        ] {
            let f: FBin = core::convert::TryFrom::try_from(v).unwrap();
            assert_eq!(residue(&f), residue(&v), "FBig/f64 num_hash disagree for {v}");
        }
    }

    // -- NumOrd for Repr (same base) --

    #[test]
    fn test_num_ord_repr_zero_vs_neg_zero() {
        // +0 == -0 (IEEE 754)
        assert_eq!(Repr::<2>::zero().num_cmp(&Repr::<2>::neg_zero()), Ordering::Equal);
        assert_eq!(Repr::<2>::neg_zero().num_cmp(&Repr::<2>::zero()), Ordering::Equal);
    }

    #[test]
    fn test_num_ord_repr_neg_zero_vs_finite() {
        let one = Repr::<2>::one();
        let neg_one = Repr::<2>::neg_one();
        // -0 < positive
        assert_eq!(Repr::<2>::neg_zero().num_cmp(&one), Ordering::Less);
        // -0 > negative
        assert_eq!(Repr::<2>::neg_zero().num_cmp(&neg_one), Ordering::Greater);
    }

    #[test]
    fn test_num_ord_repr_infinities() {
        // +inf > -inf
        assert_eq!(Repr::<2>::infinity().num_cmp(&Repr::<2>::neg_infinity()), Ordering::Greater);
        // -inf < +inf
        assert_eq!(Repr::<2>::neg_infinity().num_cmp(&Repr::<2>::infinity()), Ordering::Less);
        // +inf == +inf
        assert_eq!(Repr::<2>::infinity().num_cmp(&Repr::<2>::infinity()), Ordering::Equal);
        // -inf == -inf
        assert_eq!(Repr::<2>::neg_infinity().num_cmp(&Repr::<2>::neg_infinity()), Ordering::Equal);
    }

    #[test]
    fn test_num_ord_repr_zero_vs_infinity() {
        // +0 < +inf
        assert_eq!(Repr::<2>::zero().num_cmp(&Repr::<2>::infinity()), Ordering::Less);
        // -0 < +inf
        assert_eq!(Repr::<2>::neg_zero().num_cmp(&Repr::<2>::infinity()), Ordering::Less);
        // +0 > -inf
        assert_eq!(Repr::<2>::zero().num_cmp(&Repr::<2>::neg_infinity()), Ordering::Greater);
        // -0 > -inf
        assert_eq!(Repr::<2>::neg_zero().num_cmp(&Repr::<2>::neg_infinity()), Ordering::Greater);
    }

    // -- NumOrd for Repr (cross-base) --

    #[test]
    fn test_num_ord_repr_cross_base_zero() {
        // Base-2 neg_zero == Base-10 zero
        assert_eq!(Repr::<2>::neg_zero().num_cmp(&Repr::<10>::zero()), Ordering::Equal);
        // Base-2 neg_zero == Base-10 neg_zero
        assert_eq!(Repr::<2>::neg_zero().num_cmp(&Repr::<10>::neg_zero()), Ordering::Equal);
    }

    #[test]
    fn test_num_ord_repr_cross_base_infinity() {
        // Base-2 +inf == Base-10 +inf
        assert_eq!(Repr::<2>::infinity().num_cmp(&Repr::<10>::infinity()), Ordering::Equal);
        // Base-2 +inf > Base-10 -inf
        assert_eq!(Repr::<2>::infinity().num_cmp(&Repr::<10>::neg_infinity()), Ordering::Greater);
        // Base-2 -inf == Base-10 -inf
        assert_eq!(Repr::<2>::neg_infinity().num_cmp(&Repr::<10>::neg_infinity()), Ordering::Equal);
    }

    // -- NumOrd for FBig --

    #[test]
    fn test_num_ord_fbig_neg_zero() {
        let negz: FBin = FBig::from_repr_const(Repr::<2>::neg_zero());
        let posz = FBin::ZERO;
        assert_eq!(negz.num_cmp(&posz), Ordering::Equal);
        assert_eq!(posz.num_cmp(&negz), Ordering::Equal);

        // -0 < +1, -0 > -1
        assert_eq!(negz.num_cmp(&FBin::ONE), Ordering::Less);
        assert_eq!(negz.num_cmp(&FBin::NEG_ONE), Ordering::Greater);
    }

    #[test]
    fn test_num_ord_fbig_cross_base_zero() {
        let negz: FBin = FBig::from_repr_const(Repr::<2>::neg_zero());
        assert_eq!(negz.num_cmp(&DBig::ZERO), Ordering::Equal);
        assert_eq!(DBig::ZERO.num_cmp(&negz), Ordering::Equal);
    }

    // -- NumHash for Repr --

    #[test]
    fn test_num_hash_repr_zero_neg_zero_equal() {
        // +0 and -0 compare equal, so they must hash the same
        assert_eq!(num_hash(&Repr::<2>::zero()), num_hash(&Repr::<2>::neg_zero()));
        assert_eq!(num_hash(&Repr::<10>::zero()), num_hash(&Repr::<10>::neg_zero()));
    }

    #[test]
    fn test_num_hash_repr_infinities_same_sign() {
        // Same-sign infinities hash the same
        assert_eq!(num_hash(&Repr::<2>::infinity()), num_hash(&Repr::<10>::infinity()));
        assert_eq!(num_hash(&Repr::<2>::neg_infinity()), num_hash(&Repr::<10>::neg_infinity()));
    }

    #[test]
    fn test_num_hash_repr_zero_matches_integer_zero() {
        // +0 and -0 should hash the same as integer zero
        assert_eq!(num_hash(&Repr::<2>::zero()), num_hash(&0i128));
        assert_eq!(num_hash(&Repr::<2>::neg_zero()), num_hash(&0i128));
    }

    // -- NumHash for FBig --

    #[test]
    fn test_num_hash_fbig_neg_zero() {
        let negz: FBin = FBig::from_repr_const(Repr::<2>::neg_zero());
        assert_eq!(num_hash(&negz), num_hash(&FBin::ZERO));
    }

    #[test]
    fn test_num_hash_fbig_cross_base_zero() {
        let negz: FBin = FBig::from_repr_const(Repr::<2>::neg_zero());
        assert_eq!(num_hash(&negz), num_hash(&DBig::ZERO));
    }
}
