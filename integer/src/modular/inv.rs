use dashu_base::ExtendedGcd;

use crate::{
    buffer::Buffer,
    gcd,
    helper_macros::debug_assert_zero,
    memory::MemoryAllocation,
    primitive::{locate_top_word_plus_one, lowest_dword, PrimitiveSigned},
    shift::{shl_in_place, shr_in_place},
    Sign,
};

use super::{
    modulo::{Modulo, ModuloDoubleRaw, ModuloLargeRaw, ModuloRepr, ModuloSingleRaw},
    modulo_ring::{ModuloRingDouble, ModuloRingLarge, ModuloRingSingle},
};

impl<'a> Modulo<'a> {
    // TODO(v0.3): rename to inverse and take input as reference, move to div.rs to be consistent with ibig
    /// Multiplicative inverse.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::{modular::ModuloRing, UBig};
    /// // A Mersenne prime.
    /// let p = UBig::from(2u8).pow(127) - UBig::ONE;
    /// let ring = ModuloRing::new(p.clone());
    /// // Fermat's little theorem: a^(p-2) = a^-1 (mod p)
    /// let a = ring.convert(123);
    /// let ainv = a.clone().inv().unwrap();
    /// assert_eq!(ainv, a.pow(&(p - UBig::from(2u8))));
    /// assert_eq!((a * ainv).residue(), 1);
    /// ```
    #[inline]
    pub fn inverse(self) -> Option<Modulo<'a>> {
        match self.into_repr() {
            ModuloRepr::Single(raw, ring) => ring.inv(raw).map(|v| Modulo::from_single(v, ring)),
            ModuloRepr::Double(raw, ring) => ring.inv(raw).map(|v| Modulo::from_double(v, ring)),
            ModuloRepr::Large(raw, ring) => ring.inv(raw).map(|v| Modulo::from_large(v, ring)),
        }
    }
}

macro_rules! impl_mod_inv_for_primitive {
    ($ring:ty, $raw:ident) => {
        impl $ring {
            #[inline]
            /// Modular inverse.
            fn inv(&self, raw: $raw) -> Option<$raw> {
                if raw.0 == 0 {
                    return None;
                }
                let (g, _, coeff) = self.0.divisor().gcd_ext(raw.0 >> self.shift());
                if g != 1 {
                    return None;
                }
                let (sign, coeff) = coeff.to_sign_magnitude();
                let coeff = $raw(coeff << self.shift());
                if sign == Sign::Negative {
                    Some(self.negate(coeff))
                } else {
                    Some(coeff)
                }
            }
        }
    };
}
impl_mod_inv_for_primitive!(ModuloRingSingle, ModuloSingleRaw);
impl_mod_inv_for_primitive!(ModuloRingDouble, ModuloDoubleRaw);

impl ModuloRingLarge {
    #[inline]
    fn inv(&self, mut raw: ModuloLargeRaw) -> Option<ModuloLargeRaw> {
        // prepare modulus
        let mut modulus = Buffer::allocate_exact(self.normalized_modulus().len());
        modulus.push_slice(self.normalized_modulus());
        debug_assert_zero!(shr_in_place(&mut modulus, self.shift()));

        // prepare modulo value
        debug_assert_zero!(shr_in_place(&mut raw.0, self.shift()));
        let raw_len = locate_top_word_plus_one(&raw.0);

        // call extended gcd
        let (is_g_one, b_sign) = match raw_len {
            0 => return None,
            1 => {
                let (g, _, b_sign) = gcd::gcd_ext_word(&mut modulus, *raw.0.first().unwrap());
                (g == 1, b_sign)
            }
            2 => {
                let (g, _, b_sign) = gcd::gcd_ext_dword(&mut modulus, lowest_dword(&raw.0));
                (g == 1, b_sign)
            }
            _ => {
                let mut allocation = MemoryAllocation::new(gcd::memory_requirement_ext_exact(
                    modulus.len(),
                    raw_len,
                ));
                let (g_len, b_len, b_sign) = gcd::gcd_ext_in_place(
                    &mut modulus,
                    &mut raw.0[..raw_len],
                    &mut allocation.memory(),
                );
                modulus[b_len..].fill(0);

                // check if inverse exists
                (g_len == 1 && *raw.0.first().unwrap() == 1, b_sign)
            }
        };
        if !is_g_one {
            return None;
        }

        // return inverse
        shl_in_place(&mut modulus, self.shift());
        let mut inv = ModuloLargeRaw(modulus.into_boxed_slice());
        debug_assert!(self.is_valid(&inv));
        if b_sign == Sign::Negative {
            self.negate_in_place(&mut inv);
        }
        Some(inv)
    }
}
