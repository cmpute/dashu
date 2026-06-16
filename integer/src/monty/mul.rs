//! Montgomery multiplication and squaring.

use crate::{
    add,
    arch::word::Word,
    cmp,
    error::panic_different_rings,
    memory::{self, Memory, MemoryAllocation},
    mul,
    primitive::{extend_word, locate_top_word_plus_one, split_dword},
    sqr,
};
use alloc::alloc::Layout;
use core::ops::{Mul, MulAssign};
use num_modular::Reducer;

use super::repr::{Montgomery, MontgomeryInner, MontgomeryLargeRepr, MontgomeryLargeVal};

impl<'a> Mul<Montgomery<'a>> for Montgomery<'a> {
    type Output = Montgomery<'a>;

    #[inline]
    fn mul(self, rhs: Montgomery<'a>) -> Montgomery<'a> {
        self.mul(&rhs)
    }
}

impl<'a> Mul<&Montgomery<'a>> for Montgomery<'a> {
    type Output = Montgomery<'a>;

    #[inline]
    fn mul(mut self, rhs: &Montgomery<'a>) -> Montgomery<'a> {
        self.mul_assign(rhs);
        self
    }
}

impl<'a> Mul<Montgomery<'a>> for &Montgomery<'a> {
    type Output = Montgomery<'a>;

    #[inline]
    fn mul(self, rhs: Montgomery<'a>) -> Montgomery<'a> {
        rhs.mul(self)
    }
}

impl<'a> Mul<&Montgomery<'a>> for &Montgomery<'a> {
    type Output = Montgomery<'a>;

    #[inline]
    fn mul(self, rhs: &Montgomery<'a>) -> Montgomery<'a> {
        self.clone().mul(rhs)
    }
}

impl<'a> MulAssign<Montgomery<'a>> for Montgomery<'a> {
    #[inline]
    fn mul_assign(&mut self, rhs: Montgomery<'a>) {
        self.mul_assign(&rhs)
    }
}

impl<'a> MulAssign<&Montgomery<'a>> for Montgomery<'a> {
    #[inline]
    fn mul_assign(&mut self, rhs: &Montgomery<'a>) {
        match (self.repr_mut(), rhs.repr()) {
            (MontgomeryInner::Single(raw0, ring), MontgomeryInner::Single(raw1, ring1)) => {
                Montgomery::check_same_ring_single(ring, ring1);
                ring.0.mul_in_place(raw0, raw1);
            }
            (MontgomeryInner::Double(raw0, ring), MontgomeryInner::Double(raw1, ring1)) => {
                Montgomery::check_same_ring_double(ring, ring1);
                ring.0.mul_in_place(raw0, raw1);
            }
            (MontgomeryInner::Large(raw0, ring), MontgomeryInner::Large(raw1, ring1)) => {
                Montgomery::check_same_ring_large(ring, ring1);
                let memory_requirement = mul_memory_requirement(ring);
                let mut allocation = MemoryAllocation::new(memory_requirement);
                mul_in_place_large(ring, raw0, raw1, &mut allocation.memory());
            }
            _ => panic_different_rings(),
        }
    }
}

impl<'a> Montgomery<'a> {
    /// Calculate target^2 mod m in Montgomery form.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::{monty::MontgomeryRepr, UBig};
    /// let ring = MontgomeryRepr::new(UBig::from(0x1234_5679u32));
    /// let a = ring.reduce(4000u32);
    /// assert_eq!(a.sqr(), ring.reduce(4000u32 * 4000u32));
    /// ```
    pub fn sqr(&self) -> Self {
        match self.repr() {
            MontgomeryInner::Single(raw, ring) => Montgomery::from_single(ring.0.sqr(*raw), ring),
            MontgomeryInner::Double(raw, ring) => Montgomery::from_double(ring.0.sqr(*raw), ring),
            MontgomeryInner::Large(raw, ring) => {
                let mut result = raw.clone();
                let memory_requirement = mul_memory_requirement(ring);
                let mut allocation = MemoryAllocation::new(memory_requirement);
                sqr_in_place_large(ring, &mut result, &mut allocation.memory());
                Montgomery::from_large(result, ring)
            }
        }
    }
}

/// Temporary scratch space required for Montgomery multiplication on a large ring.
pub(crate) fn mul_memory_requirement(ring: &MontgomeryLargeRepr) -> Layout {
    let s = ring.modulus.len();
    memory::add_layout(
        // Product buffer: 2s words for a*b plus one word for the REDC carry.
        memory::array_layout::<Word>(2 * s + 1),
        // Scratch reused by the `a*b` multiply and the `a^2` squaring (REDC itself is in place).
        memory::max_layout(
            mul::memory_requirement_exact(2 * s, s),
            sqr::memory_requirement_exact(s),
        ),
    )
}

