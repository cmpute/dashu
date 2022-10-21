//! Word buffer.

use crate::{
    arch::word::{DoubleWord, Word},
    error::{panic_allocate_too_much, panic_out_of_memory},
    primitive::{double_word, WORD_BITS_USIZE},
};
use alloc::{alloc::Layout, boxed::Box};
use core::{
    fmt,
    hash::{Hash, Hasher},
    mem,
    ops::{Deref, DerefMut},
    ptr::{self, NonNull},
    slice,
};

/// Buffer of words allocated on heap. It's like a `Vec<Word>` with functionalities specialized for words.
///
/// This struct is ensured to be consistent with [Repr][crate::repr::Repr] in struct layout
/// (that's why `repr(C)` is necessary), but the big integer represented by this buffer is unsigned.
///
/// UBig operations are usually performed by creating a Buffer with appropriate capacity, filling it
/// in with Words, and then converting to UBig.
///
/// If its capacity is exceeded, the `Buffer` will panic.
#[repr(C)]
pub struct Buffer {
    ptr: NonNull<Word>,
    len: usize,
    capacity: usize,
}

// SAFETY: the pointer to the allocated space is uniquely owned by this struct.
unsafe impl Send for Buffer {}

// SAFETY: we don't provide interior mutability for Repr and Buffer
unsafe impl Sync for Buffer {}

impl Buffer {
    /// Maximum number of `Word`s.
    ///
    /// This ensures that the number of **bits** fits in `usize`, which is useful for bit count
    /// operations, and for radix conversions (even base 2 can be represented).
    ///
    /// Furthermore, this also ensures that the capacity of the buffer won't exceed isize::MAX,
    /// and ensures the safety for pointer movement.
    pub const MAX_CAPACITY: usize = usize::MAX / WORD_BITS_USIZE;

    /// Default capacity for a given number of `Word`s.
    /// It should be between `num_words` and `max_compact_capacity(num_words).
    ///
    /// Requires that `num_words <= MAX_CAPACITY`.
    ///
    /// Provides `2 + 0.125 * num_words` extra space.
    #[inline]
    pub fn default_capacity(num_words: usize) -> usize {
        debug_assert!(num_words <= Self::MAX_CAPACITY);
        (num_words + num_words / 8 + 2).min(Self::MAX_CAPACITY)
    }

    /// Maximum capacity for a given number of `Word`s to be considered as `compact`.
    ///
    /// Requires that `num_words <= Buffer::MAX_CAPACITY`.
    ///
    /// Allows `4 + 0.25 * num_words` overhead.
    #[inline]
    pub fn max_compact_capacity(num_words: usize) -> usize {
        debug_assert!(num_words <= Self::MAX_CAPACITY);
        (num_words + num_words / 4 + 4).min(Self::MAX_CAPACITY)
    }

    /// Return buffer capacity.
    ///
    /// The capacity will not be zero even if the numeric value represented by the buffer is 0.
    /// (the capacity is still 1 in this case)
    #[inline]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Return the length of words contained in the buffer
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Allocates words on heap, return the pointer and allocated size,
    /// the caller needs to handle the deallocation of the words.
    ///
    /// This function should NOT BE EXPOSED to public!
    #[inline]
    pub fn allocate_raw(capacity: usize) -> NonNull<Word> {
        debug_assert!(capacity <= Self::MAX_CAPACITY);

        unsafe {
            let layout = Layout::array::<Word>(capacity).unwrap();
            let ptr = alloc::alloc::alloc(layout);
            if ptr.is_null() {
                panic_out_of_memory();
            }
            NonNull::new(ptr).unwrap().cast()
        }
    }

    /// Deallocates the words on heap. The caller must make sure the ptr is valid.
    ///
    /// This function should NOT BE EXPOSED to public!
    #[inline]
    pub unsafe fn deallocate_raw(ptr: NonNull<Word>, capacity: usize) {
        let layout = Layout::array::<Word>(capacity).unwrap();
        alloc::alloc::dealloc(ptr.as_ptr() as _, layout);
    }

    /// Creates a `Buffer` with at least specified capacity.
    ///
    /// It leaves some extra space for future growth, and it allocates several words
    /// even if `num_words` is zero.
    #[inline]
    pub fn allocate(num_words: usize) -> Self {
        Self::allocate_exact(Self::default_capacity(num_words))
    }

    /// Creates a `Buffer` with exactly specified capacity (in words).
    pub fn allocate_exact(capacity: usize) -> Self {
        if capacity > Self::MAX_CAPACITY {
            panic_allocate_too_much()
        }

        let ptr = Self::allocate_raw(capacity);
        Buffer {
            capacity,
            ptr,
            len: 0,
        }
    }

