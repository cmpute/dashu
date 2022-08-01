//! Multiplication.

use crate::{
    add,
    arch::word::{DoubleWord, SignedWord, Word},
    math,
    memory::{self, Memory},
    primitive::{double_word, extend_word, split_dword},
    sign::Sign,
};
use alloc::alloc::Layout;
use core::mem;
use static_assertions::const_assert;

/// If smaller operand length <= this, simple multiplication will be used.
const THRESHOLD_SIMPLE: usize = 24;
const_assert!(THRESHOLD_SIMPLE <= simple::MAX_SMALLER_LEN);
const_assert!(THRESHOLD_SIMPLE + 1 >= karatsuba::MIN_LEN);

/// If smaller operand length <= this, Karatsuba multiplication will be used.
const THRESHOLD_KARATSUBA: usize = 192;
const_assert!(THRESHOLD_KARATSUBA + 1 >= toom_3::MIN_LEN);

mod helpers;
mod karatsuba;
pub(crate) mod ntt;
mod simple;
mod toom_3;

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
    debug_assert!(
        rhs > Word::MAX as DoubleWord,
        "call mul_word_in_place when rhs is small"
    );

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
    let n = rhs.len();
    let mut carry = add_mul_word_same_len_in_place(&mut words[..n], mult, rhs);
    if words.len() > n {
        carry = Word::from(add::add_word_in_place(&mut words[n..], carry));
    }
    carry
}

/// words -= mult * rhs
///
/// Returns borrow.
#[must_use]
pub fn sub_mul_word_same_len_in_place(words: &mut [Word], mult: Word, rhs: &[Word]) -> Word {
    assert!(words.len() == rhs.len());
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
pub fn memory_requirement_up_to(_total_len: usize, smaller_len: usize) -> Layout {
    if smaller_len <= THRESHOLD_SIMPLE {
        memory::zero_layout()
    } else if smaller_len <= THRESHOLD_KARATSUBA {
        karatsuba::memory_requirement_up_to(smaller_len)
    } else {
        toom_3::memory_requirement_up_to(smaller_len)
    }
}

/// Temporary scratch space required for multiplication.
pub fn memory_requirement_exact(total_len: usize, smaller_len: usize) -> Layout {
    memory_requirement_up_to(total_len, smaller_len)
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

    if b.len() <= THRESHOLD_SIMPLE {
        simple::add_signed_mul(c, sign, a, b, memory)
    } else if b.len() <= THRESHOLD_KARATSUBA {
        karatsuba::add_signed_mul(c, sign, a, b, memory)
    } else {
        toom_3::add_signed_mul(c, sign, a, b, memory)
    }
}

/// c += sign * a * b
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

    if n <= THRESHOLD_SIMPLE {
        simple::add_signed_mul_same_len(c, sign, a, b, memory)
    } else if n <= THRESHOLD_KARATSUBA {
        karatsuba::add_signed_mul_same_len(c, sign, a, b, memory)
    } else {
        toom_3::add_signed_mul_same_len(c, sign, a, b, memory)
    }
}
