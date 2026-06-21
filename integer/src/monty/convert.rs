//! Conversion between Montgomery form, UBig and IBig.

use crate::{
    arch::word::{DoubleWord, Word},
    buffer::Buffer,
    ibig::IBig,
    memory::MemoryAllocation,
    primitive::shrink_dword,
    repr::{Repr, TypedReprRef},
    ubig::UBig,
    Sign,
};
use dashu_base::UnsignedAbs;
use num_modular::Reducer;

use super::mul::{mul_memory_requirement, mul_normalized_large, residue_normalized_large};
use super::repr::{
    to_exact_words, Montgomery, MontgomeryInner, MontgomeryLargeVal, MontgomeryRepr,
    MontgomeryReprData,
};

impl Montgomery<'_> {
    /// Get the residue in range `0..m`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::{monty::MontgomeryRepr, UBig};
    /// let ring = MontgomeryRepr::new(UBig::from(101u8));
    /// let x = ring.reduce(UBig::from(234u8));
    /// assert_eq!(x.residue(), UBig::from(32u8));
    /// ```
    #[inline]
    pub fn residue(&self) -> UBig {
        match self.repr() {
            MontgomeryInner::Single(raw, ring) => UBig::from_word(ring.0.residue(*raw)),
            MontgomeryInner::Double(raw, ring) => UBig::from_dword(ring.0.residue(*raw)),
            MontgomeryInner::Large(raw, ring) => {
                let memory_requirement = mul_memory_requirement(ring);
                let mut allocation = MemoryAllocation::new(memory_requirement);
                let mut memory = allocation.memory();
                let res = residue_normalized_large(ring, &raw.0, &mut memory);
                UBig(Repr::from_buffer(Buffer::from(res)))
            }
        }
    }

    /// Get the modulus of the ring that this element belongs to.
    pub fn modulus(&self) -> UBig {
        match self.repr() {
            MontgomeryInner::Single(_, ring) => UBig::from_word(ring.0.modulus()),
            MontgomeryInner::Double(_, ring) => UBig::from_dword(ring.0.modulus()),
            MontgomeryInner::Large(_, ring) => {
                UBig(Repr::from_buffer(Buffer::from(&ring.modulus[..])))
            }
        }
    }
}

/// Trait for types that can be converted into a [`Montgomery`] value by a [`MontgomeryRepr`].
pub trait IntoMontgomeryRing<'a, Ring> {
    type RingElement: 'a;
    fn into_monty(self, ring: &'a Ring) -> Self::RingElement;
}

impl<'a> IntoMontgomeryRing<'a, MontgomeryRepr> for UBig {
    type RingElement = Montgomery<'a>;

    #[inline]
    fn into_monty(self, ring: &'a MontgomeryRepr) -> Montgomery<'a> {
        match ring.data() {
            MontgomeryReprData::Single(r) => {
                let modulus = r.0.modulus();
                let residue = &self % &UBig::from_word(modulus);
                Montgomery::from_single(r.0.transform(ubig_to_word(&residue)), r)
            }
            MontgomeryReprData::Double(r) => {
                let modulus = r.0.modulus();
                let residue = &self % &UBig::from_dword(modulus);
                Montgomery::from_double(r.0.transform(ubig_to_dword(&residue)), r)
            }
            MontgomeryReprData::Large(r) => {
                let s = r.modulus.len();
                let modulus = UBig(Repr::from_buffer(Buffer::from(&r.modulus[..])));
                let residue = &self % &modulus;
                let residue_words = to_exact_words(&residue, s);
                let memory_requirement = mul_memory_requirement(r);
                let mut allocation = MemoryAllocation::new(memory_requirement);
                let mut memory = allocation.memory();
                let monty = mul_normalized_large(r, &residue_words, &r.r2_mod_m, &mut memory);
                Montgomery::from_large(
                    MontgomeryLargeVal(Buffer::from(monty).into_boxed_slice()),
                    r,
                )
            }
        }
    }
}

impl<'a> IntoMontgomeryRing<'a, MontgomeryRepr> for IBig {
    type RingElement = Montgomery<'a>;

    #[inline]
    fn into_monty(self, ring: &'a MontgomeryRepr) -> Montgomery<'a> {
        let sign = self.sign();
        let modulo = self.unsigned_abs().into_monty(ring);
        match sign {
            Sign::Positive => modulo,
            Sign::Negative => -modulo,
        }
    }
}

/// Implement [`IntoMontgomeryRing`] for unsigned primitives.
macro_rules! impl_into_monty_for_unsigned {
    ($t:ty) => {
        impl<'a> IntoMontgomeryRing<'a, MontgomeryRepr> for $t {
            type RingElement = Montgomery<'a>;
            #[inline]
            fn into_monty(self, ring: &'a MontgomeryRepr) -> Montgomery<'a> {
                UBig::from(self).into_monty(ring)
            }
        }
    };
}

/// Implement [`IntoMontgomeryRing`] for signed primitives.
macro_rules! impl_into_monty_for_signed {
    ($t:ty) => {
        impl<'a> IntoMontgomeryRing<'a, MontgomeryRepr> for $t {
            type RingElement = Montgomery<'a>;
            #[inline]
            fn into_monty(self, ring: &'a MontgomeryRepr) -> Montgomery<'a> {
                IBig::from(self).into_monty(ring)
            }
        }
    };
}

impl_into_monty_for_unsigned!(bool);
impl_into_monty_for_unsigned!(u8);
impl_into_monty_for_unsigned!(u16);
impl_into_monty_for_unsigned!(u32);
impl_into_monty_for_unsigned!(u64);
impl_into_monty_for_unsigned!(u128);
impl_into_monty_for_unsigned!(usize);
impl_into_monty_for_signed!(i8);
impl_into_monty_for_signed!(i16);
impl_into_monty_for_signed!(i32);
impl_into_monty_for_signed!(i64);
impl_into_monty_for_signed!(i128);
impl_into_monty_for_signed!(isize);

impl MontgomeryRepr {
    /// Create an element of the Montgomery ring from another type.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::{monty::MontgomeryRepr, UBig, IBig};
    /// let ring = MontgomeryRepr::new(UBig::from(101u8));
    /// let x = ring.reduce(-5);
    /// let y = ring.reduce(IBig::from(96));
    /// assert!(x == y);
    /// ```
    pub fn reduce<'a, T: IntoMontgomeryRing<'a, MontgomeryRepr, RingElement = Montgomery<'a>>>(
        &'a self,
        x: T,
    ) -> Montgomery<'a> {
        x.into_monty(self)
    }
}

/// Extract a `Word` from a `UBig` known to fit in a single word.
fn ubig_to_word(u: &UBig) -> Word {
    match u.repr() {
        TypedReprRef::RefSmall(d) => shrink_dword(d).expect("value fits in a word"),
        TypedReprRef::RefLarge(_) => unreachable!("value is less than a single-word modulus"),
    }
}

/// Extract a `DoubleWord` from a `UBig` known to fit in a double word.
fn ubig_to_dword(u: &UBig) -> DoubleWord {
    match u.repr() {
        TypedReprRef::RefSmall(d) => d,
        TypedReprRef::RefLarge(_) => unreachable!("value is less than a double-word modulus"),
    }
}
