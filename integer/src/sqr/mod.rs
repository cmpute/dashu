//! Square.

use alloc::alloc::Layout;

use crate::{
    arch::word::Word,
    helper_macros::debug_assert_zero,
    memory::{self, Memory},
    mul, Sign,
};

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

/// b = a * a, b must be filled with zeros.
pub fn sqr(b: &mut [Word], a: &[Word], memory: &mut Memory) {
    debug_assert!(a.len() >= 2, "use native multiplication when a is small");
    debug_assert!(b.len() == a.len() * 2);
    debug_assert!(b.iter().all(|&v| v == 0));

    if a.len() <= MAX_LEN_SIMPLE {
        simple::square(b, a);
    } else {
        debug_assert_zero!(mul::add_signed_mul_same_len(b, Sign::Positive, a, a, memory));
    }
}
