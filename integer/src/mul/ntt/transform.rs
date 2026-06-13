//! Iterative in-place radix-2 NTT over primes of the form `2^64 - 2^b + 1`.
//!
//! Uses decimation-in-time (DIT) Cooley–Tukey butterflies.  All modular
//! arithmetic delegates to `num_modular::FixedTrinomialSolinas64`.

use num_modular::{FixedTrinomialSolinas64, ModularPow, ModularUnaryOps, Reducer};

// ---- dispatch helpers (b is a runtime value, const-generic under the hood) ----

#[inline]
fn reduce_double(v: u128, b: u32) -> u64 {
    match b {
        32 => FixedTrinomialSolinas64::<64, 32, 1>::reduce_double(v),
        34 => FixedTrinomialSolinas64::<64, 34, 1>::reduce_double(v),
        40 => FixedTrinomialSolinas64::<64, 40, 1>::reduce_double(v),
        _ => unreachable!(),
    }
}

#[inline]
fn mul_mod(a: u64, b_val: u64, b: u32) -> u64 {
    reduce_double((a as u128) * (b_val as u128), b)
}

#[inline]
fn add_mod(a: u64, b_val: u64, p: u64, b: u32) -> u64 {
    match b {
        32 => FixedTrinomialSolinas64::<64, 32, 1>::new(&p).add(&a, &b_val),
        34 => FixedTrinomialSolinas64::<64, 34, 1>::new(&p).add(&a, &b_val),
        40 => FixedTrinomialSolinas64::<64, 40, 1>::new(&p).add(&a, &b_val),
        _ => unreachable!(),
    }
}

#[inline]
fn sub_mod(a: u64, b_val: u64, p: u64, b: u32) -> u64 {
    match b {
        32 => FixedTrinomialSolinas64::<64, 32, 1>::new(&p).sub(&a, &b_val),
        34 => FixedTrinomialSolinas64::<64, 34, 1>::new(&p).sub(&a, &b_val),
        40 => FixedTrinomialSolinas64::<64, 40, 1>::new(&p).sub(&a, &b_val),
        _ => unreachable!(),
    }
}

// ---- public API ----

/// Precompute the twiddle-factor table for transform length `n`.
///
/// Returns `n/2` entries: `omega_n^k` for `k = 0..n/2`, where
/// `omega_n = omega_2_32^{2^32 / n}` (a primitive `n`-th root of unity).
pub fn precompute_twiddles(
    n: usize,
    p: u64,
    b: u32,
    omega_2_32: u64,
    inverse: bool,
) -> alloc::vec::Vec<u64> {
    let shift = 32 - n.trailing_zeros();
    let omega_n = omega_2_32.powm(&(1u64 << shift), &p);

    let base = if inverse {
        omega_n.invm(&p).expect("omega_n not invertible")
    } else {
        omega_n
    };

    let mut twiddles = alloc::vec![0u64; n / 2];
    twiddles[0] = 1;
    for k in 1..(n / 2) {
        twiddles[k] = mul_mod(twiddles[k - 1], base, b);
    }
    twiddles
}

/// Bit-reverse `a` in place.  Length must be a power of two.
pub fn bit_reverse(a: &mut [u64]) {
    let n = a.len();
    assert!(n.is_power_of_two());
    let log_n = n.trailing_zeros();
    for i in 0..n {
        let j = i.reverse_bits() >> (usize::BITS - log_n);
        if i < j {
            a.swap(i, j);
        }
    }
}

/// Forward NTT in place (decimation-in-time, radix-2).
pub fn forward(a: &mut [u64], twiddles: &[u64], p: u64, b: u32) {
    ntt_core(a, twiddles, p, b);
}

/// Inverse NTT in place.
///
/// Computed as `bit_reverse → forward(ω^{-1}) → scale`, producing output
/// in **natural order**.
///
/// `twiddles` must have been precomputed with `inverse = true`
/// (i.e. using `omega_n^{-1}`).
pub fn inverse(a: &mut [u64], twiddles: &[u64], p: u64, b: u32) {
    let n = a.len();
    bit_reverse(a);
    ntt_core(a, twiddles, p, b);
    let n_inv = (n as u64).invm(&p).expect("n not invertible mod p");
    for x in a.iter_mut() {
        *x = mul_mod(*x, n_inv, b);
    }
}

