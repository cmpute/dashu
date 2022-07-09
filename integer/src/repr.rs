//! Word buffer.

use crate::{
    arch::word::{DoubleWord, Word},
    primitive::{double_word, split_dword, WORD_BITS_USIZE},
    sign::Sign,
};
use alloc::alloc::Layout;
use core::{
    fmt::{self, Write},
    hash::{Hash, Hasher},
    mem,
    num::NonZeroIsize,
    ops::{Deref, DerefMut},
    ptr::{self, NonNull},
    slice,
};
use static_assertions::const_assert_eq;

// TODO: impl Send and Sync for ReprData

/// This union contains the raw representation of words, the words are either inlined
/// or on the heap. The flag used to distinguishing them is the `len` field of the buffer.
#[repr(C)]
union ReprData {
    inline: [Word; 2],        // lo, hi
    heap: (*mut Word, usize), // ptr, len
}

/// Internal representation for big integers.
///
/// It's optimized so that small integers (single or double words) will not be allocated on heap.
/// When the data is allocated on the heap, it can be casted to [Buffer] efficiently, but modifying
/// the buffer inplace is not allowed because that can break the rule on the `capacity` field.
#[repr(C)]
pub(crate) struct Repr {
    /// The capacity is designed to be not zero so that it provides a niche value for other use.
    ///
    /// How to intepret the `data` field:
    /// - `capacity` = 1: the words are inlined and the high word is 0. (including the case where low word is also 0)
    /// - `capacity` = 2: the words are inlined
    /// - `capacity` >= 3: the words are on allocated on the heap. In this case, data.len >= 3 will also be forced.
    /// - `capacity` < 0: similiar to the cases above, but negative capacity value is used to mark the integer is negative.
    capacity: NonZeroIsize,

    /// The words in the `data` field are ordered from LSB to MSB.
    data: ReprData,
}

/// Buffer of words allocated on heap. It's like a `Vec<Word>` with limited functionalities.
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
    len: usize,
}
const_assert_eq!(mem::size_of::<Buffer>(), mem::size_of::<Repr>());

/// A strong typed safe representation of a `Repr` without sign
#[derive(Clone)]
pub(crate) enum TypedRepr {
    Small(DoubleWord),
    Large(Buffer),
}

