//! Unsigned big integer.

use crate::{
    arch::{ntt, word::Word},
    repr::{Buffer, Repr, TypedRepr, TypedReprRef},
    math,
    primitive::WORD_BITS_USIZE,
};

/// Unsigned big integer.
///
/// Arbitrarily large unsigned integer.
///
/// # Examples
///
/// ```
/// # use dashu_int::{error::ParseError, ubig, UBig};
/// let a = ubig!(a2a123bbb127779cccc123123ccc base 32);
/// let b = ubig!(0x1231abcd4134);
/// let c = UBig::from_str_radix("a2a123bbb127779cccc123123ccc", 32)?;
/// let d = UBig::from_str_radix("1231abcd4134", 16)?;
/// assert_eq!(a, c);
/// assert_eq!(b, d);
/// # Ok::<(), ParseError>(())
/// ```
#[derive(Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct UBig(pub(crate) Repr);

impl UBig {
    /// Get the representation of UBig.
    #[inline]
    pub(crate) fn repr(&self) -> TypedReprRef<'_> {
        self.0.as_typed()
    }

    /// Convert into representation.
    #[inline]
    pub(crate) fn into_repr(self) -> TypedRepr {
        self.0.into_typed()
    }

    /// Create a UBig with value 0
    #[inline]
    pub const fn zero() -> Self {
        UBig(Repr::zero())
    }

    /// Check whether the value of UBig is 0
    #[inline]
    pub const fn is_zero(&self) -> bool {
        self.0.is_zero()
    }

    /// Create a UBig with value 1
    #[inline]
    pub const fn one() -> Self {
        UBig(Repr::one())
    }

    /// Check whether the value of UBig is 1
    #[inline]
    pub const fn is_one(&self) -> bool {
        self.0.is_one()
    }

    /// Representation in Words.
    // TODO: expose this
    #[inline]
    pub(crate) fn as_words(&self) -> &[Word] {
        let (sign, words) = self.0.as_sign_slice();
        debug_assert!(matches!(sign, crate::sign::Sign::Positive));
        words
    }

    /// Maximum length in `Word`s.
    ///
    /// Ensures that the number of bits fits in `usize`, which is useful for bit count
    /// operations, and for radix conversions (even base 2 can be represented).
    ///
    /// This also guarantees that up to 16 * length will not overflow.
    ///
    /// We also make sure that any multiplication whose result fits in `MAX_LEN` can fit
    /// within the largest possible number-theoretic transform.
    ///
    /// Also make sure this is even, useful for checking whether a square will overflow.
    // TODO: only check allocation failure when doing multiplication or shifting
    pub(crate) const MAX_LEN: usize = math::min_usize(
        usize::MAX / WORD_BITS_USIZE,
        match 1usize.checked_shl(ntt::MAX_ORDER) {
            Some(ntt_len) => ntt_len,
            None => usize::MAX,
        },
    ) & !1usize;

    /// Maximum length in bits.
    ///
    /// [UBig]s up to this length are supported. Creating a longer number
    /// will panic.
    ///
    /// This does not guarantee that there is sufficient memory to store numbers
    /// up to this length. Memory allocation may fail even for Smaller numbers.
    ///
    /// The fact that this limit fits in `usize` guarantees that all bit
    /// addressing operations can be performed using `usize`.
    ///
    /// It is typically close to `usize::MAX`, but the exact value is platform-dependent.
    pub const MAX_BIT_LEN: usize = UBig::MAX_LEN * WORD_BITS_USIZE;

    pub(crate) fn panic_number_too_large() -> ! {
        panic!("number too large, maximum is {} bits", UBig::MAX_BIT_LEN)
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

// TODO: we shouldn't need this if we implemented all ops as repr
impl From<Buffer> for UBig {
    #[inline]
    fn from(buffer: Buffer) -> UBig {
        UBig(Repr::from_buffer(buffer))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    impl UBig {
        /// Capacity in Words.
        #[inline]
        pub(crate) fn capacity(&self) -> usize {
            self.0.capacity()
        }
    }

    #[test]
    fn test_buffer_to_ubig() {
        let buf = Buffer::allocate(5);
        let num: UBig = buf.into();
        assert_eq!(num, UBig::zero());

        let mut buf = Buffer::allocate(5);
        buf.push(7);
        let num: UBig = buf.into();
        assert_eq!(num, UBig::from(7u8));

        let mut buf = Buffer::allocate(100);
        buf.push(7);
        buf.push(0);
        buf.push(0);
        let num: UBig = buf.into();
        assert_eq!(num, UBig::from(7u8));

        let mut buf = Buffer::allocate(5);
        buf.push(1);
        buf.push(2);
        buf.push(3);
        buf.push(4);
        let num: UBig = buf.into();
        assert_eq!(num.capacity(), 7);

        let mut buf = Buffer::allocate(100);
        buf.push(1);
        buf.push(2);
        buf.push(3);
        buf.push(4);
        let num: UBig = buf.into();
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

    fn gen_ubig(num_words: u16) -> UBig {
        let mut buf = Buffer::allocate(num_words.into());
        for i in 0..num_words {
            buf.push(i.into());
        }
        buf.into()
    }
}
