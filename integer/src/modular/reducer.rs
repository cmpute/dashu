use core::cmp::Ordering;

use crate::{
    div_const::{ConstDivisor, ConstDivisorRepr},
    helper_macros::debug_assert_zero,
    primitive::shrink_dword,
    repr::{Repr, TypedRepr, TypedReprRef},
    shift,
    ubig::UBig, cmp, math,
};
use num_modular::Reducer;

impl Reducer<UBig> for ConstDivisor {
    #[inline]
    fn new(m: &UBig) -> Self {
        ConstDivisor::new(m.clone())
    }

    fn transform(&self, target: UBig) -> UBig {
        UBig(match &self.0 {
            ConstDivisorRepr::Single(d) => Repr::from_word(match target.into_repr() {
                TypedRepr::Small(dword) => if let Some(word) = shrink_dword(dword) {
                    d.rem_word(word)
                } else {
                    d.rem_dword(dword)
                },
                TypedRepr::Large(words) => d.rem_large(&words),
            }),
            ConstDivisorRepr::Double(d) => Repr::from_dword(match target.into_repr() {
                TypedRepr::Small(dword) => d.rem_dword(dword),
                TypedRepr::Large(words) => d.rem_large(&words),
            }),
            ConstDivisorRepr::Large(d) => Repr::from_buffer(d.rem_repr(target.into_repr()))
        })
    }
    fn check(&self, target: &UBig) -> bool {
        match (&self.0, target.repr()) {
            (ConstDivisorRepr::Single(d), TypedReprRef::RefSmall(dw)) => match shrink_dword(dw) {
                Some(w) => d.0.check(&w),
                None => false
            },
            (ConstDivisorRepr::Single(_), TypedReprRef::RefLarge(_)) => false,
            (ConstDivisorRepr::Double(d), TypedReprRef::RefSmall(dw)) => d.0.check(&dw),
            (ConstDivisorRepr::Double(_), TypedReprRef::RefLarge(_)) => false,
            (ConstDivisorRepr::Large(_), TypedReprRef::RefSmall(_)) => true,
            (ConstDivisorRepr::Large(d), TypedReprRef::RefLarge(words)) => {
                cmp::cmp_in_place(words, &d.normalized_modulus) == Ordering::Less && 
                words[0] & math::ones_word(d.shift) == 0 // must be shifted
            }
        }
    }

    #[inline]
    fn modulus(&self) -> UBig {
        self.divisor()
    }

    fn residue(&self, target: UBig) -> UBig {
        UBig(match target.into_repr() {
            TypedRepr::Small(dw) => match &self.0 {
                ConstDivisorRepr::Single(d) => Repr::from_word(shrink_dword(dw).unwrap() >> d.0.shift()),
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

    fn add(&self, lhs: &UBig, rhs: &UBig) -> UBig {
        todo!()
    }
    fn add_in_place(&self, lhs: &mut UBig, rhs: &UBig) {
        todo!()
    }
    fn double(&self, target: UBig) -> UBig {
        todo!()
    }
    fn sub(&self, lhs: &UBig, rhs: &UBig) -> UBig {
        todo!()
    }
    fn sub_in_place(&self, lhs: &mut UBig, rhs: &UBig) {
        todo!()
    }
    fn neg(&self, target: UBig) -> UBig {
        todo!()
    }
    fn mul(&self, lhs: &UBig, rhs: &UBig) -> UBig {
        todo!()
    }
    fn mul_assign(&self, lhs: &mut UBig, rhs: &UBig) {
        todo!()
    }
    fn square(&self, target: UBig) -> UBig {
        todo!()
    }
    fn inv(&self, target: UBig) -> Option<UBig> {
        todo!()
    }
    fn pow(&self, base: UBig, exp: UBig) -> UBig {
        todo!()
    }
}
