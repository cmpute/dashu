//! Random complex number generation with the `rand` crate.
//!
//! [`UniformCBig`] samples a complex number with each part uniform in a per-part range
//! `[low, high)` — i.e. uniformly over the **box** `[low.re, high.re) × [low.im, high.im)`. The
//! builtin rand distributions (`Standard`/`StandardUniform`, `Open01`, `OpenClosed01`) generate a
//! complex number with each part uniform in `[0, 1)` — the **unit square** `[0, 1)²` — at inline
//! precision (each part's significand fits in a `DoubleWord`).
//!
//! The distribution is defined here once, generic over [`dashu_int::rand::BitRng`], and reuses
//! [`dashu_float::rand::UniformFBig`] for each part (a random `CBig` is just two independent
//! random `FBig` parts). Each rand version's `Distribution` impls live in the `rand_v08` /
//! `rand_v09` / `rand_v010` modules; enable the matching feature and adapt that version's RNG with
//! `dashu_int::rand::bridge_v08` / `bridge_v09` / `bridge_v010`.

use crate::cbig::CBig;
use dashu_float::rand::{Uniform01 as FloatUniform01, UniformFBig};
use dashu_float::round::Round;
use dashu_float::FBig;
use dashu_int::rand::BitRng;
use dashu_int::Word;

/// Uniform distribution over the box `[low, high)`: the real part is uniform in `[low.re, high.re)`
/// and the imaginary part in `[low.im, high.im)`, each at a chosen precision.
///
/// There is no single-axis `Uniform`/`SampleUniform` for `CBig` — complex numbers have no interval
/// order. Use this box sampler, or compose two [`UniformFBig`] ranges via [`CBig::from_parts`].
pub struct UniformCBig<R: Round, const B: Word> {
    pub(crate) re: UniformFBig<R, B>,
    pub(crate) im: UniformFBig<R, B>,
}

impl<R: Round, const B: Word> UniformCBig<R, B> {
    /// Create a sampler over the box `[low, high)` at `precision` (the two parts are sampled
    /// independently; each part's range is `[low.part, high.part)`).
    ///
    /// # Panics
    ///
    /// Panics if `low.re > high.re` or `low.im > high.im`.
    pub fn new(low: &CBig<R, B>, high: &CBig<R, B>, precision: usize) -> Self {
        Self {
            re: UniformFBig::new(&part_view(low, true), &part_view(high, true), precision),
            im: UniformFBig::new(&part_view(low, false), &part_view(high, false), precision),
        }
    }

    /// Draw a random [`CBig`] from this sampler's box.
    pub fn sample_cbig<BR: BitRng + ?Sized>(&self, rng: &mut BR) -> CBig<R, B> {
        let re = self.re.sample_fbig(rng);
        let im = self.im.sample_fbig(rng);
        CBig::from_parts(re, im)
    }
}

/// Borrow one part of a [`CBig`] as an [`FBig`] view (clones the `Repr`, attaches the shared
/// context). Used only to feed [`UniformFBig::new`], which reads the value and precision.
fn part_view<R: Round, const B: Word>(z: &CBig<R, B>, re: bool) -> FBig<R, B> {
    let ctx = z.context().float();
    let repr = if re { z.re().clone() } else { z.im().clone() };
    FBig::from_repr(repr, ctx)
}

/// Uniform distribution over the unit square `(0, 1)²` — each part independent and uniform in
/// `(0, 1)`, at the chosen precision (mirroring [`dashu_float::rand::Uniform01`]).
///
/// Used by the builtin `rand` distribution impls (`Standard`, `Open01`, `OpenClosed01`) for
/// [`CBig`]; can also be constructed directly for custom-precision sampling.
pub struct Uniform01<const BASE: Word> {
    pub(crate) re: FloatUniform01<BASE>,
    pub(crate) im: FloatUniform01<BASE>,
}

impl<const B: Word> Uniform01<B> {
    /// Create a uniform distribution in `[0, 1)²` at the given precision.
    #[inline]
    pub fn new(precision: usize) -> Self {
        Self {
            re: FloatUniform01::new(precision),
            im: FloatUniform01::new(precision),
        }
    }

    /// Create a uniform distribution in `[0, 1]²` at the given precision.
    #[inline]
    pub fn new_closed(precision: usize) -> Self {
        Self {
            re: FloatUniform01::new_closed(precision),
            im: FloatUniform01::new_closed(precision),
        }
    }

    /// Create a uniform distribution in `(0, 1)²` at the given precision.
    #[inline]
    pub fn new_open(precision: usize) -> Self {
        Self {
            re: FloatUniform01::new_open(precision),
            im: FloatUniform01::new_open(precision),
        }
    }

    /// Draw a random [`CBig`] with both parts in this sampler's interval.
    pub fn sample_cbig<R: Round, BR: BitRng + ?Sized>(&self, rng: &mut BR) -> CBig<R, B> {
        let re = self.re.sample01(rng);
        let im = self.im.sample01(rng);
        CBig::from_parts(re, im)
    }
}
