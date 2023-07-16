//! Division functions.

use crate::{
    arch::word::{DoubleWord, Word},
    helper_macros::debug_assert_zero,
    math::{shl_dword, shr_word},
    memory::{self, Memory},
    primitive::{double_word, extend_word, highest_dword, lowest_dword, split_dword, WORD_BITS},
    shift,
};
use alloc::alloc::Layout;

type FastDivideNormalized = num_modular::Normalized2by1Divisor<Word>;
type FastDivideNormalized2 = num_modular::Normalized3by2Divisor<Word, DoubleWord>;

mod divide_conquer;
mod simple;
pub(crate) use simple::div_rem_highest_word;

/// If divisor or quotient is at most this length, use the simple division algorithm.
const THRESHOLD_SIMPLE: usize = 32;

/// Normalize a divisor represented as words.
///
/// Returns (shift, fast division for the top words).
#[inline]
pub(crate) fn normalize(words: &mut [Word]) -> (u32, FastDivideNormalized2) {
    let shift = words.last().unwrap().leading_zeros();
    debug_assert_zero!(shift::shl_in_place(words, shift));
    let top_words = highest_dword(words);
    (shift, FastDivideNormalized2::new(top_words))
}

/// words = words / rhs
///
/// rhs must be non-zero
///
/// Returns words % rhs. Panics if `words` is empty.
#[must_use]
pub fn div_by_word_in_place(words: &mut [Word], rhs: Word) -> Word {
    debug_assert!(rhs != 0 && !words.is_empty());

    if rhs == 1 {
        return 0;
    } else if rhs.is_power_of_two() {
        let shift = rhs.trailing_zeros();
        let rem = shift::shr_in_place(words, shift);
        return rem >> (WORD_BITS - shift);
    }

    let shift = rhs.leading_zeros();
    let fast_div_rhs = FastDivideNormalized::new(rhs << shift);
    fast_div_by_word_in_place(words, shift, fast_div_rhs)
}

/// words = words / rhs
///
/// Returns words % rhs.
#[must_use]
pub(crate) fn fast_div_by_word_in_place(
    words: &mut [Word],
    shift: u32,
    fast_div_rhs: FastDivideNormalized,
) -> Word {
    let mut rem = shift::shl_in_place(words, shift);

    for word in words.iter_mut().rev() {
        let a = double_word(*word, rem);
        let (q, r) = fast_div_rhs.div_rem_2by1(a);
        *word = q;
        rem = r;
    }
    rem >> shift
}

/// Panics if `words` is empty
pub fn rem_by_word(words: &[Word], rhs: Word) -> Word {
    debug_assert!(rhs != 0 && !words.is_empty());

    // shortcut
    if rhs.is_power_of_two() {
        return words[0] & (rhs - 1);
    }

    // calculate remainder without normalizing the words
    let shift = rhs.leading_zeros();
    let fast_div_rhs = FastDivideNormalized::new(rhs << shift);
    let rem = fast_rem_by_normalized_word(words, fast_div_rhs);

    // normalize the remainder
    let a = extend_word(rem) << shift;
    let (_, rem) = fast_div_rhs.div_rem_2by1(a);
    rem >> shift
}

pub(crate) fn fast_rem_by_normalized_word(
    words: &[Word],
    fast_div_rhs: FastDivideNormalized,
) -> Word {
    debug_assert!(!words.is_empty());

    // first calculate the highest remainder
    let (last, words_lo) = words.split_last().unwrap();
    let mut rem = fast_div_rhs.div_rem_1by1(*last).1;

    // then iterate through the words
    for word in words_lo.iter().rev() {
        let a = double_word(*word, rem);
        rem = fast_div_rhs.div_rem_2by1(a).1;
    }

    rem
}

/// words = words / rhs
///
/// rhs must not fit in a word, there could be one leading zero in words.
///
/// Returns words % rhs. Panics if `words` is too short (<= 2 words)
pub fn div_by_dword_in_place(words: &mut [Word], rhs: DoubleWord) -> DoubleWord {
    debug_assert!(rhs > Word::MAX as DoubleWord, "call div_by_word_in_place when rhs is small");
    debug_assert!(words.len() >= 2);

    if rhs.is_power_of_two() {
        let first = shift::shr_in_place_one_word(words);
        let shift = rhs.trailing_zeros() - WORD_BITS;
        if shift == 0 {
            return extend_word(first);
        } else {
            let n2 = shift::shr_in_place(words, shift);
            let (n1, n0) = shr_word(first, shift);
            return double_word(n0, n1 | n2) >> (WORD_BITS - shift);
        }
    }

    let shift = rhs.leading_zeros();
    debug_assert!(shift < WORD_BITS); // high word of rhs must not be zero
    let fast_div_rhs = FastDivideNormalized2::new(rhs << shift);
    fast_div_by_dword_in_place(words, shift, fast_div_rhs)
}

