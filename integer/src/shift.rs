//! Bit shift functions.

use crate::{
    arch::word::Word,
    math::shr_word,
    primitive::{double_word, extend_word, split_dword, WORD_BITS},
};

/// Shift left by less than WORD_BITS in place.
/// Returns carry.
pub fn shl_in_place(words: &mut [Word], shift: u32) -> Word {
    debug_assert!(shift < WORD_BITS);
    if shift == 0 {
        return 0;
    }
    let mut carry = 0;
    for word in words {
        let (new_word, new_carry) = split_dword(extend_word(*word) << shift);
        *word = new_word | carry;
        carry = new_carry;
    }
    carry
}

/// Shift right by less than WORD_BITS in place.
/// Returns shifted bits.
#[inline]
pub fn shr_in_place(words: &mut [Word], shift: u32) -> Word {
    shr_in_place_with_carry(words, shift, 0)
}

/// Shift right by less than WORD_BITS in place.
/// An optional carry could be provided from a higher word.
/// Returns shifted bits.
pub fn shr_in_place_with_carry(words: &mut [Word], shift: u32, mut carry: Word) -> Word {
    debug_assert!(shift < WORD_BITS);
    if shift == 0 {
        debug_assert_eq!(carry, 0);
        return 0;
    }
    for word in words.iter_mut().rev() {
        let (new_word, new_carry) = shr_word(*word, shift);
        *word = new_word | carry;
        carry = new_carry;
    }
    carry >> (WORD_BITS - shift) // TODO(next): don't shift the output
}
