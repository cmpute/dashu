// Tests for the sign related traits on primitive types.

use core::cmp::Ordering;
use dashu_base::{Abs, AbsOrd};

#[test]
fn test_abs_cmp_signed_min() {
    // the magnitude of iN::MIN doesn't fit in iN
    assert_eq!(0i8.abs_cmp(&-127), Ordering::Less);
    assert_eq!(0i8.abs_cmp(&i8::MIN), Ordering::Less);
    assert_eq!(i8::MIN.abs_cmp(&1), Ordering::Greater);
    assert_eq!(i8::MIN.abs_cmp(&i8::MAX), Ordering::Greater);
    assert_eq!(i8::MAX.abs_cmp(&i8::MIN), Ordering::Less);
    assert_eq!(i8::MIN.abs_cmp(&i8::MIN), Ordering::Equal);
    assert_eq!(i16::MIN.abs_cmp(&i16::MAX), Ordering::Greater);
    assert_eq!(i32::MIN.abs_cmp(&0), Ordering::Greater);
    assert_eq!(i64::MIN.abs_cmp(&0), Ordering::Greater);
    assert_eq!(i128::MIN.abs_cmp(&-1), Ordering::Greater);
    assert_eq!(isize::MIN.abs_cmp(&isize::MAX), Ordering::Greater);

    // exhaustive check on i8
    for a in i8::MIN..=i8::MAX {
        for b in i8::MIN..=i8::MAX {
            assert_eq!(a.abs_cmp(&b), (a as i16).abs().cmp(&(b as i16).abs()));
        }
    }
}

#[test]
fn test_float_abs_negative_zero() {
    assert_eq!(Abs::abs(-0.0f64).to_bits(), 0.0f64.to_bits());
    assert_eq!(Abs::abs(-0.0f32).to_bits(), 0.0f32.to_bits());
    assert_eq!(Abs::abs(0.0f64).to_bits(), 0.0f64.to_bits());
    assert_eq!(Abs::abs(-1.5f64), 1.5);
    assert_eq!(Abs::abs(1.5f32), 1.5);
    assert_eq!(Abs::abs(f64::NEG_INFINITY), f64::INFINITY);
    assert_eq!(Abs::abs(-f32::from_bits(1)), f32::from_bits(1));
    assert!(Abs::abs(f64::NAN).is_nan());
    assert!(Abs::abs(-f32::NAN).is_nan());
    assert_eq!((-0.0f64).abs_cmp(&0.0), Ordering::Equal);
}
