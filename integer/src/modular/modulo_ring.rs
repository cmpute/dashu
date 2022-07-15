//! A ring of integers modulo a positive integer.

use super::modulo::{ModuloDoubleRaw, ModuloLargeRaw, ModuloSingleRaw};
use crate::{
    arch::word::{DoubleWord, Word},
    buffer::Buffer,
    cmp, div,
    fast_divide::{FastDivideNormalized, FastDivideNormalized2},
    math,
    primitive::shrink_dword,
    repr::TypedRepr,
    ubig::UBig,
};
use alloc::boxed::Box;
use core::cmp::Ordering;

/// A ring of integers modulo a positive integer.
///
/// # Examples
///
/// ```
/// # use dashu_int::{modular::ModuloRing, ubig};
/// let ring = ModuloRing::new(ubig!(100));
/// assert_eq!(ring.modulus(), ubig!(100));
/// ```
pub struct ModuloRing(ModuloRingRepr);

pub(crate) enum ModuloRingRepr {
    Single(ModuloRingSingle),
    Double(ModuloRingDouble),
    Large(ModuloRingLarge),
}

pub(crate) struct ModuloRingSingle {
    normalized_modulus: Word,
    shift: u32,
    fast_div: FastDivideNormalized,
}

pub(crate) struct ModuloRingDouble {
    normalized_modulus: DoubleWord,
    shift: u32,
    fast_div: FastDivideNormalized2,
}

pub(crate) struct ModuloRingLarge {
    normalized_modulus: Box<[Word]>,
    shift: u32,
    fast_div_top: FastDivideNormalized2,
}

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
    /// # use dashu_int::{modular::ModuloRing, ubig};
    /// let ring = ModuloRing::new(ubig!(100));
    /// assert_eq!(ring.modulus(), ubig!(100));
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if `n` is zero.
    #[inline]
    pub fn new(n: UBig) -> ModuloRing {
        match n.into_repr() {
            TypedRepr::Small(0) => panic!("modulus cannot be 0"),
            TypedRepr::Small(dword) => {
                if let Some(word) = shrink_dword(dword) {
                    ModuloRing(ModuloRingRepr::Single(ModuloRingSingle::new(word)))
                } else {
                    ModuloRing(ModuloRingRepr::Double(ModuloRingDouble::new(dword)))
                }
            }
            TypedRepr::Large(words) => {
                ModuloRing(ModuloRingRepr::Large(ModuloRingLarge::new(words)))
            }
        }
    }

    #[inline]
    pub(crate) fn repr(&self) -> &ModuloRingRepr {
        &self.0
    }
}

impl ModuloRingSingle {
    /// Create a new ring of integers modulo a single word number `n`.
    #[inline]
    pub const fn new(n: Word) -> ModuloRingSingle {
        debug_assert!(n != 0);
        let shift = n.leading_zeros();
        let normalized_modulus = n << shift;
        let fast_div = FastDivideNormalized::new(normalized_modulus);
        ModuloRingSingle {
            normalized_modulus,
            shift,
            fast_div,
        }
    }

    // Directly expose this through public field?
    #[inline]
    pub const fn normalized_modulus(&self) -> Word {
        self.normalized_modulus
    }

    #[inline]
    pub const fn shift(&self) -> u32 {
        self.shift
    }

    #[inline]
    pub const fn fast_div(&self) -> FastDivideNormalized {
        self.fast_div
    }

    #[inline]
    pub const fn is_valid(&self, val: ModuloSingleRaw) -> bool {
        val.0 < self.normalized_modulus && val.0 & math::ones_word(self.shift) == 0
    }
}

impl ModuloRingDouble {
    /// Create a new ring of integers modulo a double word number `n`.
    #[inline]
    pub const fn new(n: DoubleWord) -> ModuloRingDouble {
        debug_assert!(n > Word::MAX as DoubleWord);
        let shift = n.leading_zeros();
        let normalized_modulus = n << shift;
        let fast_div = FastDivideNormalized2::new(normalized_modulus);
        ModuloRingDouble {
            normalized_modulus,
            shift,
            fast_div,
        }
    }

    #[inline]
    pub const fn normalized_modulus(&self) -> DoubleWord {
        self.normalized_modulus
    }

    #[inline]
    pub const fn shift(&self) -> u32 {
        self.shift
    }

    #[inline]
    pub const fn fast_div(&self) -> FastDivideNormalized2 {
        self.fast_div
    }

    #[inline]
    pub const fn is_valid(&self, val: ModuloDoubleRaw) -> bool {
        val.0 < self.normalized_modulus && val.0 & math::ones_dword(self.shift) == 0
    }
}

impl ModuloRingLarge {
    /// Create a new large ring of integers modulo `n`.
    fn new(mut n: Buffer) -> ModuloRingLarge {
        let (shift, fast_div_top) = div::normalize(&mut n);
        ModuloRingLarge {
            normalized_modulus: n.into_boxed_slice(),
            shift,
            fast_div_top,
        }
    }

    pub fn normalized_modulus(&self) -> &[Word] {
        &self.normalized_modulus
    }

    pub fn shift(&self) -> u32 {
        self.shift
    }

    pub fn fast_div_top(&self) -> FastDivideNormalized2 {
        self.fast_div_top
    }

    pub fn is_valid(&self, val: &ModuloLargeRaw) -> bool {
        val.0.len() == self.normalized_modulus.len()
            && cmp::cmp_same_len(&val.0, &self.normalized_modulus) == Ordering::Less
            && val.0[0] & math::ones_word(self.shift) == 0
    }
}
