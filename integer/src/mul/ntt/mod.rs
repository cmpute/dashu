//! NTT-based multiplication for very large integers.
//!
//! Uses Number Theoretic Transforms over Proth primes of the form
//! `K * 2^N + 1` combined with the Chinese Remainder Theorem (CRT).

use crate::{
    add,
    arch::word::{SignedWord, Word},
    memory::{self, Memory},
    Sign::{self, *},
};
use alloc::alloc::Layout;
use core::mem;

mod crt;
mod pack;
mod transform;

use crate::arch::ntt::{mul_mod, B_PACK_CANDIDATES, B_PACK_MIN, K, MAX_LOG_N, MODULI, OMEGA_MAX};
use crate::mul::ntt::crt::{garner_combine, CrtAccum};

/// Minimum smaller-operand length (in words) for the NTT path.
pub const THRESHOLD_NTT: usize = 40_000;

/// Select NTT parameters for operands with the given word lengths.
///
/// Returns `(b_pack, N, K_eff)`.
pub fn select_params(la_words: usize, lb_words: usize) -> (u32, usize, usize) {
    let word_bits = Word::BITS;
    let la_bits = la_words as u64 * word_bits as u64;
    let lb_bits = lb_words as u64 * word_bits as u64;
    let prod_2 = (MODULI[0] as u128) * (MODULI[1] as u128);

    for &b_pack in B_PACK_CANDIDATES {
        let coeffs_a = (la_bits + b_pack as u64 - 1) / b_pack as u64;
        let coeffs_b = (lb_bits + b_pack as u64 - 1) / b_pack as u64;
        let total_coeffs = (coeffs_a + coeffs_b - 1) as usize;
        let n = total_coeffs.next_power_of_two().max(2);

        if (n.trailing_zeros()) > MAX_LOG_N {
            continue;
        }

        // Compute max coefficient value, guarding against u128 overflow for
        // b_pack = 64 where (2^64−1)² ≈ 2^128 and n/2 can push it past 2^128.
        let coeff_max = (1u128 << b_pack) - 1;
        let max_coeff = coeff_max
            .checked_mul(coeff_max)
            .and_then(|sq| (n as u128 / 2).checked_mul(sq));

        let k_eff = match max_coeff {
            Some(mc) if mc < prod_2 => 2,
            _ => K,
        };
        return (b_pack, n, k_eff);
    }

    unreachable!(
        "b_pack = {} always passes the headroom check",
        B_PACK_CANDIDATES.last().unwrap()
    )
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
    use crate::arch::ntt::Lane;

    let word_bits = Word::BITS;
    let max_coeffs =
        (total_len as u64 * word_bits as u64 + B_PACK_MIN as u64 - 1) / B_PACK_MIN as u64;
    let n_max = ((max_coeffs + 1) as usize).next_power_of_two().max(2);

    let lanes = 2 * n_max;
    let residues = K * n_max;
    let twiddles = n_max;
    let product = total_len;

    let lane_bytes = mem::size_of::<Lane>();
    let u64_bytes = 8usize;
    let word_bytes = mem::size_of::<Word>();

    let lanes_words = lanes * lane_bytes / word_bytes;
    let residues_words = residues * lane_bytes / word_bytes;
    let twiddles_words = twiddles * lane_bytes / word_bytes;
    let product_words = product * u64_bytes / word_bytes;
    let total_words = product_words + lanes_words + residues_words + twiddles_words;

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
    use crate::arch::ntt::Lane;

    let la = a.len();
    let lb = b.len();

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

    // ---- Memory carve (longest-lived first) ----

    // 1. Product buffer (always u64)
    let prod_len = la + lb;
    let (prod, mut mem) = memory.allocate_slice_fill::<u64>(prod_len, 0);

    // 2. Residue storage (per-prime inverse results)
    let residues_len = k_eff * nn;
    let (residues, mut mem) = mem.allocate_slice_fill::<Lane>(residues_len, 0);

    // 3. Lane buffers (reused across primes)
    let (a_lane, mut mem) = mem.allocate_slice_fill::<Lane>(nn, 0);
    let (b_lane, mut mem) = mem.allocate_slice_fill::<Lane>(nn, 0);

    // 4. Twiddle tables (fwd + inv, reused per prime)
    let (fwd_twiddles, mut mem) = mem.allocate_slice_fill::<Lane>(nn / 2, 0);
    let (inv_twiddles, _) = mem.allocate_slice_fill::<Lane>(nn / 2, 0);

    // ---- Per-prime transforms (const-generic dispatch) ----
    for pi in 0..k_eff {
        let mut ctx = TransformCtx {
            a_lane,
            b_lane,
            fwd_twiddles,
            inv_twiddles,
            p: MODULI[pi],
            omega_max: OMEGA_MAX[pi],
            nn,
            b_pack,
            residues,
            pi,
        };
        match pi {
            0 => process_prime::<0>(a, b, &mut ctx),
            1 => process_prime::<1>(a, b, &mut ctx),
            2 => process_prime::<2>(a, b, &mut ctx),
            _ => unreachable!(),
        }
    }

    // ---- CRT per coefficient + accumulate ----
    // Extract prime constants as both u64 and u32 so the Lane-size
    // dispatch below type-checks correctly in both branches.
    // The dead branch (wrong width) is eliminated by the compiler.
    let primes_u64: [u64; K] = [MODULI[0], MODULI[1], MODULI[2]];
    let crt_inv_u64: [[u64; K]; K] = {
        use crate::arch::ntt::CRT_INV_IJ;
        let mut m = [[0u64; K]; K];
        for i in 0..K {
            for j in 0..K {
                m[i][j] = CRT_INV_IJ[i][j];
            }
        }
        m
    };
    let primes_u32: [u32; K] = [
        MODULI[0] as u32,
        MODULI[1] as u32,
        MODULI[2] as u32,
    ];
    let crt_inv_u32: [[u32; K]; K] = {
        let mut m = [[0u32; K]; K];
        for i in 0..K {
            for j in 0..K {
                m[i][j] = crt_inv_u64[i][j] as u32;
            }
        }
        m
    };

    #[allow(clippy::unnecessary_cast)]
    if mem::size_of::<Lane>() == 8 {
        // SAFETY: mem::size_of::<Lane>() == 8, so Lane = u64 and
        // residues is backed by u64 elements.
        let residues_u64: &[u64] =
            unsafe { core::slice::from_raw_parts(residues.as_ptr() as *const u64, residues.len()) };
        do_crt_u64(
            prod,
            residues_u64,
            k_eff,
            nn,
            output_coeffs,
            b_pack,
            &primes_u64,
            &crt_inv_u64,
        );
    } else {
        #[allow(clippy::unnecessary_cast)]
        // SAFETY: mem::size_of::<Lane>() != 8, so Lane = u32 and
        // residues is backed by u32 elements.
        let residues_u32: &[u32] =
            unsafe { core::slice::from_raw_parts(residues.as_ptr() as *const u32, residues.len()) };
        do_crt_u32(
            prod,
            residues_u32,
            k_eff,
            nn,
            output_coeffs,
            b_pack,
            &primes_u32,
            &crt_inv_u32,
        );
    }

    // ---- Fold product into c with sign ----
    let output_words = la + lb;
    fold_prod_into_c(c, sign, prod, output_words)
}

