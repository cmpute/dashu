//! Implementation of core::iter traits

use crate::{
    error::assert_finite,
    fbig::FBig,
    repr::{Context, Repr, Word},
    round::Round,
    utils::{digit_len, shl_digits},
};
use alloc::vec::Vec;
use core::iter::{Product, Sum};
use dashu_base::Sign;
use dashu_int::{IBig, UBig};

/// An upper bound on the position just above the top digit of a nonzero `repr`:
/// `|repr| < B^top_bound(repr)`.
fn top_bound<const B: Word>(repr: &Repr<B>) -> i128 {
    repr.exponent as i128 + repr.digits_ub() as i128
}

/// An upper bound on the magnitude of the sum of `addends` (nonzero, sorted by decreasing
/// [`top_bound`]), as an exponent: `|Σ addends| < B^tail_bound(addends)`.
fn tail_bound<const B: Word>(addends: &[Repr<B>]) -> i128 {
    // each of the n addends is below B^top, and n < 2^bits(n) <= B^bits(n)
    let bits = usize::BITS - addends.len().leading_zeros();
    top_bound(&addends[0]) + bits as i128
}

/// The sign of the exact sum of `addends` (nonzero, sorted by decreasing [`top_bound`]), or
/// `None` when they cancel exactly. The addends are accumulated exactly from the largest
/// down, but only until the rest can no longer change the sign.
fn tail_sign<const B: Word>(addends: &[Repr<B>]) -> Option<Sign> {
    let mut acc = Repr::<B>::zero();
    for (i, addend) in addends.iter().enumerate() {
        if !acc.significand.is_zero() {
            // |acc| >= B^(exponent + digits − 1) > |rest|
            let floor = acc.exponent as i128 + acc.digits() as i128 - 1;
            if tail_bound(&addends[i..]) <= floor {
                break;
            }
        }
        acc = &acc + addend;
    }
    if acc.significand.is_zero() {
        None
    } else {
        Some(acc.sign())
    }
}

/// Correctly-rounded summation of finite floats.
///
/// The result is the mathematically exact sum rounded a single time to the target context
/// (MPFR `mpfr_sum` semantics): there is no per-step rounding. The target context is
/// `Context::max` over all addend contexts (matching chained `+`). A sum beyond the exponent
/// range saturates like `+` does (see [`Context::unwrap_fp`]); the range is judged on the
/// rounded sum, so partial sums may leave it.
///
/// The addends are accumulated exactly at the [`Repr`] level from the largest magnitude down.
/// Once the partial sum is settled far enough above everything that is left — the rest lies
/// entirely below its sticky threshold, see [`Context::sticky_threshold`] — the rest can only
/// influence the rounding through its sign, so it is never aligned with the partial sum: the
/// cost of a sum does not grow with the exponent gaps between its addends (only an
/// unlimited-precision sum has to materialize the whole span, as its result does).
fn precise_sum<R: Round, const B: Word>(
    iter: impl Iterator<Item = (Repr<B>, Context<R>)>,
) -> FBig<R, B> {
    let mut context: Option<Context<R>> = None;
    let mut addends: Vec<Repr<B>> = Vec::new();
    let (mut all_pos_zero, mut all_neg_zero) = (true, true);
    for (repr, ctx) in iter {
        assert_finite(&repr);
        context = Some(match context {
            Some(context) => Context::max(context, ctx),
            None => ctx,
        });
        all_pos_zero &= repr.is_pos_zero();
        all_neg_zero &= repr.is_neg_zero();
        if !repr.significand.is_zero() {
            addends.push(repr);
        }
    }
    let context = match context {
        Some(context) => context,
        None => return FBig::ZERO, // empty iterator → additive identity
    };

    addends.sort_unstable_by_key(|repr| core::cmp::Reverse(top_bound(repr)));

    // The partial sum is `signif · B^exponent`, with the exponent kept wide: a partial sum
    // can leave the exponent range although the whole sum does not (addends at the largest
    // exponent that cancel later), and the range is judged on the rounded result only.
    let (mut signif, mut exponent) = (IBig::ZERO, 0i128);
    for (i, addend) in addends.iter().enumerate() {
        if !signif.is_zero() && context.is_limited() {
            // the sticky threshold of the partial sum, see `Context::sticky_threshold`
            let top = exponent + digit_len::<B>(&signif) as i128;
            let threshold = exponent.min(top - context.precision as i128 - 2) - 1;
            if tail_bound(&addends[i..]) <= threshold {
                // The rest is either a nonzero amount below the sticky threshold, which
                // rounds like one unit of its sign at the threshold, or it cancels exactly
                // and the partial sum is the exact sum.
                if let Some(sign) = tail_sign(&addends[i..]) {
                    signif = shl_digits::<B>(&signif, (exponent - threshold) as usize)
                        + IBig::from_parts(sign, UBig::ONE);
                    exponent = threshold;
                }
                break;
            }
        }

        let addend_exponent = addend.exponent as i128;
        if signif.is_zero() {
            signif = addend.significand.clone();
            exponent = addend_exponent;
        } else if addend_exponent >= exponent {
            let shift = (addend_exponent - exponent) as usize;
            signif += shl_digits::<B>(&addend.significand, shift);
        } else {
            let shift = (exponent - addend_exponent) as usize;
            signif = shl_digits::<B>(&signif, shift) + &addend.significand;
            exponent = addend_exponent;
        }
        // keep the partial sum normalized (and a cancelled one at `+0`)
        let normalized = Repr::<B>::new(signif, 0);
        exponent += normalized.exponent as i128;
        signif = normalized.significand;
    }

    // The sign of an exact zero sum follows IEEE 754 §6.3 folded left, as the chained `+`
    // does: a sum of like-signed zeros keeps their sign, while mixed-sign zeros and an exact
    // cancellation of nonzero addends give `-0` under roundTowardNegative and `+0` otherwise.
    if signif.is_zero() {
        let negative = all_neg_zero || (R::IS_ROUND_TOWARD_NEGATIVE && !all_pos_zero);
        return FBig::new(Repr::zero_with_sign(Sign::from(negative)), context);
    }
    // A sum beyond the exponent range saturates like the chained `+` does.
    let rounded = context.repr_round_wide(signif, exponent);
    context.unwrap_fp(rounded.map(|r| r.map(|v| FBig::new(v, context))))
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

    // The sign of a zero sum follows IEEE 754 §6.3 folded left: like-signed zeros keep their
    // sign, a single addend is itself, and mixed-sign zeros cancel to +0 (nearest).
    #[test]
    fn sum_keeps_sign_of_all_neg_zero_addends() {
        let nz = -F::ZERO; // -0
        assert!(nz.repr().is_neg_zero());
        let s: F = core::iter::once(nz.clone()).sum();
        assert!(s.repr().is_neg_zero(), "sum of one -0 is -0");
        let s: F = [nz.clone(), nz.clone()].into_iter().sum();
        assert!(s.repr().is_neg_zero(), "sum of two -0 is -0");
        let s: F = [nz, F::ZERO].into_iter().sum();
        assert!(s.repr().is_pos_zero(), "-0 + (+0) is +0 (nearest)");
    }

    #[test]
    fn sum_zero_is_neg_zero_under_down() {
        use crate::round::mode::Down;
        type FD = FBig<Down, 10>;
        // exact cancellation of nonzero addends → -0 under roundTowardNegative
        let one = FD::from_str("1.0").unwrap();
        let s: FD = [one.clone(), -one].into_iter().sum();
        assert!(s.repr().is_neg_zero());
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
