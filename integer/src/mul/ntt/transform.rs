//! Iterative in-place radix-2 NTT over primes of the form `2^64 - 2^b + 1`.
//!
//! All functions are const-generic over `B` (the Solinas exponent, one of
//! `{32, 34, 40}`) so the compiler monomorphizes each prime's hot path.
//! Modular arithmetic delegates to `num_modular::FixedTrinomialSolinas64`.

use num_modular::{FixedTrinomialSolinas64, ModularPow, ModularUnaryOps, Reducer};

// ---- dispatch helpers (the match is optimized away since B is const) ----

#[inline]
fn mul_mod<const B: u32>(a: u64, b_val: u64) -> u64 {
    let prod = (a as u128) * (b_val as u128);
    match B {
        32 => FixedTrinomialSolinas64::<64, 32, 1>::reduce_double(prod),
        34 => FixedTrinomialSolinas64::<64, 34, 1>::reduce_double(prod),
        40 => FixedTrinomialSolinas64::<64, 40, 1>::reduce_double(prod),
        _ => unreachable!(),
    }
}

#[inline]
fn add_mod<const B: u32>(a: u64, b_val: u64, p: u64) -> u64 {
    match B {
        32 => FixedTrinomialSolinas64::<64, 32, 1>::new(&p).add(&a, &b_val),
        34 => FixedTrinomialSolinas64::<64, 34, 1>::new(&p).add(&a, &b_val),
        40 => FixedTrinomialSolinas64::<64, 40, 1>::new(&p).add(&a, &b_val),
        _ => unreachable!(),
    }
}

#[inline]
fn sub_mod<const B: u32>(a: u64, b_val: u64, p: u64) -> u64 {
    match B {
        32 => FixedTrinomialSolinas64::<64, 32, 1>::new(&p).sub(&a, &b_val),
        34 => FixedTrinomialSolinas64::<64, 34, 1>::new(&p).sub(&a, &b_val),
        40 => FixedTrinomialSolinas64::<64, 40, 1>::new(&p).sub(&a, &b_val),
        _ => unreachable!(),
    }
}

// ---- public API ----

