//! Test for importing items from dashu, and do basic operations

use dashu::{float::*, integer::*, rational::*, *};

#[test]
#[rustfmt::skip::macros(fbig)]
fn test_macros() {
    // small numbers
    const A: UBig = ubig!(1234);
    const B: IBig = ibig!(-1234);
    assert_eq!(A + B, ibig!(0));

    static SA: &'static UBig = static_ubig!(1234);
    static SB: &'static IBig = static_ibig!(-1234);
    assert_eq!(SA + SB, ibig!(0));

    const C: FBig = fbig!(0x1234p-4);
    const D: DBig = dbig!(12.34);
    assert!(C.to_decimal().value() > D);

    const E: RBig = rbig!(2 / 5);
    const F: Relaxed = rbig!(~2/7);
    assert!(E.relax() > F);

    // large numbers
    let a = ubig!(0xfffffffffffffffffffffffffffffffffffffffffffffffe);
    let b = ibig!(-0xffffffffffffffffffffffffffffffffffffffffffffffff);
    assert_eq!(a + b, ibig!(-1));

    let c = fbig!(0xffffffffffffffffffffffffffffffffffffffffffffffffp-192);
    let d = dbig!(999999999999999999999999999999999999999999999999999999999999e-60);
    assert!(c < d.to_binary().value());

    // let e = rbig!(0xfffffffffffffffffffffffffffffffffffffffffffffffe/0xffffffffffffffffffffffffffffffffffffffffffffffff);
    let e = rbig!(
        6277101735386680763835789423207666416102355444464034512894
            / 6277101735386680763835789423207666416102355444464034512895
    );
    let f = rbig!(~999999999999999999999999999999999999999999999999999999999998/999999999999999999999999999999999999999999999999999999999999);
    assert!(e < f.canonicalize());
}
