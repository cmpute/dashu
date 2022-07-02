//! Word buffer.

use crate::{
    arch::word::Word,
    primitive::WORD_BITS_USIZE
};

use alloc::{vec::Vec, alloc::Layout};
use core::{
    iter,
    ops::{Deref, DerefMut},
    num::NonZeroIsize,
    ptr::{self, NonNull},
};

/// This union contains the raw representation of words, the words are either inlined
/// or on the heap. The flag used to distinguishing them is the `len` field of the buffer.
union BufferData {
    inline: [Word; 2], // lo, hi
    heap: (*mut Word, usize) // ptr, len
}

/// Buffer for Words. The words in the buffer are ordered from LSB to MSB.
///
/// UBig operations are usually performed by creating a Buffer with appropriate capacity, filling it
/// in with Words, and then converting to UBig.
///
/// If its capacity is exceeded, the `Buffer` will panic.
pub(crate) struct BufferNew {
    data: BufferData,

    /// The capacity is designed to be not zero so that it provides a niche value for other use.
    /// 
    /// How to intepret the `data` field:
    /// - capacity = 1: the words are inlined and the high word is 0
    /// - capacity = 2: the words are inlined
    /// - capacity >= 3: the words are on allocated on the heap
    /// - capacity < 0: similiar to the cases above, but negative capacity value is used to mark the integer is negative.
    capacity: NonZeroIsize,
}

impl BufferNew {
    /// Creates a buffer with a single word
    pub(crate) fn from_word(n: Word) -> BufferNew {
        BufferNew { data: BufferData { inline: [n, 0] }, capacity: NonZeroIsize::new(1).unwrap() }
    }

    /// Creates a buffer with a double word represented in [lo, hi].
    pub(crate) fn from_dword(n: [Word; 2]) -> BufferNew {
        if n[1] == 0 {
            Self::from_word(n[0])
        } else {
            BufferNew { data: BufferData { inline: n }, capacity: NonZeroIsize::new(2).unwrap() }
        }
    }

    /// Maximum number of `Word`s.
    ///
    /// This ensures that the number of **bits** fits in `usize`, which is useful for bit count
    /// operations, and for radix conversions (even base 2 can be represented).
    /// 
    /// Furthermore, this also ensures that the capacity of the buffer won't exceed isize::MAX,
    /// and ensures the safety for pointer movement.
    pub(crate) const MAX_CAPACITY: usize = usize::MAX / WORD_BITS_USIZE;

    /// Default capacity for a given number of `Word`s.
    /// It should be between `num_words` and `max_compact_capacity(num_words).
    ///
    /// Requires that `num_words <= MAX_CAPACITY`.
    ///
    /// Provides `2 + 0.125 * num_words` extra space.
    #[inline]
    fn default_capacity(num_words: usize) -> usize {
        debug_assert!(num_words <= Self::MAX_CAPACITY);
        (num_words + num_words / 8 + 2).min(Self::MAX_CAPACITY)
    }
    
    /// Maximum capacity for a given number of `Word`s to be considered as `compact`.
    ///
    /// Requires that `num_words <= Buffer::MAX_CAPACITY`.
    ///
    /// Allows `4 + 0.25 * num_words` overhead.
    #[inline]
    fn max_compact_capacity(num_words: usize) -> usize {
        debug_assert!(num_words <= Self::MAX_CAPACITY);
        (num_words + num_words / 4 + 4).min(Self::MAX_CAPACITY)
    }

    /// Return buffer capacity.
    /// 
    /// The capacity will not be zero even if the numeric value represented by the buffer is 0.
    /// (the capacity is still 1 in this case) 
    #[inline]
    pub(crate) fn capacity(&self) -> usize {
        self.capacity.get().unsigned_abs()
    }

    #[inline]
    pub fn len(&self) -> usize {
        let cap = self.capacity();
        if cap > 2 {
            // SAFETY: the heap variant is guaranteed to be initialized when capacity > 2
            unsafe { self.data.heap.1 }
        } else {
            cap
        }
    }

