//! Operators on the sign of [IBig].

use crate::{
    ibig::IBig,
    buffer::TypedReprRef::RefSmall,
    ops::{Abs, UnsignedAbs},
    ubig::UBig,
};
use core::ops::Neg;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub(crate) enum Sign {
    Positive,
    Negative,
}

use Sign::*;

impl Neg for Sign {
    type Output = Sign;

    #[inline]
    fn neg(self) -> Sign {
        match self {
            Positive => Negative,
            Negative => Positive,
        }
    }
}

impl IBig {
    /// A number representing the sign of `self`.
    ///
    /// * -1 if the number is negative
    /// * 0 if the number is zero
    /// * 1 if the number is positive
    ///
    /// # Examples
    /// ```
    /// # use dashu_int::ibig;
    /// assert_eq!(ibig!(-500).signum(), ibig!(-1));
    /// ```
    #[inline]
    pub fn signum(&self) -> IBig {
        let (sign, repr) = self.as_sign_repr();
        if let RefSmall(0) = repr {
            IBig::zero()
        } else {
            match sign {
                Positive => IBig::one(),
                Negative => IBig::neg_one(),
            }
        }
    }
}

impl Neg for IBig {
    type Output = IBig;

    #[inline]
    fn neg(self) -> IBig {
        let mut repr = self.0;
        repr.set_sign(-repr.sign());
        IBig(repr)
    }
}

impl Neg for &IBig {
    type Output = IBig;

    #[inline]
    fn neg(self) -> IBig {
        self.clone().neg()
    }
}

impl Abs for IBig {
    type Output = IBig;

    #[inline]
    fn abs(self) -> IBig {
        IBig::from(self.unsigned_abs())
    }
}

impl Abs for &IBig {
    type Output = IBig;

    #[inline]
    fn abs(self) -> IBig {
        IBig::from(self.unsigned_abs())
    }
}

impl UnsignedAbs for IBig {
    type Output = UBig;

    #[inline]
    fn unsigned_abs(self) -> UBig {
        let mut repr = self.0;
        repr.set_sign(Sign::Positive);
        UBig(repr)
    }
}

impl UnsignedAbs for &IBig {
    type Output = UBig;

    #[inline]
    fn unsigned_abs(self) -> UBig {
        self.magnitude().clone()
    }
}
