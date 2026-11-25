use dashu_base::Approximation::*;
use dashu_float::{
    round::{mode::*, Rounding::*},
    DBig,
};
use dashu_ratio::RBig;
use std::str::FromStr;

mod helper_macros;

type FBin = dashu_float::FBig;

#[test]
fn test_to_float_decimal() {
    assert_eq!(rbig!(0).to_float(1), Exact(DBig::ZERO));
    assert_eq!(rbig!(0).to_float::<HalfEven, 10>(20).unwrap().precision(), 20);
    assert_eq!(rbig!(1).to_float(1), Exact(DBig::ONE));
    assert_eq!(rbig!(-1).to_float(1), Exact(DBig::NEG_ONE));
    assert_eq!(rbig!(1 / 2).to_float(1), Exact(DBig::from_str("0.5").unwrap()));
    assert_eq!(rbig!(2 / 5).to_float(1), Exact(DBig::from_str("0.4").unwrap()));
    assert_eq!(rbig!(9 / 100).to_float(1), Exact(DBig::from_str("0.09").unwrap()));
    assert_eq!(rbig!(21 / 33).to_float(4), Inexact(DBig::from_str("0.6364").unwrap(), AddOne));
    assert_eq!(
        rbig!(2 / 33333333).to_float(4),
        Inexact(DBig::from_str("6.000e-8").unwrap(), NoOp)
    );
    assert_eq!(
        rbig!(22222222 / 3).to_float(4),
        Inexact(DBig::from_str("7.407e6").unwrap(), NoOp)
    );
}

#[test]
fn test_to_float_binary() {
    assert_eq!(rbig!(0).to_float(1), Exact(FBin::ZERO));
    assert_eq!(rbig!(0).to_float::<Zero, 2>(20).unwrap().precision(), 20);
    assert_eq!(rbig!(1).to_float(1), Exact(FBin::ONE));
    assert_eq!(rbig!(-1).to_float(1), Exact(FBin::NEG_ONE));
    assert_eq!(rbig!(1 / 2).to_float(1), Exact(FBin::from_str("0x1p-1").unwrap()));
    assert_eq!(rbig!(2 / 5).to_float(1), Inexact(FBin::from_str("0x1p-2").unwrap(), NoOp));
    assert_eq!(rbig!(9 / 100).to_float(4), Inexact(FBin::from_str("0xbp-7").unwrap(), NoOp));
}

#[test]
fn test_from_float_binary() {
    assert_eq!(RBig::simplest_from_float(&FBin::ZERO), Some(rbig!(0)));
    assert_eq!(RBig::simplest_from_float(&FBin::ONE), Some(rbig!(1)));
    assert_eq!(RBig::simplest_from_float(&FBin::NEG_ONE), Some(rbig!(-1)));
    assert_eq!(RBig::simplest_from_float(&FBin::INFINITY), None);
    assert_eq!(RBig::simplest_from_float(&FBin::NEG_INFINITY), None);

    let f = FBin::from(3) / FBin::from(7);
    assert_eq!(RBig::simplest_from_float(&f), Some(rbig!(2 / 5)));
    let f = FBin::from(3) / FBin::from(7).with_precision(4).unwrap();
    assert_eq!(RBig::simplest_from_float(&f), Some(rbig!(3 / 7)));
    let f = FBin::from(-3) / FBin::from(7).with_precision(16).unwrap();
    assert_eq!(RBig::simplest_from_float(&f), Some(rbig!(-3 / 7)));
}

#[test]
fn test_from_float_decimal() {
    assert_eq!(RBig::simplest_from_float(&DBig::ZERO), Some(rbig!(0)));
    assert_eq!(RBig::simplest_from_float(&DBig::ONE), Some(rbig!(1)));
    assert_eq!(RBig::simplest_from_float(&DBig::NEG_ONE), Some(rbig!(-1)));
    assert_eq!(RBig::simplest_from_float(&DBig::INFINITY), None);
    assert_eq!(RBig::simplest_from_float(&DBig::NEG_INFINITY), None);

    let f = DBig::from(3) / DBig::from(7);
    assert_eq!(RBig::simplest_from_float(&f), Some(rbig!(2 / 5)));
    let f = DBig::from(3) / DBig::from(7).with_precision(4).unwrap();
    assert_eq!(RBig::simplest_from_float(&f), Some(rbig!(3 / 7)));
    let f = DBig::from(-3) / DBig::from(7).with_precision(10).unwrap();
    assert_eq!(RBig::simplest_from_float(&f), Some(rbig!(-3 / 7)));
}

