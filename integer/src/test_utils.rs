use crate::Word;
use alloc::vec::Vec;

/// Fixed dense operand pattern (no generator state): the i-th word is
/// `(i+1)·step + i`, wrapping — distinct, carry-rich words, identical on
/// every run (and sharing their low bits across word sizes, since the
/// arithmetic is modular). The top word is kept nonzero so the operand
/// really spans `len` words.
pub(crate) fn pattern_words(len: usize, step: u64) -> Vec<Word> {
    (0..len)
        .map(|i| {
            let w = (i as u64 + 1).wrapping_mul(step).wrapping_add(i as u64) as Word;
            if i + 1 == len && w == 0 {
                1
            } else {
                w
            }
        })
        .collect()
}
