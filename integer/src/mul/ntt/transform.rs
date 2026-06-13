//! Iterative in-place radix-2 NTT over Proth primes `K * 2^N + 1`.
//!
//! All functions are const-generic over `PI` (the prime index `0..K`).
//! Modular arithmetic delegates to `crate::arch::ntt`.

use crate::arch::ntt::{add_mod, mul_mod, sub_mod, to_monty, Lane, MAX_LOG_N};
use num_modular::{ModularPow, ModularUnaryOps};

// ---- public API ----

/// Fill `out[0..n/2]` with twiddle factors `omega_n^k` in Montgomery form.
///
/// Panics if `out.len() < n / 2`.
pub fn precompute_twiddles<const PI: usize>(
    out: &mut [Lane],
    n: usize,
    p: Lane,
    omega_max: Lane,
    inverse: bool,
) {
    assert!(out.len() >= n / 2);
    let shift = MAX_LOG_N - n.trailing_zeros();
    let omega_n = omega_max.powm(&((1u64 as Lane) << shift), &p);

    let base = if inverse {
        omega_n.invm(&p).expect("omega_n not invertible")
    } else {
        omega_n
    };

    // Convert base and 1 to Montgomery form
    let base_mont = to_monty::<PI>(base);
    out[0] = to_monty::<PI>(1);
    for k in 1..(n / 2) {
        out[k] = mul_mod::<PI>(out[k - 1], base_mont);
    }
}

