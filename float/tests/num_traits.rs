//! Tests for the `num-traits` trait implementations.

use core::str::FromStr;
use dashu_float::{
    round::mode::{HalfEven, Zero},
    DBig, FBig,
};
use num_traits_v02::{FromPrimitive, Signed, ToPrimitive};

// `Signed::abs_sub` is the *positive difference*: zero when `self <= other`. It used to
// return `|self - other|`.
#[test]
fn test_abs_sub() {
    let d = |s: &str| DBig::from_str(s).unwrap();
    assert_eq!(Signed::abs_sub(&d("3"), &d("1")), d("2"));
    assert_eq!(Signed::abs_sub(&d("2"), &d("2")), DBig::ZERO);
    assert_eq!(Signed::abs_sub(&d("1"), &d("3")), DBig::ZERO);
    assert_eq!(Signed::abs_sub(&d("-5"), &d("2")), DBig::ZERO);
    assert_eq!(Signed::abs_sub(&d("2"), &d("-5")), d("7"));
    assert_eq!(Signed::abs_sub(&d("-2"), &d("-5")), d("3"));
    assert_eq!(Signed::abs_sub(&d("-5"), &d("-2")), DBig::ZERO);
    assert_eq!(Signed::abs_sub(&d("1.5"), &d("0.25")), d("1.25"));
    assert_eq!(Signed::abs_sub(&d("0.25"), &d("1.5")), DBig::ZERO);
    assert_eq!(Signed::abs_sub(&d("0"), &d("-0")), DBig::ZERO);

    let (a, b): (FBig<Zero, 2>, FBig<Zero, 2>) = (FBig::from(1), FBig::from(3));
    assert_eq!(Signed::abs_sub(&a, &b), FBig::<Zero, 2>::ZERO);
    assert_eq!(Signed::abs_sub(&b, &a), FBig::<Zero, 2>::from(2));

    // the same rule as the primitive floats, at the four standard precisions
    for precision in [20usize, 50, 100, 500] {
        for (x, y) in [
            (1.0f64, 3.0),
            (3.0, 1.0),
            (-5.0, 2.0),
            (2.5, -0.125),
            (-1.0, -1.0),
        ] {
            let fx = FBig::<HalfEven, 2>::try_from(x)
                .unwrap()
                .with_precision(precision)
                .value();
            let fy = FBig::<HalfEven, 2>::try_from(y)
                .unwrap()
                .with_precision(precision)
                .value();
            let expected = FBig::<HalfEven, 2>::try_from(Signed::abs_sub(&x, &y)).unwrap();
            assert_eq!(Signed::abs_sub(&fx, &fy), expected, "abs_sub({x}, {y})");
        }
    }

    // infinities
    assert_eq!(Signed::abs_sub(&DBig::INFINITY, &d("1")), DBig::INFINITY);
    assert_eq!(Signed::abs_sub(&d("1"), &DBig::NEG_INFINITY), DBig::INFINITY);
    assert_eq!(Signed::abs_sub(&d("1"), &DBig::INFINITY), DBig::ZERO);
    assert_eq!(Signed::abs_sub(&DBig::NEG_INFINITY, &d("1")), DBig::ZERO);
    assert_eq!(Signed::abs_sub(&DBig::INFINITY, &DBig::INFINITY), DBig::ZERO);
}

// The integer `ToPrimitive` conversions return `None` for a value that does not fit. The
// infinities used to panic (in `to_int`), and a finite value with a huge exponent was written
// out as an integer first.
#[test]
fn test_to_primitive_out_of_range() {
    fn all_none<T: ToPrimitive>(x: &T) {
        assert_eq!(x.to_i8(), None);
        assert_eq!(x.to_i16(), None);
        assert_eq!(x.to_i32(), None);
        assert_eq!(x.to_i64(), None);
        assert_eq!(x.to_i128(), None);
        assert_eq!(x.to_isize(), None);
        assert_eq!(x.to_u8(), None);
        assert_eq!(x.to_u16(), None);
        assert_eq!(x.to_u32(), None);
        assert_eq!(x.to_u64(), None);
        assert_eq!(x.to_u128(), None);
        assert_eq!(x.to_usize(), None);
    }
    all_none(&FBig::<Zero, 2>::INFINITY);
    all_none(&FBig::<Zero, 2>::NEG_INFINITY);
    all_none(&DBig::INFINITY);
    all_none(&DBig::NEG_INFINITY);
    all_none(&FBig::<Zero, 2>::from_f64(f64::INFINITY).unwrap());
    all_none(&FBig::<Zero, 2>::from_f64(1e300).unwrap());
    all_none(&DBig::from_str("-1e300").unwrap());
    all_none(&DBig::from_parts(1.into(), isize::MAX / 8));
    all_none(&FBig::<Zero, 2>::from_parts((-1).into(), 1 << 50));
    // the infinities still convert to the float types
    assert_eq!(ToPrimitive::to_f64(&DBig::INFINITY), Some(f64::INFINITY));
    assert_eq!(ToPrimitive::to_f32(&DBig::NEG_INFINITY), Some(f32::NEG_INFINITY));

    // in-range values, including the extremes of the widest types
    let d = |s: &str| DBig::from_str(s).unwrap();
    assert_eq!(d("127").to_i8(), Some(127));
    assert_eq!(d("128").to_i8(), None);
    assert_eq!(d("-128").to_i8(), Some(-128));
    assert_eq!(d("255").to_u8(), Some(255));
    assert_eq!(d("-1").to_u8(), None);
    assert_eq!(d("340282366920938463463374607431768211455").to_u128(), Some(u128::MAX));
    assert_eq!(d("340282366920938463463374607431768211456").to_u128(), None);
    assert_eq!(d("-170141183460469231731687303715884105728").to_i128(), Some(i128::MIN));
    assert_eq!(FBig::<Zero, 2>::from_parts(1.into(), 127).to_u128(), Some(1 << 127));
    assert_eq!(FBig::<Zero, 2>::from_parts(1.into(), 128).to_u128(), None);
    assert_eq!(d("-0").to_u8(), Some(0));
    assert_eq!(d("1e-400").to_i32(), Some(0));
}

// TODO: cover `from_f32`/`from_f64` of the smallest subnormals (one to three significant
// bits) and of the signed zeros here once `with_base` handles them — cross-base conversion
// of a subnormal is inexact, and at unlimited precision it panics instead of rounding, so
// the decimal rows of this test fail until the base-conversion kernel is fixed.
