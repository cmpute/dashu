#![allow(clippy::cmp_owned)]

use core::cmp::Ordering;

use dashu_base::{AbsOrd, Sign};
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
fn test_abs_ord() {
    // assert!(ubig!(12).abs_eq(&ubig!(12)));
    // assert!(ubig!(12).abs_eq(&ibig!(-12)));
    // assert!(ibig!(-12).abs_eq(&ubig!(12)));
    // assert!(ibig!(12).abs_eq(&ibig!(-12)));

    assert!(ibig!(-12).abs_cmp(&ubig!(10)).is_ge());
    assert!(ibig!(-12).abs_cmp(&ubig!(12)).is_eq());
    assert!(ibig!(-12).abs_cmp(&ubig!(14)).is_le());
    assert!(ubig!(12).abs_cmp(&ibig!(-10)).is_ge());
    assert!(ubig!(12).abs_cmp(&ibig!(-12)).is_eq());
    assert!(ubig!(12).abs_cmp(&ibig!(-14)).is_le());
}

#[test]
fn test_eq_across_representations() {
    // Regression for Repr::eq's inline-DoubleWord path and its sign/scale
    // short-circuits, plus the canonical-encoding requirement it relies on.
    // inline equal / unequal (capacity 1 and capacity 2)
    assert_eq!(ubig!(5), ubig!(5));
    assert_ne!(ubig!(5), ubig!(7));
    assert_eq!(ubig!(0x10000000000000000), ubig!(0x10000000000000000)); // 2^64 (cap 2)
    assert_ne!(ubig!(0x10000000000000000), ubig!(0x10000000000000001));
    // scale mismatch: an inline value can never equal a heap value
    assert_ne!(ubig!(5), ubig!(1) << 200);
    assert_ne!(ubig!(1) << 200, ubig!(5));
    // heap equal / unequal, same length and different length
    assert_eq!(ubig!(1) << 200, ubig!(1) << 200);
    assert_ne!((ubig!(1) << 200) + ubig!(1), (ubig!(1) << 200) + ubig!(2));
    assert_ne!(ubig!(1) << 200, ubig!(1) << 201);
    // cross-representation canonical equality: `ones` must compare equal to the
    // inline form of the same value (the from_buffer canonicalisation).
    assert_eq!(UBig::ones(128), UBig::from(u128::MAX));
    assert_eq!(UBig::ones(256), (ubig!(1) << 256) - ubig!(1));
    // IBig: zero is canonically positive, so sign disagreement is never equal
    assert_eq!(ibig!(-5), ibig!(-5));
    assert_ne!(ibig!(5), ibig!(-5));
    assert_eq!(ibig!(0), ibig!(0));
    assert_ne!(ibig!(0), ibig!(1));
    assert_eq!(-(ibig!(1) << 200), -(ibig!(1) << 200));
    assert_ne!(-(ibig!(1) << 200), ibig!(1) << 200);
}
