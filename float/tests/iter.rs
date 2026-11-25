use dashu_float::DBig;
type FBig = dashu_float::FBig;

mod helper_macros;

#[test]
#[rustfmt::skip::macros(fbig)]
fn test_sum() {
    let nums = [
        fbig!(-0x1p2),
        fbig!(-0x1p1),
        fbig!(-0x1),
        fbig!(-0x1p-1),
        fbig!(-0x1p-2),
        fbig!(0),
        fbig!(0x1p-2),
        fbig!(0x1p-1),
        fbig!(0x1),
        fbig!(0x1p1),
        fbig!(0x1p2),
    ];

    assert_eq!(nums[..0].iter().sum::<FBig>(), fbig!(0));
    assert_eq!(nums[..1].iter().sum::<FBig>(), fbig!(-0x1p2));
    assert_eq!(nums[..2].iter().sum::<FBig>(), fbig!(-0x3p1));
    assert_eq!(nums[..4].iter().sum::<FBig>(), fbig!(-0xfp-1));
    // The full set is symmetric around zero, so the correctly-rounded sum is exactly 0. (The previous
    // per-step fold accumulated rounding and reported 0x1p-2 here.)
    assert_eq!(nums.iter().sum::<FBig>(), fbig!(0));
    assert_eq!(nums.into_iter().sum::<FBig>(), fbig!(0));

    let nums = [
        dbig!(-0001e2),
        dbig!(-1e1),
        dbig!(-1),
        dbig!(-1e-1),
        dbig!(-1e-2),
        dbig!(0),
        dbig!(1e-2),
        dbig!(1e-1),
        dbig!(1),
        dbig!(1e1),
        dbig!(1e2),
    ];

    assert_eq!(nums[..0].iter().sum::<DBig>(), dbig!(0));
    assert_eq!(nums[..1].iter().sum::<DBig>(), dbig!(-1e2));
    assert_eq!(nums[..2].iter().sum::<DBig>(), dbig!(-11e1));
    assert_eq!(nums[..4].iter().sum::<DBig>(), dbig!(-1111e-1));
    // Symmetric around zero → the correctly-rounded sum is exactly 0 (the previous fold gave 1e-2).
    assert_eq!(nums.iter().sum::<DBig>(), dbig!(0));
    assert_eq!(nums.into_iter().sum::<DBig>(), dbig!(0));
}

#[test]
#[rustfmt::skip::macros(fbig)]
fn test_prod() {
    let nums = [
        fbig!(-0x1p2),
        fbig!(0x1p1),
        fbig!(-0x1),
        fbig!(0x1p-1),
        fbig!(-0x1p-2),
        fbig!(0),
    ];

    assert_eq!(nums[..0].iter().product::<FBig>(), fbig!(0x1));
    assert_eq!(nums[..1].iter().product::<FBig>(), fbig!(-0x1p2));
    assert_eq!(nums[..2].iter().product::<FBig>(), fbig!(-0x1p3));
    assert_eq!(nums[..4].iter().product::<FBig>(), fbig!(0x1p2));
    assert_eq!(nums.iter().product::<FBig>(), fbig!(0));
    assert_eq!(nums.into_iter().product::<FBig>(), fbig!(0));

    let nums = [
        dbig!(-0001e2),
        dbig!(1e1),
        dbig!(-1),
        dbig!(1e-1),
        dbig!(-1e-2),
        dbig!(0),
    ];

    assert_eq!(nums[..0].iter().product::<DBig>(), dbig!(1));
    assert_eq!(nums[..1].iter().product::<DBig>(), dbig!(-1e2));
    assert_eq!(nums[..2].iter().product::<DBig>(), dbig!(-1e3));
    assert_eq!(nums[..4].iter().product::<DBig>(), dbig!(1e2));
    assert_eq!(nums.iter().product::<DBig>(), dbig!(0));
    assert_eq!(nums.into_iter().product::<DBig>(), dbig!(0));
}

// ---------------------------------------------------------------------------
// Regressions for the Sum reports of the Hegel triage (see triage/float-arith.md).
// ---------------------------------------------------------------------------

