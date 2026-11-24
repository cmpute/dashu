//! Multiplication.

use crate::{
    arch::word::{DoubleWord, SignedWord, Word},
    helper_macros::debug_assert_zero,
    math,
    memory::{self, Memory},
    primitive::{double_word, split_dword},
    Sign,
};
use alloc::alloc::Layout;
use core::mem;
use static_assertions::const_assert;

/// If smaller operand length <= this, simple multiplication will be used.
const THRESHOLD_SIMPLE_DEFAULT: usize = 24;
const_assert!(THRESHOLD_SIMPLE_DEFAULT <= simple::MAX_SMALLER_LEN);
const_assert!(THRESHOLD_SIMPLE_DEFAULT + 1 >= karatsuba::MIN_LEN);

/// If smaller operand length <= this, Karatsuba multiplication will be used.
/// Tuned so that Toom-3 kicks in earlier (~96 words vs the old 192),
/// closing the gap with malachite/rug at ~10000-bit sizes.
const THRESHOLD_KARATSUBA_DEFAULT: usize = 96;
const_assert!(THRESHOLD_KARATSUBA_DEFAULT + 1 >= toom_3::MIN_LEN);

/// Smaller operand length at or above which moderately unbalanced products
/// (1.5:1 up to 2.5:1) use the Toom-4x2 kernel.
const THRESHOLD_TOOM42_MIN_DEFAULT: usize = 96;
const_assert!(THRESHOLD_TOOM42_MIN_DEFAULT >= toom_4_2::MIN_LEN);

/// Smaller operand length at or above which heavily unbalanced products use
/// the chunked NTT path (smaller operand transformed once) even below the
/// NTT threshold.
#[cfg(not(any(force_bits = "16", target_pointer_width = "16")))]
const THRESHOLD_NTT_ASYM_MIN_DEFAULT: usize = 2500;
/// Minimum ratio between the operand lengths for the chunked NTT path. At
/// least three full 2*b chunks are needed to amortize the b-hat transform;
/// below that the chunked Toom-3/Toom-4 path measured faster head-to-head.
#[cfg(not(any(force_bits = "16", target_pointer_width = "16")))]
const THRESHOLD_NTT_ASYM_RATIO_DEFAULT: usize = 6;

/// If smaller operand length <= this, Toom-3 multiplication will be used.
const THRESHOLD_TOOM4_MUL_DEFAULT: usize = 1000;
const_assert!(THRESHOLD_TOOM4_MUL_DEFAULT + 1 >= toom_4::MIN_LEN);
const_assert!(THRESHOLD_TOOM4_MUL_DEFAULT > THRESHOLD_KARATSUBA_DEFAULT);

/// If smaller operand length > this, NTT multiplication will be used.
#[cfg(not(any(force_bits = "16", target_pointer_width = "16")))]
const THRESHOLD_NTT_DEFAULT: usize = ntt::THRESHOLD_NTT;
/// NTT unavailable on 16/32-bit word targets — use `usize::MAX` so dispatch never
/// routes to the NTT path.
#[cfg(any(force_bits = "16", target_pointer_width = "16"))]
const THRESHOLD_NTT_DEFAULT: usize = usize::MAX;
#[cfg(not(any(force_bits = "16", target_pointer_width = "16")))]
const_assert!(THRESHOLD_NTT_DEFAULT > THRESHOLD_TOOM4_MUL_DEFAULT);

/// Environment-variable overrides for multiplication thresholds.
///
/// When the `tuning` feature is active the user may set `DASHU_THRESHOLD_SIMPLE_MUL`,
/// `DASHU_THRESHOLD_KARATSUBA_MUL`, `DASHU_THRESHOLD_TOOM42_MIN`,
/// `DASHU_THRESHOLD_TOOM4_MUL` or `DASHU_THRESHOLD_NTT_MUL` to override the
/// compile-time defaults.
mod threshold {
    #[inline]
    pub fn simple() -> usize {
        #[cfg(feature = "tuning")]
        {
            if let Ok(s) = std::env::var("DASHU_THRESHOLD_SIMPLE_MUL") {
                if let Ok(v) = s.parse() {
                    return v;
                }
            }
        }
        super::THRESHOLD_SIMPLE_DEFAULT
    }
    #[inline]
    pub fn karatsuba() -> usize {
        #[cfg(feature = "tuning")]
        {
            if let Ok(s) = std::env::var("DASHU_THRESHOLD_KARATSUBA_MUL") {
                if let Ok(v) = s.parse() {
                    return v;
                }
            }
        }
        super::THRESHOLD_KARATSUBA_DEFAULT
    }
    #[inline]
    pub fn toom42_min() -> usize {
        #[cfg(feature = "tuning")]
        {
            if let Ok(s) = std::env::var("DASHU_THRESHOLD_TOOM42_MIN") {
                if let Ok(v) = s.parse() {
                    return v;
                }
            }
        }
        super::THRESHOLD_TOOM42_MIN_DEFAULT
    }
    #[cfg(not(any(force_bits = "16", target_pointer_width = "16")))]
    #[inline]
    pub fn ntt_asym_min() -> usize {
        #[cfg(feature = "tuning")]
        {
            if let Ok(s) = std::env::var("DASHU_THRESHOLD_NTT_ASYM_MIN") {
                if let Ok(v) = s.parse() {
                    return v;
                }
            }
        }
        super::THRESHOLD_NTT_ASYM_MIN_DEFAULT
    }
    #[cfg(not(any(force_bits = "16", target_pointer_width = "16")))]
    #[inline]
    pub fn ntt_asym_ratio() -> usize {
        #[cfg(feature = "tuning")]
        {
            if let Ok(s) = std::env::var("DASHU_THRESHOLD_NTT_ASYM_RATIO") {
                if let Ok(v) = s.parse() {
                    return v;
                }
            }
        }
        super::THRESHOLD_NTT_ASYM_RATIO_DEFAULT
    }
    #[inline]
    pub fn toom4() -> usize {
        #[cfg(feature = "tuning")]
        {
            if let Ok(s) = std::env::var("DASHU_THRESHOLD_TOOM4_MUL") {
                if let Ok(v) = s.parse() {
                    return v;
                }
            }
        }
        super::THRESHOLD_TOOM4_MUL_DEFAULT
    }
    #[inline]
    pub fn ntt() -> usize {
        #[cfg(feature = "tuning")]
        {
            if let Ok(s) = std::env::var("DASHU_THRESHOLD_NTT_MUL") {
                if let Ok(v) = s.parse() {
                    return v;
                }
            }
        }
        super::THRESHOLD_NTT_DEFAULT
    }
}

