//! Test for importing items from dashu, and do basic operations

use dashu::*;

#[test]
fn test_macros() {
    let a = ubig!(1234);
    let b = ibig!(-1234);
    assert_eq!(a + b, ubig!(0));

    let c = fbig!(0x1234p-4);
    let d = dbig!(12.34);
    assert!(c.to_decimal().value() > d);
}
