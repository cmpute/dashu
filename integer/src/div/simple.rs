//! Simple (School book) division algorithm.

use crate::{
    add,
    arch::word::Word,
    cmp,
    fast_div::FastDivideNormalized2,
    mul,
    primitive::{double_word, highest_dword, split_dword},
};

/// Division in place using the simple algorithm.
///
/// Divide lhs by rhs, replacing the top words of lhs by the quotient and the
/// bottom words of lhs by the remainder.
///
/// `lhs = [lhs % rhs, lhs / rhs]`
///
/// Returns carry in the quotient. It is at most 1 because rhs is normalized.
#[must_use]
pub(crate) fn div_rem_in_place(
    lhs: &mut [Word],
    rhs: &[Word],
    fast_div_rhs_top: FastDivideNormalized2,
) -> bool {
    // The Art of Computer Programming, algorithm 4.3.1D.

    let n = rhs.len();
    assert!(n >= 2);

    let lhs_len = lhs.len();
    assert!(lhs_len >= n);

    let quotient_carry = cmp::cmp_same_len(&lhs[lhs_len - n..], rhs).is_ge();
    if quotient_carry {
        let overflow = add::sub_same_len_in_place(&mut lhs[lhs_len - n..], rhs);
        debug_assert!(!overflow);
    }

    // keep track of the position of remainder
    let mut rem = lhs;
    while rem.len() > n {
        let (lhs_top, lhs_lo) = rem.split_last_mut().unwrap();

        // Get the next digit of quotient
        *lhs_top = div_rem_highest_word(*lhs_top, lhs_lo, rhs, fast_div_rhs_top);

        // Shrink the remainder.
        rem = lhs_lo;
    }
    // Quotient is now in lhs[n..] and remainder in lhs[..n].
    quotient_carry
}

/// Do one step division on lhs by rhs, get the higest word of the quotient.
///
/// Rhs must be normalized, lhs.len() > rhs.len() and lhs[lhs.len() - rhs.len()..]
/// must be smaller than rhs.
///
/// The remainder will be put in lhs_lo and the quotient word will be returned.
#[inline]
pub(crate) fn div_rem_highest_word(
    lhs_top: Word,
    lhs_lo: &mut [Word],
    rhs: &[Word],
    fast_div_rhs_top: FastDivideNormalized2,
) -> Word {
    let n = rhs.len();
    let (rhs_top, rhs_lo) = rhs.split_last().unwrap();

    let lhs_lo_len = lhs_lo.len();
    debug_assert!(lhs_lo_len >= n);
    debug_assert!(lhs_top
        .cmp(rhs_top)
        .then(cmp::cmp_same_len(
            &lhs_lo[lhs_lo_len - rhs_lo.len()..],
            rhs_lo
        ))
        .is_le());

    // lhs0 = lhs_top
    let (lhs2, lhs1) = split_dword(highest_dword(lhs_lo));
    let lhs01 = double_word(lhs1, lhs_top);

    // Approximate the next word of quotient by
    // q = floor([lhs0, lhs1, lhs2] / [rhs0, rhs1])
    // q may be too large (by 1), but never too Small
    let mut q = if lhs_top < *rhs_top {
        fast_div_rhs_top.div_rem(lhs2, lhs01).0
    } else {
        // In this case MAX is accurate (r is already overflown).
        Word::MAX
    };

    // Subtract a multiple of rhs.
    let mut borrow = mul::sub_mul_word_same_len_in_place(&mut lhs_lo[lhs_lo_len - n..], q, rhs);

    if borrow > lhs_top {
        // Unlikely case: q is too large (by 1), add a correction.
        q -= 1;
        let carry = add::add_same_len_in_place(&mut lhs_lo[lhs_lo_len - n..], rhs);
        debug_assert!(carry);
        borrow -= 1;
    }
    debug_assert!(borrow == lhs_top);

    q
}
