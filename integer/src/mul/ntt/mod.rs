//! NTT-based multiplication for very large integers.
//!
//! Uses Number Theoretic Transforms over several 64-bit primes of the form
//! `2^64 - 2^b + 1` combined with the Chinese Remainder Theorem (CRT).

use crate::{
    add,
    arch::word::{SignedWord, Word},
    memory::{self, Memory},
    Sign::{self, *},
};
use alloc::alloc::Layout;

mod crt;
mod pack;
mod primes;
mod transform;

use crate::mul::ntt::crt::CrtConstants;
pub use primes::{K, PRIMES};

/// Minimum smaller-operand length (in words) for the NTT path.
pub const THRESHOLD_NTT: usize = 2048;

/// Smallest admissible coefficient bit width (used for worst-case memory bound).
const B_PACK_MIN: u32 = 16;

/// Maximum `log2(transform length)`, set by `min(v2) = 32` across all primes.
const MAX_LOG_N: u32 = 32;

/// Select NTT parameters for operands with the given word lengths.
///
/// Returns `(b_pack, N, K_eff)`.
pub fn select_params(la_words: usize, lb_words: usize) -> (u32, usize, usize) {
    let b_pack = B_PACK_MIN;
    let word_bits = Word::BITS;

    let la_bits = la_words as u64 * word_bits as u64;
    let lb_bits = lb_words as u64 * word_bits as u64;

    let coeffs_a = (la_bits + b_pack as u64 - 1) / b_pack as u64;
    let coeffs_b = (lb_bits + b_pack as u64 - 1) / b_pack as u64;
    let total_coeffs = (coeffs_a + coeffs_b - 1) as usize;
    let n = total_coeffs.next_power_of_two().max(2);

    assert!(
        (n.trailing_zeros()) <= MAX_LOG_N,
        "N = {n} too large for prime set (max log2 = {MAX_LOG_N})"
    );

    let k_eff = K;

    // Headroom check: max convolution coefficient < product of K_eff primes.
    // max_coeff fits in u128; compare against smallest prime p0.
    let max_coeff = (n as u128 / 2) * ((1u128 << b_pack) - 1) * ((1u128 << b_pack) - 1);
    let p0 = PRIMES[0].p as u128;
    assert!(
        max_coeff < p0,
        "headroom check failed: max coeff {max_coeff} >= smallest prime {p0}"
    );

    (b_pack, n, k_eff)
}

/// Estimate bit length from a word slice (excludes leading zeros).
fn bit_len(words: &[Word]) -> u64 {
    let leading_zeros = words.iter().rev().take_while(|&&w| w == 0).count();
    let used = words.len() - leading_zeros;
    if used == 0 {
        return 0;
    }
    let hi_word = words[used - 1];
    let hi_bits = Word::BITS - hi_word.leading_zeros();
    (used as u64 - 1) * Word::BITS as u64 + hi_bits as u64
}

/// Count number of coefficients needed for a given bit length.
fn coeff_count(bit_len: u64, b_pack: u32) -> usize {
    ((bit_len + b_pack as u64 - 1) / b_pack as u64) as usize
}

/// Worst-case scratch memory bound.
pub fn memory_requirement_up_to(total_len: usize, _smaller_len: usize) -> Layout {
    let word_bits = Word::BITS;
    let max_coeffs =
        (total_len as u64 * word_bits as u64 + B_PACK_MIN as u64 - 1) / B_PACK_MIN as u64;
    let n_max = ((max_coeffs + 1) as usize).next_power_of_two().max(2);

    // Everything is in u64 units for simplicity.
    let lanes_u64 = 2 * n_max; // a_lane + b_lane
    let residues_u64 = K * n_max; // per-prime inverse results
    let product_u64 = total_len; // product buffer (Word=u64 on 64-bit, else u64 takes more space)

    // On 64-bit targets Word = u64.  On narrow targets (Word < u64),
    // we need extra space for the u64 allocations.  Use the maximum
    // of Word and u64 sizes.
    let u64_bytes = 8usize;
    let word_bytes = core::mem::size_of::<Word>();
    let factor = (u64_bytes + word_bytes - 1) / word_bytes;
    let total_words = product_u64 + (lanes_u64 + residues_u64) * factor;

    memory::array_layout::<Word>(total_words)
}