mod helpers;
mod karatsuba;
#[cfg(not(any(force_bits = "16", target_pointer_width = "16")))]
pub(crate) mod ntt;
mod simple;
pub(crate) mod toom_3;
mod toom_4;
mod toom_4_2;

pub use simple::{
    add_mul_dword_same_len_in_place, add_mul_word_in_place, add_mul_word_same_len_in_place,
    sub_mul_dword_same_len_in_place, sub_mul_word_same_len_in_place,
};

/// Multiply a word sequence by a `Word` in place.
///
/// Returns carry.
#[must_use]
#[inline]
pub fn mul_word_in_place(words: &mut [Word], rhs: Word) -> Word {
    mul_word_in_place_with_carry(words, rhs, 0)
}

/// Multiply a word sequence by a `DoubleWord` in place.
///
/// Returns carry as a double word.
#[must_use]
pub fn mul_dword_in_place(words: &mut [Word], rhs: DoubleWord) -> DoubleWord {
    debug_assert!(rhs > Word::MAX as DoubleWord, "call mul_word_in_place when rhs is small");

    // chunk the words into double words, and do 2by2 multiplications
    let mut dwords = words.chunks_exact_mut(2);
    let mut carry = 0;
    for chunk in &mut dwords {
        let lo = chunk.first().unwrap();
        let hi = chunk.last().unwrap();
        let (p, new_carry) = math::mul_add_carry_dword(double_word(*lo, *hi), rhs, carry);
        let (new_lo, new_hi) = split_dword(p);
        *chunk.first_mut().unwrap() = new_lo;
        *chunk.last_mut().unwrap() = new_hi;
        carry = new_carry;
    }

    // there might be a single word left, do two 1by1 multiplications
    let r = dwords.into_remainder();
    if !r.is_empty() {
        debug_assert!(r.len() == 1);
        let r0 = r.first_mut().unwrap();
        let (m_lo, m_hi) = split_dword(rhs);
        let (c_lo, c_hi) = split_dword(carry);
        let (n_lo, nc_lo) = math::mul_add_carry(*r0, m_lo, c_lo);
        let (n_hi, nc_hi) = math::mul_add_2carry(*r0, m_hi, nc_lo, c_hi);
        *r0 = n_lo;
        carry = double_word(n_hi, nc_hi);
    }
    carry
}

/// Multiply a word sequence by a `Word` in place with carry in.
///
/// Returns carry.
#[must_use]
pub fn mul_word_in_place_with_carry(words: &mut [Word], rhs: Word, mut carry: Word) -> Word {
    if rhs == 0 {
        return 0;
    }

    for a in words {
        let (v_lo, v_hi) = math::mul_add_carry(*a, rhs, carry);
        *a = v_lo;
        carry = v_hi;
    }
    carry
}

/// Temporary scratch space required for multiplication.
///
/// Despite the name, this is only valid for a product with exactly these lengths: the
/// budget is not monotone in either of them. See [`memory_requirement_up_to_operand_len`]
/// and [`memory_chain_budget_envelope`] for bounds over a range of shapes.
pub fn memory_requirement_up_to(total_len: usize, smaller_len: usize) -> Layout {
    memory_chain_budget(total_len - smaller_len, smaller_len)
}

