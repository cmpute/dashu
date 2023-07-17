//! Element of modular arithmetic.

use crate::div_const::{ConstDoubleDivisor, ConstLargeDivisor, ConstSingleDivisor};
use crate::{
    arch::word::{DoubleWord, Word},
    buffer::Buffer,
    cmp,
    error::panic_different_rings,
    math,
};
use alloc::boxed::Box;
use core::ptr;

/// Modular arithmetic.
///
/// # Examples
///
/// ```
/// # use dashu_int::{fast_div::ConstDivisor, UBig};
/// let ring = ConstDivisor::new(UBig::from(10000u32));
/// let x = ring.reduce(12345);
/// let y = ring.reduce(55443);
/// assert_eq!((x - y).residue(), UBig::from(6902u32));
/// ```
pub struct Reduced<'a>(ReducedRepr<'a>);

pub(crate) enum ReducedRepr<'a> {
    Single(ReducedWord, &'a ConstSingleDivisor),
    Double(ReducedDword, &'a ConstDoubleDivisor),
    Large(ReducedLarge, &'a ConstLargeDivisor),
}

/// Single word modular value in some unknown ring. The ring must be provided to operations.
///
/// The internal value must be in range 0..modulus and divisible by the shift for ModuloSingleRing
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ReducedWord(pub(crate) Word);

/// Double word modular value in some unknown ring. The ring must be provided to operations.
///
/// The internal value must be in range 0..modulus and divisible by the shift for ModuloDoubleRing
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ReducedDword(pub(crate) DoubleWord);

/// Multi-word modular value in some unknown ring. `self.0.len() == ring.normalized_modulus.len()`
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct ReducedLarge(pub(crate) Box<[Word]>);

impl<'a> Reduced<'a> {
    /// Get representation.
    #[inline]
    pub(crate) fn repr(&self) -> &ReducedRepr<'a> {
        &self.0
    }

    /// Get mutable representation.
    #[inline]
    pub(crate) fn repr_mut(&mut self) -> &mut ReducedRepr<'a> {
        &mut self.0
    }

    #[inline]
    pub(crate) fn into_repr(self) -> ReducedRepr<'a> {
        self.0
    }

    #[inline]
    pub(crate) const fn from_single(raw: ReducedWord, ring: &'a ConstSingleDivisor) -> Self {
        debug_assert!(raw.is_valid(ring));
        Reduced(ReducedRepr::Single(raw, ring))
    }

    #[inline]
    pub(crate) const fn from_double(raw: ReducedDword, ring: &'a ConstDoubleDivisor) -> Self {
        debug_assert!(raw.is_valid(ring));
        Reduced(ReducedRepr::Double(raw, ring))
    }

    #[inline]
    pub(crate) fn from_large(raw: ReducedLarge, ring: &'a ConstLargeDivisor) -> Self {
        debug_assert!(raw.is_valid(&ring));
        Reduced(ReducedRepr::Large(raw, ring))
    }

    #[inline]
    pub(crate) fn check_same_ring_single(lhs: &ConstSingleDivisor, rhs: &ConstSingleDivisor) {
        if !ptr::eq(lhs, rhs) {
            // Equality is identity: two rings are not equal even if they have the same modulus.
            panic_different_rings();
        }
    }

    #[inline]
    pub(crate) fn check_same_ring_double(lhs: &ConstDoubleDivisor, rhs: &ConstDoubleDivisor) {
        if !ptr::eq(lhs, rhs) {
            // Equality is identity: two rings are not equal even if they have the same modulus.
            panic_different_rings();
        }
    }

    #[inline]
    pub(crate) fn check_same_ring_large(lhs: &ConstLargeDivisor, rhs: &ConstLargeDivisor) {
        if !ptr::eq(lhs, rhs) {
            // Equality is identity: two rings are not equal even if they have the same modulus.
            panic_different_rings();
        }
    }
}

impl ReducedWord {
    pub const fn one(ring: &ConstSingleDivisor) -> Self {
        Self(1 << ring.shift())
    }

    #[inline]
    pub(crate) const fn is_valid(&self, ring: &ConstSingleDivisor) -> bool {
        self.0 & math::ones_word(ring.shift()) == 0 && self.0 < ring.normalized_divisor()
    }
}

impl ReducedDword {
    pub const fn one(ring: &ConstDoubleDivisor) -> Self {
        Self(1 << ring.shift())
    }

    #[inline]
    pub(crate) const fn is_valid(&self, ring: &ConstDoubleDivisor) -> bool {
        self.0 & math::ones_dword(ring.shift()) == 0 && self.0 < ring.normalized_divisor()
    }
}

impl ReducedLarge {
    pub fn one(ring: &ConstLargeDivisor) -> Self {
        let modulus = &ring.normalized_divisor;
        let mut buf = Buffer::allocate_exact(modulus.len());
        buf.push(1 << ring.shift);
        buf.push_zeros(modulus.len() - 1);
        Self(buf.into_boxed_slice())
    }

    #[inline]
    pub(crate) fn is_valid(&self, ring: &ConstLargeDivisor) -> bool {
        self.0.len() == ring.normalized_divisor.len()
            && cmp::cmp_same_len(&self.0, &ring.normalized_divisor).is_le()
            && self.0[0] & math::ones_word(ring.shift) == 0
    }
}

impl Clone for Reduced<'_> {
    #[inline]
    fn clone(&self) -> Self {
        Reduced(self.0.clone())
    }
    #[inline]
    fn clone_from(&mut self, source: &Self) {
        self.0.clone_from(&source.0);
    }
}

impl Clone for ReducedRepr<'_> {
    #[inline]
    fn clone(&self) -> Self {
        match self {
            ReducedRepr::Single(modulo, ring) => ReducedRepr::Single(*modulo, ring),
            ReducedRepr::Double(modulo, ring) => ReducedRepr::Double(*modulo, ring),
            ReducedRepr::Large(modulo, ring) => ReducedRepr::Large(modulo.clone(), ring),
        }
    }

    #[inline]
    fn clone_from(&mut self, source: &Self) {
        if let (ReducedRepr::Large(raw, ring), ReducedRepr::Large(src_raw, src_ring)) =
            (&mut *self, source)
        {
            *ring = src_ring;

            // this can be efficient if ring.len() == src_ring.len()
            raw.0.clone_from(&src_raw.0);
        } else {
            *self = source.clone();
        }
    }
}

/// Equality within a ring.
///
/// # Panics
///
/// Panics if the two values are from different rings.
impl PartialEq for Reduced<'_> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        match (self.repr(), other.repr()) {
            (ReducedRepr::Single(raw0, ring0), ReducedRepr::Single(raw1, ring1)) => {
                Reduced::check_same_ring_single(ring0, ring1);
                raw0.eq(raw1)
            }
            (ReducedRepr::Double(raw0, ring0), ReducedRepr::Double(raw1, ring1)) => {
                Reduced::check_same_ring_double(ring0, ring1);
                raw0.eq(raw1)
            }
            (ReducedRepr::Large(raw0, ring0), ReducedRepr::Large(raw1, ring1)) => {
                Reduced::check_same_ring_large(ring0, ring1);
                raw0.eq(raw1)
            }
            _ => panic_different_rings(),
        }
    }
}

impl Eq for Reduced<'_> {}
