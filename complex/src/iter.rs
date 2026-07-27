//! Implementation of core::iter traits for [`CBig`].
//!
//! `Sum` is exact-accumulating: since complex addition is componentwise, the real and imaginary
//! parts are summed independently via `FBig`'s exact-accumulating `Sum` (each axis accumulates its
//! `Repr`s losslessly and rounds once), so the result is the correctly-rounded complex sum. `Product`
//! folds with the binary `*` operator (componentwise near-correct). The impls are concrete
//! (`Sum`/`Sum<&CBig>`, `Product`/`Product<&CBig>`) rather than generic over `Add`/`Mul`, matching
//! the narrowed iter surface used for `FBig`.

use crate::cbig::CBig;
use alloc::vec::Vec;
use core::iter::{Product, Sum};
use dashu_float::round::Round;
use dashu_float::FBig;
use dashu_int::Word;

/// Exact-accumulating complex sum: split into real/imaginary [`FBig`] streams and sum each via
/// `FBig`'s `Sum` (which accumulates `Repr`s exactly and rounds once at the target context). An
/// empty stream sums to `CBig::ZERO` (both axes are `FBig::ZERO`).
fn precise_sum<R: Round, const B: Word>(
    parts: impl Iterator<Item = (FBig<R, B>, FBig<R, B>)>,
) -> CBig<R, B> {
    let (re_parts, im_parts): (Vec<FBig<R, B>>, Vec<FBig<R, B>>) = parts.unzip();
    CBig::from_parts(re_parts.into_iter().sum(), im_parts.into_iter().sum())
}

impl<R: Round, const B: Word> Sum for CBig<R, B> {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        precise_sum(iter.map(CBig::into_parts))
    }
}

impl<'a, R: Round, const B: Word> Sum<&'a CBig<R, B>> for CBig<R, B> {
    fn sum<I: Iterator<Item = &'a Self>>(iter: I) -> Self {
        precise_sum(iter.map(|c| c.clone().into_parts()))
    }
}

impl<R: Round, const B: Word> Product for CBig<R, B> {
    fn product<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(CBig::ONE, |acc, x| acc * x)
    }
}

impl<'a, R: Round, const B: Word> Product<&'a CBig<R, B>> for CBig<R, B> {
    fn product<I: Iterator<Item = &'a Self>>(iter: I) -> Self {
        iter.fold(CBig::ONE, |acc, x| acc * x)
    }
}

#[cfg(test)]
mod tests {
    use crate::cbig::CBig;
    use dashu_float::round::mode::HalfAway;
    use dashu_float::FBig;
    use dashu_int::IBig;

    type C = CBig<HalfAway, 10>;
    type F = FBig<HalfAway, 10>;

    #[test]
    fn sum_owned_and_ref() {
        let z = C::from_parts(F::from(3), F::from(4)); // 3 + 4i
        let w = C::from_parts(F::from(1), F::from(2)); // 1 + 2i
        let expected = C::from_parts(F::from(4), F::from(6));

        let owned: C = [z.clone(), w.clone()].into_iter().sum();
        assert_eq!(owned, expected);

        let by_ref: C = [z, w].iter().sum();
        assert_eq!(by_ref, expected);
    }

    #[test]
    fn product_owned_and_ref() {
        // (3+4i)(1+2i) = (3-8) + (6+4)i = -5 + 10i
        let z = C::from_parts(F::from(3), F::from(4));
        let w = C::from_parts(F::from(1), F::from(2));
        let expected = C::from_parts(F::from(-5), F::from(10));

        let owned: C = [z.clone(), w.clone()].into_iter().product();
        assert_eq!(owned, expected);

        let by_ref: C = [z, w].iter().product();
        assert_eq!(by_ref, expected);
    }

    #[test]
    fn sum_and_product_empty() {
        let s: C = core::iter::empty::<C>().sum();
        assert_eq!(s, C::ZERO);
        let p: C = core::iter::empty::<C>().product();
        assert_eq!(p, C::ONE);
    }

    #[test]
    fn sum_exact_cancellation() {
        // z + (-z) = +0 on both axes (canonical +0, not -0, under HalfAway).
        let z = C::from_parts(F::from(3), F::from(4));
        let s: C = [z, -C::from_parts(F::from(3), F::from(4))]
            .into_iter()
            .sum();
        assert_eq!(s, C::ZERO);
    }

    #[test]
    fn sum_exact_accumulates_wide_magnitudes() {
        // At precision 4, (1e20 + 1 - 1e20) folds to 0 (the +1 is lost when added to 1e20), but
        // exact accumulation preserves it: the sum is 1 on each axis. This distinguishes the
        // exact-accumulating `Sum` from a naive fold.
        let mk = |sig: i32, exp: isize| {
            F::from_parts(IBig::from(sig), exp)
                .with_precision(4)
                .value()
        };
        let big = C::from_parts(mk(1, 20), mk(1, 20));
        let one = C::from_parts(mk(1, 0), mk(1, 0));
        let neg_big = C::from_parts(mk(-1, 20), mk(-1, 20));
        let s: C = [big, one, neg_big].into_iter().sum();
        assert_eq!(s, C::from_parts(mk(1, 0), mk(1, 0)));
    }
}
