use crate::{repr::Repr, rbig::{RBig, Relaxed}};
use dashu_base::ConversionError;
use dashu_float::{FBig, round::{Round, Rounded}};
use dashu_int::Word;

impl<R: Round, const B: Word> From<Repr> for FBig<R, B> {
    fn from(_: Repr) -> Self {
        todo!()
    }
}

impl<R: Round, const B: Word> TryFrom<FBig<R, B>> for Repr {
    type Error = ConversionError;
    fn try_from(value: FBig<R, B>) -> Result<Self, Self::Error> {
        todo!()
    }
}

// TODO(next): forward the `From` implementations for RBig and Relaxed to Repr using macros

impl Repr {
    fn to_float<R: Round, const B: Word>(&self) -> Rounded<FBig<R, B>> {
        todo!()
    }
}

impl RBig {
    #[inline]
    pub fn to_float<R: Round, const B: Word>(&self) -> Rounded<FBig<R, B>> {
        self.0.to_float()
    }

    pub fn simplest_from_float<R: Round, const B: Word>(float: &FBig<R, B>) -> Self {
        todo!()
    }
}

impl Relaxed {
    #[inline]
    pub fn to_float<R: Round, const B: Word>(&self) -> Rounded<FBig<R, B>> {
        self.0.to_float()
    }
}
