//! Implement zeroize traits.

use crate::{
    fbig::FBig,
    repr::{Context, Repr, Word},
    round::Round,
};
use zeroize::Zeroize;

impl<const B: Word> Zeroize for Repr<B> {
    #[inline]
    fn zeroize(&mut self) {
        self.significand.zeroize();
        self.exponent.zeroize();
    }
}

impl<R: Round> Zeroize for Context<R> {
    #[inline]
    fn zeroize(&mut self) {
        self.precision.zeroize();
    }
}

impl<R: Round, const B: Word> Zeroize for FBig<R, B> {
    #[inline]
    fn zeroize(&mut self) {
        self.repr.zeroize();
        self.context.zeroize();
    }
}
