//! Signed big integer.

use crate::{
    sign::Sign::{self, *},
    buffer::{Buffer, Repr, TypedRepr, TypedReprRef},
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
pub struct IBig(pub(crate) Repr);

impl IBig {
    #[inline]
    pub(crate) fn signed_repr(&self) -> (Sign, TypedReprRef<'_>) {
        self.0.as_sign_typed()
    }

    #[inline]
    pub(crate) fn into_sign_repr(self) -> (Sign, TypedRepr) {
        self.0.into_sign_typed()
    }

    #[inline]
    pub(crate) fn from_sign_magnitude(sign: Sign, mut magnitude: UBig) -> IBig {
        if magnitude != UBig::from_word(0) { // TODO: specialize is_zero
            magnitude.0.set_sign(sign)
        }
        IBig(magnitude.0)
    }

    #[inline]
    pub(crate) fn sign(&self) -> Sign {
        unimplemented!()
    }

    #[inline]
    pub(crate) fn magnitude(&self) -> &UBig {
        unimplemented!()
    }

    #[inline]
    pub(crate) fn into_sign_magnitude(self) -> (Sign, UBig) {
        unimplemented!()
    }
}
