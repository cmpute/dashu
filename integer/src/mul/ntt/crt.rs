//! Garner CRT: combine `K` residues modulo `K` primes into a small integer.

use crate::arch::ntt::K;
use num_modular::ModularCoreOps;

/// Accumulator for Garner CRT.
///
/// Implemented by [`crate::arch::word::TripleWord`] (192 bits on 64-bit
/// targets, 96 bits on 32-bit targets).
pub trait CrtAccum: Default + Copy {
    type Lane: Copy
        + Default
        + Into<u128>
        + for<'a> ModularCoreOps<Self::Lane, &'a Self::Lane, Output = Self::Lane>;
    fn from_lane(v: Self::Lane) -> Self;
    /// `self += t * factor`
    fn add_product(&mut self, t: Self::Lane, factor: u128);
    /// `self mod m`
    fn rem_lane(&self, m: Self::Lane) -> Self::Lane;
    /// Write the value into `out` as little-endian `Word` values,
    /// returning the number of non-zero words written.
    fn write_words(&self, out: &mut [crate::arch::word::Word; 6]) -> u32;
}

// ── Garner combine ─────────────────────────────────────────────────────

/// Combine `K` residues into a [`CrtAccum`] via Garner's algorithm.
///
/// All arithmetic is standard-form.  `crt_inv_ij[i][j]`
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

// ── TripleWord impls (cfg-gated per arch) ─────────────────────────────

/// 64-bit: 3 × u64 = 192 bits.
#[cfg(not(any(force_bits = "32", target_pointer_width = "32")))]
mod triple_impl {
    use super::CrtAccum;
    use crate::arch::word::TripleWord;

    impl CrtAccum for TripleWord {
        type Lane = u64;

        #[inline]
        fn from_lane(v: u64) -> Self {
            TripleWord([v, 0, 0])
        }

        #[inline]
        fn add_product(&mut self, t: u64, factor: u128) {
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

        #[inline]
        fn rem_lane(&self, m: u64) -> u64 {
            let m128 = m as u128;
            let mut r: u128 = 0;
            for &word in self.0.iter().rev() {
                r = (r << 64) | (word as u128);
                r %= m128;
            }
            r as u64
        }

        #[inline]
        fn write_words(&self, out: &mut [crate::arch::word::Word; 6]) -> u32 {
            out[0] = self.0[0];
            out[1] = self.0[1];
            out[2] = self.0[2];
            if self.0[2] != 0 { 3 } else if self.0[1] != 0 { 2 } else { 1 }
        }
    }
}

/// 32-bit: 3 × u32 = 96 bits.
#[cfg(any(force_bits = "32", target_pointer_width = "32"))]
mod triple_impl {
    use super::CrtAccum;
    use crate::arch::word::TripleWord;

    impl CrtAccum for TripleWord {
        type Lane = u32;

        #[inline]
        fn from_lane(v: u32) -> Self {
            TripleWord([v, 0, 0])
        }

        #[inline]
        fn add_product(&mut self, t: u32, factor: u128) {
            let factor_lo = factor as u32;
            let factor_hi = (factor >> 32) as u32;
            let m_lo = (t as u64) * (factor_lo as u64);
            let lo = m_lo as u32;
            let m_mid = (m_lo >> 32) as u32;
            let m_hi = (t as u64) * (factor_hi as u64) + m_mid as u64;
            let mid = m_hi as u32;
            let hi = (m_hi >> 32) as u32;
            let (r0, c0) = self.0[0].overflowing_add(lo);
            self.0[0] = r0;
            let (r1, c1) = self.0[1].overflowing_add(mid.wrapping_add(c0 as u32));
            self.0[1] = r1;
            self.0[2] = self.0[2].wrapping_add(hi.wrapping_add(c1 as u32));
        }

        #[inline]
        fn rem_lane(&self, m: u32) -> u32 {
            let m64 = m as u64;
            let mut r: u64 = 0;
            for &word in self.0.iter().rev() {
                r = (r << 32) | (word as u64);
                r %= m64;
            }
            r as u32
        }

        #[inline]
        fn write_words(&self, out: &mut [crate::arch::word::Word; 6]) -> u32 {
            out[0] = self.0[0];
            out[1] = self.0[1];
            out[2] = self.0[2];
            if self.0[2] != 0 { 3 } else if self.0[1] != 0 { 2 } else { 1 }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arch::word::TripleWord;
    #[cfg(not(feature = "std"))]
    use alloc::vec;

    #[test]
    fn test_garner_roundtrip() {
        use crate::arch::ntt::{CRT_INV_IJ, MODULI};

        type Lane = <TripleWord as CrtAccum>::Lane;
        let p0 = MODULI[0];
        let p1 = MODULI[1];
        let p2 = MODULI[2];
        let primes = [p0, p1, p2];

        let residues = vec![12345u64 as Lane, 67890u64 as Lane, 11111u64 as Lane];
        let x = garner_combine::<TripleWord>(&residues, &CRT_INV_IJ, &primes);
        assert_eq!(x.rem_lane(p0), residues[0]);
        assert_eq!(x.rem_lane(p1), residues[1]);
        assert_eq!(x.rem_lane(p2), residues[2]);

        let x = garner_combine::<TripleWord>(&residues[..2], &CRT_INV_IJ, &primes);
        assert_eq!(x.rem_lane(p0), residues[0]);
        assert_eq!(x.rem_lane(p1), residues[1]);

        let x = garner_combine::<TripleWord>(&residues[..1], &CRT_INV_IJ, &primes);
        let mut buf = [crate::arch::word::Word::default(); 6];
        x.write_words(&mut buf);
        assert_eq!(buf[0] as u64, residues[0] as u64);
    }
}
