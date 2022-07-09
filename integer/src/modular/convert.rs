//! Conversion between Modulo, UBig and IBig.

use crate::{
    arch::word::{DoubleWord, Word},
    div,
    ibig::IBig,
    math::{shl_dword, self},
    memory::MemoryAllocation,
    modular::{
        modulo::{Modulo, ModuloRepr, ModuloSingleRaw},
        modulo_ring::{ModuloRing, ModuloRingLarge, ModuloRingRepr, ModuloRingSingle},
    },
    primitive::{double_word, extend_word},
    repr::{Buffer, Repr, TypedRepr::*, TypedReprRef::*},
    shift,
    sign::Sign::*,
    ubig::UBig,
};
use dashu_base::UnsignedAbs;

use super::{modulo::{ModuloLargeRaw, ModuloDoubleRaw}, modulo_ring::ModuloRingDouble};

impl ModuloRing {
    /// The ring modulus.
    ///
    /// # Example
    ///
    /// ```
    /// # use dashu_int::{modular::ModuloRing, ubig};
    /// let ring = ModuloRing::new(ubig!(100));
    /// assert_eq!(ring.modulus(), ubig!(100));
    /// ```
    #[inline]
    pub fn modulus(&self) -> UBig {
        match self.repr() {
            ModuloRingRepr::Single(single) => single.modulus().into(),
            ModuloRingRepr::Double(double) => double.modulus().into(),
            ModuloRingRepr::Large(large) => large.modulus(),
        }
    }

    // TODO: rename as convert / transform
    /// Create an element of the ring from another type.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::{modular::ModuloRing, ubig};
    /// let ring = ModuloRing::new(ubig!(100));
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
    /// let ring = ModuloRing::new(ubig!(100));
    /// let x = ring.from(-1234);
    /// assert_eq!(x.residue(), ubig!(66));
    /// ```
    #[inline]
    pub fn residue(&self) -> UBig {
        let repr = match self.repr() {
            ModuloRepr::Single(raw, ring) => Repr::from_word(raw.residue(ring)),
            ModuloRepr::Double(raw, ring) => Repr::from_dword(raw.residue(ring)),
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

impl ModuloRingDouble {
    #[inline]
    pub(crate) fn modulus(&self) -> DoubleWord {
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
        UBig(Repr::from_buffer(buffer))
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

impl ModuloDoubleRaw {
    #[inline]
    const fn from_dword(dword: DoubleWord, ring: &ModuloRingDouble) -> Self {
        let rem = if ring.shift() == 0 {
            ring.fast_div().div_rem_dword(dword).1
        } else {
            let (n0, n1, n2) = shl_dword(dword, ring.shift());
            ring.fast_div().div_rem(n0, double_word(n1, n2)).1
        };
        ModuloDoubleRaw(rem)
    }

    fn from_large(words: &[Word], ring: &ModuloRingDouble) -> Self {
        let mut rem = div::fast_rem_by_normalized_dword(words, ring.fast_div());
        if ring.shift() != 0 {
            let (r0, r1, r2) = shl_dword(rem, ring.shift());
            rem = ring.fast_div().div_rem(r0, double_word(r1, r2)).1
        }
        ModuloDoubleRaw(rem)
    }
    
    #[inline]
    pub(crate) fn from_ubig(x: &UBig, ring: &ModuloRingDouble) -> Self {
        match x.repr() {
            RefSmall(dword) => Self::from_dword(dword, ring),
            RefLarge(words) => Self::from_large(words, ring),
        }
    }

    #[inline]
    pub(crate) fn residue(self, ring: &ModuloRingDouble) -> DoubleWord {
        debug_assert!(ring.is_valid(self));
        self.0 >> ring.shift()
    }
}

impl ModuloLargeRaw {
    pub(crate) fn from_ubig(x: UBig, ring: &ModuloRingLarge) -> ModuloLargeRaw {
        let modulus = ring.normalized_modulus();
        let mut buffer = match x.into_repr() {
            Small(dword) => {
                let (lo, mid, hi) = math::shl_dword(dword, ring.shift());
                let mut buffer = Buffer::allocate_exact(modulus.len());
                buffer.push(lo);
                buffer.push(mid);
                buffer.push(hi);

                // because ModuloLarge is used only for integer with more than two words,
                // word << ring.shift() must be smaller than the normalized modulus
                buffer
            }
            Large(mut words) => {
                // normalize
                let carry = shift::shl_in_place(&mut words, ring.shift());
                if carry != 0 {
                    words.push_resizing(carry);
                }

                // reduce
                if words.len() >= modulus.len() {
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
                    words.truncate(modulus.len());
                }
                words.ensure_capacity_exact(modulus.len());
                words
            }
        };
        dbg!(modulus.len(), buffer.len());
        buffer.push_zeros(modulus.len() - buffer.len());
        ModuloLargeRaw(buffer.into_boxed_slice())
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
            ModuloRingRepr::Single(ring) => {
                Modulo::from_single(ModuloSingleRaw::from_ubig(&self, ring), ring)
            }
            ModuloRingRepr::Double(ring) => {
                Modulo::from_double(ModuloDoubleRaw::from_ubig(&self, ring), ring)
            }
            ModuloRingRepr::Large(ring) => {
                Modulo::from_large(ModuloLargeRaw::from_ubig(self, ring), ring)
            }
        }
    }
}

impl IntoModulo for &UBig {
    #[inline]
    fn into_modulo(self, ring: &ModuloRing) -> Modulo {
        match ring.repr() {
            ModuloRingRepr::Single(ring) => {
                Modulo::from_single(ModuloSingleRaw::from_ubig(&self, ring), ring)
            }
            ModuloRingRepr::Double(ring) => {
                Modulo::from_double(ModuloDoubleRaw::from_ubig(&self, ring), ring)
            }
            ModuloRingRepr::Large(ring) => {
                Modulo::from_large(ModuloLargeRaw::from_ubig(self.clone(), ring), ring)
            }
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
