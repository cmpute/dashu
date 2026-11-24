use core::str::FromStr;

use dashu_float::DBig;
use dashu_macros::{dbig, fbig};
type FBig = dashu_float::FBig;

#[test]
#[rustfmt::skip::macros(fbig)]
fn test_fbig() {
    // binary digits
    assert_eq!(fbig!(0), FBig::ZERO);
    assert_eq!(fbig!(0).precision(), 0);
    assert_eq!(fbig!(00001), FBig::ONE);
    assert_eq!(fbig!(00001).precision(), 5);
    assert_eq!(fbig!(-1.), FBig::NEG_ONE);
    assert_eq!(fbig!(-1.00), FBig::NEG_ONE);
    assert_eq!(fbig!(-1.00).precision(), 3);
    assert_eq!(fbig!(-101.001), FBig::from_str("-101.001").unwrap());
    assert_eq!(fbig!(1001.b23), FBig::from_str("1001.b23").unwrap());

    // hex digits
    assert_eq!(fbig!(0x1234), FBig::from_str("0x1234").unwrap());
    assert_eq!(fbig!(-_0x1.02), FBig::from_str("-0x1.02").unwrap());
    assert_eq!(fbig!(_0x1.), FBig::from_str("0x1.").unwrap());
    assert_eq!(fbig!(-_0x.02), FBig::from_str("-0x.02").unwrap());
    assert_eq!(fbig!(-_0x1.02p2), FBig::from_str("-0x1.02p2").unwrap());
    assert_eq!(fbig!(0x1p2), FBig::from_str("0x1p2").unwrap());
    assert_eq!(fbig!(_0x1.p - 2), FBig::from_str("0x1.p-2").unwrap());
    assert_eq!(fbig!(_0x.02p2), FBig::from_str("0x.02p2").unwrap());
    assert_eq!(fbig!(-_0x.02p-2), FBig::from_str("-0x.02p-2").unwrap());

    // big float
    assert_eq!(
        fbig!(0x5a4653ca673768565b41f775d6947d55cf3813d1p-200),
        FBig::from_str("0x5a4653ca673768565b41f775d6947d55cf3813d1p-200").unwrap()
    );
    assert_eq!(fbig!(0x5a4653ca673768565b41f775d6947d55cf3813d1p-200).precision(), 160);
    assert_eq!(
        fbig!(0x5a4653ca673768565b41f0000000000000000000p-200),
        FBig::from_str("0x5a4653ca673768565b41f0000000000000000000p-200").unwrap()
    );
    assert_eq!(fbig!(0x5a4653ca673768565b41f0000000000000000000p-200).precision(), 160);

    // const test
    const _: FBig = fbig!(0);
    const _: FBig = fbig!(1);
    const _: FBig = fbig!(-1);
    const _: FBig = fbig!(-10.01b100);
    const _: FBig = fbig!(0xffffffffp-1234);
}

#[test]
fn test_static_fbig() {
    use dashu_macros::static_fbig;

    let zero: &'static FBig = static_fbig!(0);
    assert_eq!(*zero, FBig::ZERO);

    let one: &'static FBig = static_fbig!(1);
    assert_eq!(*one, FBig::ONE);

    let big: &'static FBig = static_fbig!(0x5a4653ca673768565b41f775d6947d55cf3813d1p - 200);
    assert_eq!(*big, FBig::from_str("0x5a4653ca673768565b41f775d6947d55cf3813d1p-200").unwrap());
}

#[test]
fn test_dbig() {
    assert_eq!(dbig!(0), DBig::ZERO);
    assert_eq!(dbig!(0).precision(), 0);
    assert_eq!(dbig!(00001), DBig::ONE);
    assert_eq!(dbig!(00001).precision(), 5);
    assert_eq!(dbig!(-1.), DBig::NEG_ONE);
    assert_eq!(dbig!(-1.00), DBig::NEG_ONE);
    assert_eq!(dbig!(-1.00).precision(), 3);
    assert_eq!(dbig!(-123.004), DBig::from_str("-123.004").unwrap());

    assert_eq!(dbig!(1234.e23), DBig::from_str("1234.e23").unwrap());
    assert_eq!(dbig!(12.34e-5), DBig::from_str("12.34e-5").unwrap());

    // big float
    assert_eq!(
        dbig!(515377520732011331036461129765621272702107522001e-100),
        DBig::from_str("515377520732011331036461129765621272702107522001e-100").unwrap()
    );
    assert_eq!(dbig!(515377520732011331036461129765621272702107522001e-100).precision(), 48);
    assert_eq!(
        dbig!(515377520732011331036461129765621272702107500000e-100),
        DBig::from_str("515377520732011331036461129765621272702107500000e-100").unwrap()
    );
    assert_eq!(dbig!(515377520732011331036461129765621272702107500000e-100).precision(), 48);

    // const test
    const _: DBig = dbig!(0);
    const _: DBig = dbig!(1);
    const _: DBig = dbig!(-1);
    const _: DBig = dbig!(-2.55e100);
    const _: DBig = dbig!(4294967295e-1234);
}

