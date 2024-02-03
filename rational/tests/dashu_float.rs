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
