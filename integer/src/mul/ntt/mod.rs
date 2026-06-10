//! NTT-based multiplication for very large integers.
//!
//! Uses Number Theoretic Transforms over several 64-bit primes of the form
//! `2^64 - 2^b + 1` combined with the Chinese Remainder Theorem (CRT).

use crate::mul::ntt::crt::ModOps;
use crate::{
    add,
    arch::word::{SignedWord, Word},
    memory::{self, Memory},
    Sign::{self, *},
};
use alloc::alloc::Layout;
use num_modular::{FixedTrinomialSolinas64, Reducer};

mod crt;
mod pack;
mod primes;
mod transform;

use crate::mul::ntt::crt::U192;
pub use primes::{K, PRIMES};

/// Minimum smaller-operand length (in words) for the NTT path.
///
/// With `b_pack = 64` the crossover is at ~25 000 words (~1.6 M bits) on
/// Apple M4 Pro.  N-doubling at 32 769 / 65 537 words creates narrow
/// regression windows; radix-4 will shrink the step size further.
/// Chosen at 40 000 words where NTT is ≥18% faster.
pub const THRESHOLD_NTT: usize = 40_000;

/// Smallest admissible coefficient bit width (used for worst-case memory bound).
const B_PACK_MIN: u32 = 16;

/// Preferred coefficient bit width.  32 bits gives 2 coeffs/word and halves
/// the transform length vs. 16 bits, while staying comfortably within the
/// ~2^128 headroom of the two smallest primes.
/// Coefficient bit widths to try, in descending preference.
/// 64 uses K_eff = 3 primes; 32 and 16 use K_eff = 2.
const B_PACK_CANDIDATES: &[u32] = &[64, 32, 16];

/// Maximum `log2(transform length)`, set by `min(v2) = 32` across all primes.
const MAX_LOG_N: u32 = 32;

