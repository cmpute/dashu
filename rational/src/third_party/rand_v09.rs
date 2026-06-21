use crate::{rbig::RBig, repr::Repr, Relaxed};
use dashu_int::{rand::UniformIBig, DoubleWord, UBig};
use rand_v09::{
    distr::{
        uniform::{Error, SampleBorrow, SampleUniform, UniformSampler},
        Distribution, Open01, OpenClosed01, StandardUniform,
    },
    Rng,
};

use super::rand::{Uniform01, UniformRBig};

fn bridge<R: Rng + ?Sized>(rng: &mut R) -> impl dashu_int::rand::BitRng + '_ {
    dashu_int::rand::bridge_v09(rng)
}

impl<'a> Distribution<Repr> for Uniform01<'a> {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Repr {
        self.sample_repr(&mut bridge(rng))
    }
}

impl<'a> Distribution<RBig> for Uniform01<'a> {
    #[inline]
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> RBig {
        self.sample_rbig(&mut bridge(rng))
    }
}

impl<'a> Distribution<Relaxed> for Uniform01<'a> {
    #[inline]
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Relaxed {
        self.sample_relaxed(&mut bridge(rng))
    }
}

macro_rules! impl_builtin_distr {
    (impl $dist:ident for $t:ty, $ctor:ident, $core:ident) => {
        impl Distribution<$t> for $dist {
            #[inline]
            fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> $t {
                let limit = UBig::from_dword(DoubleWord::MAX);
                Uniform01::$ctor(&limit).$core(&mut bridge(rng))
            }
        }
    };
}

impl_builtin_distr!(impl StandardUniform for RBig, new, sample_rbig);
impl_builtin_distr!(impl StandardUniform for Relaxed, new, sample_relaxed);
impl_builtin_distr!(impl Open01 for RBig, new_open, sample_rbig);
impl_builtin_distr!(impl Open01 for Relaxed, new_open, sample_relaxed);
impl_builtin_distr!(impl OpenClosed01 for RBig, new_open_closed, sample_rbig);
impl_builtin_distr!(impl OpenClosed01 for Relaxed, new_open_closed, sample_relaxed);

impl UniformSampler for UniformRBig {
    type X = RBig;

    #[inline]
    fn new<B1, B2>(low: B1, high: B2) -> Result<UniformRBig, Error>
    where
        B1: SampleBorrow<RBig> + Sized,
        B2: SampleBorrow<RBig> + Sized,
    {
        let (low_n, high_n, den) = UniformRBig::parse_bounds(low.borrow(), high.borrow());
        Ok(UniformRBig::from_parts(UniformIBig::new(low_n, high_n)?, den))
    }

    #[inline]
    fn new_inclusive<B1, B2>(low: B1, high: B2) -> Result<UniformRBig, Error>
    where
        B1: SampleBorrow<RBig> + Sized,
        B2: SampleBorrow<RBig> + Sized,
    {
        let (low_n, high_n, den) = UniformRBig::parse_bounds(low.borrow(), high.borrow());
        Ok(UniformRBig::from_parts(UniformIBig::new_inclusive(low_n, high_n)?, den))
    }

    #[inline]
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> RBig {
        self.sample_rbig(&mut bridge(rng))
    }
}

impl SampleUniform for RBig {
    type Sampler = UniformRBig;
}
