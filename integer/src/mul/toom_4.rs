//! Toom-Cook-4 multiplication algorithm.

use crate::{
    add,
    arch::word::{SignedWord, Word},
    div,
    helper_macros::debug_assert_zero,
    math,
    memory::{self, Memory},
    mul::{self, helpers},
    shift,
    Sign::{self, *},
};
use alloc::alloc::Layout;

/* We must have:
 * n4 = ceil(n / 4), s = n - 3 * n4
 * 1 <= s <= n4, and n4 >= 16
 *
 * For n >= 64 (MIN_LEN): n4 = (n+3)/4 >= 16; s = n - 3*(n+3)/4 >= (n-9)/4 >= 13,
 * and s <= n - 3*n/4 = n/4 <= n4.
 */
/// Minimum supported length of the factors.
pub const MIN_LEN: usize = 64;

/// Temporary memory required for multiplication.
///
/// n bounds the length of the smaller factor in words.
pub fn memory_requirement_up_to(n: usize) -> Layout {
    /* Buffers that must stay alive across the recursive products (main
     * chain): V(0), V(inf) and six interpolation buffers, in words:
     *   2*n4 + 2*s + 6*(2*n4 + 2)                    = 14*n4 + 2*s + 12
     * The largest scoped phase (evaluation at ±2) allocates:
     *   10*(n4 + 1)                                  = 10*n4 + 10
     * Level peak (recursion happens inside the scoped phase):
     *   24*n4 + 2*s + 22
     *
     * Prove by induction that f(n) <= 12n + 24*ceil(log2 n) + 256.
     * Step (with n4 = ceil(n/4), s = n - 3*n4):
     * f(n) >= 24*n4 + 2*s + 22 + f(n4 + 1)
     *      >= 24*n4 + 2*s + 22 + 12*(n4+1) + 24*log2(n4+1) + 256
     * and 12n + 24*log2 n + 256 = 36*n4 + 12*s + 24*log2 n + 256, so the
     * step requires 12*n4 + 10*s + 24*log2 n >= 12*n4 + 12 + 24*log2(n4+1),
     * i.e. 10*s + 24*log2(n/(n4+1)) >= 12, true since s >= 13 and
     * n/(n4+1) <= 4. Base case f(n) >= 0 for n <= 64.
     *
     * The recurrence also holds when recursion transitions to the smaller
     * Toom-3/Karatsuba kernels, whose requirements are smaller.
     */
    let num_words = 12 * n + 24 * (math::ceil_log2(n) as usize) + 256;
    memory::array_layout::<Word>(num_words)
}

/// c += sign * a * b
/// Toom-Cook-4 method. O(a.len() * b.len()^0.41).
///
/// Returns carry.
#[must_use]
pub fn add_signed_mul(
    c: &mut [Word],
    sign: Sign,
    a: &[Word],
    b: &[Word],
    memory: &mut Memory,
) -> SignedWord {
    assert!(a.len() >= b.len() && b.len() >= MIN_LEN && c.len() == a.len() + b.len());

    helpers::add_signed_mul_split_into_chunks(
        c,
        sign,
        a,
        b,
        b.len(),
        memory,
        add_signed_mul_same_len,
    )
}

