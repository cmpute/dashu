use std::str::FromStr;

use dashu_macros::rbig;
use dashu_ratio::{RBig, Relaxed};

#[test]
fn test_rbig() {
    assert_eq!(rbig!(0), RBig::ZERO);
    assert_eq!(rbig!(1), RBig::ONE);
    assert_eq!(rbig!(-1), RBig::NEG_ONE);
    assert_eq!(rbig!(12 / 34), RBig::from_str("12/34").unwrap());
    assert_eq!(rbig!(0xc / 0xd), RBig::from_str_radix("c/d", 16).unwrap());
    assert_eq!(rbig!(0xc / d), RBig::from_str_radix("c/d", 16).unwrap());
    assert_eq!(rbig!(c/d base 16), RBig::from_str_radix("c/d", 16).unwrap());
    assert_eq!(rbig!(c/d base 32), RBig::from_str_radix("c/d", 32).unwrap());
    assert_eq!(rbig!(-12_34 / 56_78), RBig::from_str("-1234/5678").unwrap());
    assert_eq!(rbig!(-12_34 / -56_78), RBig::from_str("1234/5678").unwrap());

    // const test
    const _: RBig = rbig!(0);
    const _: RBig = rbig!(1);
    const _: RBig = rbig!(0xffffffff / 0xfffffffe);
    const _: RBig = rbig!(0xfffeffff0001 / 0xfffefffe0002); // has a common factor 0xffff
}

#[test]
fn test_relaxed() {
    assert_eq!(rbig!(~0), Relaxed::ZERO);
    assert_eq!(rbig!(~1), Relaxed::ONE);
    assert_eq!(rbig!(~-1), Relaxed::NEG_ONE);
    assert_eq!(rbig!(~12/34), Relaxed::from_str("12/34").unwrap());
    assert_eq!(rbig!(~0xc/0xd), Relaxed::from_str_radix("c/d", 16).unwrap());
    assert_eq!(rbig!(~0xc/d), Relaxed::from_str_radix("c/d", 16).unwrap());
    assert_eq!(rbig!(~c/d base 16), Relaxed::from_str_radix("c/d", 16).unwrap());
    assert_eq!(rbig!(~c/d base 32), Relaxed::from_str_radix("c/d", 32).unwrap());
    assert_eq!(rbig!(~-12_34/56_78), Relaxed::from_str("-1234/5678").unwrap());
    assert_eq!(rbig!(~-12_34/56_78), Relaxed::from_str("-1234/5678").unwrap());

    // const test
    const _: Relaxed = rbig!(~0);
    const _: Relaxed = rbig!(~1);
    const _: Relaxed = rbig!(~0xffffffff/0xfffffffe);
    const _: Relaxed = rbig!(~0xffffffff00000000/0xfffffffe00000000);
}
