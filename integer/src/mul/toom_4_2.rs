//! Toom-Cook 4x2 multiplication algorithm (unbalanced operands).

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

/* Structural validity of the 4x2 split for (xs, ys), xs >= ys:
 * n = ceil(xs/4) if xs >= 2*ys, else ceil(ys/2)
 * xs = 3*n + s with 1 <= s <= n
 * ys = n + t with 1 <= t <= n
 *
 * The first branch requires xs >= 2*ys so that t = ys - n <= ys/2 <= n;
 * the second requires xs < 2*ys so that s = xs - 3*n <= 2*ys - 3*ceil(ys/2)
 * <= n. The band where both hold is approximately 1.5 <= xs/ys < 2.5
 * (outside it one of s, t falls out of range).
 */
/// Minimum supported length of the smaller factor.
pub const MIN_LEN: usize = 32;

/// Temporary memory required for multiplication.
///
/// n bounds the length of the smaller factor in words.
pub fn memory_requirement_up_to(n: usize) -> Layout {
    /* Level peak (main-chain buffers plus the largest scoped phase):
     *   v0(2n) + vinf(s+t) + 4*(2n+3) [pv1, pvm1, pv2, o1] + tmp(2n+3)
     *   + evals <= 8*(n+2)
     *   <= 22n + 40
     * The recursive products have smaller factor n+1, so by induction
     * f(n) <= 22n + 40 + f(n+1) <= 30n + 24*ceil(log2 n) + 256.
     */
    let num_words = 30 * n + 24 * (math::ceil_log2(n) as usize) + 256;
    memory::array_layout::<Word>(num_words)
}

/// Split parameters for the 4x2 decomposition, if the operands are in band.
///
/// Returns `(n, s, t)` with `xs = 3n + s`, `ys = n + t`.
pub(crate) fn split_params(xs_len: usize, ys_len: usize) -> Option<(usize, usize, usize)> {
    debug_assert!(xs_len >= ys_len);
    let n = if xs_len >= ys_len << 1 {
        (xs_len + 3) / 4
    } else {
        (ys_len + 1) / 2
    };
    let s = xs_len.checked_sub(3 * n)?;
    let t = ys_len.checked_sub(n)?;
    if s >= 1 && s <= n && t >= 1 && t <= n {
        Some((n, s, t))
    } else {
        None
    }
}

/// Whether the operand pair is in the Toom-4x2 band (1.5:1 up to 2.5:1).
pub(crate) fn in_band(xs_len: usize, ys_len: usize) -> bool {
    // Exclude the upper end where 5 unbalanced products lose to chunked
    // Toom-3; the lower end is enforced by the structural split.
    xs_len << 1 < 5 * ys_len && split_params(xs_len, ys_len).is_some()
}