/// Select NTT parameters for operands with the given word lengths.
///
/// Returns `(b_pack, N, K_eff)`.
pub fn select_params(la_words: usize, lb_words: usize) -> (u32, usize, usize) {
    let word_bits = Word::BITS;
    let la_bits = la_words as u64 * word_bits as u64;
    let lb_bits = lb_words as u64 * word_bits as u64;
    let prod_2 = (PRIMES[0].p as u128) * (PRIMES[1].p as u128);

    for &b_pack in B_PACK_CANDIDATES {
        let coeffs_a = (la_bits + b_pack as u64 - 1) / b_pack as u64;
        let coeffs_b = (lb_bits + b_pack as u64 - 1) / b_pack as u64;
        let total_coeffs = (coeffs_a + coeffs_b - 1) as usize;
        let n = total_coeffs.next_power_of_two().max(2);

        if (n.trailing_zeros()) > MAX_LOG_N {
            continue;
        }

        // Compute max coefficient value, guarding against u128 overflow for
        // b_pack = 64 where (2^64−1)^2 ≈ 2^128 and n/2 can push it past 2^128.
        let coeff_max = (1u128 << b_pack) - 1;
        let max_coeff = coeff_max
            .checked_mul(coeff_max)
            .and_then(|sq| (n as u128 / 2).checked_mul(sq));

        let k_eff = match max_coeff {
            Some(mc) if mc < prod_2 => 2,
            // Overflow or exceeds two-prime product → need all three primes.
            // Three-prime product ≈ 2^192 ≫ any max_coeff we can encounter.
            _ => K,
        };
        return (b_pack, n, k_eff);
    }

    unreachable!("b_pack = 16 always passes the headroom check")
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

    let lanes_u64 = 2 * n_max; // a_lane + b_lane
    let residues_u64 = K * n_max; // per-prime inverse results
    let twiddles_u64 = n_max; // fwd + inv twiddle tables (n_max/2 each, reused)
    let product_u64 = total_len;

    let u64_bytes = 8usize;
    let word_bytes = core::mem::size_of::<Word>();
    let factor = (u64_bytes + word_bytes - 1) / word_bytes;
    let total_words = product_u64 + (lanes_u64 + residues_u64 + twiddles_u64) * factor;

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

    // Per-prime CRT reducers (no allocation)
    let r0 = FixedTrinomialSolinas64::<64, 32, 1>::new(&PRIMES[0].p);
    let r1 = FixedTrinomialSolinas64::<64, 34, 1>::new(&PRIMES[1].p);
    let r2 = FixedTrinomialSolinas64::<64, 40, 1>::new(&PRIMES[2].p);
    let crt_reducers: [&dyn ModOps; 3] = [&r0, &r1, &r2];

    // ---- Memory carve (longest-lived first) ----

    // 1. Product buffer
    let prod_len = la + lb;
    let (prod, mut mem) = memory.allocate_slice_fill::<u64>(prod_len, 0);

    // 2. Residue storage (per-prime inverse results)
    let residues_len = k_eff * nn;
    let (residues, mut mem) = mem.allocate_slice_fill::<u64>(residues_len, 0);

    // 3. Lane buffers (reused across primes)
    let (a_lane, mut mem) = mem.allocate_slice_fill::<u64>(nn, 0);
    let (b_lane, mut mem) = mem.allocate_slice_fill::<u64>(nn, 0);

    // 4. Twiddle tables (fwd + inv, reused per prime)
    let (fwd_twiddles, mut mem) = mem.allocate_slice_fill::<u64>(nn / 2, 0);
    let (inv_twiddles, _) = mem.allocate_slice_fill::<u64>(nn / 2, 0);

    // ---- Per-prime transforms (const-generic dispatch) ----
    for (pi, prime) in PRIMES[..k_eff].iter().enumerate() {
        let mut ctx = TransformCtx {
            a_lane,
            b_lane,
            fwd_twiddles,
            inv_twiddles,
            p: prime.p,
            omega_2_32: prime.omega_2_32,
            nn,
            b_pack,
            residues,
            pi,
        };
        match prime.b {
            32 => process_prime::<32>(a, b, &mut ctx),
            34 => process_prime::<34>(a, b, &mut ctx),
            40 => process_prime::<40>(a, b, &mut ctx),
            _ => unreachable!(),
        }
    }

    // ---- CRT per coefficient + accumulate ----
    let output_words = la + lb;
    for k in 0..output_coeffs {
        let mut coeff_residues = [0u64; 3];
        #[allow(clippy::needless_range_loop)]
        for pi in 0..k_eff {
            coeff_residues[pi] = residues[pi * nn + k];
        }
        let crt_val = crt::garner_combine(&coeff_residues[..k_eff], &crt_reducers[..k_eff]);
        add_shifted_to_prod(prod, &crt_val, k, b_pack);
    }

    // ---- Fold product into c with sign ----
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

/// Scratch buffers and parameters for one prime's NTT pipeline.
struct TransformCtx<'a> {
    a_lane: &'a mut [u64],
    b_lane: &'a mut [u64],
    fwd_twiddles: &'a mut [u64],
    inv_twiddles: &'a mut [u64],
    p: u64,
    omega_2_32: u64,
    nn: usize,
    b_pack: u32,
    residues: &'a mut [u64],
    pi: usize,
}

