#![cfg(feature = "num-traits")]

use dashu_int::IBig;
use num_traits_v02::Signed;

#[test]
fn test_abs_sub() {
    // abs_sub is the positive difference: max(a - b, 0)
    for (a, b) in [
        (3i64, 1i64),
        (1, 3),
        (0, 1),
        (3, 5),
        (-5, 2),
        (2, 2),
        (-3, -1),
        (-1, -3),
    ] {
        assert_eq!(
            Signed::abs_sub(&IBig::from(a), &IBig::from(b)),
            IBig::from(Signed::abs_sub(&a, &b)),
            "abs_sub({}, {})",
            a,
            b
        );
    }
    let big = IBig::ONE << 200;
    assert_eq!(Signed::abs_sub(&big, &-&big), IBig::ONE << 201);
    assert_eq!(Signed::abs_sub(&-&big, &big), IBig::ZERO);
}

#[test]
fn test_ubig_from_primitive_float_truncates() {
    use dashu_int::UBig;
    use num_traits_v02::FromPrimitive;

    // same results as the primitive unsigned integers: the fraction is truncated
    // towards zero, so only values at or below -1 are out of range
    for x in [
        0.5f64, 0.0, -0.0, -0.5, -0.999, -1e-300, -1.0, -1.5, 1.5, 12345.75,
    ] {
        assert_eq!(
            <UBig as FromPrimitive>::from_f64(x),
            <u64 as FromPrimitive>::from_f64(x).map(UBig::from),
            "x = {}",
            x
        );
    }
    for x in [0.5f32, -0.0, -0.5, -0.999, -1.0, 1.5] {
        assert_eq!(
            <UBig as FromPrimitive>::from_f32(x),
            <u32 as FromPrimitive>::from_f32(x).map(UBig::from),
            "x = {}",
            x
        );
    }
    assert_eq!(<UBig as FromPrimitive>::from_f64(-0.5), Some(UBig::ZERO));
    assert_eq!(<UBig as FromPrimitive>::from_f64(-1.0), None);
    assert_eq!(<UBig as FromPrimitive>::from_f64(f64::NAN), None);
}
