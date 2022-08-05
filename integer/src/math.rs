//! Mathematical functions.

use crate::{
    arch::word::{DoubleWord, Word},
    primitive::{double_word, extend_word, split_dword, PrimitiveUnsigned, WORD_BITS, DWORD_BITS},
};

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

// 8bit fixed point estimation of log2(x), x from 0x80 to 0xff, rounding down.
const LOG2_TAB: [u8; 128] = [
    0x00, 0x02, 0x05, 0x08, 0x0b, 0x0e, 0x10, 0x13,
    0x16, 0x19, 0x1b, 0x1e, 0x21, 0x23, 0x26, 0x28,
    0x2b, 0x2e, 0x30, 0x33, 0x35, 0x38, 0x3a, 0x3d,
    0x3f, 0x41, 0x44, 0x46, 0x49, 0x4b, 0x4d, 0x50,
    0x52, 0x54, 0x57, 0x59, 0x5b, 0x5d, 0x60, 0x62,
    0x64, 0x66, 0x68, 0x6a, 0x6d, 0x6f, 0x71, 0x73,
    0x75, 0x77, 0x79, 0x7b, 0x7d, 0x7f, 0x81, 0x84,
    0x86, 0x88, 0x8a, 0x8c, 0x8d, 0x8f, 0x91, 0x93,
    0x95, 0x97, 0x99, 0x9b, 0x9d, 0x9f, 0xa1, 0xa2,
    0xa4, 0xa6, 0xa8, 0xaa, 0xac, 0xad, 0xaf, 0xb1,
    0xb3, 0xb5, 0xb6, 0xb8, 0xba, 0xbc, 0xbd, 0xbf,
    0xc1, 0xc2, 0xc4, 0xc6, 0xc8, 0xc9, 0xcb, 0xcd,
    0xce, 0xd0, 0xd1, 0xd3, 0xd5, 0xd6, 0xd8, 0xda,
    0xdb, 0xdd, 0xde, 0xe0, 0xe1, 0xe3, 0xe5, 0xe6,
    0xe8, 0xe9, 0xeb, 0xec, 0xee, 0xef, 0xf1, 0xf2,
    0xf4, 0xf5, 0xf7, 0xf8, 0xfa, 0xfb, 0xfd, 0xfe,
];

/// A 8bit fixed point estimation of log2(n), the result
/// is always less than the exact value and estimation error ≤ 2.
#[inline]
pub const fn log2_word_fp8(n: Word) -> u32 {
    debug_assert!(n > 0);

    let nbits = WORD_BITS - n.leading_zeros();
    if n < 0x80 {
        // err = 0 in this range
        let shift = 8 - nbits;
        let lookup = LOG2_TAB[(n << shift) as usize - 0x80];
        lookup as u32 + (7 - shift) * 256
    } else if n < 0x200 {
        // err = 0~2 in this range
        let shift = nbits - 8;
        let lookup = LOG2_TAB[(n >> shift) as usize - 0x80];
        lookup as u32 + (7 + shift) * 256
    } else if n < (0x4000 + 0x80) {
        // err = 0~3, use extra 2 bits to reduce error
        let shift = nbits - 8;
        let mask = n >> (shift - 2);
        let lookup = LOG2_TAB[(mask >> 2) as usize - 0x80];
        let est = lookup as u32 + (7 + shift) * 256;

        // err could be 0 if mask & 3 < 3
        est + (mask & 3 == 3) as u32
    } else {
        // err = 0~3, use extra 7 bits to reduce error
        let shift = nbits - 8;
        let mask = n >> (shift - 7);
        let top_est = LOG2_TAB[(mask >> 7) as usize - 0x80];
        let est = top_est as u32 + (7 + shift) * 256;

        // err could be 0 if mask & 127 < 80
        est + (mask & 127 >= 80) as u32
    }
}

/// A 8bit fixed point estimation of log2(n), the result
/// is always greater than the exact value and estimation error ≤ 2.
/// 
/// # Panics
/// 
/// Panics if n is a power of two, in which case the log should
/// be trivially handled.
#[inline]
pub const fn ceil_log2_word_fp8(n: Word) -> u32 {
    debug_assert!(n > 0);
    debug_assert!(!n.is_power_of_two());

    let nbits = WORD_BITS - n.leading_zeros();
    if n < 0x80 {
        // err = 0 in this range
        let shift = 8 - nbits;
        let top_est = LOG2_TAB[(n << shift) as usize - 0x80];
        top_est as u32 + (7 - shift) * 256 + 1
    } else if n < 0x200 {
        // err = 0 in 0x80 ~ 0x100, err = 0~2 in 0x100 ~ 0x200
        let shift = nbits - 8;
        let top_est = LOG2_TAB[(n >> shift) as usize - 0x80];
        let est = top_est as u32 + (7 + shift) * 256 + 1;

        if n > 0x100 && n & 1 == 1 {
            est + 2
        } else {
            est
        }
    } else {
        // err = 0~3, use extra 2 bits to reduce error
        let shift = WORD_BITS - n.leading_zeros() - 8;
        let mask10 = n >> (shift - 2);
        let mask8 = mask10 >> 2;
        if mask8 == 255 {
            0x100 + (7 + shift) * 256
        } else {
            // find next item in LOG2_TAB
            let top_est = LOG2_TAB[mask8 as usize + 1 - 0x80];
            let est = top_est as u32 + (7 + shift) * 256 + 1;
            est - (mask10 & 3 == 0) as u32                
        }
    }
}

