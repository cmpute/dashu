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

pub(crate) mod crt;
mod pack;
mod transform;

use crate::arch::ntt::{
    B_PACK_CANDIDATES, B_PACK_MIN, CRT_INV_IJ, K, MAX_LOG_N, MODULI, OMEGA_MAX, P0, P1, P2,
};
use crate::mul::ntt::crt::{garner_combine, CrtAccum};
use num_modular::Reducer;

/// Minimum smaller-operand length (in words) for the NTT path.
///
/// Crossover with Toom-3 lies at ~3 200 words; chosen at 4 000 where
/// NTT is a clear 30%+ faster.
pub const THRESHOLD_NTT: usize = 4_000;

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
    let word_bytes = mem::size_of::<Word>();

    let lanes_words = lanes * lane_bytes / word_bytes;
    let residues_words = residues * lane_bytes / word_bytes;
    let twiddles_words = twiddles * lane_bytes / word_bytes;
    let total_words = product + lanes_words + residues_words + twiddles_words;

    memory::array_layout::<Word>(total_words)
}

/// `c += sign * a * b` with equal-length operands.
///
/// Returns carry.
#[must_use]
#[inline]
pub fn add_signed_mul_same_len(
    c: &mut [Word],
    sign: Sign,
    a: &[Word],
    b: &[Word],
    memory: &mut Memory,
) -> SignedWord {
    let n = a.len();
    debug_assert!(b.len() == n && c.len() == 2 * n);
    add_signed_mul_conv(c, sign, a, b, memory)
}

/// `c += sign * a * b` (general, a may be longer than b).
///
/// When `a ≫ b` the implementation forks:
/// - If `b` is below [`THRESHOLD_NTT`], dispatch already routes to
///   `toom_3::add_signed_mul` (which uses
///   `add_signed_mul_split_into_chunks` from
///   [`helpers`](crate::mul::helpers)).
/// - If `b` is above [`THRESHOLD_NTT`], this function pre-transforms
///   `b` once per prime and reuses `b̂` across chunks of `a` via
///   [`add_signed_mul_chunked`].
///
/// Returns carry.
#[must_use]
#[inline]
pub fn add_signed_mul(
    c: &mut [Word],
    sign: Sign,
    a: &[Word],
    b: &[Word],
    memory: &mut Memory,
) -> SignedWord {
    debug_assert!(a.len() >= b.len() && c.len() == a.len() + b.len());
    if a.len() > 2 * b.len() {
        return add_signed_mul_chunked(c, sign, a, b, memory);
    }
    add_signed_mul_conv(c, sign, a, b, memory)
}

/// `c += sign * a²` with a.len() == c.len()/2.
///
/// Returns carry.
#[must_use]
#[inline]
pub(crate) fn add_signed_sqr_same_len(
    c: &mut [Word],
    sign: Sign,
    a: &[Word],
    memory: &mut Memory,
) -> SignedWord {
    let n = a.len();
    debug_assert!(c.len() == 2 * n);
    add_signed_sqr_conv(c, sign, a, memory)
}

