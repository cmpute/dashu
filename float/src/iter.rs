//! Implementation of core::iter traits

use crate::{
    error::assert_finite,
    fbig::FBig,
    repr::{Context, Repr, Word},
    round::Round,
};
use core::iter::{Product, Sum};

/// Correctly-rounded summation of finite floats.
///
/// Every addend is accumulated exactly at the [`Repr`] level (no per-step rounding); the exact total
/// is then rounded a single time to the target context. The target context is `Context::max` over all
/// addend contexts (matching chained `+`), and the final round reuses [`Context::repr_round`] — this
/// yields the same result as rounding the mathematically exact sum (MPFR `mpfr_sum` semantics).
///
/// Because the accumulator is exact, summing addends with widely differing exponents can grow the
/// intermediate significand to span the full exponent range of the inputs.
fn precise_sum<R: Round, const B: Word>(
    mut iter: impl Iterator<Item = (Repr<B>, Context<R>)>,
) -> FBig<R, B> {
    let (mut acc, mut context) = match iter.next() {
        Some((repr, ctx)) => {
            assert_finite(&repr);
            (repr, ctx)
        }
        None => return FBig::ZERO, // empty iterator → additive identity
    };
    for (repr, ctx) in iter {
        assert_finite(&repr);
        acc = acc + &repr;
        context = Context::max(context, ctx);
    }
    // Exact cancellation can leave `acc` with a zero significand whose exponent coincides with the
    // `-0` sentinel (-1) — `Repr::new` then mislabels it `-0`. Canonicalize the sign per IEEE 754
    // §6.3 (x + (-x) = +0 except under roundTowardNegative), mirroring `Add`'s `cancel_zero`.
    if acc.significand.is_zero() {
        acc = if R::IS_ROUND_TOWARD_NEGATIVE {
            Repr::neg_zero()
        } else {
            Repr::zero()
        };
    }
    FBig::new(context.repr_round(acc).value(), context)
}

impl<R: Round, const B: Word> Sum for FBig<R, B> {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        precise_sum(iter.map(|v| (v.repr, v.context)))
    }
}

impl<'a, R: Round, const B: Word> Sum<&'a FBig<R, B>> for FBig<R, B> {
    fn sum<I: Iterator<Item = &'a FBig<R, B>>>(iter: I) -> Self {
        precise_sum(iter.map(|v| (v.repr.clone(), v.context)))
    }
}

impl<R: Round, const B: Word> Product for FBig<R, B> {
    fn product<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(FBig::ONE, |acc, x| acc * x)
    }
}

impl<'a, R: Round, const B: Word> Product<&'a FBig<R, B>> for FBig<R, B> {
    fn product<I: Iterator<Item = &'a FBig<R, B>>>(iter: I) -> Self {
        iter.fold(FBig::ONE, |acc, x| acc * x)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::round::mode::{HalfAway, HalfEven, Zero};
    use alloc::vec::Vec;
    use core::str::FromStr;
    use dashu_int::IBig;

    type F = FBig<HalfAway, 10>;

    fn r<const B: Word>(sig: i128, exp: isize) -> Repr<B> {
        Repr::new(IBig::from(sig), exp)
    }

    #[test]
    fn sum_empty() {
        let s: F = core::iter::empty::<F>().sum();
        assert_eq!(s, F::ZERO);
        assert!(s.repr().is_pos_zero());
    }

    #[test]
    fn sum_single() {
        let a = F::from_str("1.234").unwrap();
        let s: F = core::iter::once(a.clone()).sum();
        assert_eq!(s, a);
    }

    #[test]
    fn sum_cancellation_is_positive_zero() {
        // 0.5 + (-0.5) must yield +0 under round-to-nearest (IEEE 754 §6.3), matching chained `+` —
        // the exact-cancellation exponent (-1) must not be mistaken for the `-0` sentinel.
        let half = F::from_str("0.5").unwrap();
        let s: F = [half.clone(), -half].into_iter().sum();
        assert!(s.repr().is_pos_zero());
        assert!(!s.repr().is_neg_zero());
    }

    #[test]
    fn sum_ref_iter() {
        let a = F::from_str("1.23").unwrap();
        let b = F::from_str("4.56").unwrap();
        let vals = [a, b];
        let s: F = vals.iter().sum();
        assert_eq!(s, F::from_str("5.79").unwrap());
    }

