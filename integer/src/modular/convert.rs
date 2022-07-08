//! Conversion between Modulo, UBig and IBig.

use crate::{
    arch::word::{DoubleWord, Word},
    div,
    ibig::IBig,
    memory::MemoryAllocation,
    modular::{
        modulo::{Modulo, ModuloRepr, ModuloSingleRaw},
        modulo_ring::{ModuloRing, ModuloRingLarge, ModuloRingRepr, ModuloRingSingle},
    },
    primitive::{extend_word, split_dword, double_word},
    repr::{Buffer, TypedRepr::*, TypedReprRef::*, Repr},
    shift,
    sign::Sign::*,
    ubig::UBig, math::shl_dword,
};
use alloc::vec::Vec;
use core::iter;
use dashu_base::UnsignedAbs;

use super::modulo::ModuloLargeRaw;

impl ModuloRing {
    /// The ring modulus.
    ///
    /// # Example
    ///
    /// ```
    /// # use dashu_int::{modular::ModuloRing, ubig};
    /// let ring = ModuloRing::new(&ubig!(100));
    /// assert_eq!(ring.modulus(), ubig!(100));
    /// ```
    #[inline]
    pub fn modulus(&self) -> UBig {
        match self.repr() {
            ModuloRingRepr::Single(single) => single.modulus().into(),
            // ModuloRingRepr::Double(double) => double.modulus().into(),
            ModuloRingRepr::Large(large) => large.modulus(),
        }
    }

    /// Create an element of the ring from another type.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::{modular::ModuloRing, ubig};
    /// let ring = ModuloRing::new(&ubig!(100));
    /// let x = ring.from(-1234);
    /// let y = ring.from(ubig!(3366));
    /// assert!(x == y);
    /// ```
    #[inline]
    pub fn from<T: IntoModulo>(&self, x: T) -> Modulo {
        x.into_modulo(self)
    }
}

impl Modulo<'_> {
    /// Get the residue in range `0..n` in an n-element ring.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::{modular::ModuloRing, ubig};
    /// let ring = ModuloRing::new(&ubig!(100));
    /// let x = ring.from(-1234);
    /// assert_eq!(x.residue(), ubig!(66));
    /// ```
    #[inline]
    pub fn residue(&self) -> UBig {
        let repr = match self.repr() {
            ModuloRepr::Small(raw, ring) => Repr::from_word(raw.residue(ring)),
            ModuloRepr::Large(raw, ring) => Repr::from_buffer(raw.residue(ring)),
        };
        UBig(repr)
    }
}

impl ModuloRingSingle {
    #[inline]
    pub(crate) fn modulus(&self) -> Word {
        self.normalized_modulus() >> self.shift()
    }
}

impl ModuloRingLarge {
    pub(crate) fn modulus(&self) -> UBig {
        let normalized_modulus = self.normalized_modulus();
        let mut buffer = Buffer::allocate(normalized_modulus.len());
        buffer.push_slice(normalized_modulus);
        let low_bits = shift::shr_in_place(&mut buffer, self.shift());
        assert!(low_bits == 0);
        buffer.into()
    }
}

impl ModuloSingleRaw {
    #[inline]
    pub(crate) const fn from_word(word: Word, ring: &ModuloRingSingle) -> Self {
        let rem = if ring.shift() == 0 {
            ring.fast_div().div_rem_word(word).1
        } else {
            ring.fast_div().div_rem(extend_word(word) << ring.shift()).1
        };
        ModuloSingleRaw(rem)
    }

    #[inline]
    const fn from_dword(dword: DoubleWord, ring: &ModuloRingSingle) -> Self {
        let rem = if ring.shift() == 0 {
            ring.fast_div().div_rem(dword).1
        } else {
            let (n0, n1, n2) = shl_dword(dword, ring.shift());
            let (_, r1) = ring.fast_div().div_rem(double_word(n1, n2));
            ring.fast_div().div_rem(double_word(n0, r1)).1
        };
        ModuloSingleRaw(rem)
    }

    fn from_large(words: &[Word], ring: &ModuloRingSingle) -> Self {
        let mut rem = div::fast_rem_by_normalized_word(words, ring.fast_div());
        if ring.shift() != 0 {
            rem = ring.fast_div().div_rem(extend_word(rem) << ring.shift()).1
        }
        ModuloSingleRaw(rem)
    }

    #[inline]
    pub(crate) fn from_ubig(x: &UBig, ring: &ModuloRingSingle) -> Self {
        match x.repr() {
            RefSmall(dword) => {
                if let Ok(word) = Word::try_from(dword) {
                    Self::from_word(word, ring)
                } else {
                    Self::from_dword(dword, ring)
                }
            }
            RefLarge(words) => Self::from_large(words, ring),
        }
    }

