use core::cmp::Ordering;

use dashu_float::{DBig, FBig};

mod helper_macros;
type FBin = FBig;

#[test]
fn test_eq_binary() {
    assert_eq!(fbig!(0x0p1), fbig!(-0x0p-1));
    assert_ne!(fbig!(0x0p1), fbig!(0x1p1));
    assert_ne!(fbig!(0x0p1), fbig!(-0x1p1));
    assert_ne!(fbig!(0x1p1), fbig!(-0x1p1));

    assert_eq!(fbig!(0x1000), fbig!(0x1p12));
    assert_ne!(fbig!(0x1001), fbig!(0x1p12));

    assert_eq!(FBin::INFINITY, FBin::INFINITY);
    assert_eq!(FBin::NEG_INFINITY, FBin::NEG_INFINITY);
    assert_ne!(FBin::INFINITY, fbig!(0x1));
    assert_ne!(FBin::NEG_INFINITY, fbig!(-0x1));
    assert_ne!(FBin::INFINITY, FBin::NEG_INFINITY);
}

#[test]
fn test_eq_decimal() {
    assert_eq!(dbig!(0e1), dbig!(-0e-1));
    assert_ne!(dbig!(0e1), dbig!(1e1));
    assert_ne!(dbig!(0e1), dbig!(-1e1));
    assert_ne!(dbig!(1e1), dbig!(-1e1));

    assert_eq!(dbig!(1000), dbig!(1e3));
    assert_ne!(dbig!(1001), dbig!(1e3));

    assert_eq!(DBig::INFINITY, DBig::INFINITY);
    assert_eq!(DBig::NEG_INFINITY, DBig::NEG_INFINITY);
    assert_ne!(DBig::INFINITY, dbig!(1));
    assert_ne!(DBig::NEG_INFINITY, dbig!(-1));
    assert_ne!(DBig::INFINITY, DBig::NEG_INFINITY);
}

#[test]
fn test_cmp_binary() {
    // case 1: compare with inf
    assert_eq!(FBin::INFINITY.cmp(&FBin::INFINITY), Ordering::Equal);
    assert_eq!(FBin::NEG_INFINITY.cmp(&FBin::NEG_INFINITY), Ordering::Equal);
    assert!(FBin::INFINITY > FBin::NEG_INFINITY);
    assert!(FBin::NEG_INFINITY < FBin::INFINITY);

    assert!(FBin::INFINITY > fbig!(0x1));
    assert!(FBin::INFINITY > fbig!(0x1p100));
    assert!(FBin::INFINITY > fbig!(-0x1));
    assert!(FBin::INFINITY > fbig!(-0x1p100));
    assert!(FBin::NEG_INFINITY < FBin::INFINITY);
    assert!(FBin::NEG_INFINITY < fbig!(0x1));
    assert!(FBin::NEG_INFINITY < fbig!(0x1p100));
    assert!(FBin::NEG_INFINITY < fbig!(-0x1));
    assert!(FBin::NEG_INFINITY < fbig!(-0x1p100));

    // case 2: compare sign
    assert!(fbig!(0x1) > fbig!(0));
    assert!(fbig!(-0x1) < fbig!(0));
    assert!(fbig!(0x1) > fbig!(-0x1));
    assert!(fbig!(-0x1) < fbig!(0x1));
    assert!(fbig!(0x1) > fbig!(-0x1p100));
    assert!(fbig!(-0x1) < fbig!(0x1p-100));

    // case 3: compare exponent and precision
    assert!(fbig!(0x1p100) > fbig!(0x1));
    assert!(fbig!(0x1p-100) < fbig!(0x1));
    assert!(fbig!(-0x1p100) < fbig!(-0x1));
    assert!(fbig!(-0x1p-100) > fbig!(-0x1));
    assert!(fbig!(0xffff) < fbig!(0x1p17));
    assert!(fbig!(-0xffffp-17) > fbig!(-0x1));

    // case 4: compare exponent and digits
    assert!(fbig!(0x0000ffff) < fbig!(0x1p17));
    assert!(fbig!(-0x0000ffffp-17) > fbig!(-0x1));

    // case 5: compare exact values
    assert!(fbig!(0xffff) < fbig!(0x1p16));
    assert!(fbig!(-0xffffp-16) > fbig!(-0x1));
    assert!(fbig!(0xfffd) < fbig!(0xffff));
    assert!(fbig!(0xfffdp1) > fbig!(0xffff));
    assert!(fbig!(0x1234p16) < fbig!(0x12345678));
    assert!(fbig!(-0x1234p-16) > fbig!(-0x12345678p-32));
}

#[test]
fn test_cmp_decimal() {
    // case 1: compare with inf
    assert_eq!(DBig::INFINITY.cmp(&DBig::INFINITY), Ordering::Equal);
    assert_eq!(DBig::NEG_INFINITY.cmp(&DBig::NEG_INFINITY), Ordering::Equal);
    assert!(DBig::INFINITY > DBig::NEG_INFINITY);
    assert!(DBig::NEG_INFINITY < DBig::INFINITY);

    assert!(DBig::INFINITY > dbig!(1));
    assert!(DBig::INFINITY > dbig!(1e100));
    assert!(DBig::INFINITY > dbig!(-1));
    assert!(DBig::INFINITY > dbig!(-1e100));
    assert!(DBig::NEG_INFINITY < DBig::INFINITY);
    assert!(DBig::NEG_INFINITY < dbig!(1));
    assert!(DBig::NEG_INFINITY < dbig!(1e100));
    assert!(DBig::NEG_INFINITY < dbig!(-1));
    assert!(DBig::NEG_INFINITY < dbig!(-1e100));

    // case 2: compare sign
    assert!(dbig!(1) > dbig!(0));
    assert!(dbig!(-1) < dbig!(0));
    assert!(dbig!(1) > dbig!(-1));
    assert!(dbig!(-1) < dbig!(1));
    assert!(dbig!(1) > dbig!(-1e100));
    assert!(dbig!(-1) < dbig!(1e-100));

    // case 3: compare exponent and precision
    assert!(dbig!(1e100) > dbig!(1));
    assert!(dbig!(1e-100) < dbig!(1));
    assert!(dbig!(-1e100) < dbig!(-1));
    assert!(dbig!(-1e-100) > dbig!(-1));
    assert!(dbig!(9999) < dbig!(1e4));
    assert!(dbig!(-9999e-4) > dbig!(-1));

    // case 4: compare exponent and digits
    assert!(dbig!(00009999) < dbig!(1e4));
    assert!(dbig!(-00009999e-4) > dbig!(-1));

    // case 5: compare exact values
    assert!(dbig!(9999) < dbig!(1e16));
    assert!(dbig!(-9999e-16) > dbig!(-1));
    assert!(dbig!(9998) < dbig!(9999));
    assert!(dbig!(9998e1) > dbig!(9999));
    assert!(dbig!(1234e4) < dbig!(12345678));
    assert!(dbig!(-1234e-4) > dbig!(-12345678e-8));
}
