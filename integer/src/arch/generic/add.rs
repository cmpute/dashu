use crate::arch::word::Word;

/// Add a + b + carry.
///
/// Returns (result, overflow).
#[inline]
pub fn add_with_carry(a: Word, b: Word, carry: Word) -> (Word, Word) {
    let sum_nc = a.wrapping_add(b);
    let sum = sum_nc.wrapping_add(carry);
    let carry_out = Word::from(sum_nc < a) | Word::from(sum < sum_nc);
    (sum, carry_out)
}

/// Subtract a - b - borrow.
///
/// Returns (result, overflow).
#[inline]
pub fn sub_with_borrow(a: Word, b: Word, borrow: Word) -> (Word, Word) {
    let diff_nb = a.wrapping_sub(b);
    let diff = diff_nb.wrapping_sub(borrow);
    let borrow_out = Word::from(diff_nb > a) | Word::from(diff > diff_nb);
    (diff, borrow_out)
}
