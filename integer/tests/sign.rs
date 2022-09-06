use dashu_int::{
    ops::{Abs, UnsignedAbs},
    Sign,
};

mod helper_macros;

#[test]
#[allow(clippy::double_neg)]
fn test_neg() {
    assert_eq!(-ubig!(123), ibig!(-123));
    assert_eq!(-ibig!(123), ibig!(-123));
    assert_eq!(-ibig!(-123), ibig!(123));
    assert_eq!(-ibig!(0), ibig!(0));

    assert_eq!(-&ubig!(123), ibig!(-123));
    assert_eq!(-&ibig!(123), ibig!(-123));
    assert_eq!(-&ibig!(0), ibig!(0));
}

#[test]
fn test_abs() {
    assert_eq!(ibig!(123).abs(), ibig!(123));
    assert_eq!(ibig!(-123).abs(), ibig!(123));

    assert_eq!((&ibig!(-123)).abs(), ibig!(123));
}

#[test]
fn test_unsigned_abs() {
    assert_eq!(ibig!(123).unsigned_abs(), ubig!(123));
    assert_eq!(ibig!(-123).unsigned_abs(), ubig!(123));

    assert_eq!((&ibig!(-123)).unsigned_abs(), ubig!(123));
}

#[test]
fn test_signum() {
    assert_eq!(ibig!(-500).signum(), ibig!(-1));
    assert_eq!(ibig!(0).signum(), ibig!(0));
    assert_eq!(ibig!(500).signum(), ibig!(1));
}

#[test]
fn test_mul() {
    assert_eq!(Sign::Positive * ubig!(0), ibig!(0));
    assert_eq!(Sign::Negative * ubig!(0), ibig!(0));
    assert_eq!(Sign::Positive * ubig!(123), ibig!(123));
    assert_eq!(Sign::Negative * ubig!(123), ibig!(-123));

    assert_eq!(Sign::Positive * ibig!(0), ibig!(0));
    assert_eq!(Sign::Negative * ibig!(0), ibig!(0));
    assert_eq!(Sign::Positive * ibig!(123), ibig!(123));
    assert_eq!(Sign::Negative * ibig!(123), ibig!(-123));
    assert_eq!(Sign::Positive * ibig!(-123), ibig!(-123));
    assert_eq!(Sign::Negative * ibig!(-123), ibig!(123));
}
