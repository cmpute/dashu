//! Definitions of [UBig].
//!
//! Conversion from internal representations including [Buffer][crate::buffer::Buffer], [TypedRepr], [TypedReprRef]
//! to [UBig] is not implemented, the designed way to construct UBig from them is first convert them
//! into [Repr], and then directly construct from the [Repr]. This restriction is set to make
//! the source type explicit.

use crate::repr::{Repr, TypedRepr, TypedReprRef};

// TODO(v0.5): move all the detailed explanations of the num types from the docs to the guide, and leave some links or brief explanations.

/// An unsigned arbitrary precision integer.
///
/// This struct represents an arbitrarily large unsigned integer. Technically the size of the integer
/// is bounded by the memory size, but it's enough for practical use on modern devices.
///
/// # Parsing and printing
///
/// To create a [UBig] instance, there are three ways:
/// 1. Use predifined constants (e.g. [UBig::ZERO], [UBig::ONE]).
/// 1. Use the literal macro `ubig!` defined in the [`dashu-macro`](https://docs.rs/dashu-macros/latest/dashu_macros/) crate.
/// 1. Parse from a string.
///
/// Parsing from either literal or string supports representation with base 2~36.
///
/// For printing, the [UBig] type supports common formatting traits ([Display][core::fmt::Display],
/// [Debug][core::fmt::Debug], [LowerHex][core::fmt::LowerHex], etc.). Specially, printing huge number
/// using [Debug][core::fmt::Debug] will conveniently omit the middle digits of the number, only print
/// the least and most significant (decimal) digits.
///
/// ```
/// # use dashu_base::ParseError;
/// # use dashu_int::{UBig, Word};
/// // parsing
/// let a = UBig::from(408580953453092208335085386466371u128);
/// let b = UBig::from(0x1231abcd4134u64);
/// let c = UBig::from_str_radix("a2a123bbb127779cccc123", 32)?;
/// let d = UBig::from_str_radix("1231abcd4134", 16)?;
/// assert_eq!(a, c);
/// assert_eq!(b, d);
///
/// // printing
/// assert_eq!(format!("{}", UBig::from(12u8)), "12");
/// assert_eq!(format!("{:#X}", UBig::from(0xabcdu16)), "0xABCD");
/// if Word::BITS == 64 {
///     // number of digits to display depends on the word size
///     assert_eq!(
///         format!("{:?}", UBig::ONE << 1000),
///         "1071508607186267320..4386837205668069376"
///     );
/// }
/// # Ok::<(), ParseError>(())
/// ```
///
/// # Memory
///
/// Integers that fit in a [DoubleWord][crate::DoubleWord] will be inlined on stack and
/// no heap allocation will be invoked. For large integers, they will be represented as
/// an array of [Word][crate::Word]s, and stored on heap.
///
/// Note that the [UBig] struct has a niche bit, therefore it can be used within simple
/// enums with no memory overhead.
///
/// ```
/// # use dashu_int::UBig;
/// use core::mem::size_of;
/// assert_eq!(size_of::<UBig>(), size_of::<Option<UBig>>());
/// ```
#[derive(Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct UBig(pub(crate) Repr);

