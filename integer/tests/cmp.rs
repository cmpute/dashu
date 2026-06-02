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
fn test_cmp_across_representations() {
    // Regression test for UBig::cmp (magnitude_cmp) / IBig::cmp (signed_cmp):
    // capacity only selects the inline-vs-heap branch; ordering is decided by
    // the actual words. Every pair below shares a scale/capacity yet differs in
    // value (or sits on the inline/heap boundary).
    use core::cmp::Ordering::*;
    let ubig_cases = [
        // both single-word inline (capacity 1)
        (ubig!(5), ubig!(7), Less),
        (ubig!(5), ubig!(5), Equal),
        // both two-word inline (capacity 2): 2^64 vs 2^64+1, 2^96 vs 2^64
        (ubig!(0x10000000000000000), ubig!(0x10000000000000001), Less),
        (ubig!(0x1000000000000000000000000), ubig!(0x10000000000000000), Greater),
        // capacity 1 vs capacity 2 (mixed inline scale): 2^64-1 vs 2^64
        (ubig!(0xffffffffffffffff), ubig!(0x10000000000000000), Less),
        // both heap, same length, differ only in the low word
        ((ubig!(1) << 130) + ubig!(1), (ubig!(1) << 130) + ubig!(2), Less),
        // both heap, different length
        (ubig!(1) << 200, ubig!(1) << 130, Greater),
        // inline/heap boundary: 2^128-1 (inline) vs 2^128 (heap)
        ((ubig!(1) << 128) - ubig!(1), ubig!(1) << 128, Less),
    ];
    for (a, b, want) in &ubig_cases {
        assert_eq!(a.cmp(b), *want, "{a} cmp {b}");
        assert_eq!(b.cmp(a), want.reverse(), "{b} cmp {a}");
        assert_eq!(a.partial_cmp(b), Some(*want));
    }
    let ibig_cases = [
        (ibig!(-5), ibig!(3), Less),  // neg < pos
        (ibig!(-5), ibig!(-3), Less), // more negative < less negative
        (ibig!(0), ibig!(-1), Greater),
        (ibig!(0), ibig!(1), Less),
        (ibig!(-5), ibig!(-5), Equal),
        // across inline/heap, both negative
        (-(ibig!(1) << 200), -(ibig!(1) << 130), Less),
        (-((ibig!(1) << 128) - ibig!(1)), -(ibig!(1) << 128), Greater),
    ];
    for (a, b, want) in &ibig_cases {
        assert_eq!(a.cmp(b), *want, "{a} cmp {b}");
        assert_eq!(b.cmp(a), want.reverse(), "{b} cmp {a}");
        assert_eq!(a.partial_cmp(b), Some(*want));
    }
}