mod triage {
    use dashu_float::{
        round::{mode, Round},
        Context, FBig, Repr, Word,
    };
    use dashu_int::IBig;
    use proptest::prelude::*;

    fn fbig<R: Round, const B: Word>(sig: i64, exp: isize, precision: usize) -> FBig<R, B> {
        FBig::from_repr(Repr::new(sig.into(), exp), Context::new(0))
            .with_precision(precision)
            .value()
    }

    /// A sum beyond the exponent range saturates exactly like `+` does (to an infinity, or to
    /// the largest finite value under an inward mode). `Sum` used to return the unchecked
    /// rounded value — a nonzero significand at the sentinel exponent `isize::MAX` where `+`
    /// gives `inf`, and vice versa (F75). The range is judged on the rounded sum: partial
    /// sums may leave it.
    #[test]
    fn test_sum_exponent_overflow_matches_add() {
        const MAX: isize = isize::MAX;

        #[track_caller]
        fn same<R: Round, const B: Word>(got: &FBig<R, B>, want: &FBig<R, B>) {
            assert_eq!(got.repr(), want.repr());
            assert_eq!(got.precision(), want.precision());
        }

        fn check<R: Round, const B: Word>() {
            let base = B as i64;
            for precision in [1usize, 2, 20, 50, 100, 500] {
                for sig in [1, base - 1, base + 1, base * base - 1, (base + 1) / 2] {
                    if Repr::<B>::new(sig.into(), 0).digits() > precision {
                        continue; // not a value of this precision
                    }
                    for sig in [sig, -sig] {
                        let a = fbig::<R, B>(sig, MAX - 1, precision);
                        // two addends: one `+`
                        let sum: FBig<R, B> = [a.clone(), a.clone()].iter().sum();
                        same(&sum, &(&a + &a));
                        let value = sum.repr();
                        // an infinity, the largest finite value, or a value in range
                        let endpoint = (IBig::from(B).pow(precision) - IBig::ONE) * sig.signum();
                        assert!(
                            value.is_infinite()
                                || value.exponent() < MAX
                                || value == &Repr::new(endpoint, MAX),
                            "{sum:?}"
                        );
                        // one exponent lower the sum is in range
                        let b = fbig::<R, B>(sig, MAX - 3, precision);
                        let sum: FBig<R, B> = [b.clone(), b.clone()].into_iter().sum();
                        same(&sum, &(&b + &b));
                        assert!(sum.repr().exponent() < MAX);

                        // partial sums beyond the range that cancel again: the sum is exact
                        let mut terms = vec![a.clone(); 2 * B as usize + 3];
                        terms.extend(vec![-a.clone(); 2 * B as usize + 2]);
                        let sum: FBig<R, B> = terms.iter().sum();
                        same(&sum, &a);
                    }
                }

                // overflow by the rounding of a far smaller addend (the sticky path)
                let ones = IBig::from(B).pow(precision) - IBig::ONE;
                for sign in [1, -1] {
                    let big = Repr::new(ones.clone() * sign, MAX - precision as isize);
                    let big = FBig::<R, B>::from_repr(big, Context::new(precision));
                    let tiny = fbig::<R, B>(sign, -7, precision);
                    let sum: FBig<R, B> = [tiny.clone(), big.clone()].into_iter().sum();
                    same(&sum, &(&big + &tiny));
                }
            }
        }
        check::<mode::HalfAway, 10>();
        check::<mode::HalfEven, 2>();
        check::<mode::Zero, 10>();
        check::<mode::Zero, 2>();
        check::<mode::Up, 3>();
        check::<mode::Down, 10>();
        check::<mode::Away, 2>();

        // the reported cases
        let a = FBig::<mode::HalfAway, 10>::from_parts(5.into(), MAX - 1);
        let sum: FBig<mode::HalfAway, 10> = [a.clone(), a.clone()].iter().sum();
        assert!(sum.repr().is_infinite());
        let a = FBig::<mode::Zero, 10>::from_parts(99.into(), MAX - 1);
        let sum: FBig<mode::Zero, 10> = [a.clone(), a.clone()].iter().sum();
        assert_eq!(sum.repr(), &Repr::new(99.into(), MAX));
        let a = FBig::<mode::HalfAway, 2>::from_parts(1.into(), MAX - 1);
        let sum: FBig<mode::HalfAway, 2> = [a.clone(), a.clone()].iter().sum();
        assert!(sum.repr().is_infinite());
    }

