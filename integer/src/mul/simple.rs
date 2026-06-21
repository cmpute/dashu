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
///
/// On x86-64 with the `std` feature this dispatches at runtime to a BMI2 build
/// of the identical arithmetic; LLVM then lowers the widening multiplies to
/// `mulx` (which leaves the flags untouched) and unrolls the loop, without
/// changing the result. This lets a portable baseline binary use `mulx` on
/// modern CPUs. If the crate is itself built with BMI2 (e.g.
/// `-C target-cpu=native` or `-C target-cpu=x86-64-v3`), the portable build
/// already lowers to `mulx`, so the runtime check is skipped entirely.
#[inline]
pub fn add_mul_dword_same_len_in_place(
    words: &mut [Word],
    rhs: &[Word],
    mult0: Word,
    mult1: Word,
) -> (Word, Word) {
    // Crate already built with BMI2: `_impl` itself uses `mulx`, skip the check.
    #[cfg(target_feature = "bmi2")]
    {
        add_mul_dword_same_len_in_place_impl(words, rhs, mult0, mult1)
    }
    // Otherwise detect BMI2 at runtime so a baseline binary still uses `mulx`.
    #[cfg(not(target_feature = "bmi2"))]
    {
        #[cfg(all(target_arch = "x86_64", feature = "std"))]
        if std::is_x86_feature_detected!("bmi2") {
            // SAFETY: `bmi2` support was just confirmed at runtime.
            return unsafe { add_mul_dword_same_len_in_place_bmi2(words, rhs, mult0, mult1) };
        }
        add_mul_dword_same_len_in_place_impl(words, rhs, mult0, mult1)
    }
}