/// c += sign * a * b
/// Toom-Cook-4 method: O(n^1.40).
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
    debug_assert!(n >= MIN_LEN);

    /* We evaluate the polynomials A(x) = a3*x^3 + a2*x^2 + a1*x + a0 and
     * B(x) = b3*x^3 + b2*x^2 + b1*x + b0 at the points 0, 1, -1, 2, -2, 1/2
     * and infinity. Multiplying pointwise gives the values of
     * V(x) = A(x)*B(x) = z6*x^6 + ... + z1*x + z0 at the same points
     * (7 recursive multiplications), from which the coefficients are
     * recovered by interpolation:
     *
     * z0 = V(0),  z6 = V(inf)
     * e1 = (V(1) + V(-1))/2 - z0 - z6          = z2 + z4
     * e2 = ((V(2) + V(-2))/2 - z0 - 64*z6)/4   = z2 + 4*z4
     * o1 = (V(1) - V(-1))/2                    = z1 + z3 + z5
     * o2 = ((V(2) - V(-2))/2)/2                = z1 + 4*z3 + 16*z5
     * t3 = (o2 - o1)/3                         = z3 + 5*z5
     * r  = (Vh - 64*z0 - z6 - 16*z2 - 4*z4)/2  = 16*z1 + 4*z3 + z5
     *      where Vh = 64 * V(1/2): the evaluation at 1/2 is scaled by 64
     *      (Xh = 8*x0 + 4*x1 + 2*x2 + x3) so that it stays in integers
     * z5 = (r + 12*t3 - 16*o1) / 45
     * z4 = (e2 - e1) / 3,   z2 = e1 - z4
     * z3 = t3 - 5*z5,       z1 = o1 - z3 - z5
     *
     * All divisions are exact, and every intermediate value is provably
     * non-negative with the orderings used below.
     */

    // Split into 4 parts. Note: a3, b3 may be shorter.
    let n4 = (n + 3) / 4;
    let s = n - 3 * n4;

    let (a0, a123) = a.split_at(n4);
    let (a1, a23) = a123.split_at(n4);
    let (a2, a3) = a23.split_at(n4);
    let (b0, b123) = b.split_at(n4);
    let (b1, b23) = b123.split_at(n4);
    let (b2, b3) = b23.split_at(n4);

    let mut carry: SignedWord = 0;
    let mut carry_c0: SignedWord = 0; // at 2*n4
    let mut carry_c1: SignedWord = 0; // at 3*n4+2
    let mut carry_c2: SignedWord = 0; // at 4*n4+2
    let mut carry_c3: SignedWord = 0; // at 5*n4+2
    let mut carry_c4: SignedWord = 0; // at 6*n4+2
    let mut carry_c5: SignedWord = 0; // at 2*n

    // Buffers that stay alive until the placement (main allocation chain).
    // Evaluate at 0: V(0) = a0 * b0 (z0).
    let (v0, mut memory) = memory.allocate_slice_fill(2 * n4, 0);
    debug_assert_zero!(mul::add_signed_mul_same_len(&mut v0[..], Positive, a0, b0, &mut memory));

    // Evaluate at inf: V(inf) = a3 * b3 (z6).
    let (vinf, mut memory) = memory.allocate_slice_fill(2 * s, 0);
    debug_assert_zero!(mul::add_signed_mul_same_len(&mut vinf[..], Positive, a3, b3, &mut memory));

    let (pv2, mut memory) = memory.allocate_slice_fill(2 * n4 + 2, 0); // -> e2, then z4
    let (pv1, mut memory) = memory.allocate_slice_fill(2 * n4 + 2, 0); // -> e1, then z2
    let (pvh, mut memory) = memory.allocate_slice_fill(2 * n4 + 2, 0); // -> r
    let (o1, mut memory) = memory.allocate_slice_fill(2 * n4 + 2, 0); // -> o1, then z1
    let (o2, mut memory) = memory.allocate_slice_fill(2 * n4 + 2, 0); // -> o2, then t3, z3
    let (z5, mut memory) = memory.allocate_slice_fill(2 * n4 + 2, 0);

    // Evaluate at 2 and -2 via u = x0 + 4*x2, v = x1 + 4*x3:
    // X(2) = u + 2*v, X(-2) = u - 2*v. Products into pv2 (V(2)) and the
    // scoped pvm2 (|V(-2)|), whose sign is tracked in sigma2.
    let sigma2;
    {
        let (ua, mut memory) = memory.allocate_slice_copy_fill(n4 + 1, a0, 0);
        let (va, mut memory) = memory.allocate_slice_copy_fill(n4 + 1, a1, 0);
        ua[n4] = mul::add_mul_word_same_len_in_place(&mut ua[..n4], 4, a2);
        va[n4] = mul::add_mul_word_in_place(&mut va[..n4], 4, a3);
        let (ub, mut memory) = memory.allocate_slice_copy_fill(n4 + 1, b0, 0);
        let (vb, mut memory) = memory.allocate_slice_copy_fill(n4 + 1, b1, 0);
        ub[n4] = mul::add_mul_word_same_len_in_place(&mut ub[..n4], 4, b2);
        vb[n4] = mul::add_mul_word_in_place(&mut vb[..n4], 4, b3);
        let (x2a, mut memory) = memory.allocate_slice_copy_fill(n4 + 1, ua, 0);
        let (x2b, mut memory) = memory.allocate_slice_copy_fill(n4 + 1, ub, 0);
        debug_assert_zero!(mul::add_mul_word_same_len_in_place(&mut x2a[..], 2, va));
        debug_assert_zero!(mul::add_mul_word_same_len_in_place(&mut x2b[..], 2, vb));
        let (t2a, mut memory) = memory.allocate_slice_copy_fill(n4 + 1, va, 0);
        debug_assert_zero!(mul::mul_word_in_place(&mut t2a[..], 2));
        let (t2b, mut memory) = memory.allocate_slice_copy_fill(n4 + 1, vb, 0);
        debug_assert_zero!(mul::mul_word_in_place(&mut t2b[..], 2));
        let (xm2a, mut memory) = memory.allocate_slice_copy_fill(n4 + 1, ua, 0);
        let (xm2b, mut memory) = memory.allocate_slice_copy_fill(n4 + 1, ub, 0);
        let sx = add::sub_in_place_with_sign(&mut xm2a[..], t2a);
        let sy = add::sub_in_place_with_sign(&mut xm2b[..], t2b);
        sigma2 = sx * sy;
        debug_assert_zero!(mul::add_signed_mul_same_len(
            &mut pv2[..],
            Positive,
            x2a,
            x2b,
            &mut memory
        ));
        let (pvm2, mut memory) = memory.allocate_slice_fill(2 * n4 + 2, 0);
        debug_assert_zero!(mul::add_signed_mul_same_len(
            &mut pvm2[..],
            Positive,
            xm2a,
            xm2b,
            &mut memory
        ));
        // o2 = ((V(2) - sigma2*|V(-2)|)/2)/2, pv2 -> e2.
        o2.copy_from_slice(pv2);
        debug_assert_zero!(add::add_signed_in_place(&mut o2[..], -sigma2, pvm2));
        debug_assert_zero!(shift::shr_in_place(&mut o2[..], 2));
        debug_assert_zero!(add::add_signed_in_place(&mut pv2[..], sigma2, pvm2));
        debug_assert_zero!(shift::shr_in_place(&mut pv2[..], 1));
        debug_assert_zero!(add::sub_in_place(&mut pv2[..], v0));
        {
            let (tmp, _) = memory.allocate_slice_copy_fill(2 * n4 + 2, vinf, 0);
            debug_assert_zero!(mul::mul_word_in_place(&mut tmp[..], 64));
            debug_assert_zero!(add::sub_in_place(&mut pv2[..], &tmp[..2 * s + 1]));
        }
        debug_assert_zero!(shift::shr_in_place(&mut pv2[..], 2));
    }

    // Evaluate at 1 and -1 via a02 = x0 + x2, a13 = x1 + x3:
    // X(1) = a02 + a13, X(-1) = a02 - a13. Products into pv1 (V(1)) and the
    // scoped pvm1 (|V(-1)|), sign tracked in sigma1.
    let sigma1;
    {
        let (a02, mut memory) = memory.allocate_slice_copy_fill(n4 + 1, a0, 0);
        a02[n4] = Word::from(add::add_in_place(&mut a02[..n4], a2));
        let (a13, mut memory) = memory.allocate_slice_copy_fill(n4 + 1, a1, 0);
        a13[n4] = Word::from(add::add_in_place(&mut a13[..n4], a3));
        let (b02, mut memory) = memory.allocate_slice_copy_fill(n4 + 1, b0, 0);
        b02[n4] = Word::from(add::add_in_place(&mut b02[..n4], b2));
        let (b13, mut memory) = memory.allocate_slice_copy_fill(n4 + 1, b1, 0);
        b13[n4] = Word::from(add::add_in_place(&mut b13[..n4], b3));
        let (x1a, mut memory) = memory.allocate_slice_copy_fill(n4 + 1, a02, 0);
        let (x1b, mut memory) = memory.allocate_slice_copy_fill(n4 + 1, b02, 0);
        debug_assert_zero!(add::add_same_len_in_place(&mut x1a[..], a13));
        debug_assert_zero!(add::add_same_len_in_place(&mut x1b[..], b13));
        let (xm1a, mut memory) = memory.allocate_slice_copy_fill(n4 + 1, a02, 0);
        let (xm1b, mut memory) = memory.allocate_slice_copy_fill(n4 + 1, b02, 0);
        let sx = add::sub_in_place_with_sign(&mut xm1a[..], a13);
        let sy = add::sub_in_place_with_sign(&mut xm1b[..], b13);
        sigma1 = sx * sy;
        debug_assert_zero!(mul::add_signed_mul_same_len(
            &mut pv1[..],
            Positive,
            x1a,
            x1b,
            &mut memory
        ));
        let (pvm1, mut memory) = memory.allocate_slice_fill(2 * n4 + 2, 0);
        debug_assert_zero!(mul::add_signed_mul_same_len(
            &mut pvm1[..],
            Positive,
            xm1a,
            xm1b,
            &mut memory
        ));
        // o1 = (V(1) - sigma1*|V(-1)|)/2, pv1 -> e1 = (V(1) + V(-1))/2 - z0 - z6.
        o1.copy_from_slice(pv1);
        debug_assert_zero!(add::add_signed_in_place(&mut o1[..], -sigma1, pvm1));
        debug_assert_zero!(shift::shr_in_place(&mut o1[..], 1));
        debug_assert_zero!(add::sub_in_place(&mut pv1[..], o1));
        debug_assert_zero!(add::sub_in_place(&mut pv1[..], v0));
        debug_assert_zero!(add::sub_in_place(&mut pv1[..], vinf));
    }

    // Evaluate at 1/2: Xh = 8*x0 + 4*x1 + 2*x2 + x3 = 64 * X(1/2).
    // Product Vh = Xh * Yh into pvh.
    {
        let (xh, mut memory) = memory.allocate_slice_copy_fill(n4 + 1, a0, 0);
        xh[n4] = mul::mul_word_in_place(&mut xh[..n4], 8);
        xh[n4] += mul::add_mul_word_same_len_in_place(&mut xh[..n4], 4, a1);
        xh[n4] += mul::add_mul_word_in_place(&mut xh[..n4], 2, a2);
        xh[n4] += mul::add_mul_word_in_place(&mut xh[..n4], 1, a3);
        let (yh, mut memory) = memory.allocate_slice_copy_fill(n4 + 1, b0, 0);
        yh[n4] = mul::mul_word_in_place(&mut yh[..n4], 8);
        yh[n4] += mul::add_mul_word_same_len_in_place(&mut yh[..n4], 4, b1);
        yh[n4] += mul::add_mul_word_in_place(&mut yh[..n4], 2, b2);
        yh[n4] += mul::add_mul_word_in_place(&mut yh[..n4], 1, b3);
        debug_assert_zero!(mul::add_signed_mul_same_len(
            &mut pvh[..],
            Positive,
            xh,
            yh,
            &mut memory
        ));
    }

    // z4 = (e2 - e1)/3 (in pv2), z2 = e1 - z4 (in pv1).
    debug_assert_zero!(add::sub_in_place(&mut pv2[..], pv1));
    debug_assert_zero!(div::div_by_word_in_place(&mut pv2[..], 3));
    debug_assert_zero!(add::sub_in_place(&mut pv1[..], pv2));

    // r = (Vh - 64*z0 - z6 - 16*z2 - 4*z4)/2 = 16*z1 + 4*z3 + z5, in pvh.
    {
        let (tmp, _) = memory.allocate_slice_copy_fill(2 * n4 + 2, v0, 0);
        debug_assert_zero!(mul::mul_word_in_place(&mut tmp[..], 64));
        debug_assert_zero!(add::sub_in_place(&mut pvh[..], &tmp[..2 * n4 + 1]));
    }
    debug_assert_zero!(add::sub_in_place(&mut pvh[..], vinf));
    {
        let (tmp, _) = memory.allocate_slice_copy_fill(2 * n4 + 2, pv1, 0);
        debug_assert_zero!(mul::mul_word_in_place(&mut tmp[..], 16));
        debug_assert_zero!(add::sub_in_place(&mut pvh[..], &tmp[..2 * n4 + 1]));
    }
    {
        let (tmp, _) = memory.allocate_slice_copy_fill(2 * n4 + 2, pv2, 0);
        debug_assert_zero!(mul::mul_word_in_place(&mut tmp[..], 4));
        debug_assert_zero!(add::sub_in_place(&mut pvh[..], &tmp[..2 * n4 + 1]));
    }
    debug_assert_zero!(shift::shr_in_place(&mut pvh[..], 1));

    // t3 = (o2 - o1)/3 = z3 + 5*z5 (in o2).
    debug_assert_zero!(add::sub_in_place(&mut o2[..], o1));
    debug_assert_zero!(div::div_by_word_in_place(&mut o2[..], 3));

    // z5 = (r + 12*t3 - 16*o1)/45. r + 12*t3 - 16*o1 = 45*z5 >= 0.
    z5.copy_from_slice(o2);
    debug_assert_zero!(mul::mul_word_in_place(&mut z5[..], 12));
    debug_assert_zero!(add::add_signed_in_place(&mut z5[..], Positive, pvh));
    {
        let (tmp, _) = memory.allocate_slice_copy_fill(2 * n4 + 2, o1, 0);
        debug_assert_zero!(mul::mul_word_in_place(&mut tmp[..], 16));
        debug_assert_zero!(add::sub_in_place(&mut z5[..], &tmp[..2 * n4 + 1]));
    }
    debug_assert_zero!(div::div_by_word_in_place(&mut z5[..], 45));

    // z3 = t3 - 5*z5 (in o2), z1 = o1 - z3 - z5 (in o1).
    {
        let (tmp, _) = memory.allocate_slice_copy_fill(2 * n4 + 2, z5, 0);
        debug_assert_zero!(mul::mul_word_in_place(&mut tmp[..], 5));
        debug_assert_zero!(add::sub_in_place(&mut o2[..], &tmp[..2 * n4 + 1]));
    }
    debug_assert_zero!(add::sub_in_place(&mut o1[..], o2));
    debug_assert_zero!(add::sub_in_place(&mut o1[..], z5));

    // ---- Placement into c ----
    // z_i are added at word offset i*n4; overlapping windows accumulate, and
    // the carries between windows are chained through the carry_cN words.
    // z0 = v0, z1 = o1, z2 = pv1, z3 = o2, z4 = pv2, z5 = z5, z6 = vinf.
    carry_c0 += add::add_signed_same_len_in_place(&mut c[..2 * n4], sign, v0);
    carry_c1 += add::add_signed_same_len_in_place(&mut c[n4..3 * n4 + 2], sign, o1);
    carry_c2 += add::add_signed_same_len_in_place(&mut c[2 * n4..4 * n4 + 2], sign, pv1);
    carry_c3 += add::add_signed_same_len_in_place(&mut c[3 * n4..5 * n4 + 2], sign, o2);
    carry_c4 += add::add_signed_same_len_in_place(&mut c[4 * n4..6 * n4 + 2], sign, pv2);
    carry_c5 += add::add_signed_in_place(&mut c[5 * n4..], sign, &z5[..n4 + s + 1]);
    carry += add::add_signed_in_place(&mut c[6 * n4..], sign, vinf);

    // Apply carries.
    carry_c1 += add::add_signed_word_in_place(&mut c[2 * n4..3 * n4 + 2], carry_c0);
    carry_c2 += add::add_signed_word_in_place(&mut c[3 * n4 + 2..4 * n4 + 2], carry_c1);
    carry_c3 += add::add_signed_word_in_place(&mut c[4 * n4 + 2..5 * n4 + 2], carry_c2);
    carry_c4 += add::add_signed_word_in_place(&mut c[5 * n4 + 2..6 * n4 + 2], carry_c3);
    carry_c5 += add::add_signed_word_in_place(&mut c[6 * n4 + 2..], carry_c4);
    carry += carry_c5;

    debug_assert!(carry.abs() <= 1);
    carry
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(not(feature = "std"))]
    use alloc::vec;
    #[cfg(not(feature = "std"))]
    use alloc::vec::Vec;

    fn lcg_words(seed: u64, len: usize) -> Vec<Word> {
        let mut st = seed | 1;
        let mut v = Vec::with_capacity(len);
        for _ in 0..len {
            st = st
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            v.push(st as Word);
        }
        if let Some(top) = v.last_mut() {
            *top |= 1 << (Word::BITS - 1);
        }
        v
    }

    /// Naive schoolbook multiplication for comparison.
    fn schoolbook_mul(a: &[Word], b: &[Word]) -> Vec<Word> {
        let mut c = vec![0 as Word; a.len() + b.len()];
        for (i, &ai) in a.iter().enumerate() {
            let mut carry: u128 = 0;
            for (j, &bj) in b.iter().enumerate() {
                let idx = i + j;
                let prod = (ai as u128) * (bj as u128) + (c[idx] as u128) + carry;
                c[idx] = prod as Word;
                carry = prod >> Word::BITS;
            }
            let mut k = i + b.len();
            while carry != 0 {
                let sum = (c[k] as u128) + carry;
                c[k] = sum as Word;
                carry = sum >> Word::BITS;
                k += 1;
            }
        }
        c
    }

    fn run_toom4_vs_schoolbook(n: usize) {
        let a = lcg_words(0xDEAD + n as u64, n);
        let b = lcg_words(0xBEEF + n as u64, n);
        let expected = schoolbook_mul(&a, &b);

        let mut c = vec![0 as Word; 2 * n];
        let layout = memory_requirement_up_to(n);
        let mut alloc = crate::memory::MemoryAllocation::new(layout);
        let mut memory = alloc.memory();
        let carry = add_signed_mul_same_len(&mut c, Positive, &a, &b, &mut memory);
        assert_eq!(carry, 0);
        assert_eq!(&c[..], &expected[..], "toom4 mismatch at n={n}");
    }

    #[test]
    fn toom4_matches_schoolbook() {
        // Band edges and length-mod-4 variants (s = n - 3*n4 patterns).
        for &n in &[
            MIN_LEN,
            MIN_LEN + 1,
            MIN_LEN + 2,
            MIN_LEN + 3,
            MIN_LEN + 4,
            80,
            96,
            128,
            191,
            192,
            193,
            255,
            256,
            257,
        ] {
            run_toom4_vs_schoolbook(n);
        }
    }

    #[test]
    fn toom4_matches_schoolbook_deep_recursion() {
        // Exercises several recursion levels (n4 >= MIN_LEN again) and the
        // memory requirement bound at depth.
        for &n in &[512, 1024, 1536] {
            run_toom4_vs_schoolbook(n);
        }
    }

    #[test]
    fn toom4_all_ones() {
        let a = vec![Word::MAX; 128];
        let expected = schoolbook_mul(&a, &a);
        let mut c = vec![0 as Word; 256];
        let layout = memory_requirement_up_to(128);
        let mut alloc = crate::memory::MemoryAllocation::new(layout);
        let mut memory = alloc.memory();
        let carry = add_signed_mul_same_len(&mut c, Positive, &a, &a, &mut memory);
        assert_eq!(carry, 0);
        assert_eq!(&c[..], &expected[..]);
    }

    #[test]
    fn toom4_sparse_limbs() {
        let mut a = vec![0 as Word; 100];
        let mut b = vec![0 as Word; 100];
        for i in 17..83 {
            a[i] = (i as Word + 1).wrapping_mul(0xDEAD_BEEF);
            b[i] = (i as Word + 1).wrapping_mul(0xCAFE_BABE);
        }
        let expected = schoolbook_mul(&a, &b);
        let mut c = vec![0 as Word; 200];
        let layout = memory_requirement_up_to(100);
        let mut alloc = crate::memory::MemoryAllocation::new(layout);
        let mut memory = alloc.memory();
        let carry = add_signed_mul_same_len(&mut c, Positive, &a, &b, &mut memory);
        assert_eq!(carry, 0);
        assert_eq!(&c[..], &expected[..]);
    }

    #[test]
    fn toom4_sign_cancel() {
        let a = lcg_words(0x1234, 100);
        let b = lcg_words(0x5678, 100);
        let mut c = vec![0 as Word; 200];
        let layout = memory_requirement_up_to(100);
        let mut alloc = crate::memory::MemoryAllocation::new(layout);
        let mut memory1 = alloc.memory();
        let _c = add_signed_mul_same_len(&mut c, Positive, &a, &b, &mut memory1);
        let mut alloc2 = crate::memory::MemoryAllocation::new(layout);
        let mut memory2 = alloc2.memory();
        let _c = add_signed_mul_same_len(&mut c, Negative, &a, &b, &mut memory2);
        assert!(c.iter().all(|&w| w == 0));
    }

    #[test]
    fn toom4_single_word_positions() {
        for pa in [0usize, 10, 20, 30, 40, 50, 63] {
            for pb in [0usize, 10, 30, 63] {
                let mut a = vec![0 as Word; 64];
                let mut b = vec![0 as Word; 64];
                a[pa] = 3;
                b[pb] = 7;
                let mut c = vec![0 as Word; 128];
                let layout = memory_requirement_up_to(64);
                let mut alloc = crate::memory::MemoryAllocation::new(layout);
                let mut memory = alloc.memory();
                let carry = add_signed_mul_same_len(&mut c, Positive, &a, &b, &mut memory);
                assert_eq!(carry, 0);
                let expect_at = pa + pb;
                for (i, w) in c.iter().enumerate() {
                    let want = if i == expect_at { 21 } else { 0 };
                    if *w != want {
                        panic!("pa={pa} pb={pb}: c[{i}]={w} want {want}");
                    }
                }
            }
        }
    }
}
