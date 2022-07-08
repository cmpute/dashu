//! Element of modular arithmetic.

use crate::{
    arch::word::Word,
    assert::debug_assert_in_const_fn,
    modular::modulo_ring::{ModuloRingLarge, ModuloRingSingle},
};
use alloc::vec::Vec;

/// Modular arithmetic.
///
/// # Examples
///
/// ```
/// # use dashu_int::{modular::ModuloRing, ubig};
/// let ring = ModuloRing::new(&ubig!(10000));
/// let x = ring.from(12345);
/// let y = ring.from(55443);
/// assert_eq!((x - y).residue(), ubig!(6902));
/// ```
pub struct Modulo<'a>(ModuloRepr<'a>);

pub(crate) enum ModuloRepr<'a> {
    Small(ModuloSingleRaw, &'a ModuloRingSingle),
    Large(ModuloLargeRaw, &'a ModuloRingLarge),
}

/// Single word modular value in some unknown ring. The ring must be provided to operations.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ModuloSingleRaw(pub(crate) Word);

/// Multi-word modular value in some unknown ring. `self.0.len() == ring.normalized_modulus.len()`
///
/// The vanilla `Vec` is used instead of `Buffer` here because we want fixed and compact capacity in the modulo.
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct ModuloLargeRaw(pub(crate) Vec<Word>); // TODO: use Box<[Word]>

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
    pub(crate) const fn from_small(raw: ModuloSingleRaw, ring: &'a ModuloRingSingle) -> Self {
        debug_assert_in_const_fn!(ring.is_valid(raw));
        Modulo(ModuloRepr::Small(raw, ring))
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

impl ModuloLargeRaw {
    pub fn one(ring: &ModuloRingLarge) -> Self {
        let modulus = ring.normalized_modulus();
        let mut vec = Vec::with_capacity(modulus.len());
        vec.push(1 << ring.shift());
        vec.extend(core::iter::repeat(0).take(modulus.len() - 1));
        let modulo = Self(vec);
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
            ModuloRepr::Small(modulo, ring) => ModuloRepr::Small(modulo.clone(), ring),
            ModuloRepr::Large(modulo, ring) => ModuloRepr::Large(modulo.clone(), ring),
        }
    }

    #[inline]
    fn clone_from(&mut self, source: &Self) {
        // TODO: do we actually need this? it seems that the large modulo is fixed in size
        if let (ModuloRepr::Large(raw, ring), ModuloRepr::Large(src_raw, src_ring)) =
            (&mut *self, source)
        {
            *ring = src_ring;
            if raw.0.len() == src_raw.0.len() {
                raw.0.copy_from_slice(&src_raw.0)
            } else {
                // We don't want to have spare capacity, so do not clone_from.
                raw.0 = src_raw.0.clone();
            }
        } else {
            *self = source.clone();
        }
    }
}