/// Core NTT squaring: transform `a` once, pointwise-square, inverse.
fn add_signed_sqr_conv(
    c: &mut [Word],
    sign: Sign,
    a: &[Word],
    memory: &mut Memory,
) -> SignedWord {
    use crate::arch::ntt::Lane;

    let la = a.len();
    debug_assert!(la > 0);
    let (b_pack, nn, k_eff) = select_params(la, la);
    let la_bits = bit_len(a);
    debug_assert!(la_bits > 0);

    let coeffs_a = coeff_count(la_bits, b_pack);
    let output_coeffs = 2 * coeffs_a - 1;
    let out_words = 2 * la;

    let geom = NttGeometry {
        nn,
        b_pack,
        k_eff,
        output_coeffs,
    };

    // Allocate: product, residues, a_lane, twiddle buffers.
    let (prod, mut m) = memory.allocate_slice_fill::<Word>(out_words, 0);
    let (residues, mut m) = m.allocate_slice_fill::<Lane>(k_eff * nn, 0);
    let (a_lane, mut m) = m.allocate_slice_fill::<Lane>(nn, 0);
    let (fwd_twiddles, mut m) = m.allocate_slice_fill::<Lane>(nn / 2, 0);
    let (inv_twiddles, _) = m.allocate_slice_fill::<Lane>(nn / 2, 0);

    for (pi, &omega) in OMEGA_MAX.iter().enumerate().take(k_eff) {
        match pi {
            0 => {
                transform::precompute_twiddles(fwd_twiddles, nn, omega, false, &P0);
                transform::precompute_twiddles(inv_twiddles, nn, omega, true, &P0);
                process_prime_square(a, a_lane, fwd_twiddles, inv_twiddles, residues, pi, &geom, &P0);
            }
            1 => {
                transform::precompute_twiddles(fwd_twiddles, nn, omega, false, &P1);
                transform::precompute_twiddles(inv_twiddles, nn, omega, true, &P1);
                process_prime_square(a, a_lane, fwd_twiddles, inv_twiddles, residues, pi, &geom, &P1);
            }
            2 => {
                transform::precompute_twiddles(fwd_twiddles, nn, omega, false, &P2);
                transform::precompute_twiddles(inv_twiddles, nn, omega, true, &P2);
                process_prime_square(a, a_lane, fwd_twiddles, inv_twiddles, residues, pi, &geom, &P2);
            }
            _ => unreachable!(),
        }
    }

    do_crt::<crate::arch::word::TripleWord>(prod, residues, &geom, &MODULI, &CRT_INV_IJ);

    match sign {
        Positive => add::add_signed_in_place(&mut c[..out_words], Positive, &prod[..out_words]),
        Negative => add::add_signed_in_place(&mut c[..out_words], Negative, &prod[..out_words]),
    }
}

/// Per-prime NTT squaring pipeline.
///
/// Packs + Montgomery converts + forward transforms `a`, pointwise
/// squares, inverse transforms, and stores the residues.
#[allow(clippy::too_many_arguments)]
fn process_prime_square<R: Reducer<crate::arch::ntt::Lane>>(
    a: &[Word],
    a_lane: &mut [crate::arch::ntt::Lane],
    fwd_twiddles: &[crate::arch::ntt::Lane],
    inv_twiddles: &[crate::arch::ntt::Lane],
    residues: &mut [crate::arch::ntt::Lane],
    pi: usize,
    geom: &NttGeometry,
    r: &R,
) {
    let nn = geom.nn;

    // Transform a
    pack::pack(a_lane, a, geom.b_pack, nn);
    for c in a_lane[..nn].iter_mut() {
        *c = r.transform(*c);
    }
    transform::bit_reverse(&mut a_lane[..nn]);
    transform::forward(&mut a_lane[..nn], fwd_twiddles, r);

    // Pointwise square (no separate b̂ needed)
    transform::pointwise_square(&mut a_lane[..nn], r);

    // Inverse transform
    transform::inverse(&mut a_lane[..nn], inv_twiddles, r);
    for c in a_lane[..nn].iter_mut() {
        *c = r.residue(*c);
    }

    let offset = pi * nn;
    residues[offset..offset + nn].copy_from_slice(&a_lane[..nn]);
}

