//! Random distributions.

use crate::{
    arch::word::Word,
    buffer::Buffer,
    error::panic_empty_range,
    ibig::IBig,
    ops::UnsignedAbs,
    repr::{Repr, TypedReprRef::*},
    ubig::UBig,
};

use rand::{
    distributions::uniform::{SampleBorrow, SampleUniform, UniformSampler},
    Rng,
};

impl SampleUniform for UBig {
    type Sampler = UniformUBig;
}

impl SampleUniform for IBig {
    type Sampler = UniformIBig;
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

/// Uniform [UBig] distribution.
///
/// # Examples
///
/// ```
/// use dashu_int::UBig;
/// use rand::{distributions::uniform::Uniform, thread_rng, Rng};
/// let a = thread_rng().gen_range(UBig::from(3u8)..UBig::from(10u8));
/// let b = thread_rng().sample(Uniform::new(UBig::ZERO, &a));
/// assert!(a >= 3 && a < 10);
/// assert!(b >= 0 && b < a);
/// ```
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

/// Uniform [IBig] distribution.
///
/// # Examples
///
/// ```
/// use dashu_int::IBig;
/// use rand::{distributions::uniform::Uniform, thread_rng, Rng};
/// let a = thread_rng().gen_range(IBig::from(3)..IBig::from(10));
/// let b = thread_rng().sample(Uniform::new(IBig::from(-5), &a));
/// assert!(a >= 3 && a < 10);
/// assert!(b >= -5 && b < a);
/// ```
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
