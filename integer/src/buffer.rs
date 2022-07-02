//! Word buffer. TODO: rename to repr.rs

use crate::{
    arch::word::{Word, DoubleWord},
    primitive::{WORD_BITS_USIZE, double_word, split_double_word},
    sign::Sign,
};
use static_assertions::const_assert_eq;
use alloc::{vec::Vec, alloc::Layout};
use core::{
    slice,
    fmt::{self, Write},
    mem,
    ops::{Deref, DerefMut},
    num::NonZeroIsize,
    ptr::{self, NonNull},
    hash::{Hash, Hasher}
};

/// This union contains the raw representation of words, the words are either inlined
/// or on the heap. The flag used to distinguishing them is the `len` field of the buffer.
#[repr(C)]
union ReprData {
    inline: [Word; 2], // lo, hi
    heap: (*mut Word, usize) // ptr, len
}

/// Internal representation for big integers.
/// 
/// It's optimized so that Single integers (single or double words) will not be allocated on heap.
/// When the data is allocated on the heap, it can be casted to [Buffer] efficiently, but modifying
/// the buffer inplace is not allowed because that can break the rule on the `capacity` field.
#[repr(C)]
pub(crate) struct Repr {
    /// The capacity is designed to be not zero so that it provides a niche value for other use.
    /// 
    /// How to intepret the `data` field:
    /// - capacity = 1: the words are inlined and the high word is 0
    /// - capacity = 2: the words are inlined
    /// - capacity >= 3: the words are on allocated on the heap. In this case, data.len >= 3 will also be forced.
    /// - capacity < 0: similiar to the cases above, but negative capacity value is used to mark the integer is negative.
    capacity: NonZeroIsize,

    /// The words in the `data` field are ordered from LSB to MSB.
    data: ReprData,
}

/// Buffer of words allocated on heap.
/// 
/// This struct is ensured to be consistent with [Repr] in struct layout (that's why `repr(C)` is necessary),
/// but the big integer represented by this buffer is unsigned.
/// 
/// UBig operations are usually performed by creating a Buffer with appropriate capacity, filling it
/// in with Words, and then converting to UBig.
///
/// If its capacity is exceeded, the `Buffer` will panic.
#[repr(C)]
pub(crate) struct Buffer {
    capacity: usize,
    ptr: NonNull<Word>,
    len: usize
}
const_assert_eq!(mem::size_of::<Buffer>(), mem::size_of::<Repr>());


pub(crate) enum StrongRepr {
    Single(Word),
    Double(DoubleWord),
    Large(Buffer)
}

pub(crate) enum StrongReprRef<'a> {
    RefSingle(Word),
    RefDouble(DoubleWord),
    RefLarge(&'a [Word])
}

impl Buffer {
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
        self.capacity
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Creates a `Buffer` with existing words, the words will be copied.
    ///
    /// It leaves some extra space for future growth.
    pub(crate) fn allocate(num_words: usize) -> Self {
        if num_words > Self::MAX_CAPACITY {
            panic!("too many words to be allocated, maximum is {} bits", Self::MAX_CAPACITY);
        }

        unsafe {
            let len = num_words;
            let capacity = Self::default_capacity(len);
            let layout = Layout::array::<Word>(capacity).unwrap();
            let ptr = alloc::alloc::alloc(layout);
            let ptr = NonNull::new(ptr).unwrap().cast();
            Buffer { capacity, ptr, len }
        }
    }

    /// Change capacity to store `num_words` plus some extra space for future growth.
    /// Note that this function should not be called when capacity = num_words
    fn reallocate(&mut self, num_words: usize) {
        debug_assert!(num_words >= self.len());

        unsafe {
            let old_layout = Layout::array::<Word>(self.capacity).unwrap();
            let new_capacity = Self::default_capacity(num_words);
            let new_layout = Layout::array::<Word>(new_capacity).unwrap();
            let new_ptr = alloc::alloc::realloc(
                self.ptr.as_ptr() as _,
                old_layout,
                new_layout.size()
            );

            // update allocation info
            self.ptr = NonNull::new(new_ptr).unwrap().cast();
            self.capacity = new_capacity;
        }
    }
    