/// CRT + accumulate for 64-bit lanes (U192 accumulator).
#[allow(clippy::too_many_arguments)]
#[inline(never)]
fn do_crt_u64(
    prod: &mut [u64],
    residues: &[u64],
    k_eff: usize,
    nn: usize,
    output_coeffs: usize,
    b_pack: u32,
    primes: &[u64; K],
    crt_inv: &[[u64; K]; K],
) {
    use crate::mul::ntt::crt::U192;

    for k in 0..output_coeffs {
        let mut coeff_residues = [0u64; 3];
        #[allow(clippy::needless_range_loop)]
        for pi in 0..k_eff {
            coeff_residues[pi] = residues[pi * nn + k];
        }
        let crt_val =
            garner_combine::<U192>(&coeff_residues[..k_eff], crt_inv, primes);
        add_shifted_to_prod(prod, crt_val.as_u64_slice(), crt_val.len_words(), k, b_pack);
    }
}

/// CRT + accumulate for 32-bit lanes (U96 accumulator).
#[allow(clippy::too_many_arguments)]
#[inline(never)]
fn do_crt_u32(
    prod: &mut [u64],
    residues: &[u32],
    k_eff: usize,
    nn: usize,
    output_coeffs: usize,
    b_pack: u32,
    primes: &[u32; K],
    crt_inv: &[[u32; K]; K],
) {
    use crate::mul::ntt::crt::U96;

    for k in 0..output_coeffs {
        let mut coeff_residues = [0u32; 3];
        #[allow(clippy::needless_range_loop)]
        for pi in 0..k_eff {
            coeff_residues[pi] = residues[pi * nn + k];
        }
        let crt_val =
            garner_combine::<U96>(&coeff_residues[..k_eff], crt_inv, primes);
        add_shifted_to_prod(prod, crt_val.as_u64_slice(), crt_val.len_words(), k, b_pack);
    }
}

