use crate::{
    add_ops::repr::{sub_large, sub_large_dword, sub_large_ref_val},
    buffer::Buffer,
    cmp,
    div_const::{ConstDivisor, ConstDivisorRepr},
    helper_macros::debug_assert_zero,
    math,
    primitive::shrink_dword,
    repr::{Repr, TypedRepr, TypedReprRef},
    shift,
    ubig::UBig,
};
use num_modular::Reducer;

use super::{
    repr::{ReducedDword, ReducedLarge, ReducedRepr, ReducedWord},
    Reduced,
};

impl ConstDivisor {
    /// If target is larger than the normalized divisor, then subtract it once.
    fn reduce_once(&self, target: UBig) -> UBig {
        if !self.check(&target) {
            match &self.0 {
                ConstDivisorRepr::Single(d) => target - d.normalized_divisor(),
                ConstDivisorRepr::Double(d) => target - d.normalized_divisor(),
                ConstDivisorRepr::Large(d) => {
                    match target.into_repr() {
                        TypedRepr::Small(s) => UBig::from_dword(s), // no need to reduce
                        TypedRepr::Large(s) => UBig(sub_large(s, &d.normalized_divisor)),
                    }
                }
            }
        } else {
            target
        }
    }

    /// Reduce -target
    fn reduce_negate(&self, target: UBig) -> UBig {
        match &self.0 {
            ConstDivisorRepr::Single(d) => d.normalized_divisor() - target,
            ConstDivisorRepr::Double(d) => d.normalized_divisor() - target,
            ConstDivisorRepr::Large(d) => match target.into_repr() {
                TypedRepr::Small(s) => {
                    UBig(sub_large_dword(d.normalized_divisor.as_ref().into(), s))
                }
                TypedRepr::Large(s) => UBig(sub_large_ref_val(&d.normalized_divisor, s)),
            },
        }
    }

    fn convert_from_normalized(&self, target: &UBig) -> Reduced {
        match &self.0 {
            ConstDivisorRepr::Single(d) => {
                Reduced::from_single(ReducedWord(target.try_into().unwrap()), d)
            }
            ConstDivisorRepr::Double(d) => {
                Reduced::from_double(ReducedDword(target.try_into().unwrap()), d)
            }
            ConstDivisorRepr::Large(d) => {
                let mut buf = Buffer::allocate_exact(d.normalized_divisor.len());
                let words = target.as_words();
                buf.push_slice(words);
                buf.push_zeros(d.normalized_divisor.len() - words.len());
                Reduced::from_large(ReducedLarge(buf.into_boxed_slice()), d)
            }
        }
    }

    fn convert_to_normalized(&self, target: Reduced) -> UBig {
        // this function is for internal use only!
        // it's not checked whether `target` is using `self` as the ring!
        match target.repr() {
            ReducedRepr::Single(raw, _) => UBig(Repr::from_word(raw.0)),
            ReducedRepr::Double(raw, _) => UBig(Repr::from_dword(raw.0)),
            ReducedRepr::Large(raw, _) => {
                let mut buf = Buffer::from(raw.0.as_ref());
                buf.pop_zeros();
                UBig(Repr::from_buffer(buf))
            }
        }
    }
}

impl Reducer<UBig> for ConstDivisor {
    #[inline]
    fn new(m: &UBig) -> Self {
        ConstDivisor::new(m.clone())
    }

    fn transform(&self, target: UBig) -> UBig {
        UBig(match &self.0 {
            ConstDivisorRepr::Single(d) => Repr::from_word(ReducedWord::from_ubig(&target, d).0),
            ConstDivisorRepr::Double(d) => Repr::from_dword(ReducedDword::from_ubig(&target, d).0),
            ConstDivisorRepr::Large(d) => Repr::from_buffer(d.rem_repr(target.into_repr())),
        })
    }
    fn check(&self, target: &UBig) -> bool {
        // check whether target < self.divisor()
        match (&self.0, target.repr()) {
            (ConstDivisorRepr::Single(d), TypedReprRef::RefSmall(dw)) => match shrink_dword(dw) {
                Some(w) => d.0.check(&w),
                None => false,
            },
            (ConstDivisorRepr::Single(_), TypedReprRef::RefLarge(_)) => false,
            (ConstDivisorRepr::Double(d), TypedReprRef::RefSmall(dw)) => d.0.check(&dw),
            (ConstDivisorRepr::Double(_), TypedReprRef::RefLarge(_)) => false,
            (ConstDivisorRepr::Large(_), TypedReprRef::RefSmall(_)) => true,
            (ConstDivisorRepr::Large(d), TypedReprRef::RefLarge(words)) => {
                cmp::cmp_in_place(words, &d.normalized_divisor).is_le()
                    && words[0] & math::ones_word(d.shift) == 0 // must be shifted
            }
        }
    }

    #[inline]
    fn modulus(&self) -> UBig {
        self.value()
    }

    fn residue(&self, target: UBig) -> UBig {
        UBig(match target.into_repr() {
            TypedRepr::Small(dw) => match &self.0 {
                ConstDivisorRepr::Single(d) => {
                    Repr::from_word(shrink_dword(dw).unwrap() >> d.0.shift())
                }
                ConstDivisorRepr::Double(d) => Repr::from_dword(dw >> d.0.shift()),
                ConstDivisorRepr::Large(d) => Repr::from_dword(dw >> d.shift),
            },
            TypedRepr::Large(mut buffer) => {
                if let ConstDivisorRepr::Large(d) = &self.0 {
                    debug_assert_zero!(shift::shr_in_place(&mut buffer, d.shift));
                    Repr::from_buffer(buffer)
                } else {
                    unreachable!()
                }
            }
        })
    }

    #[inline(always)]
    fn is_zero(&self, target: &UBig) -> bool {
        target.is_zero()
    }

    #[inline]
    fn add(&self, lhs: &UBig, rhs: &UBig) -> UBig {
        self.reduce_once(lhs + rhs)
    }
    #[inline]
    fn dbl(&self, target: UBig) -> UBig {
        self.reduce_once(target << 1)
    }
    #[inline]
    fn sub(&self, lhs: &UBig, rhs: &UBig) -> UBig {
        if lhs >= rhs {
            lhs - rhs
        } else {
            self.reduce_negate(rhs - lhs)
        }
    }
    #[inline]
    fn neg(&self, target: UBig) -> UBig {
        if target.is_zero() {
            target
        } else {
            self.reduce_negate(target)
        }
    }

    // for the following operations, copying is relatively cheap and the implementations of
    // the `Reduced` type are relied on.

    #[inline]
    fn mul(&self, lhs: &UBig, rhs: &UBig) -> UBig {
        let lhs = self.convert_from_normalized(lhs);
        let rhs = self.convert_from_normalized(rhs);
        self.convert_to_normalized(lhs * rhs)
    }
    #[inline]
    fn sqr(&self, target: UBig) -> UBig {
        self.convert_to_normalized(self.convert_from_normalized(&target).sqr())
    }
    #[inline]
    fn inv(&self, target: UBig) -> Option<UBig> {
        self.convert_from_normalized(&target)
            .inv()
            .map(|v| self.convert_to_normalized(v))
    }
    #[inline]
    fn pow(&self, base: UBig, exp: &UBig) -> UBig {
        self.convert_to_normalized(self.convert_from_normalized(&base).pow(exp))
    }
}
