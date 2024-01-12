//! Test for importing items from dashu, and do basic operations

use dashu::{rational::Relaxed, *};

#[test]
#[rustfmt::skip::macros(fbig)]
fn test_macros() {
    // small numbers
    const A: Natural = ubig!(1234);
    const B: Integer = ibig!(-1234);
    assert_eq!(A + B, ibig!(0));

    const C: Real = fbig!(0x1234p-4);
    const D: Decimal = dbig!(12.34);
    assert!(C.to_decimal().value() > D);

    const E: Rational = rbig!(2 / 5);
    const F: Relaxed = rbig!(~2/7);
    assert!(E.relax() > F);

    // large numbers
    let a = ubig!(0xfffffffffffffffffffffffffffffffffffffffffffffffe);
    let b = ibig!(-0xffffffffffffffffffffffffffffffffffffffffffffffff);
    assert_eq!(a + b, ibig!(-1));

    let c = fbig!(0xffffffffffffffffffffffffffffffffffffffffffffffffp-192);
    let d = dbig!(999999999999999999999999999999999999999999999999999999999999e-60);
    assert!(c < d.to_binary().value());

    let e = rbig!(
        0xfffffffffffffffffffffffffffffffffffffffffffffffe
            / 0xffffffffffffffffffffffffffffffffffffffffffffffff
    );
    let f = rbig!(~
        999999999999999999999999999999999999999999999999999999999998
            / 999999999999999999999999999999999999999999999999999999999999);
    assert!(e < f.canonicalize());
}

#[test]
#[rustversion::since(1.64)]
#[rustfmt::skip::macros(static_fbig)]
fn test_static_macros() {
    static SA: &Natural = static_ubig!(1234);
    static SB: &Integer = static_ibig!(-1234);
    assert_eq!(SA + SB, ibig!(0));

    static SC: &Real = static_fbig!(0x1234p-4);
    static SD: &Decimal = static_dbig!(12.34);
    assert!(SC.to_decimal().value() > *SD);

    static SE: &Rational = static_rbig!(2 / 5);
    static SF: &Relaxed = static_rbig!(~2/7);
    assert!(SE.as_relaxed() > SF);

    static BA: &Natural = static_ubig!(0xfffffffffffffffffffffffffffffffffffffffffffffffe);
    static BB: &Integer = static_ibig!(-0xffffffffffffffffffffffffffffffffffffffffffffffff);
    assert_eq!(BA + BB, ibig!(-1));

    static BC: &Real = static_fbig!(0xffffffffffffffffffffffffffffffffffffffffffffffffp-192);
    static BD: &Decimal =
        static_dbig!(999999999999999999999999999999999999999999999999999999999999e-60);
    assert!(*BC < BD.clone().with_base_and_precision(200).value());

    static BE: &Rational = static_rbig!(
        0xfffffffffffffffffffffffffffffffffffffffffffffffe
            / 0xffffffffffffffffffffffffffffffffffffffffffffffff
    );
    static BF: &Relaxed = static_rbig!(~
        999999999999999999999999999999999999999999999999999999999998
            / 999999999999999999999999999999999999999999999999999999999999);
    assert!(*BE < BF.clone().canonicalize());
}
