//! Random integers generation with the `rand` crate.
//!
//! There are four distributions for generating random integers. The first two are [UniformBits],
//! and [UniformBelow] which limit the bit size or the magnitude of the generated integer.
//! The other two are [UniformUBig] and [UniformIBig], which supports generating random integers
//! uniformly in a given range. These traits are also the backends for the [SampleUniform] trait.
//!
//! # Examples
//!
//! ```
//! # use dashu_int::{UBig, IBig};
//! use rand::{distributions::uniform::Uniform, thread_rng, Rng};
//!
//! // generate UBigs in a range
//! let a = thread_rng().gen_range(UBig::from(3u8)..UBig::from(10u8));
//! let b = thread_rng().sample(Uniform::new(UBig::ZERO, &a));
//! assert!(a >= UBig::from(3u8) && a < UBig::from(10u8));
//! assert!(b >= UBig::ZERO && b < a);
//!
//! // generate IBigs in a range
//! let a = thread_rng().gen_range(IBig::from(3)..IBig::from(10));
//! let b = thread_rng().sample(Uniform::new(IBig::from(-5), &a));
//! assert!(a >= IBig::from(3) && a < IBig::from(10));
//! assert!(b >= IBig::from(-5) && b < a);
//!
//! // generate UBigs and IBigs with a given bit size limit.
//! use dashu_base::BitTest;
//! use dashu_int::rand::UniformBits;
//! let a: UBig = thread_rng().sample(UniformBits::new(10));
//! let b: IBig = thread_rng().sample(UniformBits::new(10));
//! assert!(a.bit_len() <= 10 && b.bit_len() <= 10);
//!
//! // generate UBigs and IBigs with a given magnitude limit.
//! use dashu_int::rand::UniformBelow;
//! let a: UBig = thread_rng().sample(UniformBelow::new(&10u8.into()));
//! let b: IBig = thread_rng().sample(UniformBelow::new(&10u8.into()));
//! assert!(a < UBig::from(10u8));
//! assert!(b < IBig::from(10) && b > IBig::from(-10));
//! ```

use crate::{
    arch::word::{DoubleWord, Word},
    buffer::Buffer,
    error::panic_empty_range,
    ibig::IBig,
    math::ceil_div,
    ops::UnsignedAbs,
    primitive::{DWORD_BITS_USIZE, WORD_BITS_USIZE},
    repr::{Repr, TypedReprRef::*},
    ubig::UBig,
};

use dashu_base::Sign;
use rand::{
    distributions::uniform::{SampleBorrow, SampleUniform, UniformSampler},
    prelude::Distribution,
    Rng,
};

/// Uniform distribution for both [UBig] and [IBig] specified by bits.
///
/// This distribution generate random integers uniformly between `[0, 2^bits)` for [UBig],
/// between `(-2^bits, 2^bits)` for [IBig].
pub struct UniformBits {
    bits: usize,
}

impl UniformBits {
    /// Create a [UniformBits] distribution with a given limit on the integer's bit length.
    #[inline]
    pub const fn new(bits: usize) -> Self {
        UniformBits { bits }
    }
}

impl Distribution<UBig> for UniformBits {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> UBig {
        if self.bits == 0 {
            UBig::ZERO
        } else if self.bits <= DWORD_BITS_USIZE {
            let dword: DoubleWord = rng.gen();
            UBig::from_dword(dword >> (DWORD_BITS_USIZE - self.bits))
        } else {
            let num_words = ceil_div(self.bits, WORD_BITS_USIZE);
            let mut buffer = Buffer::allocate(num_words);
            buffer.push_zeros(num_words);

            rng.fill(buffer.as_mut());

            let rem = self.bits % WORD_BITS_USIZE;
            if rem != 0 {
                *buffer.last_mut().unwrap() >>= WORD_BITS_USIZE - rem;
            }

            UBig(Repr::from_buffer(buffer))
        }
    }
}

impl Distribution<IBig> for UniformBits {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> IBig {
        loop {
            let mag: UBig = self.sample(rng);
            let neg = rng.gen();
            if mag.is_zero() && neg {
                // Reject negative zero so that all possible integers
                // have the same probability. This branch should happen
                // very rarely.
                continue;
            }
            break IBig::from_parts(Sign::from(neg), mag);
        }
    }
}

/// Uniform distribution around zero for both [UBig] and [IBig] specified by a limit of the magnitude.
///
/// This distribution generate random integers uniformly between `[0, limit)` for [UBig],
/// between `(-limit, limit)` for [IBig].
pub struct UniformBelow<'a> {
    limit: &'a UBig,
}

impl<'a> UniformBelow<'a> {
    /// Create a [UniformBelow] distribution with a given limit on the integer's magnitude.
    #[inline]
    pub const fn new(limit: &'a UBig) -> Self {
        Self { limit }
    }
}

impl<'a> Distribution<UBig> for UniformBelow<'a> {
    #[inline]
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> UBig {
        UBig::uniform(self.limit, rng)
    }
}

