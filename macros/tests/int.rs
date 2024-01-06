use std::str::FromStr;

use dashu_int::{IBig, UBig};
use dashu_macros::{ibig, static_ibig, static_ubig, ubig};

#[test]
fn test_ubig() {
    // decimals
    assert_eq!(ubig!(0), UBig::ZERO);
    assert_eq!(ubig!(00000001), UBig::ONE);
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
    assert_eq!(ubig!(0 base 16), UBig::ZERO);
    assert_eq!(ubig!(1 base 16), UBig::ONE);
    assert_eq!(ubig!(0x00000001), UBig::ONE);
    assert_eq!(ubig!(0x12341234), UBig::from(0x12341234u32));
    assert_eq!(ubig!(abcdef base 16), UBig::from(0xabcdefu32));
    assert_eq!(
        ubig!(123456789012345678901234567890123456789012345678901234567890 base 16),
        UBig::from_str_radix("123456789012345678901234567890123456789012345678901234567890", 16)
            .unwrap()
    );
    assert_eq!(
        ubig!(1234567890_1234567890_1234567890_1234567890_1234567890 base 16),
        UBig::from_str_radix("1234567890_1234567890_1234567890_1234567890_1234567890", 16).unwrap()
    );

    // other radix tests
    assert_eq!(ubig!(a3gp1 base 32), UBig::from_str_radix("a3gp1", 32).unwrap());
    assert_eq!(ubig!(13agp base 32), UBig::from_str_radix("13agp", 32).unwrap());

    // const test
    const _: UBig = ubig!(0);
    const _: UBig = ubig!(1);
    const _: UBig = ubig!(0xffffffff);
}

#[test]
fn test_static_ubig() {
    let zero: &'static UBig = static_ubig!(0);
    assert_eq!(*zero, UBig::ZERO);

    let one: &'static UBig = static_ubig!(1);
    assert_eq!(*one, UBig::ONE);

    let medium1: &'static UBig = static_ubig!(0xfffffffffffffff);
    assert_eq!(*medium1, UBig::from(0xfffffffffffffffu64));
    let medium2: &'static UBig = static_ubig!(0xfffffffffffffffff);
    assert_eq!(*medium2, UBig::from(0xfffffffffffffffffu128));

    let big: &'static UBig =
        static_ubig!(123456789012345678901234567890123456789012345678901234567890);
    assert_eq!(
        *big,
        UBig::from_str("123456789012345678901234567890123456789012345678901234567890").unwrap()
    );
}

#[test]
fn test_ibig() {
    // decimals
    assert_eq!(ibig!(0), IBig::ZERO);
    assert_eq!(ibig!(+0), IBig::ZERO);
    assert_eq!(ibig!(-0), IBig::ZERO);
    assert_eq!(ibig!(00000001), IBig::ONE);
    assert_eq!(ibig!(+00000001), IBig::ONE);
    assert_eq!(ibig!(-00000001), IBig::NEG_ONE);
    assert_eq!(ibig!(-12341234), IBig::from(-12341234));
    assert_eq!(ibig!(-12_34_12_34_), IBig::from(-12341234));
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
    assert_eq!(ibig!(0 base 16), IBig::ZERO);
    assert_eq!(ibig!(+0 base 16), IBig::ZERO);
    assert_eq!(ibig!(-0 base 16), IBig::ZERO);
    assert_eq!(ibig!(1 base 16), IBig::ONE);
    assert_eq!(ibig!(+1 base 16), IBig::ONE);
    assert_eq!(ibig!(-1 base 16), IBig::NEG_ONE);
    assert_eq!(ibig!(0x00000001), IBig::ONE);
    assert_eq!(ibig!(+0x00000001), IBig::ONE);
    assert_eq!(ibig!(-0x00000001), IBig::NEG_ONE);
    assert_eq!(ibig!(-0x12341234), IBig::from(-0x12341234));
    assert_eq!(ibig!(-abcdef base 16), IBig::from(-0xabcdef));
    assert_eq!(
        ibig!(123456789012345678901234567890123456789012345678901234567890 base 16),
        IBig::from_str_radix("123456789012345678901234567890123456789012345678901234567890", 16)
            .unwrap()
    );
    assert_eq!(
        ibig!(+123456789012345678901234567890123456789012345678901234567890 base 16),
        IBig::from_str_radix("+123456789012345678901234567890123456789012345678901234567890", 16)
            .unwrap()
    );
    assert_eq!(
        ibig!(-123456789012345678901234567890123456789012345678901234567890 base 16),
        IBig::from_str_radix("-123456789012345678901234567890123456789012345678901234567890", 16)
            .unwrap()
    );
    assert_eq!(
        ibig!(-1234567890_1234567890_1234567890_1234567890_1234567890 base 16),
        IBig::from_str_radix("-1234567890_1234567890_1234567890_1234567890_1234567890", 16)
            .unwrap()
    );

    // other radix tests
    assert_eq!(ibig!(a3gp1 base 32), IBig::from_str_radix("a3gp1", 32).unwrap());
    assert_eq!(ibig!(+a3gp1 base 32), IBig::from_str_radix("+a3gp1", 32).unwrap());
    assert_eq!(ibig!(-a3gp1 base 32), IBig::from_str_radix("-a3gp1", 32).unwrap());
    assert_eq!(ibig!(13agp base 32), IBig::from_str_radix("13agp", 32).unwrap());
    assert_eq!(ibig!(+13agp base 32), IBig::from_str_radix("+13agp", 32).unwrap());
    assert_eq!(ibig!(-13agp base 32), IBig::from_str_radix("-13agp", 32).unwrap());

    // const test
    const _: IBig = ibig!(0);
    const _: IBig = ibig!(-1);
    const _: IBig = ibig!(-0xffffffff);
}

#[test]
fn test_static_ibig() {
    let zero: &'static IBig = static_ibig!(0);
    assert_eq!(*zero, IBig::ZERO);

    let one: &'static IBig = static_ibig!(1);
    assert_eq!(*one, IBig::ONE);

    let medium1: &'static IBig = static_ibig!(-0xfffffffffffffff);
    assert_eq!(*medium1, IBig::from(-0xfffffffffffffffi64));
    let medium2: &'static IBig = static_ibig!(-0xfffffffffffffffff);
    assert_eq!(*medium2, IBig::from(-0xfffffffffffffffffi128));

    let big: &'static IBig =
        static_ibig!(-123456789012345678901234567890123456789012345678901234567890);
    assert_eq!(
        *big,
        IBig::from_str("-123456789012345678901234567890123456789012345678901234567890").unwrap()
    );
}
