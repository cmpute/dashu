use crate::{fbig::FBig, repr::Word, round::Round};
use rand_v09::{
    distr::{
        uniform::{Error, SampleBorrow, SampleUniform, UniformSampler},
        Distribution, Open01, OpenClosed01, StandardUniform,
    },
    Rng,
};

use super::rand::{get_inline_precision, Uniform01, UniformFBig};

fn bridge<R: Rng + ?Sized>(rng: &mut R) -> impl dashu_int::rand::BitRng + '_ {
    dashu_int::rand::bridge_v09(rng)
}

impl<R: Round, const B: Word> Distribution<FBig<R, B>> for UniformFBig<R, B> {
    #[inline]
    fn sample<RNG: Rng + ?Sized>(&self, rng: &mut RNG) -> FBig<R, B> {
        self.sample_fbig(&mut bridge(rng))
    }
}

impl<R: Round, const B: Word> Distribution<FBig<R, B>> for Uniform01<B> {
    #[inline]
    fn sample<RNG: Rng + ?Sized>(&self, rng: &mut RNG) -> FBig<R, B> {
        self.sample01::<R, _>(&mut bridge(rng))
    }
}

impl<R: Round, const B: Word> Distribution<FBig<R, B>> for StandardUniform {
    #[inline]
    fn sample<RNG: Rng + ?Sized>(&self, rng: &mut RNG) -> FBig<R, B> {
        Uniform01::<B>::new(get_inline_precision::<B>()).sample01::<R, _>(&mut bridge(rng))
    }
}

impl<R: Round, const B: Word> Distribution<FBig<R, B>> for Open01 {
    #[inline]
    fn sample<RNG: Rng + ?Sized>(&self, rng: &mut RNG) -> FBig<R, B> {
        Uniform01::<B>::new_open(get_inline_precision::<B>()).sample01::<R, _>(&mut bridge(rng))
    }
}

impl<R: Round, const B: Word> Distribution<FBig<R, B>> for OpenClosed01 {
    #[inline]
    fn sample<RNG: Rng + ?Sized>(&self, rng: &mut RNG) -> FBig<R, B> {
        Uniform01::<B>::new_open_closed(get_inline_precision::<B>())
            .sample01::<R, _>(&mut bridge(rng))
    }
}

impl<R: Round, const B: Word> UniformSampler for UniformFBig<R, B> {
    type X = FBig<R, B>;

    #[inline]
    fn new<B1, B2>(low: B1, high: B2) -> Result<Self, Error>
    where
        B1: SampleBorrow<Self::X> + Sized,
        B2: SampleBorrow<Self::X> + Sized,
    {
        let (low, high) = (low.borrow(), high.borrow());
        if low.repr().is_infinite() || high.repr().is_infinite() {
            return Err(Error::NonFinite);
        }
        if low >= high {
            return Err(Error::EmptyRange);
        }
        let precision = low.precision().max(high.precision());
        Ok(UniformFBig::new(low, high, precision))
    }

    #[inline]
    fn new_inclusive<B1, B2>(low: B1, high: B2) -> Result<Self, Error>
    where
        B1: SampleBorrow<Self::X> + Sized,
        B2: SampleBorrow<Self::X> + Sized,
    {
        let (low, high) = (low.borrow(), high.borrow());
        if low.repr().is_infinite() || high.repr().is_infinite() {
            return Err(Error::NonFinite);
        }
        if low > high {
            return Err(Error::EmptyRange);
        }
        let precision = low.precision().max(high.precision());
        Ok(UniformFBig::new_inclusive(low, high, precision))
    }

    #[inline]
    fn sample<RNG: Rng + ?Sized>(&self, rng: &mut RNG) -> Self::X {
        self.sample_fbig(&mut bridge(rng))
    }
}

impl<R: Round, const B: Word> SampleUniform for FBig<R, B> {
    type Sampler = UniformFBig<R, B>;
}
