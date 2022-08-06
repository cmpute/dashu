//! Greatest Common Divisor
use crate::{
    add,
    arch::word::{DoubleWord, SignedDoubleWord, SignedWord, Word},
    div,
    memory::Memory,
    mul,
    primitive::{extend_word, shrink_dword, PrimitiveSigned},
    sign::Sign,
};
use alloc::alloc::Layout;
use dashu_base::ExtendedGcd;

mod lehmer;

/// Greatest common divisor for two multi-digit integers
///
/// This function assumes lhs > rhs.
///
/// The result is stored in the low bits of lhs or rhs. The first returned value
/// is the word length of result number, and the second returned value determine
/// if the result is in lhs (false) or rhs (true)
pub fn gcd_in_place(lhs: &mut [Word], rhs: &mut [Word], memory: &mut Memory) -> (usize, bool) {
    debug_assert!(
        lhs.last().unwrap() != &0 && rhs.last().unwrap() != &0,
        "leading zeros are not allowed!"
    );

    lehmer::gcd_in_place(lhs, rhs, memory)
}

/// Memory requirement for GCD.
pub fn memory_requirement_exact(lhs_len: usize, rhs_len: usize) -> Layout {
    lehmer::memory_requirement_up_to(lhs_len, rhs_len)
}

/// Extended greatest common divisor for two multi-digit integers
///
/// The GCD result is stored in one of the input, while the Bézout coefficient for
/// the operand is stored in another input slice. The length and sign of the result
/// are returned.
///
/// Specifically this function assumes `lhs > rhs`. If `g = gcd(lhs, rhs)`,
/// `lhs * a + rhs * b = g`, then g is stored in **rhs**, b (unsigned) is
/// stored in **lhs**, and the returned tuple is (length of g, length of b, sign of b).
/// The other coefficient a could be computed when needed, by `a = (g - rhs * b) / lhs`
pub fn gcd_ext_in_place(
    lhs: &mut [Word],
    rhs: &mut [Word],
    memory: &mut Memory,
) -> (usize, usize, Sign) {
    lehmer::gcd_ext_in_place(lhs, rhs, memory)
}

/// Memory requirement for extended GCD.
pub fn memory_requirement_ext_exact(lhs_len: usize, rhs_len: usize) -> Layout {
    lehmer::memory_requirement_ext_up_to(lhs_len, rhs_len)
}

/// Extended greatest common divisor between a large number and small number.
///
/// If `g = gcd(lhs, rhs)`, `lhs * a + rhs * b = g`, b (unsigned) is
/// stored in **lhs**, and the returned tuple is (g, a, sign of b).
pub fn gcd_ext_word(lhs: &mut [Word], rhs: Word) -> (Word, SignedWord, Sign) {
    debug_assert!(rhs != 0);
    let rem = div::div_by_word_in_place(lhs, rhs);
    if rem == 0 {
        *lhs.first_mut().unwrap() = 1;
        lhs[1..].fill(0);
        (rhs, 0, Sign::Positive)
    } else {
        // a = t, b = s - t * lhs
        let (r, s, t) = rhs.gcd_ext(rem);
        let (s_sign, s_mag) = s.to_sign_magnitude();
        let t_mag = t.unsigned_abs();
        let carry = mul::mul_word_in_place(lhs, t_mag);
        let carry2 = add::add_word_in_place(lhs, s_mag);
        debug_assert!(carry == 0 && !carry2);
        (r, t, s_sign)
    }
}

pub fn gcd_ext_dword(lhs: &mut [Word], rhs: DoubleWord) -> (DoubleWord, SignedDoubleWord, Sign) {
    debug_assert!(rhs > Word::MAX as DoubleWord, "call gcd_ext_word when rhs is small");
    let rem = div::div_by_dword_in_place(lhs, rhs);
    if rem == 0 {
        *lhs.first_mut().unwrap() = 1;
        lhs[1..].fill(0);
        (rhs, 0, Sign::Positive)
    } else {
        // a = t, b = s - t * lhs
        let (r, s, t) = rhs.gcd_ext(rem);
        let (s_sign, s_mag) = s.to_sign_magnitude();
        let t_mag = t.unsigned_abs();
        let carry = if let Some(st) = shrink_dword(t_mag) {
            extend_word(mul::mul_word_in_place(lhs, st))
        } else {
            mul::mul_dword_in_place(lhs, t_mag)
        };
        let carry2 = add::add_dword_in_place(lhs, s_mag);
        debug_assert!(carry == 0 && !carry2);
        (r, t, s_sign)
    }
}
