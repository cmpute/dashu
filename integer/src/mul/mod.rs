//! Multiplication.

use crate::{
    add,
    arch::word::{DoubleWord, SignedWord, Word},
    helper_macros::debug_assert_zero,
    math,
    memory::{self, Memory},
    primitive::{double_word, extend_word, split_dword},
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

/// If smaller operand length > this, NTT multiplication will be used.
#[cfg(not(any(force_bits = "16", target_pointer_width = "16")))]
const THRESHOLD_NTT_DEFAULT: usize = ntt::THRESHOLD_NTT;
/// NTT unavailable on 16/32-bit word targets — use `usize::MAX` so dispatch never
/// routes to the NTT path.
#[cfg(any(force_bits = "16", target_pointer_width = "16"))]
const THRESHOLD_NTT_DEFAULT: usize = usize::MAX;
#[cfg(not(any(force_bits = "16", target_pointer_width = "16")))]
const_assert!(THRESHOLD_NTT_DEFAULT + 1 >= toom_3::MIN_LEN);

/// Environment-variable overrides for multiplication thresholds.
///
/// When the `tuning` feature is active the user may set `DASHU_THRESHOLD_SIMPLE_MUL`,
/// `DASHU_THRESHOLD_KARATSUBA_MUL` or `DASHU_THRESHOLD_NTT_MUL` to override the
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

/// words += mult * rhs
///
/// Returns carry.
#[must_use]
pub fn add_mul_word_same_len_in_place(words: &mut [Word], mult: Word, rhs: &[Word]) -> Word {
    assert!(words.len() == rhs.len());
    if mult == 0 {
        return 0;
    }

    let mut carry: Word = 0;
    for (a, b) in words.iter_mut().zip(rhs.iter()) {
        let (v_lo, v_hi) = math::mul_add_2carry(mult, *b, *a, carry);
        *a = v_lo;
        carry = v_hi;
    }
    carry
}

/// words += mult * rhs
///
/// Returns carry.
#[must_use]
pub fn add_mul_word_in_place(words: &mut [Word], mult: Word, rhs: &[Word]) -> Word {
    assert!(words.len() >= rhs.len());
    if mult == 0 {
        return 0;
    }

    let n = rhs.len();
    let mut carry = add_mul_word_same_len_in_place(&mut words[..n], mult, rhs);
    if words.len() > n {
        carry = Word::from(add::add_word_in_place(&mut words[n..], carry));
    }
    carry
}

/// `words += (mult0 + mult1 * 2^WORD_BITS) * rhs` where `words.len() == rhs.len()`.
///
/// This is the double-multiplier "addmul_2" kernel (mirrors GMP's `mpn_addmul_2`): it sweeps
/// over `rhs` once while accumulating two independent multiply chains, returning the two carry
/// words `(carry_lo, carry_hi)` to be added by the caller at `words[n]` and `words[n + 1]`.
#[inline]
pub(crate) fn add_mul_dword_same_len_in_place(
    words: &mut [Word],
    rhs: &[Word],
    mult0: Word,
    mult1: Word,
) -> (Word, Word) {
    simple::add_mul_dword_same_len_in_place(words, rhs, mult0, mult1)
}

/// words -= mult * rhs
///
/// Returns borrow.
#[must_use]
pub fn sub_mul_word_same_len_in_place(words: &mut [Word], mult: Word, rhs: &[Word]) -> Word {
    assert!(words.len() == rhs.len());
    if mult == 0 {
        return 0;
    }

    // carry is in -Word::MAX..0
    // carry_plus_max = carry + Word::MAX
    let mut carry_plus_max = Word::MAX;
    for (a, b) in words.iter_mut().zip(rhs.iter()) {
        // Compute val = a - mult * b + carry_plus_max - MAX + (MAX << BITS)
        // val >= 0 - MAX * MAX - MAX + MAX*(MAX+1) = 0
        // val <= MAX - 0 + MAX - MAX + (MAX<<BITS) = DoubleWord::MAX
        // This fits exactly in DoubleWord!
        // We have to be careful to calculate in the correct order to avoid overflow.
        let v = extend_word(*a)
            + extend_word(carry_plus_max)
            + (double_word(0, Word::MAX) - extend_word(Word::MAX))
            - extend_word(mult) * extend_word(*b);
        let (v_lo, v_hi) = split_dword(v);
        *a = v_lo;
        carry_plus_max = v_hi;
    }
    Word::MAX - carry_plus_max
}

/// Temporary scratch space required for multiplication.
pub fn memory_requirement_up_to(total_len: usize, smaller_len: usize) -> Layout {
    if smaller_len <= threshold::simple() {
        memory::zero_layout()
    } else if smaller_len <= threshold::karatsuba() {
        karatsuba::memory_requirement_up_to(smaller_len)
    } else if smaller_len <= threshold::ntt() {
        toom_3::memory_requirement_up_to(smaller_len)
    } else {
        // NTT path — only available on 64-bit word targets.
        #[cfg(not(any(force_bits = "16", target_pointer_width = "16")))]
        {
            ntt::memory_requirement_up_to(total_len, smaller_len)
        }
        #[cfg(any(force_bits = "16", target_pointer_width = "16"))]
        {
            let _ = (total_len, smaller_len);
            unreachable!("NTT unavailable on 16-bit targets");
        }
    }
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

    if b.len() <= threshold::simple() {
        simple::add_signed_mul(c, sign, a, b, memory)
    } else if b.len() <= threshold::karatsuba() {
        karatsuba::add_signed_mul(c, sign, a, b, memory)
    } else if b.len() <= threshold::ntt() {
        toom_3::add_signed_mul(c, sign, a, b, memory)
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
    } else if n <= threshold::ntt() {
        toom_3::add_signed_mul_same_len(c, sign, a, b, memory)
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
}
