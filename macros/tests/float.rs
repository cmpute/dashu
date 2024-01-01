use core::str::FromStr;

use dashu_float::DBig;
use dashu_macros::{dbig, fbig, static_fbig};
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
    let zero: &'static FBig = static_fbig!(0);
    assert_eq!(*zero, FBig::ZERO);

    let one: &'static FBig = static_fbig!(1);
    assert_eq!(*one, FBig::ONE);

    let big: &'static FBig =
        static_fbig!(0x5a4653ca673768565b41f775d6947d55cf3813d1p-200);
    assert_eq!(
        *big,
        FBig::from_str("0x5a4653ca673768565b41f775d6947d55cf3813d1p-200").unwrap()
    );
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