/// Scratch buffers and parameters for one prime's NTT pipeline.
struct TransformCtx<'a> {
    a_lane: &'a mut [crate::arch::ntt::Lane],
    b_lane: &'a mut [crate::arch::ntt::Lane],
    fwd_twiddles: &'a mut [crate::arch::ntt::Lane],
    inv_twiddles: &'a mut [crate::arch::ntt::Lane],
    p: crate::arch::ntt::Lane,
    omega_max: crate::arch::ntt::Lane,
    nn: usize,
    b_pack: u32,
    residues: &'a mut [crate::arch::ntt::Lane],
    pi: usize,
}

/// Per-prime NTT pipeline, monomorphized for a specific prime index `PI`.
#[inline(never)]
fn process_prime<const PI: usize>(
    a: &[Word],
    b: &[Word],
    ctx: &mut TransformCtx<'_>,
) {
    use crate::arch::ntt::{to_monty, from_monty};

    pack::pack(ctx.a_lane, a, ctx.b_pack, ctx.nn);
    pack::pack(ctx.b_lane, b, ctx.b_pack, ctx.nn);

    // Convert standard-form coefficients to Montgomery form.
    // transform() handles any value in [0, 2^BITS), no pre-reduction needed.
    for c in ctx.a_lane[..ctx.nn].iter_mut() {
        *c = to_monty::<PI>(*c);
    }
    for c in ctx.b_lane[..ctx.nn].iter_mut() {
        *c = to_monty::<PI>(*c);
    }

    transform::precompute_twiddles::<PI>(ctx.fwd_twiddles, ctx.nn, ctx.p, ctx.omega_max, false);
    transform::precompute_twiddles::<PI>(ctx.inv_twiddles, ctx.nn, ctx.p, ctx.omega_max, true);

    transform::bit_reverse(ctx.a_lane);
    transform::bit_reverse(ctx.b_lane);
    transform::forward::<PI>(ctx.a_lane, ctx.fwd_twiddles);
    transform::forward::<PI>(ctx.b_lane, ctx.fwd_twiddles);
    transform::pointwise_mul::<PI>(ctx.a_lane, ctx.b_lane);
    transform::inverse::<PI>(ctx.a_lane, ctx.inv_twiddles, ctx.p);

    // Convert residues back from Montgomery to standard form.
    for c in ctx.a_lane[..ctx.nn].iter_mut() {
        *c = from_monty::<PI>(*c);
    }

    let offset = ctx.pi * ctx.nn;
    ctx.residues[offset..offset + ctx.nn].copy_from_slice(ctx.a_lane);
}

/// Add a CRT value (as u64 words) to `prod`, shifted left by `k * b_pack` bits.
fn add_shifted_to_prod(prod: &mut [u64], words: &[u64], count: u32, k: usize, b_pack: u32) {
    let shift_bits = (k as u32).wrapping_mul(b_pack);
    let word_idx = (shift_bits / 64) as usize;
    let bit_shift = shift_bits % 64;

    let mut carry: u64 = 0;
    #[allow(clippy::needless_range_loop)]
    for vi in 0..(count as usize) {
        let idx = word_idx + vi;
        if idx >= prod.len() {
            return;
        }
        let v = words[vi];
        let v128 = v as u128;

        if bit_shift == 0 {
            let sum = v128.wrapping_add(carry as u128);
            let (r, c) = prod[idx].overflowing_add(sum as u64);
            prod[idx] = r;
            carry = (sum >> 64) as u64 + c as u64;
        } else {
            let lo = v128 << bit_shift;
            let sum = lo.wrapping_add(carry as u128);
            let lo_carry = (sum >> 64) as u64;
            let lo_word = sum as u64;

            let (r, c1) = prod[idx].overflowing_add(lo_word);
            prod[idx] = r;
            carry = lo_carry + c1 as u64;

            if idx + 1 < prod.len() && carry != 0 {
                let (r2, c2) = prod[idx + 1].overflowing_add(carry);
                prod[idx + 1] = r2;
                carry = c2 as u64;
            }
        }
    }

    let mut idx = word_idx + count as usize;
    while carry != 0 && idx < prod.len() {
        let (r, c) = prod[idx].overflowing_add(carry);
        prod[idx] = r;
        carry = c as u64;
        idx += 1;
    }
}

