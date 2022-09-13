//! Conversion between Modulo, UBig and IBig.

use crate::{
    arch::word::{DoubleWord, Word},
    buffer::Buffer,
    helper_macros::debug_assert_zero,
    ibig::IBig,
    primitive::shrink_dword,
    repr::{Repr, TypedReprRef::*},
    shift,
    ubig::UBig,
    Sign::*,
};
use dashu_base::UnsignedAbs;

use super::{
    modulo::{Modulo, ModuloDoubleRaw, ModuloLargeRaw, ModuloRepr, ModuloSingleRaw},
    modulo_ring::{
        ModuloRing, ModuloRingDouble, ModuloRingLarge, ModuloRingRepr, ModuloRingSingle,
    },
};

impl ModuloRing {
    /// The ring modulus.
    ///
    /// Note that there is overhead for retrieving the original modulus.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::{modular::ModuloRing, UBig};
    /// let ring = ModuloRing::new(UBig::from(100u8));
    /// assert_eq!(ring.modulus(), 100);
    /// ```
    #[inline]
    pub fn modulus(&self) -> UBig {
        match self.repr() {
            ModuloRingRepr::Single(single) => single.modulus(),
            ModuloRingRepr::Double(double) => double.modulus(),
            ModuloRingRepr::Large(large) => large.modulus(),
        }
    }

    /// Create an element of the ring from another type.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::{modular::ModuloRing, UBig};
    /// let ring = ModuloRing::new(UBig::from(100u8));
    /// let x = ring.convert(-1234);
    /// let y = ring.convert(UBig::from(3366u32));
    /// assert!(x == y);
    /// ```
    #[inline]
    pub fn convert<T: IntoModulo>(&self, x: T) -> Modulo {
        x.into_modulo(self)
    }
}

impl Modulo<'_> {
    /// Get the residue in range `0..n` in an n-element ring.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::{modular::ModuloRing, UBig};
    /// let ring = ModuloRing::new(UBig::from(100u8));
    /// let x = ring.convert(-1234);
    /// assert_eq!(x.residue(), 66);
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

impl ModuloSingleRaw {
    #[inline]
    pub const fn from_word(word: Word, ring: &ModuloRingSingle) -> Self {
        Self(ring.0.rem_word(word))
    }

    #[inline]
    pub fn from_ubig(x: &UBig, ring: &ModuloRingSingle) -> Self {
        Self(match x.repr() {
            RefSmall(dword) => {
                if let Some(word) = shrink_dword(dword) {
                    ring.0.rem_word(word)
                } else {
                    ring.0.rem_dword(dword)
                }
            }
            RefLarge(words) => ring.0.rem_large(words),
        })
    }

    #[inline]
    pub fn residue(self, ring: &ModuloRingSingle) -> Word {
        debug_assert!(ring.is_valid(self));
        self.0 >> ring.shift()
    }
}

impl ModuloDoubleRaw {
    #[inline]
    pub fn from_ubig(x: &UBig, ring: &ModuloRingDouble) -> Self {
        Self(match x.repr() {
            RefSmall(dword) => ring.0.rem_dword(dword),
            RefLarge(words) => ring.0.rem_large(words),
        })
    }

    #[inline]
    pub fn residue(self, ring: &ModuloRingDouble) -> DoubleWord {
        debug_assert!(ring.is_valid(self));
        self.0 >> ring.shift()
    }
}

impl ModuloLargeRaw {
    pub fn from_ubig(x: UBig, ring: &ModuloRingLarge) -> ModuloLargeRaw {
        let mut buffer = ring.0.rem_repr(x.into_repr());
        let modulus_len = ring.normalized_modulus().len();
        buffer.ensure_capacity_exact(modulus_len);
        buffer.push_zeros(modulus_len - buffer.len());
        Self(buffer.into_boxed_slice())
    }

    pub fn residue(&self, ring: &ModuloRingLarge) -> Buffer {
        let mut buffer: Buffer = self.0.as_ref().into();
        debug_assert_zero!(shift::shr_in_place(&mut buffer, ring.shift()));
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
                Modulo::from_single(ModuloSingleRaw::from_ubig(self, ring), ring)
            }
            ModuloRingRepr::Double(ring) => {
                Modulo::from_double(ModuloDoubleRaw::from_ubig(self, ring), ring)
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