/// words = words / rhs
///
/// Returns words % rhs.
#[must_use]
pub(crate) fn fast_div_by_dword_in_place(
    words: &mut [Word],
    shift: u32,
    fast_div_rhs: FastDivideNormalized2,
) -> DoubleWord {
    debug_assert!(words.len() >= 2 && shift < WORD_BITS);
    let hi = shift::shl_in_place(words, shift);

    // first div [hi, last word, second last word] by rhs
    let (top_hi, words_lo) = words.split_last_mut().unwrap();
    let (top_lo, words_lo) = words_lo.split_last_mut().unwrap();
    let (q, mut rem) = fast_div_rhs.div_rem_3by2(*top_lo, double_word(*top_hi, hi));
    *top_hi = 0;
    *top_lo = q;

    // chunk the words into double words, and do 4by2 divisions
    let mut dwords = words_lo.rchunks_exact_mut(2);
    for chunk in &mut dwords {
        let dword = lowest_dword(chunk);
        let (q, new_rem) = fast_div_rhs.div_rem_4by2(dword, rem);
        let (new_lo, new_hi) = split_dword(q);
        *chunk.first_mut().unwrap() = new_lo;
        *chunk.last_mut().unwrap() = new_hi;
        rem = new_rem;
    }

    // there might be a single word left, do a 3by2 division
    let r = dwords.into_remainder();
    if !r.is_empty() {
        debug_assert!(r.len() == 1);
        let r0 = r.first_mut().unwrap();
        let (q, new_rem) = fast_div_rhs.div_rem_3by2(*r0, rem);
        *r0 = q;
        rem = new_rem;
    }

    rem >> shift
}

/// words % rhs, panics if `words` is too short (<= 2 words) or rhs fits in a single Word.
pub fn rem_by_dword(words: &[Word], rhs: DoubleWord) -> DoubleWord {
    debug_assert!(rhs > Word::MAX as DoubleWord, "call div_by_word_in_place when rhs is small");
    debug_assert!(words.len() >= 2);

    if rhs.is_power_of_two() {
        return lowest_dword(words) & (rhs - 1);
    }

    // calculate remainder without normalizing the words
    let shift = rhs.leading_zeros();
    debug_assert!(shift < WORD_BITS);
    let fast_div_rhs = FastDivideNormalized2::new(rhs << shift);
    let rem = fast_rem_by_normalized_dword(words, fast_div_rhs);

    // normalize the remainder
    let (a0, a1, a2) = shl_dword(rem, shift);
    let (_, rem) = fast_div_rhs.div_rem_3by2(a0, double_word(a1, a2));
    rem >> shift
}

pub(crate) fn fast_rem_by_normalized_dword(
    words: &[Word],
    fast_div_rhs: FastDivideNormalized2,
) -> DoubleWord {
    debug_assert!(words.len() >= 2);

    // first calculate the highest remainder
    let (top_hi, words_lo) = words.split_last().unwrap();
    let (top_lo, words_lo) = words_lo.split_last().unwrap();
    let mut rem = fast_div_rhs.div_rem_2by2(double_word(*top_lo, *top_hi)).1;

    // then iterate through the words
    // chunk the words into double words, and do 4by2 divisions
    let mut dwords = words_lo.rchunks_exact(2);
    for chunk in &mut dwords {
        let dword = lowest_dword(chunk);
        rem = fast_div_rhs.div_rem_4by2(dword, rem).1;
    }

    // there might be a single word left, do a 3by2 division
    let r = dwords.remainder();
    if !r.is_empty() {
        debug_assert!(r.len() == 1);
        let r0 = r.first().unwrap();
        rem = fast_div_rhs.div_rem_3by2(*r0, rem).1;
    }

    rem
}

/// Memory requirement for division.
pub fn memory_requirement_exact(lhs_len: usize, rhs_len: usize) -> Layout {
    assert!(lhs_len >= rhs_len && rhs_len >= 2);
    if rhs_len <= THRESHOLD_SIMPLE || lhs_len - rhs_len <= THRESHOLD_SIMPLE {
        memory::zero_layout()
    } else {
        divide_conquer::memory_requirement_exact(lhs_len, rhs_len)
    }
}

/// Divide lhs by rhs, replacing the top words of lhs by the quotient and the
/// bottom words of lhs by the remainder.
///
/// `lhs = [lhs % rhs, lhs / rhs]`
///
/// `rhs` must have at least 2 words and be normalized (the top bit must be 1).
/// `lhs` must be pre-applied the shift from fast_div_rhs_top.
///
/// Returns carry in the quotient. It is at most 1 because rhs is normalized.
#[must_use]
pub(crate) fn div_rem_in_place(
    lhs: &mut [Word],
    rhs: &[Word],
    fast_div_rhs_top: FastDivideNormalized2,
    memory: &mut Memory,
) -> bool {
    debug_assert!(lhs.len() >= rhs.len() && rhs.len() >= 2);

    if rhs.len() <= THRESHOLD_SIMPLE || lhs.len() - rhs.len() <= THRESHOLD_SIMPLE {
        simple::div_rem_in_place(lhs, rhs, fast_div_rhs_top)
    } else {
        divide_conquer::div_rem_in_place(lhs, rhs, fast_div_rhs_top, memory)
    }
}

/// Divide lhs by rhs, replacing the top words of lhs by the quotient and the
/// bottom words of lhs by the remainder.
///
/// `lhs = [lhs % rhs, lhs / rhs]`
///
/// `rhs` must have at least 2 words and be normalized (the top bit must be 1).
/// `lhs` should not be preprocessed, it will be shifted by this function
///
/// Returns carry in the quotient. It is at most 1 because rhs is normalized.
/// To get the actual remainder, the remainder should be shifted back.
pub(crate) fn div_rem_unshifted_in_place(
    lhs: &mut [Word],
    rhs: &[Word],
    shift: u32,
    fast_div_rhs_top: FastDivideNormalized2,
    memory: &mut Memory,
) -> Word {
    // prerequisite: let (shift, fast_div_rhs_top) = normalize(rhs);
    let lhs_carry = shift::shl_in_place(lhs, shift);
    let mut q_top = if lhs_carry > 0 {
        div_rem_highest_word(lhs_carry, lhs, rhs, fast_div_rhs_top)
    } else {
        0
    };
    let overflow = div_rem_in_place(lhs, rhs, fast_div_rhs_top, memory);
    q_top += overflow as Word;
    q_top
}
