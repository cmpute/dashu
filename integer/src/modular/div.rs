use crate::{
    buffer::Buffer,
    div_const::ConstLargeDivisor,
    error::panic_divide_by_invalid_modulo,
    gcd,
    helper_macros::debug_assert_zero,
    memory::MemoryAllocation,
    primitive::{locate_top_word_plus_one, lowest_dword},
    shift::{shl_in_place, shr_in_place},
    Sign,
};

use core::ops::{Deref, Div, DivAssign};

use super::{
    add::negate_in_place,
    repr::{Reduced, ReducedDword, ReducedLarge, ReducedRepr, ReducedWord},
};
use num_modular::Reducer;

impl<'a> Reduced<'a> {
    /// Multiplicative inverse.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::{fast_div::ConstDivisor, UBig};
    /// // A Mersenne prime.
    /// let p = UBig::from(2u8).pow(127) - UBig::ONE;
    /// let ring = ConstDivisor::new(p.clone());
    /// // Fermat's little theorem: a^(p-2) = a^-1 (mod p)
    /// let a = ring.reduce(123);
    /// let ainv = a.clone().inv().unwrap();
    /// assert_eq!(ainv, a.pow(&(p - UBig::from(2u8))));
    /// assert_eq!((a * ainv).residue(), UBig::ONE);
    /// ```
    #[inline]
    pub fn inv(&self) -> Option<Reduced<'a>> {
        match self.repr() {
            ReducedRepr::Single(raw, ring) => ring
                .0
                .inv(raw.0)
                .map(|v| Reduced::from_single(ReducedWord(v), ring)),
            ReducedRepr::Double(raw, ring) => ring
                .0
                .inv(raw.0)
                .map(|v| Reduced::from_double(ReducedDword(v), ring)),
            ReducedRepr::Large(raw, ring) => {
                inv_large(ring, raw.clone()).map(|v| Reduced::from_large(v, ring))
            }
        }
    }
}

fn inv_large(ring: &ConstLargeDivisor, mut raw: ReducedLarge) -> Option<ReducedLarge> {
    // prepare modulus
    let mut modulus = Buffer::from(ring.normalized_divisor.deref());
    debug_assert_zero!(shr_in_place(&mut modulus, ring.shift));

    // prepare modulo value
    debug_assert_zero!(shr_in_place(&mut raw.0, ring.shift));
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
            let mut allocation =
                MemoryAllocation::new(gcd::memory_requirement_ext_exact(modulus.len(), raw_len));
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
    shl_in_place(&mut modulus, ring.shift);
    let mut inv = ReducedLarge(modulus.into_boxed_slice());
    debug_assert!(inv.is_valid(ring));
    if b_sign == Sign::Negative {
        negate_in_place(ring, &mut inv);
    }
    Some(inv)
}

impl<'a> Div<Reduced<'a>> for Reduced<'a> {
    type Output = Reduced<'a>;

    #[inline]
    fn div(self, rhs: Reduced<'a>) -> Reduced<'a> {
        (&self).div(&rhs)
    }
}

impl<'a> Div<&Reduced<'a>> for Reduced<'a> {
    type Output = Reduced<'a>;

    #[inline]
    fn div(self, rhs: &Reduced<'a>) -> Reduced<'a> {
        (&self).div(rhs)
    }
}

impl<'a> Div<Reduced<'a>> for &Reduced<'a> {
    type Output = Reduced<'a>;

    #[inline]
    fn div(self, rhs: Reduced<'a>) -> Reduced<'a> {
        self.div(&rhs)
    }
}

impl<'a> Div<&Reduced<'a>> for &Reduced<'a> {
    type Output = Reduced<'a>;

    #[inline]
    fn div(self, rhs: &Reduced<'a>) -> Reduced<'a> {
        // Clippy doesn't like that div is implemented using mul.
        #[allow(clippy::suspicious_arithmetic_impl)]
        match rhs.inv() {
            None => panic_divide_by_invalid_modulo(),
            Some(inv_rhs) => self * inv_rhs,
        }
    }
}

impl<'a> DivAssign<Reduced<'a>> for Reduced<'a> {
    #[inline]
    fn div_assign(&mut self, rhs: Reduced<'a>) {
        self.div_assign(&rhs)
    }
}

impl<'a> DivAssign<&Reduced<'a>> for Reduced<'a> {
    #[inline]
    fn div_assign(&mut self, rhs: &Reduced<'a>) {
        *self = (&*self).div(rhs)
    }
}