/// Memory budget for `c += a * b` (a >= b), faithfully modeling the
/// dispatcher's behavior including the chunk-loop tails: every chunked
/// wrapper (karatsuba, toom-3, toom-4, chunked NTT) processes full chunks
/// through its own kernel and re-dispatches the tail through
/// [`add_signed_mul`], which can reach a different algorithm with a larger
/// scratch appetite than the top-level claim (e.g. a Toom-3 chunk tail
/// landing in the Toom-4x2 band). The chain's second argument strictly
/// decreases at every step, so the recursion terminates.
fn memory_chain_budget(a: usize, b: usize) -> Layout {
    debug_assert!(a >= b);
    #[cfg(not(any(force_bits = "16", target_pointer_width = "16")))]
    if b >= threshold::ntt_asym_min() && a >= threshold::ntt_asym_ratio() * b {
        // The chunked NTT budget is expressed over the full product and
        // covers its internal chunk pipelines and their tails.
        return ntt::memory_requirement_up_to(a + b, b);
    }
    #[cfg(any(force_bits = "16", target_pointer_width = "16"))]
    let _ = (a, b);
    if b >= threshold::toom42_min() && b <= threshold::ntt() && toom_4_2::in_band(a, b) {
        return toom_4_2::memory_chain_budget(a, b);
    }
    let ladder = if b <= threshold::simple() {
        // The schoolbook kernel consumes the whole operand in one pass,
        // there is no chunk loop and no tail re-dispatch.
        return memory::zero_layout();
    } else if b <= threshold::karatsuba() {
        karatsuba::memory_requirement_up_to(b)
    } else if b <= threshold::toom4() {
        toom_3::memory_requirement_up_to(b)
    } else if b <= threshold::ntt() {
        toom_4::memory_requirement_up_to(b)
    } else {
        // NTT path — only available on 64-bit word targets; its budget
        // covers the chunk pipelines and their tails.
        #[cfg(not(any(force_bits = "16", target_pointer_width = "16")))]
        {
            ntt::memory_requirement_up_to(a + b, b)
        }
        #[cfg(any(force_bits = "16", target_pointer_width = "16"))]
        {
            unreachable!("NTT unavailable on 16-bit targets");
        }
    };
    // The chunked NTT keeps its pipeline (transformed b-hat, twiddles)
    // alive while the tail re-dispatch runs, so the tail budget adds up.
    #[cfg(not(any(force_bits = "16", target_pointer_width = "16")))]
    if b > threshold::ntt() {
        return chunk_tail_budget(a, b, ladder);
    }
    // The other chunked wrappers (karatsuba, toom-3, toom-4) run every
    // chunk and the tail re-dispatch sequentially over the same scratch,
    // so the tail needs max(own, tail), not the sum.
    chunk_tail_budget_max(a, b, ladder)
}

/// Maximum of three layouts (word counts).
pub(crate) fn max_layout3(l0: Layout, l1: Layout, l2: Layout) -> Layout {
    memory::max_layout(memory::max_layout(l0, l1), l2)
}

/// Budget of the balanced kernels (Karatsuba, Toom-3, Toom-4) for a smaller operand of `b`
/// words, without the tail of the chunk loop. It is monotone in `b`.
fn ladder_budget(b: usize) -> Layout {
    if b <= threshold::simple() {
        memory::zero_layout()
    } else if b <= threshold::karatsuba() {
        karatsuba::memory_requirement_up_to(b)
    } else if b <= threshold::toom4() {
        toom_3::memory_requirement_up_to(b)
    } else {
        toom_4::memory_requirement_up_to(b)
    }
}

/// Bound on the Toom-4x2 budget ([`toom_4_2::memory_chain_budget`]) of every in-band shape
/// with total length `<= total`. It is monotone in `total`.
///
/// An in-band shape `(a, b)` with `a + b = t` splits at `n <= t/5 + 1` (either
/// `n = ceil(a/4)` with `a < 5t/7`, or `n = ceil(b/2)` with `b < 2t/5`). Its level costs
/// `18n + 64` words, and its children are the balanced products `(n, n)` and `(n+2, n+1)`,
/// which only pay the ladder, and a product of total length `<= 2n` that can be of any shape
/// (possibly in band again).
fn band_budget_envelope(total: usize) -> Layout {
    // No shape this short is in the band (the smaller operand has at least `toom42_min` words).
    if total < 2 * threshold::toom42_min().max(toom_4_2::MIN_LEN) {
        return memory::zero_layout();
    }
    let n = total / 5 + 1;
    memory::add_layout(
        memory::array_layout::<Word>(18 * n + 64),
        memory::max_layout(ladder_budget(n + 1), band_budget_envelope(2 * n)),
    )
}

