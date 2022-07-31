//! Definitions of [UBig].
//!
//! Conversion from internal representations including [Buffer][crate::buffer::Buffer], [TypedRepr], [TypedReprRef]
//! to [UBig] is not implemented, the designed way to construct UBig from them is first convert them
//! into [Repr], and then directly construct from the [Repr]. This restriction is set to make
//! the source type explicit.

use crate::{
    repr::{Repr, TypedRepr, TypedReprRef},
};

/// Unsigned big integer.
///
/// Arbitrarily large unsigned integer.
///
/// # Examples
///
/// ```
/// # use dashu_int::{error::ParseError, UBig};
/// let a = UBig::from(408580953453092208335085386466371u128);
/// let b = UBig::from(0x1231abcd4134u64);
/// let c = UBig::from_str_radix("a2a123bbb127779cccc123", 32)?;
/// let d = UBig::from_str_radix("1231abcd4134", 16)?;
/// assert_eq!(a, c);
/// assert_eq!(b, d);
/// # Ok::<(), ParseError>(())
/// ```
/// 
/// The UBig struct has a niche bit, therefore it can be used within simple enums
/// with no additional memory requirement.
/// 
/// ```
/// # use dashu_int::UBig;
/// use core::mem;
/// assert_eq!(mem::size_of::<UBig>(), mem::size_of::<Option<UBig>>());
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

    /// [UBig] with value 0
    pub const ZERO: Self = Self(Repr::zero());
    /// [UBig] with value 1
    pub const ONE: Self = Self(Repr::one());

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
    use crate::buffer::Buffer;

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

    fn gen_ubig(num_words: u16) -> UBig {
        let mut buf = Buffer::allocate(num_words.into());
        for i in 0..num_words {
            buf.push(i.into());
        }
        UBig(Repr::from_buffer(buf))
    }
}
