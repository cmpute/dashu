//! Toom-Cook-3 squaring algorithm.

use crate::{
    add,
    arch::word::{SignedWord, Word},
    div,
    helper_macros::debug_assert_zero,
    math,
    memory::{self, Memory},
    mul,
    shift,
    sqr,
    Sign::*,
};
use alloc::alloc::Layout;

/// Minimum supported length.
pub const MIN_LEN: usize = 16;

/// Temporary memory required for squaring.
///
/// n bounds the operand length in words.
pub fn memory_requirement_up_to(n: usize) -> Layout {
    // f(n) = 6*(ceil(n/3)+1) + f(ceil(n/3)+1), proves to <= 3n + 10 ceil_log2 n.
    let num_words = 3 * n + 10 * (math::ceil_log2(n) as usize);
    memory::array_layout::<Word>(num_words)
}

/// b = a². b must be filled with zeros. n >= MIN_LEN.
///
/// Evaluates a(x) = a0 + a1·x + a2·x² at 0, 1, −1, 2, ∞, squares
/// each evaluation, then interpolates via the same formulas as Toom-3
/// multiplication.
pub fn square(b: &mut [Word], a: &[Word], memory: &mut Memory) {
    let n = a.len();
    debug_assert!(n >= MIN_LEN && b.len() == 2 * n);

    // Split into 3 parts. a2 may be shorter.
    let n3 = (n + 2) / 3;
    let n3_short = n - 2 * n3;

    let (a0, a12) = a.split_at(n3);
    let (a1, a2) = a12.split_at(n3);

    let mut carry: SignedWord = 0;
    let mut carry_c0: SignedWord = 0; // at 2*n3
    let mut carry_c1: SignedWord = 0; // at 3*n3+2
    let mut carry_c2: SignedWord = 0; // at 4*n3+2
    let mut carry_c3: SignedWord = 0; // at 5*n3+2

    // Evaluate at 0: V(0) = sqr(a0).
    // c_0 += V(0), c_2 -= V(0), t1 = 3·V(0).
    let (t1, mut memory) = memory.allocate_slice_fill(2 * n3 + 2, 0);
    {
        let t1_short = &mut t1[..2 * n3];
        sqr::sqr(t1_short, a0, &mut memory);
        carry_c0 += add::add_signed_same_len_in_place(&mut b[..2 * n3], Positive, t1_short);
        carry_c2 += add::add_signed_in_place(&mut b[2 * n3..4 * n3 + 2], Negative, t1_short);
        t1[2 * n3] = mul::mul_word_in_place(t1_short, 3);
        t1[2 * n3 + 1] = 0;
    }

    // Evaluate at 2: a_eval = a0 + 2·a1 + 4·a2, V(2) = sqr(a_eval).
    // t1 += V(2).
    let (a_eval, mut memory) = memory.allocate_slice_copy_fill(n3 + 1, a0, 0);
    {
        a_eval[n3] = mul::add_mul_word_same_len_in_place(&mut a_eval[..n3], 2, a1);
        a_eval[n3] += mul::add_mul_word_in_place(&mut a_eval[..n3], 4, a2);
        // V(2) = sqr(a_eval), accumulate into t1.
        let (v2, mut mem) = memory.allocate_slice_fill(2 * (n3 + 1), 0);
        sqr::sqr(v2, a_eval, &mut mem);
        debug_assert_zero!(add::add_signed_same_len_in_place(t1, Positive, v2));
    }

    // Evaluate at ∞: V(∞) = sqr(a2).
    // c_2 -= V(∞), c_4 += V(∞), t1 -= 12·V(∞).
    // Now t1 = 3·V(0) + V(2) − 12·V(∞).
    {
        let (c_eval, mut memory) = memory.allocate_slice_fill(2 * n3 + 2, 0);
        let c_short = &mut c_eval[..2 * n3_short];
        sqr::sqr(c_short, a2, &mut memory);
        carry_c2 += add::add_signed_in_place(&mut b[2 * n3..4 * n3 + 2], Negative, c_short);
        carry += add::add_signed_same_len_in_place(&mut b[4 * n3..], Positive, c_short);
        c_eval[2 * n3_short] = mul::mul_word_in_place(c_short, 12);
        debug_assert_zero!(add::sub_in_place(t1, &c_eval[..2 * n3_short + 1]));
    }

    // Evaluate at 1 and −1.
    let (t2, mut memory) = memory.allocate_slice_fill(2 * n3 + 2, 0);
    {
        // a02 = a0 + a2
        let (a02, mut memory) = memory.allocate_slice_copy_fill(n3 + 1, a0, 0);
        a02[n3] = Word::from(add::add_in_place(&mut a02[..n3], a2));

        // V(1) = sqr(a02 + a1), store in t2.
        // c_1 += V(1).
        a_eval.copy_from_slice(a02);
        a_eval[n3] += Word::from(add::add_same_len_in_place(&mut a_eval[..n3], a1));
        sqr::sqr(t2, a_eval, &mut memory);
        carry_c1 += add::add_signed_in_place(&mut b[n3..3 * n3 + 2], Positive, t2);

        // V(−1) = sqr(|a02 − a1|).
        // Compute |a02 − a1| into a_eval.
        a_eval.copy_from_slice(a02);
        let neg_sign = add::sub_in_place_with_sign(a_eval, a1);
        if neg_sign == Negative {
            // |a02 − a1| = a1 − a02.
            a_eval[..n3].copy_from_slice(a1);
            a_eval[n3] = 0;
            debug_assert_zero!(add::sub_in_place(a_eval, a02));
        }
        // Exit block — a02 freed.
    }

    // V(−1) result into c_eval, then add to t2 and 2· to t1.
    let (c_eval, mut memory) = memory.allocate_slice_fill(2 * (n3 + 1), 0);
    sqr::sqr(c_eval, a_eval, &mut memory);

    // t2 += V(−1),   t1 += 2·V(−1).   Both always Positive.
    // Now: t1 = 3·V(0) + 2·V(−1) + V(2) − 12·V(∞),
    //      t2 = V(1) + V(−1).
    debug_assert_zero!(add::add_signed_same_len_in_place(t2, Positive, c_eval));
    debug_assert_zero!(mul::add_mul_word_same_len_in_place(t1, 2, c_eval));

    // Interpolate.
    let t1_rem = div::div_by_word_in_place(t1, 6);
    let t2_rem = shift::shr_in_place(t2, 1);
    assert_eq!(t1_rem, 0);
    assert_eq!(t2_rem, 0);

    // c_1 -= t1, c_3 += t1.
    // c_2 += t2, c_3 -= t2.
    carry_c1 += add::add_signed_same_len_in_place(&mut b[n3..3 * n3 + 2], Negative, t1);
    carry_c3 += add::add_signed_same_len_in_place(&mut b[3 * n3..5 * n3 + 2], Positive, t1);
    carry_c2 += add::add_signed_same_len_in_place(&mut b[2 * n3..4 * n3 + 2], Positive, t2);
    carry_c3 += add::add_signed_same_len_in_place(&mut b[3 * n3..5 * n3 + 2], Negative, t2);

    // Apply carries.
    carry_c1 += add::add_signed_word_in_place(&mut b[2 * n3..3 * n3 + 2], carry_c0);
    carry_c2 += add::add_signed_word_in_place(&mut b[3 * n3 + 2..4 * n3 + 2], carry_c1);
    carry_c3 += add::add_signed_word_in_place(&mut b[4 * n3 + 2..5 * n3 + 2], carry_c2);
    carry += add::add_signed_word_in_place(&mut b[5 * n3 + 2..], carry_c3);

    debug_assert!(carry.abs() <= 1);
}
