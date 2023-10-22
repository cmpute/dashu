//! Memory allocation.

use crate::error::{panic_allocate_too_much, panic_out_of_memory};
use crate::Word;
use alloc::alloc::Layout;
use core::{fmt, marker::PhantomData, slice};

/// Chunk of memory directly allocated from the global allocator.
pub struct MemoryAllocation<T: Copy> {
    capacity: usize,
    start: *mut T,
}

/// Chunk of memory.
pub struct Memory<'a, T: Copy = Word> {
    /// Start pointer.
    start: *mut T,
    /// Capacity.
    capacity: usize,
    /// Logically, Memory contains a reference to some data with lifetime 'a.
    phantom_data: PhantomData<&'a mut T>,
}

impl<T: Copy> fmt::Debug for Memory<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Memory chunk ({} items)", self.capacity)
    }
}

impl<T: Copy> MemoryAllocation<T> {
    /// Allocate memory.
    pub fn new(capacity: usize) -> Self {
        let start = if capacity == 0 {
            // We should use layout.dangling(), but that is unstable.
            core::ptr::NonNull::dangling().as_ptr()
        } else {
            let layout = Layout::array::<T>(capacity).unwrap_or_else(|_| panic_allocate_too_much());
            // SAFETY: it's checked above that layout.size() != 0.
            let ptr = unsafe { alloc::alloc::alloc(layout) };
            if ptr.is_null() {
                panic_out_of_memory();
            }
            ptr.cast()
        };

        Self { capacity, start }
    }

    /// Get memory.
    #[inline]
    pub fn memory(&mut self) -> Memory<T> {
        Memory {
            start: self.start,
            capacity: self.capacity,
            phantom_data: PhantomData,
        }
    }
}

impl<T: Copy> Drop for MemoryAllocation<T> {
    fn drop(&mut self) {
        if self.capacity != 0 {
            // SAFETY: the memory was allocated with the same layout.
            unsafe {
                alloc::alloc::dealloc(self.start.cast(), Layout::array::<T>(self.capacity).unwrap())
            };
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
        self.allocate_slice_initialize(n, |ptr| {
            for i in 0..n {
                // SAFETY: ptr is properly aligned and has enough space.
                unsafe {
                    ptr.add(i).write(val);
                };
            }
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
        self.allocate_slice_initialize(source.len(), |ptr| {
            for (i, v) in source.iter().enumerate() {
                // SAFETY: ptr is properly aligned and has enough space.
                unsafe {
                    ptr.add(i).write(*v);
                };
            }
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

        self.allocate_slice_initialize(n, |ptr| {
            for (i, v) in source.iter().enumerate() {
                // SAFETY: ptr is properly aligned and has enough space.
                unsafe {
                    ptr.add(i).write(*v);
                };
            }
            for i in source.len()..n {
                // SAFETY: ptr is properly aligned and has enough space.
                unsafe {
                    ptr.add(i).write(val);
                };
            }
        })
    }

    /// First allocate a slice of size n, and then initialize the memory with `F`.
    /// The initializer `F` must ensure that all allocated words are initialized.
    fn allocate_slice_initialize<F>(&mut self, n: usize, init: F) -> (&mut [T], Memory<'_, T>)
    where
        F: FnOnce(*mut T),
    {
        #[allow(clippy::redundant_closure)]
        let (ptr, slice_end) = self
            .try_find_memory_for_slice(n)
            .expect("internal error: not enough memory allocated");

        init(ptr);

        // SAFETY: ptr is properly sized and aligned guaranteed by `try_find_memory_for_slice`.
        let slice = unsafe { slice::from_raw_parts_mut(ptr, n) };
        let new_memory = Self {
            start: slice_end,
            capacity: self.capacity - n,
            phantom_data: PhantomData,
        };

        (slice, new_memory)
    }

    fn try_find_memory_for_slice(&self, n: usize) -> Option<(*mut T, *mut T)> {
        if n <= self.capacity {
            // SAFETY: We just checked there is enough capacity
            unsafe { Some((self.start, self.start.add(n))) }
        } else {
            None
        }
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