/// Word-by-word Montgomery reduction (REDC) of a `(2s+1)`-word buffer `t` in place.
///
/// Uses the per-word constant `n0 = -m^{-1} mod 2^WORD_BITS`: at step `i` the value
/// `q = t[i]*n0 mod 2^WORD_BITS` is added (times the modulus) at position `i`, which makes word
/// `i` cancel to zero. After `s` steps, `t[0..s]` is zero and the result — lying in `[0, 2m)` —
/// occupies `t[s..2s+1]`.
pub(crate) fn redc_in_place(t: &mut [Word], ring: &MontgomeryLargeRepr) {
    let s = ring.modulus.len();
    let n0 = ring.n0;
    let modulus = &ring.modulus;
    debug_assert_eq!(t.len(), 2 * s + 1);

    for i in 0..s {
        let q = t[i].wrapping_mul(n0);
        // t[i..i+s] += q * modulus
        let carry = mul::add_mul_word_same_len_in_place(&mut t[i..i + s], q, modulus);
        // propagate the carry into the upper words (fits within the buffer)
        let overflow = add::add_word_in_place(&mut t[i + s..], carry);
        debug_assert!(!overflow);
        debug_assert_eq!(t[i], 0);
    }
}

/// Reduce the REDC output in `t[s..2s+1]` (value in `[0, 2m)`) down to the canonical `[0, m)`,
/// returning the `s`-word result slice `t[s..2s]`.
fn canonicalize<'a>(t: &'a mut [Word], ring: &MontgomeryLargeRepr) -> &'a [Word] {
    let s = ring.modulus.len();
    let modulus = &ring.modulus;
    // result >= m iff the top carry word are set, or the low s words exceed the modulus.
    if t[2 * s] != 0 || cmp::cmp_same_len(&t[s..2 * s], modulus).is_ge() {
        let borrow = add::sub_in_place(&mut t[s..2 * s + 1], modulus);
        debug_assert!(!borrow);
    }
    debug_assert_eq!(t[2 * s], 0);
    &t[s..2 * s]
}

/// Returns `a * b` (Montgomery product) as an `s`-word slice allocated in `memory`.
pub(crate) fn mul_normalized_large<'a>(
    ring: &MontgomeryLargeRepr,
    a: &[Word],
    b: &[Word],
    memory: &'a mut Memory,
) -> &'a [Word] {
    let s = ring.modulus.len();
    debug_assert!(a.len() == s && b.len() == s);

    let na = locate_top_word_plus_one(a);
    let nb = locate_top_word_plus_one(b);

    // product = a * b (2s words + 1 carry word, zero-filled)
    let (product, mut memory) = memory.allocate_slice_fill::<Word>(2 * s + 1, 0);
    if na | nb == 0 {
        return &product[s..2 * s];
    } else if na == 1 && nb == 1 {
        let (lo, hi) = split_dword(extend_word(a[0]) * extend_word(b[0]));
        product[0] = lo;
        product[1] = hi;
    } else {
        mul::multiply(&mut product[..na + nb], &a[..na], &b[..nb], &mut memory);
    }

    redc_in_place(product, ring);
    canonicalize(product, ring)
}

/// Returns `a^2` (Montgomery square) as an `s`-word slice allocated in `memory`.
pub(crate) fn sqr_normalized_large<'a>(
    ring: &MontgomeryLargeRepr,
    a: &[Word],
    memory: &'a mut Memory,
) -> &'a [Word] {
    let s = ring.modulus.len();
    debug_assert!(a.len() == s);

    let na = locate_top_word_plus_one(a);

    // product = a * a
    let (product, mut memory) = memory.allocate_slice_fill::<Word>(2 * s + 1, 0);
    if na == 0 {
        return &product[s..2 * s];
    } else if na == 1 {
        let (lo, hi) = split_dword(extend_word(a[0]) * extend_word(a[0]));
        product[0] = lo;
        product[1] = hi;
    } else {
        sqr::sqr(&mut product[..2 * na], &a[..na], &mut memory);
    }

    redc_in_place(product, ring);
    canonicalize(product, ring)
}

/// Returns the plain residue `a * R^{-1} mod m` (i.e. exit Montgomery form) as an `s`-word
/// slice allocated in `memory`. Computed as `REDC(a)` with `a` placed in the low half of a
/// zero-filled buffer.
pub(crate) fn residue_normalized_large<'a>(
    ring: &MontgomeryLargeRepr,
    a: &[Word],
    memory: &'a mut Memory,
) -> &'a [Word] {
    let s = ring.modulus.len();
    debug_assert_eq!(a.len(), s);

    let (product, _memory) = memory.allocate_slice_fill::<Word>(2 * s + 1, 0);
    product[..s].copy_from_slice(a);

    redc_in_place(product, ring);
    canonicalize(product, ring)
}

/// lhs *= rhs
pub(crate) fn mul_in_place_large(
    ring: &MontgomeryLargeRepr,
    lhs: &mut MontgomeryLargeVal,
    rhs: &MontgomeryLargeVal,
    memory: &mut Memory,
) {
    let prod = if lhs.0 == rhs.0 {
        sqr_normalized_large(ring, &lhs.0, memory)
    } else {
        mul_normalized_large(ring, &lhs.0, &rhs.0, memory)
    };
    lhs.0.copy_from_slice(prod);
}

/// raw = raw^2
pub(crate) fn sqr_in_place_large(
    ring: &MontgomeryLargeRepr,
    raw: &mut MontgomeryLargeVal,
    memory: &mut Memory,
) {
    let prod = sqr_normalized_large(ring, &raw.0, memory);
    raw.0.copy_from_slice(prod);
}
