//! Integer roots

use alloc::alloc::Layout;

use crate::{
    arch::word::Word,
    memory::{self, Memory},
};

mod karatsuba;
mod newton;

/// The memory requirement for the n-th root of the integer
pub fn memory_requirement_exact(len: usize, n: usize) -> Layout {
    debug_assert!(n > 1);

    if n <= 3 {
        karatsuba::memory_requirement_exact(len, n)
    } else {
        unimplemented!()
    }
}

/// b = floor(sqrt(a)), remainder r = a^2 - b, it will be put in
/// the low words in a.
fn sqrt_rem(b: &mut [Word], a: &mut [Word], memory: &mut Memory) {
    debug_assert!(a.len() >= 2, "use native sqrt when a is small");
    debug_assert!(a.len() == b.len() * 2);

    let r_top = karatsuba::sqrt_rem(b, a, memory);
}
