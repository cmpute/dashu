//! Random integer generation with the `rand` crate.
//!
//! There are four distributions for generating random integers. The first two are [UniformBits]
//! and [UniformBelow], which limit the bit size or the magnitude of the generated integer. The
//! other two are [UniformUBig] and [UniformIBig], which generate random integers uniformly in a
//! given range; they are also the backends for rand's `SampleUniform` trait.
//!
//! The distributions and their sampling algorithms are defined here once, generic over the
//! [BitRng] trait. Each rand version's `Distribution` / `UniformSampler` / `SampleUniform` impls
//! live in the `rand_v08` / `rand_v09` / `rand_v010` modules (enable the matching feature); use
//! `bridge_v08` / `bridge_v09` / `bridge_v010` to adapt that version's RNG into a [BitRng].
//! See those modules for usage examples.

use crate::{
    arch::word::{DoubleWord, Word},
    buffer::Buffer,
    ibig::IBig,
    math::ceil_div,
    primitive::{DWORD_BITS_USIZE, WORD_BITS_USIZE},
    repr::{Repr, TypedReprRef::*},
    ubig::UBig,
};
use dashu_base::Sign;

/// Version-agnostic source of random machine words.
///
/// Each `rand_vXX` module implements this for a thin adapter wrapping that rand version's RNG,
/// so the generation algorithms live once in this module instead of being copy-pasted per rand
/// version. Users may also implement it themselves to drive these distributions from a non-`rand`
/// RNG.
pub trait BitRng {
    /// A uniformly random full `Word`.
    fn next_word(&mut self) -> Word;
    /// A uniformly random `DoubleWord`.
    fn next_double_word(&mut self) -> DoubleWord;
    /// A uniformly random `bool`.
    fn next_bool(&mut self) -> bool;
    /// Fill `words` entirely with uniformly random data.
    fn fill_words(&mut self, words: &mut [Word]);
    /// Uniformly random `Word` in `0..=high` (inclusive).
    fn gen_word_inclusive(&mut self, high: Word) -> Word;
    /// Uniformly random `DoubleWord` in `0..high` (exclusive).
    fn gen_dword_exclusive(&mut self, high: DoubleWord) -> DoubleWord;
}

/// Uniform distribution for both [UBig] and [IBig] specified by bits.
///
/// This distribution generates random integers uniformly between `[0, 2^bits)` for [UBig], and
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

    /// Draw a random [UBig] within this distribution's bit bound.
    pub fn sample_ubig<R: BitRng + ?Sized>(&self, rng: &mut R) -> UBig {
        if self.bits == 0 {
            UBig::ZERO
        } else if self.bits <= DWORD_BITS_USIZE {
            let dword: DoubleWord = rng.next_double_word();
            UBig::from_dword(dword >> (DWORD_BITS_USIZE - self.bits))
        } else {
            let num_words = ceil_div(self.bits, WORD_BITS_USIZE);
            let mut buffer = Buffer::allocate(num_words);
            buffer.push_zeros(num_words);

            rng.fill_words(buffer.as_mut());

            let rem = self.bits % WORD_BITS_USIZE;
            if rem != 0 {
                *buffer.last_mut().unwrap() >>= WORD_BITS_USIZE - rem;
            }

            UBig(Repr::from_buffer(buffer))
        }
    }

    /// Draw a random [IBig] within this distribution's bit bound.
    pub fn sample_ibig<R: BitRng + ?Sized>(&self, rng: &mut R) -> IBig {
        loop {
            let mag: UBig = self.sample_ubig(rng);
            let neg = rng.next_bool();
            if mag.is_zero() && neg {
                // Reject negative zero so that all possible integers have the same
                // probability. This branch should happen very rarely.
                continue;
            }
            break IBig::from_parts(Sign::from(neg), mag);
        }
    }
}

/// Uniform distribution around zero for both [UBig] and [IBig], bounded by magnitude.
///
/// This distribution generates random integers uniformly between `[0, limit)` for [UBig], and
/// between `(-limit, limit)` for [IBig].
pub struct UniformBelow<'a> {
    limit: &'a UBig,
}

impl<'a> UniformBelow<'a> {
    /// Create a [UniformBelow] distribution with a given limit on the magnitude.
    #[inline]
    pub const fn new(limit: &'a UBig) -> Self {
        Self { limit }
    }

    /// Draw a random [UBig] below the limit.
    #[inline]
    pub fn sample_ubig<R: BitRng + ?Sized>(&self, rng: &mut R) -> UBig {
        uniform(self.limit, rng)
    }

