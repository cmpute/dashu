//! Bit shift functions.

use crate::{
    arch::word::Word,
    primitive::{double_word, extend_word, split_dword, WORD_BITS},
};

/// Shift left by less than WORD_BITS in place.
/// Returns carry.
pub(crate) fn shl_in_place(words: &mut [Word], shift: u32) -> Word {
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
pub(crate) fn shr_in_place(words: &mut [Word], shift: u32) -> Word {
    shr_in_place_with_carry(words, shift, 0)
}

/// Shift right by less than WORD_BITS in place.
/// An optional carry could be provided from a higher word.
/// Returns shifted bits.
pub(crate) fn shr_in_place_with_carry(words: &mut [Word], shift: u32, mut carry: Word) -> Word {
    debug_assert!(shift < WORD_BITS);
    if shift == 0 {
        debug_assert!(carry == 0);
        return 0;
    }
    for word in words.iter_mut().rev() {
        let (new_carry, new_word) = split_dword(double_word(0, *word) >> shift);
        *word = new_word | carry;
        carry = new_carry;
    }
    carry >> (WORD_BITS - shift)
}
