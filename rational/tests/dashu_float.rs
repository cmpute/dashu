use dashu_base::Approximation::*;
use dashu_float::{
    round::{mode::*, Rounding::*},
    DBig, FBig,
};
use dashu_ratio::{RBig, Relaxed};
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
fn test_to_float_rounds_once() {
    // The quotient must not be rounded to an integer before the last digit is decided: the two
    // roundings can compound into an error of one unit in the last digit
    // (https://github.com/cmpute/dashu/issues/125).

    // 6/11 = 0.5454…: the 17-digit quotient ends in …45 with a remainder left
    assert_eq!(
        rbig!(6 / 11).to_float::<HalfEven, 10>(16),
        Inexact(FBig::<HalfEven, 10>::from_str("0.5454545454545455").unwrap(), AddOne)
    );
    assert_eq!(
        rbig!(-6 / 11).to_float::<HalfEven, 10>(16),
        Inexact(FBig::<HalfEven, 10>::from_str("-0.5454545454545455").unwrap(), SubOne)
    );

    // 123449.9 at 4 digits lies just below the tie 1234|50 (the default decimal mode)
    assert_eq!(
        rbig!(1234499 / 10).to_float(4),
        Inexact(DBig::from_str("1.234e5").unwrap(), NoOp)
    );

    // 123450.25 at 4 digits lies just above the same tie
    assert_eq!(
        rbig!(493801 / 4).to_float::<HalfEven, 10>(4),
        Inexact(FBig::<HalfEven, 10>::from_str("1.235e5").unwrap(), AddOne)
    );

    // 1010.0101…b at 2 bits lies just above the midpoint between 1000b and 1100b
    assert_eq!(
        rbig!(31 / 3).to_float::<HalfEven, 2>(2),
        Inexact(FBig::<HalfEven, 2>::from_str("0x1.8p3").unwrap(), AddOne)
    );
}

#[test]
fn test_to_float_precision_and_relaxed() {
    // the result carries the requested precision, however many digits the operands have
    let values = [
        rbig!(6 / 11),
        rbig!(493801 / 4),
        rbig!(12345678901234567890123456789 / 7),
        rbig!(-1 / 12345678901234567890123456789),
    ];
    for value in &values {
        for precision in [1usize, 2, 7, 8, 9, 16, 17, 100] {
            let f = value.to_float::<HalfEven, 10>(precision).value();
            assert_eq!(f.precision(), precision, "{value:?} at {precision}");
            assert!(f.digits() <= precision, "{value:?} at {precision}");
        }
    }

    // a value stored unreduced converts like the reduced one
    let reduced = rbig!(6 / 11);
    let relaxed = Relaxed::from_parts(18.into(), 33u8.into());
    assert_eq!(relaxed.to_float::<HalfEven, 10>(16), reduced.to_float::<HalfEven, 10>(16));
    assert_eq!(relaxed.to_float::<HalfAway, 10>(4), reduced.to_float::<HalfAway, 10>(4));
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
