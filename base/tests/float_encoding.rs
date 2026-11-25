//! Regression and oracle tests for [FloatEncoding::encode].

use core::cmp::Ordering;
use dashu_base::{
    Approximation::{self, *},
    FloatEncoding,
    Sign::{self, *},
};

/// Exact reference for `f32::encode`, computed through f64 arithmetic: every input
/// used with this oracle is exactly representable as a f64, and the conversion from
/// f64 to f32 rounds to nearest, ties to even.
fn f32_reference(man: i32, exp: i16) -> Approximation<f32, Sign> {
    let exact = man as f64 * 2f64.powi(exp as i32);
    assert!(exact.is_finite() && exact.abs() >= f64::MIN_POSITIVE);
    let f = exact as f32;
    match (f as f64).partial_cmp(&exact).unwrap() {
        Ordering::Equal => Exact(f),
        Ordering::Greater => Inexact(f, Positive),
        Ordering::Less => Inexact(f, Negative),
    }
}

fn assert_same_f32(got: Approximation<f32, Sign>, want: Approximation<f32, Sign>, ctx: &str) {
    let same = match (got, want) {
        (Exact(a), Exact(b)) => a.to_bits() == b.to_bits(),
        (Inexact(a, sa), Inexact(b, sb)) => a.to_bits() == b.to_bits() && sa == sb,
        _ => false,
    };
    assert!(same, "{}: got {:?}, expected {:?}", ctx, got, want);
}

#[test]
fn test_encode_sticky_bits() {
    // 2^25 + 1 and 2^54 + 1 are not representable: they round down, inexactly
    assert_eq!(f32::encode(33554433, 0), Inexact(33554432f32, Negative));
    assert_eq!(f64::encode(18014398509481985, 0), Inexact(18014398509481984f64, Negative));
    // these lie above the midpoint, so they must round up
    assert_eq!(f32::encode(33565715, 0), Inexact(33565716f32, Positive));
    assert_eq!(f32::encode(-33565715, 0), Inexact(-33565716f32, Negative));
    assert_eq!(f64::encode(18014398509481987, 0), Inexact(18014398509481988f64, Positive));
    assert_eq!(f64::encode(-18014398509481987, 0), Inexact(-18014398509481988f64, Negative));
    // neighbours that are representable stay exact
    assert_eq!(f32::encode(33554436, 0), Exact(33554436f32));
    assert_eq!(f64::encode(18014398509481988, 0), Exact(18014398509481988f64));
    // a sticky bit right below the round bit of a full-width mantissa
    assert_eq!(
        f32::encode((1 << 30) | (1 << 6) | (1 << 5), 0),
        Inexact(1073741952f32, Positive)
    );
    assert_eq!(f32::encode((1 << 30) | (1 << 5), 0), Inexact(1073741824f32, Negative));
    assert_eq!(
        f64::encode((1 << 62) | (1 << 9) | (1 << 8), 0),
        Inexact(4611686018427388928f64, Positive)
    );
    assert_eq!(f64::encode((1 << 62) | (1 << 8), 0), Inexact(4611686018427387904f64, Negative));
}

#[test]
fn test_encode_near_smallest_subnormal() {
    // exact ties: round to even
    assert_eq!(f32::encode(1, -150), Inexact(0f32, Negative));
    assert_eq!(f32::encode(3, -150), Inexact(f32::from_bits(2), Positive));
    assert_eq!(f32::encode(22, -151), Inexact(f32::from_bits(6), Positive));
    assert_eq!(f64::encode(1, -1075), Inexact(0f64, Negative));
    assert_eq!(f64::encode(3, -1075), Inexact(f64::from_bits(2), Positive));
    // 0.75 * smallest subnormal rounds up to the smallest subnormal
    assert_eq!(f32::encode(3, -151), Inexact(f32::from_bits(1), Positive));
    assert_eq!(f32::encode(-3, -151), Inexact(-f32::from_bits(1), Negative));
    assert_eq!(f64::encode(3, -1076), Inexact(f64::from_bits(1), Positive));
    assert_eq!(f64::encode(-3, -1076), Inexact(-f64::from_bits(1), Negative));
    // 2.75 * smallest subnormal rounds up to 3 * smallest subnormal
    assert_eq!(f32::encode(22, -152), Inexact(f32::from_bits(3), Positive));
    assert_eq!(f64::encode(22, -1077), Inexact(f64::from_bits(3), Positive));
    // 1.25 * smallest subnormal is not exact
    assert_eq!(f32::encode(5, -151), Inexact(f32::from_bits(1), Negative));
    assert_eq!(f64::encode(5, -1076), Inexact(f64::from_bits(1), Negative));
    // slightly above, and exactly at, half of the smallest subnormal
    assert_eq!(f32::encode(i32::MAX, -181), Inexact(0f32, Negative));
    assert_eq!(f32::encode((1 << 30) + 1, -180), Inexact(f32::from_bits(1), Positive));
    assert_eq!(f32::encode(1 << 30, -180), Inexact(0f32, Negative));
    assert_eq!(f32::encode(i32::MIN, -181), Inexact(-0f32, Positive));
    assert_eq!(f64::encode((1 << 62) + 1, -1137), Inexact(f64::from_bits(1), Positive));
    assert_eq!(f64::encode(1 << 62, -1137), Inexact(0f64, Negative));
    // far below the smallest subnormal
    assert_eq!(f32::encode(i32::MAX, i16::MIN), Inexact(0f32, Negative));
    assert_eq!(f64::encode(i64::MIN, i16::MIN), Inexact(-0f64, Positive));
    assert!(f64::encode(i64::MIN, i16::MIN).value().is_sign_negative());
}

