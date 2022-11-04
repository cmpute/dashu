use dashu_base::{Approximation::*, Sign::*, ConversionError::*};
use dashu_ratio::RBig;

mod helper_macros;

#[test]
fn test_from_integers() {
    assert_eq!(RBig::from(0u8), rbig!(0));
    assert_eq!(RBig::from(1u8), rbig!(1));
    assert_eq!(RBig::from(u8::MAX), rbig!(0xff));
    assert_eq!(RBig::from(u16::MAX), rbig!(0xffff));
    assert_eq!(RBig::from(u32::MAX), rbig!(0xffffffff));
    assert_eq!(RBig::from(u64::MAX), rbig!(0xffffffffffffffff));
    assert_eq!(RBig::from(u128::MAX), rbig!(0xffffffffffffffffffffffffffffffff));
    assert_eq!(RBig::from(0i8), rbig!(0));
    assert_eq!(RBig::from(-1i8), rbig!(-1));
    assert_eq!(RBig::from(i8::MIN), rbig!(-0x80));
    assert_eq!(RBig::from(i16::MIN), rbig!(-0x8000));
    assert_eq!(RBig::from(i32::MIN), rbig!(-0x80000000));
    assert_eq!(RBig::from(i64::MIN), rbig!(-0x8000000000000000));
    assert_eq!(RBig::from(i128::MIN), rbig!(-0x80000000000000000000000000000000));
}

#[test]
fn test_to_integers() {
    assert_eq!(u8::try_from(rbig!(0)), Ok(0));
    assert_eq!(u8::try_from(rbig!(1)), Ok(1));
    assert_eq!(u8::try_from(rbig!(0xff)), Ok(u8::MAX));
    assert_eq!(u16::try_from(rbig!(0xffff)), Ok(u16::MAX));
    assert_eq!(u32::try_from(rbig!(0xffffffff)), Ok(u32::MAX));
    assert_eq!(u64::try_from(rbig!(0xffffffffffffffff)), Ok(u64::MAX));
    assert_eq!(u128::try_from(rbig!(0xffffffffffffffffffffffffffffffff)), Ok(u128::MAX));
    assert_eq!(i8::try_from(rbig!(0)), Ok(0));
    assert_eq!(i8::try_from(rbig!(-1)), Ok(-1));
    assert_eq!(i8::try_from(rbig!(-0x80)), Ok(i8::MIN));
    assert_eq!(i16::try_from(rbig!(-0x8000)), Ok(i16::MIN));
    assert_eq!(i32::try_from(rbig!(-0x80000000)), Ok(i32::MIN));
    assert_eq!(i64::try_from(rbig!(-0x8000000000000000)), Ok(i64::MIN));
    assert_eq!(i128::try_from(rbig!(-0x80000000000000000000000000000000)), Ok(i128::MIN));
    
    assert_eq!(u8::try_from(rbig!(0x100)), Err(OutOfBounds));
    assert_eq!(u8::try_from(rbig!(-1)), Err(OutOfBounds));
    assert_eq!(i8::try_from(rbig!(-0x81)), Err(OutOfBounds));
}

#[test]
fn test_from_f32() {
    assert_eq!(RBig::try_from(0f32), Ok(RBig::ZERO));
    assert_eq!(RBig::try_from(1f32), Ok(RBig::ONE));
    assert_eq!(RBig::try_from(-1f32), Ok(RBig::NEG_ONE));

    assert_eq!(RBig::try_from(2.25f32), Ok(rbig!(9/4)));
    assert_eq!(RBig::try_from(-2.25e4f32), Ok(rbig!(-22500)));
    assert_eq!(RBig::try_from(1.1773109e-2f32), Ok(rbig!(12345/1048576)));

    assert_eq!(RBig::try_from(f32::INFINITY), Err(OutOfBounds));
    assert_eq!(RBig::try_from(f32::NEG_INFINITY), Err(OutOfBounds));
    assert_eq!(RBig::try_from(f32::NAN), Err(OutOfBounds));
}

#[test]
fn test_from_f64() {
    assert_eq!(RBig::try_from(0f64), Ok(RBig::ZERO));
    assert_eq!(RBig::try_from(1f64), Ok(RBig::ONE));
    assert_eq!(RBig::try_from(-1f64), Ok(RBig::NEG_ONE));

    assert_eq!(RBig::try_from(2.25f64), Ok(rbig!(9/4)));
    assert_eq!(RBig::try_from(-2.25e4f64), Ok(rbig!(-22500)));
    assert_eq!(RBig::try_from(1.1773109436035156e-2f64), Ok(rbig!(12345/1048576)));

    assert_eq!(RBig::try_from(f64::INFINITY), Err(OutOfBounds));
    assert_eq!(RBig::try_from(f64::NEG_INFINITY), Err(OutOfBounds));
    assert_eq!(RBig::try_from(f64::NAN), Err(OutOfBounds));
}

