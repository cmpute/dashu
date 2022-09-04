use crate::{
    fbig::FBig,
    repr::{Context, Word, Repr},
    round::{Round, Rounded},
};

impl<R: Round, const B: Word> PartialEq for FBig<R, B> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        // the representation is normalized so direct comparing is okay
        self.repr == other.repr
    }
}
impl<R: Round, const B: Word> Eq for FBig<R, B> {}