    #[inline]
    pub(crate) fn residue(self, ring: &ModuloRingSingle) -> Word {
        debug_assert!(ring.is_valid(self));
        self.0 >> ring.shift()
    }
}

impl ModuloLargeRaw {
    pub(crate) fn from_ubig(mut x: UBig, ring: &ModuloRingLarge) -> ModuloLargeRaw {
        x <<= ring.shift() as usize;
        let modulus = ring.normalized_modulus();
        let mut vec = Vec::with_capacity(modulus.len());
        match x.into_repr() {
            Small(word) => {
                let (lo, hi) = split_dword(word);
                vec.push(lo);
                vec.push(hi);
            }
            Large(mut words) => {
                if words.len() < modulus.len() {
                    vec.extend(&*words);
                } else {
                    let mut allocation = MemoryAllocation::new(div::memory_requirement_exact(
                        words.len(),
                        modulus.len(),
                    ));
                    let mut memory = allocation.memory();
                    let _overflow = div::div_rem_in_place(
                        &mut words,
                        modulus,
                        ring.fast_div_top(),
                        &mut memory,
                    );
                    vec.extend(&words[..modulus.len()]);
                }
            }
        }
        vec.extend(iter::repeat(0).take(modulus.len() - vec.len()));
        ModuloLargeRaw(vec)
    }

    pub(crate) fn residue(&self, ring: &ModuloRingLarge) -> Buffer {
        let mut buffer = Buffer::allocate(self.0.len());
        buffer.push_slice(&self.0);
        let low_bits = shift::shr_in_place(&mut buffer, ring.shift());
        debug_assert!(low_bits == 0);
        buffer
    }
}

/// Trait for types that can be converted into [Modulo] in a [ModuloRing].
pub trait IntoModulo {
    fn into_modulo(self, ring: &ModuloRing) -> Modulo;
}

impl IntoModulo for UBig {
    #[inline]
    fn into_modulo(self, ring: &ModuloRing) -> Modulo {
        match ring.repr() {
            ModuloRingRepr::Single(ring) => Modulo::from_small(ModuloSingleRaw::from_ubig(&self, ring), ring),
            ModuloRingRepr::Large(ring) => Modulo::from_large(ModuloLargeRaw::from_ubig(self, ring), ring),
        }
    }
}

impl IntoModulo for &UBig {
    #[inline]
    fn into_modulo(self, ring: &ModuloRing) -> Modulo {
        match ring.repr() {
            ModuloRingRepr::Single(ring) => Modulo::from_small(ModuloSingleRaw::from_ubig(&self, ring), ring),
            ModuloRingRepr::Large(ring) => Modulo::from_large(ModuloLargeRaw::from_ubig(self.clone(), ring), ring),
        }
    }
}

impl IntoModulo for IBig {
    #[inline]
    fn into_modulo(self, ring: &ModuloRing) -> Modulo {
        let sign = self.sign();
        let modulo = self.unsigned_abs().into_modulo(ring);
        match sign {
            Positive => modulo,
            Negative => -modulo,
        }
    }
}

impl IntoModulo for &IBig {
    #[inline]
    fn into_modulo(self, ring: &ModuloRing) -> Modulo {
        let modulo = self.unsigned_abs().into_modulo(ring);
        match self.sign() {
            Positive => modulo,
            Negative => -modulo,
        }
    }
}

/// Implement `IntoModulo` for unsigned primitives.
macro_rules! impl_into_modulo_for_unsigned {
    ($t:ty) => {
        impl IntoModulo for $t {
            #[inline]
            fn into_modulo<'a>(self, ring: &'a ModuloRing) -> Modulo<'a> {
                UBig::from(self).into_modulo(ring)
            }
        }
    };
}

/// Implement `IntoModulo` for signed primitives.
macro_rules! impl_into_modulo_for_signed {
    ($t:ty) => {
        impl IntoModulo for $t {
            #[inline]
            fn into_modulo<'a>(self, ring: &'a ModuloRing) -> Modulo<'a> {
                IBig::from(self).into_modulo(ring)
            }
        }
    };
}

impl_into_modulo_for_unsigned!(bool);
impl_into_modulo_for_unsigned!(u8);
impl_into_modulo_for_unsigned!(u16);
impl_into_modulo_for_unsigned!(u32);
impl_into_modulo_for_unsigned!(u64);
impl_into_modulo_for_unsigned!(u128);
impl_into_modulo_for_unsigned!(usize);
impl_into_modulo_for_signed!(i8);
impl_into_modulo_for_signed!(i16);
impl_into_modulo_for_signed!(i32);
impl_into_modulo_for_signed!(i64);
impl_into_modulo_for_signed!(i128);
impl_into_modulo_for_signed!(isize);