/// A strong typed safe representation of a reference to `Repr` without sign
#[derive(Clone, Copy)]
pub(crate) enum TypedReprRef<'a> {
    RefSmall(DoubleWord),
    RefLarge(&'a [Word]),
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
    pub fn capacity(&self) -> usize {
        self.capacity
    }

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
            let ptr = NonNull::new(ptr).unwrap().cast();
            ptr
        }
    }

    /// Deallocates the words on heap. The caller must make sure the ptr is valid.
    ///
    /// This function should NOT BE EXPOSED to public!
    #[inline]
    pub(crate) unsafe fn deallocate_raw(ptr: NonNull<Word>, capacity: usize) {
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
            panic!(
                "too many words to be allocated, maximum is {} words",
                Self::MAX_CAPACITY
            );
        }

        let ptr = Self::allocate_raw(capacity);
        Buffer { capacity, ptr, len: 0 }
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
        debug_assert!(num_words >= self.len());
        self.reallocate_raw(Self::default_capacity(num_words));
    }

    /// Ensure there is enough capacity in the buffer for `num_words`,
    /// reallocate if necessary.
    #[inline]
    pub(crate) fn ensure_capacity(&mut self, num_words: usize) {
        if num_words > self.capacity && num_words > 2 {
            self.reallocate(num_words);
        }
    }

    /// Ensure there is enough capacity that is not less than the given value,
    /// reallocate if necessary.
    #[inline]
    pub(crate) fn ensure_capacity_exact(&mut self, capacity: usize) {
        if capacity > self.capacity && capacity > 2 {
            self.reallocate_raw(capacity);
        }
    }

    /// Makes sure that the capacity is compact for existing data.
    #[inline]
    pub(crate) fn shrink_to_fit(&mut self) {
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
    pub(crate) fn push_resizing(&mut self, word: Word) {
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
        self.len = new_len;
    }

    /// Get the first double word of the buffer, assuming the buffer has at least two words.
    ///
    /// # Panics
    ///
    /// Panics if the buffer is empty or has only 1 word
    #[inline]
    pub(crate) fn first_dword(&self) -> DoubleWord {
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
    pub(crate) fn first_dword_mut(&mut self) -> (&mut Word, &mut Word) {
        assert!(self.len >= 2);

        unsafe {
            let ptr = self.ptr.as_ptr();
            (&mut *ptr, &mut *ptr.add(1))
        }
    }

    /// Make the data in `Repr` a copy of another slice.
    ///
    /// It reallocates if capacity is too small or too large.
    pub(crate) fn clone_from_slice(&mut self, src: &[Word]) {
        if self.capacity >= src.len() && self.capacity <= Buffer::max_compact_capacity(src.len()) {
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

impl Repr {
    /// Get the length of the number (in `Word`s)
    #[inline]
    pub const fn len(&self) -> usize {
        match self.capacity() {
            // 0 => unreachable!(),
            1 => 1,
            2 => 2,
            _ => unsafe { self.data.heap.1 },
        }
    }

    /// Get the capacity of the representation (in `Word`s)
    ///
    /// It will not be zero even if the underlying number is zero.
    #[inline]
    pub const fn capacity(&self) -> usize {
        self.capacity.get().unsigned_abs()
    }

    /// Get the sign of the repr
    #[inline]
    pub fn sign(&self) -> Sign {
        if self.capacity.get() > 0 {
            Sign::Positive
        } else {
            Sign::Negative
        }
    }

    /// Get the capacity of Repr and sign simultaneously
    #[inline]
    pub fn sign_capacity(&self) -> (usize, Sign) {
        if self.capacity.get() > 0 {
            (self.capacity.get() as usize, Sign::Positive)
        } else {
            // wrapping will never happen because MAX_CAPACITY < isize::MAX
            (self.capacity.get().wrapping_neg() as usize, Sign::Negative)
        }
    }

    /// Set the sign flag and return the changed representation. The sign will not
    /// be flipped if self is zero
    #[inline]
    pub fn with_sign(mut self, sign: Sign) -> Self {
        if !self.is_zero() && ((sign == Sign::Positive) ^ (self.capacity.get() > 0)) {
            self.capacity = unsafe {
                // SAFETY: capacity is not allowed to be zero
                NonZeroIsize::new_unchecked(-self.capacity.get())
            }
        }
        self
    }

    /// Cast the reference of `Repr` to a strong typed representation, assuming the underlying data is unsigned.
    ///
    /// # Panics
    ///
    /// Panics if the `capacity` is negative
    #[inline]
    pub fn as_typed(&self) -> TypedReprRef<'_> {
        assert!(self.capacity.get() > 0);

        unsafe {
            match self.capacity.get() {
                1 | 2 => {
                    TypedReprRef::RefSmall(double_word(self.data.inline[0], self.data.inline[1]))
                }
                _ => TypedReprRef::RefLarge(slice::from_raw_parts(
                    self.data.heap.0,
                    self.data.heap.1,
                )),
            }
        }
    }

    /// Cast the reference of `Repr` to a strong typed representation, and return with the sign.
    ///
    /// # Panics
    ///
    /// Panics if the `capacity` is negative
    #[inline]
    pub fn as_sign_typed(&self) -> (Sign, TypedReprRef<'_>) {
        let (abs_capacity, sign) = self.sign_capacity();

        let typed = unsafe {
            match abs_capacity {
                1 | 2 => {
                    TypedReprRef::RefSmall(double_word(self.data.inline[0], self.data.inline[1]))
                }
                _ => TypedReprRef::RefLarge(slice::from_raw_parts(
                    self.data.heap.0,
                    self.data.heap.1,
                )),
            }
        };
        (sign, typed)
    }

    /// Cast the `Repr` to a strong typed representation, assuming the underlying data is unsigned.
    ///
    /// # Panics
    ///
    /// Panics if the `capacity` is negative
    #[inline]
    pub fn into_typed(self) -> TypedRepr {
        assert!(self.capacity.get() > 0);

        unsafe {
            match self.capacity.get() {
                1 | 2 => TypedRepr::Small(double_word(self.data.inline[0], self.data.inline[1])),
                _ => {
                    // SAFETY: An `Buffer` and `Repr` have the same layout
                    //     and we have made sure that the data is allocated on heap
                    TypedRepr::Large(mem::transmute(self))
                }
            }
        }
    }

    /// Cast the `Repr` to a strong typed representation and return with the sign.
    pub fn into_sign_typed(mut self) -> (Sign, TypedRepr) {
        let (abs_capacity, sign) = self.sign_capacity();
        self.capacity = unsafe {
            // SAFETY: capacity is not allowed to be zero
            NonZeroIsize::new_unchecked(abs_capacity as isize)
        };
        (sign, self.into_typed())
    }

    /// Get a reference to the words in the `Repr`, together with the sign.
    pub fn as_sign_slice(&self) -> (Sign, &[Word]) {
        let (capacity, sign) = self.sign_capacity();
        let words = unsafe {
            match capacity {
                0 => unreachable!(),
                1 => &self.data.inline[..1],
                2 => &self.data.inline,
                _ => slice::from_raw_parts(self.data.heap.0, self.data.heap.1),
            }
        };
        (sign, words)
    }

    /// Creates a `Repr` with a single word
    #[inline]
    pub(crate) fn from_word(n: Word) -> Self {
        Repr {
            data: ReprData { inline: [n, 0] },
            capacity: NonZeroIsize::new(1).unwrap(),
        }
    }

    /// Creates a `Repr` with a double word
    #[inline]
    pub(crate) fn from_dword(n: DoubleWord) -> Self {
        let (lo, hi) = split_dword(n);
        Repr {
            data: ReprData { inline: [lo, hi] },
            capacity: NonZeroIsize::new(1 + (hi != 0) as isize).unwrap(),
        }
    }

    /// Creates a `Repr` with a buffer allocated on heap. The leading zeros in the buffer
    /// will be trimmed and the buffer will be shrunk if there is exceeded capacity.
    pub(crate) fn from_buffer(mut buffer: Buffer) -> Self {
        buffer.pop_zeros();

        match buffer.len() {
            0 => Self::from_word(0),
            1 => Self::from_word(buffer[0]),
            2 => Self::from_dword(double_word(buffer[0], buffer[1])),
            _ => {
                // If the Buffer was allocated with `Buffer::allocate(n)`
                // and the normalized length is between `n - 2` and `n + 2`
                // (or even approximately between `0.9 * n` and `1.125 * n`),
                // there will be no reallocation here.
                buffer.shrink_to_fit();

                // SAFETY: the length has been checked and capacity >= lenght,
                //         so capacity is nonzero and larger than 2
                unsafe { mem::transmute(buffer) }
            }
        }
    }

    /// Creates a `Repr` with value 0
    #[inline]
    pub(crate) const fn zero() -> Self {
        Repr {
            capacity: unsafe { NonZeroIsize::new_unchecked(1) },
            data: ReprData { inline: [0, 0] },
        }
    }

    /// Check if the underlying value is zero
    #[inline]
    pub(crate) const fn is_zero(&self) -> bool {
        self.capacity() == 1 && unsafe { self.data.inline[0] == 0 }
    }

    /// Creates a `Repr` with value 1
    #[inline]
    pub(crate) const fn one() -> Self {
        Repr {
            capacity: unsafe { NonZeroIsize::new_unchecked(1) },
            data: ReprData { inline: [1, 0] },
        }
    }

    /// Check if the underlying value is zero
    #[inline]
    pub(crate) const fn is_one(&self) -> bool {
        self.capacity.get() == 1 && unsafe { self.data.inline[0] == 1 }
    }

    /// Creates a `Repr` with value -1
    #[inline]
    pub(crate) const fn neg_one() -> Self {
        Repr {
            capacity: unsafe { NonZeroIsize::new_unchecked(-1) },
            data: ReprData { inline: [1, 0] },
        }
    }

    /// Flip the sign bit of the Repr and return it
    pub fn neg(mut self) -> Self {
        if !self.is_zero() {
            self.capacity = unsafe { NonZeroIsize::new_unchecked(-self.capacity.get()) }
        }
        self
    }
}

// Cloning for Repr is written in a verbose way because it's performance critical.
impl Clone for Repr {
    fn clone(&self) -> Self {
        let (capacity, sign) = self.sign_capacity();

        let new = unsafe {
            // inline the data if the length is less than 3
            // SAFETY: we check the capacity before accessing the variants
            if capacity <= 2 {
                Repr { 
                    data: ReprData { inline: self.data.inline },
                    capacity: NonZeroIsize::new_unchecked(capacity as isize),
                }
            } else {
                let (ptr, len) = self.data.heap;
                let mut new_buffer = Buffer::allocate(len);
                new_buffer.push_slice(slice::from_raw_parts(ptr, len));

                // SAFETY: abs(self.capacity) >= 3 => self.data.len >= 3
                // so the capacity and len of new_buffer will be both >= 3
                mem::transmute(new_buffer)
            }
        };
        new.with_sign(sign)
    }

    fn clone_from(&mut self, src: &Self) {
        let (src_cap, src_sign) = src.sign_capacity();
        let (cap, _) = self.sign_capacity();

        unsafe {
            // shortcut for inlined data
            if src_cap <= 2 {
                if cap > 2 {
                    // release the old buffer if necessary
                    Buffer::deallocate_raw(NonNull::new_unchecked(self.data.heap.0), cap);
                }
                self.data.inline = src.data.inline;
                self.capacity = src.capacity;
                return;
            }

            // SAFETY: we checked that abs(src.capacity) > 2
            let (src_ptr, src_len) = src.data.heap;
            debug_assert!(src_len >= 3);

            // check if we need reallocation, the strategy here is the same as `Buffer::clone_from()`
            if cap < src_len || cap > Buffer::max_compact_capacity(src_len) {
                if cap > 2 {
                    // release the old buffer if necessary
                    Buffer::deallocate_raw(NonNull::new_unchecked(self.data.heap.0), cap);
                }

                let new_cap = Buffer::default_capacity(src_len);
                let new_ptr = Buffer::allocate_raw(new_cap);
                self.data.heap.0 = new_ptr.as_ptr();
                // SAFETY: allocate_raw will allocates at least 2 words even if src_len is 0
                self.capacity = NonZeroIsize::new_unchecked(new_cap as isize);
            }

            // SAFETY: src.ptr and self.ptr are both properly allocated by `Buffer::allocate()`.
            //         src.ptr and self.ptr cannot alias, because the ptr should be uniquely owned by the Buffer
            ptr::copy_nonoverlapping(src_ptr, self.data.heap.0, src_len);

            // update length and sign
            self.data.heap.1 = src_len;
            if (src_sign == Sign::Positive) ^ (self.capacity.get() > 0) {
                self.capacity = NonZeroIsize::new_unchecked(-self.capacity.get());
            }
        }
    }
}

impl Drop for Repr {
    fn drop(&mut self) {
        let cap = self.capacity();
        if cap > 2 {
            unsafe {
                Buffer::deallocate_raw(NonNull::new_unchecked(self.data.heap.0), cap);
            }
        }
    }
}

impl PartialEq for Repr {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.as_sign_slice() == other.as_sign_slice()
    }
}
impl Eq for Repr {}

impl fmt::Debug for Repr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (sign, words) = self.as_sign_slice();
        if let Sign::Negative = sign {
            f.write_char('-')?;
        }
        f.debug_list().entries(words).finish()
    }
}