/// NTT multiplication with asymmetric chunking.
///
/// When `la > 2 * lb`, transform `b` once and reuse `b̂` across chunks
/// of `a`, reducing total transform work from O((la+lb)·log(la+lb))
/// to O(la + lb·log(lb)).
fn add_signed_mul_chunked(
    c: &mut [Word],
    sign: Sign,
    a: &[Word],
    b: &[Word],
    memory: &mut Memory,
) -> SignedWord {
    use crate::arch::ntt::Lane;
    use crate::mul::helpers::add_signed_mul_split_into_chunks;

    let lb = b.len();
    let chunk_len = lb * 2;

    // Parameters for chunk-sized transforms.
    let (b_pack, nn_chunk, k_eff) = select_params(chunk_len, lb); // a_chunk ≈ 2*lb

    // ---- Allocate long-lived buffers ----

    // Per-prime forward-transformed b̂ and cached twiddles (fwd + inv).
    // Twiddles depend only on (pi, nn_chunk, omega_max) — precompute
    // once so the per-chunk closure can copy instead of recomputing.
    let b_hat_len = k_eff * nn_chunk;
    let twiddle_len = k_eff * (nn_chunk / 2);
    let (b_hat, mut mem) = memory.allocate_slice_fill::<Lane>(b_hat_len, 0);
    let (fwd_tw_cache, mut mem) = mem.allocate_slice_fill::<Lane>(twiddle_len, 0);
    let (inv_tw_cache, mut mem) = mem.allocate_slice_fill::<Lane>(twiddle_len, 0);

    // ---- Transform b once per prime; also precompute twiddles ----
    let geom = NttGeometry {
        nn: nn_chunk,
        b_pack,
        k_eff,
        output_coeffs: 0, // unused by prepare_b_hat_and_twiddles
    };
    prepare_b_hat_and_twiddles(b_hat, fwd_tw_cache, inv_tw_cache, b, &geom, &mut mem);

    // ---- Setup for the closure ----
    let lb_bits = bit_len(b);
    let coeffs_b = coeff_count(lb_bits, b_pack);

    // ---- Chunked multiply ----
    add_signed_mul_split_into_chunks(
        c,
        sign,
        a,
        b,
        chunk_len,
        &mut mem,
        |c_slice, sign, a_chunk, b, mem| {
            let a_bits = bit_len(a_chunk);
            if a_bits == 0 {
                return 0;
            }
            let coeffs_a = coeff_count(a_bits, b_pack);
            let output_coeffs = coeffs_a + coeffs_b - 1;
            let out_words = a_chunk.len() + b.len();

            let geom = NttGeometry {
                nn: nn_chunk,
                b_pack,
                k_eff,
                output_coeffs,
            };
            run_ntt_pipeline(
                a_chunk,
                b_hat,
                fwd_tw_cache,
                inv_tw_cache,
                &geom,
                out_words,
                c_slice,
                sign,
                mem,
            )
        },
    )
}

/// Run the full NTT pipeline: allocate → per-prime transform → CRT → fold into `c_out`.
///
/// `b_hat`, `fwd_tw_cache`, and `inv_tw_cache` must have been precomputed by the
/// caller (pack + Montgomery convert + bit-reverse + forward-transform for `b_hat`;
/// forward/inverse twiddle tables for the caches).  See `transform_b_forward` and
/// `transform::precompute_twiddles`.
///
/// Shared body of `add_signed_mul_conv` and the per-chunk callback in
/// `add_signed_mul_chunked`.
#[allow(clippy::too_many_arguments)]
fn run_ntt_pipeline(
    a: &[Word],
    b_hat: &[crate::arch::ntt::Lane],
    fwd_tw_cache: &[crate::arch::ntt::Lane],
    inv_tw_cache: &[crate::arch::ntt::Lane],
    geom: &NttGeometry,
    out_words: usize,
    c_out: &mut [Word],
    sign: Sign,
    mem: &mut Memory,
) -> SignedWord {
    use crate::arch::ntt::Lane;

    let nn = geom.nn;
    let k_eff = geom.k_eff;

    let (prod, mut m) = mem.allocate_slice_fill::<Word>(out_words, 0);
    let (residues, mut m) = m.allocate_slice_fill::<Lane>(k_eff * nn, 0);
    let (a_lane, mut m) = m.allocate_slice_fill::<Lane>(nn, 0);
    let (b_lane, mut m) = m.allocate_slice_fill::<Lane>(nn, 0);
    let (fwd_twiddles, mut m) = m.allocate_slice_fill::<Lane>(nn / 2, 0);
    let (inv_twiddles, _) = m.allocate_slice_fill::<Lane>(nn / 2, 0);

    let mut ctx = TransformCtx {
        a_lane,
        b_lane,
        fwd_twiddles,
        inv_twiddles,
        geom: NttGeometry { ..*geom },
    };

    for pi in 0..k_eff {
        let tw_off = pi * (nn / 2);
        ctx.fwd_twiddles
            .copy_from_slice(&fwd_tw_cache[tw_off..tw_off + nn / 2]);
        ctx.inv_twiddles
            .copy_from_slice(&inv_tw_cache[tw_off..tw_off + nn / 2]);

        let b_hat_slice = &b_hat[pi * nn..(pi + 1) * nn];

        match pi {
            0 => process_prime(a, b_hat_slice, &mut ctx, residues, pi, &P0),
            1 => process_prime(a, b_hat_slice, &mut ctx, residues, pi, &P1),
            2 => process_prime(a, b_hat_slice, &mut ctx, residues, pi, &P2),
            _ => unreachable!(),
        }
    }

    do_crt::<crate::arch::word::TripleWord>(prod, residues, &ctx.geom, &MODULI, &CRT_INV_IJ);

    match sign {
        Positive => add::add_signed_in_place(&mut c_out[..out_words], Positive, &prod[..out_words]),
        Negative => add::add_signed_in_place(&mut c_out[..out_words], Negative, &prod[..out_words]),
    }
}

