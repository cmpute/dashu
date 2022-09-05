use crate::{
    fbig::FBig,
    repr::{Context, Word, Repr},
    round::{Round, Rounded},
};

impl<R: Round, const B: Word> PartialEq for FBig<R, B> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        // the representation is normalized so direct comparing is okay,
        // and the context doesn't count in comparison
        self.repr == other.repr

        // TODO: what about inf?
    }
}
impl<R: Round, const B: Word> Eq for FBig<R, B> {}

// TODO: implement comparision
// 1. compare sign, inf
// 2. compare exponent + precision
// 3. compare exponent + significand.log2_est (with 2^-8 err bound?)
// 4. compare exact values by shifting etc.
