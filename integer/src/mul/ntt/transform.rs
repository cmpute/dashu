//! Iterative in-place radix-2 NTT over Proth primes `K * 2^N + 1`.
//!
//! All functions are generic over `R: Reducer<Lane>` so each prime's
//! reducer is monomorphized at the call site.

use crate::arch::ntt::{Lane, MAX_LOG_N};
use num_modular::Reducer;

// ---- public API ----

/// Fill `out[0..n/2]` with twiddle factors `omega_n^k` in Montgomery form.
///
/// Panics if `out.len() < n / 2`.
pub fn precompute_twiddles<R: Reducer<Lane>>(
    out: &mut [Lane],
    n: usize,
    omega_max: Lane,
    inverse: bool,
    r: &R,
) {
    assert!(out.len() >= n / 2);
    let shift = MAX_LOG_N - n.trailing_zeros();
    let omega_max_mont = r.transform(omega_max);
    let omega_n_mont = r.pow(omega_max_mont, &((1u64 as Lane) << shift));

    let base_mont = if inverse {
        r.inv(omega_n_mont).expect("omega_n not invertible")
    } else {
        omega_n_mont
    };

    out[0] = r.transform(1);
    for k in 1..(n / 2) {
        out[k] = r.mul(&out[k - 1], &base_mont);
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
pub fn forward<R: Reducer<Lane>>(a: &mut [Lane], twiddles: &[Lane], r: &R) {
    ntt_core(a, twiddles, r);
}

/// Inverse NTT in place.
///
/// Computed as `bit_reverse → forward(ω⁻¹) → scale`, producing output
/// in **natural order**.
///
/// `twiddles` must have been precomputed with `inverse = true`.
pub fn inverse<R: Reducer<Lane>>(a: &mut [Lane], twiddles: &[Lane], r: &R) {
    let n = a.len();
    bit_reverse(a);
    ntt_core(a, twiddles, r);
    let n_mont = r.transform(n as Lane);
    let n_inv_mont = r.inv(n_mont).expect("n not invertible mod p");
    for x in a.iter_mut() {
        *x = r.mul(x, &n_inv_mont);
    }
}

/// In-place radix-2 DIT NTT (Cooley–Tukey).
fn ntt_core<R: Reducer<Lane>>(a: &mut [Lane], twiddles: &[Lane], r: &R) {
    let n = a.len();
    debug_assert!(n.is_power_of_two() && twiddles.len() == n / 2);

    let mut sub_len = 2usize;
    while sub_len <= n {
        let half = sub_len / 2;
        let step = n / sub_len;

        for i in (0..n).step_by(sub_len) {
            for j in 0..half {
                let u = a[i + j];
                let v = r.mul(&a[i + j + half], &twiddles[j * step]);
                a[i + j] = r.add(&u, &v);
                a[i + j + half] = r.sub(&u, &v);
            }
        }

        sub_len *= 2;
    }
}

/// Pointwise multiply of two transformed vectors in place.
pub fn pointwise_mul<R: Reducer<Lane>>(a_hat: &mut [Lane], b_hat: &[Lane], r: &R) {
    assert_eq!(a_hat.len(), b_hat.len());
    for (a, &b_val) in a_hat.iter_mut().zip(b_hat.iter()) {
        *a = r.mul(a, &b_val);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arch::ntt::{K, MODULI, OMEGA_MAX, P0, P1, P2};
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
        ($r:ident, $p:ident, $omega:ident, $body:block) => {
            for idx in 0..K {
                let $p = MODULI[idx];
                let $omega = OMEGA_MAX[idx];
                match idx {
                    0 => {
                        fn go<R: Reducer<Lane>>($r: &R, $p: Lane, $omega: Lane) $body
                        go::<crate::arch::ntt::Rp0>(&P0, $p, $omega);
                    }
                    1 => {
                        fn go<R: Reducer<Lane>>($r: &R, $p: Lane, $omega: Lane) $body
                        go::<crate::arch::ntt::Rp1>(&P1, $p, $omega);
                    }
                    2 => {
                        fn go<R: Reducer<Lane>>($r: &R, $p: Lane, $omega: Lane) $body
                        go::<crate::arch::ntt::Rp2>(&P2, $p, $omega);
                    }
                    _ => unreachable!(),
                }
            }
        };
    }

    #[test]
    fn test_forward_inverse_roundtrip() {
        for_each_prime!(r, p, omega, {
            for &n in &[2, 4, 8, 16, 32, 64, 128, 256, 512] {
                let mut fwd_twiddles = alloc::vec![0u64 as Lane; n / 2];
                let mut inv_twiddles = alloc::vec![0u64 as Lane; n / 2];
                precompute_twiddles(&mut fwd_twiddles, n, omega, false, r);
                precompute_twiddles(&mut inv_twiddles, n, omega, true, r);

                let mut a: Vec<Lane> = (0..n)
                    .map(|i| ((i as Lane + 1).wrapping_mul(123456789)) % p)
                    .collect();
                for val in a.iter_mut() {
                    *val = r.transform(*val);
                }
                let orig = a.clone();

                bit_reverse(&mut a);
                forward(&mut a, &fwd_twiddles, r);
                inverse(&mut a, &inv_twiddles, r);

                assert_all_eq(&a, &orig, "roundtrip failed for n={n}");
            }
        });
    }

    #[test]
    fn test_convolution_via_ntt() {
        for_each_prime!(r, p, omega, {
            for len_a in [1, 2, 3, 5] {
                for len_b in [1, 2, 3, 5] {
                    let conv_len: usize = len_a + len_b - 1;
                    let n = conv_len.next_power_of_two().max(2);

                    let a: Vec<Lane> = (0..len_a).map(|i| ((i + 1) as Lane * 12345) % p).collect();
                    let b_vec: Vec<Lane> =
                        (0..len_b).map(|i| ((i + 1) as Lane * 67890) % p).collect();

                    let mut expected = vec![0u64 as Lane; conv_len];
                    for (i, &ai) in a.iter().enumerate() {
                        for (j, &bj) in b_vec.iter().enumerate() {
                            let prod = (ai as u128 * bj as u128 % p as u128) as Lane;
                            expected[i + j] = r.add(&expected[i + j], &prod);
                        }
                    }

                    let mut fwd_twiddles = alloc::vec![0u64 as Lane; n / 2];
                    let mut inv_twiddles = alloc::vec![0u64 as Lane; n / 2];
                    precompute_twiddles(&mut fwd_twiddles, n, omega, false, r);
                    precompute_twiddles(&mut inv_twiddles, n, omega, true, r);

                    let mut a_pad = vec![0u64 as Lane; n];
                    let mut b_pad = vec![0u64 as Lane; n];
                    for i in 0..len_a {
                        a_pad[i] = r.transform(a[i]);
                    }
                    for i in 0..len_b {
                        b_pad[i] = r.transform(b_vec[i]);
                    }

                    bit_reverse(&mut a_pad);
                    bit_reverse(&mut b_pad);
                    forward(&mut a_pad, &fwd_twiddles, r);
                    forward(&mut b_pad, &fwd_twiddles, r);
                    pointwise_mul(&mut a_pad, &b_pad, r);
                    inverse(&mut a_pad, &inv_twiddles, r);
                    for val in a_pad[..conv_len].iter_mut() {
                        *val = r.residue(*val);
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
}