#[test]
fn test_to_float_rounds_once() {
    use dashu_float::FBig;
    use dashu_ratio::Relaxed;

    // 2.49 is nearer to 2 than to 3 (rounding 2.49 -> 2.5 -> 3 rounds twice)
    assert_eq!(
        rbig!(249 / 100).to_float::<HalfAway, 10>(1),
        Inexact(FBig::<HalfAway, 10>::from(2), NoOp)
    );
    assert_eq!(
        rbig!(249 / 100).to_float::<HalfEven, 10>(1),
        Inexact(FBig::<HalfEven, 10>::from(2), NoOp)
    );
    assert_eq!(
        rbig!(12 / 5).to_float::<HalfAway, 10>(1),
        Inexact(FBig::<HalfAway, 10>::from(2), NoOp)
    );
    // 2.51 is nearer to 3 than to 2
    assert_eq!(
        rbig!(251 / 100).to_float::<HalfEven, 10>(1),
        Inexact(FBig::<HalfEven, 10>::from(3), AddOne)
    );
    assert_eq!(
        rbig!(-251 / 100).to_float::<HalfEven, 10>(1),
        Inexact(FBig::<HalfEven, 10>::from(-3), SubOne)
    );
    // real ties
    assert_eq!(
        rbig!(5 / 2).to_float::<HalfEven, 10>(1),
        Inexact(FBig::<HalfEven, 10>::from(2), NoOp)
    );
    assert_eq!(
        rbig!(5 / 2).to_float::<HalfAway, 10>(1),
        Inexact(FBig::<HalfAway, 10>::from(3), AddOne)
    );
    assert_eq!(
        rbig!(7 / 2).to_float::<HalfEven, 10>(1),
        Inexact(FBig::<HalfEven, 10>::from(4), AddOne)
    );
    // 2.5 lies between the one-bit numbers 2 and 4, nearer to 2
    assert_eq!(
        rbig!(5 / 2).to_float::<HalfAway, 2>(1),
        Inexact(FBig::<HalfAway, 2>::from(2), NoOp)
    );
    assert_eq!(
        Relaxed::from_parts(15.into(), 6u8.into()).to_float::<HalfAway, 2>(1),
        Inexact(FBig::<HalfAway, 2>::from(2), NoOp)
    );
    // 14.4 and 14.6 are both nearer to 10 than to 20
    for r in [rbig!(72 / 5), rbig!(73 / 5)] {
        assert_eq!(r.to_float::<HalfEven, 10>(1).value(), FBig::<HalfEven, 10>::from(10));
        assert_eq!(r.to_float::<HalfAway, 10>(1).value(), FBig::<HalfAway, 10>::from(10));
    }
    // 449.352941176470..., 1.2349
    let r = rbig!(7639 / 17);
    assert_eq!(
        r.to_float::<HalfAway, 10>(11).value(),
        FBig::<HalfAway, 10>::from_parts(44935294118i64.into(), -8)
    );
    assert_eq!(
        r.to_float::<HalfAway, 10>(12).value(),
        FBig::<HalfAway, 10>::from_parts(449352941176i64.into(), -9)
    );
    assert_eq!(
        rbig!(12349 / 10000).to_float::<HalfEven, 10>(3).value(),
        DBig::from_str("1.23").unwrap()
    );
    assert_eq!(
        rbig!(12349 / 10000).to_float::<HalfAway, 10>(3).value(),
        DBig::from_str("1.23").unwrap()
    );
    // a carry out of the last digit: 9.96 -> 10
    assert_eq!(rbig!(996 / 100).to_float(2), Inexact(DBig::from_str("10").unwrap(), AddOne));
    assert_eq!(
        rbig!(996 / 100)
            .to_float::<HalfAway, 10>(2)
            .value()
            .precision(),
        2
    );
}

