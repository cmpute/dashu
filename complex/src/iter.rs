//! Implementation of core::iter traits for [`CBig`].
//!
//! `Sum`/`Product` fold with the binary `+`/`*` operators (componentwise perfectly-rounded real
//! ops on each part). The impls are concrete (`Sum`/`Sum<&CBig>`, `Product`/`Product<&CBig>`)
//! rather than generic over `Add`/`Mul`, matching the narrowed iter surface used for `FBig`. A
//! correctly-rounded (exact-accumulating) `Sum` for `CBig` is a possible future refinement.

use crate::cbig::CBig;
use core::iter::{Product, Sum};
use dashu_float::round::Round;
use dashu_int::Word;

impl<R: Round, const B: Word> Sum for CBig<R, B> {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(CBig::ZERO, |acc, x| acc + x)
    }
}

impl<'a, R: Round, const B: Word> Sum<&'a CBig<R, B>> for CBig<R, B> {
    fn sum<I: Iterator<Item = &'a Self>>(iter: I) -> Self {
        iter.fold(CBig::ZERO, |acc, x| acc + x)
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
}