/// Scratch budget dominating [`memory_chain_budget`] for every product
/// shape with total length `<= total_cap` and smaller side `<= smaller_cap`.
/// It is monotone in both arguments.
///
/// The per-shape budget is *not* monotone in the smaller side or in the total: splits whose
/// ratio falls in the Toom-4x2 band (about 1.5:1 to 2.5:1) pay the Toom-4x2
/// appetite (roughly `11*b` words) while a slightly more balanced split of
/// the same total pays only the Toom-3 ladder (roughly `4*b`), and inside
/// the band the child products can re-enter the band, so sampling one split
/// does not bound the neighbors. Callers that cannot predict the exact
/// operand lengths of their products (the division and GCD recursions, the modular
/// multiplications of residues with leading zero words) must use this envelope instead.
///
/// The bound is the maximum over the kinds of shape within the caps:
/// * ladder shapes pay the (monotone) ladder budget of their smaller side, or the budget of
///   their chunk-loop tail `(b, a % b)`, which is again a shape within the caps;
/// * in-band shapes with a smaller side `b` have a total below `3.5 * b`, and are bounded by
///   [`band_budget_envelope`];
/// * transform-based shapes pay a budget that is monotone in their total, plus (when the
///   long operand is processed in chunks, i.e. it is at least 3 times the short one) the
///   budget of the tail `(b, a % b)`, whose total is below `2 * b <= total / 2`.
pub(crate) fn memory_chain_budget_envelope(total_cap: usize, smaller_cap: usize) -> Layout {
    let cap = smaller_cap.min(total_cap / 2);
    if cap == 0 {
        return memory::zero_layout();
    }

    // The ladder and the band only take smaller sides up to the transform threshold.
    let direct_cap = cap.min(threshold::ntt());
    let mut best = ladder_budget(direct_cap);
    if direct_cap >= threshold::toom42_min() {
        let band_total = total_cap.min(direct_cap.saturating_mul(7) / 2);
        best = memory::max_layout(best, band_budget_envelope(band_total));
    }

    #[cfg(not(any(force_bits = "16", target_pointer_width = "16")))]
    {
        let asym_reachable = cap >= threshold::ntt_asym_min()
            && total_cap / (threshold::ntt_asym_ratio().saturating_add(1))
                >= threshold::ntt_asym_min();
        if cap > threshold::ntt() || asym_reachable {
            let mut transform = ntt::memory_requirement_up_to(total_cap, cap);
            let chunked_cap = cap.min(total_cap / (ntt::CHUNKING_RATIO + 1));
            if chunked_cap > threshold::ntt() {
                transform = memory::add_layout(
                    transform,
                    memory_chain_budget_envelope(2 * chunked_cap, chunked_cap),
                );
            }
            best = memory::max_layout(best, transform);
        }
    }
    best
}

/// Budget for the tail re-dispatch of the chunked NTT over `a` in chunks
/// of `2*b`: the tail re-enters the dispatcher while the persistent
/// pipeline is still allocated, so its budget adds to the pipeline's.
#[cfg(not(any(force_bits = "16", target_pointer_width = "16")))]
fn chunk_tail_budget(a: usize, b: usize, own: Layout) -> Layout {
    let tail = a % b;
    // Without chunking the operands go through a single convolution: there is no tail.
    if tail == 0 || !ntt::is_chunked(a, b) {
        return own;
    }
    let tail_layout = memory_chain_budget(b, tail);
    memory::add_layout(own, tail_layout)
}

/// Budget for the tail re-dispatch of a chunk loop over `a` in chunks of
/// `b`: the tail has length `a % b` and re-enters the dispatcher swapped
/// as `(b, a % b)`. The chunk kernels and the tail run sequentially over
/// the same scratch, so the tail needs `max(own, tail)`, not the sum.
fn chunk_tail_budget_max(a: usize, b: usize, own: Layout) -> Layout {
    let tail = a % b;
    if tail == 0 {
        return own;
    }
    let tail_layout = memory_chain_budget(b, tail);
    memory::max_layout(own, tail_layout)
}

/// Temporary scratch space sufficient for any multiplication whose operands both have at
/// most `max_len` words.
///
/// Unlike [`memory_requirement_exact`], which is only valid for the exact operand lengths
/// it is given, this covers every shorter product too.
#[inline]
pub fn memory_requirement_up_to_operand_len(max_len: usize) -> Layout {
    memory_chain_budget_envelope(2 * max_len, max_len)
}

/// Temporary scratch space required for multiplication.
#[inline]
pub fn memory_requirement_exact(total_len: usize, smaller_len: usize) -> Layout {
    memory_requirement_up_to(total_len, smaller_len)
}

/// c = a * b, c must be filled with zeros.
#[inline]
pub fn multiply<'a>(c: &mut [Word], a: &'a [Word], b: &'a [Word], memory: &mut Memory) {
    debug_assert!(c.iter().all(|&v| v == 0));
    debug_assert_zero!(add_signed_mul(c, Sign::Positive, a, b, memory));
}

/// c += sign * a * b
///
/// Returns carry.
#[must_use]
pub fn add_signed_mul<'a>(
    c: &mut [Word],
    sign: Sign,
    mut a: &'a [Word],
    mut b: &'a [Word],
    memory: &mut Memory,
) -> SignedWord {
    debug_assert!(c.len() == a.len() + b.len());

    if a.len() < b.len() {
        mem::swap(&mut a, &mut b);
    }

    // Heavily unbalanced: transform the smaller operand once and reuse its
    // spectrum across chunks of the larger one.
    #[cfg(not(any(force_bits = "16", target_pointer_width = "16")))]
    if b.len() >= threshold::ntt_asym_min() && a.len() >= threshold::ntt_asym_ratio() * b.len() {
        return ntt::add_signed_mul(c, sign, a, b, memory);
    }
    #[cfg(any(force_bits = "16", target_pointer_width = "16"))]
    let _ = (&a, &b);

    if b.len() >= threshold::toom42_min()
        && b.len() <= threshold::ntt()
        && toom_4_2::in_band(a.len(), b.len())
    {
        toom_4_2::add_signed_mul(c, sign, a, b, memory)
    } else if b.len() <= threshold::simple() {
        simple::add_signed_mul(c, sign, a, b, memory)
    } else if b.len() <= threshold::karatsuba() {
        karatsuba::add_signed_mul(c, sign, a, b, memory)
    } else if b.len() <= threshold::toom4() {
        toom_3::add_signed_mul(c, sign, a, b, memory)
    } else if b.len() <= threshold::ntt() {
        toom_4::add_signed_mul(c, sign, a, b, memory)
    } else {
        #[cfg(not(any(force_bits = "16", target_pointer_width = "16")))]
        {
            ntt::add_signed_mul(c, sign, a, b, memory)
        }
        #[cfg(any(force_bits = "16", target_pointer_width = "16"))]
        {
            let _ = (c, sign, a, b, memory);
            unreachable!("NTT unavailable on 16-bit targets");
        }
    }
}

