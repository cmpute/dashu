//! Mathematical functions.

use crate::{
    arch::word::{Word, DoubleWord},
    assert::debug_assert_in_const_fn,
    primitive::{PrimitiveUnsigned, WORD_BITS, split_dword, extend_word}
};

/// The length of an integer in bits.
/// 0 for 0.
#[inline]
pub(crate) fn bit_len<T: PrimitiveUnsigned>(x: T) -> u32 {
    T::BIT_SIZE - x.leading_zeros()
}

/// The length of an integer in bits.
/// 0 for 0.
#[inline]
pub(crate) const fn bit_len_word(x: Word) -> u32 {
    Word::BIT_SIZE - x.leading_zeros()
}

/// Ceiling of log_2(x).
/// x must be non-zero.
#[inline]
pub(crate) fn ceil_log_2<T: PrimitiveUnsigned>(x: T) -> u32 {
    debug_assert!(x != T::from(0u8));
    bit_len(x - T::from(1u8))
}

/// Ceiling of log_2(x).
/// x must be non-zero.
#[inline]
pub(crate) const fn ceil_log_2_word(x: Word) -> u32 {
    debug_assert_in_const_fn!(x != 0);
    bit_len_word(x - 1)
}

/// Ceiling of a / b.
#[inline]
pub(crate) fn ceil_div<T: PrimitiveUnsigned>(a: T, b: T) -> T {
    if a == T::from(0u8) {
        T::from(0u8)
    } else {
        (a - T::from(1u8)) / b + T::from(1u8)
    }
}

/// Ceiling of a / b.
#[inline]
pub(crate) const fn ceil_div_usize(a: usize, b: usize) -> usize {
    if a == 0 {
        0
    } else {
        (a - 1) / b + 1
    }
}

/// Round up a to a multiple of b.
#[inline]
pub(crate) fn round_up<T: PrimitiveUnsigned>(a: T, b: T) -> T {
    ceil_div(a, b) * b
}

/// Round up a to a multiple of b.
#[inline]
pub(crate) const fn round_up_usize(a: usize, b: usize) -> usize {
    ceil_div_usize(a, b) * b
}

/// n ones: 2^n - 1
#[inline]
pub(crate) const fn ones_word(n: u32) -> Word {
    if n == 0 {
        0
    } else {
        Word::MAX >> (Word::BIT_SIZE - n)
    }
}

/// n ones: 2^n - 1
#[inline]
pub(crate) const fn ones_dword(n: u32) -> DoubleWord {
    if n == 0 {
        0
    } else {
        DoubleWord::MAX >> (DoubleWord::BIT_SIZE - n)
    }
}

#[inline]
pub(crate) const fn min_usize(a: usize, b: usize) -> usize {
    if a < b {
        a
    } else {
        b
    }
}

/// Multiply two `Word`s with carries and return the (low, high) parts of the product
// TODO: do we need two carries? check lehmer gcd
#[inline(always)]
pub(crate) const fn mul_add_carry(lhs: Word, rhs: Word, c1: Word, c2: Word) -> (Word, Word) {
    split_dword(extend_word(lhs) * extend_word(rhs) + extend_word(c1) + extend_word(c2))
}

/// Multiply two `DoubleWord`s and return the (low, high) parts of the product
#[inline]
pub(crate) const fn mul_add_carry_dword(lhs: DoubleWord, rhs: DoubleWord) -> (DoubleWord, DoubleWord) {
    // TODO: use mul_add_carry to implement this, and accept two carries
    /// Split double word without narrowing
    #[inline(always)]
    const fn split(v: DoubleWord) -> (DoubleWord, DoubleWord) {
        (v >> WORD_BITS, v & (DoubleWord::MAX >> WORD_BITS))
    }

    let (x1, x0) = split(lhs);
    let (y1, y0) = split(rhs);

    let z2 = x1 * y1;
    let (c0, z0) = split(x0 * y0); // c0 <= umax::MAX - 1
    let (c1, z1) = split(x1 * y0 + c0);
    let z2 = z2 + c1;
    let (c1, z1) = split(x0 * y1 + z1);
    
    let lo = z0 | z1 << WORD_BITS;
    let hi = z2 + c1;
    (lo, hi)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bit_len() {
        assert_eq!(bit_len(0u32), 0);
        assert_eq!(bit_len(0b10011101u32), 8);
        assert_eq!(bit_len(0b10000000u32), 8);
        assert_eq!(bit_len(0b1111111u32), 7);
    }

    #[test]
    fn test_ceil_log_2() {
        assert_eq!(ceil_log_2(1u32), 0);
        assert_eq!(ceil_log_2(7u32), 3);
        assert_eq!(ceil_log_2(8u32), 3);
        assert_eq!(ceil_log_2(9u32), 4);
        assert_eq!(ceil_log_2(u32::MAX), 32);
    }

    #[test]
    fn test_ceil_div() {
        assert_eq!(ceil_div(0u32, 10u32), 0);
        assert_eq!(ceil_div(9u32, 10u32), 1);
        assert_eq!(ceil_div(10u32, 10u32), 1);
        assert_eq!(ceil_div(11u32, 10u32), 2);
    }

    #[test]
    fn test_round_up() {
        assert_eq!(round_up(0u32, 10u32), 0);
        assert_eq!(round_up(9u32, 10u32), 10);
        assert_eq!(round_up(10u32, 10u32), 10);
        assert_eq!(round_up(11u32, 10u32), 20);
    }

    #[test]
    fn test_ones() {
        assert_eq!(ones_word(0), 0);
        assert_eq!(ones_word(5), 0b11111);
        assert_eq!(ones_word(16), u16::MAX as Word);
    }
}
