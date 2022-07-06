//! Signed big integer.

use crate::{
    sign::Sign,
    buffer::{Repr, TypedRepr, TypedReprRef},
    ubig::UBig,
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

    #[inline]
    pub(crate) fn from_sign_magnitude(sign: Sign, magnitude: UBig) -> IBig {
        let repr = magnitude.0;
        if repr.is_zero() {
            IBig(repr)
        } else {
            IBig(repr.with_sign(sign))
        }
    }

    #[inline]
    pub(crate) fn sign(&self) -> Sign {
        self.0.sign()
    }

    #[inline]
    pub(crate) fn into_sign_magnitude(self) -> (Sign, UBig) {
        (self.0.sign(), UBig(self.0.with_sign(Sign::Positive)))
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
