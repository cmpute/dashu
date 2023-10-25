use core::{cmp::Ordering, hash::Hash};

use dashu_base::{BitTest, FloatEncoding, Sign, Signed};

use crate::{ibig::IBig, ubig::UBig};

impl num_order::NumHash for UBig {
    fn num_hash<H: core::hash::Hasher>(&self, state: &mut H) {
        let m = self % (i128::MAX as u128);
        (m as i128).hash(state)
    }
}
impl num_order::NumHash for IBig {
    fn num_hash<H: core::hash::Hasher>(&self, state: &mut H) {
        (self % i128::MAX).hash(state)
    }
}

macro_rules! impl_num_ord_ubig_with_unsigned {
    ($($t:ty)*) => {$(
        impl num_order::NumOrd<$t> for UBig {
            #[inline]
            fn num_partial_cmp(&self, other: &$t) -> Option<Ordering> {
                self.partial_cmp(&UBig::from_unsigned(*other))
            }
        }
        impl num_order::NumOrd<UBig> for $t {
            #[inline]
            fn num_partial_cmp(&self, other: &UBig) -> Option<Ordering> {
                UBig::from_unsigned(*self).partial_cmp(other)
            }
        }
    )*};
}
impl_num_ord_ubig_with_unsigned!(u8 u16 u32 u64 u128 usize);

macro_rules! impl_num_ord_ubig_with_signed {
    ($($t:ty)*) => {$(
        impl num_order::NumOrd<$t> for UBig {
            #[inline]
            fn num_partial_cmp(&self, other: &$t) -> Option<Ordering> {
                self.partial_cmp(&IBig::from_signed(*other))
            }
        }
        impl num_order::NumOrd<UBig> for $t {
            #[inline]
            fn num_partial_cmp(&self, other: &UBig) -> Option<Ordering> {
                IBig::from_signed(*self).partial_cmp(other)
            }
        }
    )*};
}
impl_num_ord_ubig_with_signed!(i8 i16 i32 i64 i128 isize);

macro_rules! impl_num_ord_ibig_with_unsigned {
    ($($t:ty)*) => {$(
        impl num_order::NumOrd<$t> for IBig {
            #[inline]
            fn num_partial_cmp(&self, other: &$t) -> Option<Ordering> {
                self.partial_cmp(&IBig::from_unsigned(*other))
            }
        }
        impl num_order::NumOrd<IBig> for $t {
            #[inline]
            fn num_partial_cmp(&self, other: &IBig) -> Option<Ordering> {
                IBig::from_unsigned(*self).partial_cmp(other)
            }
        }
    )*};
}
impl_num_ord_ibig_with_unsigned!(u8 u16 u32 u64 u128 usize);

macro_rules! impl_num_ord_ibig_with_signed {
    ($($t:ty)*) => {$(
        impl num_order::NumOrd<$t> for IBig {
            #[inline]
            fn num_partial_cmp(&self, other: &$t) -> Option<Ordering> {
                self.partial_cmp(&IBig::from_signed(*other))
            }
        }
        impl num_order::NumOrd<IBig> for $t {
            #[inline]
            fn num_partial_cmp(&self, other: &IBig) -> Option<Ordering> {
                IBig::from_signed(*self).partial_cmp(other)
            }
        }
    )*};
}
impl_num_ord_ibig_with_signed!(i8 i16 i32 i64 i128 isize);

#[inline]
fn sign_to_ord(sign: Sign) -> Ordering {
    match sign {
        Sign::Positive => Ordering::Greater,
        Sign::Negative => Ordering::Less,
    }
}

