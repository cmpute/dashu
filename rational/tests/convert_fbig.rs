use dashu_base::Approximation::*;
use dashu_float::{round::Rounding::*, DBig};

mod helper_macros;

type FBig = dashu_float::FBig;

#[test]
fn test_to_float_decimal() {
    assert_eq!(rbig!(0).to_float(1), Exact(DBig::ZERO));
    assert_eq!(rbig!(1).to_float(1), Exact(DBig::ONE));
    assert_eq!(rbig!(-1).to_float(1), Exact(DBig::NEG_ONE));
    assert_eq!(rbig!(1 / 2).to_float(1), Exact(DBig::from_str_native("0.5").unwrap()));
    assert_eq!(rbig!(2 / 5).to_float(1), Exact(DBig::from_str_native("0.4").unwrap()));
    assert_eq!(rbig!(9 / 100).to_float(1), Exact(DBig::from_str_native("0.09").unwrap()));
    assert_eq!(
        rbig!(21 / 33).to_float(4),
        Inexact(DBig::from_str_native("0.6364").unwrap(), AddOne)
    );
    assert_eq!(
        rbig!(2 / 33333333).to_float(4),
        Inexact(DBig::from_str_native("6.000e-8").unwrap(), NoOp)
    );
    assert_eq!(
        rbig!(22222222 / 3).to_float(4),
        Inexact(DBig::from_str_native("7.407e6").unwrap(), NoOp)
    );
}

#[test]
fn test_to_float_binary() {
    assert_eq!(rbig!(0).to_float(1), Exact(FBig::ZERO));
    assert_eq!(rbig!(1).to_float(1), Exact(FBig::ONE));
    assert_eq!(rbig!(-1).to_float(1), Exact(FBig::NEG_ONE));
    assert_eq!(rbig!(1 / 2).to_float(1), Exact(FBig::from_str_native("0x1p-1").unwrap()));
    assert_eq!(
        rbig!(2 / 5).to_float(1),
        Inexact(FBig::from_str_native("0x1p-2").unwrap(), NoOp)
    );
    assert_eq!(
        rbig!(9 / 100).to_float(4),
        Inexact(FBig::from_str_native("0xbp-7").unwrap(), NoOp)
    );
}
