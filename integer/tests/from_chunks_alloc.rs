//! Memory usage of `UBig::from_chunks`.
//!
//! This test installs a global allocator, so it lives in its own test binary.

use dashu_int::UBig;
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};

/// An allocator recording the size of the largest single allocation.
struct Peak;
static MAX: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Peak {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        MAX.fetch_max(layout.size(), SeqCst);
        System.alloc(layout)
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout)
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        MAX.fetch_max(new_size, SeqCst);
        System.realloc(ptr, layout, new_size)
    }
}

#[global_allocator]
static GLOBAL: Peak = Peak;

// a single test in this binary, so that no other thread allocates concurrently
#[test]
fn test_from_chunks_allocation_is_proportional_to_result() {
    for (chunk_count, chunk_bits) in [(2usize, 1usize << 20), (2, (1 << 20) + 1), (9, 1 << 17)] {
        let chunks = vec![UBig::ONE; chunk_count];
        let expected =
            (0..chunk_count).fold(UBig::ZERO, |acc, i| acc + (UBig::ONE << (i * chunk_bits)));
        let result_bytes = ((chunk_count - 1) * chunk_bits / 8) + 8;

        MAX.store(0, SeqCst);
        let value = UBig::from_chunks(chunks.iter(), chunk_bits);
        let peak = MAX.load(SeqCst);

        assert_eq!(value, expected);
        assert!(
            peak <= 4 * result_bytes,
            "largest allocation is {} bytes for a result of {} bytes",
            peak,
            result_bytes
        );
    }
}