/// `c += sign * a * b` with equal-length operands.
///
/// Returns carry.
#[must_use]
pub fn add_signed_mul_same_len(
    c: &mut [Word],
    sign: Sign,
    a: &[Word],
    b: &[Word],
    memory: &mut Memory,
) -> SignedWord {
    let n = a.len();
    debug_assert!(b.len() == n && c.len() == 2 * n);
    add_signed_mul_impl(c, sign, a, b, memory)
}

/// `c += sign * a * b` (general, a may be longer than b).
///
/// Returns carry.
#[must_use]
pub fn add_signed_mul(
    c: &mut [Word],
    sign: Sign,
    a: &[Word],
    b: &[Word],
    memory: &mut Memory,
) -> SignedWord {
    debug_assert!(a.len() >= b.len() && c.len() == a.len() + b.len());
    add_signed_mul_impl(c, sign, a, b, memory)
}

/// Core implementation: c += sign * a * b.
///
/// Does a single NTT convolution of the full operands (no chunking).
fn add_signed_mul_impl(
    c: &mut [Word],
    sign: Sign,
    a: &[Word],
    b: &[Word],
    memory: &mut Memory,
) -> SignedWord {
    let la = a.len();
    let lb = b.len();

    // Skip zero-length or zero-value operands
    if la == 0 || lb == 0 {
        return 0;
    }

    let (b_pack, nn, k_eff) = select_params(la, lb);
    let la_bits = bit_len(a);
    let lb_bits = bit_len(b);
    if la_bits == 0 || lb_bits == 0 {
        return 0;
    }

    let coeffs_a = coeff_count(la_bits, b_pack);
    let coeffs_b = coeff_count(lb_bits, b_pack);
    let output_coeffs = coeffs_a + coeffs_b - 1;

    // CRT constants
    let primes_p: alloc::vec::Vec<u64> = PRIMES[..k_eff].iter().map(|np| np.p).collect();
    let crt_constants = CrtConstants::new(&primes_p);

    // ---- Memory carve (longest-lived first) ----
    // All buffers are u64 since lane arithmetic is always u64.

    // 1. Product buffer
    let prod_len = la + lb;
    let (prod, mut mem) = memory.allocate_slice_fill::<u64>(prod_len, 0);

    // 2. Residue storage (per-prime inverse results, as u64)
    let residues_len = k_eff * nn;
    let (residues, mut mem) = mem.allocate_slice_fill::<u64>(residues_len, 0);

    // 3. Lane buffers (reused across primes, as u64)
    let (a_lane, mut mem) = mem.allocate_slice_fill::<u64>(nn, 0);
    let (b_lane, _mem) = mem.allocate_slice_fill::<u64>(nn, 0);

    // ---- Per-prime transforms ----
    for (pi, prime) in PRIMES[..k_eff].iter().enumerate() {
        let p = prime.p;
        let b_exp = prime.b;

        // Precompute twiddles
        let fwd_twiddles = transform::precompute_twiddles(nn, p, b_exp, prime.omega_2_32, false);
        let inv_twiddles = transform::precompute_twiddles(nn, p, b_exp, prime.omega_2_32, true);

        // Pack operands into lane buffers
        pack_into(a, b_pack, a_lane);
        pack_into(b, b_pack, b_lane);

        // Forward NTT
        transform::bit_reverse(a_lane);
        transform::bit_reverse(b_lane);
        transform::forward(a_lane, &fwd_twiddles, p, b_exp);
        transform::forward(b_lane, &fwd_twiddles, p, b_exp);
        transform::pointwise_mul(a_lane, b_lane, b_exp);
        transform::inverse(a_lane, &inv_twiddles, p, b_exp);

        // Store residues for this prime
        let offset = pi * nn;
        residues[offset..offset + nn].copy_from_slice(a_lane);
    }

    // ---- CRT per coefficient + accumulate ----
    // We'll accumulate each coefficient into the product buffer with
    // b_pack-bit shift.
    let output_words = la + lb;
    for k in 0..output_coeffs {
        let mut coeff_residues = [0u64; 3];
        #[allow(clippy::needless_range_loop)]
        for pi in 0..k_eff {
            let offset = pi * nn;
            coeff_residues[pi] = residues[offset + k];
        }
        let crt_val = crt::garner_combine(&coeff_residues[..k_eff], &primes_p, &crt_constants);

        // Unpack-accumulate this coefficient into prod
        // crt_val is a small integer (≤ 3 u64 words)
        add_shifted_to_prod(prod, &crt_val, k, b_pack);
    }

    // ---- Fold product into c with sign ----
    // Convert u64 slice to Word slice for the add function.
    // On 64-bit targets these are the same type.
    assert_eq!(
        core::mem::size_of::<Word>(),
        core::mem::size_of::<u64>(),
        "NTT requires 64-bit Word"
    );
    // SAFETY: Word and u64 have the same size (asserted above) and
    // prod is allocated with u64 alignment, compatible with Word.
    let prod_words: &[Word] =
        unsafe { core::slice::from_raw_parts(prod.as_ptr() as *const Word, output_words) };
    match sign {
        Positive => add::add_signed_in_place(c, Positive, prod_words),
        Negative => add::add_signed_in_place(c, Negative, prod_words),
    }
}