    /// The sign of a zero sum is the left fold of IEEE 754 §6.3, i.e. what chained `+` gives:
    /// like-signed zeros keep their sign in every rounding mode — `+0` addends used to sum to
    /// `-0` under `mode::Down` — and anything else is `-0` under roundTowardNegative, `+0`
    /// otherwise.
    #[test]
    fn test_sum_zero_sign_matches_chained_add() {
        fn check<R: Round, const B: Word>() {
            for precision in [0usize, 20, 50, 100, 500] {
                let pz = fbig::<R, B>(0, 0, precision);
                let nz = -pz.clone();
                let (one, half) = (fbig::<R, B>(1, 0, precision), fbig::<R, B>(1, -1, precision));
                let lists: [Vec<FBig<R, B>>; 12] = [
                    vec![pz.clone()],
                    vec![nz.clone()],
                    vec![pz.clone(), pz.clone()],
                    vec![nz.clone(), nz.clone()],
                    vec![pz.clone(), nz.clone()],
                    vec![nz.clone(), pz.clone()],
                    vec![one.clone(), -one.clone()],
                    vec![half.clone(), -half.clone()],
                    vec![pz.clone(), one.clone(), -one.clone()],
                    vec![one.clone(), -one.clone(), pz.clone()],
                    vec![nz.clone(), half.clone(), -half.clone(), nz.clone()],
                    vec![one.clone(), half.clone(), -one.clone(), -half.clone()],
                ];
                for list in lists {
                    let folded = list.iter().skip(1).fold(list[0].clone(), |acc, x| acc + x);
                    assert!(folded.repr().significand().is_zero());
                    for sum in [
                        list.iter().sum::<FBig<R, B>>(),
                        list.clone().into_iter().sum(),
                    ] {
                        assert!(sum.repr().significand().is_zero());
                        assert_eq!(sum.sign(), folded.sign(), "{list:?}");
                        assert_eq!(sum.precision(), precision);
                    }
                }
            }
            assert!(core::iter::empty::<FBig<R, B>>()
                .sum::<FBig<R, B>>()
                .repr()
                .is_pos_zero());
        }
        check::<mode::Down, 10>();
        check::<mode::Down, 2>();
        check::<mode::HalfEven, 10>();
        check::<mode::HalfAway, 10>();
        check::<mode::Zero, 2>();
        check::<mode::Up, 2>();

        // the reported case
        type FD = FBig<mode::Down, 10>;
        assert!(core::iter::once(FD::ZERO).sum::<FD>().repr().is_pos_zero());
        assert!([FD::ZERO, FD::ZERO].iter().sum::<FD>().repr().is_pos_zero());
    }

