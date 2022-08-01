//! Square.

use alloc::alloc::Layout;

use crate::{mul, arch::word::Word, memory::{Memory, self}, Sign};

mod simple;

/// If operand length <= this, simple squaring will be used.
const MAX_LEN_SIMPLE: usize = 30;

pub fn memory_requirement_exact(len: usize) -> Layout {
    if len <= MAX_LEN_SIMPLE {
        memory::zero_layout()
    } else {
        mul::memory_requirement_up_to(2 * len, len)
    }
}

/// b = a * a
pub fn square<'a>(
    b: &mut [Word],
    a: &'a [Word],
    memory: &mut Memory,
) {
    debug_assert!(a.len() >= 2);
    debug_assert!(b.len() == a.len() * 2);
    debug_assert!(b.iter().all(|&v| v == 0));

    if a.len() < MAX_LEN_SIMPLE {
        simple::square(b, a);
    } else {
        let carry = mul::add_signed_mul(b, Sign::Positive, a, a, memory);
        debug_assert!(carry == 0);
    }
}
