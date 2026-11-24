//! Construction and oracle helpers shared by the fixed-case transcendental tests.

#![allow(dead_code)]

use dashu_base::Approximation;
use dashu_cmplx::{CBig, CRounded, FBig};
use dashu_float::round::{Round, Rounding};
use dashu_float::{Context as FloatContext, Repr};
use dashu_int::{IBig, Word};

/// The exact value `sig · B^exp` (unlimited precision).
pub fn fb<R: Round, const B: Word>(sig: i64, exp: isize) -> FBig<R, B> {
    // (`FBig::from_parts` would give the value a precision equal to its digit count)
    FBig::from_repr(Repr::new(IBig::from(sig), exp), FloatContext::new(0))
}

/// The exact complex value `re.0 · B^re.1 + i · im.0 · B^im.1` (unlimited precision).
pub fn cb<R: Round, const B: Word>(re: (i64, isize), im: (i64, isize)) -> CBig<R, B> {
    CBig::from_parts(fb(re.0, re.1), fb(im.0, im.1))
}

/// A signed zero (unlimited precision).
pub fn zero<R: Round, const B: Word>(negative: bool) -> FBig<R, B> {
    let repr = if negative {
        Repr::neg_zero()
    } else {
        Repr::zero()
    };
    FBig::from_repr(repr, FloatContext::new(0))
}

/// The same exact value under another rounding mode (unlimited precision).
pub fn retag<R1: Round, R2: Round, const B: Word>(v: &FBig<R1, B>) -> FBig<R2, B> {
    FBig::from_repr(v.repr().clone(), FloatContext::new(0))
}

/// The same exact complex value under another rounding mode (unlimited precision).
pub fn retag_c<R1: Round, R2: Round, const B: Word>(z: &CBig<R1, B>) -> CBig<R2, B> {
    let (re, im) = z.clone().into_parts();
    CBig::from_parts(retag(&re), retag(&im))
}

/// Round `v` (taken as an exact value) to `precision` digits under the mode `R2`.
pub fn reround<R1: Round, R2: Round, const B: Word>(
    v: &FBig<R1, B>,
    precision: usize,
) -> FBig<R2, B> {
    retag::<R1, R2, B>(v).with_precision(precision).value()
}

/// The per-part rounding flags of a complex result: `None` for a part-wise exact result.
pub fn flags<R: Round, const B: Word>(r: &CRounded<R, B>) -> Option<(Rounding, Rounding)> {
    match r {
        Approximation::Exact(_) => None,
        Approximation::Inexact(_, f) => Some(*f),
    }
}

/// Assert that `got` is the high-precision reference `hi` (computed under any mode, with enough
/// extra digits that it is not itself near a rounding boundary) rounded part-wise to `precision`
/// digits under `got`'s mode.
pub fn assert_rounds_from<R1: Round, R2: Round, const B: Word>(
    got: &CBig<R2, B>,
    hi: &CBig<R1, B>,
    precision: usize,
    what: &str,
) {
    let (hre, him) = hi.clone().into_parts();
    let want_re: FBig<R2, B> = reround(&hre, precision);
    let want_im: FBig<R2, B> = reround(&him, precision);
    assert_eq!(got.re(), want_re.repr(), "{what}: real part");
    assert_eq!(got.im(), want_im.repr(), "{what}: imaginary part");
}