#[test]
fn test_static_dbig() {
    use dashu_macros::static_dbig;

    let zero: &'static DBig = static_dbig!(0);
    assert_eq!(*zero, DBig::ZERO);

    let one: &'static DBig = static_dbig!(1);
    assert_eq!(*one, DBig::ONE);

    let big: &'static DBig = static_dbig!(515377520732011331036461129765621272702107522001e-100);
    assert_eq!(
        *big,
        DBig::from_str("515377520732011331036461129765621272702107522001e-100").unwrap()
    );
}

#[test]
#[rustfmt::skip::macros(fbig, dbig)]
fn test_negative_zero() {
    // the sign of a zero literal is kept, as by `from_str`
    assert!(FBig::from_str("-0.0").unwrap().repr().is_neg_zero());
    assert!(fbig!(-0.0).repr().is_neg_zero());
    assert!(fbig!(-0).repr().is_neg_zero());
    assert!(fbig!(-0x0p3).repr().is_neg_zero());
    assert!(fbig!(0.0).repr().is_pos_zero());
    assert!(fbig!(+0.0).repr().is_pos_zero());

    assert!(DBig::from_str("-0.0").unwrap().repr().is_neg_zero());
    assert!(dbig!(-0.0).repr().is_neg_zero());
    assert!(dbig!(-0).repr().is_neg_zero());
    assert!(dbig!(-0e5).repr().is_neg_zero());
    assert!(dbig!(0.0).repr().is_pos_zero());
    assert!(dbig!(+0.0).repr().is_pos_zero());
    assert_eq!(dbig!(-0.0).to_string(), "-0");

    // also in const and static position
    const F: FBig = fbig!(-0.0);
    const D: DBig = dbig!(-0.0);
    assert!(F.repr().is_neg_zero() && D.repr().is_neg_zero());
    use dashu_macros::{static_dbig, static_fbig};
    assert!(static_fbig!(-0.0).repr().is_neg_zero());
    assert!(static_dbig!(-0.0).repr().is_neg_zero());
    assert!(static_dbig!(0.0).repr().is_pos_zero());

    // nonzero literals are unaffected
    assert_eq!(dbig!(-1.0), DBig::from_str("-1.0").unwrap());
    assert_eq!(fbig!(-1.0), FBig::from_str("-1.0").unwrap());
}

#[test]
#[rustfmt::skip::macros(static_fbig, static_dbig)]
fn test_static_precision() {
    use dashu_macros::{static_dbig, static_fbig};

    // static floats have unlimited precision, whether or not the significand fits in 32 bits
    assert_eq!(static_dbig!(4294967295).precision(), 0);
    assert_eq!(static_dbig!(4294967296).precision(), 0);
    assert_eq!(static_dbig!(12300).precision(), 0);
    assert_eq!(static_dbig!(5.000).precision(), 0);
    assert_eq!(static_dbig!(-1.5e-3).precision(), 0);
    assert_eq!(static_fbig!(0xffffffff).precision(), 0);
    assert_eq!(static_fbig!(0x100000000).precision(), 0);
    assert_eq!(static_fbig!(-1.01).precision(), 0);

    // the values are those of the non-static literals
    assert_eq!(*static_dbig!(4294967295), dbig!(4294967295));
    assert_eq!(*static_dbig!(12300), dbig!(12300));
    assert_eq!(*static_dbig!(5.000), dbig!(5));
    assert_eq!(*static_dbig!(-1.5e-3), dbig!(-1.5e-3));
    assert_eq!(*static_fbig!(0xffffffff), fbig!(0xffffffff));
    assert_eq!(*static_fbig!(-1.01), fbig!(-1.01));
}
