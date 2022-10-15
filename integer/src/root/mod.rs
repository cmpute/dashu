//! Integer roots

use alloc::alloc::Layout;

mod karatsuba;
mod newton;

/// The memory requirement for the n-th root of the integer
pub fn memory_requirement_exact(len: usize, n: usize) -> Layout {
    debug_assert!(n > 1);

    match n {
        2 => karatsuba::memory_requirement_sqrt_rem(len / 2),
        _ => unimplemented!()
    }
}

pub use karatsuba::sqrt_rem;