/// Shared body of [`add_mul_dword_same_len_in_place`] and its BMI2 build.
///
/// `#[inline(always)]` so the body is re-code-generated under each caller's
/// target features: the portable call site emits plain `mul`, while the BMI2
/// wrapper below emits `mulx`. Writing the arithmetic once guarantees the two
/// builds can never diverge.
#[inline(always)]
fn add_mul_dword_same_len_in_place_impl(
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

/// BMI2/`mulx` build of [`add_mul_dword_same_len_in_place_impl`].
///
/// Only called from the baseline dispatch path. When the crate is itself built
/// with BMI2 the dispatch short-circuits straight to `_impl` (which already uses
/// `mulx`), so this wrapper is then unused and dropped by the linker.
#[cfg(all(target_arch = "x86_64", feature = "std"))]
#[cfg_attr(target_feature = "bmi2", allow(dead_code))]
#[target_feature(enable = "bmi2")]
#[inline]
unsafe fn add_mul_dword_same_len_in_place_bmi2(
    words: &mut [Word],
    rhs: &[Word],
    mult0: Word,
    mult1: Word,
) -> (Word, Word) {
    add_mul_dword_same_len_in_place_impl(words, rhs, mult0, mult1)
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
    // Crate already built with BMI2: `_impl` itself uses `mulx`, skip the check.
    #[cfg(target_feature = "bmi2")]
    {
        sub_mul_dword_same_len_in_place_impl(words, rhs, mult0, mult1)
    }
    // Otherwise detect BMI2 at runtime so a baseline binary still uses `mulx`.
    #[cfg(not(target_feature = "bmi2"))]
    {
        #[cfg(all(target_arch = "x86_64", feature = "std"))]
        if std::is_x86_feature_detected!("bmi2") {
            // SAFETY: `bmi2` support was just confirmed at runtime.
            return unsafe { sub_mul_dword_same_len_in_place_bmi2(words, rhs, mult0, mult1) };
        }
        sub_mul_dword_same_len_in_place_impl(words, rhs, mult0, mult1)
    }
}

/// Shared body of [`sub_mul_dword_same_len_in_place`] and its BMI2 build; see
/// [`add_mul_dword_same_len_in_place_impl`] for why it is `#[inline(always)]`.
#[inline(always)]
fn sub_mul_dword_same_len_in_place_impl(
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

/// BMI2/`mulx` build of [`sub_mul_dword_same_len_in_place_impl`]; see
/// [`add_mul_dword_same_len_in_place_bmi2`] for the dead-code allowance.
#[cfg(all(target_arch = "x86_64", feature = "std"))]
#[cfg_attr(target_feature = "bmi2", allow(dead_code))]
#[target_feature(enable = "bmi2")]
#[inline]
unsafe fn sub_mul_dword_same_len_in_place_bmi2(
    words: &mut [Word],
    rhs: &[Word],
    mult0: Word,
    mult1: Word,
) -> (Word, Word) {
    sub_mul_dword_same_len_in_place_impl(words, rhs, mult0, mult1)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arch::word::Word;
    use alloc::vec::Vec;
    use rand_v08::{rngs::StdRng, Rng, SeedableRng};

    /// Independent reference for `words[..n] += rhs[..n] * (m0 + m1 * B)` (B = 2^Word::BITS),
    /// returning the full `n + 2` result limbs (low `n` are the updated words, the top two are
    /// the carry-out pair). Works for any `Word` width (16/32/64-bit).
    fn add_mul_dword_ref(words: &[Word], rhs: &[Word], m0: Word, m1: Word) -> Vec<Word> {
        let n = rhs.len();
        let bits = Word::BITS as usize;
        let lo = (1u128 << bits) - 1; // mask for the low `bits` of a limb
                                      // Accumulate in u128 (Word is at most u64, so two-word products fit with carry room).
        let mut col: Vec<u128> = (0..n + 2)
            .map(|i| if i < n { words[i] as u128 } else { 0 })
            .collect();
        for i in 0..n {
            let a = rhs[i] as u128 * m0 as u128;
            col[i] += a & lo;
            col[i + 1] += a >> bits;
            let b = rhs[i] as u128 * m1 as u128;
            col[i + 1] += b & lo;
            col[i + 2] += b >> bits;
        }
        let mut carry: u128 = 0;
        for v in col.iter_mut() {
            let s = *v + carry;
            carry = s >> bits; // the high part propagates to the next limb
            *v = s; // the low `bits` are extracted below
        }
        assert_eq!(carry, 0);
        col.iter().map(|&c| c as Word).collect()
    }

    #[test]
    fn add_mul_dword_matches_reference_and_bmi2() {
        let mut rng = StdRng::seed_from_u64(0x1234_5678_9abc_def0);
        for _ in 0..4000 {
            let n = rng.gen_range(0..7usize); // 0..=6
            let words: Vec<Word> = (0..n).map(|_| rng.gen()).collect();
            let rhs: Vec<Word> = (0..n).map(|_| rng.gen()).collect();
            let m0: Word = rng.gen();
            let m1: Word = rng.gen();

            let expected = add_mul_dword_ref(&words, &rhs, m0, m1);

            let mut w = words.clone();
            let (cl, ch) = add_mul_dword_same_len_in_place_impl(&mut w, &rhs, m0, m1);
            assert_eq!(&w[..], &expected[..n], "portable words mismatch");
            assert_eq!(cl, expected[n], "portable carry_lo mismatch");
            assert_eq!(ch, expected[n + 1], "portable carry_hi mismatch");

            #[cfg(all(target_arch = "x86_64", feature = "std"))]
            {
                if std::is_x86_feature_detected!("bmi2") {
                    let mut w2 = words.clone();
                    // SAFETY: bmi2 was just confirmed at runtime.
                    let (cl2, ch2) =
                        unsafe { add_mul_dword_same_len_in_place_bmi2(&mut w2, &rhs, m0, m1) };
                    assert_eq!(&w2[..], &expected[..n], "bmi2 words mismatch");
                    assert_eq!(cl2, expected[n], "bmi2 carry_lo mismatch");
                    assert_eq!(ch2, expected[n + 1], "bmi2 carry_hi mismatch");
                    assert_eq!(&w[..], &w2[..], "portable != bmi2 words");
                    assert_eq!((cl, ch), (cl2, ch2), "portable != bmi2 carries");
                }
            }
        }
    }

    #[test]
    fn sub_mul_dword_bmi2_matches_portable() {
        let mut rng = StdRng::seed_from_u64(0xdead_beef_cafe_babe);
        for _ in 0..4000 {
            let n = 1 + rng.gen_range(0..6usize); // 1..=6
            let words: Vec<Word> = (0..n).map(|_| rng.gen()).collect();
            let rhs: Vec<Word> = (0..n).map(|_| rng.gen()).collect();
            let m0: Word = rng.gen();
            let m1: Word = rng.gen();

            let mut w_p = words.clone();
            // The portable carries are only compared against the BMI2 kernel below; on
            // targets without that cfg there is nothing to check them against.
            #[cfg(all(target_arch = "x86_64", feature = "std"))]
            let (cl_p, ch_p) = sub_mul_dword_same_len_in_place_impl(&mut w_p, &rhs, m0, m1);
            #[cfg(not(all(target_arch = "x86_64", feature = "std")))]
            sub_mul_dword_same_len_in_place_impl(&mut w_p, &rhs, m0, m1);

            #[cfg(all(target_arch = "x86_64", feature = "std"))]
            {
                if std::is_x86_feature_detected!("bmi2") {
                    let mut w_b = words.clone();
                    // SAFETY: bmi2 was just confirmed at runtime.
                    let (cl_b, ch_b) =
                        unsafe { sub_mul_dword_same_len_in_place_bmi2(&mut w_b, &rhs, m0, m1) };
                    assert_eq!(&w_p[..], &w_b[..], "sub portable != bmi2 words");
                    assert_eq!((cl_p, ch_p), (cl_b, ch_b), "sub portable != bmi2 carries");
                }
            }
        }
    }
}
