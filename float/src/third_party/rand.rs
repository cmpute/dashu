//! Support the uniform distribution with the `rand` crate.

use crate::{fbig::FBig, repr::Word, round::Round};

use dashu_base::EstimatedLog2;
use dashu_int::{
    rand::{UniformBits, UniformUBig},
    DoubleWord, UBig,
};
use rand::{
    distributions::{
        uniform::{SampleBorrow, SampleUniform, UniformSampler},
        Open01, OpenClosed01, Standard,
    },
    prelude::Distribution,
    Rng,
};

enum PrecisionLimit {
    /// Stores number of bits, for non-inclusive only
    Bits(usize),
    /// Stores the precision and base ^ precision (+1 if inclusive)
    Pow(usize, UBig),
}

pub struct UniformFBig<R: Round, const B: Word> {
    precision: PrecisionLimit,
    scale: FBig<R, B>,
    offset: FBig<R, B>,
}

impl<R: Round, const B: Word> UniformFBig<R, B> {
    fn new(low: &FBig<R, B>, high: &FBig<R, B>, precision: usize) -> Self {
        let (scale, offset) = (high - low, low.clone());
        let precision = if B == 2 {
            PrecisionLimit::Bits(precision)
        } else {
            PrecisionLimit::Pow(precision, UBig::from_word(B).pow(precision))
        };
        Self {
            precision,
            scale,
            offset,
        }
    }

    fn new_inclusive(low: &FBig<R, B>, high: &FBig<R, B>, precision: usize) -> Self {
        let (scale, offset) = (high - low, low.clone());
        let precision =
            PrecisionLimit::Pow(precision, UBig::from_word(B).pow(precision) + UBig::ONE);
        Self {
            precision,
            scale,
            offset,
        }
    }
}

impl<R: Round, const B: Word> UniformSampler for UniformFBig<R, B> {
    type X = FBig<R, B>;

    fn new<B1, B2>(low: B1, high: B2) -> Self
    where
        B1: SampleBorrow<Self::X> + Sized,
        B2: SampleBorrow<Self::X> + Sized,
    {
        let precision = (DoubleWord::BITS as f32 / B.log2_bounds().1) as usize;
        UniformFBig::new(low.borrow(), high.borrow(), precision)
    }

    fn new_inclusive<B1, B2>(low: B1, high: B2) -> Self
    where
        B1: SampleBorrow<Self::X> + Sized,
        B2: SampleBorrow<Self::X> + Sized,
    {
        let precision = (DoubleWord::BITS as f32 / B.log2_bounds().1) as usize;
        UniformFBig::new_inclusive(low.borrow(), high.borrow(), precision)
    }

    fn sample<RNG: Rng + ?Sized>(&self, rng: &mut RNG) -> Self::X {
        let unit = match &self.precision {
            PrecisionLimit::Bits(bits) => {
                let signif: UBig = UniformBits::new(*bits).sample(rng);
                FBig::from_parts(signif.into(), -(*bits as isize))
            }
            PrecisionLimit::Pow(prec, pow) => {
                let signif = UniformUBig::new(UBig::ZERO, pow).sample(rng);
                FBig::from_parts(signif.into(), -(*prec as isize))
            }
        };
        unit * &self.scale + &self.offset
    }
}

impl<R: Round, const B: Word> SampleUniform for FBig<R, B> {
    type Sampler = UniformFBig<R, B>;
}

impl<R: Round, const B: Word> Distribution<FBig<R, B>> for Standard {
    fn sample<RNG: Rng + ?Sized>(&self, rng: &mut RNG) -> FBig<R, B> {
        // when sampling with the standard distribution, the precision is choosen
        // such that the significand fits in a double word and no allocation is required.
        if B == 2 {
            let signif: DoubleWord = rng.gen();
            FBig::from_parts(signif.into(), -(DoubleWord::BITS as isize))
        } else {
            let precision = (DoubleWord::BITS as f32 / B.log2_bounds().1) as u32;
            let signif = rng.gen_range(0..(B as DoubleWord).pow(precision));
            FBig::from_parts(signif.into(), -(precision as isize))
        }
    }
}

impl<R: Round, const B: Word> Distribution<FBig<R, B>> for Open01 {
    fn sample<RNG: Rng + ?Sized>(&self, rng: &mut RNG) -> FBig<R, B> {
        if B == 2 {
            // simply reject the last value
            let signif: DoubleWord = loop {
                let dword = rng.gen();
                if dword != DoubleWord::MAX {
                    break dword;
                }
            };
            FBig::from_parts(signif.into(), -(DoubleWord::BITS as isize))
        } else {
            let precision = (DoubleWord::BITS as f32 / B.log2_bounds().1) as u32;
            let signif = rng.gen_range(0..(B as DoubleWord).pow(precision) - 1);
            FBig::from_parts(signif.into(), -(precision as isize))
        }
    }
}

impl<R: Round, const B: Word> Distribution<FBig<R, B>> for OpenClosed01 {
    fn sample<RNG: Rng + ?Sized>(&self, rng: &mut RNG) -> FBig<R, B> {
        // sample in [0, 1) and convert 0 to 1
        let f: FBig<R, B> = Standard.sample(rng);
        if f.repr().is_zero() {
            FBig::ONE
        } else {
            f
        }
    }
}
