//! Comparisons.

use super::{
    modulo::{Modulo, ModuloRepr},
    modulo_ring::{ModuloRing, ModuloRingLarge, ModuloRingSingle},
};
use core::ptr;

use super::modulo_ring::ModuloRingDouble;

/// Equality is identity: two rings are not equal even if they have the same modulus.
impl PartialEq for ModuloRing {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        ptr::eq(self, other)
    }
}

impl Eq for ModuloRing {}

impl PartialEq for ModuloRingSingle {
    /// Equality is identity: two rings are not equal even if they have the same modulus.
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        ptr::eq(self, other)
    }
}

impl Eq for ModuloRingSingle {}

impl PartialEq for ModuloRingDouble {
    /// Equality is identity: two rings are not equal even if they have the same modulus.
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        ptr::eq(self, other)
    }
}

impl Eq for ModuloRingDouble {}

impl PartialEq for ModuloRingLarge {
    /// Equality is identity: two rings are not equal even if they have the same modulus.
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        ptr::eq(self, other)
    }
}

impl Eq for ModuloRingLarge {}

/// Equality within a ring.
///
/// # Panics
///
/// Panics if the two values are from different rings.
impl PartialEq for Modulo<'_> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        match (self.repr(), other.repr()) {
            (ModuloRepr::Single(raw0, ring0), ModuloRepr::Single(raw1, ring1)) => {
                Modulo::check_same_ring_single(ring0, ring1);
                raw0.eq(raw1)
            }
            (ModuloRepr::Double(raw0, ring0), ModuloRepr::Double(raw1, ring1)) => {
                Modulo::check_same_ring_double(ring0, ring1);
                raw0.eq(raw1)
            }
            (ModuloRepr::Large(raw0, ring0), ModuloRepr::Large(raw1, ring1)) => {
                Modulo::check_same_ring_large(ring0, ring1);
                raw0.eq(raw1)
            }
            _ => Modulo::panic_different_rings(),
        }
    }
}

impl Eq for Modulo<'_> {}
