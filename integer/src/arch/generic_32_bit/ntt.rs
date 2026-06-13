//! NTT primes and constants for 32-bit Word targets.
//!
//! Uses Proth primes of the form `K * 2^N + 1`.
//! All constants computed by `integer/src/mul/ntt/compute_constants.py`.

use num_modular::{FixedProth32, Reducer};

// Proth reducer instances — each with a different (N, K) pair.
pub const P0: FixedProth32<26, 7> = FixedProth32::<26, 7>;
pub const P1: FixedProth32<27, 15> = FixedProth32::<27, 15>;
pub const P2: FixedProth32<27, 17> = FixedProth32::<27, 17>;

pub const K: usize = 3;
pub const MAX_LOG_N: u32 = 26;
pub const B_PACK_MIN: u32 = 8;
pub const B_PACK_CANDIDATES: &[u32] = &[32, 16, 8];

pub type Lane = u32;
pub type DoubleLane = u64;

/// Primitive `MAX_LOG_N`-th roots of unity for each prime.
pub const OMEGA_MAX: [Lane; K] = [
    0x0000088b, // P0
    0x3a26eef8, // P1
    0x1aa0ab5e, // P2
];

pub const CRT_INV_IJ: [[Lane; K]; K] = [
    [0, 0x4e42c85b, 0x5fb425ef],
    [0, 0, 0x44000009],
    [0, 0, 0],
];

/// Prime moduli indexed by PI.
pub const MODULI: [Lane; K] = [
    FixedProth32::<26, 7>::MODULUS,
    FixedProth32::<27, 15>::MODULUS,
    FixedProth32::<27, 17>::MODULUS,
];

#[inline]
pub fn to_monty<const PI: usize>(val: Lane) -> Lane {
    match PI {
        0 => P0.transform(val),
        1 => P1.transform(val),
        2 => P2.transform(val),
        _ => unreachable!(),
    }
}

#[inline]
pub fn from_monty<const PI: usize>(val: Lane) -> Lane {
    match PI {
        0 => P0.residue(val),
        1 => P1.residue(val),
        2 => P2.residue(val),
        _ => unreachable!(),
    }
}

#[inline]
pub fn mul_mod<const PI: usize>(a: Lane, b_val: Lane) -> Lane {
    let prod = (a as DoubleLane) * (b_val as DoubleLane);
    match PI {
        0 => P0.reduce(prod),
        1 => P1.reduce(prod),
        2 => P2.reduce(prod),
        _ => unreachable!(),
    }
}

#[inline]
pub fn add_mod<const PI: usize>(a: Lane, b_val: Lane) -> Lane {
    match PI {
        0 => P0.add(&a, &b_val),
        1 => P1.add(&a, &b_val),
        2 => P2.add(&a, &b_val),
        _ => unreachable!(),
    }
}

#[inline]
pub fn sub_mod<const PI: usize>(a: Lane, b_val: Lane) -> Lane {
    match PI {
        0 => P0.sub(&a, &b_val),
        1 => P1.sub(&a, &b_val),
        2 => P2.sub(&a, &b_val),
        _ => unreachable!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
            let sqr = |w: Lane| -> Lane {
                match pi {
                    0 => P0.reduce((w as u64) * (w as u64)),
                    1 => P1.reduce((w as u64) * (w as u64)),
                    2 => P2.reduce((w as u64) * (w as u64)),
                    _ => unreachable!(),
                }
            };

            let mut w = match pi {
                0 => to_monty::<0>(omega_max),
                1 => to_monty::<1>(omega_max),
                2 => to_monty::<2>(omega_max),
                _ => unreachable!(),
            };
            for _ in 0..MAX_LOG_N - 1 {
                w = sqr(w);
            }
            let w_std = match pi {
                0 => from_monty::<0>(w),
                1 => from_monty::<1>(w),
                2 => from_monty::<2>(w),
                _ => unreachable!(),
            };
            assert_eq!(w_std, p - 1, "omega^(2^(MAX_LOG_N-1)) != -1 mod p for prime {pi}");
            w = sqr(w);
            let one = match pi {
                0 => from_monty::<0>(w),
                1 => from_monty::<1>(w),
                2 => from_monty::<2>(w),
                _ => unreachable!(),
            };
            assert_eq!(one, 1, "omega^(2^MAX_LOG_N) != 1 mod p for prime {pi}");
        }
    }
}
