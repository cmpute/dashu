use dashu_base::{Approximation::*, ConversionError::*, Sign::*};
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

    assert_eq!(RBig::try_from(2.25f32), Ok(rbig!(9 / 4)));
    assert_eq!(RBig::try_from(-2.25e4f32), Ok(rbig!(-22500)));
    assert_eq!(RBig::try_from(1.1773109e-2f32), Ok(rbig!(12345 / 1048576)));
    assert_eq!(RBig::try_from(core::f32::consts::PI), Ok(rbig!(13176795 / 4194304)));

    assert_eq!(RBig::try_from(f32::INFINITY), Err(OutOfBounds));
    assert_eq!(RBig::try_from(f32::NEG_INFINITY), Err(OutOfBounds));
    assert_eq!(RBig::try_from(f32::NAN), Err(OutOfBounds));
}

#[test]
fn test_from_f64() {
    assert_eq!(RBig::try_from(0f64), Ok(RBig::ZERO));
    assert_eq!(RBig::try_from(1f64), Ok(RBig::ONE));
    assert_eq!(RBig::try_from(-1f64), Ok(RBig::NEG_ONE));

    assert_eq!(RBig::try_from(2.25f64), Ok(rbig!(9 / 4)));
    assert_eq!(RBig::try_from(-2.25e4f64), Ok(rbig!(-22500)));
    assert_eq!(RBig::try_from(1.1773109436035156e-2f64), Ok(rbig!(12345 / 1048576)));
    assert_eq!(
        RBig::try_from(core::f64::consts::PI),
        Ok(rbig!(884279719003555 / 281474976710656))
    );

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
        (rbig!(-1 / 4), -0.25f32),
        (rbig!(-3 / 4), -0.75f32),
        (RBig::from_parts(ibig!(1), ubig!(1) << 149), f32::from_bits(0x1)),
        (RBig::from_parts(ibig!(-1), ubig!(1) << 149), -f32::from_bits(0x1)),
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
        (rbig!(-1 / 4), -0.25f64),
        (rbig!(-3 / 4), -0.75f64),
        (RBig::from_parts(ibig!(1), ubig!(1) << 1074), f64::from_bits(0x1)),
        (RBig::from_parts(ibig!(-1), ubig!(1) << 1074), -f64::from_bits(0x1)),
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

#[test]
fn test_float_try_from_large_values() {
    use dashu_ratio::Relaxed;

    // every finite float converts to a rational and back, including the integers that
    // do not fit in the mantissa type of the float encoding (2^31 for f32, 2^63 for f64)
    let f64_cases = [
        2f64.powi(62),
        2f64.powi(63),
        -(2f64.powi(63)),
        1e20,
        2f64.powi(100),
        1.5e300,
        f64::MAX,
        f64::MIN,
        f64::MIN_POSITIVE,
        f64::from_bits(1),
        -f64::from_bits(0x000f_ffff_ffff_ffff),
        core::f64::consts::PI,
    ];
    for x in f64_cases {
        assert_eq!(f64::try_from(RBig::try_from(x).unwrap()), Ok(x), "{x:e}");
        assert_eq!(f64::try_from(Relaxed::try_from(x).unwrap()), Ok(x), "{x:e}");
    }
    let f32_cases = [
        2f32.powi(30),
        2f32.powi(31),
        -(2f32.powi(31)),
        3e9,
        2f32.powi(100),
        f32::MAX,
        f32::MIN,
        f32::MIN_POSITIVE,
        f32::from_bits(1),
        -f32::from_bits(0x007f_ffff),
        core::f32::consts::PI,
    ];
    for x in f32_cases {
        assert_eq!(f32::try_from(RBig::try_from(x).unwrap()), Ok(x), "{x:e}");
        assert_eq!(f32::try_from(Relaxed::try_from(x).unwrap()), Ok(x), "{x:e}");
    }

    // integers that need more bits than the float has
    assert_eq!(f32::try_from(rbig!(0x1000001)), Err(LossOfPrecision)); // 2^24 + 1
    assert_eq!(f32::try_from(rbig!(0x80000080)), Err(LossOfPrecision)); // 2^31 + 2^7
    assert_eq!(f32::try_from(rbig!(0x80000100)), Ok(2147483904.0)); // 2^31 + 2^8
    assert_eq!(f64::try_from(rbig!(0x20000000000001)), Err(LossOfPrecision)); // 2^53 + 1
    assert_eq!(f64::try_from(rbig!(0x8000000000000400)), Err(LossOfPrecision)); // 2^63 + 2^10
    assert_eq!(f64::try_from(rbig!(0x8000000000000800)), Ok(9223372036854777856.0));
    // beyond the largest finite value
    assert_eq!(f32::try_from(RBig::from(ubig!(1) << 128)), Err(OutOfBounds));
    assert_eq!(f64::try_from(RBig::from(ibig!(-1) << 1024)), Err(OutOfBounds));
    // a Relaxed with a common odd factor is converted by its value
    assert_eq!(f64::try_from(rbig!(~9/6)), Ok(1.5));
    assert_eq!(f32::try_from(rbig!(~-9/6)), Ok(-1.5));
    assert_eq!(f64::try_from(rbig!(~1/3)), Err(LossOfPrecision));
}

#[test]
fn test_relaxed_to_integers() {
    use dashu_int::{IBig, UBig};
    use dashu_ratio::Relaxed;

    // the value is an integer although the denominator is not one
    let one = Relaxed::from_parts(3.into(), 3u8.into());
    assert_eq!(one.denominator(), &UBig::from(3u8));
    assert_eq!(IBig::try_from(one.clone()), Ok(IBig::ONE));
    assert_eq!(UBig::try_from(one.clone()), Ok(UBig::ONE));
    assert_eq!(i64::try_from(one.clone()), Ok(1));
    assert_eq!(u8::try_from(one), Ok(1));

    let product = rbig!(~2/3) * rbig!(~3);
    assert_eq!(IBig::try_from(product), Ok(IBig::from(2)));
    assert_eq!(i8::try_from(rbig!(~-35/7)), Ok(-5));
    assert_eq!(IBig::try_from(rbig!(~4/2)), Ok(IBig::from(2)));

    // failures keep their meaning
    assert_eq!(IBig::try_from(rbig!(~10/6)), Err(LossOfPrecision));
    assert_eq!(u32::try_from(rbig!(~10/6)), Err(LossOfPrecision));
    assert_eq!(UBig::try_from(rbig!(~-9/3)), Err(OutOfBounds));
    assert_eq!(u8::try_from(rbig!(~-9/3)), Err(OutOfBounds));
    assert_eq!(u8::try_from(rbig!(~900/3)), Err(OutOfBounds));
}

/// Number of representable floats between two floats of the same sign.
fn ulps_apart_f64(a: f64, b: f64) -> u64 {
    (a.to_bits() as i64 - b.to_bits() as i64).unsigned_abs()
}

fn ulps_apart_f32(a: f32, b: f32) -> u32 {
    (a.to_bits() as i32 - b.to_bits() as i32).unsigned_abs()
}

#[test]
fn test_to_float_fast_accuracy() {
    use dashu_base::BitTest;
    use dashu_int::{IBig, UBig};
    use dashu_ratio::Relaxed;

    // the fast conversions are documented to be at most one unit in the last place
    // away from the correctly rounded result
    let two64 = ubig!(1) << 64;
    let f64_cases = [
        RBig::from_parts(7.into(), &two64 + ubig!(23482)),
        RBig::from_parts(1.into(), &two64 * ubig!(4) + ubig!(12289)),
        RBig::from_parts(22.into(), 7u8.into()),
        RBig::from_parts((-7).into(), &two64 + ubig!(23482)),
    ];
    for r in f64_cases {
        let (fast, exact) = (r.to_f64_fast(), r.to_f64().value());
        assert!(ulps_apart_f64(fast, exact) <= 1, "{r}: {fast:e} vs {exact:e}");
    }
    for (n, d) in [
        (1u64, 74559079u64),
        (1, 74559080),
        (33107751, 2152003815),
        (22, 7),
    ] {
        let r = RBig::from_parts(n.into(), d.into());
        let x = Relaxed::from_parts(n.into(), d.into());
        let exact = r.to_f32().value();
        assert!(ulps_apart_f32(r.to_f32_fast(), exact) <= 1, "{n}/{d}");
        assert!(ulps_apart_f32(x.to_f32_fast(), exact) <= 1, "{n}/{d}");
        assert!(ulps_apart_f32((-r).to_f32_fast(), -exact) <= 1, "-{n}/{d}");
    }

    // pseudo random operands of various sizes (xorshift, fixed seed)
    let mut state = 0x9e3779b97f4a7c15u64;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    let mut random_bits = |bits: usize| {
        let mut n = UBig::ZERO;
        for _ in 0..bits / 64 + 1 {
            n = (n << 64) | UBig::from(next());
        }
        // sometimes produce long runs of ones or zeros, which sit close to rounding boundaries
        match next() % 4 {
            0 => n |= (UBig::ONE << (bits / 2)) - UBig::ONE,
            1 => n = (n >> (bits / 2)) << (bits / 2),
            _ => {}
        }
        n |= UBig::ONE << (bits + 64);
        (&n >> (n.bit_len() - bits)) | UBig::ONE
    };
    let sizes = [
        1usize, 2, 24, 31, 32, 33, 53, 63, 64, 65, 100, 127, 128, 129, 200, 400,
    ];
    let (mut differ64, mut differ32, mut total) = (0, 0, 0);
    for num_bits in sizes {
        for den_bits in sizes {
            for _ in 0..8 {
                let r =
                    Relaxed::from_parts(IBig::from(random_bits(num_bits)), random_bits(den_bits));
                let (fast, exact) = (r.to_f64_fast(), r.to_f64().value());
                assert!(ulps_apart_f64(fast, exact) <= 1, "{r}: {fast:e} vs {exact:e}");
                differ64 += (fast != exact) as usize;
                let (fast, exact) = (r.to_f32_fast(), r.to_f32().value());
                assert!(ulps_apart_f32(fast, exact) <= 1, "{r}: {fast:e} vs {exact:e}");
                differ32 += (fast != exact) as usize;
                total += 1;
            }
        }
    }
    // the fast result is almost always the correctly rounded one
    assert!(
        differ64 * 50 <= total && differ32 * 50 <= total,
        "{differ64} {differ32} {total}"
    );

    // around the limits of the float range, and close to the subnormals
    let shifts = [
        (1023usize, 0usize),
        (1024, 0),
        (1025, 0),
        (2000, 0),
        (0, 1021),
        (0, 1022),
        (0, 1050),
        (0, 1073),
        (0, 1074),
        (0, 1075),
        (0, 1076),
        (0, 3000),
        (127, 0),
        (128, 0),
        (0, 126),
        (0, 148),
        (0, 149),
        (0, 150),
        (0, 151),
    ];
    let scales = [
        (1u32, 1u32),
        (3, 1),
        (5, 3),
        (0xffffff, 0x1000000),
        (0x1000001, 0x1000000),
    ];
    for (num_shift, den_shift) in shifts {
        for (n, d) in scales {
            for negative in [false, true] {
                let num = IBig::from(n) << num_shift;
                let num = if negative { -num } else { num };
                let r = RBig::from_parts(num, UBig::from(d) << den_shift);
                let (fast, exact) = (r.to_f64_fast(), r.to_f64().value());
                assert!(ulps_apart_f64(fast, exact) <= 1, "{r}: {fast:e} vs {exact:e}");
                assert_eq!(fast.is_sign_negative(), negative, "{r}");
                let (fast, exact) = (r.to_f32_fast(), r.to_f32().value());
                assert!(ulps_apart_f32(fast, exact) <= 1, "{r}: {fast:e} vs {exact:e}");
                assert_eq!(fast.is_sign_negative(), negative, "{r}");
            }
        }
    }
}
