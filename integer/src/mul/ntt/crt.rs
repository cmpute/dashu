//! Garner CRT: combine `K` residues modulo `K` primes into a small integer.
//!
//! Generic over the lane type — supports u64 (→[`U192`]) and u32 (→[`U128`]).
#![allow(clippy::unnecessary_cast)]

use crate::arch::ntt::K;
use num_modular::ModularCoreOps;

/// Accumulator for Garner CRT — either [`U192`] (64-bit lanes) or [`U96`] (32-bit).
pub trait CrtAccum: Default + Copy {
    type Lane: Copy + Into<u128> + for<'a> ModularCoreOps<Self::Lane, &'a Self::Lane, Output = Self::Lane>;
    fn from_lane(v: Self::Lane) -> Self;
    /// `self += t * factor` where `t` is a lane-sized coefficient.
    fn add_product(&mut self, t: Self::Lane, factor: u128);
    /// `self mod m`
    fn rem_lane(&self, m: Self::Lane) -> Self::Lane;
    /// Number of non-zero u64 words.
    #[allow(dead_code)]
    fn len_words(&self) -> u32;
    /// View as `&[u64]`.
    fn as_u64_slice(&self) -> &[u64];
}

// ── U192 (64-bit lanes) ────────────────────────────────────────────────

/// A 192-bit unsigned integer (3 × u64, little-endian).
///
/// Used to hold Garner CRT results for three ≈2^64 primes (product < 2^192).
#[derive(Clone, Copy, Debug, Default)]
pub struct U192(pub [u64; 3]);

impl U192 {
    #[inline]
    pub fn new(lo: u64) -> Self {
        U192([lo, 0, 0])
    }

    /// `self += v` where `v` fits in 128 bits.
    #[inline]
    #[allow(dead_code)]
    pub fn add_u128(&mut self, v: u128) {
        let lo = v as u64;
        let hi = (v >> 64) as u64;
        let (r0, c0) = self.0[0].overflowing_add(lo);
        self.0[0] = r0;
        let (r1, c1) = self.0[1].overflowing_add(hi.wrapping_add(c0 as u64));
        self.0[1] = r1;
        self.0[2] = self.0[2].wrapping_add(c1 as u64);
    }

    /// `self += t * factor`  where `t` < 2^64, `factor` < 2^128.
    #[inline]
    pub fn add_mul_u64_u128(&mut self, t: u64, factor: u128) {
        let fac_lo = factor as u64;
        let fac_hi = (factor >> 64) as u64;

        let m_lo_full = (t as u128) * (fac_lo as u128);
        let lo = m_lo_full as u64;
        let m_lo = (m_lo_full >> 64) as u64;

        let m_hi_full = (t as u128) * (fac_hi as u128);
        let m_hi = m_hi_full as u64;
        let hi = (m_hi_full >> 64) as u64;

        let (mid, c) = m_lo.overflowing_add(m_hi);
        let hi_word = hi.wrapping_add(c as u64);

        let (r0, c0) = self.0[0].overflowing_add(lo);
        self.0[0] = r0;
        let (r1, c1) = self.0[1].overflowing_add(mid.wrapping_add(c0 as u64));
        self.0[1] = r1;
        self.0[2] = self.0[2].wrapping_add(hi_word.wrapping_add(c1 as u64));
    }

    /// `self mod m`, where `m` < 2^64.
    #[inline]
    pub fn rem_u64(&self, m: u64) -> u64 {
        let m128 = m as u128;
        let mut r: u128 = 0;
        for &word in self.0.iter().rev() {
            r = (r << 64) | (word as u128);
            r %= m128;
        }
        r as u64
    }

    #[inline]
    pub fn len_words(&self) -> u32 {
        if self.0[2] != 0 {
            3
        } else if self.0[1] != 0 {
            2
        } else {
            1
        }
    }
}

impl CrtAccum for U192 {
    type Lane = u64;

    #[inline]
    fn from_lane(v: u64) -> Self {
        U192::new(v)
    }

    #[inline]
    fn add_product(&mut self, t: u64, factor: u128) {
        self.add_mul_u64_u128(t, factor);
    }

    #[inline]
    fn rem_lane(&self, m: u64) -> u64 {
        self.rem_u64(m)
    }

    #[inline]
    fn len_words(&self) -> u32 {
        self.len_words()
    }

    #[inline]
    fn as_u64_slice(&self) -> &[u64] {
        &self.0[..self.len_words() as usize]
    }
}

// ── U96 (32-bit lanes) ─────────────────────────────────────────────────

/// A value bounded by 2^96 (product of three ≈2^32 primes).
///
/// Stored as `[u64; 2]` (128 bits) so [`as_u64_slice`] can return a
/// `&[u64]` for [`add_shifted_to_prod`].  The upper 32 bits of the
/// second limb are always zero.
///
/// [`add_shifted_to_prod`]: super::add_shifted_to_prod
#[derive(Clone, Copy, Debug, Default)]
pub struct U96(pub [u64; 2]);

