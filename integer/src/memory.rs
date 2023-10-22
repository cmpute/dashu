//! Memory allocation.

use crate::error::panic_allocate_too_much;
use crate::Word;
use alloc::vec::Vec;
use core::fmt;
use core::mem::{transmute, MaybeUninit};

/// Chunk of memory directly allocated from the global allocator.
pub struct MemoryAllocation<T: Copy> {
    storage: Vec<T>,
}

/// Chunk of memory.
pub struct Memory<'a, T: Copy = Word> {
    slice: &'a mut [MaybeUninit<T>],
}

impl<T: Copy> fmt::Debug for Memory<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Memory chunk ({} items)", self.slice.len())
    }
}

impl<T: Copy> MemoryAllocation<T> {
    /// Allocate memory.
    pub fn new(capacity: usize) -> Self {
        Self {
            storage: Vec::with_capacity(capacity),
        }
    }

    /// Get memory.
    #[inline]
    pub fn memory(&mut self) -> Memory<T> {
        Memory {
            slice: self.storage.spare_capacity_mut(),
        }
    }
}

impl<T: Copy> Memory<'_, T> {
    /// Allocate a slice with a given value.
    ///
    /// Returns the remaining chunk of memory.
    ///
    /// The original memory is not usable until both the new memory and the slice are dropped.
    ///
    /// The elements of the slice never get dropped!
    pub fn allocate_slice_fill(&mut self, n: usize, val: T) -> (&mut [T], Memory<'_, T>) {
        self.allocate_slice_initialize(n, |slice| {
            for item in slice.iter_mut() {
                item.write(val);
            }
            // SAFETY: Slice has just been initialized
            unsafe { transmute(slice) }
        })
    }

    /// Allocate a slice by copying another slice.
    ///
    /// Returns the remaining chunk of memory.
    ///
    /// The original memory is not usable until both the new memory and the slice are dropped.
    ///
    /// The elements of the slice never get dropped!
    pub fn allocate_slice_copy(&mut self, source: &[T]) -> (&mut [T], Memory<'_, T>) {
        self.allocate_slice_initialize(source.len(), |slice| {
            for (item, &source) in slice.iter_mut().zip(source) {
                item.write(source);
            }
            // SAFETY: Slice has just been initialized
            unsafe { transmute(slice) }
        })
    }

    /// Allocate a slice by copying a Smaller slice and filling the remainder with a value.
    ///
    /// Returns the remaining chunk of memory.
    ///
    /// The original memory is not usable until both the new memory and the slice are dropped.
    ///
    /// The elements of the slice never get dropped!
    pub fn allocate_slice_copy_fill(
        &mut self,
        n: usize,
        source: &[T],
        val: T,
    ) -> (&mut [T], Memory<'_, T>) {
        assert!(n >= source.len());

        self.allocate_slice_initialize(n, |slice| {
            for (item, &source) in slice.iter_mut().zip(source) {
                item.write(source);
            }
            for item in slice[source.len()..].iter_mut() {
                item.write(val);
            }
            // SAFETY: Slice has just been initialized
            unsafe { transmute(slice) }
        })
    }

    /// First allocate a slice of size n, and then initialize the memory with `F`.
    /// The initializer `F` must ensure that all allocated words are initialized.
    fn allocate_slice_initialize<F>(&mut self, n: usize, init: F) -> (&mut [T], Memory<'_, T>)
    where
        F: FnOnce(&mut [MaybeUninit<T>]) -> &mut [T],
    {
        let (slice, remaining) = self.slice.split_at_mut(n);
        let slice = init(slice);

        let new_memory = Memory { slice: remaining };

        (slice, new_memory)
    }
}

pub fn add_capacity(a: usize, b: usize) -> usize {
    a.checked_add(b)
        .unwrap_or_else(|| panic_allocate_too_much())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory() {
        let mut scratchpad = MemoryAllocation::<u32>::new(2);
        let mut memory = scratchpad.memory();
        let (a, mut new_memory) = memory.allocate_slice_fill(1, 3);
        assert_eq!(a, &[3]);
        // Neither of these should compile:
        // let _ = scratchpad.memory();
        // let _ = memory.allocate_slice(1, 3);
        let (b, _) = new_memory.allocate_slice_fill(1, 4);
        assert_eq!(b, &[4]);
        // Now we can reuse the memory.
        let (c, _) = memory.allocate_slice_copy(&[4, 5]);
        assert_eq!(c, &[4, 5]);
        // Reuse the memory again.
        let (c, _) = memory.allocate_slice_copy_fill(2, &[4], 7);
        assert_eq!(c, &[4, 7]);
    }

    #[test]
    #[should_panic]
    fn test_memory_ran_out() {
        let mut scratchpad = MemoryAllocation::<u32>::new(2);
        let mut memory = scratchpad.memory();
        let (a, mut new_memory) = memory.allocate_slice_fill(1, 3);
        assert_eq!(a, &[3]);
        let _ = new_memory.allocate_slice_fill(2, 4);
    }

    #[test]
    fn test_add_capacity() {
        let capacity = add_capacity(1, 8);
        assert_eq!(capacity, 9);
    }
}
