use std::str::FromStr;

use dashu_macros::rbig;
use dashu_ratio::{RBig, Relaxed};

#[test]
fn test_rbig() {
    assert_eq!(rbig!(0), RBig::ZERO);
    assert_eq!(rbig!(1), RBig::ONE);
    assert_eq!(rbig!(-1), RBig::NEG_ONE);
    assert_eq!(rbig!(12 / 34), RBig::from_str("12/34").unwrap());
    assert_eq!(rbig!(-12_34 / 56_78), RBig::from_str("-12_34/56_78").unwrap());

    // const test
    const _: RBig = rbig!(0);
    const _: RBig = rbig!(1);
    // const _: RBig = rbig!(0xffffffff/0xfffffffe);
}

#[test]
fn test_relaxed() {
    assert_eq!(rbig!(~0), Relaxed::ZERO);
    assert_eq!(rbig!(~1), Relaxed::ONE);
    assert_eq!(rbig!(~-1), Relaxed::NEG_ONE);
    assert_eq!(rbig!(~12/34), Relaxed::from_str("12/34").unwrap());
    assert_eq!(rbig!(~-12_34/56_78), Relaxed::from_str("-12_34/56_78").unwrap());

    // const test
    const _: Relaxed = rbig!(~0);
    const _: Relaxed = rbig!(~1);
    // const _: Relaxed = rbig!(~0xffffffff/0xfffffffe);
}
