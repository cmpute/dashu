//! A ring of integers modulo a positive integer.

use crate::{
    arch::word::{DoubleWord, Word},
    assert::debug_assert_in_const_fn,
    cmp, div,
    fast_divide::{FastDivideNormalized, FastDivideNormalized2},
    math,
    primitive::{shrink_dword, split_dword},
    repr::TypedReprRef,
    ubig::UBig,
};
use alloc::vec::Vec;
use core::cmp::Ordering;

use super::modulo::{ModuloLargeRaw, ModuloSingleRaw};

/// A ring of integers modulo a positive integer.
///
/// # Examples
///
/// ```
/// # use dashu_int::{modular::ModuloRing, ubig};
/// let ring = ModuloRing::new(&ubig!(100));
/// assert_eq!(ring.modulus(), ubig!(100));
/// ```
pub struct ModuloRing(ModuloRingRepr);

pub(crate) enum ModuloRingRepr {
    Single(ModuloRingSingle),
    // Double(ModuloRingDouble),
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
    normalized_modulus: Vec<Word>, // TODO: use Box<[Word]>
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
    /// let ring = ModuloRing::new(&ubig!(100));
    /// assert_eq!(ring.modulus(), ubig!(100));
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if `n` is zero.
    #[inline]
    pub fn new(n: &UBig) -> ModuloRing {
        match n.repr() {
            TypedReprRef::RefSmall(0) => panic!("modulus cannot be 0"),
            TypedReprRef::RefSmall(dword) => {
                if let Some(word) = shrink_dword(dword) {
                    ModuloRing(ModuloRingRepr::Single(ModuloRingSingle::new(word)))
                } else {
                    // ModuloRing(ModuloRingRepr::Double(ModuloRingDouble::new(dword)))
                    // TODO: bandaid here
                    let (lo, hi) = split_dword(dword);
                    let dword_slice: [Word; 2] = [lo, hi];
                    ModuloRing(ModuloRingRepr::Large(ModuloRingLarge::new(&dword_slice)))
                }
            }
            TypedReprRef::RefLarge(words) => {
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
    pub(crate) const fn new(n: Word) -> ModuloRingSingle {
        debug_assert_in_const_fn!(n != 0);
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
    pub(crate) const fn normalized_modulus(&self) -> Word {
        self.normalized_modulus
    }

    #[inline]
    pub(crate) const fn shift(&self) -> u32 {
        self.shift
    }

    #[inline]
    pub(crate) const fn fast_div(&self) -> FastDivideNormalized {
        self.fast_div
    }

    #[inline]
    pub(crate) const fn is_valid(&self, val: ModuloSingleRaw) -> bool {
        val.0 < self.normalized_modulus && val.0 & math::ones_word(self.shift) == 0
    }
}

impl ModuloRingDouble {
    /// Create a new ring of integers modulo a double word number `n`.
    #[inline]
    pub(crate) const fn new(n: DoubleWord) -> ModuloRingDouble {
        debug_assert_in_const_fn!(n > Word::MAX as DoubleWord);
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
    pub(crate) const fn normalized_modulus(&self) -> DoubleWord {
        self.normalized_modulus
    }

    #[inline]
    pub(crate) const fn shift(&self) -> u32 {
        self.shift
    }

    #[inline]
    pub(crate) const fn fast_div(&self) -> FastDivideNormalized2 {
        self.fast_div
    }
}

impl ModuloRingLarge {
    /// Create a new large ring of integers modulo `n`.
    fn new(n: &[Word]) -> ModuloRingLarge {
        let mut normalized_modulus = n.to_vec();
        let (shift, fast_div_top) = div::normalize_large(&mut normalized_modulus);
        ModuloRingLarge {
            normalized_modulus,
            shift,
            fast_div_top,
        }
    }

    pub(crate) fn normalized_modulus(&self) -> &[Word] {
        &self.normalized_modulus
    }

    pub(crate) fn shift(&self) -> u32 {
        self.shift
    }

    pub(crate) fn fast_div_top(&self) -> FastDivideNormalized2 {
        self.fast_div_top
    }

    pub(crate) fn is_valid(&self, val: &ModuloLargeRaw) -> bool {
        val.0.len() == self.normalized_modulus.len()
            && cmp::cmp_same_len(&val.0, &self.normalized_modulus) == Ordering::Less
            && val.0[0] & math::ones_word(self.shift) == 0
    }
}
