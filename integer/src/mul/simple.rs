//! Simple multiplication algorithm.

use crate::{
    add,
    arch::word::{SignedWord, Word},
    math,
    memory::Memory,
    mul::{self, helpers},
    primitive::double_word,
    Sign::{self, *},
};

/// Split larger length into chunks of CHUNK_LEN..2 * CHUNK_LEN for memory locality.
const CHUNK_LEN: usize = 1024;

/// Max supported Smaller factor length.
pub const MAX_SMALLER_LEN: usize = CHUNK_LEN;

/// c += sign * a * b
/// Simple method: O(a.len() * b.len()).
///
/// Returns carry.
pub fn add_signed_mul(
    c: &mut [Word],
    sign: Sign,
    a: &[Word],
    b: &[Word],
    memory: &mut Memory,
) -> SignedWord {
    debug_assert!(a.len() >= b.len() && c.len() == a.len() + b.len());
    debug_assert!(b.len() <= MAX_SMALLER_LEN);
    if a.len() <= CHUNK_LEN {
        add_signed_mul_chunk(c, sign, a, b, memory)
    } else {
        helpers::add_signed_mul_split_into_chunks(
            c,
            sign,
            a,
            b,
            CHUNK_LEN,
            memory,
            add_signed_mul_chunk,
        )
    }
}

/// c += sign * a * b
/// Simple method: O(a.len() * b.len()).
///
/// Returns carry.
pub fn add_signed_mul_same_len(
    c: &mut [Word],
    sign: Sign,
    a: &[Word],
    b: &[Word],
    memory: &mut Memory,
) -> SignedWord {
    debug_assert!(a.len() == b.len() && c.len() == a.len() + b.len());
    debug_assert!(b.len() <= MAX_SMALLER_LEN);
    add_signed_mul_chunk(c, sign, a, b, memory)
}

/// c += sign * a * b
/// Simple method: O(a.len() * b.len()).
///
/// Returns carry.
fn add_signed_mul_chunk(
    c: &mut [Word],
    sign: Sign,
    a: &[Word],
    b: &[Word],
    _memory: &mut Memory,
) -> SignedWord {
    debug_assert!(a.len() >= b.len() && c.len() == a.len() + b.len());
    debug_assert!(a.len() <= CHUNK_LEN);

    match sign {
        Positive => SignedWord::from(add_mul_chunk(c, a, b)),
        Negative => -SignedWord::from(sub_mul_chunk(c, a, b)),
    }
}

/// `words[..n] += rhs[..n] * (mult0 + mult1 * B)`, where `n == rhs.len()`.
///
/// This is an "addmul_2" kernel: it consumes two multiplier words per sweep over
/// `rhs`, so the accumulator word `words[k]` is loaded and stored once per two
/// multiplier words instead of once per word. This halves the memory traffic on
/// `words` and exposes two independent multiply chains, mirroring GMP's
/// `mpn_addmul_2`.
///
/// Only `words[..n]` is modified. The two extra high words of the product (the
/// carries out of columns `n` and `n + 1`) are returned as `(carry_lo, carry_hi)`,
/// to be accumulated by the caller at `words[n]` and `words[n + 1]`.
#[inline]
fn add_mul_dword_same_len_in_place(
    words: &mut [Word],
    rhs: &[Word],
    mult0: Word,
    mult1: Word,
) -> (Word, Word) {
    debug_assert!(words.len() == rhs.len());
    let mut carry_lo: Word = 0;
    let mut carry_hi: Word = 0;
    for (x, &y) in words.iter_mut().zip(rhs.iter()) {
        (*x, carry_lo) = math::mul_add_2carry(y, mult0, *x, carry_lo);
        (carry_lo, carry_hi) = math::mul_add_2carry(y, mult1, carry_lo, carry_hi);
    }
    (carry_lo, carry_hi)
}

/// c += a * b
/// Simple method: O(a.len() * b.len()).
///
/// Returns carry.
fn add_mul_chunk(c: &mut [Word], a: &[Word], b: &[Word]) -> bool {
    debug_assert!(a.len() >= b.len() && c.len() == a.len() + b.len());
    debug_assert!(a.len() < 2 * CHUNK_LEN);
    let n = a.len();
    let mut overflow = false;
    let mut i = 0;

    // Consume the multiplier two words at a time via the addmul_2 kernel.
    let mut pairs = b.chunks_exact(2);
    for pair in &mut pairs {
        let (carry_lo, carry_hi) =
            add_mul_dword_same_len_in_place(&mut c[i..i + n], a, pair[0], pair[1]);
        overflow |= add::add_dword_in_place(&mut c[i + n..], double_word(carry_lo, carry_hi));
        i += 2;
    }

    // Handle the leftover odd multiplier word with a plain addmul_1.
    if let &[m] = pairs.remainder() {
        let carry_word = mul::add_mul_word_same_len_in_place(&mut c[i..i + n], m, a);
        overflow |= add::add_word_in_place(&mut c[i + n..], carry_word);
    }

    overflow
}

