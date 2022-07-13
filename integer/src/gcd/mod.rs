//! Greatest Common Divisor
use crate::{arch::word::Word, memory::{self, Memory}, sign::Sign};
use alloc::alloc::Layout;

mod naive;
mod binary;
mod lehmer;

/// Greatest common divisor for two multi-digit integers
///
/// The result is stored in the low bits of lhs or rhs. The first returned value
/// is the word length of result number, and the second returned value determine
/// if the result is in lhs (false) or rhs (true)
pub fn gcd_in_place(lhs: &mut [Word], rhs: &mut [Word], memory: &mut Memory) -> (usize, bool) {
    debug_assert!(lhs.last().unwrap() != &0 && rhs.last().unwrap() != &0, "leading zeros are not allowed!");

    // TODO: pre-remove the trailing zero words, and give the number of zeros as input
    // to low-level algorithms.
    lehmer::gcd_in_place(lhs, rhs, memory)
}

/// Memory requirement for GCD.
pub fn memory_requirement_exact(lhs_len: usize, rhs_len: usize) -> Layout {
    lehmer::memory_requirement_up_to(lhs_len, rhs_len)
}

/// Extended greatest common divisor for two multi-digit integers
///
/// The GCD result is stored in g (need to be pre-allocated and zero filled), while the Bézout coefficient
/// for the two operands is stored in the input slices, and the sign of the two coefficients are returned.
///
/// Specifically if g = gcd(lhs, rhs), lhs * a + rhs * b = g, then a is stored in **rhs**, b is stored in **lhs**,
/// and the returned tuple is (sign of a, sign of b)
pub fn gcd_ext_in_place(
    lhs: &mut [Word],
    rhs: &mut [Word],
    g: &mut [Word],
    bonly: bool,
    memory: &mut Memory,
) -> (Sign, Sign) {
    // binary::gcd_ext_in_place(lhs, rhs, g, bonly, memory)
    let (_g_len, swapped) = naive::gcd_ext_in_place(lhs, rhs, g, bonly, memory);
    if swapped {
        (Sign::Negative, Sign::Positive)
    } else {
        (Sign::Positive, Sign::Negative)
    }
}

/// Memory requirement for extended GCD.
pub fn memory_requirement_ext_exact(lhs_len: usize, rhs_len: usize) -> Layout {
    // binary::memory_requirement_ext_up_to(lhs_len, rhs_len)
    naive::memory_requirement_ext_up_to(lhs_len, rhs_len)
}
