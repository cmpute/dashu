use std::str::FromStr;

use dashu_int::{IBig, UBig, Word};
use dashu_macros::{ibig, ubig};

#[test]
fn test_ubig() {
    // decimals
    assert_eq!(ubig!(0), UBig::zero());
    assert_eq!(ubig!(00000001), UBig::one());
    assert_eq!(ubig!(12341234), UBig::from(12341234u32));
    assert_eq!(ubig!(12_34_12_34_), UBig::from(12_34_12_34_u32));
    assert_eq!(
        ubig!(123456789012345678901234567890123456789012345678901234567890),
        UBig::from_str("123456789012345678901234567890123456789012345678901234567890").unwrap()
    );
    assert_eq!(
        ubig!(1234567890_1234567890_1234567890_1234567890_1234567890),
        UBig::from_str("1234567890_1234567890_1234567890_1234567890_1234567890").unwrap()
    );

    // hexadecimals
    assert_eq!(ubig!(0 base 16), UBig::zero());
    assert_eq!(ubig!(1 base 16), UBig::one());
    assert_eq!(ubig!(0x00000001), UBig::one());
    assert_eq!(ubig!(0x12341234), UBig::from(0x12341234u32));
    assert_eq!(ubig!(abcdef base 16), UBig::from(0xabcdefu32));
    assert_eq!(
        ubig!(123456789012345678901234567890123456789012345678901234567890 base 16),
        UBig::from_str_radix(
            "123456789012345678901234567890123456789012345678901234567890",
            16
        )
        .unwrap()
    );
    assert_eq!(
        ubig!(1234567890_1234567890_1234567890_1234567890_1234567890 base 16),
        UBig::from_str_radix("1234567890_1234567890_1234567890_1234567890_1234567890", 16).unwrap()
    );

    // other radix tests
    assert_eq!(
        ubig!(a3gp1 base 32),
        UBig::from_str_radix("a3gp1", 32).unwrap()
    );
    assert_eq!(
        ubig!(13agp base 32),
        UBig::from_str_radix("13agp", 32).unwrap()
    );

    // const test
    const _: UBig = ubig!(0);
    const _: UBig = ubig!(1);
    #[cfg(target_pointer_width = "64")]
    {
        assert!(Word::BITS >= 64); // assumption only for testing
        const _: UBig = ubig!(0xffffffffffffffff);
        const _: UBig = ubig!(0xffffffffffffffffffffffffffffffff);
    }
}

#[test]
fn test_ibig() {
    // decimals
    assert_eq!(ibig!(0), IBig::zero());
    assert_eq!(ibig!(+0), IBig::zero());
    assert_eq!(ibig!(-0), IBig::zero());
    assert_eq!(ibig!(00000001), IBig::one());
    assert_eq!(ibig!(+00000001), IBig::one());
    assert_eq!(ibig!(-00000001), IBig::neg_one());
    assert_eq!(ibig!(-12341234), IBig::from(-12341234));
    assert_eq!(ibig!(-12_34_12_34_), IBig::from(-12_34_12_34_));
    assert_eq!(
        ibig!(123456789012345678901234567890123456789012345678901234567890),
        IBig::from_str("123456789012345678901234567890123456789012345678901234567890").unwrap()
    );
    assert_eq!(
        ibig!(+123456789012345678901234567890123456789012345678901234567890),
        IBig::from_str("+123456789012345678901234567890123456789012345678901234567890").unwrap()
    );
    assert_eq!(
        ibig!(-123456789012345678901234567890123456789012345678901234567890),
        IBig::from_str("-123456789012345678901234567890123456789012345678901234567890").unwrap()
    );
    assert_eq!(
        ibig!(-1234567890_1234567890_1234567890_1234567890_1234567890),
        IBig::from_str("-1234567890_1234567890_1234567890_1234567890_1234567890").unwrap()
    );

    // hexadecimals
    assert_eq!(ibig!(0 base 16), IBig::zero());
    assert_eq!(ibig!(+0 base 16), IBig::zero());
    assert_eq!(ibig!(-0 base 16), IBig::zero());
    assert_eq!(ibig!(1 base 16), IBig::one());
    assert_eq!(ibig!(+1 base 16), IBig::one());
    assert_eq!(ibig!(-1 base 16), IBig::neg_one());
    assert_eq!(ibig!(0x00000001), IBig::one());
    assert_eq!(ibig!(+0x00000001), IBig::one());
    assert_eq!(ibig!(-0x00000001), IBig::neg_one());
    assert_eq!(ibig!(-0x12341234), IBig::from(-0x12341234));
    assert_eq!(ibig!(-abcdef base 16), IBig::from(-0xabcdef));
    assert_eq!(
        ibig!(123456789012345678901234567890123456789012345678901234567890 base 16),
        IBig::from_str_radix(
            "123456789012345678901234567890123456789012345678901234567890",
            16
        )
        .unwrap()
    );
    assert_eq!(
        ibig!(+123456789012345678901234567890123456789012345678901234567890 base 16),
        IBig::from_str_radix(
            "+123456789012345678901234567890123456789012345678901234567890",
            16
        )
        .unwrap()
    );
    assert_eq!(
        ibig!(-123456789012345678901234567890123456789012345678901234567890 base 16),
        IBig::from_str_radix(
            "-123456789012345678901234567890123456789012345678901234567890",
            16
        )
        .unwrap()
    );
    assert_eq!(
        ibig!(-1234567890_1234567890_1234567890_1234567890_1234567890 base 16),
        IBig::from_str_radix(
            "-1234567890_1234567890_1234567890_1234567890_1234567890",
            16
        )
        .unwrap()
    );

    // other radix tests
    assert_eq!(
        ibig!(a3gp1 base 32),
        IBig::from_str_radix("a3gp1", 32).unwrap()
    );
    assert_eq!(
        ibig!(+a3gp1 base 32),
        IBig::from_str_radix("+a3gp1", 32).unwrap()
    );
    assert_eq!(
        ibig!(-a3gp1 base 32),
        IBig::from_str_radix("-a3gp1", 32).unwrap()
    );
    assert_eq!(
        ibig!(13agp base 32),
        IBig::from_str_radix("13agp", 32).unwrap()
    );
    assert_eq!(
        ibig!(+13agp base 32),
        IBig::from_str_radix("+13agp", 32).unwrap()
    );
    assert_eq!(
        ibig!(-13agp base 32),
        IBig::from_str_radix("-13agp", 32).unwrap()
    );

    // const test
    const _: IBig = ibig!(0);
    const _: IBig = ibig!(-1);
    #[cfg(target_pointer_width = "64")]
    {
        assert!(Word::BITS >= 64); // assumption only for testing
        const _: IBig = ibig!(-0xffffffffffffffff);
        const _: IBig = ibig!(-0xffffffffffffffffffffffffffffffff);
    }
}