/// Check that `f` is `x` correctly rounded to `precision` digits in base `B` under the
/// rounding mode `R`, using exact rational arithmetic.
fn assert_correctly_rounded<R: dashu_float::round::Round, const B: dashu_int::Word>(
    x: &RBig,
    precision: usize,
) {
    use core::any::type_name;
    use core::cmp::Ordering;

    let f = x.to_float::<R, B>(precision).value();
    assert_eq!(f.precision(), precision);
    let got = RBig::try_from(f).unwrap();
    let context = || format!("{x} at {precision} digits in base {B} ({})", type_name::<R>());

    // the unit in the last place of the numbers with the magnitude of x: B^(e - precision + 1)
    // where B^e <= |x| < B^(e + 1)
    let base = RBig::from(B);
    let abs = if *x < RBig::ZERO {
        -x.clone()
    } else {
        x.clone()
    };
    let mut e = 0isize;
    while abs < base.pow(e) {
        e -= 1;
    }
    while abs >= base.pow(e + 1) {
        e += 1;
    }
    let ulp = base.pow(e - precision as isize + 1);

    // the result is on the grid of the representable numbers...
    assert!((&got / &ulp).denominator().is_one(), "{}: {got} is off the grid", context());
    // ... and it is the right neighbour of x on that grid
    let diff = &got - x;
    let abs_diff = if diff < RBig::ZERO {
        -diff.clone()
    } else {
        diff.clone()
    };
    assert!(abs_diff < ulp, "{}: {got}", context());
    let mode = type_name::<R>();
    let positive = *x > RBig::ZERO;
    let expected_direction = match mode.rsplit("::").next().unwrap() {
        "Zero" => Some(!positive),
        "Away" => Some(positive),
        "Down" => Some(false),
        "Up" => Some(true),
        _ => None,
    };
    match expected_direction {
        // directed rounding: the error has a fixed sign (true = rounded up)
        Some(up) => {
            if up {
                assert!(diff >= RBig::ZERO, "{}: {got}", context());
            } else {
                assert!(diff <= RBig::ZERO, "{}: {got}", context());
            }
        }
        None => {
            let half = &ulp / RBig::from(2);
            match abs_diff.cmp(&half) {
                Ordering::Less => {}
                Ordering::Greater => panic!("{}: {got} is not the nearest", context()),
                Ordering::Equal => {
                    let away = (got > *x) == positive;
                    if mode.ends_with("HalfAway") {
                        assert!(away, "{}: {got}", context());
                    } else {
                        // ties to even: the last digit of the result is even
                        let last_place = (&got / &ulp).numerator().clone();
                        assert!(
                            (last_place % dashu_int::IBig::from(2)).is_zero(),
                            "{}: {got}",
                            context()
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn test_to_float_correctly_rounded() {
    let mut values = Vec::new();
    for den in [
        1u64, 2, 3, 5, 7, 10, 17, 100, 127, 1000, 4096, 9999, 65537, 1000003,
    ] {
        for num in [
            1u64, 2, 3, 5, 9, 10, 14, 15, 25, 99, 149, 150, 151, 249, 251, 995, 996, 1005, 4095,
            12349, 98765, 999999, 123456789,
        ] {
            values.push(RBig::from_parts(num.into(), den.into()));
            values.push(RBig::from_parts((-(num as i64)).into(), den.into()));
        }
    }
    // values just around rounding boundaries of many digits
    let big = RBig::from_parts(1.into(), dashu_int::UBig::from(10u8).pow(600));
    values.push(rbig!(5 / 2) + &big);
    values.push(rbig!(5 / 2) - &big);
    values.push(rbig!(1) - &big);
    values.push(rbig!(-1) + &big);
    values.push(rbig!(1 / 3) * (rbig!(1) + &big));

    for x in &values {
        // 20, 50, 100 and 500 bits cover the significand widths of the binary float,
        // the small precisions exercise the boundaries
        for precision in [1usize, 2, 3, 7, 20, 50, 100, 500] {
            assert_correctly_rounded::<Zero, 2>(x, precision);
            assert_correctly_rounded::<Away, 2>(x, precision);
            assert_correctly_rounded::<Down, 2>(x, precision);
            assert_correctly_rounded::<Up, 2>(x, precision);
            assert_correctly_rounded::<HalfEven, 2>(x, precision);
            assert_correctly_rounded::<HalfAway, 2>(x, precision);
        }
        for precision in [1usize, 2, 3, 4, 11, 12, 40, 160] {
            assert_correctly_rounded::<Zero, 10>(x, precision);
            assert_correctly_rounded::<Away, 10>(x, precision);
            assert_correctly_rounded::<Down, 10>(x, precision);
            assert_correctly_rounded::<Up, 10>(x, precision);
            assert_correctly_rounded::<HalfEven, 10>(x, precision);
            assert_correctly_rounded::<HalfAway, 10>(x, precision);
        }
        for precision in [1usize, 2, 5] {
            assert_correctly_rounded::<HalfEven, 3>(x, precision);
            assert_correctly_rounded::<HalfAway, 7>(x, precision);
            assert_correctly_rounded::<Up, 16>(x, precision);
        }
    }
}

#[test]
fn test_simplest_from_float_power_of_base() {
    use dashu_float::FBig;

    // Below a power of the base the floats are denser, so the rounding interval is
    // narrower on that side: [9.5, 15) for 10 at one digit.
    let f = DBig::from(10).with_precision(1).value();
    assert_eq!(RBig::simplest_from_float(&f), Some(rbig!(10)));
    let f = DBig::from(-10).with_precision(1).value();
    assert_eq!(RBig::simplest_from_float(&f), Some(rbig!(-10)));
    let f = DBig::from(20).with_precision(1).value();
    assert_eq!(RBig::simplest_from_float(&f), Some(rbig!(15)));
    // [99995000, 100050000)
    let f = DBig::from(100_000_000).with_precision(4).value();
    assert_eq!(RBig::simplest_from_float(&f), Some(rbig!(99995000)));
    // (0.9, 1] rounds up to 1 at one digit
    let f = FBig::<Up, 10>::from(1).with_precision(1).value();
    assert_eq!(RBig::simplest_from_float(&f), Some(rbig!(1)));
    // [0.1, 0.2) rounds down to 0.1 at one digit, [0.1, 0.11) at two digits
    let f = FBig::<Down, 10>::from_parts(1.into(), -1)
        .with_precision(1)
        .value();
    assert_eq!(RBig::simplest_from_float(&f), Some(rbig!(1 / 6)));
    let f = FBig::<Down, 10>::from_parts(1.into(), -1)
        .with_precision(2)
        .value();
    assert_eq!(RBig::simplest_from_float(&f), Some(rbig!(1 / 10)));
    // (0.09, 0.1] is rounded away from zero to 0.1, and so is [-0.1, -0.09) to -0.1
    let f = FBig::<Away, 10>::from_parts((-1).into(), -1)
        .with_precision(1)
        .value();
    assert_eq!(RBig::simplest_from_float(&f), Some(rbig!(-1 / 10)));
    let f = FBig::<Away, 10>::from_parts((-1).into(), -1)
        .with_precision(2)
        .value();
    assert_eq!(RBig::simplest_from_float(&f), Some(rbig!(-1 / 10)));
    // 1024 = 10_2 * 2^9: [896, 1280] with both ends included (even significand)
    let f = FBig::<HalfEven, 2>::from(1024).with_precision(2).value();
    assert_eq!(RBig::simplest_from_float(&f), Some(rbig!(896)));
    // 1536 = 11_2 * 2^9: (1280, 1792) with both ends excluded (odd significand)
    let f = FBig::<HalfEven, 2>::from(1536).with_precision(2).value();
    assert_eq!(RBig::simplest_from_float(&f), Some(rbig!(1281)));
}

#[test]
fn test_simplest_from_float_included_endpoint() {
    use dashu_float::FBig;

    // [1, 2) is truncated to 1 at one digit, and 1 is its simplest element
    let f = FBig::<Zero, 10>::from(1).with_precision(1).value();
    assert_eq!(RBig::simplest_from_float(&f), Some(rbig!(1)));
    let f = FBig::<Zero, 10>::from(-1).with_precision(1).value();
    assert_eq!(RBig::simplest_from_float(&f), Some(rbig!(-1)));
    let f = FBig::<Down, 10>::from(-2).with_precision(1).value();
    assert_eq!(RBig::simplest_from_float(&f), Some(rbig!(-2)));
    // the binary float converted from 1.0f64 (53 bits, truncating)
    let f = FBin::try_from(1.0f64).unwrap();
    assert_eq!(f.precision(), 53);
    assert_eq!(RBig::simplest_from_float(&f), Some(rbig!(1)));
    let f = FBin::try_from(-0.75f32).unwrap();
    assert_eq!(RBig::simplest_from_float(&f), Some(rbig!(-3 / 4)));

    // numbers with unlimited precision are exact, under every rounding mode
    assert_eq!(RBig::simplest_from_float(&FBig::<Up, 2>::ONE), Some(rbig!(1)));
    assert_eq!(RBig::simplest_from_float(&FBig::<Down, 10>::NEG_ONE), Some(rbig!(-1)));
    let f = FBig::<HalfEven, 10>::from_parts(314159.into(), -5)
        .with_precision(0)
        .value();
    assert_eq!(RBig::simplest_from_float(&f), Some(rbig!(314159 / 100000)));
    // both zeros
    let zero = DBig::ZERO.with_precision(5).value();
    assert_eq!(RBig::simplest_from_float(&zero), Some(rbig!(0)));
    assert_eq!(RBig::simplest_from_float(&-zero), Some(rbig!(0)));
}

/// Check `simplest_from_float` on the float nearest to `x`: the result rounds back to the
/// float, and (with `exhaustive`) no simpler number does.
fn check_simplest_from_float<R: dashu_float::round::ErrorBounds, const B: dashu_int::Word>(
    x: &RBig,
    precision: usize,
    exhaustive: bool,
) {
    let f = x.to_float::<R, B>(precision).value();
    let exact = RBig::try_from(f.clone()).unwrap();
    let r = RBig::simplest_from_float(&f).unwrap();
    let context = || {
        format!(
            "{exact} at {precision} digits in base {B} ({}) -> {r}",
            core::any::type_name::<R>()
        )
    };

    // the result is inside the rounding interval of the float
    assert_eq!(r.to_float::<R, B>(precision).value(), f, "{}", context());
    // the float itself is in the interval, so it can't be simpler than the result
    assert!(!exact.is_simpler_than(&r), "{}", context());
    assert_eq!(r > rbig!(0), exact > rbig!(0), "{}", context());

    let small_denominator = r.denominator() <= &dashu_int::UBig::from(2000u16);
    if exhaustive && small_denominator {
        // every number that is simpler than the result is rounded to something else
        let rounds_to_f = |n: &dashu_int::IBig, d: u32| {
            let candidate = RBig::from_parts(n.clone(), d.into());
            candidate.to_float::<R, B>(precision).value() == f
        };
        let den: u32 = r.denominator().try_into().unwrap();
        for d in 1..den {
            // the numbers n/d around the float
            let n = (&exact * RBig::from(d)).floor();
            for delta in -1..=2 {
                let n = &n + dashu_int::IBig::from(delta);
                assert!(!rounds_to_f(&n, d), "{}: {n}/{d} is simpler", context());
            }
        }
        // same denominator: a numerator of a smaller magnitude
        let toward_zero = if r > rbig!(0) { -1 } else { 1 };
        let n = r.numerator() + dashu_int::IBig::from(toward_zero);
        assert!(!rounds_to_f(&n, den), "{}: {n}/{den} is simpler", context());
    }
}

#[test]
fn test_simplest_from_float_roundtrip() {
    let mut values = Vec::new();
    for den in [1u64, 3, 7, 10, 16, 1024] {
        for num in [1u64, 2, 3, 5, 9, 10, 16, 27, 99, 101, 128, 1000] {
            values.push(RBig::from_parts(num.into(), den.into()));
            values.push(RBig::from_parts((-(num as i64)).into(), den.into()));
        }
    }
    for x in &values {
        for precision in [1usize, 2, 3] {
            check_simplest_from_float::<Zero, 2>(x, precision, true);
            check_simplest_from_float::<Away, 2>(x, precision, true);
            check_simplest_from_float::<Down, 2>(x, precision, true);
            check_simplest_from_float::<Up, 2>(x, precision, true);
            check_simplest_from_float::<HalfEven, 2>(x, precision, true);
            check_simplest_from_float::<HalfAway, 2>(x, precision, true);
            check_simplest_from_float::<Zero, 10>(x, precision, true);
            check_simplest_from_float::<Away, 10>(x, precision, true);
            check_simplest_from_float::<Down, 10>(x, precision, true);
            check_simplest_from_float::<Up, 10>(x, precision, true);
            check_simplest_from_float::<HalfEven, 10>(x, precision, true);
            check_simplest_from_float::<HalfAway, 10>(x, precision, true);
            // odd bases: the midpoints are not representable with one more digit
            check_simplest_from_float::<HalfEven, 3>(x, precision, true);
            check_simplest_from_float::<HalfAway, 3>(x, precision, true);
            check_simplest_from_float::<Up, 7>(x, precision, true);
        }
        // the precisions matching f32 / f64 / u128 significands and beyond
        for precision in [20usize, 50, 100, 500] {
            check_simplest_from_float::<Zero, 2>(x, precision, false);
            check_simplest_from_float::<Up, 2>(x, precision, false);
            check_simplest_from_float::<HalfEven, 2>(x, precision, false);
            check_simplest_from_float::<HalfAway, 10>(x, precision, false);
            check_simplest_from_float::<Down, 10>(x, precision, false);
        }
    }
}

// An unlimited-precision float is exact, so the simplest rational that rounds to it is the
// float itself. Under `Down` and `Up` this used to panic while asking for its ulp.
#[test]
fn test_from_float_unlimited_precision() {
    use dashu_float::{round::ErrorBounds, Context, FBig, Repr};

    fn check<R: ErrorBounds, const B: dashu_int::Word>() {
        let exact =
            |n: i32, exp: isize| FBig::<R, B>::from_repr(Repr::new(n.into(), exp), Context::new(0));
        assert_eq!(RBig::simplest_from_float(&exact(1, 0)), Some(rbig!(1)));
        assert_eq!(RBig::simplest_from_float(&exact(-1, 0)), Some(rbig!(-1)));
        assert_eq!(RBig::simplest_from_float(&exact(7, 0)), Some(rbig!(7)));
        assert_eq!(RBig::simplest_from_float(&exact(0, 0)), Some(rbig!(0)));
        let base = RBig::from(B);
        assert_eq!(RBig::simplest_from_float(&exact(-7, 1)), Some(rbig!(-7) * &base));
        assert_eq!(RBig::simplest_from_float(&exact(5, -2)), Some(rbig!(5) / (&base * &base)));
    }
    check::<Zero, 2>();
    check::<Away, 2>();
    check::<Down, 2>();
    check::<Up, 2>();
    check::<HalfEven, 2>();
    check::<HalfAway, 2>();
    check::<Down, 10>();
    check::<Up, 10>();
    check::<Down, 3>();
    check::<Up, 3>();
    assert_eq!(RBig::simplest_from_float(&dashu_float::FBig::<Down, 2>::ONE), Some(rbig!(1)));
    assert_eq!(
        RBig::simplest_from_float(&dashu_float::FBig::<Up, 10>::NEG_ONE),
        Some(rbig!(-1))
    );
}

// The simplest rational in the rounding interval of a float rounds back to that float. In an
// odd base the round-to-nearest interval used to be reported a third wider than it is.
#[test]
fn test_from_float_rounds_back() {
    use dashu_float::{round::ErrorBounds, Context, Repr};

    fn check<R: ErrorBounds, const B: dashu_int::Word>() {
        for precision in [1usize, 2, 3, 20, 50, 100] {
            let context = Context::<R>::new(precision);
            for (num, den) in [(1, 3), (2, 7), (-5, 9), (22, 7), (-355, 113), (1, 1000)] {
                let f = context
                    .div::<B>(&Repr::new(num.into(), 0), &Repr::new(den.into(), 0))
                    .unwrap()
                    .value();
                let simplest = RBig::simplest_from_float(&f).unwrap();
                // round it back with one correctly rounded float division
                let (n, d) = simplest.into_parts();
                let back = context
                    .div::<B>(&Repr::new(n, 0), &Repr::new(d.into(), 0))
                    .unwrap()
                    .value();
                assert_eq!(back, f, "{num}/{den} in base {B} at precision {precision}");
            }
        }
    }
    check::<HalfAway, 3>();
    check::<HalfEven, 3>();
    check::<HalfAway, 7>();
    check::<HalfEven, 2>();
    check::<HalfAway, 10>();
    check::<Zero, 3>();
    check::<Down, 10>();
    check::<Up, 3>();
    check::<Away, 2>();
}