    /// Change capacity to the given value
    ///
    /// # Panics
    ///
    /// Panics if `capacity < len()`.
    fn reallocate_raw(&mut self, capacity: usize) {
        debug_assert!(capacity >= self.len());

        unsafe {
            let old_layout = Layout::array::<Word>(self.capacity).unwrap();
            let new_layout = Layout::array::<Word>(capacity).unwrap();
            let new_ptr =
                alloc::alloc::realloc(self.ptr.as_ptr() as _, old_layout, new_layout.size());

            // update allocation info
            self.ptr = NonNull::new(new_ptr).unwrap().cast();
            self.capacity = capacity;
        }
    }

    /// Change capacity to store `num_words` plus some extra space for future growth.
    ///
    /// Note that it's advised to prevent calling this function when capacity = num_words
    ///
    /// # Panics
    ///
    /// Panics if `num_words < len()`.
    #[inline]
    fn reallocate(&mut self, num_words: usize) {
        assert!(num_words >= self.len());
        self.reallocate_raw(Self::default_capacity(num_words));
    }

    /// Ensure there is enough capacity in the buffer for `num_words`,
    /// reallocate if necessary.
    #[inline]
    pub fn ensure_capacity(&mut self, num_words: usize) {
        if num_words > self.capacity && num_words > 2 {
            self.reallocate(num_words);
        }
    }

    /// Ensure there is enough capacity that is not less than the given value,
    /// reallocate if necessary.
    #[inline]
    pub fn ensure_capacity_exact(&mut self, capacity: usize) {
        if capacity > self.capacity && capacity > 2 {
            self.reallocate_raw(capacity);
        }
    }

    /// Makes sure that the capacity is compact for existing data.
    #[inline]
    pub fn shrink_to_fit(&mut self) {
        if self.capacity > Self::max_compact_capacity(self.len) {
            self.reallocate(self.len);
        }
    }

    /// Append a Word to the buffer.
    ///
    /// # Panics
    ///
    /// Panics if there is not enough capacity.
    #[inline]
    pub fn push(&mut self, word: Word) {
        assert!(self.len < self.capacity);

        unsafe {
            let end = self.ptr.as_ptr().add(self.len);
            core::ptr::write(end, word);
            self.len += 1;
        }
    }

    /// Append a Word and reallocate if necessary. No-op if word is 0.
    #[inline]
    pub fn push_resizing(&mut self, word: Word) {
        if word != 0 {
            self.ensure_capacity(self.len + 1);
            self.push(word);
        }
    }

    /// Append `n` zeros.
    ///
    /// # Panics
    ///
    /// Panics if there is not enough capacity.
    pub fn push_zeros(&mut self, n: usize) {
        assert!(n <= self.capacity - self.len);

        unsafe {
            let mut ptr = self.ptr.as_ptr().add(self.len);
            for _ in 0..n {
                ptr::write(ptr, 0);
                ptr = ptr.add(1);
            }
            self.len += n;
        }
    }

    /// Insert `n` zeros in front.
    ///
    /// # Panics
    ///
    /// Panics if there is not enough capacity.
    pub fn push_zeros_front(&mut self, n: usize) {
        assert!(n <= self.capacity - self.len);

        unsafe {
            // move data
            let mut ptr = self.ptr.as_ptr();
            ptr::copy(ptr, ptr.add(n), self.len);

            // fill zeros
            for _ in 0..n {
                ptr::write(ptr, 0);
                ptr = ptr.add(1);
            }
            self.len += n;
        }
    }

    /// Append words by copying from slice.
    ///
    /// # Panics
    ///
    /// Panics if there is not enough capacity.
    #[inline]
    pub fn push_slice(&mut self, words: &[Word]) {
        let (src_ptr, src_len) = (words.as_ptr(), words.len());
        assert!(src_len <= self.capacity - self.len);

        unsafe {
            ptr::copy_nonoverlapping(src_ptr, self.ptr.as_ptr().add(self.len), src_len);
            self.len += src_len;
        }
    }

    /// Pop leading zero words.
    #[inline]
    pub fn pop_zeros(&mut self) {
        unsafe {
            if self.len > 0 {
                // adjust len until leading zeros are removed
                let mut tail_ptr = self.ptr.as_ptr().add(self.len - 1);
                while ptr::read(tail_ptr) == 0 && self.len > 0 {
                    tail_ptr = tail_ptr.sub(1);
                    self.len -= 1;
                }
            }
        }
    }

    /// Truncate length to `len`.
    ///
    /// # Panics
    ///
    /// Panics if the current length is less than `len`
    #[inline]
    pub fn truncate(&mut self, len: usize) {
        assert!(self.len >= len);
        self.len = len;
    }

