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
mod toom_4;

/// If operand length <= this, simple squaring will be used.
const THRESHOLD_SIMPLE_SQR_DEFAULT: usize = 30;
const_assert!(THRESHOLD_SIMPLE_SQR_DEFAULT + 1 >= karatsuba::MIN_LEN);

/// If operand length <= this, Karatsuba squaring will be used.
const THRESHOLD_KARATSUBA_SQR_DEFAULT: usize = 96;
const_assert!(THRESHOLD_KARATSUBA_SQR_DEFAULT + 1 >= toom_3::MIN_LEN);

/// If operand length <= this, Toom-4 squaring will be used.
const THRESHOLD_TOOM4_SQR_DEFAULT: usize = 1000;
const_assert!(THRESHOLD_TOOM4_SQR_DEFAULT + 1 >= toom_4::MIN_LEN);
const_assert!(THRESHOLD_TOOM4_SQR_DEFAULT > THRESHOLD_KARATSUBA_SQR_DEFAULT);

/// If operand length > this, NTT squaring will be used (64-bit targets only).
#[cfg(not(any(force_bits = "16", target_pointer_width = "16")))]
const THRESHOLD_NTT_SQR_DEFAULT: usize = crate::mul::ntt::THRESHOLD_NTT;
#[cfg(any(force_bits = "16", target_pointer_width = "16"))]
const THRESHOLD_NTT_SQR_DEFAULT: usize = usize::MAX;
#[cfg(not(any(force_bits = "16", target_pointer_width = "16")))]
const_assert!(THRESHOLD_NTT_SQR_DEFAULT > THRESHOLD_TOOM4_SQR_DEFAULT);

/// Environment-variable overrides for squaring thresholds.
///
/// When the `tuning` feature is active the user may set `DASHU_THRESHOLD_SIMPLE_SQR`,
/// `DASHU_THRESHOLD_KARATSUBA_SQR`, `DASHU_THRESHOLD_TOOM4_SQR` or
/// `DASHU_THRESHOLD_NTT_SQR` to override the compile-time defaults.
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
    pub fn toom4() -> usize {
        #[cfg(feature = "tuning")]
        {
            if let Ok(s) = std::env::var("DASHU_THRESHOLD_TOOM4_SQR") {
                if let Ok(v) = s.parse() {
                    return v;
                }
            }
        }
        super::THRESHOLD_TOOM4_SQR_DEFAULT
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
    } else if len <= threshold::toom4() {
        toom_3::memory_requirement_up_to(len)
    } else if len <= threshold::ntt() {
        toom_4::memory_requirement_up_to(len)
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
    } else if a.len() <= threshold::toom4() {
        toom_3::square(b, a, memory);
    } else if a.len() <= threshold::ntt() {
        toom_4::square(b, a, memory);
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

#[cfg(all(test, feature = "std"))]
mod threshold_tests {
    use super::*;
    use crate::arch::word::Word;

    /// Compare toom-3 vs toom-4 squaring to find [`THRESHOLD_TOOM4_SQR`].
    /// Run with:
    ///   DASHU_THRESHOLD_TOOM4_SQR=99999999 cargo test -p dashu-int --features tuning --release \
    ///     -- sqr::threshold_tests::crossover_toom4_sqr --ignored --nocapture
    #[test]
    #[ignore]
    fn crossover_toom4_sqr() {
        use std::time::Instant;

        let sizes: &[usize] = &[400, 640, 800, 1000, 1300, 1600, 2000, 2600, 3200, 4000];

        println!("{:>8} {:>14} {:>14} {:>10}", "words", "toom-3(µs)", "toom-4(µs)", "ratio");
        println!("{}", "-".repeat(50));

        for &n in sizes {
            let a: Vec<Word> = (0..n)
                .map(|i| (i as Word + 1).wrapping_mul(0x9E3779B97F4A7C15u64 as Word))
                .collect();
            let mut b3 = vec![0 as Word; 2 * n];
            let mut b4 = vec![0 as Word; 2 * n];
            let l3 = toom_3::memory_requirement_up_to(n);
            let l4 = toom_4::memory_requirement_up_to(n);
            let layout = if l3.size() > l4.size() { l3 } else { l4 };
            let warmup = 5;
            let iters = 20;

            let time = |f: &mut dyn FnMut(&mut Memory)| {
                let mut best = f64::MAX;
                for _ in 0..warmup {
                    let mut alloc = crate::memory::MemoryAllocation::new(layout);
                    let mut mem = alloc.memory();
                    f(&mut mem);
                }
                for _ in 0..iters {
                    let mut alloc = crate::memory::MemoryAllocation::new(layout);
                    let mut mem = alloc.memory();
                    let start = Instant::now();
                    f(&mut mem);
                    let elapsed = start.elapsed().as_secs_f64() * 1_000_000.0;
                    if elapsed < best {
                        best = elapsed;
                    }
                }
                best
            };

            let t3 = time(&mut |mem| {
                b3.fill(0);
                toom_3::square(&mut b3, &a, mem);
            });
            let t4 = time(&mut |mem| {
                b4.fill(0);
                toom_4::square(&mut b4, &a, mem);
            });

            assert_eq!(&b3[..], &b4[..], "mismatch at n={n}");
            println!("{:>8} {:>14.1} {:>14.1} {:>9.2}x", n, t3, t4, t4 / t3);
        }
    }
}