    /// Draw a random [IBig] with magnitude below the limit.
    pub fn sample_ibig<R: BitRng + ?Sized>(&self, rng: &mut R) -> IBig {
        loop {
            let mag: UBig = uniform(self.limit, rng);
            let neg = rng.next_bool();
            if mag.is_zero() && neg {
                // Reject negative zero so that all possible integers have the same
                // probability. This branch should happen very rarely.
                continue;
            }
            break IBig::from_parts(Sign::from(neg), mag);
        }
    }
}

/// Random [UBig] in range `[0..range)`.
fn uniform<R: BitRng + ?Sized>(range: &UBig, rng: &mut R) -> UBig {
    debug_assert!(!range.is_zero());

    match range.repr() {
        RefSmall(dword) => UBig::from(rng.gen_dword_exclusive(dword)),
        RefLarge(words) => uniform_large(words, rng),
    }
}

/// Random [UBig] in range `[0..words)`.
fn uniform_large<R: BitRng + ?Sized>(words: &[Word], rng: &mut R) -> UBig {
    let mut buffer = Buffer::allocate(words.len());
    buffer.push_zeros(words.len());
    while !try_fill_uniform(words, rng, &mut buffer) {
        // Repeat.
    }
    UBig(Repr::from_buffer(buffer))
}

/// Try to fill `result` with a random number in range `[0..words)`. May fail randomly.
///
/// Returns true on success.
fn try_fill_uniform<R: BitRng + ?Sized>(words: &[Word], rng: &mut R, result: &mut [Word]) -> bool {
    let n = words.len();
    debug_assert!(n > 0 && result.len() == n);
    let mut i = n - 1;
    result[i] = rng.gen_word_inclusive(words[i]);
    // With at least 50% probability this loop executes 0 times (and thus doesn't fail).
    while result[i] == words[i] {
        if i == 0 {
            // result == words
            return false;
        }
        i -= 1;
        result[i] = rng.next_word();
        if result[i] > words[i] {
            return false;
        }
    }
    rng.fill_words(&mut result[..i]);
    true
}

/// The back-end implementing `rand::distributions::uniform::UniformSampler` for [UBig].
pub struct UniformUBig {
    pub(crate) range: UBig,
    pub(crate) offset: UBig,
}

impl UniformUBig {
    #[inline]
    pub(crate) const fn from_parts(range: UBig, offset: UBig) -> Self {
        UniformUBig { range, offset }
    }

    /// Draw a random [UBig] from this sampler's `[low, high)` range.
    #[inline]
    pub fn sample<R: BitRng + ?Sized>(&self, rng: &mut R) -> UBig {
        uniform(&self.range, rng) + &self.offset
    }
}

/// The back-end implementing `rand::distributions::uniform::UniformSampler` for [IBig].
pub struct UniformIBig {
    pub(crate) range: UBig,
    pub(crate) offset: IBig,
}

impl UniformIBig {
    #[inline]
    pub(crate) const fn from_parts(range: UBig, offset: IBig) -> Self {
        UniformIBig { range, offset }
    }

    /// Draw a random [IBig] from this sampler's `[low, high)` range.
    #[inline]
    pub fn sample<R: BitRng + ?Sized>(&self, rng: &mut R) -> IBig {
        IBig::from(uniform(&self.range, rng)) + &self.offset
    }
}

/// Adapt a rand 0.8 [`Rng`](rand_v08::Rng) into a [`BitRng`].
#[cfg(feature = "rand_v08")]
pub fn bridge_v08<'a, R: rand_v08::Rng + ?Sized>(rng: &'a mut R) -> impl BitRng + 'a {
    super::rand_v08::RngBridge(rng)
}

/// Adapt a rand 0.9 [`Rng`](rand_v09::Rng) into a [`BitRng`].
#[cfg(feature = "rand_v09")]
pub fn bridge_v09<'a, R: rand_v09::Rng + ?Sized>(rng: &'a mut R) -> impl BitRng + 'a {
    super::rand_v09::RngBridge(rng)
}

/// Adapt a rand 0.10 [`Rng`](rand_v010::Rng) into a [`BitRng`].
#[cfg(feature = "rand_v010")]
pub fn bridge_v010<'a, R: rand_v010::Rng + ?Sized>(rng: &'a mut R) -> impl BitRng + 'a {
    super::rand_v010::RngBridge(rng)
}
