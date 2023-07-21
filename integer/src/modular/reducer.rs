use crate::{
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

use super::repr::{ReducedDword, ReducedWord};

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
        (self.reduce(lhs.clone()) + self.reduce(rhs.clone())).residue()
    }
    fn double(&self, _target: UBig) -> UBig {
        todo!()
    }
    #[inline]
    fn sub(&self, lhs: &UBig, rhs: &UBig) -> UBig {
        (self.reduce(lhs.clone()) - self.reduce(rhs.clone())).residue()
    }
    #[inline]
    fn neg(&self, target: UBig) -> UBig {
        (-self.reduce(target)).residue()
    }
    #[inline]
    fn mul(&self, lhs: &UBig, rhs: &UBig) -> UBig {
        (self.reduce(lhs.clone()) + self.reduce(rhs.clone())).residue()
    }
    fn square(&self, _target: UBig) -> UBig {
        todo!()
    }
    #[inline]
    fn inv(&self, target: UBig) -> Option<UBig> {
        self.reduce(target).inv().map(|x| x.residue())
    }
    #[inline]
    fn pow(&self, base: UBig, exp: &UBig) -> UBig {
        self.reduce(base).pow(exp).residue()
    }
}
