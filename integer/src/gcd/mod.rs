//! Greatest Common Divisor
use crate::{arch::word::Word, memory::Memory, sign::Sign};
use alloc::alloc::Layout;

mod binary;

/// Greatest common divisor for two multi-digit integers
///
/// The result is stored in the low bits of lhs.
/// The word length of the result number is returned.
pub fn gcd_in_place(lhs: &mut [Word], rhs: &mut [Word]) -> usize {
    if lhs.last().unwrap() == &0 || rhs.last().unwrap() == &0 {
        panic!("leading zero!")
    }
    binary::gcd_in_place(lhs, rhs)
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
    binary::gcd_ext_in_place(lhs, rhs, g, bonly, memory)
}

/// Memory requirement for GCD.
pub fn memory_requirement_exact(lhs_len: usize, rhs_len: usize) -> Layout {
    binary::memory_requirement_up_to(lhs_len, rhs_len)
}
