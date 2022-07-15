//! Modular addition and subtraction.

use super::{
    modulo::{Modulo, ModuloDoubleRaw, ModuloLargeRaw, ModuloRepr, ModuloSingleRaw},
    modulo_ring::{ModuloRingDouble, ModuloRingLarge, ModuloRingSingle},
};
use crate::{add, cmp};
use core::{
    cmp::Ordering,
    ops::{Add, AddAssign, Neg, Sub, SubAssign},
};

impl<'a> Neg for Modulo<'a> {
    type Output = Modulo<'a>;

    #[inline]
    fn neg(self) -> Modulo<'a> {
        match self.into_repr() {
            ModuloRepr::Single(raw, ring) => Self::from_single(ring.negate(raw), ring),
            ModuloRepr::Double(raw, ring) => Self::from_double(ring.negate(raw), ring),
            ModuloRepr::Large(mut raw, ring) => {
                ring.negate_in_place(&mut raw);
                Self::from_large(raw, ring)
            }
        }
    }
}

impl<'a> Neg for &Modulo<'a> {
    type Output = Modulo<'a>;

    #[inline]
    fn neg(self) -> Modulo<'a> {
        self.clone().neg()
    }
}

impl<'a> Add<Modulo<'a>> for Modulo<'a> {
    type Output = Modulo<'a>;

    #[inline]
    fn add(self, rhs: Modulo<'a>) -> Modulo<'a> {
        self.add(&rhs)
    }
}

impl<'a> Add<&Modulo<'a>> for Modulo<'a> {
    type Output = Modulo<'a>;

    #[inline]
    fn add(mut self, rhs: &Modulo<'a>) -> Modulo<'a> {
        self.add_assign(rhs);
        self
    }
}

impl<'a> Add<Modulo<'a>> for &Modulo<'a> {
    type Output = Modulo<'a>;

    #[inline]
    fn add(self, rhs: Modulo<'a>) -> Modulo<'a> {
        rhs.add(self)
    }
}

impl<'a> Add<&Modulo<'a>> for &Modulo<'a> {
    type Output = Modulo<'a>;

    #[inline]
    fn add(self, rhs: &Modulo<'a>) -> Modulo<'a> {
        self.clone().add(rhs)
    }
}

impl<'a> AddAssign<Modulo<'a>> for Modulo<'a> {
    #[inline]
    fn add_assign(&mut self, rhs: Modulo<'a>) {
        self.add_assign(&rhs)
    }
}

impl<'a> AddAssign<&Modulo<'a>> for Modulo<'a> {
    #[inline]
    fn add_assign(&mut self, rhs: &Modulo<'a>) {
        match (self.repr_mut(), rhs.repr()) {
            (ModuloRepr::Single(raw0, ring), ModuloRepr::Single(raw1, ring1)) => {
                Modulo::check_same_ring_single(ring, ring1);
                *raw0 = ring.add(*raw0, *raw1);
            }
            (ModuloRepr::Double(raw0, ring), ModuloRepr::Double(raw1, ring1)) => {
                Modulo::check_same_ring_double(ring, ring1);
                *raw0 = ring.add(*raw0, *raw1);
            }
            (ModuloRepr::Large(raw0, ring), ModuloRepr::Large(raw1, ring1)) => {
                Modulo::check_same_ring_large(ring, ring1);
                ring.add_in_place(raw0, raw1);
            }
            _ => Modulo::panic_different_rings(),
        }
    }
}

impl<'a> Sub<Modulo<'a>> for Modulo<'a> {
    type Output = Modulo<'a>;

    #[inline]
    fn sub(self, rhs: Modulo<'a>) -> Modulo<'a> {
        self.sub(&rhs)
    }
}

impl<'a> Sub<&Modulo<'a>> for Modulo<'a> {
    type Output = Modulo<'a>;

    #[inline]
    fn sub(mut self, rhs: &Modulo<'a>) -> Modulo<'a> {
        self.sub_assign(rhs);
        self
    }
}

impl<'a> Sub<Modulo<'a>> for &Modulo<'a> {
    type Output = Modulo<'a>;

    #[inline]
    fn sub(self, rhs: Modulo<'a>) -> Modulo<'a> {
        match (self.repr(), rhs.into_repr()) {
            (ModuloRepr::Single(raw0, ring), ModuloRepr::Single(raw1, ring1)) => {
                Modulo::check_same_ring_single(ring, ring1);
                Modulo::from_single(ring.sub(*raw0, raw1), ring)
            }
            (ModuloRepr::Double(raw0, ring), ModuloRepr::Double(raw1, ring1)) => {
                Modulo::check_same_ring_double(ring, ring1);
                Modulo::from_double(ring.sub(*raw0, raw1), ring)
            }
            (ModuloRepr::Large(raw0, ring), ModuloRepr::Large(mut raw1, ring1)) => {
                Modulo::check_same_ring_large(ring, ring1);
                ring.sub_in_place_swap(raw0, &mut raw1);
                Modulo::from_large(raw1, ring)
            }
            _ => Modulo::panic_different_rings(),
        }
    }
}

impl<'a> Sub<&Modulo<'a>> for &Modulo<'a> {
    type Output = Modulo<'a>;

    #[inline]
    fn sub(self, rhs: &Modulo<'a>) -> Modulo<'a> {
        self.clone().sub(rhs)
    }
}

impl<'a> SubAssign<Modulo<'a>> for Modulo<'a> {
    #[inline]
    fn sub_assign(&mut self, rhs: Modulo<'a>) {
        self.sub_assign(&rhs)
    }
}