/// Core implementation: c += sign * a * b.
///
/// Does a single NTT convolution of the full operands (no chunking).
fn add_signed_mul_conv(
    c: &mut [Word],
    sign: Sign,
    a: &[Word],
    b: &[Word],
    memory: &mut Memory,
) -> SignedWord {
    use crate::arch::ntt::Lane;

    let la = a.len();
    let lb = b.len();

    debug_assert!(la > 0 && lb > 0);
    let (b_pack, nn, k_eff) = select_params(la, lb);
    let la_bits = bit_len(a);
    let lb_bits = bit_len(b);
    debug_assert!(la_bits > 0 && lb_bits > 0);

    let coeffs_a = coeff_count(la_bits, b_pack);
    let coeffs_b = coeff_count(lb_bits, b_pack);
    let output_coeffs = coeffs_a + coeffs_b - 1;

    // Pre-transform b and precompute twiddles.
    let b_hat_len = k_eff * nn;
    let twiddle_len = k_eff * (nn / 2);
    let (b_hat, mut mem) = memory.allocate_slice_fill::<Lane>(b_hat_len, 0);
    let (fwd_tw_cache, mut mem) = mem.allocate_slice_fill::<Lane>(twiddle_len, 0);
    let (inv_tw_cache, mut mem) = mem.allocate_slice_fill::<Lane>(twiddle_len, 0);

    let geom = NttGeometry {
        nn,
        b_pack,
        k_eff,
        output_coeffs,
    };
    prepare_b_hat_and_twiddles(b_hat, fwd_tw_cache, inv_tw_cache, b, &geom, &mut mem);
    run_ntt_pipeline(a, b_hat, fwd_tw_cache, inv_tw_cache, &geom, la + lb, c, sign, &mut mem)
}

/// CRT + accumulate, generic over the accumulator type.
fn do_crt<A: CrtAccum>(
    prod: &mut [Word],
    residues: &[A::Lane],
    geom: &NttGeometry,
    primes: &[A::Lane; K],
    crt_inv: &[[A::Lane; K]; K],
) {
    let g = geom;
    for k in 0..g.output_coeffs {
        let mut coeff_residues = [A::Lane::default(); 3];
        #[allow(clippy::needless_range_loop)]
        for pi in 0..g.k_eff {
            coeff_residues[pi] = residues[pi * g.nn + k];
        }
        let crt_val = garner_combine::<A>(&coeff_residues[..g.k_eff], crt_inv, primes);
        let mut crt_buf = [Word::default(); 6];
        let crt_n = crt_val.write_words(&mut crt_buf);
        add_shifted_to_prod(prod, &crt_buf[..crt_n as usize], crt_n, k, g.b_pack);
    }
}

/// Geometry constants for an NTT pipeline invocation.
struct NttGeometry {
    nn: usize,
    b_pack: u32,
    k_eff: usize,
    output_coeffs: usize,
}

