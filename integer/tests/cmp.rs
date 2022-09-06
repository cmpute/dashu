use core::cmp::Ordering;

mod helper_macros;

#[test]
fn test_cmp() {
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
fn test_cross_type_cmp() {
    assert_eq!(ubig!(500), ibig!(500));
    assert_ne!(ubig!(500), ibig!(-500));
    assert_eq!(ibig!(500), ubig!(500));
    assert_ne!(ibig!(-500), ubig!(500));

    assert!(ubig!(500) > ibig!(499));
    assert!(ibig!(500) > ubig!(499));
    assert!(ubig!(500) > ibig!(-500));
    assert!(ibig!(-500) < ubig!(500));

    assert_eq!(ubig!(500), 500);
    assert_ne!(ubig!(500), -500);
    assert_eq!(ibig!(500), 500);
    assert_ne!(ibig!(-500), 500);

    assert!(ubig!(500) > 499);
    assert!(ibig!(500) > 499);
    assert!(ubig!(500) > -500);
    assert!(ibig!(-500) < 500);
}