impl<'a> SubAssign<&Modulo<'a>> for Modulo<'a> {
    #[inline]
    fn sub_assign(&mut self, rhs: &Modulo<'a>) {
        match (self.repr_mut(), rhs.repr()) {
            (ModuloRepr::Single(raw0, ring), ModuloRepr::Single(raw1, ring1)) => {
                Modulo::check_same_ring_single(ring, ring1);
                *raw0 = ring.sub(*raw0, *raw1);
            }
            (ModuloRepr::Double(raw0, ring), ModuloRepr::Double(raw1, ring1)) => {
                Modulo::check_same_ring_double(ring, ring1);
                *raw0 = ring.sub(*raw0, *raw1);
            }
            (ModuloRepr::Large(raw0, ring), ModuloRepr::Large(raw1, ring1)) => {
                Modulo::check_same_ring_large(ring, ring1);
                ring.sub_in_place(raw0, raw1);
            }
            _ => Modulo::panic_different_rings(),
        }
    }
}

impl ModuloRingSingle {
    #[inline]
    pub const fn negate(&self, raw: ModuloSingleRaw) -> ModuloSingleRaw {
        debug_assert!(self.is_valid(raw));
        let val = match raw.0 {
            0 => 0,
            x => self.normalized_modulus() - x,
        };
        ModuloSingleRaw(val)
    }

    #[inline]
    const fn add(&self, lhs: ModuloSingleRaw, rhs: ModuloSingleRaw) -> ModuloSingleRaw {
        debug_assert!(self.is_valid(lhs) && self.is_valid(rhs));
        let (mut val, overflow) = lhs.0.overflowing_add(rhs.0);
        let m = self.normalized_modulus();
        if overflow || val >= m {
            let (v, overflow2) = val.overflowing_sub(m);
            debug_assert!(overflow == overflow2);
            val = v;
        }
        ModuloSingleRaw(val)
    }

    #[inline]
    const fn sub(&self, lhs: ModuloSingleRaw, rhs: ModuloSingleRaw) -> ModuloSingleRaw {
        debug_assert!(self.is_valid(lhs) && self.is_valid(rhs));
        let (mut val, overflow) = lhs.0.overflowing_sub(rhs.0);
        if overflow {
            let m = self.normalized_modulus();
            let (v, overflow2) = val.overflowing_add(m);
            debug_assert!(overflow2);
            val = v;
        }
        ModuloSingleRaw(val)
    }
}

impl ModuloRingDouble {
    #[inline]
    pub const fn negate(&self, raw: ModuloDoubleRaw) -> ModuloDoubleRaw {
        debug_assert!(self.is_valid(raw));
        let val = match raw.0 {
            0 => 0,
            x => self.normalized_modulus() - x,
        };
        ModuloDoubleRaw(val)
    }

    #[inline]
    const fn add(&self, lhs: ModuloDoubleRaw, rhs: ModuloDoubleRaw) -> ModuloDoubleRaw {
        debug_assert!(self.is_valid(lhs) && self.is_valid(rhs));
        let (mut val, overflow) = lhs.0.overflowing_add(rhs.0);
        let m = self.normalized_modulus();
        if overflow || val >= m {
            let (v, overflow2) = val.overflowing_sub(m);
            debug_assert!(overflow == overflow2);
            val = v;
        }
        ModuloDoubleRaw(val)
    }

    #[inline]
    const fn sub(&self, lhs: ModuloDoubleRaw, rhs: ModuloDoubleRaw) -> ModuloDoubleRaw {
        debug_assert!(self.is_valid(lhs) && self.is_valid(rhs));
        let (mut val, overflow) = lhs.0.overflowing_sub(rhs.0);
        if overflow {
            let m = self.normalized_modulus();
            let (v, overflow2) = val.overflowing_add(m);
            debug_assert!(overflow2);
            val = v;
        }
        ModuloDoubleRaw(val)
    }
}

impl ModuloRingLarge {
    pub fn negate_in_place(&self, raw: &mut ModuloLargeRaw) {
        debug_assert!(self.is_valid(&*raw));
        if !raw.0.iter().all(|w| *w == 0) {
            let overflow = add::sub_same_len_in_place_swap(self.normalized_modulus(), &mut raw.0);
            assert!(!overflow);
        }
    }

    fn add_in_place(&self, lhs: &mut ModuloLargeRaw, rhs: &ModuloLargeRaw) {
        debug_assert!(self.is_valid(&*lhs) && self.is_valid(rhs));
        let modulus = self.normalized_modulus();
        let overflow = add::add_same_len_in_place(&mut lhs.0, &rhs.0);
        if overflow || cmp::cmp_same_len(&lhs.0, modulus) >= Ordering::Equal {
            let overflow2 = add::sub_same_len_in_place(&mut lhs.0, modulus);
            debug_assert_eq!(overflow, overflow2);
        }
    }

    fn sub_in_place(&self, lhs: &mut ModuloLargeRaw, rhs: &ModuloLargeRaw) {
        debug_assert!(self.is_valid(&*lhs) && self.is_valid(rhs));
        let modulus = self.normalized_modulus();
        let overflow = add::sub_same_len_in_place(&mut lhs.0, &rhs.0);
        if overflow {
            let overflow2 = add::add_same_len_in_place(&mut lhs.0, modulus);
            debug_assert!(overflow2);
        }
    }

    /// rhs = self - rhs
    fn sub_in_place_swap(&self, lhs: &ModuloLargeRaw, rhs: &mut ModuloLargeRaw) {
        debug_assert!(self.is_valid(&*lhs) && self.is_valid(rhs));
        let modulus = self.normalized_modulus();
        let overflow = add::sub_same_len_in_place_swap(&lhs.0, &mut rhs.0);
        if overflow {
            let overflow2 = add::add_same_len_in_place(&mut rhs.0, modulus);
            debug_assert!(overflow2);
        }
    }
}