/// Scratch buffers and geometry for the per-prime NTT pipeline.
struct TransformCtx<'a> {
    a_lane: &'a mut [crate::arch::ntt::Lane],
    b_lane: &'a mut [crate::arch::ntt::Lane],
    fwd_twiddles: &'a mut [crate::arch::ntt::Lane],
    inv_twiddles: &'a mut [crate::arch::ntt::Lane],
    geom: NttGeometry,
}

/// Transform `b` and leave the result in `b_lane` (forward-transformed,
/// Montgomery form).  `fwd_twiddles` must already be precomputed.
fn transform_b_forward<R: Reducer<crate::arch::ntt::Lane>>(
    b_lane: &mut [crate::arch::ntt::Lane],
    b: &[Word],
    nn: usize,
    b_pack: u32,
    fwd_twiddles: &[crate::arch::ntt::Lane],
    r: &R,
) {
    pack::pack(b_lane, b, b_pack, nn);
    for c in b_lane[..nn].iter_mut() {
        *c = r.transform(*c);
    }
    transform::bit_reverse(&mut b_lane[..nn]);
    transform::forward(&mut b_lane[..nn], fwd_twiddles, r);
}

/// Pre-transform `b` and precompute twiddles, storing results into the
/// pre-allocated cache slices.
///
/// `b_hat` must have length `geom.k_eff * geom.nn`, `fwd_tw_cache` and
/// `inv_tw_cache` each `geom.k_eff * (geom.nn / 2)`.
fn prepare_b_hat_and_twiddles(
    b_hat: &mut [crate::arch::ntt::Lane],
    fwd_tw_cache: &mut [crate::arch::ntt::Lane],
    inv_tw_cache: &mut [crate::arch::ntt::Lane],
    b: &[Word],
    geom: &NttGeometry,
    mem: &mut Memory,
) {
    use crate::arch::ntt::Lane;

    let nn = geom.nn;
    let b_pack = geom.b_pack;
    let k_eff = geom.k_eff;

    for (pi, &omega) in OMEGA_MAX.iter().enumerate().take(k_eff) {
        let (b_lane, mut rest) = mem.allocate_slice_fill::<Lane>(nn, 0);
        let (fwd_tw, mut rest) = rest.allocate_slice_fill::<Lane>(nn / 2, 0);
        let (inv_tw, _) = rest.allocate_slice_fill::<Lane>(nn / 2, 0);

        match pi {
            0 => {
                transform::precompute_twiddles(fwd_tw, nn, omega, false, &P0);
                transform::precompute_twiddles(inv_tw, nn, omega, true, &P0);
                transform_b_forward(b_lane, b, nn, b_pack, fwd_tw, &P0);
            }
            1 => {
                transform::precompute_twiddles(fwd_tw, nn, omega, false, &P1);
                transform::precompute_twiddles(inv_tw, nn, omega, true, &P1);
                transform_b_forward(b_lane, b, nn, b_pack, fwd_tw, &P1);
            }
            2 => {
                transform::precompute_twiddles(fwd_tw, nn, omega, false, &P2);
                transform::precompute_twiddles(inv_tw, nn, omega, true, &P2);
                transform_b_forward(b_lane, b, nn, b_pack, fwd_tw, &P2);
            }
            _ => unreachable!(),
        }

        let b_off = pi * nn;
        let tw_off = pi * (nn / 2);
        b_hat[b_off..b_off + nn].copy_from_slice(b_lane);
        fwd_tw_cache[tw_off..tw_off + nn / 2].copy_from_slice(fwd_tw);
        inv_tw_cache[tw_off..tw_off + nn / 2].copy_from_slice(inv_tw);
    }
}

