//! NTT primes and constants for 64-bit Word targets.
//!
//! Uses Proth primes of the form `K * 2^N + 1`.
//! All constants computed by `integer/src/mul/ntt/compute_constants.py`.

use num_modular::FixedProth64;

// Proth reducer instances — each with a different (N, K) pair.
pub const P0: FixedProth64<57, 29> = FixedProth64::<57, 29>;
pub const P1: FixedProth64<57, 71> = FixedProth64::<57, 71>;
pub const P2: FixedProth64<57, 75> = FixedProth64::<57, 75>;

// Type aliases needed by for_each_prime! macro in transform tests.
pub type Rp0 = FixedProth64<57, 29>;
pub type Rp1 = FixedProth64<57, 71>;
pub type Rp2 = FixedProth64<57, 75>;

pub const K: usize = 3;
pub const MAX_LOG_N: u32 = 57;
pub const B_PACK_MIN: u32 = 16;
pub const B_PACK_CANDIDATES: &[u32] = &[64, 32, 16];

pub type Lane = u64;
pub type DoubleLane = u128;

/// Primitive `MAX_LOG_N`-th roots of unity for each prime:
/// `omega_max[i]` = `g^{(p_i-1) / 2^MAX_LOG_N} mod p_i`.
pub const OMEGA_MAX: [Lane; K] = [
    0x00003e6b41437d93, // P0
    0x2f754195e85edc63, // P1
    0x75544cac36cebb29, // P2
];

pub const CRT_INV_IJ: [[Lane; K]; K] = [
    [0, 0x3979e79e79e79e7c, 0x8c37a6f4de9bd37d],
    [0, 0, 0x2580000000000013],
    [0, 0, 0],
];

/// Prime moduli indexed by PI.
pub const MODULI: [Lane; K] = [
    FixedProth64::<57, 29>::MODULUS,
    FixedProth64::<57, 71>::MODULUS,
    FixedProth64::<57, 75>::MODULUS,
];

#[cfg(test)]
mod tests {
    use super::*;
    use num_modular::Reducer;

    #[test]
    fn test_primes_proth_form() {
        assert_eq!(MODULI[0], 29u64 * (1u64 << 57) + 1);
        assert_eq!(MODULI[1], 71u64 * (1u64 << 57) + 1);
        assert_eq!(MODULI[2], 75u64 * (1u64 << 57) + 1);
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
            let (sqr, to_m, from_m): (fn(Lane) -> Lane, fn(Lane) -> Lane, fn(Lane) -> Lane) =
                match pi {
                    0 => (
                        |w| P0.reduce((w as u128) * (w as u128)),
                        |v| P0.transform(v),
                        |v| P0.residue(v),
                    ),
                    1 => (
                        |w| P1.reduce((w as u128) * (w as u128)),
                        |v| P1.transform(v),
                        |v| P1.residue(v),
                    ),
                    2 => (
                        |w| P2.reduce((w as u128) * (w as u128)),
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