    /// Ensure there is enough capacity in the buffer for `num_words`,
    /// reallocate if necessary.
    #[inline]
    pub(crate) fn ensure_capacity(&mut self, num_words: usize) {
        if num_words > self.capacity && num_words > 2 {
            self.reallocate(num_words);
        }
    }

    // TODO: what's the optimal strategy to shrink the integer when casting to UBig?
    /// Makes sure that the capacity is compact.
    #[inline]
    pub(crate) fn shrink(&mut self) {
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
    pub(crate) fn push(&mut self, word: Word) {
        assert!(self.len < self.capacity);

        unsafe {
            let end = self.ptr.as_ptr().add(self.len);
            core::ptr::write(end, word);
            self.len += 1;
        }
    }

    /// Append a Word and reallocate if necessary.
    #[inline]
    pub(crate) fn push_may_reallocate(&mut self, word: Word) {
        self.ensure_capacity(self.len + 1);
        self.push(word);
    }

    /// Append `n` zeros.
    ///
    /// # Panics
    ///
    /// Panics if there is not enough capacity.
    pub(crate) fn push_zeros(&mut self, n: usize) {
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
    pub(crate) fn push_zeros_front(&mut self, n: usize) {
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
    pub(crate) fn push_slice(&mut self, words: &[Word]) {
        let (src_ptr, src_len) = (words.as_ptr(), words.len());
        assert!(src_len <= self.capacity - self.len);

        unsafe {
            ptr::copy_nonoverlapping(src_ptr, self.ptr.as_ptr().add(self.len), src_len);
            self.len += src_len;
        }
    }

    /// Pop leading zero words.
    #[inline]
    pub(crate) fn pop_zeros(&mut self) {
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
    pub(crate) fn truncate(&mut self, len: usize) {
        assert!(self.len >= len);
        self.len = len;
    }

    /// Erase first n elements.
    #[inline]
    pub(crate) fn erase_front(&mut self, n: usize) {
        assert!(self.len >= n);

        let ptr = self.ptr.as_ptr();
        let new_len = self.len - n;
        unsafe {
            // move data
            ptr::copy(ptr.add(n), ptr, new_len);
        }
    }

    // /// Clone from `src` and resize if necessary.
    // ///
    // /// Equivalent to, but more efficient than:
    // ///
    // /// ```ignore
    // /// buffer.ensure_capacity(src.len());
    // /// buffer.clone_from(src);
    // /// buffer.shrink();
    // /// ```
    // pub(crate) fn resizing_clone_from(&mut self, src: &Buffer) {
    //     if self.capacity >= src.len && self.capacity <= Buffer::max_compact_capacity(src.len) {
    //         self.clone_from(src);
    //     } else {
    //         *self = src.clone();
    //     }
    // }
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

    /// Reallocating if capacity is too Single or too large.
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
            *self = src.clone();
        }
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        unsafe {
            let layout = Layout::array::<Word>(self.capacity).unwrap();
            alloc::alloc::dealloc(self.ptr.as_ptr() as _, layout);
        }
    }
}

impl Deref for Buffer {
    type Target = [Word];

    #[inline]
    fn deref(&self) -> &[Word] {
        unsafe {
            slice::from_raw_parts(
                self.ptr.as_ptr(),
                self.len
            )
        }
    }
}

impl DerefMut for Buffer {
    #[inline]
    fn deref_mut(&mut self) -> &mut [Word] {
        unsafe {
            slice::from_raw_parts_mut(
                self.ptr.as_ptr(),
                self.len
            )
        }
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
    fn hash<H: Hasher>(&self, state: &mut H) {
        (**self).hash(state);
    }
}

impl From<&[Word]> for Buffer {
    fn from(words: &[Word]) -> Self {
        let mut buffer = Buffer::allocate(words.len());
        buffer.push_slice(words);
        buffer
    }
}

/*******************************
    Implementations for Repr
********************************/

impl Repr {
    /// Get the length of the number (in `Word`s)
    #[inline]
    pub fn len(&self) -> usize {
        match self.capacity() {
            0 => unreachable!(),
            1 => 1,
            2 => 2,
            _ => unsafe { self.data.heap.1 }
        }
    }

    /// Get the capacity of the representation (in `Word`s)
    /// 
    /// It will not be zero even if the underlying number is zero.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.capacity.get().unsigned_abs()
    }

    /// Intepret the [Repr] as a single word and get its value.
    /// 
    /// SAFETY: You need to check the capacity field before accessing the union.
    #[inline]
    unsafe fn as_word(&self) -> Word {
        debug_assert!(self.capacity() == 1);
        self.data.inline[0]
    }

    /// Intepret the [Repr] as a double word and get its value.
    /// 
    /// SAFETY: You need to check the capacity field before accessing the union.
    #[inline]
    unsafe fn as_dword(&self) -> DoubleWord {
        debug_assert!(self.capacity() == 2);
        double_word(self.data.inline[0], self.data.inline[1])
    }

    /// Get the capacity of Repr and sign simultaneously
    #[inline]
    pub fn signed_capacity(&self) -> (usize, Sign) {
        if self.capacity.get() > 0 {
            (self.capacity.get() as usize, Sign::Positive)
        } else {
            // wrapping will never happen because MAX_CAPACITY < isize::MAX
            (self.capacity.get().wrapping_neg() as usize, Sign::Negative)
        }
    }

    /// Set the sign flag of the representation
    pub fn set_sign(&mut self, sign: Sign) {
        match (self.capacity.get().signum(), sign) {
            (1, Sign::Positive) | (-1, Sign::Negative) => {},
            (1, Sign::Negative) | (-1, Sign::Positive) => {
                self.capacity = unsafe {
                    // SAFETY: capacity is not allowed to be zero
                    NonZeroIsize::new_unchecked(self.capacity.get().wrapping_neg())
                }
            },
            _ => unreachable!()
        }
    }

    /// Cast the reference of `Repr` to a strong typed representation, assuming the underlying data is unsigned.
    /// 
    /// # Panics
    /// 
    /// Panics if the `capacity` is negative
    #[inline]
    pub fn variants(&self) -> StrongReprRef {
        assert!(self.capacity.get() > 0);

        match self.capacity.get() {
            1 => StrongReprRef::RefSingle(self.data.inline[0]),
            2 => StrongReprRef::RefDouble(double_word(self.data.inline[0], self.data.inline[1])),
            _ => unsafe {
                StrongReprRef::RefLarge(slice::from_raw_parts(
                    self.data.heap.0,
                    self.data.heap.1
                ))
            }
        }
    }

    /// Cast the `Repr` to a strong typed representation, assuming the underlying data is unsigned.
    /// 
    /// # Panics
    /// 
    /// Panics if the `capacity` is negative
    #[inline]
    pub fn as_variants(self) -> StrongRepr {
        assert!(self.capacity.get() > 0);

        match self.capacity.get() {
            1 => StrongRepr::Single(self.data.inline[0]),
            2 => StrongRepr::Double(double_word(self.data.inline[0], self.data.inline[1])),
            _ => unsafe {
                // SAFETY: An `Buffer` and `Repr` have the same layout
                //     and we have made sure that the data is allocated on heap
                StrongRepr::Large(mem::transmute(self))
            }
        }
    }

    /// Cast the `Repr` to a strong typed representation and return the sign.
    pub fn as_signed_variants(mut self) -> (StrongRepr, Sign) {
        let (abs_capacity, sign) = self.signed_capacity();
        self.capacity = unsafe {
            // SAFETY: capacity is not allowed to be zero
            NonZeroIsize::new_unchecked(abs_capacity as isize)
        };
        (self.as_variants(), sign)
    }

    /// Get a reference to the words in the `Repr`, together with the sign.
    pub fn as_signed_slice(&self) -> (&[Word], Sign) {
        let (capacity, sign) = self.signed_capacity();
        let arr = unsafe {
                match capacity {
                0 => unreachable!(),
                1 => &self.data.inline[..1],
                2 => &self.data.inline,
                _ => slice::from_raw_parts(
                    self.data.heap.0,
                    self.data.heap.1
                )
            }
        };
        (arr, sign)
    }

    /// Creates a `Repr` with a single word
    #[inline]
    pub(crate) fn from_word(n: Word) -> Self {
        Repr { data: ReprData { inline: [n, 0] }, capacity: NonZeroIsize::new(1).unwrap() }
    }

    /// Creates a `Repr` with a double word represented in [lo, hi].
    #[inline]
    pub(crate) fn from_dword(n: DoubleWord) -> Self {
        let (lo, hi) = split_double_word(n);
        if hi == 0 {
            Self::from_word(lo)
        } else {
            Repr { data: ReprData { inline: [lo, hi] }, capacity: NonZeroIsize::new(2).unwrap() }
        }
    }

    /// Creates a `Repr` with a buffer allocated on heap.
    /// 
    /// Note that it's recommended to call `Buffer::pop_zeros()` before it's
    /// converted to the `Repr`.
    pub(crate) fn from_buffer(buffer: Buffer) -> Self {
        match buffer.len() {
            0 => Self::from_word(0),
            1 => Self::from_word(buffer[0]),
            2 => Self::from_dword(double_word(buffer[0], buffer[1])),
            _ => unsafe {
                // TODO: check whether this will call drop
                // SAFETY: the length has been checked and capacity >= lenght,
                //         so capacity is nonzero and larger than 2
                mem::transmute(buffer)
            }
        }
    }

    /// Creates a `Repr` with a buffer allocated on heap and the sign of the number
    /// 
    /// Note that it's recommended to call `Buffer::pop_zeros()` before it's
    /// converted to the `Repr`.
    #[inline]
    pub(crate) fn from_signed_buffer(heap: Buffer, sign: Sign) -> Self {
        let mut result = Self::from_buffer(heap);
        result.set_sign(sign);
        result
    }
}


impl Clone for Repr {
    fn clone(&self) -> Self {
        let (capacity, sign) = self.signed_capacity();

        let mut new = unsafe {
            // inline the data if the length is less than 3
            // SAFETY: we check the capacity before accessing the variants
            match capacity {
                c if c <= 2 => {
                    Repr { data: ReprData { inline: self.data.inline }, capacity: NonZeroIsize::new_unchecked(c as isize) }
                },
                _ => {
                    let (ptr, len) = self.data.heap;
                    let mut new_buffer = Buffer::allocate(len);
                    new_buffer.push_slice(slice::from_raw_parts(ptr, len));

                    // SAFETY: abs(self.capacity) >= 3 => self.data.len >= 3
                    // so the capacity and len of new_buffer will be both >= 3
                    mem::transmute(new_buffer)
                }
            }
        };
        new.set_sign(sign);
        new
    }

    #[inline]
    fn clone_from(&mut self, src: &Self) {
        let (src_cap, src_sign) = src.signed_capacity();
        let (cap, _) = self.signed_capacity();

        // shortcut for inlined data
        if src_cap <= 2 {
            *self = { Repr { data: ReprData { inline: self.data.inline }, capacity: unsafe {
                // SAFETY: the capacity from src is now allowed to be zero
                NonZeroIsize::new_unchecked(src_cap as isize)
            } } };
            self.set_sign(src_sign);
            return;
        }

        let (src_ptr, src_len) = src.data.heap;
        let ptr = self.data.heap.0;
        unsafe {
            // check if we need reallocation, the strategy here is the same as `Buffer::clone_from()`
            if cap < src_len || cap > Buffer::max_compact_capacity(src_len) {
                // release the old buffer
                // SAFETY: the old buffer is allocated through alloc::alloc::alloc
                let layout = Layout::array::<Word>(cap).unwrap();
                alloc::alloc::dealloc(ptr as _, layout);
    
                // allocate a new one
                // SAFETY: all the fields in self are now safely obselete
                let buffer_mut = &mut *(self as *mut Self as *mut Buffer);
                *buffer_mut = Buffer::allocate(src_len);
            }
            
            // SAFETY: src.ptr and self.ptr are both properly allocated by `Buffer::allocate()`.
            //         src.ptr and self.ptr cannot alias, because the ptr should be uniquely owned by the Buffer
            ptr::copy_nonoverlapping(src_ptr, ptr, src_len);
        }

        // update length and sign
        self.data.heap.1 = src_len;
        self.set_sign(src_sign);
    }
}

impl Drop for Repr {
    fn drop(&mut self) {
        unsafe {
            let capacity = self.capacity.get().unsigned_abs();
            if capacity > 2 {
                let layout = Layout::array::<Word>(capacity).unwrap();
                alloc::alloc::dealloc(self.data.heap.0 as _, layout);
            }
        }
    }
}

// impl Deref for Repr {
//     type Target = [Word];

//     #[inline]
//     fn deref(&self) -> &[Word] {
//         unsafe {
//             match self.capacity() {
//                 0 => unreachable!(),
//                 1 => &self.data.inline[..1],
//                 2 => &self.data.inline,
//                 _ => slice::from_raw_parts(
//                     self.data.heap.0,
//                     self.data.heap.1
//                 )
//             }
//         }
//     }
// }

// impl DerefMut for Repr {
//     #[inline]
//     fn deref_mut(&mut self) -> &mut [Word] {
//         unsafe {
//             match self.capacity.get().wrapping_abs() {
//                 0 => unreachable!(),
//                 1 => &mut self.data.inline[..1],
//                 2 => &mut self.data.inline,
//                 _ => slice::from_raw_parts_mut(
//                     self.data.heap.0,
//                     self.data.heap.1
//                 )
//             }
//         }
//     }
// }

impl PartialEq for Repr {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.as_signed_slice() == other.as_signed_slice()
    }
}
impl Eq for Repr {}

impl fmt::Debug for Repr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (arr, sign) = self.as_signed_slice();
        if let Sign::Negative = sign {
            f.write_char('-');
        }
        f.debug_list().entries(arr).finish()
    }
}