/// `words[..n] -= rhs[..n] * (mult0 + mult1 * B)`, where `n == rhs.len()`.
///
/// This function is the analogue of [`add_mul_dword_same_len_in_place`]: the product limbs
/// of `rhs * (mult0 + mult1 * B)` are formed with the same two-multiplier-per-sweep
/// carry recurrence (here with a zero accumulator), and subtracted from `words` in
/// the same pass, so `words[k]` is touched once per two multiplier words.
///
/// Only `words[..n]` is modified. `borrow` from the low subtraction is folded into
/// `carry_lo`, so the caller only needs [`add::sub_dword_in_place`] on the returned
/// `(carry_lo, carry_hi)`.
#[inline]
fn sub_mul_dword_same_len_in_place(
    words: &mut [Word],
    rhs: &[Word],
    mult0: Word,
    mult1: Word,
) -> (Word, Word) {
    debug_assert!(words.len() == rhs.len());
    let mut carry_lo: Word = 0;
    let mut carry_hi: Word = 0;
    let mut borrow: Word = 0;
    for (x, &y) in words.iter_mut().zip(rhs.iter()) {
        let (prod_limb, carry_a) = math::mul_add_carry(y, mult0, carry_lo);
        (carry_lo, carry_hi) = math::mul_add_2carry(y, mult1, carry_a, carry_hi);
        let (t, b1) = (*x).overflowing_sub(prod_limb);
        let (t, b2) = t.overflowing_sub(borrow);
        *x = t;
        borrow = Word::from(b1 | b2);
    }
    let (lo, overflow) = carry_lo.overflowing_add(borrow);
    (lo, carry_hi.wrapping_add(overflow as Word))
}

/// c -= a * b
/// Simple method: O(a.len() * b.len()).
///
/// Returns borrow.
fn sub_mul_chunk(c: &mut [Word], a: &[Word], b: &[Word]) -> bool {
    debug_assert!(a.len() >= b.len() && c.len() == a.len() + b.len());
    debug_assert!(a.len() < 2 * CHUNK_LEN);
    let n = a.len();
    let mut borrow_out = false;
    let mut i = 0;

    // Consume the multiplier two words at a time via the submul_2 kernel.
    let mut pairs = b.chunks_exact(2);
    for pair in &mut pairs {
        let (carry_lo, carry_hi) =
            sub_mul_dword_same_len_in_place(&mut c[i..i + n], a, pair[0], pair[1]);
        borrow_out |= add::sub_dword_in_place(&mut c[i + n..], double_word(carry_lo, carry_hi));
        i += 2;
    }

    // Handle the leftover odd multiplier word with a plain submul_1.
    if let &[m] = pairs.remainder() {
        let borrow_word = mul::sub_mul_word_same_len_in_place(&mut c[i..i + n], m, a);
        borrow_out |= add::sub_word_in_place(&mut c[i + n..], borrow_word);
    }

    borrow_out
}

#[cfg(test)]
mod addmul2_tests {
    use super::*;
    use crate::arch;
    #[cfg(not(feature = "std"))]
    use alloc::vec::Vec;

    #[cfg(feature = "std")]
    type MulChunkFn = dyn Fn(&mut [Word], &[Word], &[Word]) -> bool;

    /// Reference: the original addmul_1-based schoolbook (known correct).
    fn ref_add_mul(c: &mut [Word], a: &[Word], b: &[Word]) -> bool {
        let mut carry: Word = 0;
        for (i, m) in b.iter().enumerate() {
            let carry_word = mul::add_mul_word_same_len_in_place(&mut c[i..i + a.len()], *m, a);
            let (carry_word, carry_next) =
                arch::add::add_with_carry(c[i + a.len()], carry_word, carry);
            c[i + a.len()] = carry_word;
            carry = carry_next;
        }
        carry != 0
    }