    /// A limited-precision sum never aligns addends across their exponent gap: an addend
    /// far below the partial sum only contributes its sign. The exact accumulation used to
    /// take minutes and gigabytes for a gap of 10^9 digits.
    #[test]
    fn test_sum_huge_exponent_gap() {
        fn check<R: Round, const B: Word>() {
            for precision in [20usize, 50, 100, 500] {
                for gap in [1_000isize, 1_000_000, 1_000_000_000, isize::MAX / 4] {
                    for (big_sign, small_sign) in [(1i64, 1i64), (1, -1), (-1, 1), (-1, -1)] {
                        let a = fbig::<R, B>(big_sign * 1234567891, gap, precision);
                        let b = fbig::<R, B>(small_sign * 987654321, 0, precision);
                        let c = fbig::<R, B>(small_sign * 13, -gap, precision);
                        // a + b + c is `a` nudged toward the sign of the small addends:
                        // the same value as a + (one tiny addend of that sign)
                        let tiny = fbig::<R, B>(small_sign, gap - precision as isize - 10, 0);
                        let expected = Context::<R>::new(0)
                            .add(a.repr(), tiny.repr())
                            .unwrap()
                            .value()
                            .with_precision(precision)
                            .value();
                        for list in [
                            vec![a.clone(), b.clone(), c.clone()],
                            vec![c.clone(), b.clone(), a.clone()],
                            vec![b.clone(), a.clone(), c.clone()],
                        ] {
                            let sum: FBig<R, B> = list.iter().sum();
                            assert_eq!(sum, expected);
                            assert_eq!(sum.precision(), precision);
                        }
                        // addends that cancel across the gap leave the small ones exact
                        let sum: FBig<R, B> = [a.clone(), b.clone(), -a.clone()].into_iter().sum();
                        assert_eq!(sum, b);
                        let sum: FBig<R, B> = [a.clone(), c.clone(), -a.clone(), -c.clone()]
                            .into_iter()
                            .sum();
                        assert!(sum.repr().significand().is_zero());
                    }
                }
            }
        }
        check::<mode::HalfAway, 10>();
        check::<mode::HalfEven, 2>();
        check::<mode::Zero, 2>();
        check::<mode::Up, 10>();
        check::<mode::Down, 3>();

        // the reported case: equal to the two-operand sum
        let a = dashu_float::DBig::from_parts(IBig::from(1234567891), 1_000_000_000);
        let b = dashu_float::DBig::from_parts(IBig::from(1234567891), 0);
        assert_eq!(
            [a.clone(), b.clone()]
                .into_iter()
                .sum::<dashu_float::DBig>(),
            &a + &b
        );
    }

    fn check_sum_against_exact_fold<R: Round, const B: Word>(
        terms: &[(i64, isize)],
        precision: usize,
    ) {
        let values: Vec<FBig<R, B>> = terms
            .iter()
            .map(|&(sig, exp)| fbig::<R, B>(sig, exp, precision))
            .collect();
        // exact fold at unlimited precision, rounded once
        let exact = values
            .iter()
            .fold(FBig::<R, B>::ZERO, |acc, v| acc + v.clone().with_precision(0).value());
        let expected = exact.with_precision(precision).value();
        let sum: FBig<R, B> = values.iter().sum();
        assert_eq!(sum.repr(), expected.repr(), "{values:?}");
        assert_eq!(sum.precision(), precision);
        let mut reversed = values;
        reversed.reverse();
        assert_eq!(reversed.into_iter().sum::<FBig<R, B>>().repr(), expected.repr());
    }

    proptest! {
        #![proptest_config(ProptestConfig { cases: 256, ..Default::default() })]

        /// The sum equals the exact (unlimited-precision) fold rounded once, for addends
        /// spread over exponent gaps on both sides of the sticky shortcut, with repeated and
        /// negated terms so that partial sums cancel.
        #[test]
        fn prop_sum_matches_exact_fold(
            base_terms in prop::collection::vec((-99999i64..=99999, -150isize..=150), 1..6),
            negate in prop::collection::vec(any::<bool>(), 6),
            precision in prop_oneof![Just(20usize), Just(50), Just(100), Just(500)],
        ) {
            // append the negation of some terms to provoke exact cancellations
            let mut terms = base_terms.clone();
            for (term, negate) in base_terms.iter().zip(&negate) {
                if *negate {
                    terms.push((-term.0, term.1));
                }
            }
            check_sum_against_exact_fold::<mode::HalfEven, 2>(&terms, precision);
            check_sum_against_exact_fold::<mode::HalfAway, 10>(&terms, precision);
            check_sum_against_exact_fold::<mode::Zero, 10>(&terms, precision);
            check_sum_against_exact_fold::<mode::Down, 2>(&terms, precision);
            check_sum_against_exact_fold::<mode::Up, 3>(&terms, precision);
            check_sum_against_exact_fold::<mode::Away, 10>(&terms, precision);
        }
    }
}
