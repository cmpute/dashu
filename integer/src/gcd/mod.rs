//! Greatest Common Divisor
use crate::{
    arch::word::{SignedWord, Word},
    memory::Memory,
    sign::Sign,
};
use alloc::alloc::Layout;
use core::mem;

mod binary;

/// Single word gcd, requires a > 0 and b > 0
pub(crate) fn gcd_word_by_word(mut a: Word, mut b: Word) -> Word {
    debug_assert!(a > 0 && b > 0);

    // find common factors of 2
    let shift = (a | b).trailing_zeros();
    a >>= a.trailing_zeros();
    b >>= b.trailing_zeros();

    // reduce by division if the difference between operands is large
    let (za, zb) = (a.leading_zeros(), b.leading_zeros());
    if za > zb.wrapping_add(4) {
        let r = b % a;
        if r == 0 {
            return a << shift;
        } else {
            b = r >> r.trailing_zeros();
        }
    } else if zb > za.wrapping_add(4) {
        let r = a % b;
        if r == 0 {
            return b << shift;
        } else {
            a = r >> r.trailing_zeros();
        }
    }

    // the binary GCD algorithm
    while a != b {
        if a > b {
            a -= b;
            a >>= a.trailing_zeros();
        } else {
            b -= a;
            b >>= b.trailing_zeros();
        }
    }
    a << shift
}

/// Single extended word gcd
///
/// If bonly is set to true, then only the coefficient for b will be calculated,
/// the coefficient for a will be 1. This is useful for modular inverse calculation
///
/// Binary GCD algorithm is slower in machine word than Euclidean's method
pub(crate) fn xgcd_word_by_word(a: Word, b: Word, bonly: bool) -> (Word, SignedWord, SignedWord) {
    if b > a {
        debug_assert!(!bonly, "`bonly` option only make sense when a >= b");
        let (r, s, t) = xgcd_word_by_word(b, a, false);
        return (r, t, s);
    }
    if b == 1 {
        // this shortcut eliminates the overflow when a = Word::Max and b = 1
        return (1, 0, 1);
    }

    let (mut last_r, mut r) = (a, b);
    let (mut last_s, mut s) = (1, 0);
    let (mut last_t, mut t) = (0, 1);

    while r > 0 {
        let quo = last_r / r;
        let new_r = last_r - quo * r;
        last_r = mem::replace(&mut r, new_r);
        if !bonly {
            let new_s = last_s - quo as SignedWord * s;
            last_s = mem::replace(&mut s, new_s);
        }
        let new_t = last_t - quo as SignedWord * t;
        last_t = mem::replace(&mut t, new_t);
    }

    (last_r, last_s, last_t)
}

/// Greatest common divisor for two multi-digit integers
///
/// The result is stored in the low bits of lhs.
/// The word length of the result number is returned.
pub(crate) fn gcd_in_place(lhs: &mut [Word], rhs: &mut [Word]) -> usize {
    binary::gcd_in_place(lhs, rhs)
}

/// Extended greatest common divisor for two multi-digit integers
///
/// The GCD result is stored in g (need to be pre-allocated and zero filled), while the Bézout coefficient
/// for the two operands is stored in the input slices, and the sign of the two coefficients are returned.
///
/// Specifically if g = gcd(lhs, rhs), lhs * a + rhs * b = g, then a is stored in **rhs**, b is stored in **lhs**,
/// and the returned tuple is (sign of a, sign of b)
pub(crate) fn xgcd_in_place(
    lhs: &mut [Word],
    rhs: &mut [Word],
    g: &mut [Word],
    bonly: bool,
    memory: &mut Memory,
) -> (Sign, Sign) {
    binary::xgcd_in_place(lhs, rhs, g, bonly, memory)
}

/// Memory requirement for GCD.
pub(crate) fn memory_requirement_exact(lhs_len: usize, rhs_len: usize) -> Layout {
    binary::memory_requirement_up_to(lhs_len, rhs_len)
}
