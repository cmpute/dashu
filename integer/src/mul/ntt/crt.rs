//! Garner CRT: combine `K` residues modulo `K` primes into a small integer.
//!
//! Uses num-modular's general modular arithmetic traits since CRT is
//! called once per coefficient, not in the hot loop.
#![allow(clippy::unnecessary_cast)]

use num_modular::{ModularCoreOps, ModularUnaryOps};

/// Precomputed constants for Garner CRT with a fixed prime set.
pub struct CrtConstants {
    /// `inv(p_i mod p_j)` for i < j.
    pub inv_ij: [[u64; 3]; 3],
}

impl CrtConstants {
    /// Precompute Garner constants for the given primes.
    pub fn new(primes: &[u64]) -> Self {
        let k = primes.len();
        let mut inv_ij = [[0u64; 3]; 3];
        for i in 0..k {
            for j in (i + 1)..k {
                let p_i_mod_pj = primes[i] % primes[j];
                inv_ij[i][j] = p_i_mod_pj.invm(&primes[j]).expect("primes not coprime");
            }
        }
        CrtConstants { inv_ij }
    }
}

/// Combine `K` residues into a small integer (< P) using Garner's algorithm.
///
/// The result is returned as a little-endian `Vec<u64>` because
/// `P ≈ 2^{64K}` may exceed a single `u64`.
pub fn garner_combine(
    residues: &[u64],
    primes: &[u64],
    constants: &CrtConstants,
) -> alloc::vec::Vec<u64> {
    let k = residues.len();
    assert!(k <= 3, "CRT supports up to 3 primes");
    assert_eq!(primes.len(), k);

    let mut result = [0u64; 3];
    let p0 = primes[0];

    // x_0 = r_0
    result[0] = residues[0] % p0;

    if k == 1 {
        return result.to_vec();
    }

    // t_1 = (r_1 - x_0) * inv(p0 mod p1) mod p1
    let p1 = primes[1];
    let x0_mod_p1 = result[0] % p1;
    let diff1 = residues[1].subm(x0_mod_p1, &p1);
    let t1 = diff1.mulm(constants.inv_ij[0][1], &p1);

    // x_1 = x_0 + t_1 * p0
    add_128_to_192(&mut result, (t1 as u128) * (p0 as u128));

    if k == 2 {
        return result.to_vec();
    }

    // t_2 = (r_2 - x_1) * inv(p0*p1 mod p2) mod p2
    let p2 = primes[2];
    let x1_mod_p2 = mod_192_by_u64(&result, p2);
    let diff2 = residues[2].subm(x1_mod_p2, &p2);
    let inv_p0_mod_p2 = constants.inv_ij[0][2];
    let inv_p1_mod_p2 = constants.inv_ij[1][2];
    let inv_prod = inv_p0_mod_p2.mulm(inv_p1_mod_p2, &p2);
    let t2 = diff2.mulm(inv_prod, &p2);

    // x_2 = x_1 + t_2 * p0 * p1
    let pp = (primes[0] as u128) * (primes[1] as u128);
    let t2_64 = t2 as u64;
    let pp_lo = pp as u64;
    let pp_hi = (pp >> 64) as u64;
    let m_lo_full = (t2_64 as u128) * (pp_lo as u128);
    let m_lo = (m_lo_full >> 64) as u64;
    let lo = m_lo_full as u64;
    let m_hi_full = (t2_64 as u128) * (pp_hi as u128);
    let hi = (m_hi_full >> 64) as u64;
    let m_hi = m_hi_full as u64;
    let (mid, c) = m_lo.overflowing_add(m_hi);
    let hi_word = hi.wrapping_add(c as u64);
    let (r0, c0) = result[0].overflowing_add(lo);
    result[0] = r0;
    let (r1, c1) = result[1].overflowing_add(mid.wrapping_add(c0 as u64));
    result[1] = r1;
    result[2] = result[2].wrapping_add(hi_word.wrapping_add(c1 as u64));

    result.to_vec()
}

fn add_128_to_192(result: &mut [u64; 3], term: u128) {
    let lo = term as u64;
    let hi = (term >> 64) as u64;
    let (r0, c0) = result[0].overflowing_add(lo);
    result[0] = r0;
    let (r1, c1) = result[1].overflowing_add(hi.wrapping_add(c0 as u64));
    result[1] = r1;
    result[2] = result[2].wrapping_add(c1 as u64);
}

fn mod_192_by_u64(x: &[u64; 3], m: u64) -> u64 {
    let m128 = m as u128;
    let mut r: u128 = 0;
    for &word in x.iter().rev() {
        r = (r << 64) | (word as u128);
        r %= m128;
    }
    r as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_garner_two_primes() {
        let primes = vec![3u64, 5u64];
        let constants = CrtConstants::new(&primes);
        // x ≡ 2 mod 3, x ≡ 3 mod 5 → x = 8
        let residues = vec![2u64, 3u64];
        let result = garner_combine(&residues, &primes, &constants);
        let x = result[0] as u128;
        assert_eq!(x, 8);
    }

    #[test]
    fn test_garner_three_primes() {
        let primes = vec![3u64, 5u64, 7u64];
        let constants = CrtConstants::new(&primes);
        // x ≡ 2 (mod 3), x ≡ 3 (mod 5), x ≡ 4 (mod 7) → x = 53
        let residues = vec![2u64, 3u64, 4u64];
        let result = garner_combine(&residues, &primes, &constants);
        let x = result[0] as u128;
        assert_eq!(x, 53);
    }

    #[test]
    fn test_garner_with_ntt_primes() {
        use crate::mul::ntt::primes::PRIMES;
        let primes: Vec<u64> = PRIMES.iter().map(|np| np.p).collect();
        let constants = CrtConstants::new(&primes);
        let residues = vec![12345u64, 67890u64, 11111u64];
        let result = garner_combine(&residues, &primes, &constants);
        for (i, &p) in primes.iter().enumerate() {
            let mut rem: u128 = 0;
            for &word in result.iter().rev() {
                rem = ((rem << 64) | (word as u128)) % (p as u128);
            }
            assert_eq!(rem as u64, residues[i], "CRT mismatch for prime {i}");
        }
    }
}
