//! Toom-Cook-4 squaring algorithm.

use crate::{
    add,
    arch::word::{SignedWord, Word},
    div,
    helper_macros::debug_assert_zero,
    math,
    memory::{self, Memory},
    mul, shift, sqr,
    Sign::*,
};
use alloc::alloc::Layout;

/// Minimum supported length.
pub const MIN_LEN: usize = 64;

/// Temporary memory required for squaring.
///
/// n bounds the operand length in words.
pub fn memory_requirement_up_to(n: usize) -> Layout {
    // Same structure as the Toom-4 multiplication formula: the evaluation
    // phase needs fewer buffers (one operand side), but the interpolation
    // and placement are identical, so we keep the conservative bound.
    let num_words = 12 * n + 24 * (math::ceil_log2(n) as usize) + 256;
    memory::array_layout::<Word>(num_words)
}

/// b = a². b must be filled with zeros. n >= MIN_LEN.
///
/// Evaluates a(x) = a3*x^3 + a2*x^2 + a1*x + a0 at 0, 1, -1, 2, -2, 1/2 and
/// infinity, squares each evaluation, then interpolates via the same
/// formulas as Toom-4 multiplication (all point values are squares, hence
/// non-negative).
pub fn square(b: &mut [Word], a: &[Word], memory: &mut Memory) {
    let n = a.len();
    debug_assert!(n >= MIN_LEN && b.len() == 2 * n);

    // Split into 4 parts. a3 may be shorter.
    let n4 = (n + 3) / 4;
    let s = n - 3 * n4;

    let (a0, a123) = a.split_at(n4);
    let (a1, a23) = a123.split_at(n4);
    let (a2, a3) = a23.split_at(n4);

    let mut carry: SignedWord = 0;
    let mut carry_c0: SignedWord = 0; // at 2*n4
    let mut carry_c1: SignedWord = 0; // at 3*n4+2
    let mut carry_c2: SignedWord = 0; // at 4*n4+2
    let mut carry_c3: SignedWord = 0; // at 5*n4+2
    let mut carry_c4: SignedWord = 0; // at 6*n4+2
    let mut carry_c5: SignedWord = 0; // at 2*n

    // Evaluate at 0: V(0) = sqr(a0) (z0).
    let (v0, mut memory) = memory.allocate_slice_fill(2 * n4, 0);
    sqr::sqr(&mut v0[..], a0, &mut memory);

    // Evaluate at inf: V(inf) = sqr(a3) (z6).
    let (vinf, mut memory) = memory.allocate_slice_fill(2 * s, 0);
    sqr::sqr(&mut vinf[..], a3, &mut memory);

    let (pv2, mut memory) = memory.allocate_slice_fill(2 * n4 + 2, 0); // -> e2, then z4
    let (pv1, mut memory) = memory.allocate_slice_fill(2 * n4 + 2, 0); // -> e1, then z2
    let (pvh, mut memory) = memory.allocate_slice_fill(2 * n4 + 2, 0); // -> r
    let (o1, mut memory) = memory.allocate_slice_fill(2 * n4 + 2, 0); // -> o1, then z1
    let (o2, mut memory) = memory.allocate_slice_fill(2 * n4 + 2, 0); // -> o2, then t3, z3
    let (z5, mut memory) = memory.allocate_slice_fill(2 * n4 + 2, 0);

    // Evaluate at 2 and -2 via u = a0 + 4*a2, v = a1 + 4*a3:
    // A(2) = u + 2*v, |A(-2)| = |u - 2*v|.
    {
        let (u, mut memory) = memory.allocate_slice_copy_fill(n4 + 1, a0, 0);
        let (v, mut memory) = memory.allocate_slice_copy_fill(n4 + 1, a1, 0);
        u[n4] = mul::add_mul_word_same_len_in_place(&mut u[..n4], 4, a2);
        v[n4] = mul::add_mul_word_in_place(&mut v[..n4], 4, a3);
        let (a2p, mut memory) = memory.allocate_slice_copy_fill(n4 + 1, u, 0);
        debug_assert_zero!(mul::add_mul_word_same_len_in_place(&mut a2p[..], 2, v));
        // |A(-2)| = |u - 2v| = |2u - A(2)|: reuse A(2) instead of
        // materializing 2*v. The sign is irrelevant, the value is squared.
        let (a2m, mut memory) = memory.allocate_slice_copy_fill(n4 + 1, u, 0);
        debug_assert_zero!(mul::mul_word_in_place(&mut a2m[..], 2));
        let _sign = add::sub_in_place_with_sign(&mut a2m[..], a2p);
        sqr::sqr(&mut pv2[..], a2p, &mut memory);
        {
            let (pvm2, mut mem) = memory.allocate_slice_fill(2 * n4 + 2, 0);
            sqr::sqr(&mut pvm2[..], a2m, &mut mem);
            // o2 = ((V(2) - V(-2))/2)/2, pv2 -> e2.
            o2.copy_from_slice(pv2);
            debug_assert_zero!(add::sub_in_place(&mut o2[..], pvm2));
            debug_assert_zero!(shift::shr_in_place(&mut o2[..], 2));
            debug_assert_zero!(add::add_signed_in_place(&mut pv2[..], Positive, pvm2));
            debug_assert_zero!(shift::shr_in_place(&mut pv2[..], 1));
        }
        debug_assert_zero!(add::sub_in_place(&mut pv2[..], v0));
        {
            let borrow = mul::sub_mul_word_same_len_in_place(&mut pv2[..2 * s], 64, vinf);
            debug_assert!(borrow < Word::MAX);
            debug_assert_zero!(add::add_signed_word_in_place(
                &mut pv2[2 * s..],
                -(borrow as SignedWord)
            ));
        }
        debug_assert_zero!(shift::shr_in_place(&mut pv2[..], 2));
    }

    // Evaluate at 1 and -1 via a02 = a0 + a2, a13 = a1 + a3.
    {
        let (a02, mut memory) = memory.allocate_slice_copy_fill(n4 + 1, a0, 0);
        a02[n4] = Word::from(add::add_in_place(&mut a02[..n4], a2));
        let (a13, mut memory) = memory.allocate_slice_copy_fill(n4 + 1, a1, 0);
        a13[n4] = Word::from(add::add_in_place(&mut a13[..n4], a3));
        let (a1p, mut memory) = memory.allocate_slice_copy_fill(n4 + 1, a02, 0);
        debug_assert_zero!(add::add_same_len_in_place(&mut a1p[..], a13));
        let (a1m, mut memory) = memory.allocate_slice_copy_fill(n4 + 1, a02, 0);
        if add::sub_in_place_with_sign(&mut a1m[..], a13) == Negative {
            a1m.copy_from_slice(a13);
            debug_assert_zero!(add::sub_in_place(&mut a1m[..], a02));
        }
        sqr::sqr(&mut pv1[..], a1p, &mut memory);
        {
            let (pvm1, mut mem) = memory.allocate_slice_fill(2 * n4 + 2, 0);
            sqr::sqr(&mut pvm1[..], a1m, &mut mem);
            // o1 = (V(1) - V(-1))/2, pv1 -> e1.
            o1.copy_from_slice(pv1);
            debug_assert_zero!(add::sub_in_place(&mut o1[..], pvm1));
            debug_assert_zero!(shift::shr_in_place(&mut o1[..], 1));
        }
        debug_assert_zero!(add::sub_in_place(&mut pv1[..], o1));
        debug_assert_zero!(add::sub_in_place(&mut pv1[..], v0));
        debug_assert_zero!(add::sub_in_place(&mut pv1[..], vinf));
    }

    // Evaluate at 1/2: ah = 8*a0 + 4*a1 + 2*a2 + a3 = 64 * A(1/2).
    {
        let (ah, mut memory) = memory.allocate_slice_copy_fill(n4 + 1, a0, 0);
        ah[n4] = mul::mul_word_in_place(&mut ah[..n4], 8);
        ah[n4] += mul::add_mul_word_same_len_in_place(&mut ah[..n4], 4, a1);
        ah[n4] += mul::add_mul_word_in_place(&mut ah[..n4], 2, a2);
        ah[n4] += mul::add_mul_word_in_place(&mut ah[..n4], 1, a3);
        sqr::sqr(&mut pvh[..], ah, &mut memory);
    }

    // z4 = (e2 - e1)/3 (in pv2), z2 = e1 - z4 (in pv1).
    debug_assert_zero!(add::sub_in_place(&mut pv2[..], pv1));
    debug_assert_zero!(div::div_by_word_in_place(&mut pv2[..], 3));
    debug_assert_zero!(add::sub_in_place(&mut pv1[..], pv2));

    // r = (Vh - 64*z0 - z6 - 16*z2 - 4*z4)/2 = 16*z1 + 4*z3 + z5, in pvh.
    {
        let borrow = mul::sub_mul_word_same_len_in_place(&mut pvh[..2 * n4], 64, v0);
        debug_assert!(borrow < Word::MAX);
        debug_assert_zero!(add::add_signed_word_in_place(
            &mut pvh[2 * n4..],
            -(borrow as SignedWord)
        ));
    }
    debug_assert_zero!(add::sub_in_place(&mut pvh[..], vinf));
    {
        debug_assert_zero!(mul::sub_mul_word_same_len_in_place(&mut pvh[..], 16, pv1));
    }
    {
        debug_assert_zero!(mul::sub_mul_word_same_len_in_place(&mut pvh[..], 4, pv2));
    }
    debug_assert_zero!(shift::shr_in_place(&mut pvh[..], 1));

    // t3 = (o2 - o1)/3 = z3 + 5*z5 (in o2).
    debug_assert_zero!(add::sub_in_place(&mut o2[..], o1));
    debug_assert_zero!(div::div_by_word_in_place(&mut o2[..], 3));

    // z5 = (r + 12*t3 - 16*o1)/45.
    z5.copy_from_slice(o2);
    debug_assert_zero!(mul::mul_word_in_place(&mut z5[..], 12));
    debug_assert_zero!(add::add_signed_in_place(&mut z5[..], Positive, pvh));
    debug_assert_zero!(mul::sub_mul_word_same_len_in_place(&mut z5[..], 16, o1));
    debug_assert_zero!(div::div_by_word_in_place(&mut z5[..], 45));

    // z3 = t3 - 5*z5 (in o2), z1 = o1 - z3 - z5 (in o1).
    debug_assert_zero!(mul::sub_mul_word_same_len_in_place(&mut o2[..], 5, z5));
    debug_assert_zero!(add::sub_in_place(&mut o1[..], o2));
    debug_assert_zero!(add::sub_in_place(&mut o1[..], z5));

    // ---- Placement into b ----
    // z0 = v0, z1 = o1, z2 = pv1, z3 = o2, z4 = pv2, z5 = z5, z6 = vinf.
    carry_c0 += add::add_signed_same_len_in_place(&mut b[..2 * n4], Positive, v0);
    carry_c1 += add::add_signed_same_len_in_place(&mut b[n4..3 * n4 + 2], Positive, o1);
    carry_c2 += add::add_signed_same_len_in_place(&mut b[2 * n4..4 * n4 + 2], Positive, pv1);
    carry_c3 += add::add_signed_same_len_in_place(&mut b[3 * n4..5 * n4 + 2], Positive, o2);
    carry_c4 += add::add_signed_same_len_in_place(&mut b[4 * n4..6 * n4 + 2], Positive, pv2);
    carry_c5 += add::add_signed_in_place(&mut b[5 * n4..], Positive, &z5[..n4 + s + 1]);
    carry += add::add_signed_in_place(&mut b[6 * n4..], Positive, vinf);

    // Apply carries.
    carry_c1 += add::add_signed_word_in_place(&mut b[2 * n4..3 * n4 + 2], carry_c0);
    carry_c2 += add::add_signed_word_in_place(&mut b[3 * n4 + 2..4 * n4 + 2], carry_c1);
    carry_c3 += add::add_signed_word_in_place(&mut b[4 * n4 + 2..5 * n4 + 2], carry_c2);
    carry_c4 += add::add_signed_word_in_place(&mut b[5 * n4 + 2..6 * n4 + 2], carry_c3);
    carry_c5 += add::add_signed_word_in_place(&mut b[6 * n4 + 2..], carry_c4);
    carry += carry_c5;

    debug_assert!(carry.abs() <= 1);
}