    /// Returns a tuple with (data ptr, len ptr, capacity) only when the data is allocated on heap.
    #[inline]
    fn triplet(&self) -> (*const Word, usize, usize) {
        let capacity = self.capacity();
        debug_assert!(capacity > 2);

        unsafe {
            // SAFETY: the assertion above ensures the data is allocated on heap.
            //         The function is prive so debug_assert is sufficient.
            (self.data.heap.0, self.data.heap.1, capacity)
        }
    }

    /// Returns a mutable tuple with (data ptr, len ptr, capacity) only when the data is allocated on heap.
    #[inline]
    fn triplet_mut(&mut self) -> (*mut Word, &mut usize, usize) {
        let capacity = self.capacity();
        debug_assert!(capacity > 2);

        unsafe {
            // SAFETY: the assertion above ensures the data is allocated on heap.
            //         The function is prive so debug_assert is sufficient.
            let ptr = self.data.heap.0;
            let len = &mut self.data.heap.1;
            (ptr, len, capacity)
        }
    }

    /// Creates a `Buffer` with existing words, the words will be copied.
    ///
    /// It leaves some extra space for future growth.
    pub(crate) fn allocate(num_words: usize) -> BufferNew {
        assert!(num_words > 2, "single or double words should be inlined");
        if num_words > Self::MAX_CAPACITY {
            panic!("too many words to be allocated, maximum is {} bits", Self::MAX_CAPACITY);
        }

        unsafe {
            let capacity = Self::default_capacity(num_words);
            let layout = Layout::array::<Word>(capacity).unwrap();
            let ptr = alloc::alloc::alloc(layout);
            let alloc = NonNull::new(ptr).unwrap().cast().as_ptr();
            BufferNew {
                capacity: NonZeroIsize:: new_unchecked(capacity as isize),
                data: BufferData { heap: (alloc, 0) }
            }
        }
    }

    /// Change capacity to store `num_words` plus some extra space for future growth.
    /// Note that this function should not be called when capacity = num_words
    fn reallocate(&mut self, num_words: usize) {
        debug_assert!(num_words > self.len());
        // TODO: do we need to consider the case where the data is inlined

        unsafe {
            let old_layout = Layout::array::<Word>(self.capacity()).unwrap();
            let new_capacity = Self::default_capacity(num_words);
            let new_layout = Layout::array::<Word>(new_capacity).unwrap();
            let new_ptr = alloc::alloc::realloc(
                self.data.heap.0 as *mut u8,
                old_layout,
                new_layout.size()
            );

            self.data.heap.0 = NonNull::new(new_ptr).unwrap().cast().as_ptr();
            self.capacity = NonZeroIsize::new_unchecked(new_capacity as isize);
        }
    }
    
    /// Ensure there is enough capacity in the buffer for `num_words`,
    /// reallocate if necessary.
    #[inline]
    pub(crate) fn ensure_capacity(&mut self, num_words: usize) {
        if num_words > self.capacity() && num_words > 2 {
            self.reallocate(num_words);
        }
    }

    /// Makes sure that the capacity is compact. The words will
    /// be inlined if possible.
    #[inline]
    pub(crate) fn shrink(&mut self) {
        let (_, len, capacity) = self.triplet();

        if len < 3 {
            // shrink to inline if possible
            self.truncate(len);
        } else if capacity > Self::max_compact_capacity(len) {
            self.reallocate(len);
        }
    }

    /// Append a Word to the buffer (on heap).
    ///
    /// # Panics
    ///
    /// Panics if there is not enough capacity.
    #[inline]
    pub(crate) fn push(&mut self, word: Word) {
        let (ptr, len, capacity) = self.triplet_mut();
        assert!(*len < capacity);

        unsafe {
            let end = ptr.add(*len);
            core::ptr::write(end, word);
            *len += 1;
        }
    }

    /// Append a Word (on heap) and reallocate if necessary.
    #[inline]
    pub(crate) fn push_may_reallocate(&mut self, word: Word) {
        self.ensure_capacity(self.len() + 1);
        self.push(word);
    }

