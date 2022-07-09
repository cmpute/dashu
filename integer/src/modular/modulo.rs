//! Element of modular arithmetic.

use crate::{
    arch::word::{DoubleWord, Word},
    modular::modulo_ring::{ModuloRingLarge, ModuloRingSingle},
    repr::Buffer,
};

use super::modulo_ring::ModuloRingDouble;

/// Modular arithmetic.
///
/// # Examples
///
/// ```
/// # use dashu_int::{modular::ModuloRing, ubig};
/// let ring = ModuloRing::new(ubig!(10000));
/// let x = ring.convert(12345);
/// let y = ring.convert(55443);
/// assert_eq!((x - y).residue(), ubig!(6902));
/// ```
pub struct Modulo<'a>(ModuloRepr<'a>);

pub(crate) enum ModuloRepr<'a> {
    Single(ModuloSingleRaw, &'a ModuloRingSingle),
    Double(ModuloDoubleRaw, &'a ModuloRingDouble),
    Large(ModuloLargeRaw, &'a ModuloRingLarge),
}

/// Single word modular value in some unknown ring. The ring must be provided to operations.
///
/// The internal value must be in range 0..modulus and divisible by the shift for ModuloSingleRing
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ModuloSingleRaw(pub(crate) Word);

/// Double word modular value in some unknown ring. The ring must be provided to operations.
///
/// The internal value must be in range 0..modulus and divisible by the shift for ModuloDoubleRing
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ModuloDoubleRaw(pub(crate) DoubleWord);

/// Multi-word modular value in some unknown ring. `self.0.len() == ring.normalized_modulus.len()`
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct ModuloLargeRaw(pub(crate) Box<[Word]>);

impl<'a> Modulo<'a> {
    /// Get representation.
    #[inline]
    pub(crate) fn repr(&self) -> &ModuloRepr<'a> {
        &self.0
    }

    /// Get mutable representation.
    #[inline]
    pub(crate) fn repr_mut(&mut self) -> &mut ModuloRepr<'a> {
        &mut self.0
    }

    #[inline]
    pub(crate) fn into_repr(self) -> ModuloRepr<'a> {
        self.0
    }

    /// Panics when trying to do operations on [Modulo] values from different rings.
    pub fn panic_different_rings() -> ! {
        panic!("Modulo values from different rings")
    }

    #[inline]
    pub(crate) const fn from_single(raw: ModuloSingleRaw, ring: &'a ModuloRingSingle) -> Self {
        debug_assert!(ring.is_valid(raw));
        Modulo(ModuloRepr::Single(raw, ring))
    }

    #[inline]
    pub(crate) const fn from_double(raw: ModuloDoubleRaw, ring: &'a ModuloRingDouble) -> Self {
        debug_assert!(ring.is_valid(raw));
        Modulo(ModuloRepr::Double(raw, ring))
    }

    #[inline]
    pub(crate) fn from_large(raw: ModuloLargeRaw, ring: &'a ModuloRingLarge) -> Self {
        debug_assert!(ring.is_valid(&raw));
        Modulo(ModuloRepr::Large(raw, ring))
    }

    #[inline]
    pub(crate) fn check_same_ring_single(lhs: &ModuloRingSingle, rhs: &ModuloRingSingle) {
        if lhs != rhs {
            Self::panic_different_rings();
        }
    }

    #[inline]
    pub(crate) fn check_same_ring_double(lhs: &ModuloRingDouble, rhs: &ModuloRingDouble) {
        if lhs != rhs {
            Self::panic_different_rings();
        }
    }

    #[inline]
    pub(crate) fn check_same_ring_large(lhs: &ModuloRingLarge, rhs: &ModuloRingLarge) {
        if lhs != rhs {
            Self::panic_different_rings();
        }
    }
}

impl ModuloSingleRaw {
    pub const fn one(ring: &ModuloRingSingle) -> Self {
        let modulo = Self(1 << ring.shift());
        debug_assert!(ring.is_valid(modulo));
        modulo
    }
}

impl ModuloDoubleRaw {
    pub const fn one(ring: &ModuloRingDouble) -> Self {
        let modulo = Self(1 << ring.shift());
        debug_assert!(ring.is_valid(modulo));
        modulo
    }
}

impl ModuloLargeRaw {
    pub fn one(ring: &ModuloRingLarge) -> Self {
        let modulus = ring.normalized_modulus();
        let mut buf = Buffer::allocate_exact(modulus.len());
        buf.push(1 << ring.shift());
        buf.push_zeros(modulus.len() - 1);
        let modulo = Self(buf.into_boxed_slice());
        debug_assert!(ring.is_valid(&modulo));
        modulo
    }
}

impl Clone for Modulo<'_> {
    #[inline]
    fn clone(&self) -> Self {
        Modulo(self.0.clone())
    }

    #[inline]
    fn clone_from(&mut self, source: &Self) {
        self.0.clone_from(&source.0);
    }
}

impl Clone for ModuloRepr<'_> {
    #[inline]
    fn clone(&self) -> Self {
        match self {
            ModuloRepr::Single(modulo, ring) => ModuloRepr::Single(*modulo, ring),
            ModuloRepr::Double(modulo, ring) => ModuloRepr::Double(*modulo, ring),
            ModuloRepr::Large(modulo, ring) => ModuloRepr::Large(modulo.clone(), ring),
        }
    }

    #[inline]
    fn clone_from(&mut self, source: &Self) {
        if let (ModuloRepr::Large(raw, ring), ModuloRepr::Large(src_raw, src_ring)) =
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
