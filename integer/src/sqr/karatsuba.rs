//! Karatsuba squaring algorithm.

use crate::{
    add,
    arch::word::{SignedWord, Word},
    helper_macros::debug_assert_zero,
    math,
    memory::{self, Memory},
    sqr,
    Sign::*,
};
use alloc::alloc::Layout;

// Same constraint as Karatsuba multiplication: 3 * floor((n+1)/2) <= 2n for n >= 3.
/// Minimum supported length.
pub const MIN_LEN: usize = 3;

/// Temporary memory required for squaring.
///
/// n bounds the operand length in words.
pub fn memory_requirement_up_to(n: usize) -> Layout {
    // Same formula as Karatsuba multiplication — conservative upper bound.
    let num_words = 2 * n + 2 * (math::ceil_log2(n) as usize);
    memory::array_layout::<Word>(num_words)
}

/// b = a², b must be filled with zeros. n >= MIN_LEN.
///
/// a² = a_lo² + (a_lo² + a_hi² − (a_lo−a_hi)²)·B^mid + a_hi²·B^(2·mid)
pub fn square(b: &mut [Word], a: &[Word], memory: &mut Memory) {
    let n = a.len();
    debug_assert!(n >= MIN_LEN && b.len() == 2 * n);

    let mid = (n + 1) / 2;
    let (a_lo, a_hi) = a.split_at(mid);

    let mut carry: SignedWord = 0;
    let mut carry_c0: SignedWord = 0; // at 2*mid
    let mut carry_c1: SignedWord = 0; // at 3*mid

    {
        // P0 = sqr(a_lo)
        let (p0, mut mem) = memory.allocate_slice_fill::<Word>(2 * mid, 0);
        sqr::sqr(p0, a_lo, &mut mem);
        carry_c0 += add::add_signed_same_len_in_place(&mut b[..2 * mid], Positive, p0);
        carry_c1 += add::add_signed_same_len_in_place(&mut b[mid..3 * mid], Positive, p0);
    }
    {
        // P2 = sqr(a_hi)
        let p2_len = 2 * (n - mid);
        let (p2, mut mem) = memory.allocate_slice_fill::<Word>(p2_len, 0);
        sqr::sqr(p2, a_hi, &mut mem);
        carry += add::add_signed_same_len_in_place(&mut b[2 * mid..], Positive, p2);
        carry_c1 += add::add_signed_in_place(&mut b[mid..3 * mid], Positive, p2);
    }
    {
        // diff_sq = (|a_lo − a_hi|)²   (always non-negative)
        let (diff, mut mem) = memory.allocate_slice_copy(a_lo);
        let diff_sign = add::sub_in_place_with_sign(diff, a_hi);
        if diff_sign == Negative {
            // |a_lo − a_hi| = a_hi − a_lo
            diff[..(n - mid)].copy_from_slice(a_hi);
            diff[(n - mid)..].fill(0);
            debug_assert_zero!(add::sub_in_place(diff, a_lo));
        }
        let (diff_sq, mut mem) = mem.allocate_slice_fill::<Word>(2 * mid, 0);
        sqr::sqr(diff_sq, diff, &mut mem);
        // c[mid..3*mid] -= diff_sq  (always subtract, diff_sq is non-negative)
        carry_c1 += add::add_signed_same_len_in_place(&mut b[mid..3 * mid], Negative, diff_sq);
    }

    // Propagate carries.
    carry_c1 += add::add_signed_word_in_place(&mut b[2 * mid..3 * mid], carry_c0);
    carry += add::add_signed_word_in_place(&mut b[3 * mid..], carry_c1);

    debug_assert!(carry.abs() <= 1);
}