impl UBig {
    /// Get the representation of UBig.
    #[rustversion::attr(since(1.64), const)]
    #[inline]
    pub(crate) fn repr(&self) -> TypedReprRef<'_> {
        self.0.as_typed()
    }

    /// Convert into representation.
    #[inline]
    pub(crate) fn into_repr(self) -> TypedRepr {
        self.0.into_typed()
    }

    /// [UBig] with value 0
    pub const ZERO: Self = Self(Repr::zero());
    /// [UBig] with value 1
    pub const ONE: Self = Self(Repr::one());

    /// Get the raw representation in [Word][crate::Word]s.
    ///
    /// If the number is zero, then empty slice will be returned.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::{UBig, Word};
    /// assert_eq!(UBig::ZERO.as_words(), &[] as &[Word]);
    /// assert_eq!(UBig::ONE.as_words(), &[1]);
    /// ```
    #[inline]
    pub fn as_words(&self) -> &[crate::Word] {
        let (sign, words) = self.0.as_sign_slice();
        debug_assert!(matches!(sign, crate::Sign::Positive));
        words
    }

    /// Create a UBig from a single [Word][crate::Word].
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::UBig;
    /// const ZERO: UBig = UBig::from_word(0);
    /// assert_eq!(ZERO, UBig::ZERO);
    /// const ONE: UBig = UBig::from_word(1);
    /// assert_eq!(ONE, UBig::ONE);
    /// ```
    #[inline]
    pub const fn from_word(word: crate::Word) -> Self {
        Self(Repr::from_word(word))
    }

    /// Create a UBig from a [DoubleWord][crate::DoubleWord].
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::UBig;
    /// const ZERO: UBig = UBig::from_dword(0);
    /// assert_eq!(ZERO, UBig::ZERO);
    /// const ONE: UBig = UBig::from_dword(1);
    /// assert_eq!(ONE, UBig::ONE);
    /// ```
    #[inline]
    pub const fn from_dword(dword: crate::DoubleWord) -> Self {
        Self(Repr::from_dword(dword))
    }

    /// Convert a sequence of [Word][crate::Word]s into a UBig
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::{UBig, Word};
    /// assert_eq!(UBig::from_words(&[] as &[Word]), UBig::ZERO);
    /// assert_eq!(UBig::from_words(&[1]), UBig::ONE);
    /// assert_eq!(UBig::from_words(&[1, 1]), (UBig::ONE << Word::BITS as usize) + UBig::ONE);
    /// ```
    #[inline]
    pub fn from_words(words: &[crate::Word]) -> Self {
        Self(Repr::from_buffer(words.into()))
    }

    /// Create an UBig from a static sequence of [Word][crate::Word]s. Similar to [from_words][UBig::from_words].
    ///
    /// The top word of the input word array must not be zero.
    ///
    /// This method is unsafe because it must be carefully handled. The generated instance
    /// must not be mutated or dropped. Therefore the correct usage is to assign it to an
    /// immutable static variable. Due to the risk, it's generally not recommended to use this method.
    /// This method is intended for the use of static creation macros.
    #[doc(hidden)]
    #[inline]
    pub const unsafe fn from_static_words(words: &'static [crate::Word]) -> Self {
        Self(Repr::from_static_words(words))
    }

    /// Check whether the value is 0
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::UBig;
    /// assert!(UBig::ZERO.is_zero());
    /// assert!(!UBig::ONE.is_zero());
    /// ```
    #[inline]
    pub const fn is_zero(&self) -> bool {
        self.0.is_zero()
    }

    /// Check whether the value is 1
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::UBig;
    /// assert!(!UBig::ZERO.is_one());
    /// assert!(UBig::ONE.is_one());
    /// ```
    #[inline]
    pub const fn is_one(&self) -> bool {
        self.0.is_one()
    }

    /// Create an integer with `n` consecutive one bits (i.e. 2^n - 1).
    /// 
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::UBig;
    /// let mut n = UBig::ZERO;
    /// n.set_bit(20);
    /// n -= UBig::ONE;
    /// assert_eq!(UBig::ones(20), n);
    /// ```
    #[inline]
    pub fn ones(n: usize) -> Self {
        Self(Repr::ones(n))
    }
}