/// Per-prime NTT pipeline, monomorphized for a specific `B`.
#[inline(never)]
fn process_prime<const B: u32>(a: &[Word], b: &[Word], ctx: &mut TransformCtx<'_>) {
    pack::pack(ctx.a_lane, a, ctx.b_pack, ctx.nn);
    pack::pack(ctx.b_lane, b, ctx.b_pack, ctx.nn);

    // For b_pack = 64 coefficients may reach 2^64−1, which can exceed p.
    // Reduce each coefficient mod p (one conditional subtract suffices:
    // c < 2^64 < 2p for all three primes).
    if ctx.b_pack >= 64 {
        for c in ctx.a_lane[..ctx.nn].iter_mut() {
            if *c >= ctx.p {
                *c -= ctx.p;
            }
        }
        for c in ctx.b_lane[..ctx.nn].iter_mut() {
            if *c >= ctx.p {
                *c -= ctx.p;
            }
        }
    }

    transform::precompute_twiddles::<B>(ctx.fwd_twiddles, ctx.nn, ctx.p, ctx.omega_2_32, false);
    transform::precompute_twiddles::<B>(ctx.inv_twiddles, ctx.nn, ctx.p, ctx.omega_2_32, true);

    transform::bit_reverse(ctx.a_lane);
    transform::bit_reverse(ctx.b_lane);
    transform::forward::<B>(ctx.a_lane, ctx.fwd_twiddles, ctx.p);
    transform::forward::<B>(ctx.b_lane, ctx.fwd_twiddles, ctx.p);
    transform::pointwise_mul::<B>(ctx.a_lane, ctx.b_lane);
    transform::inverse::<B>(ctx.a_lane, ctx.inv_twiddles, ctx.p);

    let offset = ctx.pi * ctx.nn;
    ctx.residues[offset..offset + ctx.nn].copy_from_slice(ctx.a_lane);
}

/// Add a CRT value to `prod`, shifted left by `k * b_pack` bits.
fn add_shifted_to_prod(prod: &mut [u64], val: &U192, k: usize, b_pack: u32) {
    let count = val.len_words() as usize;
    let shift_bits = (k as u32).wrapping_mul(b_pack);
    let word_idx = (shift_bits / 64) as usize;
    let bit_shift = shift_bits % 64;

    let mut carry: u64 = 0;
    #[allow(clippy::needless_range_loop)]
    for vi in 0..count {
        let idx = word_idx + vi;
        if idx >= prod.len() {
            return;
        }
        let v = val.0[vi];
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

    let mut idx = word_idx + count;
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
        assert_eq!(b_pack, 64);
        assert!(n >= 2 && n.is_power_of_two());
        // b_pack = 64 needs K_eff = 3 primes.
        assert_eq!(k_eff, K);
    }

    #[test]
    fn test_select_params_large() {
        let (b_pack, n, k_eff) = select_params(THRESHOLD_NTT, THRESHOLD_NTT);
        assert_eq!(b_pack, 64);
        assert!(n.is_power_of_two());
        assert_eq!(k_eff, K);
        let coeffs_a = (THRESHOLD_NTT * Word::BITS as usize + 63) / 64;
        let coeffs_b = coeffs_a;
        let min_n = (coeffs_a + coeffs_b).next_power_of_two().max(2);
        assert!(n >= min_n, "n={n} < min_n={min_n}");
    }

    #[test]
    fn test_headroom_holds() {
        let la = THRESHOLD_NTT;
        let lb = THRESHOLD_NTT;
        let (b_pack, n, _k_eff) = select_params(la, lb);
        // For b_pack = 64 the product overflows u128; checked_mul in
        // select_params handles this and falls back to K_eff = 3.
        // Three-prime product ≈ 2^192 ≫ max_coeff for n ≤ 2^32.
        let coeff_max = (1u128 << b_pack) - 1;
        let overflow = coeff_max
            .checked_mul(coeff_max)
            .and_then(|sq| (n as u128 / 2).checked_mul(sq))
            .is_none();
        assert!(overflow || _k_eff == 2, "K_eff=2 only when max_coeff fits in u128");
        assert_eq!(b_pack, 64);
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

    const NTT_TEST_LEN: usize = 1024;

    #[test]
    fn test_ntt_multiply_small() {
        // Smoke test: NTT multiply with operands large enough to exercise
        // the full pipeline (pack, forward, pointwise, inverse, CRT, accumulate).
        let a: Vec<Word> = vec![0xDEADBEEFu64; NTT_TEST_LEN];
        let b: Vec<Word> = vec![0xCAFEBABEu64; NTT_TEST_LEN];
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
