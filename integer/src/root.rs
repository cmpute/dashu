use crate::{
    add::{add_in_place, add_word_in_place, sub_in_place, sub_one_in_place},
    arch::word::{DoubleWord, Word},
    div,
    fast_div::FastDivideNormalized2,
    memory::{self, Memory},
    mul::add_mul_word_in_place,
    primitive::{double_word, extend_word, highest_dword, split_dword, WORD_BITS},
    shift::shr_in_place_with_carry,
    sqr,
};
use alloc::alloc::Layout;
use dashu_base::{DivRem, SquareRootRem};

// n is the size of the output, or half the size of the input
pub fn memory_requirement_sqrt_rem(n: usize) -> Layout {
    if n == 2 {
        memory::zero_layout()
    } else {
        // We need to perform a squaring with n words and an n by n/2 division
        memory::max_layout(
            sqr::memory_requirement_exact(n),
            div::memory_requirement_exact(n, n - n / 2),
        )
    }
}

// Requires a is normalized to 2n words (length must be even)
// Returns the carry of the remainder
pub fn sqrt_rem(b: &mut [Word], a: &mut [Word], memory: &mut Memory) -> bool {
    debug_assert!(a.len() % 2 == 0);
    debug_assert!(a.len() >= 4, "use native sqrt when a has less than 2 words");
    debug_assert!(a.len() == b.len() * 2);

    // shortcut when a has exactly 4 words
    if a.len() == 4 {
        return sqrt_rem_42(b, a);
    }

    /*
     * the "Karatsuba Square Root" algorithm:
     * assume n = a*B^2 + b1*B + b0, B=2^k, a has 2k bits and
     * is normalized (the top two bits of a are not all zeros)
     * 1. calculate sqrt on high part:
     *     s1, r1 = sqrt_rem(a) (r1 <= 2*s1)
     * 2. estimate the root with low part
     *     q, u = div_rem(r1*B + b1, 2*s1)
     *     s = s1*B + q
     *     r = u*B + b0 - q^2
     *    at this step, since a is normalized, we have s1 >= B/2,
     *    therefore q <= floor((r1*B + b1) / B) <= r1 <= 2*s1
     *    also notice b1 < B <= 2*s1, so q <= B
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
    let n = a.len() / 2; // the length of a
    let split = n / 2; // the length of b0

    // step1: sqrt on the higher half
    // afterwards, s1 = b[split..], r1 = a[2*split..split + n]
    let r1_top = sqrt_rem(&mut b[split..], &mut a[2 * split..], memory);
    if r1_top {
        // if the remainder `r1` has a carry, subtract `s1` from it so that the carry is removed
        // so later when calculate 2*q = (r1*B + b1) / s1, the result is actually one less
        let carry = sub_in_place(&mut a[2 * split..split + n], &b[split..]);
        debug_assert!(carry);
    }

    // step2: estimate the result with lower half
    let fast_div_top = FastDivideNormalized2::new(highest_dword(b));
    let carry = div::div_rem_in_place(&mut a[split..split + n], &b[split..], fast_div_top, memory);
    let (a_lo, a_hi) = a.split_at_mut(n);
    b[..split].copy_from_slice(&a_hi[..split]);
    // by now 2*q = b[..split], u = a[split..n], carry is true only if r1 >= s1.
    // also notice that r1 <= 2 * s1, if r1 was subtracted by s1, then r1 <= s1.
    // so r_top and carry are both true only if r1 == 2 * s1 at the beginning.
    // the top bit of q is true if either r_top or carry is true, but not both
    let _ =
        shr_in_place_with_carry(&mut b[..split], 1, ((r1_top ^ carry) as Word) << (WORD_BITS - 1));
    let q_top = r1_top && carry; // true only when q = B, and then b[..split] = 0

    let mut c = 0i8; // stores final carry (top bit) of the remainder
    if a_hi[0] & 1 != 0 {
        // this step fixes the error in u caused by using s1 as divisor instead of 2*s1
        c = add_in_place(&mut a_lo[split..], &b[split..]) as i8;
    }

    // store q^2 in high part of a, ignoring q_top.
    // afterwards, the q_top flag will be considered in the subtraction,
    a_hi.fill(0);
    if !q_top {
        // if q_top is True, then q^2 = B^2, so we don't need to do squaring
        if split == 1 {
            let (b2_lo, b2_hi) = split_dword(extend_word(b[0]) * extend_word(b[0]));
            a_hi[0] = b2_lo;
            a_hi[1] = b2_hi;
        } else {
            sqr::square(&mut a_hi[..2 * split], &b[..split], memory);
        }
    }
    if 2 * split < n {
        a_hi[2 * split] = q_top as Word;
    } else {
        c -= q_top as i8;
    }
    c -= sub_in_place(a_lo, a_hi) as i8;

    // step3: fix the estimation error if necessary
    if c < 0 {
        // r += 2*s - 1; s -= 1;
        // apply the q_top to s first, and then adjust s and r
        let overflow = add_word_in_place(&mut b[split..], q_top as _);
        c += add_mul_word_in_place(a_lo, 2, b) as i8 + 2 * overflow as i8;
        c -= sub_one_in_place(a_lo) as i8;
        let borrow = sub_one_in_place(b);
        debug_assert!(!(overflow ^ borrow)); // borrow should happen if and only if when overflow is true
    }

    c > 0
}

// Special case when a has exactly 4 Words
fn sqrt_rem_42(b: &mut [Word], a: &mut [Word]) -> bool {
    debug_assert!(a.len() == 4 && b.len() == 2);

    // see sqrt_rem() for algorithm explanation
    // step1: sqrt on the higher half
    let (s1, r1) = highest_dword(a).sqrt_rem();
    let s1 = s1 as Word;

    // step2: estimate the result with lower half
    // here r0 = (r1*B + b1) / 2
    let (r1_lo, r1_hi) = split_dword(r1);
    let r0_hi = r1_hi << (WORD_BITS - 1) | r1_lo >> 1;
    let r0_lo = r1_lo << (WORD_BITS - 1) | a[1] >> 1;
    let (mut q, mut u) = double_word(r0_lo, r0_hi).div_rem(s1 as DoubleWord);
    if q >> WORD_BITS > 0 {
        // if q >= B (then q = B), reduce the overestimate
        q -= 1;
        u += s1 as DoubleWord;
    }
    u = u << 1 | (a[1] & 1) as DoubleWord;

    let q = q as Word; // now q must fit in a Word
    let (u_lo, u_hi) = split_dword(u);
    let mut s = double_word(q, s1);
    let q2 = extend_word(q) * extend_word(q);
    let (mut r, borrow) = double_word(a[0], u_lo).overflowing_sub(q2);
    let mut c: i8 = u_hi as i8 - borrow as i8;

    // step3: fix the estimation error if necessary
    if c < 0 {
        let (new_r, c1) = r.overflowing_add(s);
        s -= 1;
        let (new_r, c2) = new_r.overflowing_add(s);
        c += c1 as i8 + c2 as i8;
        r = new_r;
    }

    let (r_lo, r_hi) = split_dword(r);
    let (s_lo, s_hi) = split_dword(s);
    a[0] = r_lo;
    a[1] = r_hi;
    b[0] = s_lo;
    b[1] = s_hi;
    c > 0
}
