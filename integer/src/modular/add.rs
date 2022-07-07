//! Modular addition and subtraction.

use crate::{
    add, cmp,
    modular::{
        modulo::{Modulo, ModuloLarge, ModuloRepr, ModuloSingle, ModuloSingleRaw},
        modulo_ring::ModuloRingSingle,
    },
    assert::debug_assert_in_const_fn,
};
use core::{
    cmp::Ordering,
    ops::{Add, AddAssign, Neg, Sub, SubAssign},
};

impl<'a> Neg for Modulo<'a> {
    type Output = Modulo<'a>;

    #[inline]
    fn neg(mut self) -> Modulo<'a> {
        match self.repr_mut() {
            ModuloRepr::Small(self_small) => self_small.set_raw(self_small.ring().negate(self_small.raw())),
            ModuloRepr::Large(self_large) => self_large.negate_in_place(),
        }
        self
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
            (ModuloRepr::Small(self_small), ModuloRepr::Small(rhs_small)) => {
                self_small.set_raw(self_small.ring().add(self_small.raw(), rhs_small.raw()))
            }
            (ModuloRepr::Large(self_large), ModuloRepr::Large(rhs_large)) => {
                self_large.add_in_place(rhs_large)
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
    fn sub(self, mut rhs: Modulo<'a>) -> Modulo<'a> {
        match (self.repr(), rhs.repr_mut()) {
            (ModuloRepr::Small(self_small), ModuloRepr::Small(rhs_small)) => {
                rhs_small.set_raw(self_small.ring().sub(self_small.raw(), rhs_small.raw()));
            }
            (ModuloRepr::Large(self_large), ModuloRepr::Large(rhs_large)) => {
                self_large.sub_in_place_swap(rhs_large)
            }
            _ => Modulo::panic_different_rings(),
        }
        rhs
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
            (ModuloRepr::Small(self_small), ModuloRepr::Small(rhs_small)) => {
                self_small.set_raw(self_small.ring().sub(self_small.raw(), rhs_small.raw()));
                
            }
            (ModuloRepr::Large(self_large), ModuloRepr::Large(rhs_large)) => {
                self_large.sub_in_place(rhs_large)
            }
            _ => Modulo::panic_different_rings(),
        }
    }
}

impl ModuloRingSingle {
    #[inline]
    const fn negate(&self, raw: ModuloSingleRaw) -> ModuloSingleRaw {
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
            debug_assert_in_const_fn!(overflow == overflow2);
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

impl<'a> ModuloLarge<'a> {
    /// self = -self
    fn negate_in_place(&mut self) {
        self.modify_normalized_value(|words, ring| {
            if !words.iter().all(|w| *w == 0) {
                let overflow = add::sub_same_len_in_place_swap(ring.normalized_modulus(), words);
                assert!(!overflow);
            }
        });
    }

    /// self += rhs
    fn add_in_place(&mut self, rhs: &ModuloLarge<'a>) {
        self.check_same_ring(rhs);
        let rhs_words = rhs.normalized_value();
        self.modify_normalized_value(|words, ring| {
            let modulus = ring.normalized_modulus();
            let overflow = add::add_same_len_in_place(words, rhs_words);
            if overflow || cmp::cmp_same_len(words, modulus) >= Ordering::Equal {
                let overflow2 = add::sub_same_len_in_place(words, modulus);
                debug_assert_eq!(overflow, overflow2);
            }
        });
    }

    /// self -= rhs
    fn sub_in_place(&mut self, rhs: &ModuloLarge<'a>) {
        self.check_same_ring(rhs);
        let rhs_words = rhs.normalized_value();
        self.modify_normalized_value(|words, ring| {
            let modulus = ring.normalized_modulus();
            let overflow = add::sub_same_len_in_place(words, rhs_words);
            if overflow {
                let overflow2 = add::add_same_len_in_place(words, modulus);
                debug_assert!(overflow2);
            }
        });
    }

    /// rhs = self - rhs
    fn sub_in_place_swap(&self, rhs: &mut ModuloLarge<'a>) {
        self.check_same_ring(rhs);
        let words = self.normalized_value();
        rhs.modify_normalized_value(|rhs_words, ring| {
            let modulus = ring.normalized_modulus();
            let overflow = add::sub_same_len_in_place_swap(words, rhs_words);
            if overflow {
                let overflow2 = add::add_same_len_in_place(rhs_words, modulus);
                debug_assert!(overflow2);
            }
        });
    }
}