/// Per-prime NTT pipeline.
///
/// `b_hat_slice` must already be forward-transformed (packed, Montgomery
/// form, bit-reversed).  `ctx.fwd_twiddles` and `ctx.inv_twiddles` must
/// already be precomputed.
fn process_prime<R: Reducer<crate::arch::ntt::Lane>>(
    a: &[Word],
    b_hat_slice: &[crate::arch::ntt::Lane],
    ctx: &mut TransformCtx<'_>,
    residues: &mut [crate::arch::ntt::Lane],
    pi: usize,
    r: &R,
) {
    let nn = ctx.geom.nn;
    let b_pack = ctx.geom.b_pack;

    // Transform a
    pack::pack(ctx.a_lane, a, b_pack, nn);
    for c in ctx.a_lane[..nn].iter_mut() {
        *c = r.transform(*c);
    }
    transform::bit_reverse(&mut ctx.a_lane[..nn]);
    transform::forward(&mut ctx.a_lane[..nn], ctx.fwd_twiddles, r);

    // Copy pre-transformed b
    ctx.b_lane[..nn].copy_from_slice(b_hat_slice);

    transform::pointwise_mul(&mut ctx.a_lane[..nn], &ctx.b_lane[..nn], r);
    transform::inverse(&mut ctx.a_lane[..nn], ctx.inv_twiddles, r);
    for c in ctx.a_lane[..nn].iter_mut() {
        *c = r.residue(*c);
    }

    let offset = pi * nn;
    residues[offset..offset + nn].copy_from_slice(&ctx.a_lane[..nn]);
}

/// Add a CRT value (as `Word`-sized limbs) to `prod`, shifted left by
/// `k * b_pack` bits.
fn add_shifted_to_prod(prod: &mut [Word], words: &[Word], count: u32, k: usize, b_pack: u32) {
    let shift_bits = (k as u32).wrapping_mul(b_pack);
    let word_bits = Word::BITS;
    let start_idx = (shift_bits / word_bits) as usize;
    let bit_shift = shift_bits % word_bits;

    let mut carry: Word = 0;

    for (vi, &word) in words.iter().enumerate().take(count as usize) {
        let limb = word.wrapping_add(carry);
        let idx = start_idx + vi;
        if idx >= prod.len() {
            return;
        }

        if bit_shift == 0 {
            let (r, c) = prod[idx].overflowing_add(limb);
            prod[idx] = r;
            carry = Word::from(c);
        } else {
            let val = (limb as u128) << bit_shift;
            let lo = val as Word;
            let hi = (val >> word_bits) as Word;

            let (r, c1) = prod[idx].overflowing_add(lo);
            prod[idx] = r;
            carry = Word::from(c1).wrapping_add(hi);

            if idx + 1 < prod.len() && carry != 0 {
                let (r2, c2) = prod[idx + 1].overflowing_add(carry);
                prod[idx + 1] = r2;
                carry = Word::from(c2);
            }
        }
    }

    let mut idx = start_idx + count as usize;
    while carry != 0 && idx < prod.len() {
        let (r, c) = prod[idx].overflowing_add(carry);
        prod[idx] = r;
        carry = Word::from(c);
        idx += 1;
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
    fn test_ntt_sign_negative() {
        let a: Vec<Word> = (0..30).map(|i| (i as Word + 1) * 100).collect();
        let b: Vec<Word> = (0..30).map(|i| (i as Word + 1) * 200).collect();

        let mut c = vec![0u64 as Word; a.len() + b.len()];
        let layout = memory_requirement_up_to(c.len(), b.len());
        let mut alloc = crate::memory::MemoryAllocation::new(layout);
        let mut memory = alloc.memory();
        let _ = add_signed_mul_conv(&mut c, Positive, &a, &b, &mut memory);

        let layout2 = memory_requirement_up_to(c.len(), b.len());
        let mut alloc2 = crate::memory::MemoryAllocation::new(layout2);
        let mut memory2 = alloc2.memory();
        let _ = add_signed_mul_conv(&mut c, Negative, &a, &b, &mut memory2);

        assert!(c.iter().all(|&w| w == 0));
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
        let carry = add_signed_mul_conv(&mut c, Positive, &a, &b, &mut memory);
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
            add_signed_mul_conv(&mut c, Positive, &a, &b, &mut memory);
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
        add_signed_mul_conv(&mut c, Positive, &a, &b, &mut memory);
        assert_eq!(&c[..], &expected[..], "sparse operand mismatch");
    }
}
