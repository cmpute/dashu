use crate::{
    add,
    arch::word::Word,
    cmp, div,
    div_const::ConstLargeDivisor,
    error::panic_different_rings,
    helper_macros::debug_assert_zero,
    memory::{self, Memory, MemoryAllocation},
    modular::repr::{Reduced, ReducedRepr},
    mul,
    primitive::{extend_word, locate_top_word_plus_one, split_dword},
    shift, sqr,
};
use alloc::alloc::Layout;
use core::ops::{Deref, Mul, MulAssign};
use num_modular::Reducer;

use super::repr::{ReducedDword, ReducedLarge, ReducedWord};

impl<'a> Mul<Reduced<'a>> for Reduced<'a> {
    type Output = Reduced<'a>;

    #[inline]
    fn mul(self, rhs: Reduced<'a>) -> Reduced<'a> {
        self.mul(&rhs)
    }
}

impl<'a> Mul<&Reduced<'a>> for Reduced<'a> {
    type Output = Reduced<'a>;

    #[inline]
    fn mul(mut self, rhs: &Reduced<'a>) -> Reduced<'a> {
        self.mul_assign(rhs);
        self
    }
}

impl<'a> Mul<Reduced<'a>> for &Reduced<'a> {
    type Output = Reduced<'a>;

    #[inline]
    fn mul(self, rhs: Reduced<'a>) -> Reduced<'a> {
        rhs.mul(self)
    }
}

impl<'a> Mul<&Reduced<'a>> for &Reduced<'a> {
    type Output = Reduced<'a>;

    #[inline]
    fn mul(self, rhs: &Reduced<'a>) -> Reduced<'a> {
        self.clone().mul(rhs)
    }
}

impl<'a> MulAssign<Reduced<'a>> for Reduced<'a> {
    #[inline]
    fn mul_assign(&mut self, rhs: Reduced<'a>) {
        self.mul_assign(&rhs)
    }
}

impl<'a> MulAssign<&Reduced<'a>> for Reduced<'a> {
    #[inline]
    fn mul_assign(&mut self, rhs: &Reduced<'a>) {
        match (self.repr_mut(), rhs.repr()) {
            (ReducedRepr::Single(raw0, ring), ReducedRepr::Single(raw1, ring1)) => {
                Reduced::check_same_ring_single(ring, ring1);
                ring.0.mul_in_place(&mut raw0.0, &raw1.0)
            }
            (ReducedRepr::Double(raw0, ring), ReducedRepr::Double(raw1, ring1)) => {
                Reduced::check_same_ring_double(ring, ring1);
                ring.0.mul_in_place(&mut raw0.0, &raw1.0)
            }
            (ReducedRepr::Large(raw0, ring), ReducedRepr::Large(raw1, ring1)) => {
                Reduced::check_same_ring_large(ring, ring1);
                let memory_requirement = mul_memory_requirement(ring);
                let mut allocation = MemoryAllocation::new(memory_requirement);
                mul_in_place(ring, raw0, raw1, &mut allocation.memory());
            }
            _ => panic_different_rings(),
        }
    }
}

impl<'a> Reduced<'a> {
    /// Calculate target^2 mod m in reduced form
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::{fast_div::ConstDivisor, UBig};
    /// let p = UBig::from(0x1234u16);
    /// let ring = ConstDivisor::new(p.clone());
    /// let a = ring.reduce(4000);
    /// assert_eq!(a.sqr(), ring.reduce(4000 * 4000));
    /// ```
    pub fn sqr(&self) -> Self {
        match self.repr() {
            ReducedRepr::Single(raw, ring) => {
                Reduced::from_single(ReducedWord(ring.0.sqr(raw.0)), ring)
            }
            ReducedRepr::Double(raw, ring) => {
                Reduced::from_double(ReducedDword(ring.0.sqr(raw.0)), ring)
            }
            ReducedRepr::Large(raw, ring) => {
                let mut result = raw.clone();
                let memory_requirement = mul_memory_requirement(ring);
                let mut allocation = MemoryAllocation::new(memory_requirement);
                sqr_in_place(ring, &mut result, &mut allocation.memory());
                Reduced::from_large(result, ring)
            }
        }
    }
}

