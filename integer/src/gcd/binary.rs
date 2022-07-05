//! Binary GCD algorithms (aka Stein's algorithm).

use core::cmp::Ordering;
use dashu_base::ring::Gcd;

use crate::{
    add,
    arch::word::Word,
    bits::trailing_zeros_large,
    cmp::cmp_same_len,
    memory::{self, Memory},
    primitive::WORD_BITS_USIZE,
    shift,
    sign::Sign,
};

use alloc::alloc::Layout;

/// Naive binary GCD for two multi-digit integers
pub(crate) fn gcd_in_place(lhs: &mut [Word], rhs: &mut [Word]) -> usize {
    // find common factors of 2
    let lhs_zeros = trailing_zeros_large(lhs);
    let rhs_zeros = trailing_zeros_large(rhs);
    let init_zeros = lhs_zeros.min(rhs_zeros);

    let (lhs_pos, rhs_pos) = (lhs_zeros / WORD_BITS_USIZE, rhs_zeros / WORD_BITS_USIZE);
    shift::shr_in_place(&mut lhs[lhs_pos..], (lhs_zeros % WORD_BITS_USIZE) as u32);
    shift::shr_in_place(&mut rhs[rhs_pos..], (rhs_zeros % WORD_BITS_USIZE) as u32);

    // Use the binary GCD algorithm. Right shift operations are performed inplace,
    // we keep track of the valid range of words in lhs and rhs using lhs_cur and rhs_cur respectively.
    // The scope below is used to constraint the mutable borrow.

    let result_cur = {
        let (mut lhs_cur, mut rhs_cur) = (&mut lhs[lhs_pos..], &mut rhs[rhs_pos..]);

        loop {
            match lhs_cur
                .len()
                .cmp(&rhs_cur.len())
                .then_with(|| cmp_same_len(lhs_cur, rhs_cur))
            {
                Ordering::Equal => break,
                Ordering::Greater => {
                    // lhs -= rhs
                    assert!(!add::sub_in_place(lhs_cur, rhs_cur));

                    // truncate trailing zeros
                    let zeros = trailing_zeros_large(lhs_cur);
                    lhs_cur = &mut lhs_cur[zeros / WORD_BITS_USIZE..];
                    shift::shr_in_place(lhs_cur, (zeros % WORD_BITS_USIZE) as u32);

                    // truncate leading zeros
                    while *lhs_cur.last().unwrap() == 0 {
                        let last_pos = lhs_cur.len() - 1;
                        lhs_cur = &mut lhs_cur[..last_pos];
                    }
                }
                Ordering::Less => {
                    // rhs -= lhs
                    assert!(!add::sub_in_place(rhs_cur, lhs_cur));

                    // truncate trailing zeros
                    let zeros = trailing_zeros_large(rhs_cur);
                    rhs_cur = &mut rhs_cur[zeros / WORD_BITS_USIZE..];
                    shift::shr_in_place(rhs_cur, (zeros % WORD_BITS_USIZE) as u32);

                    // truncate leading zeros
                    while *rhs_cur.last().unwrap() == 0 {
                        let last_pos = rhs_cur.len() - 1;
                        rhs_cur = &mut rhs_cur[..last_pos];
                    }
                }
            }

            // delegate to single word version when both numbers fit in single word
            if lhs_cur.len() == 1 && rhs_cur.len() == 1 {
                let g = lhs_cur.first().unwrap().gcd(*rhs_cur.first().unwrap());
                *rhs_cur.first_mut().unwrap() = g;
                break;
            }
        }

        &*rhs_cur
    };

    // move the result from rhs to low bits of lhs, with shift taken into account
    let shift_words = init_zeros / WORD_BITS_USIZE;
    let mut final_size = result_cur.len() + shift_words;
    for i in lhs.iter_mut().take(shift_words) {
        // LEGACY: equivalent to lhs[..shift_words].fill(0) after Rust 1.50
        *i = 0;
    }
    lhs[shift_words..final_size].copy_from_slice(result_cur);
    let carry = shift::shl_in_place(
        &mut lhs[shift_words..final_size],
        (init_zeros % WORD_BITS_USIZE) as u32,
    );
    if carry > 0 {
        lhs[final_size] = carry;
        final_size += 1;
    }
    final_size
}

/// Temporary memory required for extended gcd.
pub(crate) fn memory_requirement_up_to(lhs_len: usize, rhs_len: usize) -> Layout {
    // Required memory:
    // - two numbers (s0 & s1) with at most the same size as rhs
    // - two numbers (t0 & t1) with at most the same size as lhs
    let num_words = 2 * (lhs_len + rhs_len);
    memory::array_layout::<Word>(num_words)
}

