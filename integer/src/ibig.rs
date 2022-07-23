//! Signed big integer.

use crate::{
    repr::{Repr, TypedRepr, TypedReprRef},
    sign::Sign,
    UBig,
};

/// Signed big integer.
///
/// Arbitrarily large signed integer.
///
/// # Examples
///
/// ```
/// # use dashu_int::{error::ParseError, ibig, IBig};
/// let a = ibig!(a2a123bbb127779cccc123123ccc base 32);
/// let b = ibig!(-0x1231abcd4134);
/// let c = IBig::from_str_radix("a2a123bbb127779cccc123123ccc", 32)?;
/// let d = IBig::from_str_radix("-1231abcd4134", 16)?;
/// assert_eq!(a, c);
/// assert_eq!(b, d);
/// # Ok::<(), ParseError>(())
/// ```
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

    // TODO: make Sign public
    #[inline]
    pub fn sign(&self) -> Sign {
        self.0.sign()
    }

    #[inline]
    pub fn to_sign_magnitude(self) -> (Sign, UBig) {
        let sign = self.0.sign();
        let mag = self.0.with_sign(Sign::Positive);
        (sign, UBig(mag))
    }

    #[inline]
    pub fn from_sign_magnitude(sign: Sign, magnitude: UBig) -> Self {
        IBig(magnitude.0.with_sign(sign))
    }

    /// Create an IBig with value 0
    #[inline]
    pub const fn zero() -> Self {
        IBig(Repr::zero())
    }

    /// Check whether the value of IBig is 0
    #[inline]
    pub const fn is_zero(&self) -> bool {
        self.0.is_zero()
    }

    /// Create an IBig with value 1
    #[inline]
    pub const fn one() -> Self {
        IBig(Repr::one())
    }

    /// Check whether the value of IBig is 1
    #[inline]
    pub const fn is_one(&self) -> bool {
        self.0.is_one()
    }

    /// Create an IBig with value -1
    #[inline]
    pub const fn neg_one() -> IBig {
        IBig(Repr::neg_one())
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
