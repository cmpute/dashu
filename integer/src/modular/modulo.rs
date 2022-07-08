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

/// Single word modular value in some unknown ring. The ring must be provided to operations.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ModuloSingleRaw(pub(crate) Word);

// TODO: remove ModuloSingle / ModuloLarge, combine them directly into the ModuloRepr
#[derive(Clone, Copy, Eq)]
pub(crate) struct ModuloSingle<'a> {
    ring: &'a ModuloRingSingle,
    raw: ModuloSingleRaw,
}

/// Multi-word modular value in some unknown ring. `self.0.len() == ring.normalized_modulus.len()`
/// 
/// The vanilla `Vec` is used instead of `Buffer` here because we want fixed and compact capacity in the modulo. 
#[derive(Clone)]
pub(crate) struct ModuloLargeRaw(pub(crate) Vec<Word>); // TODO: use Box<[Word]> or Buffer

pub(crate) struct ModuloLarge<'a> {
    ring: &'a ModuloRingLarge,
    raw: ModuloLargeRaw,
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

    #[inline]
    pub(crate) const fn raw(&self) -> ModuloSingleRaw {
        self.raw
    }

    #[inline]
    pub(crate) fn set_raw(&mut self, val: ModuloSingleRaw) {
        debug_assert!(self.ring().is_valid(val));
        self.raw = val
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

impl<'a> ModuloLarge<'a> {
    /// Create new ModuloLarge.
    ///
    /// normalized_value must have the same length as the modulus, be in range 0..modulus,
    /// and be divisible by the shift.
    pub(crate) fn new(raw: ModuloLargeRaw, ring: &'a ModuloRingLarge) -> Self {
        debug_assert!(ring.is_valid(&raw));
        ModuloLarge {
            ring,
            raw,
        }
    }

    /// Get the ring.
    pub(crate) fn ring(&self) -> &'a ModuloRingLarge {
        self.ring
    }

    /// Get normalized value.
    pub(crate) fn normalized_value(&self) -> &[Word] {
        &self.raw.0
    }

    #[inline]
    pub(crate) fn raw(&self) -> &ModuloLargeRaw {
        &self.raw
    }

    #[inline]
    pub(crate) fn raw_mut(&mut self) -> &mut ModuloLargeRaw {
        &mut self.raw
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
            raw: self.raw.clone(),
        }
    }

    fn clone_from(&mut self, source: &Self) {
        self.ring = source.ring;
        if self.raw.0.len() == source.raw.0.len() {
            self.raw.0
                .copy_from_slice(&source.raw.0)
        } else {
            // We don't want to have spare capacity, so do not clone_from.
            self.raw.0 = source.raw.0.clone();
        }
    }
}
