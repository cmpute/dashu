use crate::{
    arch::word::Word,
    div,
    memory::{self, Memory, MemoryAllocation},
    modular::{
        modulo::{Modulo, ModuloLarge, ModuloRepr, ModuloSingle, ModuloSingleRaw},
        modulo_ring::{ModuloRingLarge, ModuloRingSingle},
    },
    mul,
    primitive::extend_word,
    shift,
    sign::Sign::Positive,
};
use alloc::alloc::Layout;
use core::ops::{Mul, MulAssign};

use super::modulo::ModuloLargeRaw;

impl<'a> Mul<Modulo<'a>> for Modulo<'a> {
    type Output = Modulo<'a>;

    #[inline]
    fn mul(self, rhs: Modulo<'a>) -> Modulo<'a> {
        self.mul(&rhs)
    }
}

impl<'a> Mul<&Modulo<'a>> for Modulo<'a> {
    type Output = Modulo<'a>;

    #[inline]
    fn mul(mut self, rhs: &Modulo<'a>) -> Modulo<'a> {
        self.mul_assign(rhs);
        self
    }
}

impl<'a> Mul<Modulo<'a>> for &Modulo<'a> {
    type Output = Modulo<'a>;

    #[inline]
    fn mul(self, rhs: Modulo<'a>) -> Modulo<'a> {
        rhs.mul(self)
    }
}

impl<'a> Mul<&Modulo<'a>> for &Modulo<'a> {
    type Output = Modulo<'a>;

    #[inline]
    fn mul(self, rhs: &Modulo<'a>) -> Modulo<'a> {
        self.clone().mul(rhs)
    }
}

impl<'a> MulAssign<Modulo<'a>> for Modulo<'a> {
    #[inline]
    fn mul_assign(&mut self, rhs: Modulo<'a>) {
        self.mul_assign(&rhs)
    }
}

impl ModuloRingSingle {
    #[inline]
    pub const fn mul(&self, lhs: ModuloSingleRaw, rhs: ModuloSingleRaw) -> ModuloSingleRaw {
        let product = extend_word(lhs.0 >> self.shift()) * extend_word(rhs.0);
        let (_, rem) = self.fast_div().div_rem(product);
        ModuloSingleRaw(rem)
    }

    #[inline]
    pub const fn sqr(&self, raw: ModuloSingleRaw) -> ModuloSingleRaw {
        let product = (extend_word(raw.0) * extend_word(raw.0)) >> self.shift();
        let (_, rem) = self.fast_div().div_rem(product);
        ModuloSingleRaw(rem)
    }
}

impl<'a> MulAssign<&Modulo<'a>> for Modulo<'a> {
    #[inline]
    fn mul_assign(&mut self, rhs: &Modulo<'a>) {
        match (self.repr_mut(), rhs.repr()) {
            (ModuloRepr::Small(self_small), ModuloRepr::Small(rhs_small)) => {
                self_small.check_same_ring(rhs_small);
                self_small.set_raw(self_small.ring().mul(self_small.raw(), rhs_small.raw()));
            }
            (ModuloRepr::Large(self_large), ModuloRepr::Large(rhs_large)) => {
                self_large.check_same_ring(rhs_large);
                let memory_requirement = self_large.ring().mul_memory_requirement();
                let mut allocation = MemoryAllocation::new(memory_requirement);
                let mut memory = allocation.memory();
                self_large.ring().mul_in_place(self_large.raw_mut(), &rhs_large.raw(), &mut memory);
            }
            _ => Modulo::panic_different_rings(),
        }
    }
}

impl ModuloRingLarge {
    pub(crate) fn mul_memory_requirement(&self) -> Layout {
        let n = self.normalized_modulus().len();
        memory::add_layout(
            memory::array_layout::<Word>(2 * n),
            memory::max_layout(
                mul::memory_requirement_exact(2 * n, n),
                div::memory_requirement_exact(2 * n, n),
            ),
        )
    }

    /// Returns a * b allocated in memory.
    pub(crate) fn mul_normalized<'a>(
        &self,
        a: &[Word],
        b: &[Word],
        memory: &'a mut Memory,
    ) -> &'a [Word] {
        let modulus = self.normalized_modulus();
        let n = modulus.len();
        debug_assert!(a.len() == n && b.len() == n);

        let (product, mut memory) = memory.allocate_slice_fill::<Word>(2 * n, 0);
        let overflow = mul::add_signed_mul_same_len(product, Positive, a, b, &mut memory);
        assert_eq!(overflow, 0);
        shift::shr_in_place(product, self.shift());

        let _overflow = div::div_rem_in_place(product, modulus, self.fast_div_top(), &mut memory);
        &product[..n]
    }

    /// self *= rhs
    pub(crate) fn mul_in_place(&self, lhs: &mut ModuloLargeRaw, rhs: &ModuloLargeRaw, memory: &mut Memory) {
        let prod = self.mul_normalized(&lhs.0, &rhs.0, memory);
        lhs.0.copy_from_slice(prod)
    }

    pub(crate) fn sqr_in_place(&self, raw: &mut ModuloLargeRaw, memory: &mut Memory) {
        let prod = self.mul_normalized(&raw.0, &raw.0, memory);
        raw.0.copy_from_slice(prod)
    }
}