/// c += sign * a * b
/// Toom-Cook 4x2 method: O(a.len() * b.len()^0.41) for a ~ 2b.
///
/// The operands must be in band ([`in_band`]) with b.len() >= [`MIN_LEN`].
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
    let (n, s, t) = split_params(a.len(), b.len()).expect("operands not in the Toom-4x2 band");

    /* We evaluate X(x) = x3*x^3 + x2*x^2 + x1*x + x0 (the big operand,
     * chunks of n words plus the short head x3 of s words) and
     * Y(x) = y1*x + y0 (chunks of n and t words) at the points -1, 0, 1, 2
     * and infinity. Pointwise multiplication gives the values of
     * V(x) = X(x)*Y(x) = z4*x^4 + ... + z1*x + z0, from which:
     *
     * z0 = V(0),  z4 = V(inf)
     * o1 = (V(1) - V(-1))/2        = z1 + z3
     * z2 = (V(1) + V(-1))/2 - z0 - z4
     * z3 = (V(2) - z0 - 16*z4 - 4*z2 - 2*o1) / 6
     * z1 = o1 - z3
     *
     * All divisions are exact and every intermediate value is provably
     * non-negative with the orderings used below.
     */

    let (x0, x123) = a.split_at(n);
    let (x1, x23) = x123.split_at(n);
    let (x2, x3) = x23.split_at(n);
    let (y0, y1) = b.split_at(n);

    let mut carry: SignedWord = 0;
    let mut carry_c0: SignedWord = 0; // at 2*n
    let mut carry_c1: SignedWord = 0; // at 3*n+1
    let mut carry_c2: SignedWord = 0; // at 4*n+2
    let mut carry_c3: SignedWord = 0; // at 5*n+1

    // Evaluate at 0: V(0) = x0 * y0 (z0).
    let (mut v0, mut memory) = memory.allocate_slice_fill(2 * n, 0);
    debug_assert_zero!(mul::add_signed_mul_same_len(&mut v0, Positive, x0, y0, &mut memory));

    // Evaluate at inf: V(inf) = x3 * y1 (z4).
    let (mut vinf, mut memory) = memory.allocate_slice_fill(s + t, 0);
    debug_assert_zero!(mul::add_signed_mul(&mut vinf, Positive, x3, y1, &mut memory));

    let (mut pv1, mut memory) = memory.allocate_slice_fill(2 * n + 3, 0); // -> V(1), then z2
    let (mut pvm1, mut memory) = memory.allocate_slice_fill(2 * n + 3, 0); // -> |V(-1)|
    let (mut pv2, mut memory) = memory.allocate_slice_fill(2 * n + 3, 0); // -> V(2), then z3
    let (mut o1, mut memory) = memory.allocate_slice_fill(2 * n + 3, 0); // -> o1, then z1

    let mut sigma1 = Positive;

    // Evaluate at 1 and -1: as1 = x0+x1+x2+x3, asm1 = |x0-x1+x2-x3|,
    // bs1 = y0+y1, bsm1 = |y0-y1|.
    {
        let (mut a02, mut memory) = memory.allocate_slice_copy_fill(n + 1, x0, 0);
        a02[n] = Word::from(add::add_in_place(&mut a02[..n], x2));
        let (mut a13, mut memory) = memory.allocate_slice_copy_fill(n + 1, x1, 0);
        a13[n] = Word::from(add::add_in_place(&mut a13[..n], x3));
        let (mut as1, mut memory) = memory.allocate_slice_copy_fill(n + 2, &a02, 0);
        debug_assert_zero!(add::add_signed_in_place(&mut as1, Positive, &a13));
        let (mut asm1, mut memory) = memory.allocate_slice_copy_fill(n + 2, &a02, 0);
        let sx = add::sub_in_place_with_sign(&mut asm1, &a13);
        let (mut bs1, mut memory) = memory.allocate_slice_copy_fill(n + 1, y0, 0);
        debug_assert_zero!(add::add_in_place(&mut bs1, y1));
        let (mut bsm1, mut memory) = memory.allocate_slice_copy_fill(n + 1, y0, 0);
        let sy = add::sub_in_place_with_sign(&mut bsm1, y1);
        sigma1 = sx * sy;
        debug_assert_zero!(mul::add_signed_mul(&mut pv1, Positive, &as1, &bs1, &mut memory));
        debug_assert_zero!(mul::add_signed_mul(&mut pvm1, Positive, &asm1, &bsm1, &mut memory));
        // o1 = (V(1) - sigma1*|V(-1)|)/2 = z1 + z3; pv1 -> z2.
        o1.copy_from_slice(&pv1);
        debug_assert_zero!(add::add_signed_in_place(&mut o1, -sigma1, &pvm1));
        debug_assert_zero!(shift::shr_in_place(&mut o1, 1));
        debug_assert_zero!(add::add_signed_in_place(&mut pv1, sigma1, &pvm1));
        debug_assert_zero!(shift::shr_in_place(&mut pv1, 1));
        debug_assert_zero!(add::sub_in_place(&mut pv1, &v0));
        debug_assert_zero!(add::sub_in_place(&mut pv1, &vinf));
    }

    // Evaluate at 2: as2 = x0 + 2*x1 + 4*x2 + 8*x3, bs2 = y0 + 2*y1.
    // Product into pv2, then z3.
    {
        let (mut as2, mut memory) = memory.allocate_slice_copy_fill(n + 2, x0, 0);
        as2[n] = mul::add_mul_word_same_len_in_place(&mut as2[..n], 2, x1);
        as2[n] += mul::add_mul_word_same_len_in_place(&mut as2[..n], 4, x2);
        as2[n] += mul::add_mul_word_in_place(&mut as2[..n], 8, x3);
        let (mut bs2, mut memory) = memory.allocate_slice_copy_fill(n + 1, y0, 0);
        bs2[n] = mul::add_mul_word_in_place(&mut bs2[..n], 2, y1);
        debug_assert_zero!(mul::add_signed_mul(&mut pv2, Positive, &as2, &bs2, &mut memory));
        debug_assert_zero!(add::sub_in_place(&mut pv2, &v0));
        {
            let (mut tmp, mut memory) = memory.allocate_slice_copy_fill(2 * n + 3, &vinf, 0);
            debug_assert_zero!(mul::mul_word_in_place(&mut tmp, 16));
            debug_assert_zero!(add::sub_in_place(&mut pv2, &tmp));
        }
        {
            let (mut tmp, mut memory) = memory.allocate_slice_copy_fill(2 * n + 3, &pv1, 0);
            debug_assert_zero!(mul::mul_word_in_place(&mut tmp, 4));
            debug_assert_zero!(add::sub_in_place(&mut pv2, &tmp));
        }
        {
            let (mut tmp, mut memory) = memory.allocate_slice_copy_fill(2 * n + 3, &o1, 0);
            debug_assert_zero!(mul::mul_word_in_place(&mut tmp, 2));
            debug_assert_zero!(add::sub_in_place(&mut pv2, &tmp));
        }
        debug_assert_zero!(div::div_by_word_in_place(&mut pv2, 6));
    }

    // z1 = o1 - z3 (in o1).
    debug_assert_zero!(add::sub_in_place(&mut o1, &pv2));

    // ---- Placement into c ----
    // z0 = v0, z1 = o1, z2 = pv1, z3 = pv2, z4 = vinf, at word offsets k*n.
    carry_c0 += add::add_signed_same_len_in_place(&mut c[..2 * n], sign, &v0);
    carry_c1 += add::add_signed_same_len_in_place(&mut c[n..3 * n + 1], sign, &o1[..2 * n + 1]);
    carry_c2 +=
        add::add_signed_same_len_in_place(&mut c[2 * n..4 * n + 2], sign, &pv1[..2 * n + 2]);
    carry_c3 +=
        add::add_signed_same_len_in_place(&mut c[3 * n..5 * n + 1], sign, &pv2[..2 * n + 1]);
    carry += add::add_signed_in_place(&mut c[4 * n..], sign, &vinf);

    // Apply carries.
    carry_c1 += add::add_signed_word_in_place(&mut c[2 * n..3 * n + 1], carry_c0);
    carry_c2 += add::add_signed_word_in_place(&mut c[3 * n + 1..4 * n + 2], carry_c1);
    carry_c3 += add::add_signed_word_in_place(&mut c[4 * n + 2..5 * n + 1], carry_c2);
    carry += add::add_signed_word_in_place(&mut c[5 * n + 1..], carry_c3);

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

    fn run_toom42_vs_schoolbook(xs: usize, ys: usize) {
        assert!(in_band(xs, ys), "test operands must be in band");
        let a = lcg_words(0xF00D + xs as u64, xs);
        let b = lcg_words(0xBEAD + ys as u64, ys);
        let expected = schoolbook_mul(&a, &b);

        let mut c = vec![0 as Word; xs + ys];
        let layout = memory_requirement_up_to(ys);
        let mut alloc = crate::memory::MemoryAllocation::new(layout);
        let mut memory = alloc.memory();
        let carry = add_signed_mul(&mut c, Positive, &a, &b, &mut memory);
        assert_eq!(carry, 0);
        assert_eq!(&c[..], &expected[..], "toom42 mismatch at {xs}x{ys}");
    }

    #[test]
    fn toom42_matches_schoolbook() {
        // Ratio band edges and n-selection branch boundary (xs == 2*ys).
        for &ys in &[MIN_LEN, 64, 96, 128, 200] {
            let lo = 3 * ys / 2 + 1; // just above the 1.5 lower bound
            run_toom42_vs_schoolbook(lo, ys);
            run_toom42_vs_schoolbook(2 * ys, ys); // n-selection branch point
            let hi = 5 * ys / 2 - 1; // just below the 2.5 upper bound
            if hi > lo {
                run_toom42_vs_schoolbook(hi, ys);
            }
        }
    }

    #[test]
    fn toom42_matches_schoolbook_deep_recursion() {
        for &(xs, ys) in &[(1024, 512), (1536, 768), (2400, 1024)] {
            run_toom42_vs_schoolbook(xs, ys);
        }
    }

    #[test]
    fn toom42_all_ones() {
        let a = vec![Word::MAX; 256];
        let b = vec![Word::MAX; 128];
        let expected = schoolbook_mul(&a, &b);
        let mut c = vec![0 as Word; 384];
        let layout = memory_requirement_up_to(128);
        let mut alloc = crate::memory::MemoryAllocation::new(layout);
        let mut memory = alloc.memory();
        let carry = add_signed_mul(&mut c, Positive, &a, &b, &mut memory);
        assert_eq!(carry, 0);
        assert_eq!(&c[..], &expected[..]);
    }

    #[test]
    fn toom42_sign_cancel() {
        let a = lcg_words(0x9999, 400);
        let b = lcg_words(0x7777, 200);
        let mut c = vec![0 as Word; 600];
        let layout = memory_requirement_up_to(200);
        let mut alloc = crate::memory::MemoryAllocation::new(layout);
        let mut memory1 = alloc.memory();
        add_signed_mul(&mut c, Positive, &a, &b, &mut memory1);
        let mut alloc2 = crate::memory::MemoryAllocation::new(layout);
        let mut memory2 = alloc2.memory();
        let _ = add_signed_mul(&mut c, Negative, &a, &b, &mut memory2);
        assert!(c.iter().all(|&w| w == 0));
    }

    /// Compare toom-4x2 against the chunked Toom-3 path it replaces, to tune
    /// [`super::threshold::toom42_min`]. Run with:
    ///   cargo test -p dashu-int --release -- mul::toom_4_2::tests::crossover_toom42 --nocapture --ignored
    #[test]
    #[ignore]
    fn crossover_toom42() {
        use std::time::Instant;

        let sizes: &[(usize, usize)] = &[
            (192, 96),
            (256, 128),
            (384, 192),
            (512, 256),
            (768, 384),
            (1024, 512),
            (1536, 768),
            (2048, 1024),
            (3072, 1536),
            (4096, 2048),
        ];

        println!(
            "{:>9} {:>14} {:>14} {:>10}",
            "xs_x_ys", "chunked-toom3(µs)", "toom-42(µs)", "ratio"
        );
        println!("{}", "-".repeat(54));

        for &(xs, ys) in sizes {
            let a = lcg_words(0xABCD + xs as u64, xs);
            let b = lcg_words(0xDCBA + ys as u64, ys);
            let mut c0 = vec![0 as Word; xs + ys];
            let mut c1 = vec![0 as Word; xs + ys];
            let l_t = crate::mul::toom_3::memory_requirement_up_to(ys);
            let l_42 = memory_requirement_up_to(ys);
            let layout = if l_t.size() > l_42.size() { l_t } else { l_42 };
            let warmup = 5;
            let iters = 20;

            let time = |f: &mut dyn FnMut(&mut Memory)| {
                let mut best = f64::MAX;
                for _ in 0..warmup {
                    let mut alloc = crate::memory::MemoryAllocation::new(layout);
                    let mut mem = alloc.memory();
                    f(&mut mem);
                }
                for _ in 0..iters {
                    let mut alloc = crate::memory::MemoryAllocation::new(layout);
                    let mut mem = alloc.memory();
                    let start = Instant::now();
                    f(&mut mem);
                    let elapsed = start.elapsed().as_secs_f64() * 1_000_000.0;
                    if elapsed < best {
                        best = elapsed;
                    }
                }
                best
            };

            // The path toom-4x2 replaces: chunks of ys through the toom-3
            // same-length kernel.
            let t_chunk = time(&mut |mem| {
                c0.fill(0);
                let _c = helpers::add_signed_mul_split_into_chunks(
                    &mut c0,
                    Positive,
                    &a,
                    &b,
                    ys,
                    mem,
                    crate::mul::toom_3::add_signed_mul_same_len,
                );
            });
            let t_42 = time(&mut |mem| {
                c1.fill(0);
                let _c = add_signed_mul(&mut c1, Positive, &a, &b, mem);
            });

            assert_eq!(&c0[..], &c1[..], "mismatch at {xs}x{ys}");
            println!(
                "{:>9} {:>14.1} {:>14.1} {:>9.2}x",
                format!("{}x{}", xs, ys),
                t_chunk,
                t_42,
                t_42 / t_chunk
            );
        }
    }
}