/// Fill `out[0..n/2]` with twiddle factors `omega_n^k`.
///
/// Panics if `out.len() < n / 2`.
pub fn precompute_twiddles<const B: u32>(
    out: &mut [u64],
    n: usize,
    p: u64,
    omega_2_32: u64,
    inverse: bool,
) {
    assert!(out.len() >= n / 2);
    let shift = 32 - n.trailing_zeros();
    let omega_n = omega_2_32.powm(&(1u64 << shift), &p);

    let base = if inverse {
        omega_n.invm(&p).expect("omega_n not invertible")
    } else {
        omega_n
    };

    out[0] = 1;
    for k in 1..(n / 2) {
        out[k] = mul_mod::<B>(out[k - 1], base);
    }
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
pub fn forward<const B: u32>(a: &mut [u64], twiddles: &[u64], p: u64) {
    ntt_core::<B>(a, twiddles, p);
}

/// Inverse NTT in place.
///
/// Computed as `bit_reverse → forward(ω^{-1}) → scale`, producing output
/// in **natural order**.
///
/// `twiddles` must have been precomputed with `inverse = true`.
pub fn inverse<const B: u32>(a: &mut [u64], twiddles: &[u64], p: u64) {
    let n = a.len();
    bit_reverse(a);
    ntt_core::<B>(a, twiddles, p);
    let n_inv = (n as u64).invm(&p).expect("n not invertible mod p");
    for x in a.iter_mut() {
        *x = mul_mod::<B>(*x, n_inv);
    }
}

/// In-place radix-2 DIT NTT (Cooley–Tukey).
fn ntt_core<const B: u32>(a: &mut [u64], twiddles: &[u64], p: u64) {
    let n = a.len();
    debug_assert!(n.is_power_of_two() && twiddles.len() == n / 2);

    let mut sub_len = 2usize;
    while sub_len <= n {
        let half = sub_len / 2;
        let step = n / sub_len;

        for i in (0..n).step_by(sub_len) {
            for j in 0..half {
                let u = a[i + j];
                let v = mul_mod::<B>(a[i + j + half], twiddles[j * step]);
                a[i + j] = add_mod::<B>(u, v, p);
                a[i + j + half] = sub_mod::<B>(u, v, p);
            }
        }

        sub_len *= 2;
    }
}

/// Pointwise multiply of two transformed vectors in place.
pub fn pointwise_mul<const B: u32>(a_hat: &mut [u64], b_hat: &[u64]) {
    assert_eq!(a_hat.len(), b_hat.len());
    for (a, &b_val) in a_hat.iter_mut().zip(b_hat.iter()) {
        *a = mul_mod::<B>(*a, b_val);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mul::ntt::primes::PRIMES;
    #[cfg(not(feature = "std"))]
    use alloc::vec;
    #[cfg(not(feature = "std"))]
    use alloc::vec::Vec;

    fn assert_all_eq(a: &[u64], b_val: &[u64]) {
        assert_eq!(a.len(), b_val.len());
        for (i, (x, y)) in a.iter().zip(b_val.iter()).enumerate() {
            assert_eq!(x, y, "mismatch at index {i}: {x} != {y}");
        }
    }

    macro_rules! for_each_prime {
        ($b:ident, $p:ident, $omega:ident, $body:block) => {
            for prime in &PRIMES {
                let $b = prime.b;
                match $b {
                    32 => {
                        let $p = prime.p;
                        let $omega = prime.omega_2_32;
                        fn go<const B: u32>($p: u64, $omega: u64) $body
                        go::<32>($p, $omega);
                    }
                    34 => {
                        let $p = prime.p;
                        let $omega = prime.omega_2_32;
                        fn go<const B: u32>($p: u64, $omega: u64) $body
                        go::<34>($p, $omega);
                    }
                    40 => {
                        let $p = prime.p;
                        let $omega = prime.omega_2_32;
                        fn go<const B: u32>($p: u64, $omega: u64) $body
                        go::<40>($p, $omega);
                    }
                    _ => unreachable!(),
                }
            }
        };
    }

    #[test]
    fn test_forward_inverse_roundtrip() {
        for_each_prime!(b, p, omega, {
            for &n in &[2, 4, 8, 16, 32, 64, 128, 256, 512] {
                let mut fwd_twiddles = alloc::vec![0u64; n / 2];
                let mut inv_twiddles = alloc::vec![0u64; n / 2];
                precompute_twiddles::<B>(&mut fwd_twiddles, n, p, omega, false);
                precompute_twiddles::<B>(&mut inv_twiddles, n, p, omega, true);

                let mut a: Vec<u64> = (0..n)
                    .map(|i| ((i as u64 + 1).wrapping_mul(123456789)) % p)
                    .collect();
                let orig = a.clone();

                bit_reverse(&mut a);
                forward::<B>(&mut a, &fwd_twiddles, p);
                inverse::<B>(&mut a, &inv_twiddles, p);

                assert_all_eq(&a, &orig);
            }
        });
    }

    #[test]
    fn test_convolution_via_ntt() {
        for_each_prime!(b, p, omega, {
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
                            expected[i + j] =
                                add_mod::<B>(expected[i + j], mul_mod::<B>(ai, bj), p);
                        }
                    }

                    let mut fwd_twiddles = alloc::vec![0u64; n / 2];
                    let mut inv_twiddles = alloc::vec![0u64; n / 2];
                    precompute_twiddles::<B>(&mut fwd_twiddles, n, p, omega, false);
                    precompute_twiddles::<B>(&mut inv_twiddles, n, p, omega, true);

                    let mut a_pad = vec![0u64; n];
                    let mut b_pad = vec![0u64; n];
                    a_pad[..len_a].copy_from_slice(&a);
                    b_pad[..len_b].copy_from_slice(&b_vec);

                    bit_reverse(&mut a_pad);
                    bit_reverse(&mut b_pad);
                    forward::<B>(&mut a_pad, &fwd_twiddles, p);
                    forward::<B>(&mut b_pad, &fwd_twiddles, p);
                    pointwise_mul::<B>(&mut a_pad, &b_pad);
                    inverse::<B>(&mut a_pad, &inv_twiddles, p);

                    assert_all_eq(&a_pad[..conv_len], &expected);
                }
            }
        });
    }

    #[test]
    fn test_bit_reverse() {
        let mut a: Vec<u64> = (0..8).collect();
        bit_reverse(&mut a);
        assert_eq!(a, vec![0, 4, 2, 6, 1, 5, 3, 7]);
    }

    #[allow(clippy::needless_range_loop)]
    fn ntt_naive<const B: u32>(x: &[u64], omega_n: u64, p: u64) -> Vec<u64> {
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
                acc = add_mod::<B>(acc, mul_mod::<B>(x[j], twiddle), p);
            }
            result[k] = acc;
        }
        result
    }

    #[test]
    fn test_forward_correctness() {
        for_each_prime!(b, p, omega, {
            for &n in &[2usize, 4, 8] {
                let x: Vec<u64> = (0..n).map(|i| ((i + 1) as u64 * 11111) % p).collect();

                let shift = 32 - n.trailing_zeros();
                let omega_n = omega.powm(&(1u64 << shift), &p);

                let mut a = x.clone();
                bit_reverse(&mut a);

                let mut fwd_twiddles = alloc::vec![0u64; n / 2];
                precompute_twiddles::<B>(&mut fwd_twiddles, n, p, omega, false);
                forward::<B>(&mut a, &fwd_twiddles, p);

                let expected = ntt_naive::<B>(&x, omega_n, p);
                assert_eq!(a, expected, "forward NTT mismatch");
            }
        });
    }

    #[test]
    fn test_convolution_debug() {
        let prime = &PRIMES[0]; // GL: b=32
        let p = prime.p;

        let a = vec![12345u64 % p];
        let b_vec = vec![67890u64 % p, 135780u64 % p, 203670u64 % p];
        let conv_len = a.len() + b_vec.len() - 1;
        let n = 4;

        let mut expected = vec![0u64; conv_len];
        for (i, &ai) in a.iter().enumerate() {
            for (j, &bj) in b_vec.iter().enumerate() {
                expected[i + j] = add_mod::<32>(expected[i + j], mul_mod::<32>(ai, bj), p);
            }
        }

        let mut fwd_twiddles = alloc::vec![0u64; n / 2];
        let mut inv_twiddles = alloc::vec![0u64; n / 2];
        precompute_twiddles::<32>(&mut fwd_twiddles, n, p, prime.omega_2_32, false);
        precompute_twiddles::<32>(&mut inv_twiddles, n, p, prime.omega_2_32, true);

        let mut a_pad = vec![0u64; n];
        let mut b_pad = vec![0u64; n];
        a_pad[..a.len()].copy_from_slice(&a);
        b_pad[..b_vec.len()].copy_from_slice(&b_vec);

        bit_reverse(&mut a_pad);
        bit_reverse(&mut b_pad);
        forward::<32>(&mut a_pad, &fwd_twiddles, p);
        forward::<32>(&mut b_pad, &fwd_twiddles, p);
        pointwise_mul::<32>(&mut a_pad, &b_pad);
        inverse::<32>(&mut a_pad, &inv_twiddles, p);

        assert_eq!(&a_pad[..conv_len], &expected[..]);
    }

    #[test]
    fn test_length_two_edge_case() {
        for_each_prime!(b, p, omega, {
            let n = 2;
            let mut fwd_twiddles = alloc::vec![0u64; n / 2];
            let mut inv_twiddles = alloc::vec![0u64; n / 2];
            precompute_twiddles::<B>(&mut fwd_twiddles, n, p, omega, false);
            precompute_twiddles::<B>(&mut inv_twiddles, n, p, omega, true);

            let a_orig = vec![1u64 % p, 2u64 % p];
            let mut a = a_orig.clone();
            bit_reverse(&mut a);
            forward::<B>(&mut a, &fwd_twiddles, p);
            inverse::<B>(&mut a, &inv_twiddles, p);
            assert_all_eq(&a, &a_orig);
        });
    }
}
