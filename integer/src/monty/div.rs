//! Modular inverse and division.

use crate::{
    buffer::Buffer,
    error::panic_divide_by_invalid_modulo,
    gcd,
    memory::MemoryAllocation,
    primitive::{locate_top_word_plus_one, lowest_dword},
    Sign,
};

use core::ops::{Div, DivAssign};
use num_modular::Reducer;

use super::add::negate_in_place_large;
use super::mul::{mul_memory_requirement, mul_normalized_large, residue_normalized_large};
use super::repr::{Montgomery, MontgomeryInner, MontgomeryLargeRepr, MontgomeryLargeVal};

impl<'a> Montgomery<'a> {
    /// Multiplicative inverse.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::{monty::MontgomeryRepr, UBig};
    /// // A Mersenne prime.
    /// let p = UBig::from(2u8).pow(127) - UBig::ONE;
    /// let ring = MontgomeryRepr::new(p.clone());
    /// // Fermat's little theorem: a^(p-2) = a^-1 (mod p)
    /// let a = ring.reduce(123);
    /// let ainv = a.clone().inv().unwrap();
    /// assert_eq!(ainv, a.pow(&(p - UBig::from(2u8))));
    /// assert_eq!((a * ainv).residue(), UBig::ONE);
    /// ```
    #[inline]
    pub fn inv(&self) -> Option<Montgomery<'a>> {
        match self.repr() {
            MontgomeryInner::Single(raw, ring) => {
                ring.0.inv(*raw).map(|v| Montgomery::from_single(v, ring))
            }
            MontgomeryInner::Double(raw, ring) => {
                ring.0.inv(*raw).map(|v| Montgomery::from_double(v, ring))
            }
            MontgomeryInner::Large(raw, ring) => {
                inv_large(ring, raw).map(|v| Montgomery::from_large(v, ring))
            }
        }
    }
}

fn inv_large(ring: &MontgomeryLargeRepr, raw: &MontgomeryLargeVal) -> Option<MontgomeryLargeVal> {
    // 1. Exit Montgomery form to get the plain residue.
    let memory_requirement = mul_memory_requirement(ring);
    let mut allocation = MemoryAllocation::new(memory_requirement);
    let mut memory = allocation.memory();
    let residue_slice = residue_normalized_large(ring, &raw.0, &mut memory);
    let mut residue = Buffer::from(residue_slice);

    // 2. Extended GCD of (modulus, residue): bezout coefficient for residue (= the inverse)
    //    ends up stored in the modulus buffer.
    let mut modulus = Buffer::from(&ring.modulus[..]);
    let raw_len = locate_top_word_plus_one(&residue);

    let (is_g_one, b_sign) = match raw_len {
        0 => return None,
        1 => {
            let (g, _, b_sign) = gcd::gcd_ext_word(&mut modulus, residue[0]);
            (g == 1, b_sign)
        }
        2 => {
            let (g, _, b_sign) = gcd::gcd_ext_dword(&mut modulus, lowest_dword(&residue));
            (g == 1, b_sign)
        }
        _ => {
            let mut allocation =
                MemoryAllocation::new(gcd::memory_requirement_ext_exact(modulus.len(), raw_len));
            let (g_len, b_len, b_sign) = gcd::gcd_ext_in_place(
                &mut modulus,
                &mut residue[..raw_len],
                &mut allocation.memory(),
            );
            modulus[b_len..].fill(0);

            // check if inverse exists (gcd == 1)
            (g_len == 1 && residue[0] == 1, b_sign)
        }
    };
    if !is_g_one {
        return None;
    }

    // 3. Re-enter Montgomery form: REDC(|b| * R^2 mod m) = b * R mod m.
    let memory_requirement = mul_memory_requirement(ring);
    let mut allocation = MemoryAllocation::new(memory_requirement);
    let mut memory = allocation.memory();
    let monty = mul_normalized_large(ring, &modulus, &ring.r2_mod_m, &mut memory);
    let mut inv = MontgomeryLargeVal(Buffer::from(monty).into_boxed_slice());
    if b_sign == Sign::Negative {
        negate_in_place_large(ring, &mut inv);
    }
    Some(inv)
}

impl<'a> Div<Montgomery<'a>> for Montgomery<'a> {
    type Output = Montgomery<'a>;

    #[inline]
    fn div(self, rhs: Montgomery<'a>) -> Montgomery<'a> {
        (&self).div(&rhs)
    }
}

impl<'a> Div<&Montgomery<'a>> for Montgomery<'a> {
    type Output = Montgomery<'a>;

    #[inline]
    fn div(self, rhs: &Montgomery<'a>) -> Montgomery<'a> {
        (&self).div(rhs)
    }
}

impl<'a> Div<Montgomery<'a>> for &Montgomery<'a> {
    type Output = Montgomery<'a>;

    #[inline]
    fn div(self, rhs: Montgomery<'a>) -> Montgomery<'a> {
        self.div(&rhs)
    }
}

impl<'a> Div<&Montgomery<'a>> for &Montgomery<'a> {
    type Output = Montgomery<'a>;

    #[inline]
    fn div(self, rhs: &Montgomery<'a>) -> Montgomery<'a> {
        // Clippy doesn't like that div is implemented using mul.
        #[allow(clippy::suspicious_arithmetic_impl)]
        match rhs.inv() {
            None => panic_divide_by_invalid_modulo(),
            Some(inv_rhs) => self * inv_rhs,
        }
    }
}

impl<'a> DivAssign<Montgomery<'a>> for Montgomery<'a> {
    #[inline]
    fn div_assign(&mut self, rhs: Montgomery<'a>) {
        self.div_assign(&rhs)
    }
}

impl<'a> DivAssign<&Montgomery<'a>> for Montgomery<'a> {
    #[inline]
    fn div_assign(&mut self, rhs: &Montgomery<'a>) {
        *self = (&*self).div(rhs)
    }
}
