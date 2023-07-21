//! Mathematical functions.

use crate::{
    arch::word::{DoubleWord, Word},
    primitive::{double_word, extend_word, split_dword, PrimitiveUnsigned, DWORD_BITS, WORD_BITS},
};

// pub type FastDivideNormalized = num_modular::Normalized2by1Divisor<Word>;
pub type FastDivideNormalized2 = num_modular::Normalized3by2Divisor<Word, DoubleWord>;

/// The length of an integer in bits.
/// 0 for 0.
#[inline]
pub fn bit_len<T: PrimitiveUnsigned>(x: T) -> u32 {
    T::BIT_SIZE - x.leading_zeros()
}

/// Ceiling of log_2(x).
/// x must be non-zero.
#[inline]
pub fn ceil_log2<T: PrimitiveUnsigned>(x: T) -> u32 {
    debug_assert!(x != T::from(0u8));
    bit_len(x - T::from(1u8))
}

/// Ceiling of a / b.
#[inline]
pub fn ceil_div<T: PrimitiveUnsigned>(a: T, b: T) -> T {
    if a == T::from(0u8) {
        T::from(0u8)
    } else {
        (a - T::from(1u8)) / b + T::from(1u8)
    }
}

/// Ceiling of a / b.
#[inline]
pub const fn ceil_div_usize(a: usize, b: usize) -> usize {
    if a == 0 {
        0
    } else {
        (a - 1) / b + 1
    }
}

/// Round up a to a multiple of b.
#[inline]
pub fn round_up<T: PrimitiveUnsigned>(a: T, b: T) -> T {
    ceil_div(a, b) * b
}

/// Round up a to a multiple of b.
#[inline]
pub const fn round_up_usize(a: usize, b: usize) -> usize {
    ceil_div_usize(a, b) * b
}

/// n ones: 2^n - 1
#[inline]
pub const fn ones_word(n: u32) -> Word {
    if n == 0 {
        0
    } else {
        Word::MAX >> (Word::BIT_SIZE - n)
    }
}

/// n ones: 2^n - 1
#[inline]
pub const fn ones_dword(n: u32) -> DoubleWord {
    if n == 0 {
        0
    } else {
        DoubleWord::MAX >> (DWORD_BITS - n)
    }
}

/// Calculate dw << shift, assuming shift <= Word::BIT_SIZE, returns (lo, mid, hi).
#[inline]
pub const fn shl_dword(dw: DoubleWord, shift: u32) -> (Word, Word, Word) {
    debug_assert!(shift <= WORD_BITS);

    let (lo, hi) = split_dword(dw);
    let (n0, carry) = split_dword(extend_word(lo) << shift);
    let (n1, n2) = split_dword((extend_word(hi) << shift) | extend_word(carry));
    (n0, n1, n2)
}

/// Calculate w >> shift, return (result, shifted bits).
/// Note that the shifted bits are put on the highest bits of the Word
#[inline]
pub const fn shr_word(w: Word, shift: u32) -> (Word, Word) {
    let (c, r) = split_dword(double_word(0, w) >> shift);
    (r, c)
}

/// Multiply two `Word`s with carry and return the (low, high) parts of the product.
/// This operation will not overflow.
#[inline(always)]
pub const fn mul_add_carry(lhs: Word, rhs: Word, carry: Word) -> (Word, Word) {
    split_dword(extend_word(lhs) * extend_word(rhs) + extend_word(carry))
}

/// Multiply two `Word`s with 2 carries and return the (low, high) parts of the product.
/// This operation will not overflow.
#[inline(always)]
pub const fn mul_add_2carry(lhs: Word, rhs: Word, c0: Word, c1: Word) -> (Word, Word) {
    split_dword(extend_word(lhs) * extend_word(rhs) + extend_word(c0) + extend_word(c1))
}

/// Multiply two `DoubleWord`s with carry and return the (low, high) parts of the product.
/// This operation will not overflow.
#[inline]
pub const fn mul_add_carry_dword(
    lhs: DoubleWord,
    rhs: DoubleWord,
    carry: DoubleWord,
) -> (DoubleWord, DoubleWord) {
    let (x0, x1) = split_dword(lhs);
    let (y0, y1) = split_dword(rhs);
    let (ic0, ic1) = split_dword(carry);

    let (z0, c0) = mul_add_carry(x0, y0, ic0);
    let (z1, c1a) = mul_add_carry(x1, y0, c0);
    let (z1, c1b) = mul_add_2carry(x0, y1, z1, ic1);
    let (z2, z3) = mul_add_2carry(x1, y1, c1a, c1b);

    let lo = double_word(z0, z1);
    let hi = double_word(z2, z3);

    (lo, hi)
}

/// Calculate the max k such that base^k <= Word::MAX, return (k, base^k)
pub const fn max_exp_in_word(base: Word) -> (usize, Word) {
    debug_assert!(base > 2);

    // shortcut
    if base > ones_word(WORD_BITS / 2) {
        return (1, base);
    }

    // estimate log_base(Word::MAX)
    let mut exp = WORD_BITS / (WORD_BITS - base.leading_zeros());
    let mut pow: Word = base.pow(exp);
    while let Some(prod) = pow.checked_mul(base) {
        exp += 1;
        pow = prod;
    }
    (exp as usize, pow)
}

/// Calculate the max k such that base^k <= DoubleWord::MAX, return (k, base^k)
pub const fn max_exp_in_dword(base: Word) -> (usize, DoubleWord) {
    debug_assert!(base > 2);
    let (exp, pow) = max_exp_in_word(base);
    let (exp, pow) = (2 * exp, extend_word(pow) * extend_word(pow));

    // the error of exp is at most one
    if let Some(prod) = pow.checked_mul(extend_word(base)) {
        (exp + 1, prod)
    } else {
        (exp, pow)
    }
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
        assert_eq!(ceil_log2(1u32), 0);
        assert_eq!(ceil_log2(7u32), 3);
        assert_eq!(ceil_log2(8u32), 3);
        assert_eq!(ceil_log2(9u32), 4);
        assert_eq!(ceil_log2(u32::MAX), 32);
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

    #[test]
    fn test_max_exp_in_word() {
        for b in 3..30 {
            let (_, pow) = max_exp_in_word(b);
            assert!(pow.overflowing_mul(b).1);
            let (_, pow) = max_exp_in_dword(b);
            assert!(pow.overflowing_mul(extend_word(b)).1);
        }
    }
}
