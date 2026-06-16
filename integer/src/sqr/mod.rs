//! Square.

use alloc::alloc::Layout;
use static_assertions::const_assert;

use crate::{
    arch::word::Word,
    div,
    memory::{self, Memory},
};
#[cfg(not(any(force_bits = "16", target_pointer_width = "16")))]
use crate::{helper_macros::debug_assert_zero, Sign};

mod karatsuba;
#[cfg(not(any(force_bits = "16", target_pointer_width = "16")))]
mod ntt;
mod simple;
pub(crate) mod toom_3;

/// If operand length <= this, simple squaring will be used.
const THRESHOLD_SIMPLE_SQR_DEFAULT: usize = 30;
const_assert!(THRESHOLD_SIMPLE_SQR_DEFAULT + 1 >= karatsuba::MIN_LEN);

/// If operand length <= this, Karatsuba squaring will be used.
const THRESHOLD_KARATSUBA_SQR_DEFAULT: usize = 96;
const_assert!(THRESHOLD_KARATSUBA_SQR_DEFAULT + 1 >= toom_3::MIN_LEN);

/// If operand length > this, NTT squaring will be used (64-bit targets only).
#[cfg(not(any(force_bits = "16", target_pointer_width = "16")))]
const THRESHOLD_NTT_SQR_DEFAULT: usize = crate::mul::ntt::THRESHOLD_NTT;
#[cfg(any(force_bits = "16", target_pointer_width = "16"))]
const THRESHOLD_NTT_SQR_DEFAULT: usize = usize::MAX;
#[cfg(not(any(force_bits = "16", target_pointer_width = "16")))]
const_assert!(THRESHOLD_NTT_SQR_DEFAULT + 1 >= toom_3::MIN_LEN);

/// Environment-variable overrides for squaring thresholds.
///
/// When the `tuning` feature is active the user may set `DASHU_THRESHOLD_SIMPLE_SQR`,
/// `DASHU_THRESHOLD_KARATSUBA_SQR` or `DASHU_THRESHOLD_NTT_SQR` to override the
/// compile-time defaults.
mod threshold {
    #[inline]
    pub fn simple() -> usize {
        #[cfg(feature = "tuning")]
        {
            if let Ok(s) = std::env::var("DASHU_THRESHOLD_SIMPLE_SQR") {
                if let Ok(v) = s.parse() {
                    return v;
                }
            }
        }
        super::THRESHOLD_SIMPLE_SQR_DEFAULT
    }
    #[inline]
    pub fn karatsuba() -> usize {
        #[cfg(feature = "tuning")]
        {
            if let Ok(s) = std::env::var("DASHU_THRESHOLD_KARATSUBA_SQR") {
                if let Ok(v) = s.parse() {
                    return v;
                }
            }
        }
        super::THRESHOLD_KARATSUBA_SQR_DEFAULT
    }
    #[inline]
    pub fn ntt() -> usize {
        #[cfg(feature = "tuning")]
        {
            if let Ok(s) = std::env::var("DASHU_THRESHOLD_NTT_SQR") {
                if let Ok(v) = s.parse() {
                    return v;
                }
            }
        }
        super::THRESHOLD_NTT_SQR_DEFAULT
    }
}

pub fn memory_requirement_exact(len: usize) -> Layout {
    if len <= threshold::simple() {
        memory::zero_layout()
    } else if len <= threshold::karatsuba() {
        karatsuba::memory_requirement_up_to(len)
    } else if len <= threshold::ntt() {
        toom_3::memory_requirement_up_to(len)
    } else {
        #[cfg(not(any(force_bits = "16", target_pointer_width = "16")))]
        {
            crate::mul::ntt::memory_requirement_up_to(2 * len, len)
        }
        #[cfg(any(force_bits = "16", target_pointer_width = "16"))]
        {
            let _ = len;
            unreachable!("NTT unavailable on 16/32-bit targets");
        }
    }
}

/// Scratch memory required to square an `n`-word operand and reduce the `2n`-word
/// product back to `n` words (i.e. square then divide, as the modular arithmetic does).
///
/// This is the squaring analogue of `mul::memory_requirement_exact(2n, n)` augmented
/// for the reduction step: it covers the `2n`-word product buffer and the larger of the
/// squaring scratch and the reduction scratch. Squaring needs more scratch than
/// multiplication in the Karatsuba band, so the modular code must use this rather than
/// the multiplication budget (otherwise the bump allocator is exhausted mid-recursion).
pub(crate) fn sqr_memory_requirement(n: usize) -> Layout {
    memory::add_layout(
        memory::array_layout::<Word>(2 * n),
        memory::max_layout(memory_requirement_exact(n), div::memory_requirement_exact(2 * n, n)),
    )
}

/// b = a * a. b must be filled with zeros. a.len() >= 2.
pub fn sqr(b: &mut [Word], a: &[Word], memory: &mut Memory) {
    debug_assert!(a.len() >= 2, "use native multiplication when a is small");
    debug_assert!(b.len() == a.len() * 2);
    debug_assert!(b.iter().all(|&v| v == 0));

    if a.len() <= threshold::simple() {
        simple::square(b, a);
    } else if a.len() <= threshold::karatsuba() {
        karatsuba::square(b, a, memory);
    } else if a.len() <= threshold::ntt() {
        toom_3::square(b, a, memory);
    } else {
        #[cfg(not(any(force_bits = "16", target_pointer_width = "16")))]
        {
            debug_assert_zero!(ntt::add_signed_sqr_same_len(b, Sign::Positive, a, memory));
        }
        #[cfg(any(force_bits = "16", target_pointer_width = "16"))]
        {
            let _ = (b, a, memory);
            unreachable!("NTT unavailable on 16/32-bit targets");
        }
    }
}