    /// Append `n` zeros (on heap).
    ///
    /// # Panics
    ///
    /// Panics if there is not enough capacity.
    pub(crate) fn push_zeros(&mut self, n: usize) {
        let (ptr, len, capacity) = self.triplet_mut();
        assert!(n <= capacity - *len);

        unsafe {
            let mut ptr = ptr.add(*len);
            for _ in 0..n {
                ptr::write(ptr, 0);
                ptr = ptr.add(1);
            }
            *len += n;
        }
    }

    /// Insert `n` zeros in front (on heap).
    ///
    /// # Panics
    ///
    /// Panics if there is not enough capacity.
    pub(crate) fn push_zeros_front(&mut self, n: usize) {
        let (ptr, len, capacity) = self.triplet_mut();
        assert!(n <= capacity - *len);

        unsafe {
            // move data
            ptr::copy(ptr, ptr.add(n), *len);

            // fill zeros
            let mut ptr = ptr;
            for _ in 0..n {
                ptr::write(ptr, 0);
                ptr = ptr.add(1);
            }
            *len += n;
        }
    }

    /// Pop leading zero words (on heap). The words will be inlined if possible.
    #[inline]
    pub(crate) fn pop_zeros(&mut self) {
        let (ptr, mut len, _) = self.triplet();

        unsafe {
            if len > 0 {
                // adjust len until leading zeros are removed
                let mut tail_ptr = ptr.add(len - 1);
                while ptr::read(tail_ptr) == 0 && len > 2 {
                    tail_ptr = tail_ptr.sub(1);
                    len -= 1;
                }
            }
        }

        self.truncate(len);
    }

    /// Truncate length to `len` (on heap). The words will be inlined
    /// if there are less than 3 words remaining.
    /// 
    /// # Panics
    /// 
    /// Panics if the current length is less than `len`
    #[inline]
    pub(crate) fn truncate(&mut self, len: usize) {
        let (ptr, cur_len, _) = self.triplet();
        assert!(cur_len >= len);

        unsafe {
            // inline the data if the length is less than 3
            match len {
                0 => { *self = Self::from_word(0); }
                1 => { *self = Self::from_word(ptr::read(ptr)); },
                2 => {
                    let (lo, hi) = (ptr::read(ptr), ptr::read(ptr.add(1)));
                    *self = Self::from_dword([lo, hi]);
                },
                _ => {
                    // len >= 3
                    self.data.heap.1 = len;
                }
            }
        }
    }

    /// Erase first n elements (on heap). The words will be inlined if possible.
    #[inline]
    pub(crate) fn erase_front(&mut self, n: usize) {
        let (ptr, len, _) = self.triplet_mut();
        assert!(*len >= n);

        let new_len = *len - n;
        unsafe {
            // move data
            ptr::copy(ptr.add(new_len), ptr, new_len);
        }
        self.truncate(new_len);
    }

    /// Clone from `src` and resize if necessary.
    ///
    /// Equivalent to, but more efficient than:
    ///
    /// ```ignore
    /// buffer.ensure_capacity(src.len());
    /// buffer.clone_from(src);
    /// buffer.shrink();
    /// ```
    pub(crate) fn resizing_clone_from(&mut self, src: &BufferNew) {
        let cap = self.capacity();
        let n = src.len();
        if cap >= n && cap <= BufferNew::max_compact_capacity(n) {
            self.clone_from(src);
        } else {
            *self = src.clone();
        }
    }

    /// Append words by copying from slice (on heap).
    /// 
    /// # Panics
    /// 
    /// Panics if there is not enough capacity.
    pub(crate) fn push_slice(&mut self, words: &[Word]) {
        let (ptr, len, capacity) = self.triplet_mut();
        let (src_ptr, src_len) = (words.as_ptr(), words.len());
        assert!(src_len <= capacity - *len);

        unsafe {
            ptr::copy_nonoverlapping(src_ptr, ptr.add(*len), src_len);
            *len += src_len;
        }
    }

}