pub(crate) fn mul_memory_requirement(ring: &ConstLargeDivisor) -> Layout {
    let n = ring.normalized_divisor.len();
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
    ring: &ConstLargeDivisor,
    a: &[Word],
    b: &[Word],
    memory: &'a mut Memory,
) -> &'a [Word] {
    let modulus = ring.normalized_divisor.deref();
    let n = modulus.len();
    debug_assert!(a.len() == n && b.len() == n);

    // trim the leading zeros in a, b
    let na = locate_top_word_plus_one(a);
    let nb = locate_top_word_plus_one(b);

    // product = a * b
    let (product, mut memory) = memory.allocate_slice_fill::<Word>(n.max(na + nb), 0);
    if na | nb == 0 {
        return product;
    } else if na == 1 && nb == 1 {
        let (a0, b0) = (extend_word(a[0]), extend_word(b[0]));
        let (lo, hi) = split_dword(a0 * b0);
        product[0] = lo;
        product[1] = hi;
    } else {
        mul::multiply(&mut product[..na + nb], &a[..na], &b[..nb], &mut memory);
    }

    // return (product >> shift) % normalized_modulus
    debug_assert_zero!(shift::shr_in_place(product, ring.shift));
    if na + nb > n {
        let _overflow = div::div_rem_in_place(product, modulus, ring.fast_div_top, &mut memory);
        &product[..n]
    } else {
        if cmp::cmp_same_len(product, modulus).is_ge() {
            debug_assert_zero!(add::sub_same_len_in_place(product, modulus));
        }
        product
    }
}

/// lhs *= rhs
pub(crate) fn mul_in_place(
    ring: &ConstLargeDivisor,
    lhs: &mut ReducedLarge,
    rhs: &ReducedLarge,
    memory: &mut Memory,
) {
    if lhs.0 == rhs.0 {
        // shortcut to squaring
        let prod = sqr_normalized(ring, &lhs.0, memory);
        lhs.0.copy_from_slice(prod)
    } else {
        let prod = mul_normalized(ring, &lhs.0, &rhs.0, memory);
        lhs.0.copy_from_slice(prod)
    }
}

/// Returns a^2 allocated in memory.
pub(crate) fn sqr_normalized<'a>(
    ring: &ConstLargeDivisor,
    a: &[Word],
    memory: &'a mut Memory,
) -> &'a [Word] {
    let modulus = ring.normalized_divisor.deref();
    let n = modulus.len();
    debug_assert!(a.len() == n);

    // trim the leading zeros in a
    let na = locate_top_word_plus_one(a);

    // product = a * a
    let (product, mut memory) = memory.allocate_slice_fill::<Word>(n.max(na * 2), 0);
    if na == 0 {
        return product;
    } else if na == 1 {
        let a0 = extend_word(a[0]);
        let (lo, hi) = split_dword(a0 * a0);
        product[0] = lo;
        product[1] = hi;
    } else {
        sqr::sqr(&mut product[..na * 2], &a[..na], &mut memory);
    }

    // return (product >> shift) % normalized_modulus
    debug_assert_zero!(shift::shr_in_place(product, ring.shift));
    if na * 2 > n {
        let _overflow = div::div_rem_in_place(product, modulus, ring.fast_div_top, &mut memory);
        &product[..n]
    } else {
        if cmp::cmp_same_len(product, modulus).is_ge() {
            debug_assert_zero!(add::sub_same_len_in_place(product, modulus));
        }
        product
    }
}

/// raw = raw^2
pub(crate) fn sqr_in_place(ring: &ConstLargeDivisor, raw: &mut ReducedLarge, memory: &mut Memory) {
    let prod = sqr_normalized(ring, &raw.0, memory);
    raw.0.copy_from_slice(prod)
}
