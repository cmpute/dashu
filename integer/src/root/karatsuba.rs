use alloc::alloc::Layout;

use crate::{
    arch::word::Word,
    memory::{self, Memory},
};

pub fn memory_requirement_exact(len: usize, n: usize) -> Layout {
    debug_assert!(n == 2 || n == 3);
    unimplemented!()
}

pub fn sqrt_rem<'a>(b: &mut [Word], a: &mut [Word], memory: &mut Memory) {
    debug_assert!(a.len() >= 2, "use native sqrt when a is small");
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

    unimplemented!()
}