impl Clone for BufferNew {
    /// New buffer will be sized as `Buffer::allocate(self.len())`.
    fn clone(&self) -> BufferNew {
        let mut new_buffer = BufferNew::allocate(self.len());
        new_buffer.clone_from(self);
        new_buffer
    }

    /// Panics if capacity is exceeded.
    #[inline]
    fn clone_from(&mut self, src: &BufferNew) {
        let (src_ptr, src_len, _) = src.triplet();
        let (ptr, len, capacity) = self.triplet_mut();

        unsafe {
            // inline the data if the length is less than 3
            match src_len {
                0 => { *self = Self::from_word(0); }
                1 => { *self = Self::from_word(ptr::read(src_ptr)); },
                2 => {
                    let (lo, hi) = (ptr::read(src_ptr), ptr::read(src_ptr.add(1)));
                    *self = Self::from_dword([lo, hi]);
                },
                _ => {
                    assert!(capacity >= src_len);
                    ptr::copy_nonoverlapping(src_ptr, ptr, src_len);
                    *len = src_len;

                    // copy the sign from src
                    self.capacity = NonZeroIsize::new_unchecked(self.capacity.get().abs() * src.capacity.get().signum());
                }
            }
        }
    }
}

impl Drop for BufferNew {
    fn drop(&mut self) {
        unsafe {
            let capacity = self.capacity();
            if capacity > 2 {
                let layout = Layout::array::<Word>(capacity).unwrap();
                alloc::alloc::dealloc(self.data.heap.0 as _, layout);
            }
        }
    }
}

impl Deref for BufferNew {
    type Target = [Word];

    #[inline]
    fn deref(&self) -> &[Word] {
        unsafe {
            match self.capacity() {
                0 => unreachable!(),
                1 => &self.data.inline[..1],
                2 => &self.data.inline,
                _ => core::slice::from_raw_parts(
                    self.data.heap.0,
                    self.data.heap.1
                )
            }
        }
    }
}

impl DerefMut for BufferNew {
    #[inline]
    fn deref_mut(&mut self) -> &mut [Word] {
        unsafe {
            match self.capacity() {
                0 => unreachable!(),
                1 => &mut self.data.inline[..1],
                2 => &mut self.data.inline,
                _ => core::slice::from_raw_parts_mut(
                    self.data.heap.0,
                    self.data.heap.1
                )
            }
        }
    }
}

/// Buffer for Words.
///
/// UBig operations are usually performed by creating a Buffer with appropriate capacity, filling it
/// in with Words, and then converting to UBig.
///
/// If its capacity is exceeded, the `Buffer` will panic.
#[derive(Debug, Eq, Hash, PartialEq)]
pub(crate) struct Buffer(Vec<Word>);

impl Buffer {
    /// Creates a `Buffer` with at least specified capacity.
    ///
    /// It leaves some extra space for future growth.
    pub(crate) fn allocate(num_words: usize) -> Buffer {
        if num_words > Buffer::MAX_CAPACITY {
            // UBig::panic_number_too_large();
            panic!()
        }
        Buffer(Vec::with_capacity(Buffer::default_capacity(num_words)))
    }

    /// Ensure there is enough capacity in the buffer for `num_words`. Will reallocate if there is
    /// not enough.
    #[inline]
    pub(crate) fn ensure_capacity(&mut self, num_words: usize) {
        if num_words > self.capacity() {
            self.reallocate(num_words);
        }
    }

    /// Makes sure that the capacity is compact.
    #[inline]
    pub(crate) fn shrink(&mut self) {
        if self.capacity() > Buffer::max_compact_capacity(self.len()) {
            self.reallocate(self.len());
        }
    }

    /// Change capacity to store `num_words` plus some extra space for future growth.
    ///
    /// # Panics
    ///
    /// Panics if `num_words < len()`.
    fn reallocate(&mut self, num_words: usize) {
        assert!(num_words >= self.len());
        let mut new_buffer = Buffer::allocate(num_words);
        new_buffer.clone_from(self);
        *self = new_buffer
    }

