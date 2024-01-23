//! Underlying representation of big integers.

use crate::{
    arch::word::{DoubleWord, Word},
    buffer::Buffer,
    primitive::{double_word, split_dword, WORD_BITS_USIZE, DWORD_BITS_USIZE},
    Sign, math::{ones_word, ones_dword},
};
use core::{
    fmt::{self, Write},
    hash::{Hash, Hasher},
    hint::unreachable_unchecked,
    mem,
    num::NonZeroIsize,
    ptr::{self, NonNull},
    slice,
};
use static_assertions::const_assert_eq;

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
///
/// To modified the internal data, one must convert the Repr into either [TypedRepr](enum, owning the data)
/// or [Buffer](raw heap buffer). To access the internal data, one must use [TypedReprRef](enum, reference)
/// or [slice][Repr::as_slice] protocol.
#[repr(C)]
pub struct Repr {
    /// The words in the `data` field are ordered from the least significant to the most significant.
    data: ReprData,

    /// The capacity is guaranteed to be not zero so that it provides a niche value for layout optimization.
    ///
    /// How to intepret the `data` field:
    /// - `capacity` = 1: the words are inlined and the high word is 0. (including the case where low word is also 0)
    /// - `capacity` = 2: the words are inlined
    /// - `capacity` >= 3: the words are on allocated on the heap. In this case, data.len >= 3 will also be forced.
    /// - `capacity` < 0: similiar to the cases above, but negative capacity value is used to mark the integer is negative.
    ///     Note that in this case the inlined value is not allowed to be zero. (zero must have a positive sign)
    capacity: NonZeroIsize,
}

// right now on all supported architectures, Word = usize. However, for cases where
// Word > usize, an extra padding in Buffer will be necessary for this equality to hold
const_assert_eq!(mem::size_of::<Buffer>(), mem::size_of::<Repr>());

// make sure the layout optimization is effective
const_assert_eq!(mem::size_of::<Repr>(), mem::size_of::<Option<Repr>>());

// SAFETY: the pointer to the allocated space is uniquely owned by this struct.
unsafe impl Send for Repr {}

// SAFETY: we don't provide interior mutability for Repr and Buffer
unsafe impl Sync for Repr {}

/// A strong typed safe representation of a `Repr` without sign
#[derive(Clone)]
pub enum TypedRepr {
    Small(DoubleWord),
    Large(Buffer),
}

/// A strong typed safe representation of a reference to `Repr` without sign
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TypedReprRef<'a> {
    RefSmall(DoubleWord),
    RefLarge(&'a [Word]),
}

