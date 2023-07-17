//! Modular addition and subtraction.

use super::repr::{Reduced, ReducedDword, ReducedLarge, ReducedRepr, ReducedWord};
use crate::{add, cmp, div_const::ConstLargeDivisor, error::panic_different_rings};
use core::ops::{Add, AddAssign, Neg, Sub, SubAssign};
use num_modular::Reducer;

impl<'a> Neg for Reduced<'a> {
    type Output = Reduced<'a>;

    #[inline]
    fn neg(self) -> Reduced<'a> {
        match self.into_repr() {
            ReducedRepr::Single(raw, ring) => {
                Self::from_single(ReducedWord(ring.0.neg(raw.0)), ring)
            }
            ReducedRepr::Double(raw, ring) => {
                Self::from_double(ReducedDword(ring.0.neg(raw.0)), ring)
            }
            ReducedRepr::Large(mut raw, ring) => {
                negate_in_place(ring, &mut raw);
                Self::from_large(raw, ring)
            }
        }
    }
}

impl<'a> Neg for &Reduced<'a> {
    type Output = Reduced<'a>;

    #[inline]
    fn neg(self) -> Reduced<'a> {
        self.clone().neg()
    }
}

impl<'a> Add<Reduced<'a>> for Reduced<'a> {
    type Output = Reduced<'a>;

    #[inline]
    fn add(self, rhs: Reduced<'a>) -> Reduced<'a> {
        self.add(&rhs)
    }
}

impl<'a> Add<&Reduced<'a>> for Reduced<'a> {
    type Output = Reduced<'a>;

    #[inline]
    fn add(mut self, rhs: &Reduced<'a>) -> Reduced<'a> {
        self.add_assign(rhs);
        self
    }
}

impl<'a> Add<Reduced<'a>> for &Reduced<'a> {
    type Output = Reduced<'a>;

    #[inline]
    fn add(self, rhs: Reduced<'a>) -> Reduced<'a> {
        rhs.add(self)
    }
}

impl<'a> Add<&Reduced<'a>> for &Reduced<'a> {
    type Output = Reduced<'a>;

    #[inline]
    fn add(self, rhs: &Reduced<'a>) -> Reduced<'a> {
        self.clone().add(rhs)
    }
}

impl<'a> AddAssign<Reduced<'a>> for Reduced<'a> {
    #[inline]
    fn add_assign(&mut self, rhs: Reduced<'a>) {
        self.add_assign(&rhs)
    }
}

impl<'a> AddAssign<&Reduced<'a>> for Reduced<'a> {
    #[inline]
    fn add_assign(&mut self, rhs: &Reduced<'a>) {
        match (self.repr_mut(), rhs.repr()) {
            (ReducedRepr::Single(raw0, ring), ReducedRepr::Single(raw1, ring1)) => {
                Reduced::check_same_ring_single(ring, ring1);
                ring.0.add_in_place(&mut raw0.0, &raw1.0);
            }
            (ReducedRepr::Double(raw0, ring), ReducedRepr::Double(raw1, ring1)) => {
                Reduced::check_same_ring_double(ring, ring1);
                ring.0.add_in_place(&mut raw0.0, &raw1.0);
            }
            (ReducedRepr::Large(raw0, ring), ReducedRepr::Large(raw1, ring1)) => {
                Reduced::check_same_ring_large(ring, ring1);
                add_in_place(ring, raw0, raw1);
            }
            _ => panic_different_rings(),
        }
    }
}

impl<'a> Sub<Reduced<'a>> for Reduced<'a> {
    type Output = Reduced<'a>;

    #[inline]
    fn sub(self, rhs: Reduced<'a>) -> Reduced<'a> {
        self.sub(&rhs)
    }
}

impl<'a> Sub<&Reduced<'a>> for Reduced<'a> {
    type Output = Reduced<'a>;

    #[inline]
    fn sub(mut self, rhs: &Reduced<'a>) -> Reduced<'a> {
        self.sub_assign(rhs);
        self
    }
}