impl Hash for Repr {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let (arr, sign) = self.as_signed_slice();
        sign.hash(state);
        (*arr).hash(state);
    }
}

/*

/// Buffer for Words (Old).
///
/// UBig operations are usually performed by creating a BufferOld with appropriate capacity, filling it
/// in with Words, and then converting to UBig.
///
/// If its capacity is exceeded, the `BufferOld` will panic.
#[derive(Debug, Eq, Hash, PartialEq)]
pub(crate) struct BufferOld(Vec<Word>);

impl BufferOld {
    /// Creates a `BufferOld` with at least specified capacity.
    ///
    /// It leaves some extra space for future growth.
    pub(crate) fn allocate(num_words: usize) -> BufferOld {
        if num_words > BufferOld::MAX_CAPACITY {
            // UBig::panic_number_too_large();
            panic!()
        }
        BufferOld(Vec::with_capacity(BufferOld::default_capacity(num_words)))
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
        if self.capacity() > BufferOld::max_compact_capacity(self.len()) {
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
        let mut new_buffer = BufferOld::allocate(num_words);
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
    pub(crate) fn resizing_clone_from(&mut self, source: &BufferOld) {
        let cap = self.capacity();
        let n = source.len();
        if cap >= n && cap <= BufferOld::max_compact_capacity(n) {
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
        debug_assert!(num_words <= BufferOld::MAX_CAPACITY);
        (num_words + num_words / 8 + 2).min(BufferOld::MAX_CAPACITY)
    }

    /// Maximum compact capacity for a given number of `Word`s.
    ///
    /// Requires that `num_words <= BufferOld::MAX_CAPACITY`.
    ///
    /// Allows `4 + 0.25 * num_words` overhead.
    #[inline]
    fn max_compact_capacity(num_words: usize) -> usize {
        debug_assert!(num_words <= BufferOld::MAX_CAPACITY);
        (num_words + num_words / 4 + 4).min(BufferOld::MAX_CAPACITY)
    }
}

impl Clone for BufferOld {
    /// New buffer will be sized as `BufferOld::allocate(self.len())`.
    fn clone(&self) -> BufferOld {
        let mut new_buffer = BufferOld::allocate(self.len());
        new_buffer.clone_from(self);
        new_buffer
    }

    /// If capacity is exceeded, panic.
    #[inline]
    fn clone_from(&mut self, source: &BufferOld) {
        assert!(self.capacity() >= source.len());
        self.0.clone_from(&source.0);
    }
}

impl Deref for BufferOld {
    type Target = [Word];

    #[inline]
    fn deref(&self) -> &[Word] {
        &self.0
    }
}

impl DerefMut for BufferOld {
    #[inline]
    fn deref_mut(&mut self) -> &mut [Word] {
        &mut self.0
    }
}

impl Extend<Word> for BufferOld {
    fn extend<T>(&mut self, iter: T)
    where
        T: IntoIterator<Item = Word>,
    {
        for word in iter {
            self.push(word);
        }
    }
}

impl<'a> Extend<&'a Word> for BufferOld {
    fn extend<T>(&mut self, iter: T)
    where
        T: IntoIterator<Item = &'a Word>,
    {
        for word in iter {
            self.push(*word);
        }
    }
}

*/

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
        let mut buffer = Buffer::allocate(3);
        buffer.push(7);
        assert!(buffer.capacity() >= 3);
        buffer.ensure_capacity(4);
        assert!(buffer.capacity() >= 4);
        buffer.ensure_capacity(5);
        assert!(buffer.capacity() >= 5);
        assert_eq!(&buffer[..], [7]);
    }

    #[test]
    fn test_shrink() {
        let mut buffer = Buffer::allocate(100);
        buffer.push(7);
        buffer.push(8);
        buffer.push(9);
        buffer.shrink();
        assert_eq!(buffer.capacity(), Buffer::default_capacity(3));
        assert_eq!(&buffer[..], [7, 8, 9]);
    }

    #[test]
    fn test_push_pop() {
        let mut buffer = Buffer::allocate(5);
        buffer.push(1);
        buffer.push(2);
        assert_eq!(&buffer[..], [1, 2]);
        // assert_eq!(buffer.pop(), Some(2));
        // assert_eq!(buffer.pop(), Some(1));
        // assert_eq!(buffer.pop(), None);
    }

    #[test]
    fn test_pop_leading_zeros() {
        let mut buffer = Buffer::allocate(5);
        buffer.push(1);
        buffer.push(2);
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
        let mut buffer = Buffer::allocate(3);
        for _ in 0..10 {
            buffer.push(7);
        }
    }

    #[test]
    fn test_push_may_reallocate() {
        let mut buffer = Buffer::allocate(3);
        for _ in 0..10 {
            buffer.push_may_reallocate(7);
        }
        assert_eq!(buffer.len(), 10);
    }

    #[test]
    fn test_clone() {
        // TODO: test clone inline

        let mut buffer = Buffer::allocate(100);
        buffer.push(7);
        buffer.push(8);
        buffer.push(9);
        let buffer2 = buffer.clone();
        assert_eq!(buffer, buffer2);
        assert_eq!(buffer2.capacity(), Buffer::default_capacity(3));
    }

    #[test]
    fn test_clone_from() {
        // TODO: test clone inline

        let mut buffer = Buffer::allocate(100);
        buffer.push(7);
        buffer.push(8);
        buffer.push(9);
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