impl Hash for Repr {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let (sign, arr) = self.as_sign_slice();
        sign.hash(state);
        (*arr).hash(state);
    }
}

impl TypedRepr {
    /// Convert a reference of `TypedRef` to `TypedReprRef`
    #[inline]
    pub(crate) fn as_ref(&self) -> TypedReprRef {
        match self {
            Self::Small(dword) => TypedReprRef::RefSmall(*dword),
            Self::Large(buffer) => TypedReprRef::RefLarge(&buffer),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repr_inline() {
        let repr = Repr::zero();
        assert_eq!(repr.capacity(), 1);
        assert_eq!(repr.len(), 1);

        let repr = Repr::from_word(123);
        assert_eq!(repr.capacity(), 1);
        assert_eq!(repr.len(), 1);

        let repr = Repr::from_dword(123 << WORD_BITS_USIZE);
        assert_eq!(repr.capacity(), 2);
        assert_eq!(repr.len(), 2);
    }

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
    fn test_clone() {
        // test Repr
        let repr = Repr::from_word(123);
        let repr2 = repr.clone();
        assert_eq!(repr2.capacity(), 1);
        assert_eq!(repr2.len(), 1);
        assert_eq!(repr, repr2);

        let repr = Repr::from_dword(123 << WORD_BITS_USIZE);
        let repr2 = repr.clone();
        assert_eq!(repr2.capacity(), repr.capacity());
        assert_eq!(repr2.len(), repr.len());
        assert_eq!(repr, repr2);

        // test Buffer
        let mut buffer = Buffer::allocate(100);
        buffer.push(7);
        buffer.push(8);
        buffer.push(9);
        let buffer2 = buffer.clone();
        assert_eq!(buffer, buffer2);
        assert_eq!(buffer2.capacity(), Buffer::default_capacity(3));

        let repr = Repr::from_buffer(buffer);
        let repr2 = repr.clone();
        assert_eq!(repr.capacity(), Buffer::default_capacity(3));
        assert_eq!(repr, repr2);
    }

    #[test]
    fn test_clone_from() {
        // test Repr
        let repr = Repr::from_word(123);
        let mut repr2 = Repr::zero();
        repr2.clone_from(&repr);
        assert_eq!(repr2.capacity(), repr.capacity());
        assert_eq!(repr2.len(), repr.len());
        assert_eq!(repr, repr2);

        let repr = Repr::from_dword(123 << WORD_BITS_USIZE);
        let mut repr2 = Repr::zero();
        repr2.clone_from(&repr);
        assert_eq!(repr2.capacity(), repr.capacity());
        assert_eq!(repr2.len(), repr.len());
        assert_eq!(repr, repr2);

        // test Buffer
        let mut buffer = Buffer::allocate(100);
        buffer.push(7);
        buffer.push(8);
        buffer.push(9);
        let mut buffer2 = Buffer::allocate(50);
        buffer2.clone_from(&buffer);
        assert_eq!(buffer, buffer2);
        assert_ne!(buffer.capacity(), buffer2.capacity());

        let repr = Repr::from_buffer(buffer);
        let mut repr2 = Repr::from_buffer(buffer2);
        repr2.clone_from(&repr);
        assert_eq!(repr, repr2);
    }

    #[test]
    fn test_resizing_clone_from() {
        // test Buffer
        let mut buf = Buffer::allocate(5);
        assert_eq!(buf.capacity(), 7);

        let mut buf2 = Buffer::allocate(4);
        assert_eq!(buf2.capacity(), 6);
        for i in 0..4 {
            buf2.push(i);
        }
        buf.clone_from(&buf2);
        assert_eq!(buf.capacity(), 7);
        assert_eq!(&buf[..], [0, 1, 2, 3]);

        let mut buf3 = Buffer::allocate(100);
        for i in 0..100 {
            buf3.push(i);
        }
        buf.clone_from(&buf3);
        assert_eq!(buf.capacity(), Buffer::default_capacity(100));
        assert_eq!(buf.len(), 100);

        buf.clone_from(&buf2);
        assert_eq!(buf.capacity(), 6);
        assert_eq!(&buf[..], [0, 1, 2, 3]);

        // test Repr
        let mut repr = Repr::zero(); // start from inline
        let repr2 = Repr::from_buffer(buf2);
        repr.clone_from(&repr2);
        assert_eq!(repr.len(), 4);
        assert_eq!(repr, repr2);
        assert!(matches!(repr.as_typed(), TypedReprRef::RefLarge(_)));

        let repr3 = Repr::from_buffer(buf3);
        repr.clone_from(&repr3);
        assert_eq!(repr.len(), 100);
        assert_eq!(repr, repr3);
        assert!(matches!(repr.as_typed(), TypedReprRef::RefLarge(_)));

        repr.clone_from(&repr2);
        assert_eq!(repr.len(), 4);
        assert_eq!(repr, repr2);
        assert!(matches!(repr.as_typed(), TypedReprRef::RefLarge(_)));

        let repr_inline = Repr::from_word(123);
        repr.clone_from(&repr_inline);
        assert_eq!(repr.len(), 1);
        assert_eq!(repr, repr_inline);
        assert!(matches!(repr.as_typed(), TypedReprRef::RefSmall(_)));
    }

    #[test]
    fn test_into_boxed_slice () {
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
