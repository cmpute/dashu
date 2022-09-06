//! Signed big integer.

use crate::{
    repr::{Repr, TypedRepr, TypedReprRef},
    Sign,
    UBig,
};

/// Signed big integer.
///
/// Arbitrarily large signed integer.
///
/// The IBig struct is compact with a niche bit. It has the same memory size as [UBig]
/// and it can could be used within simple enums with no additional memory requirement.
///
/// ```
/// # use dashu_int::{IBig, UBig};
/// use core::mem;
/// assert_eq!(mem::size_of::<IBig>(), mem::size_of::<UBig>());
/// assert_eq!(mem::size_of::<IBig>(), mem::size_of::<Option<IBig>>());
/// ```
///
/// # Examples
///
/// ```
/// # use dashu_int::{error::ParseError, IBig};
/// let a = IBig::from(408580953453092208335085386466371u128);
/// let b = IBig::from(-0x1231abcd4134i64);
/// let c = IBig::from_str_radix("a2a123bbb127779cccc123", 32)?;
/// let d = IBig::from_str_radix("-1231abcd4134", 16)?;
/// assert_eq!(a, c);
/// assert_eq!(b, d);
/// # Ok::<(), ParseError>(())
/// ```
///
#[derive(Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct IBig(pub(crate) Repr);

impl IBig {
    #[inline]
    pub(crate) fn as_sign_repr(&self) -> (Sign, TypedReprRef<'_>) {
        self.0.as_sign_typed()
    }

    #[inline]
    pub(crate) fn into_sign_repr(self) -> (Sign, TypedRepr) {
        self.0.into_sign_typed()
    }

    /// Get the raw representation in [Word][crate::Word]s.
    ///
    /// If the number is zero, then empty slice will be returned.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::{IBig, Sign};
    /// assert_eq!(IBig::ZERO.as_sign_words(), (Sign::Positive, &[] as &[_]));
    /// assert_eq!(IBig::NEG_ONE.as_sign_words().0, Sign::Negative);
    /// assert_eq!(IBig::NEG_ONE.as_sign_words().1, &[1]);
    /// ```
    #[inline]
    pub fn as_sign_words(&self) -> (Sign, &[crate::Word]) {
        self.0.as_sign_slice()
    }

    /// Get the sign of the [IBig]. Zero value has a positive sign.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::{IBig, Sign};
    /// assert_eq!(IBig::ZERO.sign(), Sign::Positive);
    /// assert_eq!(IBig::from(2).sign(), Sign::Positive);
    /// assert_eq!(IBig::from(-3).sign(), Sign::Negative);
    /// ```
    #[inline]
    pub const fn sign(&self) -> Sign {
        self.0.sign()
    }

    /// Convert the [IBig] into its [Sign] and [UBig] magnitude
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::{IBig, Sign, UBig};
    /// assert_eq!(IBig::ZERO.into_parts(), (Sign::Positive, UBig::ZERO));
    /// assert_eq!(IBig::ONE.into_parts(), (Sign::Positive, UBig::ONE));
    /// assert_eq!(IBig::NEG_ONE.into_parts(), (Sign::Negative, UBig::ONE));
    /// ```
    #[inline]
    pub fn into_parts(self) -> (Sign, UBig) {
        let sign = self.0.sign();
        let mag = self.0.with_sign(Sign::Positive);
        (sign, UBig(mag))
    }

    /// Create an [IBig] from the [Sign] and [UBig] magnitude
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::{IBig, Sign, UBig};
    /// assert_eq!(IBig::from_parts(Sign::Positive, UBig::ZERO), IBig::ZERO);
    /// assert_eq!(IBig::from_parts(Sign::Positive, UBig::ONE), IBig::ONE);
    /// assert_eq!(IBig::from_parts(Sign::Negative, UBig::ONE), IBig::NEG_ONE);
    /// ```
    #[inline]
    pub fn from_parts(sign: Sign, magnitude: UBig) -> Self {
        IBig(magnitude.0.with_sign(sign))
    }

    /// Create an IBig from a [Sign] and a [DoubleWord][crate::DoubleWord]
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::{IBig, Sign, UBig};
    /// const ONE: IBig = IBig::from_parts_const(Sign::Positive, 1);
    /// assert_eq!(ONE, IBig::ONE);
    /// const NEG_ONE: IBig = IBig::from_parts_const(Sign::Negative, 1);
    /// assert_eq!(NEG_ONE, IBig::NEG_ONE);
    /// ```
    #[inline]
    pub const fn from_parts_const(sign: Sign, dword: crate::DoubleWord) -> Self {
        Self(Repr::from_dword(dword).with_sign(sign))
    }

    /// [IBig] with value 0
    pub const ZERO: Self = Self(Repr::zero());
    /// [IBig] with value 1
    pub const ONE: Self = Self(Repr::one());
    /// [IBig] with value -1
    pub const NEG_ONE: Self = Self(Repr::neg_one());

    /// Check whether the value is 0
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::IBig;
    /// assert!(IBig::ZERO.is_zero());
    /// assert!(!IBig::ONE.is_zero());
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
    /// # use dashu_int::IBig;
    /// assert!(!IBig::ZERO.is_one());
    /// assert!(IBig::ONE.is_one());
    /// ```
    #[inline]
    pub const fn is_one(&self) -> bool {
        self.0.is_one()
    }
}

// This custom implementation is necessary due to https://github.com/rust-lang/rust/issues/98374
impl Clone for IBig {
    #[inline]
    fn clone(&self) -> IBig {
        IBig(self.0.clone())
    }

    #[inline]
    fn clone_from(&mut self, source: &IBig) {
        self.0.clone_from(&source.0)
    }
}
