//! Simple multiplication algorithm.

use crate::{
    add,
    arch::word::{SignedWord, Word},
    math,
    memory::Memory,
    mul::helpers,
    primitive::{double_word, extend_word, split_dword},
    Sign::{self, *},
};

/// Split larger length into chunks of CHUNK_LEN..2 * CHUNK_LEN for memory locality.
const CHUNK_LEN: usize = 1024;

/// Max supported Smaller factor length.
pub const MAX_SMALLER_LEN: usize = CHUNK_LEN;

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
/// It consumes two multiplier words per sweep over `rhs`, so the accumulator
/// word `words[k]` is loaded and stored once per two multiplier words instead
/// of once per word. This halves the memory traffic on `words` and exposes two
/// independent multiply chains.
///
/// Only `words[..n]` is modified. The two extra high words of the product
/// (the carries out of columns `n` and `n + 1`) are returned as
/// `(carry_lo, carry_hi)`, to be accumulated by the caller at `words[n]`
/// and `words[n + 1]`.
#[inline]
pub fn add_mul_dword_same_len_in_place(
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

    // Consume the multiplier two words at a time via the add_mul_dword_same_len_in_place kernel.
    let mut pairs = b.chunks_exact(2);
    for pair in &mut pairs {
        let (carry_lo, carry_hi) =
            add_mul_dword_same_len_in_place(&mut c[i..i + n], a, pair[0], pair[1]);
        overflow |= add::add_dword_in_place(&mut c[i + n..], double_word(carry_lo, carry_hi));
        i += 2;
    }

    // Handle the leftover odd multiplier word with a plain add_mul_word_same_len_in_place.
    if let &[m] = pairs.remainder() {
        let carry_word = add_mul_word_same_len_in_place(&mut c[i..i + n], m, a);
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

    // Consume the multiplier two words at a time via the sub_mul_dword_same_len_in_place kernel.
    let mut pairs = b.chunks_exact(2);
    for pair in &mut pairs {
        let (carry_lo, carry_hi) =
            sub_mul_dword_same_len_in_place(&mut c[i..i + n], a, pair[0], pair[1]);
        borrow_out |= add::sub_dword_in_place(&mut c[i + n..], double_word(carry_lo, carry_hi));
        i += 2;
    }

    // Handle the leftover odd multiplier word with a plain submul_1.
    if let &[m] = pairs.remainder() {
        let borrow_word = sub_mul_word_same_len_in_place(&mut c[i..i + n], m, a);
        borrow_out |= add::sub_word_in_place(&mut c[i + n..], borrow_word);
    }

    borrow_out
}