    /// Erase first n elements.
    #[inline]
    pub fn erase_front(&mut self, n: usize) {
        assert!(self.len >= n);

        let ptr = self.ptr.as_ptr();
        let new_len = self.len - n;
        unsafe {
            // move data
            ptr::copy(ptr.add(n), ptr, new_len);
        }
        self.len = new_len;
    }

    /// Get the first double word of the buffer, assuming the buffer has at least two words.
    ///
    /// # Panics
    ///
    /// Panics if the buffer is empty or has only 1 word
    #[inline]
    pub fn lowest_dword(&self) -> DoubleWord {
        assert!(self.len >= 2);

        unsafe {
            let lo = ptr::read(self.ptr.as_ptr());
            let hi = ptr::read(self.ptr.as_ptr().add(1));
            double_word(lo, hi)
        }
    }

    /// Get the mutable reference to the first double word of the buffer,
    /// assuming the buffer has at least two words.
    ///
    /// # Panics
    ///
    /// Panics if the buffer is empty or has only 1 word
    #[inline]
    pub fn lowest_dword_mut(&mut self) -> (&mut Word, &mut Word) {
        assert!(self.len >= 2);

        unsafe {
            let ptr = self.ptr.as_ptr();
            (&mut *ptr, &mut *ptr.add(1))
        }
    }

    /// Make the data in this [Buffer] a copy of another slice.
    ///
    /// It reallocates if capacity is too small.
    pub fn clone_from_slice(&mut self, src: &[Word]) {
        if self.capacity >= src.len() {
            // direct copy if the capacity is enough
            unsafe {
                // SAFETY: src.ptr and self.ptr are both properly allocated by `Buffer::allocate()`.
                //         src.ptr and self.ptr cannot alias, because the ptr should be uniquely owned by the Buffer
                ptr::copy_nonoverlapping(src.as_ptr(), self.ptr.as_ptr(), src.len());
            }
            self.len = src.len();
        } else {
            *self = Self::from(src);
        }
    }

    pub fn into_boxed_slice(self) -> Box<[Word]> {
        // reallocate with 0 size is UB
        if self.len == 0 {
            return Box::new([]);
        }

        unsafe {
            let me = mem::ManuallyDrop::new(self);

            // first shrink the buffer to tight
            // `Layout::array` cannot overflow here because self.capacity < Self::MAX_CAPACITY
            let old_layout = Layout::array::<Word>(me.capacity).unwrap();
            let new_layout = Layout::array::<Word>(me.len).unwrap();
            let new_ptr =
                alloc::alloc::realloc(me.ptr.as_ptr() as _, old_layout, new_layout.size());

            // then convert the ptr to boxed slice
            let slice = slice::from_raw_parts_mut(new_ptr as *mut Word, me.len);
            Box::from_raw(slice)
        }
    }

    // This method is meant for implementation of zeroize traits
    #[cfg(feature = "zeroize")]
    pub fn as_full_slice(&mut self) -> &mut [Word] {
        unsafe {
            slice::from_raw_parts_mut(self.ptr.as_mut(), self.capacity)
        }
    }
}

impl Clone for Buffer {
    /// New buffer will be sized as `Buffer::allocate(self.len())`.
    #[inline]
    fn clone(&self) -> Self {
        let mut new_buffer = Buffer::allocate(self.len);
        unsafe {
            // SAFETY: src.ptr and self.ptr are both properly allocated by `Buffer::allocate()`.
            //         src.ptr and self.ptr cannot alias, because the ptr should be uniquely owned by the Buffer
            let new_ptr = new_buffer.ptr.as_ptr();
            ptr::copy_nonoverlapping(self.ptr.as_ptr(), new_ptr, self.len);
        }
        new_buffer.len = self.len;
        new_buffer
    }

    /// Reallocating if capacity is too small or too large.
    #[inline]
    fn clone_from(&mut self, src: &Self) {
        if self.capacity >= src.len && self.capacity <= Buffer::max_compact_capacity(src.len) {
            // direct copy if the capacity is enough
            unsafe {
                // SAFETY: src.ptr and self.ptr are both properly allocated by `Buffer::allocate()`.
                //         src.ptr and self.ptr cannot alias, because the ptr should be uniquely owned by the Buffer
                ptr::copy_nonoverlapping(src.ptr.as_ptr(), self.ptr.as_ptr(), src.len);
            }
            self.len = src.len;
        } else {
            // this statement drops the old buffer and deallocates the memory
            *self = src.clone();
        }
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        unsafe {
            Self::deallocate_raw(self.ptr, self.capacity);
        }
    }
}

impl Deref for Buffer {
    type Target = [Word];

    #[inline]
    fn deref(&self) -> &[Word] {
        unsafe { slice::from_raw_parts(self.ptr.as_ptr(), self.len) }
    }
}

impl DerefMut for Buffer {
    #[inline]
    fn deref_mut(&mut self) -> &mut [Word] {
        unsafe { slice::from_raw_parts_mut(self.ptr.as_ptr(), self.len) }
    }
}