/// A 8bit fixed point estimation of log2(n), the result
/// is always less than the exact value and estimation error ≤ 2.
#[inline]
pub const fn log2_dword_fp8(n: DoubleWord) -> u32 {
    let bits = DWORD_BITS - n.leading_zeros();
    if bits <= WORD_BITS {
        log2_word_fp8(n as Word)
    } else {
        let shift = bits - WORD_BITS;
        log2_word_fp8((n >> shift) as Word) + shift * 256
    }
}

/// A 8bit fixed point estimation of log2(n), the result
/// is always greater than the exact value and estimation error ≤ 2.
/// 
/// # Panics
/// 
/// Panics if n is a power of two, in which case the log should
/// be trivially handled.
#[inline]
pub const fn ceil_log2_dword_fp8(n: DoubleWord) -> u32 {
    debug_assert!(!n.is_power_of_two());

    let bits = DWORD_BITS - n.leading_zeros();
    if bits <= WORD_BITS {
        ceil_log2_word_fp8(n as Word)
    } else {
        let shift = bits - WORD_BITS;
        let hi = (n >> shift) as Word; // high word has exactly WORD_BITS
        let hi_bits = if hi == 1 << (WORD_BITS - 1) {
            // specially handled because ceil_log2_word_fp8 disallow a power of 2
            (WORD_BITS - 1) * 256 + 1
        } else {
            // in this case, the ceiling handled by the highest word
            // will cover the requirement for ceiling the low bits
            ceil_log2_word_fp8(hi)
        };
        hi_bits + shift * 256
    }
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
        DoubleWord::MAX >> (DoubleWord::BIT_SIZE - n)
    }
}

/// Calculate dw << shift, assuming shift <= Word::BIT_SIZE, returns (lo, mid, hi).
#[inline]
pub const fn shl_dword(dw: DoubleWord, shift: u32) -> (Word, Word, Word) {
    debug_assert!(shift <= Word::BIT_SIZE);

    let (lo, hi) = split_dword(dw);
    let (n0, carry) = split_dword(extend_word(lo) << shift);
    let (n1, n2) = split_dword((extend_word(hi) << shift) | extend_word(carry));
    (n0, n1, n2)
}

/// Calculate w >> shift, return (result, shifted bits)
/// Note that the shifted bits are put on the highest bits of the Word
#[inline]
pub const fn shr_word(w: Word, shift: u32) -> (Word, Word) {
    let (c, r) = split_dword(double_word(0, w) >> shift);
    (r, c)
}

/// Multiply two `Word`s with carry and return the (low, high) parts of the product
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
    
    #[test]
    fn test_log2_fp8() {
        assert_eq!(log2_word_fp8(1), 0); // err = 0
        assert_eq!(log2_word_fp8(12), 917); // err = 0
        assert_eq!(log2_word_fp8(123), 1777); // err = 0
        assert_eq!(log2_word_fp8(1234), 2628); // err = 0
        assert_eq!(log2_word_fp8(12345), 3478); // err = 1
        assert_eq!(log2_dword_fp8(12345678), 6029); // err = 1
        assert_eq!(log2_dword_fp8(1234567890), 7731); // err = 0
        assert_eq!(log2_word_fp8(0xff), 2046); // err = 0
        assert_eq!(log2_word_fp8(0x100), 2048); // err = 0
        assert_eq!(log2_word_fp8(0x101), 2048); // err = 1
        assert_eq!(log2_dword_fp8(0xff00), 4094); // err = 0
        assert_eq!(log2_dword_fp8(0xffff), 4095); // err = 0
        assert_eq!(log2_dword_fp8(0x10000), 4096); // err = 0
        assert_eq!(log2_dword_fp8(0x10001), 4096); // err = 0

        assert_eq!(ceil_log2_word_fp8(12), 918); // err = 0
        assert_eq!(ceil_log2_word_fp8(123), 1778); // err = 0
        assert_eq!(ceil_log2_word_fp8(1234), 2631); // err = 2
        assert_eq!(ceil_log2_word_fp8(12345), 3480); // err = 0
        assert_eq!(ceil_log2_dword_fp8(12345678), 6032); // err = 2
        assert_eq!(ceil_log2_dword_fp8(1234567890), 7733); // err = 1
        assert_eq!(ceil_log2_word_fp8(0xff), 2047); // err = 0
        assert_eq!(ceil_log2_word_fp8(0x101), 2051); // err = 1
        assert_eq!(ceil_log2_dword_fp8(0xff00), 4096); // err = 1
        assert_eq!(ceil_log2_dword_fp8(0xffff), 4096); // err = 0
        assert_eq!(ceil_log2_dword_fp8(0x10001), 4098); // err = 1

        if Word::BITS == 64 {
            // hard cases
            assert_eq!(log2_word_fp8(0x7f00000000000000), 16125); // err = 0
            assert_eq!(log2_word_fp8(0x7fffffffffffffff), 16127); // err = 0
            assert_eq!(log2_word_fp8(0xff00000000000000), 16382); // err = 0
            assert_eq!(log2_word_fp8(0xffffffffffffffff), 16383); // err = 0

            assert_eq!(ceil_log2_word_fp8(0x7f00000000000000), 16126); // err = 0
            assert_eq!(ceil_log2_word_fp8(0x7fffffffffffffff), 16128); // err = 0
            assert_eq!(ceil_log2_word_fp8(0xff00000000000000), 16384); // err = 1
            assert_eq!(ceil_log2_word_fp8(0xffffffffffffffff), 16384); // err = 0
        }
    }
}
