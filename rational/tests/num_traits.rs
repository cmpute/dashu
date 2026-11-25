//! Integration tests for the `num-traits` trait impls on `RBig` / `Relaxed`,
//! in particular the signed-exponent `Pow<isize>` — negative exponents
//! reciprocate first, so `(2/3)^-3 == 27/8`.

use core::fmt::Debug;

use dashu_ratio::RBig;
use num_traits_v02::Pow;

mod helper_macros;

/// Assert both the owned and the by-ref `Pow<isize>` paths agree with `expected`.
fn check_pow_isize<T>(base: &T, exp: isize, expected: &T)
where
    T: Pow<isize, Output = T> + PartialEq + Debug + Clone,
    for<'a> &'a T: Pow<isize, Output = T>,
{
    assert_eq!(base.clone().pow(exp), expected.clone(), "owned pow");
    assert_eq!(base.pow(exp), expected.clone(), "by-ref pow");
}

#[test]
fn pow_isize_rbig() {
    // negative exponents reciprocate first
    check_pow_isize(&rbig!(2 / 3), -1, &rbig!(3 / 2));
    check_pow_isize(&rbig!(2 / 3), -3, &rbig!(27 / 8));
    check_pow_isize(&rbig!(-2 / 3), -2, &rbig!(9 / 4));
    check_pow_isize(&rbig!(-2 / 3), -3, &rbig!(-27 / 8));
    // integer-valued base
    check_pow_isize(&rbig!(5), -1, &rbig!(1 / 5));
    check_pow_isize(&rbig!(5), -2, &rbig!(1 / 25));
    // non-negative exponents land on the same value as the unsigned path
    check_pow_isize(&rbig!(2 / 3), 0, &RBig::ONE);
    check_pow_isize(&rbig!(2 / 3), 5, &rbig!(32 / 243));
}

#[test]
fn pow_isize_relaxed() {
    check_pow_isize(&rbig!(~2 / 3), -1, &rbig!(~3 / 2));
    check_pow_isize(&rbig!(~-2 / 3), -2, &rbig!(~9 / 4));
    check_pow_isize(&rbig!(~-2 / 3), -3, &rbig!(~-27 / 8));
}

#[test]
fn pow_usize_unchanged() {
    // the pre-existing unsigned Pow<usize> impl is untouched
    assert_eq!(Pow::pow(rbig!(2 / 3), 5usize), rbig!(32 / 243));
    assert_eq!(Pow::pow(&rbig!(2 / 3), 5usize), rbig!(32 / 243));
    assert_eq!(Pow::pow(rbig!(~2 / 3), 4usize), rbig!(~16 / 81));
}

#[test]
#[should_panic(expected = "Divisor or denominator must not be zero")]
fn pow_isize_zero_base_negative_panics() {
    let _: RBig = Pow::pow(RBig::ZERO, -1isize);
}

#[test]
fn pow_usize_beyond_isize_max() {
    use dashu_ratio::Relaxed;

    // a usize exponent is never negative, even when it does not fit in an isize
    let huge: usize = isize::MAX as usize + 1;
    assert_eq!(Pow::pow(&RBig::ZERO, huge), RBig::ZERO);
    assert_eq!(Pow::pow(RBig::ZERO, usize::MAX), RBig::ZERO);
    assert_eq!(Pow::pow(&Relaxed::ZERO, huge), Relaxed::ZERO);
    assert_eq!(Pow::pow(Relaxed::ZERO, usize::MAX), Relaxed::ZERO);
    assert_eq!(Pow::pow(&RBig::ONE, huge), RBig::ONE);
    assert_eq!(Pow::pow(RBig::NEG_ONE, usize::MAX), RBig::NEG_ONE);
    assert_eq!(Pow::pow(&RBig::NEG_ONE, huge), RBig::ONE);
    assert_eq!(Pow::pow(Relaxed::NEG_ONE, usize::MAX), Relaxed::NEG_ONE);
    // ordinary exponents
    assert_eq!(Pow::pow(&rbig!(2 / 3), 3usize), rbig!(8 / 27));
    assert_eq!(Pow::pow(rbig!(~-2/3), 3usize), rbig!(~-8/27));
    assert_eq!(Pow::pow(&rbig!(2 / 3), 0usize), rbig!(1));
}

#[test]
fn signed_abs_sub_is_positive_difference() {
    use num_traits_v02::Signed;

    // `abs_sub` is the positive difference: zero unless self > other
    for (a, b) in [
        (1i64, 3i64),
        (-5, 2),
        (0, 1),
        (3, 1),
        (2, 2),
        (-1, -1),
        (7, -2),
        (-2, -7),
    ] {
        let expected = if a <= b { 0 } else { a - b };
        assert_eq!(Signed::abs_sub(&a, &b), expected);
        assert_eq!(Signed::abs_sub(&RBig::from(a), &RBig::from(b)), RBig::from(expected));
        assert_eq!(
            Signed::abs_sub(&RBig::from(a).relax(), &RBig::from(b).relax()),
            RBig::from(expected).relax()
        );
    }
    assert_eq!(Signed::abs_sub(&rbig!(1 / 2), &rbig!(1)), rbig!(0));
    assert_eq!(Signed::abs_sub(&rbig!(1), &rbig!(1 / 2)), rbig!(1 / 2));
    assert_eq!(Signed::abs_sub(&rbig!(~3/9), &rbig!(~1/3)), rbig!(~0));
    assert_eq!(Signed::abs_sub(&rbig!(~-1/3), &rbig!(~-1/2)), rbig!(~1/6));
}