/// Pack word slice into coefficient buffer (viewed as u64).
fn pack_into(words: &[Word], b_pack: u32, out: &mut [u64]) {
    let packed = pack::pack(words, b_pack, out.len());
    out.copy_from_slice(&packed);
}

/// Add a small multi-word integer (up to 3 u64 words) to `prod`, shifted
/// left by `k * b_pack` bits.
fn add_shifted_to_prod(prod: &mut [u64], val: &[u64], k: usize, b_pack: u32) {
    if val.is_empty() {
        return;
    }
    let shift_bits = (k as u32).wrapping_mul(b_pack);
    let word_idx = (shift_bits / 64) as usize;
    let bit_shift = shift_bits % 64;

    let mut carry: u64 = 0;
    for (vi, &v) in val.iter().enumerate() {
        let idx = word_idx + vi;
        if idx >= prod.len() {
            return;
        }
        let v128 = v as u128;

        if bit_shift == 0 {
            let sum = v128.wrapping_add(carry as u128);
            let (r, c) = prod[idx].overflowing_add(sum as u64);
            prod[idx] = r;
            carry = (sum >> 64) as u64 + c as u64;
        } else {
            // v << bit_shift has high bits = v >> (64 - bit_shift) = lo_carry.
            // No separate hi — lo_carry IS the high part.
            let lo = v128 << bit_shift;
            let sum = lo.wrapping_add(carry as u128);
            let lo_carry = (sum >> 64) as u64;
            let lo_word = sum as u64;

            let (r, c1) = prod[idx].overflowing_add(lo_word);
            prod[idx] = r;
            carry = lo_carry + c1 as u64;

            // Propagate to next word
            if idx + 1 < prod.len() && carry != 0 {
                let (r2, c2) = prod[idx + 1].overflowing_add(carry);
                prod[idx + 1] = r2;
                carry = c2 as u64;
            }
        }
    }

    // Propagate final carry
    let mut idx = word_idx + val.len();
    while carry != 0 && idx < prod.len() {
        let (r, c) = prod[idx].overflowing_add(carry);
        prod[idx] = r;
        carry = c as u64;
        idx += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_select_params_small() {
        let (b_pack, n, k_eff) = select_params(10, 10);
        assert_eq!(b_pack, 16);
        assert!(n >= 2 && n.is_power_of_two());
        assert_eq!(k_eff, K);
    }

    #[test]
    fn test_select_params_large() {
        let (b_pack, n, k_eff) = select_params(THRESHOLD_NTT, THRESHOLD_NTT);
        assert_eq!(b_pack, 16);
        assert!(n.is_power_of_two());
        assert_eq!(k_eff, K);
        let coeffs_a = (THRESHOLD_NTT * Word::BITS as usize + 15) / 16;
        let coeffs_b = coeffs_a;
        let min_n = (coeffs_a + coeffs_b).next_power_of_two().max(2);
        assert!(n >= min_n, "n={n} < min_n={min_n}");
    }

    #[test]
    fn test_headroom_holds() {
        let la = THRESHOLD_NTT;
        let lb = THRESHOLD_NTT;
        let (b_pack, n, _k_eff) = select_params(la, lb);
        let max_coeff = (n as u128 / 2) * ((1u128 << b_pack) - 1) * ((1u128 << b_pack) - 1);
        let p0 = PRIMES[0].p as u128;
        assert!(max_coeff < p0, "headroom violation: max_coeff={max_coeff} >= p0={p0}");
    }

    #[test]
    fn test_bit_len() {
        assert_eq!(bit_len(&[]), 0);
        assert_eq!(bit_len(&[0]), 0);
        assert_eq!(bit_len(&[1]), 1);
        assert_eq!(bit_len(&[0, 1]), 65);
        assert_eq!(bit_len(&[0xFF, 0]), 8);
    }

    #[test]
    fn test_ntt_multiply_one_word() {
        // Simplest case: single-word operands
        let a: Vec<Word> = vec![3];
        let b: Vec<Word> = vec![5];
        let mut c = vec![0u64; 2];
        let layout = memory_requirement_up_to(c.len(), b.len());
        let mut alloc = crate::memory::MemoryAllocation::new(layout);
        let mut memory = alloc.memory();
        let carry = add_signed_mul_impl(&mut c, Positive, &a, &b, &mut memory);
        assert_eq!(carry, 0);
        assert_eq!(c[0], 15);
        assert_eq!(c[1], 0);
    }

    #[test]
    fn test_ntt_multiply_two_words() {
        // Two-word operands
        let a: Vec<Word> = vec![Word::MAX, 1]; // 2^64 + (2^64-1)
        let b: Vec<Word> = vec![2, 0]; // 2
        let expected = schoolbook_mul(&a, &b);
        let mut c = vec![0u64; a.len() + b.len()];
        let layout = memory_requirement_up_to(c.len(), b.len());
        let mut alloc = crate::memory::MemoryAllocation::new(layout);
        let mut memory = alloc.memory();
        add_signed_mul_impl(&mut c, Positive, &a, &b, &mut memory);
        assert_eq!(&c[..], &expected[..], "two-word mismatch");
    }

    #[test]
    fn test_ntt_multiply_small() {
        // Test NTT multiply with small operands that exceed THRESHOLD_NTT.
        let a: Vec<Word> = vec![0xDEADBEEFu64; THRESHOLD_NTT];
        let b: Vec<Word> = vec![0xCAFEBABEu64; THRESHOLD_NTT];
        let mut c = vec![0u64; a.len() + b.len()];

        let layout = memory_requirement_up_to(c.len(), b.len());
        let mut alloc = crate::memory::MemoryAllocation::new(layout);
        let mut memory = alloc.memory();
        let carry = add_signed_mul_impl(&mut c, Positive, &a, &b, &mut memory);
        assert_eq!(carry, 0);
        assert!(c.iter().any(|&w| w != 0));
    }

    /// Naive schoolbook multiplication for comparison.
    fn schoolbook_mul(a: &[Word], b: &[Word]) -> Vec<Word> {
        let mut c = vec![0u64; a.len() + b.len()];
        for (i, &ai) in a.iter().enumerate() {
            let mut carry: u128 = 0;
            for (j, &bj) in b.iter().enumerate() {
                let idx = i + j;
                let prod = (ai as u128) * (bj as u128) + (c[idx] as u128) + carry;
                c[idx] = prod as u64;
                carry = prod >> 64;
            }
            // Propagate carry into higher words
            let mut k = i + b.len();
            while carry != 0 {
                let sum = (c[k] as u128) + carry;
                c[k] = sum as u64;
                carry = sum >> 64;
                k += 1;
            }
        }
        c
    }

    /// Test NTT against schoolbook with moderate operand sizes.
    fn run_ntt_vs_schoolbook(la: usize, lb: usize) {
        // Generate deterministic test data
        let a: Vec<Word> = (0..la)
            .map(|i| (i as u64 + 1).wrapping_mul(0x9E3779B97F4A7C15))
            .collect();
        let b: Vec<Word> = (0..lb)
            .map(|i| (i as u64 + 1).wrapping_mul(0xC6A4A7935BD1E995))
            .collect();
        let expected = schoolbook_mul(&a, &b);

        let mut c = vec![0u64; a.len() + b.len()];
        let layout = memory_requirement_up_to(c.len(), b.len());
        let mut alloc = crate::memory::MemoryAllocation::new(layout);
        let mut memory = alloc.memory();
        let carry = add_signed_mul_impl(&mut c, Positive, &a, &b, &mut memory);
        assert_eq!(carry, 0, "carry should be 0");

        assert_eq!(&c[..], &expected[..], "NTT mismatch: la={la}, lb={lb}");
    }

    #[test]
    fn test_ntt_vs_schoolbook_equal() {
        for &len in &[20, 30, 50, 64, 100, 128] {
            run_ntt_vs_schoolbook(len, len);
        }
    }

    #[test]
    fn test_ntt_vs_schoolbook_unequal() {
        for &(la, lb) in &[(30, 20), (50, 30), (100, 50), (128, 64), (100, 20)] {
            run_ntt_vs_schoolbook(la, lb);
        }
    }

    #[test]
    fn test_ntt_vs_schoolbook_asymmetric() {
        // Very asymmetric sizes
        for &(la, lb) in &[(200, 30), (150, 20)] {
            run_ntt_vs_schoolbook(la, lb);
        }
    }

    #[test]
    fn test_ntt_all_ones() {
        // All-ones operands stress the carry chain.
        for &len in &[20, 50] {
            let a = vec![Word::MAX; len];
            let b = vec![Word::MAX; len];
            let expected = schoolbook_mul(&a, &b);

            let mut c = vec![0u64; a.len() + b.len()];
            let layout = memory_requirement_up_to(c.len(), b.len());
            let mut alloc = crate::memory::MemoryAllocation::new(layout);
            let mut memory = alloc.memory();
            add_signed_mul_impl(&mut c, Positive, &a, &b, &mut memory);
            assert_eq!(&c[..], &expected[..], "all-ones mismatch len={len}");
        }
    }

    #[test]
    fn test_ntt_zero_operand() {
        let a = vec![0xDEADu64; 30];
        let b = vec![0u64; 30];
        let mut c = vec![0u64; a.len() + b.len()];
        let layout = memory_requirement_up_to(c.len(), b.len());
        let mut alloc = crate::memory::MemoryAllocation::new(layout);
        let mut memory = alloc.memory();
        let carry = add_signed_mul_impl(&mut c, Positive, &a, &b, &mut memory);
        assert_eq!(carry, 0);
        assert!(c.iter().all(|&w| w == 0), "zero operand should give zero product");
    }

    #[test]
    fn test_ntt_sign_negative() {
        // Test that Negative sign works (c -= a * b)
        let a: Vec<Word> = (0..30).map(|i| (i as u64 + 1) * 100).collect();
        let b: Vec<Word> = (0..30).map(|i| (i as u64 + 1) * 200).collect();
        let _expected = schoolbook_mul(&a, &b);

        // First add: c += a * b
        let mut c = vec![0u64; a.len() + b.len()];
        let layout = memory_requirement_up_to(c.len(), b.len());
        let mut alloc = crate::memory::MemoryAllocation::new(layout);
        let mut memory = alloc.memory();
        let _ = add_signed_mul_impl(&mut c, Positive, &a, &b, &mut memory);

        // Then subtract: c -= a * b
        let layout2 = memory_requirement_up_to(c.len(), b.len());
        let mut alloc2 = crate::memory::MemoryAllocation::new(layout2);
        let mut memory2 = alloc2.memory();
        let _ = add_signed_mul_impl(&mut c, Negative, &a, &b, &mut memory2);

        // Result should be zero
        assert!(c.iter().all(|&w| w == 0), "add then subtract should give zero");
    }

    #[test]
    fn test_ntt_high_low_zero_limbs() {
        // Operands with leading/trailing zero limbs
        let mut a = vec![0u64; 80];
        let mut b = vec![0u64; 80];
        for i in 20..60 {
            a[i] = (i as u64 + 1).wrapping_mul(0xDEADBEEF);
            b[i] = (i as u64 + 1).wrapping_mul(0xCAFEBABE);
        }
        let expected = schoolbook_mul(&a, &b);

        let mut c = vec![0u64; a.len() + b.len()];
        let layout = memory_requirement_up_to(c.len(), b.len());
        let mut alloc = crate::memory::MemoryAllocation::new(layout);
        let mut memory = alloc.memory();
        add_signed_mul_impl(&mut c, Positive, &a, &b, &mut memory);
        assert_eq!(&c[..], &expected[..], "sparse operand mismatch");
    }
}