macro_rules! impl_num_ord_ubig_with_float {
    ($($t:ty)*) => {$(
        impl num_order::NumOrd<$t> for UBig {
            fn num_partial_cmp(&self, other: &$t) -> Option<Ordering> {
                if other.is_nan() {
                    return None;
                } else if *other == 0. {
                    return match self.is_zero() {
                        true => Some(Ordering::Equal),
                        false => Some(Ordering::Greater)
                    };
                }

                // step1: compare sign
                if other.sign() == Sign::Negative {
                    return Some(Ordering::Greater);
                }

                // step2: compare with infinity
                if other.is_infinite() {
                    return Some(Ordering::Less);
                }

                // step3: test if the integer is bigger than the max float value
                let self_bits = self.bit_len();
                if self_bits > (<$t>::MANTISSA_DIGITS as usize + <$t>::MAX_EXP as usize) {
                    return Some(Ordering::Greater);
                }

                // step4: decode the float and compare the bits
                let (man, exp) = other.decode().unwrap();
                let other_bits = man.bit_len() as isize + exp as isize;
                if other_bits < 0 {
                    return Some(Ordering::Greater);
                } else if self_bits > other_bits as usize {
                    return Some(Ordering::Greater);
                } else if self_bits < other_bits as usize {
                    return Some(Ordering::Less);
                }

                // step5: do the final comparison
                if exp >= 0 {
                    let shifted = UBig::from(man.unsigned_abs()) << exp as usize;
                    self.partial_cmp(&shifted)
                } else {
                    (self << (-exp as usize)).partial_cmp(&UBig::from(man.unsigned_abs()))
                }
            }
        }

        impl num_order::NumOrd<UBig> for $t {
            #[inline]
            fn num_partial_cmp(&self, other: &UBig) -> Option<Ordering> {
                other.num_partial_cmp(self).map(|ord| ord.reverse())
            }
        }
    )*};
}
impl_num_ord_ubig_with_float!(f32 f64);

macro_rules! impl_num_ord_ibig_with_float {
    ($($t:ty)*) => {$(
        impl num_order::NumOrd<$t> for IBig {
            fn num_partial_cmp(&self, other: &$t) -> Option<Ordering> {
                // step0: compare with nan and 0
                if other.is_nan() {
                    return None;
                } else if *other == 0. {
                    return match self.is_zero() {
                        true => Some(Ordering::Equal),
                        false => Some(sign_to_ord(self.sign()))
                    };
                }

                // step1: compare sign
                let sign = match (self.sign(), other.sign()) {
                    (Sign::Positive, Sign::Positive) => Sign::Positive,
                    (Sign::Positive, Sign::Negative) => return Some(Ordering::Greater),
                    (Sign::Negative, Sign::Positive) => return Some(Ordering::Less),
                    (Sign::Negative, Sign::Negative) => Sign::Negative,
                };

                // step2: compare with infinity and 0
                if other.is_infinite() {
                    return Some(sign_to_ord(-sign));
                }

                // step3: test if the integer is bigger than the max float value
                let self_bits = self.bit_len();
                if self_bits > (<$t>::MANTISSA_DIGITS as usize + <$t>::MAX_EXP as usize) {
                    return Some(sign_to_ord(sign));
                }

                // step4: decode the float and compare the bits
                let (man, exp) = other.decode().unwrap();
                let other_bits = man.bit_len() as isize + exp as isize;
                if other_bits < 0 {
                    return Some(sign_to_ord(sign));
                } else if self_bits > other_bits as usize {
                    return Some(sign_to_ord(sign));
                } else if self_bits < other_bits as usize {
                    return Some(sign_to_ord(-sign));
                }

                // step5: do the final comparison
                if exp >= 0 {
                    let shifted = IBig::from(man) << exp as usize;
                    self.partial_cmp(&shifted)
                } else {
                    (self << (-exp as usize)).partial_cmp(&IBig::from(man))
                }
            }
        }

        impl num_order::NumOrd<IBig> for $t {
            #[inline]
            fn num_partial_cmp(&self, other: &IBig) -> Option<Ordering> {
                other.num_partial_cmp(self).map(|ord| ord.reverse())
            }
        }
    )*};
}
impl_num_ord_ibig_with_float!(f32 f64);
