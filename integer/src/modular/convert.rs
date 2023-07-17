//! Conversion between Modulo, UBig and IBig.

use crate::{
    arch::word::{DoubleWord, Word},
    buffer::Buffer,
    div_const::{ConstDivisorRepr, ConstDoubleDivisor, ConstLargeDivisor, ConstSingleDivisor},
    fast_div::ConstDivisor,
    helper_macros::debug_assert_zero,
    ibig::IBig,
    primitive::shrink_dword,
    repr::{Repr, TypedReprRef::*},
    shift,
    ubig::UBig,
    Sign::*,
};
use dashu_base::UnsignedAbs;
use num_modular::Reducer;

use super::repr::{Reduced, ReducedDword, ReducedLarge, ReducedRepr, ReducedWord};

impl Reduced<'_> {
    /// Get the residue in range `0..n` in an n-element ring.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::{fast_div::ConstDivisor, UBig};
    /// let ring = ConstDivisor::new(UBig::from(100u8));
    /// let x = ring.reduce(-1234);
    /// assert_eq!(x.residue(), UBig::from(66u8));
    /// ```
    #[inline]
    pub fn residue(&self) -> UBig {
        let repr = match self.repr() {
            ReducedRepr::Single(raw, ring) => Repr::from_word(raw.residue(ring)),
            ReducedRepr::Double(raw, ring) => Repr::from_dword(raw.residue(ring)),
            ReducedRepr::Large(raw, ring) => Repr::from_buffer(raw.residue(ring)),
        };
        UBig(repr)
    }

    /// Get the modulus of the ring that this element belongs to.
    pub fn modulus(&self) -> UBig {
        let repr = match self.repr() {
            ReducedRepr::Single(_, ring) => Repr::from_word(ring.divisor()),
            ReducedRepr::Double(_, ring) => Repr::from_dword(ring.divisor()),
            ReducedRepr::Large(_, ring) => Repr::from_buffer(ring.divisor()),
        };
        UBig(repr)
    }
}

impl ReducedWord {
    #[inline]
    pub fn from_word(word: Word, ring: &ConstSingleDivisor) -> Self {
        Self(ring.0.transform(word))
    }

    #[inline]
    pub fn from_ubig(x: &UBig, ring: &ConstSingleDivisor) -> Self {
        Self(match x.repr() {
            RefSmall(dword) => {
                if let Some(word) = shrink_dword(dword) {
                    ring.rem_word(word)
                } else {
                    ring.rem_dword(dword)
                }
            }
            RefLarge(words) => ring.rem_large(words),
        })
    }

    #[inline]
    pub fn residue(self, ring: &ConstSingleDivisor) -> Word {
        debug_assert!(self.is_valid(ring));
        self.0 >> ring.shift()
    }
}

impl ReducedDword {
    #[inline]
    pub fn from_ubig(x: &UBig, ring: &ConstDoubleDivisor) -> Self {
        Self(match x.repr() {
            RefSmall(dword) => ring.rem_dword(dword),
            RefLarge(words) => ring.rem_large(words),
        })
    }

    #[inline]
    pub fn residue(self, ring: &ConstDoubleDivisor) -> DoubleWord {
        debug_assert!(self.is_valid(ring));
        self.0 >> ring.shift()
    }
}

impl ReducedLarge {
    pub fn from_ubig(x: UBig, ring: &ConstLargeDivisor) -> ReducedLarge {
        let mut buffer = ring.rem_repr(x.into_repr());
        let modulus_len = ring.normalized_divisor.len();
        buffer.ensure_capacity_exact(modulus_len);
        buffer.push_zeros(modulus_len - buffer.len());
        Self(buffer.into_boxed_slice())
    }

    pub fn residue(&self, ring: &ConstLargeDivisor) -> Buffer {
        let mut buffer: Buffer = self.0.as_ref().into();
        debug_assert_zero!(shift::shr_in_place(&mut buffer, ring.shift));
        buffer
    }
}

/// Trait for types that can be converted into an [IntoRing::RingElement] by a `RingReducer`.
pub trait IntoRing<'a, RingReducer> {
    type RingElement: 'a;
    fn into_ring(self, reducer: &'a RingReducer) -> Self::RingElement;
}

impl<'a> IntoRing<'a, ConstDivisor> for UBig {
    type RingElement = Reduced<'a>;
    #[inline]
    fn into_ring(self, ring: &ConstDivisor) -> Reduced {
        match &ring.0 {
            ConstDivisorRepr::Single(ring) => {
                Reduced::from_single(ReducedWord::from_ubig(&self, ring), ring)
            }
            ConstDivisorRepr::Double(ring) => {
                Reduced::from_double(ReducedDword::from_ubig(&self, ring), ring)
            }
            ConstDivisorRepr::Large(ring) => {
                Reduced::from_large(ReducedLarge::from_ubig(self, ring), ring)
            }
        }
    }
}

impl<'a> IntoRing<'a, ConstDivisor> for IBig {
    type RingElement = Reduced<'a>;

    #[inline]
    fn into_ring(self, ring: &ConstDivisor) -> Reduced {
        let sign = self.sign();
        let modulo = self.unsigned_abs().into_ring(ring);
        match sign {
            Positive => modulo,
            Negative => -modulo,
        }
    }
}

/// Implement `IntoModulo` for unsigned primitives.
macro_rules! impl_into_ring_for_unsigned {
    ($t:ty) => {
        impl<'a> IntoRing<'a, ConstDivisor> for $t {
            type RingElement = Reduced<'a>;
            #[inline]
            fn into_ring(self, ring: &'a ConstDivisor) -> Reduced<'a> {
                UBig::from(self).into_ring(ring)
            }
        }
    };
}

/// Implement `IntoModulo` for signed primitives.
macro_rules! impl_into_modulo_for_signed {
    ($t:ty) => {
        impl<'a> IntoRing<'a, ConstDivisor> for $t {
            type RingElement = Reduced<'a>;
            #[inline]
            fn into_ring(self, ring: &'a ConstDivisor) -> Reduced<'a> {
                IBig::from(self).into_ring(ring)
            }
        }
    };
}

impl_into_ring_for_unsigned!(bool);
impl_into_ring_for_unsigned!(u8);
impl_into_ring_for_unsigned!(u16);
impl_into_ring_for_unsigned!(u32);
impl_into_ring_for_unsigned!(u64);
impl_into_ring_for_unsigned!(u128);
impl_into_ring_for_unsigned!(usize);
impl_into_modulo_for_signed!(i8);
impl_into_modulo_for_signed!(i16);
impl_into_modulo_for_signed!(i32);
impl_into_modulo_for_signed!(i64);
impl_into_modulo_for_signed!(i128);
impl_into_modulo_for_signed!(isize);

impl ConstDivisor {
    /// Create an element of the modulo ring from another type.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::{fast_div::ConstDivisor, UBig, IBig};
    /// let ring = ConstDivisor::new(UBig::from(100u8));
    /// let x = ring.reduce(-1234);
    /// let y = ring.reduce(IBig::from(3366));
    /// assert!(x == y);
    /// ```
    pub fn reduce<'a, T: IntoRing<'a, ConstDivisor, RingElement = Reduced<'a>>>(
        &'a self,
        x: T,
    ) -> Reduced {
        x.into_ring(self)
    }
}