    #[test]
    fn sum_exact_cancellation() {
        let a = F::from_str("1.00").unwrap();
        let b = F::from_str("-1.00").unwrap();
        let s: F = [a, b].into_iter().sum();
        assert!(s.repr().is_pos_zero()); // exact zero → +0
    }

    #[test]
    fn sum_uses_max_precision() {
        let a = F::from_str("1.234").unwrap(); // precision 4
        let b = F::from_str("5.6").unwrap(); // precision 2
        let s: F = [a, b].into_iter().sum();
        assert_eq!(s, F::from_str("6.834").unwrap());
        assert_eq!(s.precision(), 4); // max of the two operand precisions
    }

    // The headline regression: per-step truncation loses almost all the mass, the precise sum does
    // not. 11 copies of 0.9 at precision 1, truncation: exact sum 9.9 → 9, naive fold → 1.
    #[test]
    fn sum_precise_beats_naive_truncation() {
        type Z = FBig<Zero, 10>;
        let vals: Vec<Z> = (0..11)
            .map(|_| Z::from_parts(9.into(), -1)) // 0.9, precision 1
            .collect();

        let precise: Z = vals.iter().cloned().sum();
        assert_eq!(precise, Z::from_parts(9.into(), 0)); // 9.9 truncated to 1 digit = 9

        let naive: Z = vals.iter().cloned().fold(Z::ZERO, |acc, v| acc + v);
        assert_eq!(naive, Z::from_parts(1.into(), 0)); // 0.9+0.9=1.8→1, then stuck at 1
        assert_ne!(precise, naive);
    }

    // Cross-check against an independent oracle: an exact fold at unlimited precision (where `+` is
    // exact) rounded once must equal the precise sum.
    #[test]
    fn sum_matches_unlimited_oracle() {
        fn check<const B: Word>(strs: &[&str]) {
            let vals: Vec<FBig<HalfEven, B>> = strs
                .iter()
                .map(|s| FBig::<HalfEven, B>::from_str(s).unwrap())
                .collect();
            let target = vals.iter().map(|v| v.precision()).max().unwrap_or(0);

            let precise: FBig<HalfEven, B> = vals.clone().into_iter().sum();

            let exact: FBig<HalfEven, B> = vals
                .iter()
                .map(|v| v.clone().with_precision(0).value())
                .fold(FBig::<HalfEven, B>::ZERO, |acc, v| acc + v);
            let oracle = exact.with_precision(target).value();

            assert_eq!(precise.precision(), oracle.precision());
            assert_eq!(precise, oracle);
        }
        check::<10>(&["1.234", "5.6", "0.001"]);
        check::<10>(&["9.99", "0.009", "0.0009"]);
        check::<2>(&["1.1", "0.01", "0.001", "0.0001"]); // 1.5, 0.25, 0.125, 0.0625
    }

    #[test]
    fn product_owned_and_ref() {
        let a = F::from_str("1.2").unwrap();
        let b = F::from_str("3.0").unwrap();
        let expected = &a * &b;
        let owned: F = [a.clone(), b.clone()].into_iter().product();
        let by_ref: F = [a, b].iter().product();
        assert_eq!(owned, expected);
        assert_eq!(by_ref, expected);
    }

    #[test]
    #[should_panic(expected = "arithmetic operations with the infinity are not allowed")]
    fn sum_infinite_panics() {
        let _: F = core::iter::once(F::INFINITY).sum();
    }

    // Exercises the zero short-circuit in `impl Add for &Repr`: a `-0` sentinel exponent must not
    // corrupt the accumulator exponent.
    #[test]
    fn repr_add_neg_zero_is_identity() {
        let nz = Repr::<10>::neg_zero();
        let x = r::<10>(5, -2); // 0.05
                                // -0 + x == x, x + -0 == x (exact, pre-rounding)
        assert_eq!(&nz + &x, x);
        assert_eq!(&x + &nz, x);
        assert_eq!(&nz + &nz, Repr::<10>::neg_zero());
        // 1 + -1 cancels to +0 (not -0)
        assert_eq!(&r::<10>(1, 0) + &r::<10>(-1, 0), Repr::<10>::zero());
    }
}
