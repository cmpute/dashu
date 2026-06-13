//! Garner CRT: combine `K` residues modulo `K` primes into a small integer.
//!
//! Takes `num_modular::Reducer` implementations so the caller can supply
//! specialized Solinas reducers for the hot per-coefficient path.
#![allow(clippy::unnecessary_cast)]

use num_modular::Reducer;

/// Subset of `Reducer<u64>` that is object-safe (no `new` or other
/// non-`&self` methods).  Implemented automatically for every
/// `Reducer<u64>` via a blanket impl.
pub trait ModOps {
    fn sub(&self, lhs: &u64, rhs: &u64) -> u64;
    fn mul(&self, lhs: &u64, rhs: &u64) -> u64;
}

impl<T: Reducer<u64>> ModOps for T {
    #[inline]
    fn sub(&self, lhs: &u64, rhs: &u64) -> u64 {
        Reducer::sub(self, lhs, rhs)
    }
    #[inline]
    fn mul(&self, lhs: &u64, rhs: &u64) -> u64 {
        Reducer::mul(self, lhs, rhs)
    }
}

/// A 192-bit unsigned integer (3 × u64, little-endian).
///
/// Used to hold Garner CRT results (which are bounded by the product of
/// three ≈2^64 primes, therefore < 2^192).
#[derive(Clone, Copy, Debug, Default)]
pub struct U192(pub [u64; 3]);

impl U192 {
    #[inline]
    pub fn new(lo: u64) -> Self {
        U192([lo, 0, 0])
    }

    /// `self += v` where `v` fits in 128 bits.
    #[inline]
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

use super::primes::{CRT_INV_IJ, PRIMES};

/// Combine `K` residues into a `U192` via Garner's algorithm.
///
/// All primes and precomputed inverses are hardcoded in [`super::primes`].
/// `reducers[i]` must be a reducer for the i-th prime.
pub fn garner_combine(residues: &[u64], reducers: &[&dyn ModOps]) -> U192 {
    let k = residues.len();
    assert!(k <= 3, "CRT supports up to 3 primes");
    assert!(reducers.len() >= k);

    let p0 = PRIMES[0].p;
    let p1 = PRIMES[1].p;
    let p2 = PRIMES[2].p;
    let mut x = U192::new(residues[0]);

    if k == 1 {
        return x;
    }

    // t_1 = (r_1 - x mod p1) * inv(p0 mod p1) mod p1
    let x_mod_p1 = x.0[0] % p1;
    let diff1 = reducers[1].sub(&residues[1], &x_mod_p1);
    let t1 = reducers[1].mul(&diff1, &CRT_INV_IJ[0][1]);
    x.add_u128((t1 as u128) * (p0 as u128));

    if k == 2 {
        return x;
    }

    // t_2 = (r_2 - x mod p2) * inv(p0*p1 mod p2) mod p2
    let x_mod_p2 = x.rem_u64(p2);
    let diff2 = reducers[2].sub(&residues[2], &x_mod_p2);
    let inv_prod = reducers[2].mul(&CRT_INV_IJ[0][2], &CRT_INV_IJ[1][2]);
    let t2 = reducers[2].mul(&diff2, &inv_prod);
    x.add_mul_u64_u128(t2, (p0 as u128) * (p1 as u128));

    x
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(not(feature = "std"))]
    use alloc::vec;

    #[test]
    fn test_garner_with_ntt_primes() {
        use super::super::primes::PRIMES;
        use num_modular::FixedTrinomialSolinas64;

        let p0 = PRIMES[0].p;
        let p1 = PRIMES[1].p;
        let p2 = PRIMES[2].p;

        let r0 = FixedTrinomialSolinas64::<64, 32, 1>::new(&p0);
        let r1 = FixedTrinomialSolinas64::<64, 34, 1>::new(&p1);
        let r2 = FixedTrinomialSolinas64::<64, 40, 1>::new(&p2);
        let reducers: [&dyn ModOps; 3] = [&r0, &r1, &r2];

        let residues = vec![12345u64, 67890u64, 11111u64];
        let x = garner_combine(&residues, &reducers);
        assert_eq!(x.rem_u64(p0), residues[0]);
        assert_eq!(x.rem_u64(p1), residues[1]);
        assert_eq!(x.rem_u64(p2), residues[2]);

        let x = garner_combine(&residues[..2], &reducers[..2]);
        assert_eq!(x.rem_u64(p0), residues[0]);
        assert_eq!(x.rem_u64(p1), residues[1]);

        let x = garner_combine(&residues[..1], &reducers[..1]);
        assert_eq!(x.0[0], residues[0]);
    }
}