/// In-place radix-2 DIT NTT (Cooley–Tukey).
fn ntt_core(a: &mut [u64], twiddles: &[u64], p: u64, b: u32) {
    let n = a.len();
    debug_assert!(n.is_power_of_two() && twiddles.len() == n / 2);

    let mut sub_len = 2usize;
    while sub_len <= n {
        let half = sub_len / 2;
        let step = n / sub_len;

        for i in (0..n).step_by(sub_len) {
            for j in 0..half {
                let u = a[i + j];
                let v = mul_mod(a[i + j + half], twiddles[j * step], b);
                a[i + j] = add_mod(u, v, p, b);
                a[i + j + half] = sub_mod(u, v, p, b);
            }
        }

        sub_len *= 2;
    }
}

/// Pointwise multiply of two transformed vectors in place.
pub fn pointwise_mul(a_hat: &mut [u64], b_hat: &[u64], b: u32) {
    assert_eq!(a_hat.len(), b_hat.len());
    for (a, &b_val) in a_hat.iter_mut().zip(b_hat.iter()) {
        *a = mul_mod(*a, b_val, b);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mul::ntt::primes::PRIMES;

    fn assert_all_eq(a: &[u64], b_val: &[u64]) {
        assert_eq!(a.len(), b_val.len());
        for (i, (x, y)) in a.iter().zip(b_val.iter()).enumerate() {
            assert_eq!(x, y, "mismatch at index {i}: {x} != {y}");
        }
    }

    #[test]
    fn test_forward_inverse_roundtrip() {
        for prime in &PRIMES {
            let p = prime.p;
            let b = prime.b;
            for &n in &[2, 4, 8, 16, 32, 64, 128, 256, 512] {
                let inv_twiddles = precompute_twiddles(n, p, b, prime.omega_2_32, true);
                let fwd_twiddles = precompute_twiddles(n, p, b, prime.omega_2_32, false);

                let mut a: Vec<u64> = (0..n)
                    .map(|i| ((i as u64 + 1).wrapping_mul(123456789)) % p)
                    .collect();
                let orig = a.clone();

                bit_reverse(&mut a);
                forward(&mut a, &fwd_twiddles, p, b);
                inverse(&mut a, &inv_twiddles, p, b);

                assert_all_eq(&a, &orig);
            }
        }
    }

    #[test]
    fn test_convolution_via_ntt() {
        for prime in &PRIMES {
            let p = prime.p;
            let b = prime.b;
            for len_a in [1, 2, 3, 5] {
                for len_b in [1, 2, 3, 5] {
                    let conv_len: usize = len_a + len_b - 1;
                    let n = conv_len.next_power_of_two().max(2);

                    let a: Vec<u64> = (0..len_a).map(|i| ((i + 1) as u64 * 12345) % p).collect();
                    let b_vec: Vec<u64> =
                        (0..len_b).map(|i| ((i + 1) as u64 * 67890) % p).collect();

                    let mut expected = vec![0u64; conv_len];
                    for (i, &ai) in a.iter().enumerate() {
                        for (j, &bj) in b_vec.iter().enumerate() {
                            expected[i + j] = add_mod(expected[i + j], mul_mod(ai, bj, b), p, b);
                        }
                    }

                    let inv_twiddles = precompute_twiddles(n, p, b, prime.omega_2_32, true);
                    let fwd_twiddles = precompute_twiddles(n, p, b, prime.omega_2_32, false);

                    let mut a_pad = vec![0u64; n];
                    let mut b_pad = vec![0u64; n];
                    a_pad[..len_a].copy_from_slice(&a);
                    b_pad[..len_b].copy_from_slice(&b_vec);

                    bit_reverse(&mut a_pad);
                    bit_reverse(&mut b_pad);
                    forward(&mut a_pad, &fwd_twiddles, p, b);
                    forward(&mut b_pad, &fwd_twiddles, p, b);
                    pointwise_mul(&mut a_pad, &b_pad, b);
                    inverse(&mut a_pad, &inv_twiddles, p, b);

                    match assert_all_eq_result(&a_pad[..conv_len], &expected) {
                        Ok(()) => {}
                        Err((i, l, r)) => panic!(
                            "convolution mismatch: b={b}, len_a={len_a}, len_b={len_b}, n={n}, \
                             index {i}: {l} != {r}"
                        ),
                    }
                }
            }
        }
    }

    fn assert_all_eq_result(a: &[u64], b_val: &[u64]) -> Result<(), (usize, u64, u64)> {
        assert_eq!(a.len(), b_val.len());
        for (i, (x, y)) in a.iter().zip(b_val.iter()).enumerate() {
            if x != y {
                return Err((i, *x, *y));
            }
        }
        Ok(())
    }

    #[test]
    fn test_bit_reverse() {
        let mut a: Vec<u64> = (0..8).collect();
        bit_reverse(&mut a);
        assert_eq!(a, vec![0, 4, 2, 6, 1, 5, 3, 7]);
    }

    #[allow(clippy::needless_range_loop)]
    fn ntt_naive(x: &[u64], omega_n: u64, p: u64, b: u32) -> Vec<u64> {
        let n = x.len();
        let mut result = vec![0u64; n];
        for k in 0..n {
            let mut acc = 0u64;
            for j in 0..n {
                let twiddle = if k == 0 || j == 0 {
                    1
                } else {
                    omega_n.powm(&((k * j) as u64), &p)
                };
                acc = add_mod(acc, mul_mod(x[j], twiddle, b), p, b);
            }
            result[k] = acc;
        }
        result
    }

    #[test]
    fn test_forward_correctness() {
        for prime in &PRIMES {
            let p = prime.p;
            let b = prime.b;
            for &n in &[2usize, 4, 8] {
                let x: Vec<u64> = (0..n).map(|i| ((i + 1) as u64 * 11111) % p).collect();

                let shift = 32 - n.trailing_zeros();
                let omega_n = prime.omega_2_32.powm(&(1u64 << shift), &p);

                let mut a = x.clone();
                bit_reverse(&mut a);

                let fwd_twiddles = precompute_twiddles(n, p, b, prime.omega_2_32, false);
                forward(&mut a, &fwd_twiddles, p, b);

                let expected = ntt_naive(&x, omega_n, p, b);
                assert_eq!(
                    a, expected,
                    "forward NTT mismatch: b={b}, n={n}, expected={expected:?}, got={a:?}"
                );
            }
        }
    }

    #[test]
    fn test_convolution_debug() {
        let prime = &PRIMES[0];
        let p = prime.p;
        let b = prime.b;

        let a = vec![12345u64 % p];
        let b_vec = vec![67890u64 % p, 135780u64 % p, 203670u64 % p];
        let conv_len = a.len() + b_vec.len() - 1;
        let n = 4;

        let mut expected = vec![0u64; conv_len];
        for (i, &ai) in a.iter().enumerate() {
            for (j, &bj) in b_vec.iter().enumerate() {
                expected[i + j] = add_mod(expected[i + j], mul_mod(ai, bj, b), p, b);
            }
        }

        let fwd_twiddles = precompute_twiddles(n, p, b, prime.omega_2_32, false);
        let inv_twiddles = precompute_twiddles(n, p, b, prime.omega_2_32, true);

        let mut a_pad = vec![0u64; n];
        let mut b_pad = vec![0u64; n];
        a_pad[..a.len()].copy_from_slice(&a);
        b_pad[..b_vec.len()].copy_from_slice(&b_vec);

        bit_reverse(&mut a_pad);
        bit_reverse(&mut b_pad);
        forward(&mut a_pad, &fwd_twiddles, p, b);
        forward(&mut b_pad, &fwd_twiddles, p, b);
        pointwise_mul(&mut a_pad, &b_pad, b);
        inverse(&mut a_pad, &inv_twiddles, p, b);

        assert_eq!(&a_pad[..conv_len], &expected[..]);
    }

    #[test]
    fn test_length_two_edge_case() {
        for prime in &PRIMES {
            let p = prime.p;
            let b = prime.b;
            let n = 2;
            let fwd_twiddles = precompute_twiddles(n, p, b, prime.omega_2_32, false);
            let inv_twiddles = precompute_twiddles(n, p, b, prime.omega_2_32, true);

            let a_orig = vec![1u64 % p, 2u64 % p];
            let mut a = a_orig.clone();
            bit_reverse(&mut a);
            forward(&mut a, &fwd_twiddles, p, b);
            inverse(&mut a, &inv_twiddles, p, b);
            assert_all_eq(&a, &a_orig);
        }
    }
}