/// c += sign * a * b with len(a) == len(b)
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

    if n <= threshold::simple() {
        simple::add_signed_mul_same_len(c, sign, a, b, memory)
    } else if n <= threshold::karatsuba() {
        karatsuba::add_signed_mul_same_len(c, sign, a, b, memory)
    } else if n <= threshold::toom4() {
        toom_3::add_signed_mul_same_len(c, sign, a, b, memory)
    } else if n <= threshold::ntt() {
        toom_4::add_signed_mul_same_len(c, sign, a, b, memory)
    } else {
        #[cfg(not(any(force_bits = "16", target_pointer_width = "16")))]
        {
            ntt::add_signed_mul_same_len(c, sign, a, b, memory)
        }
        #[cfg(any(force_bits = "16", target_pointer_width = "16"))]
        {
            let _ = (c, sign, a, b, memory);
            unreachable!("NTT unavailable on 16-bit targets");
        }
    }
}

#[cfg(all(test, feature = "std"))]
mod threshold_tests {
    use super::*;
    use crate::arch::word::Word;
    use crate::Sign::Positive;

    /// Compare karatsuba vs toom-3 at various word counts to find [`THRESHOLD_KARATSUBA`].
    /// Run with:
    ///   cargo test -p dashu-int --release -- mul::threshold_tests::crossover_karatsuba --nocapture --ignored
    #[test]
    #[ignore]
    #[cfg(feature = "std")]
    fn crossover_karatsuba() {
        use std::time::Instant;

        let sizes: &[usize] = &[80, 100, 120, 140, 160, 180, 200, 240, 280, 320, 360, 400];

        println!("{:>8} {:>14} {:>14} {:>10}", "words", "karatsuba(µs)", "toom-3(µs)", "ratio");
        println!("{}", "-".repeat(50));

        for &n in sizes {
            let a: Vec<Word> = (0..n)
                .map(|i| (i as Word + 1).wrapping_mul(0x9E3779B97F4A7C15u64 as Word))
                .collect();
            let b: Vec<Word> = (0..n)
                .map(|i| (i as Word + 1).wrapping_mul(0xC6A4A7935BD1E995u64 as Word))
                .collect();
            let mut c_kara = vec![0 as Word; 2 * n];
            let mut c_toom = vec![0 as Word; 2 * n];
            let layout_kara = karatsuba::memory_requirement_up_to(n);
            let layout_toom = toom_3::memory_requirement_up_to(n);
            // Use the larger layout so both algorithms get enough memory.
            let layout = if layout_kara.size() > layout_toom.size() {
                layout_kara
            } else {
                layout_toom
            };
            let warmup = 5;
            let iters = 20;

            // Time karatsuba
            let t_kara = {
                let mut best = f64::MAX;
                for _ in 0..warmup {
                    let mut alloc = crate::memory::MemoryAllocation::new(layout);
                    let mut mem = alloc.memory();
                    c_kara.fill(0);
                    let _c =
                        karatsuba::add_signed_mul_same_len(&mut c_kara, Positive, &a, &b, &mut mem);
                }
                for _ in 0..iters {
                    let mut alloc = crate::memory::MemoryAllocation::new(layout);
                    let mut mem = alloc.memory();
                    c_kara.fill(0);
                    let start = Instant::now();
                    let _c =
                        karatsuba::add_signed_mul_same_len(&mut c_kara, Positive, &a, &b, &mut mem);
                    let elapsed = start.elapsed().as_secs_f64() * 1_000_000.0;
                    if elapsed < best {
                        best = elapsed;
                    }
                }
                best
            };

            // Time toom-3
            let t_toom = {
                let mut best = f64::MAX;
                for _ in 0..warmup {
                    let mut alloc = crate::memory::MemoryAllocation::new(layout);
                    let mut mem = alloc.memory();
                    c_toom.fill(0);
                    let _c =
                        toom_3::add_signed_mul_same_len(&mut c_toom, Positive, &a, &b, &mut mem);
                }
                for _ in 0..iters {
                    let mut alloc = crate::memory::MemoryAllocation::new(layout);
                    let mut mem = alloc.memory();
                    c_toom.fill(0);
                    let start = Instant::now();
                    let _c =
                        toom_3::add_signed_mul_same_len(&mut c_toom, Positive, &a, &b, &mut mem);
                    let elapsed = start.elapsed().as_secs_f64() * 1_000_000.0;
                    if elapsed < best {
                        best = elapsed;
                    }
                }
                best
            };

            assert_eq!(&c_kara[..], &c_toom[..], "mismatch at n={n}");
            println!("{:>8} {:>14.1} {:>14.1} {:>9.2}x", n, t_kara, t_toom, t_toom / t_kara);
        }
    }

