use dashu_int::{IBig, UBig};

mod helper_macros;

#[test]
fn test_sum() {
    let nums = [
        ubig!(0),
        ubig!(1),
        ubig!(10),
        ubig!(100),
        ubig!(10000),
        ubig!(100000000),
        ubig!(10000000000000000),
        ubig!(100000000000000000000000000000000),
    ];

    assert_eq!((&nums[..0]).iter().sum::<UBig>(), ubig!(0));
    assert_eq!((&nums[..1]).iter().sum::<UBig>(), ubig!(0));
    assert_eq!((&nums[..2]).iter().sum::<UBig>(), ubig!(1));
    assert_eq!((&nums[..4]).iter().sum::<UBig>(), ubig!(111));
    assert_eq!(nums.iter().sum::<UBig>(), ubig!(100000000000000010000000100010111));
    assert_eq!(nums.iter().sum::<IBig>(), ibig!(100000000000000010000000100010111));
    assert_eq!(nums.into_iter().sum::<UBig>(), ubig!(100000000000000010000000100010111));

    let nums = [
        ibig!(0),
        ibig!(-1),
        ibig!(10),
        ibig!(-100),
        ibig!(10000),
        ibig!(-100000000),
        ibig!(10000000000000000),
        ibig!(-100000000000000000000000000000000),
    ];

    assert_eq!((&nums[..0]).iter().sum::<IBig>(), ibig!(0));
    assert_eq!((&nums[..1]).iter().sum::<IBig>(), ibig!(0));
    assert_eq!((&nums[..2]).iter().sum::<IBig>(), ibig!(-1));
    assert_eq!((&nums[..4]).iter().sum::<IBig>(), ibig!(-91));
    assert_eq!(nums.iter().sum::<IBig>(), ibig!(-99999999999999990000000099990091));
    assert_eq!(nums.into_iter().sum::<IBig>(), ibig!(-99999999999999990000000099990091));
}

#[test]
fn test_prod() {
    assert_eq!((1..4u8).map(|v| UBig::from(v)).product::<UBig>(), ubig!(6));
    assert_eq!((1..10u8).map(|v| UBig::from(v)).product::<UBig>(), ubig!(362880));
    assert_eq!((1..10u8).map(|v| UBig::from(v)).product::<IBig>(), ibig!(362880));
    assert_eq!((0..4u8).map(|v| UBig::from(v)).product::<UBig>(), ubig!(0));
    assert_eq!((0..10u8).map(|v| UBig::from(v)).product::<UBig>(), ubig!(0));
    assert_eq!((0..10u8).map(|v| UBig::from(v)).product::<IBig>(), ibig!(0));

    assert_eq!((-4..-1).map(|v| IBig::from(v)).product::<IBig>(), ibig!(-24));
    assert_eq!((-10..-1).map(|v| IBig::from(v)).product::<IBig>(), ibig!(-3628800));
    assert_eq!((-4..4).map(|v| IBig::from(v)).product::<IBig>(), ibig!(0));
    assert_eq!((-10..10).map(|v| IBig::from(v)).product::<IBig>(), ibig!(0));
}