/// Extended binary GCD for two multi-digits numbers
pub(crate) fn xgcd_in_place(
    lhs: &mut [Word],
    rhs: &mut [Word],
    g: &mut [Word],
    bonly: bool,
    memory: &mut Memory,
) -> (Sign, Sign) {
    debug_assert!(lhs.len() > 1 && rhs.len() > 1);

    // find common factors of 2
    let lhs_zeros = trailing_zeros_large(lhs);
    let rhs_zeros = trailing_zeros_large(rhs);

    let (lhs_pos, rhs_pos) = (lhs_zeros / WORD_BITS_USIZE, rhs_zeros / WORD_BITS_USIZE);
    shift::shr_in_place(&mut lhs[lhs_pos..], (lhs_zeros % WORD_BITS_USIZE) as u32);
    shift::shr_in_place(&mut rhs[rhs_pos..], (rhs_zeros % WORD_BITS_USIZE) as u32);

    // allocate memory for coefficients
    let (s0, mut memory) = memory.allocate_slice_fill::<Word>(rhs.len(), 0);
    let (s1, mut memory) = memory.allocate_slice_fill::<Word>(rhs.len(), 0);
    let (t0, mut memory) = memory.allocate_slice_fill::<Word>(lhs.len(), 0);
    let (t1, _) = memory.allocate_slice_fill::<Word>(lhs.len(), 0);
    let (mut s0_end, mut s1_end) = (1, 1);
    let (mut t0_end, mut t1_end) = (1, 1);

    // initialize s, t
    let (init_zeros, mut shift) = match lhs_zeros.cmp(&rhs_zeros) {
        Ordering::Equal => {
            *s0.first_mut().unwrap() = 1;
            *t1.first_mut().unwrap() = 1;
            (lhs_zeros, 0)
        }
        Ordering::Greater => {
            let shift = lhs_zeros - rhs_zeros;
            *s0.first_mut().unwrap() = 1;
            *t1.get_mut(shift / WORD_BITS_USIZE).unwrap() |= 1 << (shift % WORD_BITS_USIZE);
            t1_end += shift / WORD_BITS_USIZE;
            (rhs_zeros, shift)
        }
        Ordering::Less => {
            let shift = rhs_zeros - lhs_zeros;
            *s0.get_mut(shift / WORD_BITS_USIZE).unwrap() |= 1 << (shift % WORD_BITS_USIZE);
            s0_end += shift / WORD_BITS_USIZE;
            *t1.first_mut().unwrap() = 1;
            (lhs_zeros, shift)
        }
    };

    /// lhs[..lhs_end] += rhs, assuming rhs.len() <= lhs.len()
    /// return true if lhs is fully overflowed
    #[inline]
    fn add_in_place_with_pos(lhs: &mut [Word], lhs_end: &mut usize, rhs: &[Word]) -> bool {
        *lhs_end = (*lhs_end).max(rhs.len());
        if add::add_in_place(&mut lhs[..*lhs_end], rhs) {
            debug_assert!(*lhs_end <= lhs.len());
            if *lhs_end == lhs.len() {
                return true;
            }
            lhs[*lhs_end] = 1;
            *lhs_end += 1;
        }
        false
    }
    /// lhs[..lhs_end] << shift, assuming no overflow
    #[inline]
    fn shl_in_place_with_pos(lhs: &mut [Word], lhs_end: &mut usize, shift: usize) {
        let carry = shift::shl_in_place(&mut lhs[..*lhs_end], shift as u32);
        if carry > 0 {
            debug_assert!(*lhs_end < lhs.len());
            lhs[*lhs_end] = carry;
            *lhs_end += 1;
        }
    }

    // Use the binary GCD algorithm here. Right shift operations are performed inplace just like gcd_in_place
    // We maintain U = t1 * lhs + t0 * rhs and V = s1 * lhs + s0 * rhs, where U, V are the original inputs with
    // common divisors 2 removed
    {
        let (mut lhs_cur, mut rhs_cur) = (&mut lhs[lhs_pos..], &mut rhs[rhs_pos..]);

        loop {
            match lhs_cur
                .len()
                .cmp(&rhs_cur.len())
                .then_with(|| cmp_same_len(lhs_cur, rhs_cur))
            {
                Ordering::Equal => break,
                Ordering::Greater => {
                    // lhs -= rhs
                    assert!(!add::sub_in_place(lhs_cur, rhs_cur));

                    // truncate trailing zeros
                    let zeros = trailing_zeros_large(lhs_cur);
                    lhs_cur = &mut lhs_cur[zeros / WORD_BITS_USIZE..];
                    shift::shr_in_place(lhs_cur, (zeros % WORD_BITS_USIZE) as u32);
                    shift += zeros;

                    // truncate leading zeros
                    while *lhs_cur.last().unwrap() == 0 {
                        let last_pos = lhs_cur.len() - 1;
                        lhs_cur = &mut lhs_cur[..last_pos];
                    }

                    assert!(!add_in_place_with_pos(t0, &mut t0_end, &t1[..t1_end]));
                    shl_in_place_with_pos(t1, &mut t1_end, zeros);
                    assert!(!add_in_place_with_pos(s0, &mut s0_end, &s1[..s1_end]));
                    shl_in_place_with_pos(s1, &mut s1_end, zeros);
                }
                Ordering::Less => {
                    // rhs -= lhs
                    assert!(!add::sub_in_place(rhs_cur, lhs_cur));

                    // truncate trailing zeros
                    let zeros = trailing_zeros_large(rhs_cur);
                    rhs_cur = &mut rhs_cur[zeros / WORD_BITS_USIZE..];
                    shift::shr_in_place(rhs_cur, (zeros % WORD_BITS_USIZE) as u32);
                    shift += zeros;

                    // truncate leading zeros
                    while *rhs_cur.last().unwrap() == 0 {
                        let last_pos = rhs_cur.len() - 1;
                        rhs_cur = &mut rhs_cur[..last_pos];
                    }

                    assert!(!add_in_place_with_pos(t1, &mut t1_end, &t0[..t0_end]));
                    shl_in_place_with_pos(t0, &mut t0_end, zeros);
                    assert!(!add_in_place_with_pos(s1, &mut s1_end, &s0[..s0_end]));
                    shl_in_place_with_pos(s0, &mut s0_end, zeros);
                }
            }
        }

        // copy result from lhs to g
        let carry = shift::shl_in_place(lhs_cur, (init_zeros % WORD_BITS_USIZE) as u32);
        let shift_words = init_zeros / WORD_BITS_USIZE;
        g[shift_words..shift_words + lhs_cur.len()].copy_from_slice(lhs_cur);
        if carry > 0 {
            g[shift_words + lhs_cur.len()] = carry;
        }
    };

    // now lhs = rhs = g = gcd (U,V).
    // compute U/g (store in t1) and V/g (store in s1)
    assert!(!add_in_place_with_pos(s1, &mut s1_end, &s0[..s0_end]));
    assert!(!add_in_place_with_pos(t1, &mut t1_end, &t0[..t0_end]));

    // now 2^shift * g = s0 * U - t0 * V. Get rid of the power of two, using
    // the equation s0 * U - t0 * V = (s0 + V/g) U - (t0 + U/g) V.
    for _ in 0..shift {
        let (s_carry, t_carry) = if (s0.first().unwrap() | t0.first().unwrap()) & 1 > 0 {
            (
                add_in_place_with_pos(s0, &mut s0_end, &s1[..s1_end]),
                add_in_place_with_pos(t0, &mut t0_end, &t1[..t1_end]),
            )
        } else {
            (false, false)
        };

        const HBIT: Word = 1 << (WORD_BITS_USIZE - 1);
        shift::shr_in_place_with_carry(&mut s0[..s0_end], 1, HBIT * s_carry as Word);
        shift::shr_in_place_with_carry(&mut t0[..t0_end], 1, HBIT * t_carry as Word);
    }

    // reduce s0 and t0 if s0 > V/g - s0
    // the xx_end markers are ignored here because there could be leading zeros
    // LEGACY: use Ordering::is_ge() in Rust 1.53
    let reduce = !matches!(cmp_same_len(s0, s1), Ordering::Less) || {
        assert!(!add::sub_in_place(s1, s0));
        let r = !matches!(cmp_same_len(s0, s1), Ordering::Less);
        assert!(!add::add_in_place(s1, s0));
        r
    };
    let (ssign, tsign) = if reduce {
        (
            add::sub_in_place_with_sign(s0, s1),
            add::sub_in_place_with_sign(t0, t1),
        )
    } else {
        (Sign::Positive, Sign::Positive)
    };

    // move s0 to rhs, t0 to lhs
    lhs.copy_from_slice(&t0[..lhs.len()]);
    if !bonly {
        // there's not much we can do other than not copying the result
        // in the binary extended GCD algorithm
        rhs.copy_from_slice(&s0[..rhs.len()]);
    }
    (ssign, -tsign)
}
