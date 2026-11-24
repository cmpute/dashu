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
