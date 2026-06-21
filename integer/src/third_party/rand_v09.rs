use crate::{
    arch::word::{DoubleWord, Word},
    ibig::IBig,
    ops::UnsignedAbs,
    ubig::UBig,
};
use rand_v09::{
    distr::uniform::{Error, SampleBorrow, SampleUniform, UniformSampler},
    distr::Distribution,
    Rng,
};

use super::rand::*;

/// Adapter exposing a rand 0.9 [`Rng`] as a [`BitRng`].
pub struct RngBridge<'a, R: Rng + ?Sized>(pub &'a mut R);

impl<'a, R: Rng + ?Sized> BitRng for RngBridge<'a, R> {
    #[inline]
    fn next_word(&mut self) -> Word {
        self.0.random()
    }
    #[inline]
    fn next_double_word(&mut self) -> DoubleWord {
        self.0.random()
    }
    #[inline]
    fn next_bool(&mut self) -> bool {
        self.0.random()
    }
    #[inline]
    fn fill_words(&mut self, words: &mut [Word]) {
        self.0.fill(words)
    }
    #[inline]
    fn gen_word_inclusive(&mut self, high: Word) -> Word {
        self.0.random_range(0..=high)
    }
    #[inline]
    fn gen_dword_exclusive(&mut self, high: DoubleWord) -> DoubleWord {
        self.0.random_range(0..high)
    }
}

fn bridge<R: Rng + ?Sized>(rng: &mut R) -> RngBridge<'_, R> {
    RngBridge(rng)
}

impl Distribution<UBig> for UniformBits {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> UBig {
        self.sample_ubig(&mut bridge(rng))
    }
}

impl Distribution<IBig> for UniformBits {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> IBig {
        self.sample_ibig(&mut bridge(rng))
    }
}

impl Distribution<UBig> for UniformBelow<'_> {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> UBig {
        self.sample_ubig(&mut bridge(rng))
    }
}

impl Distribution<IBig> for UniformBelow<'_> {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> IBig {
        self.sample_ibig(&mut bridge(rng))
    }
}

impl UniformSampler for UniformUBig {
    type X = UBig;

    #[inline]
    fn new<B1, B2>(low: B1, high: B2) -> Result<UniformUBig, Error>
    where
        B1: SampleBorrow<UBig> + Sized,
        B2: SampleBorrow<UBig> + Sized,
    {
        if high.borrow() <= low.borrow() {
            return Err(Error::EmptyRange);
        }
        let range = high.borrow() - low.borrow();
        Ok(UniformUBig::from_parts(range, low.borrow().clone()))
    }

    #[inline]
    fn new_inclusive<B1, B2>(low: B1, high: B2) -> Result<UniformUBig, Error>
    where
        B1: SampleBorrow<UBig> + Sized,
        B2: SampleBorrow<UBig> + Sized,
    {
        if high.borrow() < low.borrow() {
            return Err(Error::EmptyRange);
        }
        let range = high.borrow() - low.borrow() + UBig::ONE;
        Ok(UniformUBig::from_parts(range, low.borrow().clone()))
    }

    #[inline]
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> UBig {
        let mut b = bridge(rng);
        UniformUBig::sample(self, &mut b)
    }
}

impl UniformSampler for UniformIBig {
    type X = IBig;

    #[inline]
    fn new<B1, B2>(low: B1, high: B2) -> Result<UniformIBig, Error>
    where
        B1: SampleBorrow<IBig> + Sized,
        B2: SampleBorrow<IBig> + Sized,
    {
        if high.borrow() <= low.borrow() {
            return Err(Error::EmptyRange);
        }
        let range = high.borrow() - low.borrow();
        Ok(UniformIBig::from_parts(range.unsigned_abs(), low.borrow().clone()))
    }

    #[inline]
    fn new_inclusive<B1, B2>(low: B1, high: B2) -> Result<UniformIBig, Error>
    where
        B1: SampleBorrow<IBig> + Sized,
        B2: SampleBorrow<IBig> + Sized,
    {
        if high.borrow() < low.borrow() {
            return Err(Error::EmptyRange);
        }
        let range = high.borrow() - low.borrow() + IBig::ONE;
        Ok(UniformIBig::from_parts(range.unsigned_abs(), low.borrow().clone()))
    }

    #[inline]
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> IBig {
        let mut b = bridge(rng);
        UniformIBig::sample(self, &mut b)
    }
}

impl SampleUniform for UBig {
    type Sampler = UniformUBig;
}

impl SampleUniform for IBig {
    type Sampler = UniformIBig;
}
