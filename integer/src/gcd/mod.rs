//! Greatest Common Divisor
use crate::{arch::word::Word, memory::{self, Memory}, sign::Sign};
use alloc::alloc::Layout;

mod naive;
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
    naive::gcd_ext_in_place(lhs, rhs, memory)
}

/// Memory requirement for extended GCD.
pub fn memory_requirement_ext_exact(lhs_len: usize, rhs_len: usize) -> Layout {
    naive::memory_requirement_ext_up_to(lhs_len, rhs_len)
}