    /// Compare toom-3 vs toom-4 at various word counts to find [`THRESHOLD_TOOM4_MUL`].
    /// Run with:
    ///   cargo test -p dashu-int --release -- mul::threshold_tests::crossover_toom4 --nocapture --ignored
    #[test]
    #[ignore]
    #[cfg(feature = "std")]
    fn crossover_toom4() {
        use std::time::Instant;

        let sizes: &[usize] = &[
            320, 400, 500, 640, 800, 1000, 1300, 1600, 2000, 2600, 3200, 4000,
        ];

        println!("{:>8} {:>14} {:>14} {:>10}", "words", "toom-3(µs)", "toom-4(µs)", "ratio");
        println!("{}", "-".repeat(50));

        for &n in sizes {
            let a: Vec<Word> = (0..n)
                .map(|i| (i as Word + 1).wrapping_mul(0x9E3779B97F4A7C15u64 as Word))
                .collect();
            let b: Vec<Word> = (0..n)
                .map(|i| (i as Word + 1).wrapping_mul(0xC6A4A7935BD1E995u64 as Word))
                .collect();
            let mut c_toom3 = vec![0 as Word; 2 * n];
            let mut c_toom4 = vec![0 as Word; 2 * n];
            let layout_3 = toom_3::memory_requirement_up_to(n);
            let layout_4 = toom_4::memory_requirement_up_to(n);
            // Use the larger layout so both algorithms get enough memory.
            let layout = if layout_3.size() > layout_4.size() {
                layout_3
            } else {
                layout_4
            };
            let warmup = 5;
            let iters = 20;

            let time = |f: &mut dyn FnMut(&mut Memory)| {
                let mut best = f64::MAX;
                for _ in 0..warmup {
                    let mut alloc = crate::memory::MemoryAllocation::new(layout);
                    let mut mem = alloc.memory();
                    f(&mut mem);
                }
                for _ in 0..iters {
                    let mut alloc = crate::memory::MemoryAllocation::new(layout);
                    let mut mem = alloc.memory();
                    let start = Instant::now();
                    f(&mut mem);
                    let elapsed = start.elapsed().as_secs_f64() * 1_000_000.0;
                    if elapsed < best {
                        best = elapsed;
                    }
                }
                best
            };

            let t_toom3 = time(&mut |mem| {
                c_toom3.fill(0);
                let _c = toom_3::add_signed_mul_same_len(&mut c_toom3, Positive, &a, &b, mem);
            });
            let t_toom4 = time(&mut |mem| {
                c_toom4.fill(0);
                let _c = toom_4::add_signed_mul_same_len(&mut c_toom4, Positive, &a, &b, mem);
            });

            assert_eq!(&c_toom3[..], &c_toom4[..], "mismatch at n={n}");
            println!("{:>8} {:>14.1} {:>14.1} {:>9.2}x", n, t_toom3, t_toom4, t_toom4 / t_toom3);
        }
    }