impl<'a> Distribution<IBig> for UniformBelow<'a> {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> IBig {
        loop {
            let mag: UBig = UBig::uniform(self.limit, rng);
            let neg = rng.gen();
            if mag.is_zero() && neg {
                // Reject negative zero so that all possible integers
                // have the same probability. This branch should happen
                // very rarely.
                continue;
            }
            break IBig::from_parts(Sign::from(neg), mag);
        }
    }
}

impl UBig {
    /// Random UBig in range [0..range)
    #[inline]
    fn uniform<R>(range: &UBig, rng: &mut R) -> UBig
    where
        R: Rng + ?Sized,
    {
        debug_assert!(!range.is_zero());

        match range.repr() {
            RefSmall(dword) => UBig::from(rng.gen_range(0..dword)),
            RefLarge(words) => UBig::uniform_large(words, rng),
        }
    }

    /// Random UBig in range [0..words)
    fn uniform_large<R>(words: &[Word], rng: &mut R) -> UBig
    where
        R: Rng + ?Sized,
    {
        let mut buffer = Buffer::allocate(words.len());
        buffer.push_zeros(words.len());
        while !try_fill_uniform(words, rng, &mut buffer) {
            // Repeat.
        }
        UBig(Repr::from_buffer(buffer))
    }
}

/// Try to fill `sample` with random number in range [0..words).
/// May fail randomly.
///
/// Returns true on success.
fn try_fill_uniform<R>(words: &[Word], rng: &mut R, result: &mut [Word]) -> bool
where
    R: Rng + ?Sized,
{
    let n = words.len();
    debug_assert!(n > 0 && result.len() == n);
    let mut i = n - 1;
    result[i] = rng.gen_range(0..=words[i]);
    // With at least 50% probability this loop executes 0 times (and thus doesn't fail).
    while result[i] == words[i] {
        if i == 0 {
            // result == words
            return false;
        }
        i -= 1;
        result[i] = rng.gen();
        if result[i] > words[i] {
            return false;
        }
    }
    rng.fill(&mut result[..i]);
    true
}

/// The back-end implementing [UniformSampler] for [UBig].
///
/// See the module ([rand][crate::rand]) level documentation for examples.
pub struct UniformUBig {
    range: UBig,
    offset: UBig,
}

impl UniformSampler for UniformUBig {
    type X = UBig;

    #[inline]
    fn new<B1, B2>(low: B1, high: B2) -> UniformUBig
    where
        B1: SampleBorrow<UBig>,
        B2: SampleBorrow<UBig>,
    {
        let range = high.borrow() - low.borrow();
        if range.is_zero() {
            panic_empty_range()
        }
        UniformUBig {
            range,
            offset: low.borrow().clone(),
        }
    }

    #[inline]
    fn new_inclusive<B1, B2>(low: B1, high: B2) -> UniformUBig
    where
        B1: SampleBorrow<UBig>,
        B2: SampleBorrow<UBig>,
    {
        let range = high.borrow() - low.borrow() + UBig::ONE;
        UniformUBig {
            range,
            offset: low.borrow().clone(),
        }
    }

    #[inline]
    fn sample<R>(&self, rng: &mut R) -> UBig
    where
        R: Rng + ?Sized,
    {
        UBig::uniform(&self.range, rng) + &self.offset
    }
}

/// The back-end implementing [UniformSampler] for [IBig].
///
/// See the module ([rand][crate::rand]) level documentation for examples.
pub struct UniformIBig {
    range: UBig,
    offset: IBig,
}

impl UniformSampler for UniformIBig {
    type X = IBig;

    #[inline]
    fn new<B1, B2>(low: B1, high: B2) -> UniformIBig
    where
        B1: SampleBorrow<IBig>,
        B2: SampleBorrow<IBig>,
    {
        let range = high.borrow() - low.borrow();
        if range <= IBig::ZERO {
            panic_empty_range();
        }
        UniformIBig {
            range: range.unsigned_abs(),
            offset: low.borrow().clone(),
        }
    }

    #[inline]
    fn new_inclusive<B1, B2>(low: B1, high: B2) -> UniformIBig
    where
        B1: SampleBorrow<IBig>,
        B2: SampleBorrow<IBig>,
    {
        let range = high.borrow() - low.borrow() + IBig::from(1u8);
        if range <= IBig::ZERO {
            panic_empty_range()
        }
        UniformIBig {
            range: range.unsigned_abs(),
            offset: low.borrow().clone(),
        }
    }

    #[inline]
    fn sample<R>(&self, rng: &mut R) -> IBig
    where
        R: Rng + ?Sized,
    {
        IBig::from(UBig::uniform(&self.range, rng)) + &self.offset
    }
}

impl SampleUniform for UBig {
    type Sampler = UniformUBig;
}

impl SampleUniform for IBig {
    type Sampler = UniformIBig;
}