    /// Reference: the original submul_1-based schoolbook (known correct).
    fn ref_sub_mul(c: &mut [Word], a: &[Word], b: &[Word]) -> bool {
        let mut borrow: Word = 0;
        for (i, m) in b.iter().enumerate() {
            let borrow_word = mul::sub_mul_word_same_len_in_place(&mut c[i..i + a.len()], *m, a);
            let (borrow_word, borrow_next) =
                arch::add::sub_with_borrow(c[i + a.len()], borrow_word, borrow);
            c[i + a.len()] = borrow_word;
            borrow = borrow_next;
        }
        borrow != 0
    }

    /// Compare the addmul_2 basecase against the old addmul_1 basecase.
    /// Run with:
    ///   cargo test -p dashu-int --release -- mul::simple::addmul2_tests::bench --ignored --nocapture
    #[test]
    #[ignore]
    #[cfg(feature = "std")]
    fn bench_addmul2_vs_addmul1() {
        use core::hint::black_box;
        use std::time::Instant;

        let mut state: u64 = 0x9e37_79b9_7f4a_7c15;
        let mut next = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };

        println!(
            "{:>6} | {:>10} {:>10} {:>8} | {:>10} {:>10} {:>8}",
            "words", "addmul1", "addmul2", "speedup", "submul1", "submul2", "speedup"
        );
        println!("{}", "-".repeat(72));
        for &n in &[8usize, 12, 16, 20, 24, 32, 48, 64, 96] {
            let a: Vec<Word> = (0..n).map(|_| next() as Word).collect();
            let b: Vec<Word> = (0..n).map(|_| next() as Word).collect();
            let mut c = vec![0 as Word; 2 * n];
            let rounds = 200_000;
            let warm = 1000;

            // Representative timing: `c` is zeroed before each call (as in a real
            // multiply into fresh output), with the zeroing kept outside the timed
            // region. Take the min over many rounds to suppress OS jitter.
            let time_it = |f: &MulChunkFn, c: &mut [Word]| {
                for _ in 0..warm {
                    c.fill(0);
                    black_box(f(black_box(&mut *c), &a, &b));
                }
                let mut best = f64::MAX;
                for _ in 0..rounds {
                    c.fill(0);
                    let start = Instant::now();
                    black_box(f(black_box(&mut *c), &a, &b));
                    let e = start.elapsed().as_secs_f64() * 1e9;
                    if e < best {
                        best = e;
                    }
                }
                best
            };

            let add1 = time_it(&ref_add_mul, &mut c);
            let add2 = time_it(&|c, a, b| add_mul_chunk(c, a, b), &mut c);
            let sub1 = time_it(&ref_sub_mul, &mut c);
            let sub2 = time_it(&|c, a, b| sub_mul_chunk(c, a, b), &mut c);
            println!(
                "{:>6} | {:>10.1} {:>10.1} {:>7.2}x | {:>10.1} {:>10.1} {:>7.2}x",
                n,
                add1,
                add2,
                add1 / add2,
                sub1,
                sub2,
                sub1 / sub2
            );
        }
    }

    #[test]
    fn add_mul_chunk_matches_reference() {
        let mut state: u64 = 0x1234_5678_9abc_def0;
        let mut next = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        for na in 1..=10usize {
            for nb in 1..=na {
                for _ in 0..50 {
                    let a: Vec<Word> = (0..na).map(|_| next() as Word).collect();
                    let b: Vec<Word> = (0..nb).map(|_| next() as Word).collect();
                    let c0: Vec<Word> = (0..na + nb).map(|_| next() as Word).collect();

                    let mut c_ref = c0.clone();
                    let exp_overflow = ref_add_mul(&mut c_ref, &a, &b);
                    let mut c = c0.clone();
                    let overflow = add_mul_chunk(&mut c, &a, &b);
                    assert_eq!(c, c_ref, "add value mismatch na={na} nb={nb}");
                    assert_eq!(overflow, exp_overflow, "add overflow mismatch na={na} nb={nb}");

                    let mut c_ref = c0.clone();
                    let exp_borrow = ref_sub_mul(&mut c_ref, &a, &b);
                    let mut c = c0.clone();
                    let borrow = sub_mul_chunk(&mut c, &a, &b);
                    assert_eq!(c, c_ref, "sub value mismatch na={na} nb={nb}");
                    assert_eq!(borrow, exp_borrow, "sub borrow mismatch na={na} nb={nb}");
                }
            }
        }
    }
}