    /// The chunked-NTT asymmetric entry must agree with the chunked Toom-4
    /// path it replaces, at a shape that exercises the new dispatch gate.
    #[test]
    #[cfg(all(
        feature = "std",
        not(any(force_bits = "16", target_pointer_width = "16"))
    ))]
    fn ntt_asym_entry_matches_toom4_chunks() {
        let ys = threshold::ntt_asym_min();
        let xs = threshold::ntt_asym_ratio() * ys;
        let a: Vec<Word> = (0..xs)
            .map(|i| (i as Word + 1).wrapping_mul(0x9E3779B97F4A7C15u64 as Word))
            .collect();
        let b: Vec<Word> = (0..ys)
            .map(|i| (i as Word + 1).wrapping_mul(0xC6A4A7935BD1E995u64 as Word))
            .collect();
        let mut c_ntt = vec![0 as Word; xs + ys];
        let mut c_toom = vec![0 as Word; xs + ys];
        let l_ntt = ntt::memory_requirement_up_to(xs + ys, ys);
        let l_toom = toom_4::memory_requirement_up_to(ys);
        let layout = if l_ntt.size() > l_toom.size() {
            l_ntt
        } else {
            l_toom
        };
        {
            let mut alloc = crate::memory::MemoryAllocation::new(layout);
            let mut mem = alloc.memory();
            let _c = add_signed_mul(&mut c_ntt, Positive, &a, &b, &mut mem);
        }
        {
            let mut alloc = crate::memory::MemoryAllocation::new(layout);
            let mut mem = alloc.memory();
            helpers::add_signed_mul_split_into_chunks(
                &mut c_toom,
                Positive,
                &a,
                &b,
                ys,
                &mut mem,
                toom_4::add_signed_mul_same_len,
            );
        }
        assert_eq!(&c_ntt[..], &c_toom[..]);
    }

    /// Compare the chunked NTT path against chunked Toom-4 for heavily
    /// unbalanced operands, to tune [`THRESHOLD_NTT_ASYM_MIN`]. Run with:
    ///   cargo test -p dashu-int --features tuning --release \
    ///     -- mul::threshold_tests::crossover_ntt_asym --ignored --nocapture
    #[test]
    #[ignore]
    #[allow(clippy::let_underscore_must_use)]
    #[cfg(all(
        feature = "std",
        not(any(
            force_bits = "16",
            force_bits = "32",
            target_pointer_width = "16",
            target_pointer_width = "32"
        ))
    ))]
    fn crossover_ntt_asym() {
        use std::time::Instant;

        let sizes: &[(usize, usize)] = &[
            (12288, 2048),
            (12288, 3072),
            (16384, 4096),
            (24576, 4096),
            (32768, 4096),
            (65536, 4096),
        ];

        println!(
            "{:>11} {:>16} {:>14} {:>10}",
            "xs_x_ys", "chunked-toom4(µs)", "ntt-chunked(µs)", "ratio"
        );
        println!("{}", "-".repeat(56));

        for &(xs, ys) in sizes {
            let a: Vec<Word> = (0..xs)
                .map(|i| (i as Word + 1).wrapping_mul(0x9E3779B97F4A7C15u64 as Word))
                .collect();
            let b: Vec<Word> = (0..ys)
                .map(|i| (i as Word + 1).wrapping_mul(0xC6A4A7935BD1E995u64 as Word))
                .collect();
            let mut c0 = vec![0 as Word; xs + ys];
            let mut c1 = vec![0 as Word; xs + ys];
            let l_t = toom_4::memory_requirement_up_to(ys);
            let l_n = ntt::memory_requirement_up_to(xs + ys, ys);
            let layout = if l_t.size() > l_n.size() { l_t } else { l_n };
            let warmup = 3;
            let iters = 10;

            let time = |f: &mut dyn FnMut(&mut Memory)| {
                let mut best = f64::MAX;
                for _ in 0..warmup {
                    let mut alloc = crate::memory::MemoryAllocation::new(layout);
                    let mut mem = alloc.memory();
                    f(&mut mem);
                }
                for _ in 0..iters {
                    let mut alloc = crate::memory::MemoryAllocation::new(layout);
                    let mut mem = alloc.memory();
                    let start = Instant::now();
                    f(&mut mem);
                    let elapsed = start.elapsed().as_secs_f64() * 1_000_000.0;
                    if elapsed < best {
                        best = elapsed;
                    }
                }
                best
            };

            let t_toom = time(&mut |mem| {
                c0.fill(0);
                let _c = helpers::add_signed_mul_split_into_chunks(
                    &mut c0,
                    Positive,
                    &a,
                    &b,
                    ys,
                    mem,
                    toom_4::add_signed_mul_same_len,
                );
            });
            let t_ntt = time(&mut |mem| {
                c1.fill(0);
                let _c = ntt::add_signed_mul(&mut c1, Positive, &a, &b, mem);
            });

            assert_eq!(&c0[..], &c1[..], "mismatch at {xs}x{ys}");
            println!(
                "{:>11} {:>16.1} {:>14.1} {:>9.2}x",
                format!("{}x{}", xs, ys),
                t_toom,
                t_ntt,
                t_ntt / t_toom
            );
        }
    }

    /// Compare NTT against toom-3 at various word counts to find [`THRESHOLD_NTT`].
    ///
    /// Run with (set a huge NTT threshold to keep toom-3 pure):
    /// ```sh
    /// DASHU_THRESHOLD_NTT_MUL=99999999 cargo test -p dashu-int --features tuning --release \
    ///   -- mul::threshold_tests::crossover_ntt --ignored --nocapture
    /// ```
    ///
    /// The output is a table: words, b_pack, N, toom-3 time, NTT time, ratio.
    #[test]
    #[ignore]
    #[allow(clippy::let_underscore_must_use)]
    #[cfg(all(
        feature = "std",
        not(any(
            force_bits = "16",
            force_bits = "32",
            target_pointer_width = "16",
            target_pointer_width = "32"
        ))
    ))]
    fn crossover_ntt() {
        use std::time::Instant;

        let sizes: &[usize] = &[
            1_000, 2_000, 3_000, 4_000, 5_000, 6_000, 7_000, 8_000, 9_000, 10_000, 20_000, 40_000,
            80_000,
        ];

        println!(
            "{:>10} {:>4} {:>8} {:>12} {:>12} {:>10}",
            "words", "bp", "N", "toom-3(ms)", "ntt(ms)", "ratio"
        );
        println!("{}", "-".repeat(68));

        for &n in sizes {
            let a: Vec<Word> = (0..n)
                .map(|i| (i as u64 + 1).wrapping_mul(0x9E3779B97F4A7C15))
                .collect();
            let b: Vec<Word> = (0..n)
                .map(|i| (i as u64 + 1).wrapping_mul(0xC6A4A7935BD1E995))
                .collect();
            let mut c_toom = vec![0u64; 2 * n];
            let mut c_ntt = vec![0u64; 2 * n];

            let layout_ntt = super::ntt::memory_requirement_up_to(2 * n, n);
            let layout_toom = super::toom_3::memory_requirement_up_to(n);
            let layout = if layout_ntt.size() > layout_toom.size() {
                layout_ntt
            } else {
                layout_toom
            };
            let warmup = 2;
            let iters = 5;

            // toom-3 (may use NTT internally depending on DASHU_THRESHOLD_NTT_MUL)
            let t_toom = {
                let mut best = f64::MAX;
                for _ in 0..warmup {
                    let mut alloc = crate::memory::MemoryAllocation::new(layout);
                    let mut mem = alloc.memory();
                    c_toom.fill(0);
                    let _ = super::toom_3::add_signed_mul(&mut c_toom, Positive, &a, &b, &mut mem);
                }
                for _ in 0..iters {
                    let mut alloc = crate::memory::MemoryAllocation::new(layout);
                    let mut mem = alloc.memory();
                    c_toom.fill(0);
                    let start = Instant::now();
                    let _ = super::toom_3::add_signed_mul(&mut c_toom, Positive, &a, &b, &mut mem);
                    let elapsed = start.elapsed().as_secs_f64() * 1000.0;
                    if elapsed < best {
                        best = elapsed;
                    }
                }
                best
            };

            // NTT (via public entry, bypasses dispatch)
            let t_ntt = {
                let mut best = f64::MAX;
                for _ in 0..warmup {
                    let mut alloc = crate::memory::MemoryAllocation::new(layout);
                    let mut mem = alloc.memory();
                    c_ntt.fill(0);
                    let _ = super::ntt::add_signed_mul(&mut c_ntt, Positive, &a, &b, &mut mem);
                }
                for _ in 0..iters {
                    let mut alloc = crate::memory::MemoryAllocation::new(layout);
                    let mut mem = alloc.memory();
                    c_ntt.fill(0);
                    let start = Instant::now();
                    let _ = super::ntt::add_signed_mul(&mut c_ntt, Positive, &a, &b, &mut mem);
                    let elapsed = start.elapsed().as_secs_f64() * 1000.0;
                    if elapsed < best {
                        best = elapsed;
                    }
                }
                best
            };

            assert_eq!(&c_ntt[..], &c_toom[..], "mismatch at n={n}");

            let (b_pack, nn, _k_eff) = super::ntt::select_params(n, n);
            println!(
                "{:>10} {:>4} {:>8} {:>12.3} {:>12.3} {:>9.2}x",
                n,
                b_pack,
                nn,
                t_toom,
                t_ntt,
                t_ntt / t_toom
            );
        }
    }

    #[test]
    fn test_memory_envelope_below_toom42() {
        for total in [0, 1, 2, 32, 64, 128] {
            let cap = total / 2;
            assert_eq!(
                memory_chain_budget_envelope(total, cap),
                memory_chain_budget(total - cap, cap),
                "unexpected scratch for total={total} cap={cap}"
            );
        }
    }

    /// The envelope must dominate the per-shape chain budget for every shape within the
    /// caps (total length `<= total_cap`, smaller side `<= smaller_cap`) — this is the
    /// property the division, GCD and modular multiplication memory models rely on (a
    /// sampled shape does NOT bound its neighbors since the Toom-4x2 band makes the budget
    /// non-monotone in both the split and the total).
    #[test]
    fn test_memory_envelope_dominates_all_shapes_exhaustive() {
        // Every total cap up to MAX and every smaller-side cap, against every shape.
        const MAX: usize = 700;
        // by_smaller[b]: the largest budget seen so far for a smaller side of exactly b
        let mut by_smaller = vec![0usize; MAX / 2 + 1];
        for total in 2..=MAX {
            for (b, slot) in by_smaller
                .iter_mut()
                .enumerate()
                .take(total / 2 + 1)
                .skip(1)
            {
                *slot = (*slot).max(memory_chain_budget(total - b, b).size());
            }
            let mut need = 0;
            for (cap, &budget) in by_smaller.iter().enumerate().take(total / 2 + 1).skip(1) {
                need = need.max(budget);
                let envelope = memory_chain_budget_envelope(total, cap).size();
                assert!(
                    need <= envelope,
                    "envelope {envelope} bytes < shape {need} bytes at total={total} cap={cap}"
                );
            }
            // a cap beyond half the total does not add shapes
            assert_eq!(
                memory_chain_budget_envelope(total, total).size(),
                memory_chain_budget_envelope(total, total / 2).size()
            );
        }
    }

    #[test]
    fn test_memory_envelope_dominates_sampled_shapes() {
        // Larger sizes (through the Toom-4 and transform ranges): sampled shapes within
        // the caps, including the operands-at-most-n-words square used by the modular
        // multiplications.
        // (there is no transform-based multiplication with 16-bit words)
        let ntt = threshold::ntt().min(4000);
        let totals = [
            900,
            1500,
            2100,
            3333,
            5000,
            7999,
            8001,
            8002,
            9000,
            2 * ntt + 100,
            4 * ntt,
            4 * ntt + 8,
            5 * ntt + 3,
            8 * ntt + 9,
            20000,
            30011,
            50000,
        ];
        for total_cap in totals {
            for cap in [
                total_cap / 2,
                total_cap / 3,
                total_cap / 4 + 1,
                total_cap / 7,
                97,
            ] {
                let envelope = memory_chain_budget_envelope(total_cap, cap).size();
                let larger = memory_chain_budget_envelope(total_cap + 1, cap + 1).size();
                assert!(envelope <= larger, "not monotone at total={total_cap} cap={cap}");
                for total in [total_cap, total_cap - 1, total_cap - 37, total_cap * 3 / 4] {
                    let top = cap.min(total / 2);
                    let step = (top / 150).max(1);
                    let smaller = (1..=top).step_by(step).chain(top.saturating_sub(40)..=top);
                    for b in smaller.filter(|&b| b >= 1) {
                        let shape = memory_chain_budget(total - b, b).size();
                        assert!(
                            shape <= envelope,
                            "envelope {envelope} bytes < shape {shape} bytes at \
                             total_cap={total_cap} cap={cap} total={total} b={b}"
                        );
                    }
                }
            }
        }
    }
}
