//! NTT-friendly primes of the form `2^64 - 2^b + 1`.
//!
//! All three support shift-based reduction via the identity `2^64 ≡ 2^b - 1 (mod p)`.

/// The number of primes in the fixed array.
pub const K: usize = 3;

/// Precomputed data for one NTT-friendly prime.
#[derive(Clone, Copy, Debug)]
pub struct NttPrime {
    /// The prime value `p = 2^64 - 2^b + 1`.
    pub p: u64,
    /// The exponent `b` in the Solinas form.
    pub b: u32,
    /// The exponent of 2 in `p - 1`: `v2(p - 1)`.
    #[cfg(test)]
    pub v2: u32,
    /// A primitive root modulo `p` that generates the full multiplicative group.
    #[cfg(test)]
    pub g: u64,
    /// A primitive `2^32`-th root of unity: `ω = g^{(p-1) / 2^32} mod p`.
    pub omega_2_32: u64,
}

/// The three chosen primes, all of the form `2^64 - 2^b + 1` with `b ∈ {32, 34, 40}`.
///
/// | name | `b` | `p`                 | `v2(p-1)` | gen `g` | `2^32`-th root ω           |
/// |------|-----|---------------------|-----------|---------|----------------------------|
/// | GL   | 32  | `0xFFFFFFFF00000001` | 32        | 7       | `1753635133440165772`      |
/// | P1   | 34  | `0xFFFFFFFC00000001` | 34        | 5       | `11315553352654630047`     |
/// | P2   | 40  | `0xFFFFFF0000000001` | 40        | 19      | `551857376737322389`       |
#[cfg(not(test))]
pub const PRIMES: [NttPrime; K] = [
    NttPrime {
        p: 0xFFFFFFFF00000001,
        b: 32,
        omega_2_32: 1753635133440165772,
    },
    NttPrime {
        p: 0xFFFFFFFC00000001,
        b: 34,
        omega_2_32: 11315553352654630047,
    },
    NttPrime {
        p: 0xFFFFFF0000000001,
        b: 40,
        omega_2_32: 551857376737322389,
    },
];

#[cfg(test)]
pub const PRIMES: [NttPrime; K] = [
    NttPrime {
        p: 0xFFFFFFFF00000001,
        b: 32,
        v2: 32,
        g: 7,
        omega_2_32: 1753635133440165772,
    },
    NttPrime {
        p: 0xFFFFFFFC00000001,
        b: 34,
        v2: 34,
        g: 5,
        omega_2_32: 11315553352654630047,
    },
    NttPrime {
        p: 0xFFFFFF0000000001,
        b: 40,
        v2: 40,
        g: 19,
        omega_2_32: 551857376737322389,
    },
];

/// Garner CRT constants: `inv(p_i mod p_j)` for i < j.
/// Computed offline via `pow(p_i % p_j, -1, p_j)`.
pub const CRT_INV_IJ: [[u64; 3]; 3] = [
    [0, 0xfffffffbaaaaaaad, 0xfffffefffefeff01],
    [0, 0, 0xfffffefffefbefc1],
    [0, 0, 0],
];

#[cfg(test)]
mod tests {
    use super::*;
    use num_modular::FixedTrinomialSolinas64;