    /// Return buffer capacity.
    #[inline]
    pub(crate) fn capacity(&self) -> usize {
        self.0.capacity()
    }

    /// Append a Word to the buffer.
    ///
    /// # Panics
    ///
    /// Panics if there is not enough capacity.
    #[inline]
    pub(crate) fn push(&mut self, word: Word) {
        assert!(self.len() < self.capacity());
        self.0.push(word);
    }

    /// Append a Word and reallocate if necessary.
    #[inline]
    pub(crate) fn push_may_reallocate(&mut self, word: Word) {
        self.ensure_capacity(self.len() + 1);
        self.push(word);
    }

    /// Append `n` zeros.
    ///
    /// # Panics
    ///
    /// Panics if there is not enough capacity.
    pub(crate) fn push_zeros(&mut self, n: usize) {
        assert!(n <= self.capacity() - self.len());
        self.0.extend(iter::repeat(0).take(n));
    }

    /// Insert `n` zeros in front.
    ///
    /// # Panics
    ///
    /// Panics if there is not enough capacity.
    pub(crate) fn push_zeros_front(&mut self, n: usize) {
        assert!(n <= self.capacity() - self.len());
        self.0.splice(..0, iter::repeat(0).take(n));
    }

    /// Pop the most significant `Word`.
    #[inline]
    pub(crate) fn pop(&mut self) -> Option<Word> {
        self.0.pop()
    }

    /// Pop leading zero words.
    #[inline]
    pub(crate) fn pop_leading_zeros(&mut self) {
        while let Some(0) = self.last() {
            self.pop();
        }
    }

    #[inline]
    /// Truncate length to `len`.
    pub(crate) fn truncate(&mut self, len: usize) {
        assert!(self.len() >= len);

        self.0.truncate(len);
    }

    /// Erase first n elements.
    pub(crate) fn erase_front(&mut self, n: usize) {
        assert!(self.len() >= n);

        self.0.drain(..n);
    }

    /// Clone from `other` and resize if necessary.
    ///
    /// Equivalent to, but more efficient than:
    ///
    /// ```ignore
    /// buffer.ensure_capacity(source.len());
    /// buffer.clone_from(source);
    /// buffer.shrink();
    /// ```
    pub(crate) fn resizing_clone_from(&mut self, source: &Buffer) {
        let cap = self.capacity();
        let n = source.len();
        if cap >= n && cap <= Buffer::max_compact_capacity(n) {
            self.clone_from(source);
        } else {
            *self = source.clone();
        }
    }

    /// Maximum number of `Word`s.
    ///
    /// We allow 4 extra words beyond `UBig::MAX_LEN` to allow temporary space in operations.
    pub(crate) const MAX_CAPACITY: usize = crate::ubig::UBig::MAX_LEN + 4;

    /// Default capacity for a given number of `Word`s.
    /// It should be between `num_words` and `max_capacity(num_words).
    ///
    /// Requires that `num_words <= MAX_CAPACITY`.
    ///
    /// Provides `2 + 0.125 * num_words` extra space.
    #[inline]
    fn default_capacity(num_words: usize) -> usize {
        debug_assert!(num_words <= Buffer::MAX_CAPACITY);
        (num_words + num_words / 8 + 2).min(Buffer::MAX_CAPACITY)
    }

    /// Maximum compact capacity for a given number of `Word`s.
    ///
    /// Requires that `num_words <= Buffer::MAX_CAPACITY`.
    ///
    /// Allows `4 + 0.25 * num_words` overhead.
    #[inline]
    fn max_compact_capacity(num_words: usize) -> usize {
        debug_assert!(num_words <= Buffer::MAX_CAPACITY);
        (num_words + num_words / 4 + 4).min(Buffer::MAX_CAPACITY)
    }
}

impl Clone for Buffer {
    /// New buffer will be sized as `Buffer::allocate(self.len())`.
    fn clone(&self) -> Buffer {
        let mut new_buffer = Buffer::allocate(self.len());
        new_buffer.clone_from(self);
        new_buffer
    }