impl Repr {
    /// Get the length of the number (in `Word`s), return 0 when the number is zero.
    #[inline]
    pub const fn len(&self) -> usize {
        // SAFETY: the capacity is checked before accessing the fields.
        //         see the documentation for the `capacity` fields for invariants.
        unsafe {
            match self.capacity() {
                0 => unreachable_unchecked(),
                1 => (self.data.inline[0] != 0) as usize,
                2 => 2,
                _ => self.data.heap.1,
            }
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
    pub const fn sign(&self) -> Sign {
        if self.capacity.get() > 0 {
            Sign::Positive
        } else {
            Sign::Negative
        }
    }

    /// Get the capacity of Repr and sign simultaneously
    #[inline]
    pub const fn sign_capacity(&self) -> (usize, Sign) {
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
    pub const fn with_sign(mut self, sign: Sign) -> Self {
        let is_positive = match sign {
            Sign::Positive => true,
            Sign::Negative => false,
        };
        if !self.is_zero() && (is_positive ^ (self.capacity.get() > 0)) {
            // SAFETY: capacity is not allowed to be zero
            self.capacity = unsafe { NonZeroIsize::new_unchecked(-self.capacity.get()) }
        }
        self
    }

    /// Cast the reference of `Repr` to a strong typed representation, assuming the underlying data is unsigned.
    /// Panics if the `capacity` is negative
    #[rustversion::attr(since(1.64), const)]
    #[inline]
    pub fn as_typed(&self) -> TypedReprRef<'_> {
        let (sign, typed) = self.as_sign_typed();
        match sign {
            // sign check
            Sign::Positive => {}
            Sign::Negative => unreachable!(),
        }

        typed
    }

    /// Cast the reference of `Repr` to a strong typed representation, and return with the sign.
    #[rustversion::attr(since(1.64), const)]
    #[inline]
    pub fn as_sign_typed(&self) -> (Sign, TypedReprRef<'_>) {
        let (abs_capacity, sign) = self.sign_capacity();

        // SAFETY: the capacity is checked before accessing the fields.
        //         see the documentation for the `capacity` fields for invariants.
        let typed = unsafe {
            match abs_capacity {
                0 => unreachable_unchecked(),
                1 | 2 => {
                    TypedReprRef::RefSmall(double_word(self.data.inline[0], self.data.inline[1]))
                }
                _ => TypedReprRef::RefLarge(slice::from_raw_parts(
                    // need Rust 1.64 for const
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
        debug_assert!(self.capacity.get() > 0);

        // SAFETY: the capacity is checked before accessing the fields.
        //         see the documentation for the `capacity` fields for invariants.
        unsafe {
            match self.capacity.get() {
                0 => unreachable_unchecked(),
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
    #[inline]
    pub fn into_sign_typed(mut self) -> (Sign, TypedRepr) {
        let (abs_capacity, sign) = self.sign_capacity();
        // SAFETY: capacity != 0 is an invariant
        self.capacity = unsafe { NonZeroIsize::new_unchecked(abs_capacity as isize) };
        (sign, self.into_typed())
    }

    /// Get a reference to the words in the `Repr`
    ///
    /// # Panics
    ///
    /// Panics if the `capacity` is negative
    #[inline]
    pub fn as_slice(&self) -> &[Word] {
        let (sign, slice) = self.as_sign_slice();
        assert!(sign == Sign::Positive);
        slice
    }

    /// Get a reference to the words in the `Repr`, together with the sign.
    pub fn as_sign_slice(&self) -> (Sign, &[Word]) {
        let (capacity, sign) = self.sign_capacity();

        // SAFETY: the capacity is checked before accessing the fields.
        //         see the documentation for the `capacity` fields for invariants.
        let words = unsafe {
            match capacity {
                0 => unreachable_unchecked(),
                1 => {
                    if self.data.inline[0] == 0 {
                        &[]
                    } else {
                        &self.data.inline[..1]
                    }
                }
                2 => &self.data.inline,
                _ => slice::from_raw_parts(self.data.heap.0, self.data.heap.1),
            }
        };
        (sign, words)
    }

    #[cfg(feature = "zeroize")]
    /// Get all the allocated space as a mutable slice
    pub fn as_full_slice(&mut self) -> &mut [Word] {
        // SAFETY: the capacity is checked before accessing the union fields.
        //         see the documentation for the `capacity` fields for invariants.
        unsafe {
            let capacity = self.capacity();
            if capacity <= 2 {
                &mut self.data.inline
            } else {
                slice::from_raw_parts_mut(self.data.heap.0, capacity)
            }
        }
    }

    /// Creates a `Repr` with a single word
    #[inline]
    pub const fn from_word(n: Word) -> Self {
        Repr {
            data: ReprData { inline: [n, 0] },
            // SAFETY: it's safe. The unsafe constructor is necessary
            //         because it's in a const context.
            capacity: unsafe { NonZeroIsize::new_unchecked(1) },
        }
    }

    /// Creates a `Repr` with a double word
    #[inline]
    pub const fn from_dword(n: DoubleWord) -> Self {
        let (lo, hi) = split_dword(n);
        Repr {
            data: ReprData { inline: [lo, hi] },
            // SAFETY: it's safe. The value is either 1 or 2.
            capacity: unsafe { NonZeroIsize::new_unchecked(1 + (hi != 0) as isize) },
        }
    }

    /// Creates a `Repr` with a reference to static word array.
    ///
    /// This method is unsafe, because the caller must make sure that
    /// the created instance is immutable, and drop() must not be called.
    #[inline]
    pub const unsafe fn from_static_words(words: &'static [Word]) -> Repr {
        match words {
            &[] => Self::zero(),
            &[n] => Self::from_word(n),
            &[lo, hi] => {
                assert!(hi > 0);
                Self::from_dword(double_word(lo, hi))
            }
            large => {
                // this condition is always true, use this expression because unwrap() is not const
                if let Some(n) = large.last() {
                    assert!(*n != 0, "the array input must be normalized.");
                }

                let ptr = large.as_ptr() as _;
                Self {
                    data: ReprData {
                        heap: (ptr, large.len()),
                    },
                    capacity: NonZeroIsize::new_unchecked(large.len() as _),
                }
            }
        }
    }

    /// Create a `Repr` with a buffer allocated on heap. The leading zeros in the buffer
    /// will be trimmed and the buffer will be shrunk if there is exceeded capacity.
    pub fn from_buffer(mut buffer: Buffer) -> Self {
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

                // SAFETY: the length has been checked and capacity >= length,
                //         so capacity is nonzero and larger than 2
                unsafe { mem::transmute(buffer) }
            }
        }
    }

    /// Create a [Repr] cloned from a reference to another [Repr]
    pub fn from_ref(tref: TypedReprRef) -> Self {
        match tref {
            TypedReprRef::RefSmall(dw) => Self::from_dword(dw),
            TypedReprRef::RefLarge(words) => Self::from_buffer(Buffer::from(words)),
        }
    }

    /// Cast the `Repr` to a [Buffer] instance, assuming the underlying data is unsigned.
    ///
    /// # Panics
    ///
    /// Panics if the `capacity` is negative
    pub fn into_buffer(self) -> Buffer {
        debug_assert!(self.capacity.get() > 0); // invariant

        // SAFETY: the capacity is checked before accessing the union fields.
        //         see the documentation for the `capacity` fields for invariants.
        unsafe {
            match self.capacity.get() {
                0 => unreachable_unchecked(),
                1 => {
                    let mut buffer = Buffer::allocate(1);
                    if self.data.inline[0] != 0 {
                        buffer.push(self.data.inline[0]);
                    }
                    buffer
                }
                2 => {
                    debug_assert!(self.data.inline[1] != 0); // invariant
                    let mut buffer = Buffer::allocate(2);
                    buffer.push(self.data.inline[0]);
                    buffer.push(self.data.inline[1]);
                    buffer
                }
                _ => {
                    // SAFETY: An `Buffer` and `Repr` have the same layout
                    //     and we have made sure that the data is allocated on heap
                    mem::transmute(self)
                }
            }
        }
    }

    /// Creates a `Repr` with value 0
    #[inline]
    pub const fn zero() -> Self {
        Self::from_word(0)
    }

    /// Check if the underlying value is zero
    #[inline]
    pub const fn is_zero(&self) -> bool {
        // SAFETY: accessing the union field is safe because the
        //         first condition is checked before access
        self.capacity() == 1 && unsafe { self.data.inline[0] == 0 }
    }

    /// Creates a `Repr` with value 1
    #[inline]
    pub const fn one() -> Self {
        Self::from_word(1)
    }

    /// Check if the underlying value is zero
    #[inline]
    pub const fn is_one(&self) -> bool {
        // SAFETY: accessing the union field is safe because the
        //         first condition is checked before access
        self.capacity.get() == 1 && unsafe { self.data.inline[0] == 1 }
    }

    /// Creates a `Repr` with value -1
    #[inline]
    pub const fn neg_one() -> Self {
        Self::from_word(1).with_sign(Sign::Negative)
    }

    /// Create a `Repr` with n one bits
    pub fn ones(n: usize) -> Self {
        if n < WORD_BITS_USIZE {
            Self::from_word(ones_word(n as _))
        } else if n < DWORD_BITS_USIZE {
            Self::from_dword(ones_dword(n as _))
        } else {
            let lo_words = n / WORD_BITS_USIZE;
            let hi_bits = n % WORD_BITS_USIZE;
            let mut buffer = Buffer::allocate(lo_words + 1);
            buffer.push_repeat::<{ Word::MAX }>(lo_words);
            if hi_bits > 0 {
                buffer.push(ones_word(hi_bits as _));
            }

            // SAFETY: the bit length has been checked and capacity >= length,
            //         so capacity is nonzero and larger than 2
            unsafe { mem::transmute(buffer) }
        }
    }

    /// Flip the sign bit of the Repr and return it
    pub const fn neg(mut self) -> Self {
        if !self.is_zero() {
            // SAFETY: the capacity != 0 is an invariant
            self.capacity = unsafe { NonZeroIsize::new_unchecked(-self.capacity.get()) }
        }
        self
    }

    /// Returns a number representing sign of self.
    ///
    /// * [Self::zero] if the number is zero
    /// * [Self::one] if the number is positive
    /// * [Self::neg_one] if the number is negative
    pub const fn signum(&self) -> Self {
        if self.is_zero() {
            Self::zero()
        } else if self.capacity.get() < 0 {
            Self::neg_one()
        } else {
            Self::one()
        }
    }
}

// Cloning for Repr is written in a verbose way because it's performance critical.
impl Clone for Repr {
    fn clone(&self) -> Self {
        let (capacity, sign) = self.sign_capacity();

        // SAFETY: see the comments inside the block
        let new = unsafe {
            // inline the data if the length is less than 3
            // SAFETY: we check the capacity before accessing the variants
            if capacity <= 2 {
                Repr {
                    data: ReprData {
                        inline: self.data.inline,
                    },
                    // SAFETY: the capacity is from self, which guarantees it to be zero
                    capacity: NonZeroIsize::new_unchecked(capacity as isize),
                }
            } else {
                let (ptr, len) = self.data.heap;
                // SAFETY: len is at least 2 when it's heap allocated (invariant of Repr)
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

        // SAFETY: see the comments inside the block
        unsafe {
            // shortcut for inlined data
            if src_cap <= 2 {
                if cap > 2 {
                    // release the old buffer if necessary
                    // SAFETY: self.data.heap.0 must be valid pointer if cap > 2
                    Buffer::deallocate_raw(NonNull::new_unchecked(self.data.heap.0), cap);
                }
                self.data.inline = src.data.inline;
                self.capacity = src.capacity;
                return;
            }

            // SAFETY: we checked that abs(src.capacity) > 2
            let (src_ptr, src_len) = src.data.heap;
            debug_assert!(src_len >= 3);

            // check if we need reallocation, it happens when capacity is too small or too large
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
            // SAFETY: the data is heap allocated when abs(capacity) > 2 (invariant of Repr)
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
    pub fn as_ref(&self) -> TypedReprRef {
        match self {
            Self::Small(dword) => TypedReprRef::RefSmall(*dword),
            Self::Large(words) => TypedReprRef::RefLarge(words),
        }
    }
}

impl<'a> TypedReprRef<'a> {
    /// Get the length of the number in words, return 0 when the number is zero.
    #[inline]
    pub fn len(&self) -> usize {
        match self {
            Self::RefSmall(dword) => {
                if *dword == 0 {
                    0
                } else if *dword <= Word::MAX as DoubleWord {
                    1
                } else {
                    2
                }
            }
            Self::RefLarge(words) => words.len(),
        }
    }

    /// This operation just return a copy of `self`. It's meant to be used in macros.
    #[inline]
    pub fn as_ref(&self) -> TypedReprRef {
        *self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitive::WORD_BITS_USIZE;

    #[test]
    fn test_inline() {
        let repr = Repr::zero();
        assert_eq!(repr.capacity(), 1);
        assert_eq!(repr.len(), 0);

        let repr = Repr::from_word(123);
        assert_eq!(repr.capacity(), 1);
        assert_eq!(repr.len(), 1);

        let repr = Repr::from_dword(123 << WORD_BITS_USIZE);
        assert_eq!(repr.capacity(), 2);
        assert_eq!(repr.len(), 2);
    }

    #[test]
    fn test_deref() {
        let repr = Repr::zero();
        assert_eq!(repr.as_sign_slice(), (Sign::Positive, &[][..]));

        let repr = Repr::one();
        assert_eq!(repr.as_slice(), &[1][..]);
        assert_eq!(repr.as_sign_slice(), (Sign::Positive, &[1][..]));

        let mut buffer = Buffer::allocate(1);
        buffer.push(1);
        let repr = Repr::from_buffer(buffer).with_sign(Sign::Negative);
        assert_eq!(repr.as_sign_slice(), (Sign::Negative, &[1][..]));

        let mut buffer = Buffer::allocate(2);
        buffer.push(1);
        buffer.push(2);
        let repr = Repr::from_buffer(buffer);
        assert_eq!(repr.as_slice(), &[1, 2][..]);
        assert_eq!(repr.as_sign_slice(), (Sign::Positive, &[1, 2][..]));

        let mut buffer = Buffer::allocate(2);
        buffer.push(1);
        buffer.push(2);
        buffer.push(3);
        buffer.push(4);
        let repr = Repr::from_buffer(buffer);
        assert_eq!(repr.as_slice(), &[1, 2, 3, 4][..]);
        assert_eq!(repr.as_sign_slice(), (Sign::Positive, &[1, 2, 3, 4][..]));
    }

    #[test]
    fn test_sign() {
        let repr = Repr::zero();
        assert_eq!(repr.sign(), Sign::Positive);
        let repr = Repr::zero().neg();
        assert_eq!(repr.sign(), Sign::Positive);

        let repr = Repr::one();
        assert_eq!(repr.sign(), Sign::Positive);
        let repr = Repr::one().neg();
        assert_eq!(repr.sign(), Sign::Negative);
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
    fn test_convert_buffer() {
        let buffer = Buffer::allocate(0);
        let repr = Repr::from_buffer(buffer);
        assert_eq!(repr.len(), 0);
        assert!(repr.as_slice().is_empty());
        let buffer_back = repr.into_buffer();
        assert_eq!(buffer_back.len(), 0);
        assert!(buffer_back.is_empty());

        let mut buffer = Buffer::allocate(1);
        buffer.push(123);
        let repr = Repr::from_buffer(buffer);
        assert_eq!(repr.len(), 1);
        assert_eq!(repr.as_slice(), &[123][..]);
        let buffer_back = repr.into_buffer();
        assert_eq!(buffer_back.len(), 1);
        assert_eq!(&buffer_back[..], &[123][..]);

        let mut buffer = Buffer::allocate(2);
        buffer.push(123);
        buffer.push(456);
        let repr = Repr::from_buffer(buffer);
        assert_eq!(repr.len(), 2);
        assert_eq!(repr.as_slice(), &[123, 456][..]);
        let buffer_back = repr.into_buffer();
        assert_eq!(buffer_back.len(), 2);
        assert_eq!(&buffer_back[..], &[123, 456][..]);

        let mut buffer = Buffer::allocate(3);
        buffer.push(123);
        buffer.push(456);
        buffer.push(789);
        let repr = Repr::from_buffer(buffer);
        assert_eq!(repr.len(), 3);
        assert_eq!(repr.as_slice(), &[123, 456, 789][..]);
        let buffer_back = repr.into_buffer();
        assert_eq!(buffer_back.len(), 3);
        assert_eq!(&buffer_back[..], &[123, 456, 789][..]);
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
}
