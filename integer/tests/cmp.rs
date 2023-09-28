#![allow(clippy::cmp_owned)]

use core::cmp::Ordering;

use dashu_base::{AbsEq, AbsOrd, Sign};
use dashu_int::{IBig, UBig};

mod helper_macros;

#[test]
fn test_eq_and_cmp() {
    assert_eq!(ubig!(0), UBig::from_words(&[]));
    assert_eq!(ubig!(0), UBig::from_words(&[0]));
    assert_eq!(ibig!(0), IBig::from_parts(Sign::Positive, UBig::from_words(&[])));
    assert_eq!(ibig!(0), IBig::from_parts(Sign::Positive, UBig::from_words(&[0])));
    assert_eq!(ubig!(500), UBig::from_words(&[500]));
    assert_eq!(ibig!(500), IBig::from_parts(Sign::Positive, UBig::from_words(&[500])));
    assert_eq!(ibig!(-500), IBig::from_parts(Sign::Negative, UBig::from_words(&[500])));
    assert_eq!(ubig!(500).cmp(&ubig!(500)), Ordering::Equal);

    assert!(ubig!(100) < ubig!(500));
    assert!(ubig!(500) > ubig!(100));
    assert!(ubig!(0x10000000000000000) > ubig!(100));
    assert!(ubig!(100) < ubig!(0x100000000000000000000000000000000));
    assert!(
        ubig!(0x100000000000000020000000000000003) < ubig!(0x100000000000000030000000000000002)
    );
    assert!(
        ubig!(0x100000000000000030000000000000002) > ubig!(0x100000000000000020000000000000003)
    );
    assert_eq!(
        ubig!(0x100000000000000030000000000000002).cmp(&ubig!(0x100000000000000030000000000000002)),
        Ordering::Equal
    );

    assert_eq!(ibig!(500).cmp(&ibig!(500)), Ordering::Equal);
    assert_eq!(ibig!(-500).cmp(&ibig!(-500)), Ordering::Equal);
    assert!(ibig!(5) < ibig!(10));
    assert!(ibig!(10) > ibig!(5));
    assert!(ibig!(-5) < ibig!(10));
    assert!(ibig!(-15) < ibig!(10));
    assert!(ibig!(10) > ibig!(-5));
    assert!(ibig!(10) > ibig!(-15));
    assert!(ibig!(-10) < ibig!(-5));
    assert!(ibig!(-5) > ibig!(-10));
}

#[test]
fn test_abs_eq_and_cmp() {
    assert!(ubig!(12).abs_eq(&ubig!(12)));
    assert!(ubig!(12).abs_eq(&ibig!(-12)));
    assert!(ibig!(-12).abs_eq(&ubig!(12)));
    assert!(ibig!(12).abs_eq(&ibig!(-12)));

    assert!(ibig!(-12).abs_cmp(&ubig!(10)).is_ge());
    assert!(ibig!(-12).abs_cmp(&ubig!(12)).is_eq());
    assert!(ibig!(-12).abs_cmp(&ubig!(14)).is_le());
    assert!(ubig!(12).abs_cmp(&ibig!(-10)).is_ge());
    assert!(ubig!(12).abs_cmp(&ibig!(-12)).is_eq());
    assert!(ubig!(12).abs_cmp(&ibig!(-14)).is_le());
}
