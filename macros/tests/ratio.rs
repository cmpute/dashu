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
    assert_eq!(
        rbig!(-2 / -123456789012345678901234567),
        RBig::from_str("2/123456789012345678901234567").unwrap()
    );
    assert_eq!(
        rbig!(-987654321098765432109876543 / -2),
        RBig::from_str("987654321098765432109876543/2").unwrap()
    );

    // const test
    const _: RBig = rbig!(0);
    const _: RBig = rbig!(1);
    const _: RBig = rbig!(0xffffffff / 0xfffffffe);
    const _: RBig = rbig!(0xfffeffff0001 / 0xfffefffe0002); // has a common factor 0xffff
}

#[rustversion::since(1.64)]
#[rustversion::attr(since(1.64), test)]
fn test_static_rbig() {
    use dashu_macros::static_rbig;

    let zero: &'static RBig = static_rbig!(0);
    assert_eq!(*zero, RBig::ZERO);

    let one: &'static RBig = static_rbig!(1);
    assert_eq!(*one, RBig::ONE);

    let medium1: &'static RBig = static_rbig!(-1234567890123456789 / 9876543210987654323);
    assert_eq!(*medium1, RBig::from_str("-1234567890123456789/9876543210987654323").unwrap());
    let medium2: &'static RBig = static_rbig!(1234567890123456789 / 2);
    assert_eq!(*medium2, RBig::from_str("1234567890123456789/2").unwrap());
    let medium3: &'static RBig = static_rbig!(-2 / 9876543210987654323);
    assert_eq!(*medium3, RBig::from_str("-2/9876543210987654323").unwrap());

    let big: &'static RBig =
        static_rbig!(-123456789012345678901234567 / 987654321098765432109876543);
    assert_eq!(
        *big,
        RBig::from_str("-123456789012345678901234567/987654321098765432109876543").unwrap()
    );
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
    assert_eq!(
        rbig!(~2/-123456789012345678901234567),
        Relaxed::from_str("-2/123456789012345678901234567").unwrap()
    );
    assert_eq!(
        rbig!(~987654321098765432109876543/-2),
        Relaxed::from_str("-987654321098765432109876543/2").unwrap()
    );

    // const test
    const _: Relaxed = rbig!(~0);
    const _: Relaxed = rbig!(~1);
    const _: Relaxed = rbig!(~0xffffffff/0xfffffffe);
    const _: Relaxed = rbig!(~0xffffffff00000000/0xfffffffe00000000);
}

#[rustversion::since(1.64)]
#[rustversion::attr(since(1.64), test)]
fn test_static_relaxed() {
    use dashu_macros::static_rbig;

    let zero: &'static Relaxed = static_rbig!(~0);
    assert_eq!(*zero, Relaxed::ZERO);

    let one: &'static Relaxed = static_rbig!(~1);
    assert_eq!(*one, Relaxed::ONE);

    let medium1: &'static Relaxed = static_rbig!(~-123456789012345678 / 987654321098765432);
    assert_eq!(*medium1, Relaxed::from_str("-123456789012345678/987654321098765432").unwrap());
    let medium2: &'static Relaxed = static_rbig!(~123456789012345678 / 2);
    assert_eq!(*medium2, Relaxed::from_str("123456789012345678/2").unwrap());
    let medium3: &'static Relaxed = static_rbig!(~-2 / 987654321098765432);
    assert_eq!(*medium3, Relaxed::from_str("-2/987654321098765432").unwrap());

    let big: &'static Relaxed =
        static_rbig!(~-123456789012345678901234567/987654321098765432109876543);
    assert_eq!(
        *big,
        Relaxed::from_str("-123456789012345678901234567/987654321098765432109876543").unwrap()
    );
}