#[test]
fn test_to_f32() {
    // exact cases
    let exact_cases = [
        (rbig!(0), 0f32),
        (rbig!(1), 1f32),
        (rbig!(-1), -1f32),
        (rbig!(4), 4f32),
        (rbig!(-1/4), -0.25f32),
        (rbig!(-3/4), -0.75f32),
        (RBig::from_parts(ibig!(1), ubig!(1) << 149), f32::from_bits(0x1)),
        (RBig::from_parts(ibig!(-1), ubig!(1) << 149), -f32::from_bits(0x1))
    ];
    for (ratio, float) in exact_cases {
        assert_eq!(ratio.to_f32(), Exact(float));
        assert_eq!(ratio.to_f32_fast(), float);
        assert_eq!(f32::try_from(ratio), Ok(float));
    }

    // inexact cases
    let inexact_cases = [
        // (numerator, denominator, rounding)
        // NOTE: make sure each of these numbers fit in a f32
        (1i32, 3u32, Positive),
        (-2, 3, Negative),
        (-1, 7, Negative),
        (7, 11, Negative),
        (-123456789, 987654321, Negative),
        (987654321, 123456789, Negative),
    ];
    for (num, den, rnd) in inexact_cases {
        let ratio = RBig::from_parts(num.into(), den.into());
        let expected = num as f32 / den as f32;
        assert_eq!(ratio.to_f32(), Inexact(expected, rnd));
        assert_eq!(ratio.to_f32_fast(), expected);
        assert_eq!(f32::try_from(ratio), Err(LossOfPrecision));
    }

    // overflow and underflow
    let special_cases = [
        (RBig::from(ubig!(1) << 200), f32::INFINITY, Positive),
        (RBig::from(ubig!(1) << 128), f32::INFINITY, Positive),
        (RBig::from(ibig!(-1) << 200), f32::NEG_INFINITY, Negative),
        (RBig::from(ibig!(-1) << 128), f32::NEG_INFINITY, Negative),
        (RBig::from_parts(ibig!(1), ubig!(1) << 150), 0f32, Negative),
        (RBig::from_parts(ibig!(-1), ubig!(1) << 150), -0f32, Positive),
    ];
    for (ratio, float, rnd) in special_cases {
        assert_eq!(ratio.to_f32(), Inexact(float, rnd));
        assert_eq!(ratio.to_f32_fast(), float);
        if float.is_infinite() {
            assert_eq!(f32::try_from(ratio), Err(OutOfBounds));
        } else {
            assert_eq!(f32::try_from(ratio), Err(LossOfPrecision));
        }
    }
}

#[test]
fn test_to_f64() {
    // exact cases
    let exact_cases = [
        (rbig!(0), 0f64),
        (rbig!(1), 1f64),
        (rbig!(-1), -1f64),
        (rbig!(4), 4f64),
        (rbig!(-1/4), -0.25f64),
        (rbig!(-3/4), -0.75f64),
        (RBig::from_parts(ibig!(1), ubig!(1) << 1074), f64::from_bits(0x1)),
        (RBig::from_parts(ibig!(-1), ubig!(1) << 1074), -f64::from_bits(0x1))
    ];
    for (ratio, float) in exact_cases {
        assert_eq!(ratio.to_f64(), Exact(float));
        assert_eq!(ratio.to_f64_fast(), float);
        assert_eq!(f64::try_from(ratio), Ok(float));
    }

    // inexact cases
    let inexact_cases = [
        // (numerator, denominator, rounding)
        // NOTE: make sure each of these numbers fit in a f64
        (1i64, 3u64, Negative),
        (-2, 3, Positive),
        (-1, 7, Positive),
        (7, 11, Negative),
        (-1234567890123456, 987654321098765, Positive),
        (987654321098765, 1234567890123456, Negative),
    ];
    for (num, den, rnd) in inexact_cases {
        let ratio = RBig::from_parts(num.into(), den.into());
        let expected = num as f64 / den as f64;
        assert_eq!(ratio.to_f64(), Inexact(expected, rnd));
        assert_eq!(ratio.to_f64_fast(), expected);
        assert_eq!(f64::try_from(ratio), Err(LossOfPrecision));
    }
    
    // overflow and underflow
    let special_cases = [
        (RBig::from(ubig!(1) << 2000), f32::INFINITY, Positive),
        (RBig::from(ubig!(1) << 1024), f32::INFINITY, Positive),
        (RBig::from(ibig!(-1) << 2000), f32::NEG_INFINITY, Negative),
        (RBig::from(ibig!(-1) << 1024), f32::NEG_INFINITY, Negative),
        (RBig::from_parts(ibig!(1), ubig!(1) << 1075), 0f32, Negative),
        (RBig::from_parts(ibig!(-1), ubig!(1) << 1075), -0f32, Positive),
    ];
    for (ratio, float, rnd) in special_cases {
        assert_eq!(ratio.to_f32(), Inexact(float, rnd));
        assert_eq!(ratio.to_f32_fast(), float);
        if float.is_infinite() {
            assert_eq!(f32::try_from(ratio), Err(OutOfBounds));
        } else {
            assert_eq!(f32::try_from(ratio), Err(LossOfPrecision));
        }
    }
}

