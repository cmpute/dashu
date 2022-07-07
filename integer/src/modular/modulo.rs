//! Element of modular arithmetic.

use crate::{
    arch::word::Word,
    math,
    modular::modulo_ring::{ModuloRingLarge, ModuloRingSingle},
    primitive::extend_word,
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
    Small(ModuloSingle<'a>),
    Large(ModuloLarge<'a>),
}

/// Modular value in some unknown ring. The ring must be provided to operations.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ModuloSingleRaw(pub(crate) Word);

// TODO: remove ModuloSingle / ModuloLarge, combine them directly into the ModuloRepr
#[derive(Clone, Copy, Eq)]
pub(crate) struct ModuloSingle<'a> {
    ring: &'a ModuloRingSingle,
    raw: ModuloSingleRaw,
}

pub(crate) struct ModuloLarge<'a> {
    ring: &'a ModuloRingLarge,
    /// normalized_value.len() == ring.normalized_modulus.len()
    normalized_value: Vec<Word>,
}

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

    /// Panics when trying to do operations on [Modulo] values from different rings.
    pub fn panic_different_rings() -> ! {
        panic!("Modulo values from different rings")
    }
}

impl<'a> From<ModuloSingle<'a>> for Modulo<'a> {
    #[inline]
    fn from(a: ModuloSingle<'a>) -> Self {
        Modulo(ModuloRepr::Small(a))
    }
}

impl<'a> From<ModuloLarge<'a>> for Modulo<'a> {
    fn from(a: ModuloLarge<'a>) -> Self {
        Modulo(ModuloRepr::Large(a))
    }
}

impl<'a> ModuloSingle<'a> {
    #[inline]
    pub(crate) fn new(raw: ModuloSingleRaw, ring: &'a ModuloRingSingle) -> Self {
        debug_assert!(ring.is_valid(raw));
        ModuloSingle { ring, raw }
    }

    /// Get the ring.
    #[inline]
    pub(crate) fn ring(&self) -> &'a ModuloRingSingle {
        self.ring
    }

    /// Checks that two values are from the same ring.
    #[inline]
    pub(crate) fn check_same_ring(&self, other: &ModuloSingle) {
        if self.ring() != other.ring() {
            Modulo::panic_different_rings();
        }
    }

    // TODO: rename to normalized() or raw()
    #[inline]
    pub(crate) const fn normalized_value(self) -> Word {
        self.raw.0
    }

    pub(crate) const fn raw(&self) -> ModuloSingleRaw {
        self.raw
    }

    pub(crate) fn set_raw(&mut self, val: ModuloSingleRaw) {
        self.raw = val
    }
}

impl<'a> ModuloLarge<'a> {
    /// Create new ModuloLarge.
    ///
    /// normalized_value must have the same length as the modulus, be in range 0..modulus,
    /// and be divisible by the shift.
    pub(crate) fn new(normalized_value: Vec<Word>, ring: &'a ModuloRingLarge) -> Self {
        debug_assert!(ring.is_valid(&normalized_value));
        ModuloLarge {
            ring,
            normalized_value,
        }
    }

    /// Get the ring.
    pub(crate) fn ring(&self) -> &'a ModuloRingLarge {
        self.ring
    }

    /// Get normalized value.
    pub(crate) fn normalized_value(&self) -> &[Word] {
        &self.normalized_value
    }

    /// Modify normalized value.
    pub(crate) fn modify_normalized_value<F>(&mut self, f: F)
    where
        F: FnOnce(&mut [Word], &ModuloRingLarge),
    {
        f(&mut self.normalized_value, self.ring);
        debug_assert!(self.ring.is_valid(&self.normalized_value));
    }

    /// Checks that two values are from the same ring.
    pub(crate) fn check_same_ring(&self, other: &ModuloLarge) {
        if self.ring() != other.ring() {
            Modulo::panic_different_rings();
        }
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
            ModuloRepr::Small(modulo_small) => ModuloRepr::Small(modulo_small.clone()),
            ModuloRepr::Large(modulo_large) => ModuloRepr::Large(modulo_large.clone()),
        }
    }

    #[inline]
    fn clone_from(&mut self, source: &Self) {
        if let (ModuloRepr::Large(modulo_large), ModuloRepr::Large(source_large)) =
            (&mut *self, source)
        {
            modulo_large.clone_from(source_large);
        } else {
            *self = source.clone();
        }
    }
}

impl Clone for ModuloLarge<'_> {
    fn clone(&self) -> Self {
        ModuloLarge {
            ring: self.ring,
            normalized_value: self.normalized_value.clone(),
        }
    }

    fn clone_from(&mut self, source: &Self) {
        self.ring = source.ring;
        if self.normalized_value.len() == source.normalized_value.len() {
            self.normalized_value
                .copy_from_slice(&source.normalized_value)
        } else {
            // We don't want to have spare capacity, so do not clone_from.
            self.normalized_value = source.normalized_value.clone();
        }
    }
}