impl PartialEq for Buffer {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self[..] == other[..]
    }
}
impl Eq for Buffer {}

impl fmt::Debug for Buffer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl Hash for Buffer {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        (**self).hash(state);
    }
}

impl From<&[Word]> for Buffer {
    #[inline]
    fn from(words: &[Word]) -> Self {
        let mut buffer = Buffer::allocate(words.len());
        buffer.push_slice(words);
        buffer
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_capacity() {
        assert_eq!(Buffer::default_capacity(2), 4);
        assert_eq!(Buffer::default_capacity(1000), 1127);
    }

    #[test]
    fn test_max_compact_capacity() {
        assert_eq!(Buffer::max_compact_capacity(2), 6);
        assert_eq!(Buffer::max_compact_capacity(1000), 1254);
    }

    #[test]
    fn test_allocate() {
        let buffer = Buffer::allocate(1000);
        assert_eq!(buffer.len(), 0);
        assert_eq!(buffer.capacity(), Buffer::default_capacity(1000));
    }

    #[test]
    #[should_panic]
    fn test_allocate_too_large() {
        let _ = Buffer::allocate(Buffer::MAX_CAPACITY + 1);
    }

    #[test]
    fn test_ensure_capacity() {
        let mut buffer = Buffer::allocate(2);
        buffer.push(7);
        assert_eq!(buffer.capacity(), 4);
        buffer.ensure_capacity(4);
        assert_eq!(buffer.capacity(), 4);
        buffer.ensure_capacity(5);
        assert_eq!(buffer.capacity(), 7);
        assert_eq!(&buffer[..], [7]);
    }

    #[test]
    fn test_shrink() {
        let mut buffer = Buffer::allocate(100);
        buffer.push(7);
        buffer.push(8);
        buffer.push(9);
        buffer.shrink_to_fit();
        assert_eq!(buffer.capacity(), Buffer::default_capacity(3));
        assert_eq!(&buffer[..], [7, 8, 9]);
    }

    #[test]
    fn test_push_pop() {
        let mut buffer = Buffer::allocate(5);
        buffer.push(1);
        buffer.push(2);
        assert_eq!(&buffer[..], [1, 2]);

        buffer.push(0);
        buffer.push(0);
        buffer.pop_zeros();
        assert_eq!(&buffer[..], [1, 2]);
    }

    #[test]
    fn test_extend() {
        let mut buffer = Buffer::allocate(5);
        buffer.push(1);
        let list: [Word; 2] = [2, 3];
        buffer.push_slice(&list);
        assert_eq!(&buffer[..], [1, 2, 3]);
    }

    #[test]
    fn test_push_zeros() {
        let mut buffer = Buffer::allocate(5);
        buffer.push(1);
        buffer.push_zeros(2);
        assert_eq!(&buffer[..], [1, 0, 0]);
    }

    #[test]
    fn test_push_zeros_front() {
        let mut buffer = Buffer::allocate(5);
        buffer.push(1);
        buffer.push_zeros_front(2);
        assert_eq!(&buffer[..], [0, 0, 1]);
    }

    #[test]
    fn test_truncate() {
        let mut buffer = Buffer::allocate(5);
        buffer.push(1);
        buffer.push(2);
        buffer.push(3);
        buffer.truncate(1);
        assert_eq!(&buffer[..], [1]);
    }

    #[test]
    fn test_erase_front() {
        let mut buffer = Buffer::allocate(5);
        buffer.push(1);
        buffer.push(2);
        buffer.push(3);
        buffer.erase_front(2);
        assert_eq!(&buffer[..], [3]);
    }

    #[test]
    #[should_panic]
    fn test_push_failed() {
        let mut buffer = Buffer::allocate(2);
        for _ in 0..10 {
            buffer.push(7);
        }
    }

    #[test]
    fn test_push_resizing() {
        let mut buffer = Buffer::allocate(2);
        for _ in 0..10 {
            buffer.push_resizing(7);
        }
        assert_eq!(buffer.len(), 10);
    }

    #[test]
    fn test_into_boxed_slice() {
        // empty buffer
        let buffer = Buffer::allocate(2);
        let slice = buffer.into_boxed_slice();
        assert_eq!(slice.len(), 0);

        // full buffer
        let mut buffer = Buffer::allocate(2);
        buffer.push(1);
        buffer.push(2);
        let slice = buffer.into_boxed_slice();
        assert_eq!(*slice, [1, 2]);

        // partially filled buffer
        let mut buffer = Buffer::allocate(20);
        buffer.push(1);
        buffer.push(2);
        let slice = buffer.into_boxed_slice();
        assert_eq!(*slice, [1, 2]);
    }
}