impl<'a> Sub<Reduced<'a>> for &Reduced<'a> {
    type Output = Reduced<'a>;

    #[inline]
    fn sub(self, mut rhs: Reduced<'a>) -> Reduced<'a> {
        match (self.repr(), rhs.repr_mut()) {
            (ReducedRepr::Single(raw0, ring), ReducedRepr::Single(raw1, ring1)) => {
                Reduced::check_same_ring_single(ring, ring1);
                raw1.0 = ring.0.sub(&raw0.0, &raw1.0);
            }
            (ReducedRepr::Double(raw0, ring), ReducedRepr::Double(raw1, ring1)) => {
                Reduced::check_same_ring_double(ring, ring1);
                raw1.0 = ring.0.sub(&raw0.0, &raw1.0);
            }
            (ReducedRepr::Large(raw0, ring), ReducedRepr::Large(raw1, ring1)) => {
                Reduced::check_same_ring_large(ring, ring1);
                sub_in_place_swap(ring, raw0, raw1);
            }
            _ => panic_different_rings(),
        }
        rhs
    }
}

impl<'a> Sub<&Reduced<'a>> for &Reduced<'a> {
    type Output = Reduced<'a>;

    #[inline]
    fn sub(self, rhs: &Reduced<'a>) -> Reduced<'a> {
        self.clone().sub(rhs)
    }
}

impl<'a> SubAssign<Reduced<'a>> for Reduced<'a> {
    #[inline]
    fn sub_assign(&mut self, rhs: Reduced<'a>) {
        self.sub_assign(&rhs)
    }
}

impl<'a> SubAssign<&Reduced<'a>> for Reduced<'a> {
    #[inline]
    fn sub_assign(&mut self, rhs: &Reduced<'a>) {
        match (self.repr_mut(), rhs.repr()) {
            (ReducedRepr::Single(raw0, ring), ReducedRepr::Single(raw1, ring1)) => {
                Reduced::check_same_ring_single(ring, ring1);
                ring.0.sub_in_place(&mut raw0.0, &raw1.0);
            }
            (ReducedRepr::Double(raw0, ring), ReducedRepr::Double(raw1, ring1)) => {
                Reduced::check_same_ring_double(ring, ring1);
                ring.0.sub_in_place(&mut raw0.0, &raw1.0);
            }
            (ReducedRepr::Large(raw0, ring), ReducedRepr::Large(raw1, ring1)) => {
                Reduced::check_same_ring_large(ring, ring1);
                sub_in_place(ring, raw0, raw1);
            }
            _ => panic_different_rings(),
        }
    }
}

pub(crate) fn negate_in_place(ring: &ConstLargeDivisor, raw: &mut ReducedLarge) {
    debug_assert!(raw.is_valid(ring));
    if !raw.0.iter().all(|w| *w == 0) {
        let overflow = add::sub_same_len_in_place_swap(&ring.normalized_divisor, &mut raw.0);
        debug_assert!(!overflow);
    }
}

fn add_in_place(ring: &ConstLargeDivisor, lhs: &mut ReducedLarge, rhs: &ReducedLarge) {
    debug_assert!(lhs.is_valid(ring) && rhs.is_valid(ring));
    let modulus = &ring.normalized_divisor;
    let overflow = add::add_same_len_in_place(&mut lhs.0, &rhs.0);
    if overflow || cmp::cmp_same_len(&lhs.0, modulus).is_ge() {
        let overflow2 = add::sub_same_len_in_place(&mut lhs.0, modulus);
        debug_assert_eq!(overflow, overflow2);
    }
}

fn sub_in_place(ring: &ConstLargeDivisor, lhs: &mut ReducedLarge, rhs: &ReducedLarge) {
    debug_assert!(lhs.is_valid(ring) && rhs.is_valid(ring));
    let modulus = &ring.normalized_divisor;
    let overflow = add::sub_same_len_in_place(&mut lhs.0, &rhs.0);
    if overflow {
        let overflow2 = add::add_same_len_in_place(&mut lhs.0, modulus);
        debug_assert!(overflow2);
    }
}

/// rhs = self - rhs
fn sub_in_place_swap(ring: &ConstLargeDivisor, lhs: &ReducedLarge, rhs: &mut ReducedLarge) {
    debug_assert!(lhs.is_valid(ring) && rhs.is_valid(ring));
    let modulus = &ring.normalized_divisor;
    let overflow = add::sub_same_len_in_place_swap(&lhs.0, &mut rhs.0);
    if overflow {
        let overflow2 = add::add_same_len_in_place(&mut rhs.0, modulus);
        debug_assert!(overflow2);
    }
}
