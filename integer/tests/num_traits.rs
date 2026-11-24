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
