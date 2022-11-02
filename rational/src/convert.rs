use dashu_int::{error::OutOfBoundsError, IBig, UBig};

use crate::rbig::RBig;

impl From<u8> for RBig {
    #[inline]
    fn from(v: u8) -> RBig {
        RBig::from_parts(v.into(), UBig::ONE)
    }
}

impl TryFrom<RBig> for IBig {
    type Error = OutOfBoundsError; // TODO(v0.3): change to PrecisionLossError
    #[inline]
    fn try_from(value: RBig) -> Result<Self, Self::Error> {
        if value.0.denominator.is_one() {
            Ok(value.0.numerator)
        } else {
            Err(OutOfBoundsError)
        }
    }
}

impl TryFrom<RBig> for u8 {
    type Error = OutOfBoundsError;
    #[inline]
    fn try_from(value: RBig) -> Result<Self, Self::Error> {
        let int: IBig = value.try_into()?;
        int.try_into()
    }
}
