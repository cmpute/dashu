use alloc::alloc::Layout;
use core::{cmp::Ordering, mem, ptr, slice};
use dashu_base::{ExtendedGcd, Gcd};

use crate::{
    arch::word::{DoubleWord, SignedDoubleWord, SignedWord, Word},
    bits::locate_top_word_plus_one,
    cmp::cmp_in_place,
    div,
    memory::{self, Memory},
    mul::{self, add_mul_word_in_place},
    primitive::{
        extend_word, highest_dword, signed_extend_word, split_dword, split_signed_dword, WORD_BITS,
    },
    shift,
    sign::Sign,
};

#[inline]
fn trim_leading_zeros(words: &mut [Word]) -> &mut [Word] {
    words.split_at_mut(locate_top_word_plus_one(words)).0
}

/// Temporary memory required for extended gcd.
pub fn memory_requirement_ext_up_to(lhs_len: usize, rhs_len: usize) -> Layout {
    // Required memory:
    // - two numbers (t0 & t1) with at most the same size as lhs, add 1 buffer word
    // - temporary space for a division (for euclidean step), and later a mulitplication (for coeff update)
    let t_words = 2 * lhs_len + 2;
    memory::add_layout(
        memory::array_layout::<Word>(t_words),
        memory::max_layout(
            div::memory_requirement_exact(lhs_len, rhs_len), //
            mul::memory_requirement_up_to(lhs_len, lhs_len / 2), // for coeff update
        ),
    )
}

/// Extended binary GCD for two multi-digits numbers
pub fn gcd_ext_in_place(
    lhs: &mut [Word],
    rhs: &mut [Word],
    memory: &mut Memory,
) -> (usize, usize, Sign) {
    let (lhs_len, rhs_len) = (lhs.len(), rhs.len());
    let (lhs_ptr, rhs_ptr) = (lhs.as_mut_ptr(), rhs.as_mut_ptr());

    // keep x >= y though the algorithm, and track the source of x and y using the swapped flag
    debug_assert!(cmp_in_place(lhs, rhs).is_gt());
    let (mut x, mut y) = (lhs, rhs);
    let mut swapped = false;

    // the normal way is to have four variables s0, s1, t0, t1 and keep gcd(x, y) = gcd(lhs, rhs),
    // x = s0*lhs - t0*rhs, y = t1*rhs - s1*lhs. Here we simplify it by only tracking the
    // coefficient of rhs, so that x = -t0*rhs mod lhs, y = t1*rhs mod lhs,
    let (mut t0, mut memory) = memory.allocate_slice_fill::<Word>(lhs_len + 1, 0);
    let (mut t1, mut memory) = memory.allocate_slice_fill::<Word>(lhs_len + 1, 0);
    let (mut t0_len, mut t1_len) = (1, 1);
    *t1.first_mut().unwrap() = 1;

    // loop, reduce x, y until the smaller one (y) fits in a single word
    while y.len() > 1 {
        // do a euclidean step (x, y) = (y, x % y)
        let (shift, q_top) = div::div_rem_unnormalized_in_place(x, y, &mut memory);
        let (mut r, q_lo) = x.split_at_mut(y.len());
        let y_low_bits = shift::shr_in_place(y, shift);
        let r_low_bits = shift::shr_in_place(r, shift);
        debug_assert!(y_low_bits | r_low_bits == 0);
        r = trim_leading_zeros(r);

        // t0 += q*t1
        t0_len = q_lo.len() + t1_len;
        let mut t_carry = mul::add_signed_mul(
            &mut t0[..t0_len],
            Sign::Positive,
            q_lo,
            &t1[..t1_len],
            &mut memory,
        ) as Word;
        if q_top > 0 {
            t_carry += mul::add_mul_word_in_place(
                &mut t0[q_lo.len()..q_lo.len() + t0_len],
                q_top,
                &t1[..t1_len],
            );
        }
        if t_carry > 0 {
            t0[t0_len] = t_carry;
            t0_len += 1;
        } else {
            t0_len = locate_top_word_plus_one(&t0[..t0_len]);
        }

        // swap: (x, y) = (y, r)
        x = mem::replace(&mut y, r);
        mem::swap(&mut t0, &mut t1);
        mem::swap(&mut t0_len, &mut t1_len);
        swapped = !swapped;
    }

    // If y is zero, then the gcd result is in x now.
    // Note that y.len() == 0 is equivalent to y == 0, which is guaranteed by trim_leading_zeros.
    if y.len() == 0 {
        unsafe {
            if !swapped {
                // if not swapped, then x is originated from lhs, copy it to rhs
                debug_assert!(x.as_ptr() == lhs_ptr);
                debug_assert!(x.len() <= rhs_len);
                ptr::copy_nonoverlapping(x.as_ptr(), rhs_ptr, x.len());
            }
            ptr::copy_nonoverlapping(t0.as_ptr(), lhs_ptr, t0_len);
        }
        let sign = if swapped {
            Sign::Positive
        } else {
            Sign::Negative
        };
        return (x.len(), t0_len, sign);
    }

    // before forwarding to single word gcd, first reduce x by y:
    // x_word = x % y; x /= y
    let y_word = *y.first().unwrap();
    let x_word = div::div_by_word_in_place(x, y_word);
    t0_len = x.len() + t1_len;
    let t_carry = mul::add_signed_mul(
        &mut t0[..t0_len],
        Sign::Positive,
        x,
        &t1[..t1_len],
        &mut memory,
    );
    debug_assert!(t_carry == 0);
    t0_len = locate_top_word_plus_one(&t0[..t0_len]);

    // forward to single word gcd
    let (g_word, cx, cy) = x_word.gcd_ext(y_word);
    swapped ^= cx < 0;

    // let lhs stores |b| = |cx| * t0 + |cy| * t1
    // by now, number of words in |b| should be close to lhs
    let (lhs, rhs) = unsafe {
        // SAFETY: we don't hold any reference to lhs and rhs now, so there will be no
        // data racing. The pointer and length are from the original slice, so the slice
        // will be valid.
        (
            slice::from_raw_parts_mut(lhs_ptr, lhs_len),
            slice::from_raw_parts_mut(rhs_ptr, rhs_len),
        )
    };
    *rhs.first_mut().unwrap() = g_word;
    lhs.fill(0);

    let (cx, cy) = (cx.unsigned_abs(), cy.unsigned_abs());
    let carry1 = add_mul_word_in_place(lhs, cx, &t0[..t0_len]);
    let carry2 = add_mul_word_in_place(lhs, cy, &t1[..t1_len]);
    debug_assert!(carry1 | carry2 == 0);
    let sign = if swapped {
        Sign::Positive
    } else {
        Sign::Negative
    };
    (1, locate_top_word_plus_one(&lhs), sign)
}
