use alloc::alloc::Layout;
use dashu_base::RootRem;
use crate::{
    arch::word::Word,
    memory::{self, Memory}, primitive::{highest_dword, WORD_BITS, split_dword},
};

pub fn memory_requirement_exact(len: usize, n: usize) -> Layout {
    debug_assert!(n == 2 || n == 3);
    unimplemented!()
}

// Requires a is normalized to 2n words (length must be even)
pub fn sqrt_rem<'a>(b: &mut [Word], a: &mut [Word], memory: &mut Memory) -> Word {
    debug_assert!(a.len() >= 2, "use native sqrt when a is small");
    debug_assert!(a.len() % 2 == 0);
    debug_assert!(a.len() == b.len() * 2);

    /* 
     * the "Karatsuba Square Root" algorithm:
     * assume n = a*B^2 + b1*B + b0, B=2^k, a has 2k bits
     * 1. calculate sqrt on high part:
     *     s1, r1 = sqrt_rem(a)
     * 2. estimate the root with low part
     *     q, u = div_rem(r1*B + b1, 2*s1)
     *     s = s1*B + q
     *     r = u*B + b0 - q^2
     *    at this step, since a is normalized, we have s1 >= B/2,
     *    therefore q <= (r1*B + b1) / B < r1 + 1 <= B
     * 
     * 3. if a3 is normalized, then s is either correct or 1 too big.
     *    r is negative in the latter case, needs adjustment
     *     if r < 0 {
     *         r += 2*s - 1
     *         s -= 1
     *     }
     * 
     * Reference: Zimmermann, P. (1999). Karatsuba square root (Doctoral dissertation, INRIA).
     * https://hal.inria.fr/inria-00072854/en/
     */

    let half = a.len();
    let lo = half / 2;
    let hi = half / 2 - lo;

    let rem_top = if hi == 1 {
        let (s, r) = highest_dword(a).sqrt_rem();
        *b.last_mut().unwrap() = s as Word;
        let (r_lo, r_hi) = split_dword(r);
        a[lo] = r_lo;
        r_hi
    } else {
        sqrt_rem(&mut b[lo..], &mut a[half..], memory)
    };
    unimplemented!()
}

