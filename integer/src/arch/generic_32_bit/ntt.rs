//! NTT primes and constants for 32-bit Word targets.
//!
//! Uses Proth primes of the form `K * 2^N + 1`.
//! All constants computed by `integer/src/mul/ntt/compute_constants.py`.

use num_modular::FixedProth32;

// Proth reducer instances — each with a different (N, K) pair.
pub type Rp0 = FixedProth32<26, 7>;
pub type Rp1 = FixedProth32<27, 15>;
pub type Rp2 = FixedProth32<27, 17>;

pub const P0: Rp0 = FixedProth32::<26, 7>;
pub const P1: Rp1 = FixedProth32::<27, 15>;
pub const P2: Rp2 = FixedProth32::<27, 17>;

pub const K: usize = 3;
pub const MAX_LOG_N: u32 = 26;
pub const B_PACK_MIN: u32 = 8;
pub const B_PACK_CANDIDATES: &[u32] = &[32, 16, 8];

pub type Lane = u32;

/// Primitive `MAX_LOG_N`-th roots of unity for each prime.
pub const OMEGA_MAX: [Lane; K] = [
    0x0000088b, // P0
    0x3a26eef8, // P1
    0x1aa0ab5e, // P2
];

pub const CRT_INV_IJ: [[Lane; K]; K] = [[0, 0x4e42c85b, 0x5fb425ef], [0, 0, 0x44000009], [0, 0, 0]];

/// Prime moduli indexed by PI.
pub const MODULI: [Lane; K] = [Rp0::MODULUS, Rp1::MODULUS, Rp2::MODULUS];

#[cfg(test)]
mod tests {
    use super::*;
    use num_modular::Reducer;

    type ReducerFns = (fn(Lane) -> Lane, fn(Lane) -> Lane, fn(Lane) -> Lane);

    #[test]
    fn test_primes_proth_form() {
        assert_eq!(MODULI[0], 7u32 * (1u32 << 26) + 1);
        assert_eq!(MODULI[1], 15u32 * (1u32 << 27) + 1);
        assert_eq!(MODULI[2], 17u32 * (1u32 << 27) + 1);
    }

    #[test]
    fn test_primes_v2() {
        for &p in &MODULI {
            let v2 = (p - 1).trailing_zeros();
            assert!(v2 >= MAX_LOG_N, "v2(p-1) = {v2} < MAX_LOG_N");
        }
    }

    #[test]
    fn test_omega_order() {
        for (pi, &omega_max) in OMEGA_MAX.iter().enumerate() {
            let p = MODULI[pi];
            let (sqr, to_m, from_m): ReducerFns =
                match pi {
                    0 => (
                        |w| P0.reduce((w as u64) * (w as u64)),
                        |v| P0.transform(v),
                        |v| P0.residue(v),
                    ),
                    1 => (
                        |w| P1.reduce((w as u64) * (w as u64)),
                        |v| P1.transform(v),
                        |v| P1.residue(v),
                    ),
                    2 => (
                        |w| P2.reduce((w as u64) * (w as u64)),
                        |v| P2.transform(v),
                        |v| P2.residue(v),
                    ),
                    _ => unreachable!(),
                };

            let mut w = to_m(omega_max);
            for _ in 0..MAX_LOG_N - 1 {
                w = sqr(w);
            }
            assert_eq!(from_m(w), p - 1, "omega^(2^(MAX_LOG_N-1)) != -1 mod p for prime {pi}");
            w = sqr(w);
            assert_eq!(from_m(w), 1, "omega^(2^MAX_LOG_N) != 1 mod p for prime {pi}");
        }
    }
}