/// Fold the u64 product buffer into the Word output array `c`.
///
/// For 64-bit Word targets, `u64` == `Word`, so a direct transmute suffices.
/// For 32-bit Word targets, each u64 is split into two u32 words with carry.
fn fold_prod_into_c(c: &mut [Word], sign: Sign, prod: &[u64], output_words: usize) -> SignedWord {
    if mem::size_of::<Word>() == 8 {
        // 64-bit Word: direct transmute
        assert_eq!(
            mem::size_of::<Word>(),
            mem::size_of::<u64>(),
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
    } else {
        // 32-bit Word: split each u64 into two u32 words.
        // For 64-bit Word targets this branch is dead (eliminated by the compiler)
        // but must still type-check.
        let mut carry: u32 = 0;
        let double_output_words = output_words.min(prod.len() * 2);
        for i in 0..double_output_words {
            let prod_word = prod[i / 2];
            let lo_word = if i % 2 == 0 {
                (prod_word as u32).wrapping_add(carry)
            } else {
                ((prod_word >> 32) as u32).wrapping_add(carry)
            };
            // Carry out of the low-word addition
            if i % 2 == 0 {
                let lo_before = prod_word as u32;
                carry = (prod_word >> 32) as u32;
                if lo_word < lo_before {
                    carry = carry.wrapping_add(1);
                }
            }
            if i < c.len() {
                let c_val = c[i] as u32;
                match sign {
                    Positive => {
                        let (sum, c_out) = c_val.overflowing_add(lo_word);
                        c[i] = sum as Word;
                        carry = carry.wrapping_add(c_out as u32);
                    }
                    Negative => {
                        let (diff, b) = c_val.overflowing_sub(lo_word);
                        c[i] = diff as Word;
                        carry = carry.wrapping_add(b as u32);
                    }
                }
            }
        }
        if sign == Positive {
            carry as SignedWord
        } else {
            -(carry as SignedWord)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(not(feature = "std"))]
    use alloc::vec;
    #[cfg(not(feature = "std"))]
    use alloc::vec::Vec;

    #[test]
    fn test_select_params_small() {
        let (b_pack, n, k_eff) = select_params(10, 10);
        // On 64-bit: B_PACK_CANDIDATES[0] = 64, needs K_eff = 3.
        // On 32-bit: B_PACK_CANDIDATES[0] = 32, likely K_eff = 2.
        assert!(b_pack >= 32);
        assert!(n >= 2 && n.is_power_of_two());
        assert!((2..=K).contains(&k_eff));
    }

    #[test]
    fn test_select_params_large() {
        let (b_pack, n, _k_eff) = select_params(THRESHOLD_NTT, THRESHOLD_NTT);
        assert!(b_pack >= 32);
        assert!(n.is_power_of_two());
        let coeffs_a =
            (THRESHOLD_NTT * Word::BITS as usize + b_pack as usize - 1) / b_pack as usize;
        let coeffs_b = coeffs_a;
        let min_n = (coeffs_a + coeffs_b).next_power_of_two().max(2);
        assert!(n >= min_n, "n={n} < min_n={min_n}");
    }

    #[test]
    fn test_bit_len() {
        assert_eq!(bit_len(&[]), 0);
        assert_eq!(bit_len(&[0]), 0);
        assert_eq!(bit_len(&[1]), 1);
    }

    #[test]
    fn test_ntt_multiply_one_word() {
        let a: Vec<Word> = vec![3];
        let b: Vec<Word> = vec![5];
        let mut c = vec![0u64 as Word; 2];
        let layout = memory_requirement_up_to(c.len(), b.len());
        let mut alloc = crate::memory::MemoryAllocation::new(layout);
        let mut memory = alloc.memory();
        let carry = add_signed_mul_impl(&mut c, Positive, &a, &b, &mut memory);
        assert_eq!(carry, 0);
        assert_eq!(c[0], 15);
        assert_eq!(c[1], 0);
    }

    #[test]
    fn test_ntt_zero_operand() {
        let a = vec![0xDEADu64 as Word; 30];
        let b = vec![0u64 as Word; 30];
        let mut c = vec![0u64 as Word; a.len() + b.len()];
        let layout = memory_requirement_up_to(c.len(), b.len());
        let mut alloc = crate::memory::MemoryAllocation::new(layout);
        let mut memory = alloc.memory();
        let carry = add_signed_mul_impl(&mut c, Positive, &a, &b, &mut memory);
        assert_eq!(carry, 0);
        assert!(c.iter().all(|&w| w == 0));
    }

    #[test]
    fn test_ntt_sign_negative() {
        let a: Vec<Word> = (0..30).map(|i| (i as Word + 1) * 100).collect();
        let b: Vec<Word> = (0..30).map(|i| (i as Word + 1) * 200).collect();

        let mut c = vec![0u64 as Word; a.len() + b.len()];
        let layout = memory_requirement_up_to(c.len(), b.len());
        let mut alloc = crate::memory::MemoryAllocation::new(layout);
        let mut memory = alloc.memory();
        let _ = add_signed_mul_impl(&mut c, Positive, &a, &b, &mut memory);

        let layout2 = memory_requirement_up_to(c.len(), b.len());
        let mut alloc2 = crate::memory::MemoryAllocation::new(layout2);
        let mut memory2 = alloc2.memory();
        let _ = add_signed_mul_impl(&mut c, Negative, &a, &b, &mut memory2);

        assert!(c.iter().all(|&w| w == 0));
    }

    const NTT_TEST_LEN: usize = 512;

    #[test]
    fn test_ntt_multiply_small() {
        let a: Vec<Word> = vec![0xDEADBEEFu64 as Word; NTT_TEST_LEN];
        let b: Vec<Word> = vec![0xCAFEBABEu64 as Word; NTT_TEST_LEN];
        let mut c = vec![0u64 as Word; a.len() + b.len()];

        let layout = memory_requirement_up_to(c.len(), b.len());
        let mut alloc = crate::memory::MemoryAllocation::new(layout);
        let mut memory = alloc.memory();
        let carry = add_signed_mul_impl(&mut c, Positive, &a, &b, &mut memory);
        assert_eq!(carry, 0);
        assert!(c.iter().any(|&w| w != 0));
    }

    /// Naive schoolbook multiplication for comparison.
    fn schoolbook_mul(a: &[Word], b: &[Word]) -> Vec<Word> {
        let mut c = vec![0u64 as Word; a.len() + b.len()];
        for (i, &ai) in a.iter().enumerate() {
            let mut carry: u128 = 0;
            for (j, &bj) in b.iter().enumerate() {
                let idx = i + j;
                let prod = (ai as u128) * (bj as u128) + (c[idx] as u128) + carry;
                c[idx] = prod as Word;
                carry = prod >> Word::BITS;
            }
            let mut k = i + b.len();
            while carry != 0 {
                let sum = (c[k] as u128) + carry;
                c[k] = sum as Word;
                carry = sum >> Word::BITS;
                k += 1;
            }
        }
        c
    }

    fn run_ntt_vs_schoolbook(la: usize, lb: usize) {
        let a: Vec<Word> = (0..la)
            .map(|i| (i as Word + 1).wrapping_mul(0x9E3779B97F4A7C15u64 as Word))
            .collect();
        let b: Vec<Word> = (0..lb)
            .map(|i| (i as Word + 1).wrapping_mul(0xC6A4A7935BD1E995u64 as Word))
            .collect();
        let expected = schoolbook_mul(&a, &b);

        let mut c = vec![0u64 as Word; a.len() + b.len()];
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
        for &(la, lb) in &[(200, 30), (150, 20)] {
            run_ntt_vs_schoolbook(la, lb);
        }
    }

    #[test]
    fn test_ntt_all_ones() {
        for &len in &[20, 50] {
            let a = vec![Word::MAX; len];
            let b = vec![Word::MAX; len];
            let expected = schoolbook_mul(&a, &b);

            let mut c = vec![0u64 as Word; a.len() + b.len()];
            let layout = memory_requirement_up_to(c.len(), b.len());
            let mut alloc = crate::memory::MemoryAllocation::new(layout);
            let mut memory = alloc.memory();
            add_signed_mul_impl(&mut c, Positive, &a, &b, &mut memory);
            assert_eq!(&c[..], &expected[..], "all-ones mismatch len={len}");
        }
    }

    #[test]
    fn test_ntt_high_low_zero_limbs() {
        let mut a = vec![0u64 as Word; 80];
        let mut b = vec![0u64 as Word; 80];
        for i in 20..60 {
            a[i] = (i as Word + 1).wrapping_mul(0xDEADBEEF);
            b[i] = (i as Word + 1).wrapping_mul(0xCAFEBABE);
        }
        let expected = schoolbook_mul(&a, &b);

        let mut c = vec![0u64 as Word; a.len() + b.len()];
        let layout = memory_requirement_up_to(c.len(), b.len());
        let mut alloc = crate::memory::MemoryAllocation::new(layout);
        let mut memory = alloc.memory();
        add_signed_mul_impl(&mut c, Positive, &a, &b, &mut memory);
        assert_eq!(&c[..], &expected[..], "sparse operand mismatch");
    }
}
