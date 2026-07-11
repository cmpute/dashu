//! `rand` 0.8 `Distribution` impls for `CBig` (the `rand` feature aliases `rand_v08`).

use crate::cbig::CBig;
use crate::third_party::rand::UniformCBig;
use dashu_float::round::Round;
use dashu_float::FBig;
use dashu_int::Word;
use rand_v08::distributions::{Open01, OpenClosed01, Standard};
use rand_v08::prelude::Distribution;
use rand_v08::Rng;

fn bridge<R: Rng + ?Sized>(rng: &mut R) -> impl dashu_int::rand::BitRng + '_ {
    dashu_int::rand::bridge_v08(rng)
}

impl<R: Round, const B: Word> Distribution<CBig<R, B>> for UniformCBig<R, B> {
    #[inline]
    fn sample<RNG: Rng + ?Sized>(&self, rng: &mut RNG) -> CBig<R, B> {
        self.sample_cbig(&mut bridge(rng))
    }
}

impl<R: Round, const B: Word> Distribution<CBig<R, B>> for Standard {
    /// Each part uniform in `[0, 1)` → the unit square `[0, 1)²`, at inline precision.
    #[inline]
    fn sample<RNG: Rng + ?Sized>(&self, rng: &mut RNG) -> CBig<R, B> {
        let re: FBig<R, B> = Standard.sample(rng);
        let im: FBig<R, B> = Standard.sample(rng);
        CBig::from_parts(re, im)
    }
}

impl<R: Round, const B: Word> Distribution<CBig<R, B>> for Open01 {
    #[inline]
    fn sample<RNG: Rng + ?Sized>(&self, rng: &mut RNG) -> CBig<R, B> {
        let re: FBig<R, B> = Open01.sample(rng);
        let im: FBig<R, B> = Open01.sample(rng);
        CBig::from_parts(re, im)
    }
}

impl<R: Round, const B: Word> Distribution<CBig<R, B>> for OpenClosed01 {
    #[inline]
    fn sample<RNG: Rng + ?Sized>(&self, rng: &mut RNG) -> CBig<R, B> {
        let re: FBig<R, B> = OpenClosed01.sample(rng);
        let im: FBig<R, B> = OpenClosed01.sample(rng);
        CBig::from_parts(re, im)
    }
}
