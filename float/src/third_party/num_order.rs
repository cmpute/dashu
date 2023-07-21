use core::cmp::Ordering;
use dashu_int::{IBig, Word};
use num_order::NumOrd;

use crate::{round::Round, FBig, Repr};

impl<R1: Round, R2: Round, const B1: Word, const B2: Word> NumOrd<FBig<R2, B2>> for FBig<R1, B1> {
    fn num_cmp(&self, other: &FBig<R2, B2>) -> Ordering {
        todo!()
    }
    #[inline]
    fn num_partial_cmp(&self, other: &FBig<R2, B2>) -> Option<Ordering> {
        Some(self.num_cmp(other))
    }
}

impl<R: Round, const B: Word> NumOrd<IBig> for FBig<R, B> {
    fn num_cmp(&self, other: &IBig) -> Ordering {
        todo!()
    }
    #[inline]
    fn num_partial_cmp(&self, other: &IBig) -> Option<Ordering> {
        Some(self.num_cmp(other))
    }
}
