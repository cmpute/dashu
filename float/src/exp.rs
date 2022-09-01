use dashu_int::IBig;
use crate::{round::{Round, Rounded}, repr::{Word, Context}, fbig::FBig};

impl<R: Round> Context<R> {
    pub fn powi<const B: Word>(&self, x: &IBig) -> Rounded<FBig<B, R>> {
        unimplemented!()
    }

    #[inline]
    pub fn exp<const B: Word>(&self, x: &FBig<B, R>) -> Rounded<FBig<B, R>> {
        self.exp_internal(x, false)
    }

    #[inline]
    pub fn exp_m1<const B: Word>(&self, x: &FBig<B, R>) -> Rounded<FBig<B, R>> {
        self.exp_internal(x, true)
    }

    fn exp_internal<const B: Word>(&self, x: &FBig<B, R>, minus_one: bool) -> Rounded<FBig<B, R>> {
        // A simple algorithm:
        // - let r = (x - s logB) / B^m, where s = floor(x / logB), such that r < B^-m.
        // - if the target precision is n digits, then only about max_k = n/m terms in Tyler series
        // - the optimal m is sqrt(x) as given by MPFR when minus_one is false
        // - finally, exp(x) = B^s * exp(r)^(B^m) (use pow i)
        // - if minus_one is true and x is already small (x < 1/B), then directly evaluate the Tyler series (s = 0, m = 0)
        unimplemented!()
    }
}
