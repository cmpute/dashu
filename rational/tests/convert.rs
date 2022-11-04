use dashu_int::error::OutOfBoundsError;
use dashu_base::{Approximation::*, Sign::*};
use dashu_ratio::RBig;

mod helper_macros;

#[test]
fn test_from_f32() {
    assert_eq!(RBig::try_from(0f32), Ok(RBig::ZERO));
    assert_eq!(RBig::try_from(1f32), Ok(RBig::ONE));
    assert_eq!(RBig::try_from(-1f32), Ok(RBig::NEG_ONE));

    assert_eq!(RBig::try_from(2.25f32), Ok(rbig!(9/4)));
    assert_eq!(RBig::try_from(-2.25e4f32), Ok(rbig!(-22500)));
    assert_eq!(RBig::try_from(1.1773109e-2f32), Ok(rbig!(12345/1048576)));

    assert_eq!(RBig::try_from(f32::INFINITY), Err(OutOfBoundsError));
    assert_eq!(RBig::try_from(f32::NEG_INFINITY), Err(OutOfBoundsError));
    assert_eq!(RBig::try_from(f32::NAN), Err(OutOfBoundsError));
}

#[test]
fn test_from_f64() {
    assert_eq!(RBig::try_from(0f64), Ok(RBig::ZERO));
    assert_eq!(RBig::try_from(1f64), Ok(RBig::ONE));
    assert_eq!(RBig::try_from(-1f64), Ok(RBig::NEG_ONE));

    assert_eq!(RBig::try_from(2.25f64), Ok(rbig!(9/4)));
    assert_eq!(RBig::try_from(-2.25e4f64), Ok(rbig!(-22500)));
    assert_eq!(RBig::try_from(1.1773109436035156e-2f64), Ok(rbig!(12345/1048576)));

    assert_eq!(RBig::try_from(f64::INFINITY), Err(OutOfBoundsError));
    assert_eq!(RBig::try_from(f64::NEG_INFINITY), Err(OutOfBoundsError));
    assert_eq!(RBig::try_from(f64::NAN), Err(OutOfBoundsError));
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
    ];
    for (ratio, float) in exact_cases {
        assert_eq!(ratio.to_f32(), Exact(float));
        assert_eq!(ratio.to_f32_fast(), float);
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
    }

    // overflow and underflow
    assert_eq!(RBig::from(ubig!(1) << 200).to_f32(), Inexact(f32::INFINITY, Positive));
    assert_eq!(RBig::from(ubig!(1) << 200).to_f32_fast(), f32::INFINITY);
    assert_eq!(RBig::from(ubig!(1) << 128).to_f32(), Inexact(f32::INFINITY, Positive));
    assert_eq!(RBig::from(ubig!(1) << 128).to_f32_fast(), f32::INFINITY);
    assert_eq!(RBig::from(ibig!(-1) << 200).to_f32(), Inexact(f32::NEG_INFINITY, Negative));
    assert_eq!(RBig::from(ibig!(-1) << 200).to_f32_fast(), f32::NEG_INFINITY);
    assert_eq!(RBig::from(ibig!(-1) << 128).to_f32(), Inexact(f32::NEG_INFINITY, Negative));
    assert_eq!(RBig::from(ibig!(-1) << 128).to_f32_fast(), f32::NEG_INFINITY);
    assert_eq!(RBig::from_parts(ibig!(1), ubig!(1) << 149).to_f32(), Exact(f32::from_bits(0x1)));
    assert_eq!(RBig::from_parts(ibig!(1), ubig!(1) << 149).to_f32_fast(), f32::from_bits(0x1));
    assert_eq!(RBig::from_parts(ibig!(1), ubig!(1) << 150).to_f32(), Inexact(0f32, Negative));
    assert_eq!(RBig::from_parts(ibig!(1), ubig!(1) << 150).to_f32_fast(), 0f32);
    assert_eq!(RBig::from_parts(ibig!(-1), ubig!(1) << 149).to_f32(), Exact(-f32::from_bits(0x1)));
    assert_eq!(RBig::from_parts(ibig!(-1), ubig!(1) << 149).to_f32_fast(), -f32::from_bits(0x1));
    assert_eq!(RBig::from_parts(ibig!(-1), ubig!(1) << 150).to_f32(), Inexact(-0f32, Positive));
    assert_eq!(RBig::from_parts(ibig!(-1), ubig!(1) << 150).to_f32_fast(), -0f32);
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
    ];
    for (ratio, float) in exact_cases {
        assert_eq!(ratio.to_f64(), Exact(float));
        assert_eq!(ratio.to_f64_fast(), float);
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
    }

    // overflow and underflow
    assert_eq!(RBig::from(ubig!(1) << 2000).to_f64(), Inexact(f64::INFINITY, Positive));
    assert_eq!(RBig::from(ubig!(1) << 2000).to_f64_fast(), f64::INFINITY);
    assert_eq!(RBig::from(ubig!(1) << 1024).to_f64(), Inexact(f64::INFINITY, Positive));
    assert_eq!(RBig::from(ubig!(1) << 1024).to_f64_fast(), f64::INFINITY);
    assert_eq!(RBig::from(ibig!(-1) << 2000).to_f64(), Inexact(f64::NEG_INFINITY, Negative));
    assert_eq!(RBig::from(ibig!(-1) << 2000).to_f64_fast(), f64::NEG_INFINITY);
    assert_eq!(RBig::from(ibig!(-1) << 1024).to_f64(), Inexact(f64::NEG_INFINITY, Negative));
    assert_eq!(RBig::from(ibig!(-1) << 1024).to_f64_fast(), f64::NEG_INFINITY);
    assert_eq!(RBig::from_parts(ibig!(1), ubig!(1) << 1074).to_f64(), Exact(f64::from_bits(0x1)));
    assert_eq!(RBig::from_parts(ibig!(1), ubig!(1) << 1074).to_f64_fast(), f64::from_bits(0x1));
    assert_eq!(RBig::from_parts(ibig!(1), ubig!(1) << 1075).to_f64(), Inexact(0f64, Negative));
    assert_eq!(RBig::from_parts(ibig!(1), ubig!(1) << 1075).to_f64_fast(), 0f64);
    assert_eq!(RBig::from_parts(ibig!(-1), ubig!(1) << 1074).to_f64(), Exact(-f64::from_bits(0x1)));
    assert_eq!(RBig::from_parts(ibig!(-1), ubig!(1) << 1074).to_f64_fast(), -f64::from_bits(0x1));
    assert_eq!(RBig::from_parts(ibig!(-1), ubig!(1) << 1075).to_f64(), Inexact(-0f64, Positive));
    assert_eq!(RBig::from_parts(ibig!(-1), ubig!(1) << 1075).to_f64_fast(), -0f64);
}

