//! A ring of integers modulo a positive integer.

use super::modulo::{ModuloDoubleRaw, ModuloLargeRaw, ModuloSingleRaw};
use crate::{
    arch::word::{DoubleWord, Word},
    buffer::Buffer,
    cmp,
    error::panic_divide_by_0,
    fast_div::{
        ConstDoubleDivisor, ConstLargeDivisor, ConstSingleDivisor, FastDivideNormalized,
        FastDivideNormalized2,
    },
    math,
    primitive::shrink_dword,
    repr::{Repr, TypedRepr},
    ubig::UBig,
};
use core::cmp::Ordering;

/// A ring of integers modulo a positive integer.
///
/// # Examples
///
/// ```
/// # use dashu_int::{modular::ModuloRing, UBig};
/// let ring = ModuloRing::new(UBig::from(100u8));
/// assert_eq!(ring.modulus(), UBig::from(100u8));
/// ```
pub struct ModuloRing(ModuloRingRepr);

pub(crate) enum ModuloRingRepr {
    Single(ModuloRingSingle),
    Double(ModuloRingDouble),
    Large(ModuloRingLarge),
}

pub(crate) struct ModuloRingSingle(pub(super) ConstSingleDivisor);

pub(crate) struct ModuloRingDouble(pub(super) ConstDoubleDivisor);

pub(crate) struct ModuloRingLarge(pub(super) ConstLargeDivisor);

impl ModuloRing {
    /// Create a new ring of integers modulo `n`.
    ///
    /// For two [Modulo](crate::modular::Modulo) numbers to be compatible,
    /// they must come from the same [ModuloRing].
    /// Two different [ModuloRing]s are not compatible even if
    /// they have the same modulus `n`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::{modular::ModuloRing, UBig};
    /// let ring = ModuloRing::new(UBig::from(100u8));
    /// assert_eq!(ring.modulus(), UBig::from(100u8));
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if `n` is zero.
    #[inline]
    pub fn new(n: UBig) -> ModuloRing {
        Self(match n.into_repr() {
            TypedRepr::Small(0) => panic_divide_by_0(),
            TypedRepr::Small(dword) => {
                if let Some(word) = shrink_dword(dword) {
                    ModuloRingRepr::Single(ModuloRingSingle::new(word))
                } else {
                    ModuloRingRepr::Double(ModuloRingDouble::new(dword))
                }
            }
            TypedRepr::Large(words) => ModuloRingRepr::Large(ModuloRingLarge::new(words)),
        })
    }

    #[inline]
    pub(crate) fn repr(&self) -> &ModuloRingRepr {
        &self.0
    }
}

impl ModuloRingSingle {
    /// Create a new ring of integers modulo a single word number `n`.
    #[inline]
    pub const fn new(n: Word) -> Self {
        Self(ConstSingleDivisor::new(n))
    }

    // Directly expose this through public field?
    #[inline]
    pub const fn normalized_modulus(&self) -> Word {
        self.0.fast_div.divisor
    }

    #[inline]
    pub const fn modulus(&self) -> UBig {
        UBig(Repr::from_word(self.0.divisor()))
    }

    #[inline]
    pub const fn shift(&self) -> u32 {
        self.0.shift
    }

    #[inline]
    pub const fn fast_div(&self) -> FastDivideNormalized {
        self.0.fast_div
    }

    #[inline]
    pub const fn is_valid(&self, val: ModuloSingleRaw) -> bool {
        val.0 < self.normalized_modulus() && val.0 & math::ones_word(self.shift()) == 0
    }
}

impl ModuloRingDouble {
    /// Create a new ring of integers modulo a double word number `n`.
    #[inline]
    pub const fn new(n: DoubleWord) -> Self {
        Self(ConstDoubleDivisor::new(n))
    }

    #[inline]
    pub const fn normalized_modulus(&self) -> DoubleWord {
        self.0.fast_div.divisor
    }

    #[inline]
    pub const fn modulus(&self) -> UBig {
        UBig(Repr::from_dword(self.0.divisor()))
    }

    #[inline]
    pub const fn shift(&self) -> u32 {
        self.0.shift
    }

    #[inline]
    pub const fn fast_div(&self) -> FastDivideNormalized2 {
        self.0.fast_div
    }

    #[inline]
    pub const fn is_valid(&self, val: ModuloDoubleRaw) -> bool {
        val.0 < self.normalized_modulus() && val.0 & math::ones_dword(self.shift()) == 0
    }
}

impl ModuloRingLarge {
    /// Create a new large ring of integers modulo `n`.
    #[inline]
    pub fn new(n: Buffer) -> ModuloRingLarge {
        Self(ConstLargeDivisor::new(n))
    }

    #[inline]
    pub fn normalized_modulus(&self) -> &[Word] {
        &self.0.normalized_modulus
    }

    #[inline]
    pub fn modulus(&self) -> UBig {
        UBig(Repr::from_buffer(self.0.divisor()))
    }

    #[inline]
    pub fn shift(&self) -> u32 {
        self.0.shift
    }

    #[inline]
    pub fn fast_div_top(&self) -> FastDivideNormalized2 {
        self.0.fast_div_top
    }

    #[inline]
    pub fn is_valid(&self, val: &ModuloLargeRaw) -> bool {
        val.0.len() == self.normalized_modulus().len()
            && cmp::cmp_same_len(&val.0, self.normalized_modulus()) == Ordering::Less
            && val.0[0] & math::ones_word(self.shift()) == 0 // must be shifted
    }
}
