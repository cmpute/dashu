use core::{cmp::Ordering, mem, ptr, slice};
use alloc::alloc::Layout;
use dashu_base::{Gcd, ExtendedGcd};

use crate::{
    arch::word::{Word, DoubleWord, SignedWord, SignedDoubleWord},
    cmp::cmp_in_place,
    div, shift, mul::{self, add_mul_word_in_place},
    memory::{Memory, self},
    primitive::{highest_dword, extend_word, split_dword, signed_extend_word, split_signed_dword, WORD_BITS},
    bits::trim_leading_zeros,
    sign::Sign,
};

/// Temporary memory required for extended gcd.
pub fn memory_requirement_ext_up_to(lhs_len: usize, rhs_len: usize) -> Layout {
    // Required memory:
    // - two numbers (s0 & s1) with at most the same size as rhs + 1 buffer word
    // - two numbers (t0 & t1) with at most the same size as lhs + 1 buffer word
    // - temporary space for division (this should cover the space for a multiplication later)
    // TODO: check the exact requirement determine this
    let num_words = 2 * (lhs_len + rhs_len) + 4;
    memory::add_layout(
        memory::array_layout::<Word>(num_words),
        div::memory_requirement_exact(lhs_len, rhs_len),
    )
}

/// Extended binary GCD for two multi-digits numbers
pub fn gcd_ext_in_place(
    lhs: &mut [Word],
    rhs: &mut [Word],
    g: &mut [Word],
    bonly: bool,
    memory: &mut Memory,
) -> (usize, bool) {
    let (lhs_len, rhs_len) = (lhs.len(), rhs.len());
    let (lhs_ptr, rhs_ptr) = (lhs.as_mut_ptr(), rhs.as_mut_ptr());

    // keep x = s0*lhs - t0*rhs, y = t1*rhs - s1*lhs, gcd(x, y) = gcd(lhs, rhs)
    let (mut s0, mut memory) = memory.allocate_slice_fill::<Word>(rhs_len + 1, 0);
    let (mut s1, mut memory) = memory.allocate_slice_fill::<Word>(rhs_len + 1, 0);
    let (mut t0, mut memory) = memory.allocate_slice_fill::<Word>(lhs_len + 1, 0);
    let (mut t1, mut memory) = memory.allocate_slice_fill::<Word>(lhs_len + 1, 0);
    let (mut s0_len, mut s1_len) = (1, 1);
    let (mut t0_len, mut t1_len) = (1, 1);
    *s0.first_mut().unwrap() = 1;
    *t1.first_mut().unwrap() = 1;

    let mut swapped = match cmp_in_place(lhs, rhs) {
        Ordering::Equal => {
            // TODO: remove fill by returning a length as well
            // TODO: assert lhs != rhs, because this trivial cause should be pre-handled such that allocation can be avoided
            g.copy_from_slice(lhs);
            lhs[1..].fill(0);
            rhs[1..].fill(0);
            *rhs.first_mut().unwrap() = 1;
            *lhs.first_mut().unwrap() = 0;
            return (g.len(), false)
        },
        Ordering::Greater => false,
        Ordering::Less => true
    };

    // keep x >= y though the algorithm, and track the source of x and y
    let (mut x, mut y) = (lhs, rhs);
    if swapped {
        mem::swap(&mut x, &mut y);
        mem::swap(&mut s0, &mut s1);
        mem::swap(&mut t0, &mut t1);
    }

    while y.len() > 1 {
        // do a euclidean step (x, y) = (y, x % y)
        let (shift, q_top) = div::div_rem_unnormalized_in_place(x, y, &mut memory);
        let (mut r, q_lo) = x.split_at_mut(y.len());
        let y_low_bits = shift::shr_in_place(y, shift);
        let r_low_bits = shift::shr_in_place(r, shift);
        debug_assert!(y_low_bits | r_low_bits == 0);

        // s0 += q*s1, t0 += q*t1
        s0_len = q_lo.len() + s1_len;
        t0_len = q_lo.len() + t1_len;
        let mut s_carry = mul::add_signed_mul(&mut s0[..s0_len], Sign::Positive, q_lo, &s1[..s1_len], &mut memory) as Word;
        let mut t_carry = mul::add_signed_mul(&mut t0[..t0_len], Sign::Positive, q_lo, &t1[..t1_len], &mut memory) as Word;
        if q_top > 0 {
            s_carry += mul::add_mul_word_in_place(&mut s0[q_lo.len()..q_lo.len() + s0_len], q_top, &s1[..s1_len]);
            t_carry += mul::add_mul_word_in_place(&mut t0[q_lo.len()..q_lo.len() + t0_len], q_top, &t1[..t1_len]);
        }
        if s_carry > 0 {
            s0[s0_len] = s_carry;
            s0_len += 1;
        }
        if t_carry > 0 {
            t0[t0_len] = t_carry;
            t0_len += 1;
        }

        // Trim leading zero and swap
        r = trim_leading_zeros(r);
        x = mem::replace(&mut y, r);
        mem::swap(&mut s0, &mut s1);
        mem::swap(&mut t0, &mut t1);
        mem::swap(&mut s0_len, &mut s1_len);
        mem::swap(&mut t0_len, &mut t1_len);
        swapped = !swapped;
    }

    // If y is zero, then the gcd result is in x now.
    if y.len() == 0 {
        let len = x.len().min(g.len());
        g[..len].copy_from_slice(&x[..len]);
        unsafe {
            ptr::copy_nonoverlapping(t0.as_ptr(), lhs_ptr, t0_len);
            ptr::copy_nonoverlapping(s0.as_ptr(), rhs_ptr, s0_len);
            // TODO: prevent filling here
            slice::from_raw_parts_mut(lhs_ptr.add(t0_len), lhs_len - t0_len).fill(0);
            slice::from_raw_parts_mut(rhs_ptr.add(s0_len), rhs_len - s0_len).fill(0);
        }
        return (g.len(), swapped)
    }

    // forward to single word gcd, first reduce x by y:
    // x_word = x % y; x /= y
    let y_word = *y.first().unwrap();
    let x_word = div::div_by_word_in_place(x, y_word);
    s0_len = x.len() + s1_len;
    t0_len = x.len() + t1_len;
    let s_carry = mul::add_signed_mul(&mut s0[..s0_len], Sign::Positive, x, &s1[..s1_len], &mut memory);
    let t_carry = mul::add_signed_mul(&mut t0[..t0_len], Sign::Positive, x, &t1[..t1_len], &mut memory);
    debug_assert!(s_carry | t_carry == 0);

    // reconstruct the slice from pointers
    let (lhs, rhs) = unsafe {
        // SAFETY: we don't hold any reference to lhs and rhs now, so there will be no
        // data racing. The pointer and length are from the original slice, so the slice
        // will be valid.
        (slice::from_raw_parts_mut(lhs_ptr, lhs_len),
        slice::from_raw_parts_mut(rhs_ptr, rhs_len))
    };
    lhs.fill(0);
    rhs.fill(0);
    let (g_word, cx, cy) = x_word.gcd_ext(y_word);
    swapped ^= cx < 0;
    let (cx, cy) = (cx.unsigned_abs(), cy.unsigned_abs());
    let carry1 = add_mul_word_in_place(rhs, cx, &s0[..s0.len() - 1]);
    let carry2 = add_mul_word_in_place(rhs, cy, &s1[..s1.len() - 1]);
    debug_assert!(carry1 | carry2 == 0);
    let carry1 = add_mul_word_in_place(lhs, cx, &t0[..t0.len() - 1]);
    let carry2 = add_mul_word_in_place(lhs, cy, &t1[..t1.len() - 1]);
    debug_assert!(carry1 | carry2 == 0);
    g[0] = g_word;
    (1, swapped)

    // TODO: delegate to double word gcd once we have add_mul_dword_in_place
}