    /// If capacity is exceeded, panic.
    #[inline]
    fn clone_from(&mut self, source: &Buffer) {
        assert!(self.capacity() >= source.len());
        self.0.clone_from(&source.0);
    }
}

impl Deref for Buffer {
    type Target = [Word];

    #[inline]
    fn deref(&self) -> &[Word] {
        &self.0
    }
}

impl DerefMut for Buffer {
    #[inline]
    fn deref_mut(&mut self) -> &mut [Word] {
        &mut self.0
    }
}

impl Extend<Word> for Buffer {
    fn extend<T>(&mut self, iter: T)
    where
        T: IntoIterator<Item = Word>,
    {
        for word in iter {
            self.push(word);
        }
    }
}

impl<'a> Extend<&'a Word> for Buffer {
    fn extend<T>(&mut self, iter: T)
    where
        T: IntoIterator<Item = &'a Word>,
    {
        for word in iter {
            self.push(*word);
        }
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
        buffer.shrink();
        assert_eq!(buffer.capacity(), Buffer::default_capacity(2));
        assert_eq!(&buffer[..], [7, 8]);
    }

    #[test]
    fn test_push_pop() {
        let mut buffer = Buffer::allocate(5);
        buffer.push(1);
        buffer.push(2);
        assert_eq!(&buffer[..], [1, 2]);
        assert_eq!(buffer.pop(), Some(2));
        assert_eq!(buffer.pop(), Some(1));
        assert_eq!(buffer.pop(), None);
    }

    #[test]
    fn test_pop_leading_zeros() {
        let mut buffer = Buffer::allocate(5);
        buffer.push(1);
        buffer.push(2);
        buffer.push(0);
        buffer.push(0);
        buffer.pop_leading_zeros();
        assert_eq!(&buffer[..], [1, 2]);
    }

    #[test]
    fn test_extend() {
        let mut buffer = Buffer::allocate(5);
        buffer.push(1);
        let list: [Word; 2] = [2, 3];
        buffer.extend(&list);
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
    fn test_push_may_reallocate() {
        let mut buffer = Buffer::allocate(2);
        for _ in 0..10 {
            buffer.push_may_reallocate(7);
        }
        assert_eq!(buffer.len(), 10);
    }

    #[test]
    fn test_clone() {
        let mut buffer = Buffer::allocate(100);
        buffer.push(7);
        buffer.push(8);
        let buffer2 = buffer.clone();
        assert_eq!(buffer, buffer2);
        assert_eq!(buffer2.capacity(), Buffer::default_capacity(2));
    }

    #[test]
    fn test_clone_from() {
        let mut buffer = Buffer::allocate(100);
        buffer.push(7);
        buffer.push(8);
        let mut buffer2 = Buffer::allocate(50);
        buffer2.clone_from(&buffer);
        assert_eq!(buffer, buffer2);
        assert_eq!(buffer2.capacity(), Buffer::default_capacity(50));
    }

    #[test]
    fn test_resizing_clone_from() {
        let mut buf = Buffer::allocate(5);
        assert_eq!(buf.capacity(), 7);

        let mut buf2 = Buffer::allocate(4);
        assert_eq!(buf2.capacity(), 6);
        for i in 0..4 {
            buf2.push(i);
        }
        buf.resizing_clone_from(&buf2);
        assert_eq!(buf.capacity(), 7);
        assert_eq!(&buf[..], [0, 1, 2, 3]);

        let mut buf3 = Buffer::allocate(100);
        for i in 0..100 {
            buf3.push(i);
        }
        buf.resizing_clone_from(&buf3);
        assert_eq!(buf.capacity(), Buffer::default_capacity(100));
        assert_eq!(buf.len(), 100);

        buf.resizing_clone_from(&buf2);
        assert_eq!(buf.capacity(), 6);
        assert_eq!(&buf[..], [0, 1, 2, 3]);
    }
}