impl U96 {
    /// `self += t * factor` where `t` < 2^32.
    #[inline]
    pub fn add_mul_u32_u96(&mut self, t: u32, factor: u128) {
        let fac_lo = factor as u64;
        let fac_hi = (factor >> 64) as u64;

        // t × fac_lo  (max 2^32 × 2^64 = 2^96 → fits in u128)
        let m_lo_full = (t as u128) * (fac_lo as u128);
        let lo = m_lo_full as u64;
        let m_lo_carry = (m_lo_full >> 64) as u64;

        // t × fac_hi + carry
        let m_hi_full = (t as u64 as u128) * (fac_hi as u128) + m_lo_carry as u128;
        let m_hi = m_hi_full as u64;

        let (r0, c0) = self.0[0].overflowing_add(lo);
        self.0[0] = r0;
        let (r1, c1) = self.0[1].overflowing_add(m_hi.wrapping_add(c0 as u64));
        self.0[1] = r1;
        let _ = c1;
    }

    /// `self mod m`, where `m` < 2^32.
    #[inline]
    pub fn rem_u32(&self, m: u32) -> u32 {
        let m128 = m as u128;
        let mut r: u128 = 0;
        for &word in self.0.iter().rev() {
            r = (r << 64) | (word as u128);
            r %= m128;
        }
        r as u32
    }

    #[inline]
    pub fn len_words(&self) -> u32 {
        if self.0[1] != 0 {
            2
        } else {
            1
        }
    }
}

impl CrtAccum for U96 {
    type Lane = u32;

    #[inline]
    fn from_lane(v: u32) -> Self {
        U96([v as u64, 0])
    }

    #[inline]
    fn add_product(&mut self, t: u32, factor: u128) {
        self.add_mul_u32_u96(t, factor);
    }

    #[inline]
    fn rem_lane(&self, m: u32) -> u32 {
        self.rem_u32(m)
    }

    #[inline]
    fn len_words(&self) -> u32 {
        self.len_words()
    }

    #[inline]
    fn as_u64_slice(&self) -> &[u64] {
        &self.0[..self.len_words() as usize]
    }
}

// ── Garner combine ─────────────────────────────────────────────────────

/// Combine `K` residues into a [`CrtAccum`] via Garner's algorithm.
///
/// All arithmetic is standard-form (not Montgomery).  `crt_inv_ij[i][j]`
/// holds `inv(p_i mod p_j) mod p_j` for `i < j`.
/// `primes` contains the prime values (only `primes[0..k]` are used).
pub fn garner_combine<A: CrtAccum>(
    residues: &[A::Lane],
    crt_inv_ij: &[[A::Lane; K]; K],
    primes: &[A::Lane; K],
) -> A {
    let k = residues.len();
    assert!(k <= K, "CRT supports up to {K} primes");

    let p0 = primes[0];
    let p1 = primes[1];
    let p2 = primes[2];
    let mut x = A::from_lane(residues[0]);

    if k == 1 {
        return x;
    }

    // t_1 = (r_1 - x mod p1) * inv(p0 mod p1) mod p1
    let x_mod_p1 = x.rem_lane(p1);
    let diff1 = residues[1].subm(x_mod_p1, &p1);
    let t1 = diff1.mulm(crt_inv_ij[0][1], &p1);
    x.add_product(t1, p0.into());

    if k == 2 {
        return x;
    }

    // t_2 = (r_2 - x mod p2) * inv(p0*p1 mod p2) mod p2
    let x_mod_p2 = x.rem_lane(p2);
    let diff2 = residues[2].subm(x_mod_p2, &p2);
    let inv_prod = crt_inv_ij[0][2].mulm(crt_inv_ij[1][2], &p2);
    let t2 = diff2.mulm(inv_prod, &p2);
    x.add_product(t2, p0.into() * p1.into());

    x
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(not(feature = "std"))]
    use alloc::vec;

    #[test]
    fn test_garner_with_u64_primes() {
        use crate::arch::ntt::{CRT_INV_IJ, MODULI};

        let p0 = MODULI[0];
        let p1 = MODULI[1];
        let p2 = MODULI[2];
        let primes = [p0, p1, p2];

        let residues = vec![12345u64, 67890u64, 11111u64];
        let x = garner_combine::<U192>(&residues, &CRT_INV_IJ, &primes);
        assert_eq!(x.rem_u64(p0), residues[0]);
        assert_eq!(x.rem_u64(p1), residues[1]);
        assert_eq!(x.rem_u64(p2), residues[2]);

        let x = garner_combine::<U192>(&residues[..2], &CRT_INV_IJ, &primes);
        assert_eq!(x.rem_u64(p0), residues[0]);
        assert_eq!(x.rem_u64(p1), residues[1]);

        let x = garner_combine::<U192>(&residues[..1], &CRT_INV_IJ, &primes);
        assert_eq!(x.0[0], residues[0]);
    }
}