// This custom implementation is necessary due to https://github.com/rust-lang/rust/issues/98374
impl Clone for UBig {
    #[inline]
    fn clone(&self) -> UBig {
        UBig(self.0.clone())
    }
    #[inline]
    fn clone_from(&mut self, source: &UBig) {
        self.0.clone_from(&source.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{buffer::Buffer, DoubleWord, Word};

    impl UBig {
        /// Capacity in Words.
        #[inline]
        fn capacity(&self) -> usize {
            self.0.capacity()
        }
    }

    fn gen_ubig(num_words: u16) -> UBig {
        let mut buf = Buffer::allocate(num_words.into());
        for i in 0..num_words {
            buf.push(i.into());
        }
        UBig(Repr::from_buffer(buf))
    }

    #[test]
    fn test_buffer_to_ubig() {
        let buf = Buffer::allocate(5);
        let num = UBig(Repr::from_buffer(buf));
        assert_eq!(num, UBig::ZERO);

        let mut buf = Buffer::allocate(5);
        buf.push(7);
        let num = UBig(Repr::from_buffer(buf));
        assert_eq!(num, UBig::from(7u8));

        let mut buf = Buffer::allocate(100);
        buf.push(7);
        buf.push(0);
        buf.push(0);
        let num = UBig(Repr::from_buffer(buf));
        assert_eq!(num, UBig::from(7u8));

        let mut buf = Buffer::allocate(5);
        buf.push(1);
        buf.push(2);
        buf.push(3);
        buf.push(4);
        let num = UBig(Repr::from_buffer(buf));
        assert_eq!(num.capacity(), 7);

        let mut buf = Buffer::allocate(100);
        buf.push(1);
        buf.push(2);
        buf.push(3);
        buf.push(4);
        let num = UBig(Repr::from_buffer(buf));
        assert_eq!(num.capacity(), 6);
    }

    #[test]
    fn test_clone() {
        let a = UBig::from(5u8);
        assert_eq!(a.clone(), a);

        let a = gen_ubig(10);
        let b = a.clone();
        assert_eq!(a, b);
        assert_eq!(a.capacity(), b.capacity());
    }

    #[test]
    fn test_clone_from() {
        let num: UBig = gen_ubig(10);

        let mut a = UBig::from(3u8);
        a.clone_from(&num);
        assert_eq!(a, num);
        let b = UBig::from(7u8);
        a.clone_from(&b);
        assert_eq!(a, b);
        a.clone_from(&b);
        assert_eq!(a, b);

        let mut a = gen_ubig(9);
        let prev_cap = a.capacity();
        a.clone_from(&num);
        // the buffer should be reused, 9 is close enough to 10.
        assert_eq!(a.capacity(), prev_cap);
        assert_ne!(a.capacity(), num.capacity());

        let mut a = gen_ubig(3);
        let prev_cap = a.capacity();
        a.clone_from(&num);
        // the buffer should now be reallocated, it's too Small.
        assert_ne!(a.capacity(), prev_cap);
        assert_eq!(a.capacity(), num.capacity());

        let mut a = gen_ubig(100);
        let prev_cap = a.capacity();
        a.clone_from(&num);
        // the buffer should now be reallocated, it's too large.
        assert_ne!(a.capacity(), prev_cap);
        assert_eq!(a.capacity(), num.capacity());
    }

    #[test]
    fn test_const_generation() {
        const ZERO: UBig = UBig::from_word(0);
        const ONE_SINGLE: UBig = UBig::from_word(1);
        const ONE_DOUBLE: UBig = UBig::from_dword(1);
        const DMAX: UBig = UBig::from_dword(DoubleWord::MAX);

        const CDATA: [Word; 3] = [Word::MAX, Word::MAX, Word::MAX];
        // SAFETY: DATA meets the requirements of from_static_words
        static CONST_TMAX: UBig = unsafe { UBig::from_static_words(&CDATA) };
        static DATA: [Word; 3] = [Word::MAX, Word::MAX, Word::MAX];
        // SAFETY: DATA meets the requirements of from_static_words
        static STATIC_TMAX: UBig = unsafe { UBig::from_static_words(&DATA) };

        assert_eq!(ZERO, UBig::ZERO);
        assert_eq!(ONE_SINGLE, UBig::ONE);
        assert_eq!(ONE_DOUBLE, UBig::ONE);
        assert_eq!(DMAX.capacity(), 2);
        assert_eq!(CONST_TMAX.capacity(), 3);
        assert_eq!(STATIC_TMAX.capacity(), 3);
    }
}