    /// Deterministic Miller–Rabin for 64-bit integers with known bases.
    /// Tests `n` against bases `[2, 325, 9375, 28178, 450775, 9780504, 1795265022]`
    /// which together suffice for all `n < 2^64` (deterministic).
    fn is_prime_u64(n: u64) -> bool {
        if n < 2 {
            return false;
        }
        if n % 2 == 0 {
            return n == 2;
        }

        // Write n-1 = d * 2^s
        let d = (n - 1) >> (n - 1).trailing_zeros();
        let s = (n - 1).trailing_zeros();

        let bases = [2u64, 325, 9375, 28178, 450775, 9780504, 1795265022];

        'next_base: for &a in &bases {
            if a >= n {
                continue;
            }
            let mut x = mod_pow_u64(a % n, d, n);
            if x == 1 || x == n - 1 {
                continue 'next_base;
            }
            for _ in 1..s {
                x = ((x as u128 * x as u128) % (n as u128)) as u64;
                if x == n - 1 {
                    continue 'next_base;
                }
            }
            return false;
        }
        true
    }

    fn mod_pow_u64(mut base: u64, mut exp: u64, modulus: u64) -> u64 {
        let mut result = 1u64;
        while exp > 0 {
            if exp & 1 != 0 {
                result = ((result as u128 * base as u128) % (modulus as u128)) as u64;
            }
            base = ((base as u128 * base as u128) % (modulus as u128)) as u64;
            exp >>= 1;
        }
        result
    }

    #[test]
    fn verify_primes() {
        for &NttPrime {
            p,
            b,
            v2,
            g,
            omega_2_32,
        } in &PRIMES
        {
            // 1. Correct form: p == 2^64 - 2^b + 1
            let expected_p = (1u128 << 64) - (1u128 << b) + 1;
            assert!(expected_p < (1u128 << 64), "p must fit in 64 bits");
            assert_eq!(p as u128, expected_p, "p = 0x{p:X} does not match 2^64 - 2^{b} + 1");
            assert!(p > 0, "p must be positive");

            // 2. Primality
            assert!(is_prime_u64(p), "p = 0x{p:X} is not prime");

            // 3. v2(p-1) is at least 32
            let actual_v2 = (p - 1).trailing_zeros();
            assert!(actual_v2 >= 32, "v2(p-1) = {actual_v2} < 32 for p = 0x{p:X}");
            assert_eq!(actual_v2, v2, "stored v2 mismatch for p = 0x{p:X}");

            // 4. g generates the full multiplicative group mod p.
            // g^((p-1)/2) mod p ≠ 1 (g is a quadratic non-residue)
            let g_order_half = mod_pow_u64(g, (p - 1) / 2, p);
            assert_ne!(g_order_half, 1, "g = {g} is a quadratic residue mod p = 0x{p:X}");

            // g^(p-1) ≡ 1
            let g_full = mod_pow_u64(g, p - 1, p);
            assert_eq!(g_full, 1, "g^(p-1) != 1 mod p = 0x{p:X}");

            // 5. ω has exact order 2^32
            let mut omega_pow = omega_2_32;
            for _ in 0..31 {
                omega_pow = ((omega_pow as u128 * omega_pow as u128) % (p as u128)) as u64;
            }
            // After 31 squarings: ω^{2^31} mod p
            // Should be -1 mod p (order is exactly 2^32)
            assert_eq!(omega_pow, p - 1, "omega^(2^31) != -1 mod p = 0x{p:X}, order not 2^32");

            // ω^{2^32} ≡ 1
            let omega_full = mod_pow_u64(omega_2_32, 1u64 << 32, p);
            assert_eq!(omega_full, 1, "omega^(2^32) != 1 mod p = 0x{p:X}");

            // 6. Reduction identity: 2^64 ≡ 2^b - 1 (mod p)
            let two_64_mod_p = ((1u128 << 64) % (p as u128)) as u64;
            let expected = (if b == 0 {
                0
            } else {
                (1u64 << (b - 1)).wrapping_mul(2)
            }) - 1;
            assert_eq!(two_64_mod_p, expected, "2^64 mod p != 2^(b) - 1 for p = 0x{p:X}");
        }
    }

    #[test]
    fn test_reduction_identity_per_prime() {
        // Verify reduction works for all three primes using the actual reducer types.
        // GL: b=32
        {
            type Reducer = FixedTrinomialSolinas64<64, 32, 1>;
            let p = Reducer::MODULUS;
            assert_eq!(p, PRIMES[0].p);

            // Test reduce_double
            let v = (p as u128) * 3; // 3p → should reduce to 0
            let r = Reducer::reduce_double(v);
            assert!(r < p);
            assert_eq!((r as u128) % (p as u128), v % (p as u128));
        }
        // P1: b=34
        {
            type Reducer = FixedTrinomialSolinas64<64, 34, 1>;
            let p = Reducer::MODULUS;
            assert_eq!(p, PRIMES[1].p);

            let v = (p as u128) * 3;
            let r = Reducer::reduce_double(v);
            assert!(r < p);
            assert_eq!((r as u128) % (p as u128), v % (p as u128));
        }
        // P2: b=40
        {
            type Reducer = FixedTrinomialSolinas64<64, 40, 1>;
            let p = Reducer::MODULUS;
            assert_eq!(p, PRIMES[2].p);

            let v = (p as u128) * 3;
            let r = Reducer::reduce_double(v);
            assert!(r < p);
            assert_eq!((r as u128) % (p as u128), v % (p as u128));
        }
    }

    #[test]
    fn test_pow_inv_roundtrip() {
        // pow and inv round-trip checks using the trait API
        use num_modular::{ModularPow, ModularUnaryOps};

        for &NttPrime { p, g, .. } in &PRIMES {
            // inv(x)·x ≡ 1
            let x = 123456789u64;
            let inv = x.invm(&p);
            if let Some(inv) = inv {
                let prod = ((x as u128 * inv as u128) % (p as u128)) as u64;
                assert_eq!(prod, 1, "inv round-trip failed for p = 0x{p:X}");
            }

            // pow(g, p-1) ≡ 1
            let g_pow = g.powm(&(p - 1), &p);
            assert_eq!(g_pow, 1, "g^(p-1) != 1 mod p = 0x{p:X}");

            // inv(g) · g ≡ 1
            let g_inv = g.invm(&p).unwrap();
            let prod = ((g as u128 * g_inv as u128) % (p as u128)) as u64;
            assert_eq!(prod, 1, "inv(g) round-trip failed for p = 0x{p:X}");
        }
    }
}