/// Bit-reverse `a` in place.  Length must be a power of two.
pub fn bit_reverse(a: &mut [Lane]) {
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
pub fn forward<const PI: usize>(a: &mut [Lane], twiddles: &[Lane]) {
    ntt_core::<PI>(a, twiddles);
}

/// Inverse NTT in place.
///
/// Computed as `bit_reverse → forward(ω⁻¹) → scale`, producing output
/// in **natural order**.
///
/// `twiddles` must have been precomputed with `inverse = true`.
pub fn inverse<const PI: usize>(a: &mut [Lane], twiddles: &[Lane], p: Lane) {
    let n = a.len();
    bit_reverse(a);
    ntt_core::<PI>(a, twiddles);
    let n_val = n as Lane;
    let n_inv = n_val.invm(&p).expect("n not invertible mod p");
    // Convert n⁻¹ to Montgomery form so the result stays in Montgomery form.
    let n_inv_mont = to_monty::<PI>(n_inv);
    for x in a.iter_mut() {
        *x = mul_mod::<PI>(*x, n_inv_mont);
    }
}

/// In-place radix-2 DIT NTT (Cooley–Tukey).
fn ntt_core<const PI: usize>(a: &mut [Lane], twiddles: &[Lane]) {
    let n = a.len();
    debug_assert!(n.is_power_of_two() && twiddles.len() == n / 2);

    let mut sub_len = 2usize;
    while sub_len <= n {
        let half = sub_len / 2;
        let step = n / sub_len;

        for i in (0..n).step_by(sub_len) {
            for j in 0..half {
                let u = a[i + j];
                let v = mul_mod::<PI>(a[i + j + half], twiddles[j * step]);
                a[i + j] = add_mod::<PI>(u, v);
                a[i + j + half] = sub_mod::<PI>(u, v);
            }
        }

        sub_len *= 2;
    }
}

/// Pointwise multiply of two transformed vectors in place.
pub fn pointwise_mul<const PI: usize>(a_hat: &mut [Lane], b_hat: &[Lane]) {
    assert_eq!(a_hat.len(), b_hat.len());
    for (a, &b_val) in a_hat.iter_mut().zip(b_hat.iter()) {
        *a = mul_mod::<PI>(*a, b_val);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arch::ntt::{from_monty, to_monty, K, MODULI, OMEGA_MAX};
    #[cfg(not(feature = "std"))]
    use alloc::vec;
    #[cfg(not(feature = "std"))]
    use alloc::vec::Vec;

    fn assert_all_eq(a: &[Lane], b_val: &[Lane], context: &str) {
        assert_eq!(a.len(), b_val.len(), "{context}: length mismatch");
        for (i, (x, y)) in a.iter().zip(b_val.iter()).enumerate() {
            assert_eq!(x, y, "{context}: mismatch at index {i}: {x} != {y}");
        }
    }

    macro_rules! for_each_prime {
        ($pi:ident, $p:ident, $omega:ident, $body:block) => {
            for idx in 0..K {
                let $p = MODULI[idx];
                let $omega = OMEGA_MAX[idx];
                match idx {
                    0 => {
                        let $pi: usize = 0;
                        let _ = $pi;
                        fn go<const PI: usize>($p: Lane, $omega: Lane) $body
                        go::<0>($p, $omega);
                    }
                    1 => {
                        let $pi: usize = 1;
                        let _ = $pi;
                        fn go<const PI: usize>($p: Lane, $omega: Lane) $body
                        go::<1>($p, $omega);
                    }
                    2 => {
                        let $pi: usize = 2;
                        let _ = $pi;
                        fn go<const PI: usize>($p: Lane, $omega: Lane) $body
                        go::<2>($p, $omega);
                    }
                    _ => unreachable!(),
                }
            }
        };
    }

    #[test]
    fn test_forward_inverse_roundtrip() {
        for_each_prime!(pi, p, omega, {
            for &n in &[2, 4, 8, 16, 32, 64, 128, 256, 512] {
                let mut fwd_twiddles = alloc::vec![0u64 as Lane; n / 2];
                let mut inv_twiddles = alloc::vec![0u64 as Lane; n / 2];
                precompute_twiddles::<PI>(&mut fwd_twiddles, n, p, omega, false);
                precompute_twiddles::<PI>(&mut inv_twiddles, n, p, omega, true);

                let mut a: Vec<Lane> = (0..n)
                    .map(|i| ((i as Lane + 1).wrapping_mul(123456789)) % p)
                    .collect();
                // Convert to Montgomery form for the NTT pipeline
                for val in a.iter_mut() {
                    *val = to_monty::<PI>(*val);
                }
                let orig = a.clone();

                bit_reverse(&mut a);
                forward::<PI>(&mut a, &fwd_twiddles);
                inverse::<PI>(&mut a, &inv_twiddles, p);

                assert_all_eq(&a, &orig, "roundtrip failed for n={n}");
            }
        });
    }

    #[test]
    fn test_convolution_via_ntt() {
        for_each_prime!(pi, p, omega, {
            for len_a in [1, 2, 3, 5] {
                for len_b in [1, 2, 3, 5] {
                    let conv_len: usize = len_a + len_b - 1;
                    let n = conv_len.next_power_of_two().max(2);

                    let a: Vec<Lane> =
                        (0..len_a).map(|i| ((i + 1) as Lane * 12345) % p).collect();
                    let b_vec: Vec<Lane> =
                        (0..len_b).map(|i| ((i + 1) as Lane * 67890) % p).collect();

                    // Compute expected convolution in standard form
                    let mut expected = vec![0u64 as Lane; conv_len];
                    for (i, &ai) in a.iter().enumerate() {
                        for (j, &bj) in b_vec.iter().enumerate() {
                            let prod = (ai as u128 * bj as u128 % p as u128) as Lane;
                            expected[i + j] = add_mod::<PI>(expected[i + j], prod);
                        }
                    }

                        let mut fwd_twiddles = alloc::vec![0u64 as Lane; n / 2];
                    let mut inv_twiddles = alloc::vec![0u64 as Lane; n / 2];
                    precompute_twiddles::<PI>(&mut fwd_twiddles, n, p, omega, false);
                    precompute_twiddles::<PI>(&mut inv_twiddles, n, p, omega, true);

                    // Convert inputs to Montgomery form
                    let mut a_pad = vec![0u64 as Lane; n];
                    let mut b_pad = vec![0u64 as Lane; n];
                    for i in 0..len_a {
                        a_pad[i] = to_monty::<PI>(a[i]);
                    }
                    for i in 0..len_b {
                        b_pad[i] = to_monty::<PI>(b_vec[i]);
                    }

                    bit_reverse(&mut a_pad);
                    bit_reverse(&mut b_pad);
                    forward::<PI>(&mut a_pad, &fwd_twiddles);
                    forward::<PI>(&mut b_pad, &fwd_twiddles);
                    pointwise_mul::<PI>(&mut a_pad, &b_pad);
                    inverse::<PI>(&mut a_pad, &inv_twiddles, p);
                    // Convert results back to standard form
                    for val in a_pad[..conv_len].iter_mut() {
                        *val = from_monty::<PI>(*val);
                    }

                    assert_all_eq(&a_pad[..conv_len], &expected, "convolution mismatch");
                }
            }
        });
    }

    #[test]
    fn test_bit_reverse() {
        let mut a: Vec<Lane> = (0..8).map(|i| i as Lane).collect();
        bit_reverse(&mut a);
        assert_eq!(a, vec![0, 4, 2, 6, 1, 5, 3, 7]);
    }

    /// Naive O(n²) NTT using standard-form modular arithmetic.
    #[allow(clippy::needless_range_loop)]
    fn ntt_naive_std<const PI: usize>(x: &[Lane], omega_n: Lane, p: Lane) -> Vec<Lane> {
        let n = x.len();
        let mut result = vec![0u64 as Lane; n];
        for k in 0..n {
            let mut acc: u128 = 0;
            for j in 0..n {
                let twiddle = if k == 0 || j == 0 {
                    1
                } else {
                    omega_n.powm(&((k * j) as Lane), &p) as u128
                };
                acc = (acc + x[j] as u128 * twiddle) % p as u128;
            }
            result[k] = acc as Lane;
        }
        result
    }

    #[test]
    fn test_forward_correctness() {
        for_each_prime!(pi, p, omega, {
            for &n in &[2usize, 4, 8] {
                let x: Vec<Lane> = (0..n).map(|i| ((i + 1) as Lane * 11111) % p).collect();


                let shift = MAX_LOG_N - n.trailing_zeros();
                let omega_n = omega.powm(&((1u64 as Lane) << shift), &p);

                // Convert to Montgomery form
                let mut a: Vec<Lane> = x.iter().map(|&v| to_monty::<PI>(v)).collect();
                bit_reverse(&mut a);

                let mut fwd_twiddles = alloc::vec![0u64 as Lane; n / 2];
                precompute_twiddles::<PI>(&mut fwd_twiddles, n, p, omega, false);
                forward::<PI>(&mut a, &fwd_twiddles);

                // Convert forward output back to standard form for comparison
                for val in a.iter_mut() {
                    *val = from_monty::<PI>(*val);
                }

                // expected: compute naive NTT in standard form
                let expected = ntt_naive_std::<PI>(&x, omega_n, p);
                assert_eq!(a, expected, "forward NTT mismatch");
            }
        });
    }

    #[test]
    fn test_convolution_debug() {
        let p = MODULI[0];
        let omega = OMEGA_MAX[0];

        let a = [12345u64 as Lane % p];
        let b_vec = [67890u64 as Lane % p,
            135780u64 as Lane % p,
            203670u64 as Lane % p];
        let conv_len = a.len() + b_vec.len() - 1;
        let n = 4;

        // Expected values in standard form
        let mut expected = vec![0u64 as Lane; conv_len];
        for (i, &ai) in a.iter().enumerate() {
            for (j, &bj) in b_vec.iter().enumerate() {
                let prod = (ai as u128 * bj as u128 % p as u128) as Lane;
                expected[i + j] = add_mod::<0>(expected[i + j], prod);
            }
        }

        let mut fwd_twiddles = alloc::vec![0u64 as Lane; n / 2];
        let mut inv_twiddles = alloc::vec![0u64 as Lane; n / 2];
        precompute_twiddles::<0>(&mut fwd_twiddles, n, p, omega, false);
        precompute_twiddles::<0>(&mut inv_twiddles, n, p, omega, true);

        // Convert to Montgomery form
        let mut a_pad = vec![0u64 as Lane; n];
        let mut b_pad = vec![0u64 as Lane; n];
        for i in 0..a.len() {
            a_pad[i] = to_monty::<0>(a[i]);
        }
        for i in 0..b_vec.len() {
            b_pad[i] = to_monty::<0>(b_vec[i]);
        }

        bit_reverse(&mut a_pad);
        bit_reverse(&mut b_pad);
        forward::<0>(&mut a_pad, &fwd_twiddles);
        forward::<0>(&mut b_pad, &fwd_twiddles);
        pointwise_mul::<0>(&mut a_pad, &b_pad);
        inverse::<0>(&mut a_pad, &inv_twiddles, p);
        // Convert back to standard form
        for val in a_pad[..conv_len].iter_mut() {
            *val = from_monty::<0>(*val);
        }

        assert_eq!(&a_pad[..conv_len], &expected[..]);
    }

    #[test]
    fn test_length_two_edge_case() {
        for_each_prime!(pi, p, omega, {
            let n = 2;
            let mut fwd_twiddles = alloc::vec![0u64 as Lane; n / 2];
            let mut inv_twiddles = alloc::vec![0u64 as Lane; n / 2];
            precompute_twiddles::<PI>(&mut fwd_twiddles, n, p, omega, false);
            precompute_twiddles::<PI>(&mut inv_twiddles, n, p, omega, true);

            let a_std = [1u64 as Lane % p, 2u64 as Lane % p];
            // Convert to Montgomery form
            let a_orig: Vec<Lane> = a_std.iter().map(|&v| to_monty::<PI>(v)).collect();
            let mut a = a_orig.clone();
            bit_reverse(&mut a);
            forward::<PI>(&mut a, &fwd_twiddles);
            inverse::<PI>(&mut a, &inv_twiddles, p);
            assert_all_eq(&a, &a_orig, "length two roundtrip");
        });
    }
}