#[test]
fn test_encode_min_mantissa() {
    // -2^31 * 2^-180 = -2^-149 and -2^63 * 2^-1137 = -2^-1074 exactly
    assert_eq!(f32::encode(i32::MIN, -180), Exact(-f32::from_bits(1)));
    assert_eq!(f64::encode(i64::MIN, -1137), Exact(-f64::from_bits(1)));
    assert_eq!(f32::encode(-1, -149), Exact(-f32::from_bits(1)));
    assert_eq!(f64::encode(-1, -1074), Exact(-f64::from_bits(1)));
    assert_eq!(f32::encode(i32::MAX, -179), Inexact(f32::from_bits(2), Positive));
    assert_eq!(f64::encode(i64::MAX, -1136), Inexact(f64::from_bits(2), Positive));
    assert_eq!(f32::encode(i32::MIN, 0), Exact(-2147483648f32));
    assert_eq!(f64::encode(i64::MIN, 0), Exact(-9223372036854775808f64));
    assert_eq!(f32::encode(i32::MIN, 96), Exact(-(2f32.powi(127))));
    assert_eq!(f32::encode(i32::MIN, 97), Inexact(f32::NEG_INFINITY, Negative));
    // rounding up to infinity at the top of the range
    assert_eq!(f32::encode(i32::MAX, 97), Inexact(f32::INFINITY, Positive));
    assert_eq!(f32::encode(0xffffff, 104), Exact(f32::MAX));
    assert_eq!(f64::encode(i64::MAX, 961), Inexact(f64::INFINITY, Positive));
}

#[test]
fn test_encode_f32_against_f64_oracle() {
    // mantissas around every rounding boundary of interest
    let mut mantissas = vec![
        1i32, 2, 3, 5, 7, 22, 0xffffff, 0x1000001, 0x1ffffff, 33565715,
    ];
    for bits in [24, 25, 26, 27, 30] {
        for delta in -4i32..=4 {
            mantissas.push((1 << bits) + delta);
            mantissas.push((3 << (bits - 1)) + delta);
        }
    }
    for delta in 0..=8 {
        mantissas.push(i32::MAX - delta);
        mantissas.push(i32::MIN + delta);
    }
    // a simple LCG for some more arbitrary mantissas
    let mut state = 0x9e3779b9u32;
    for _ in 0..200 {
        state = state.wrapping_mul(1664525).wrapping_add(1013904223);
        mantissas.push(state as i32);
        mantissas.push((state >> 5) as i32);
    }

    // exponents covering the subnormal range, the normal range and the overflow threshold
    for &man in &mantissas {
        for exp in (-185i16..=-110).chain(-40..=40).chain(90..=100) {
            for m in [man, man.wrapping_neg()] {
                let top = 32 - m.unsigned_abs().leading_zeros() as i32 + exp as i32;
                if m == 0 || top > 128 {
                    continue;
                }
                let ctx = format!("f32::encode({}, {})", m, exp);
                assert_same_f32(f32::encode(m, exp), f32_reference(m, exp), &ctx);
            }
        }
    }
}

#[test]
fn test_encode_f64_matches_integer_cast() {
    // for exponent 0 the cast from i64 is a correctly rounded reference
    let mut mantissas = vec![1i64, 3, i64::MAX, i64::MIN, i64::MIN + 1];
    for bits in [53, 54, 55, 56, 60, 62] {
        for delta in -9i64..=9 {
            mantissas.push((1 << bits) + delta);
            mantissas.push((3 << (bits - 1)) + delta);
            mantissas.push(-(1 << bits) + delta);
        }
    }
    for man in mantissas {
        let f = man as f64;
        let want = match (f as i128).cmp(&(man as i128)) {
            Ordering::Equal => Exact(f),
            Ordering::Greater => Inexact(f, Positive),
            Ordering::Less => Inexact(f, Negative),
        };
        assert_eq!(f64::encode(man, 0), want, "f64::encode({}, 0)", man);
    }
}
