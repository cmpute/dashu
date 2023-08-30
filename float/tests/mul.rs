use core::{
    fmt::Debug,
    ops::{Mul, MulAssign},
};
use dashu_base::Approximation::*;
use dashu_float::{round::Rounding::*, Context};

mod helper_macros;

fn test_mul<'a, T>(a: &'a T, b: &'a T, c: &'a T)
where
    T: Mul<T, Output = T>,
    T: Mul<&'a T, Output = T>,
    &'a T: Mul<T, Output = T>,
    &'a T: Mul<&'a T, Output = T>,
    T: MulAssign<T>,
    T: MulAssign<&'a T>,
    T: Clone,
    T: Debug,
    T: Eq,
{
    assert_eq!(a * b, *c);
    assert_eq!(a.clone() * b, *c);
    assert_eq!(a * b.clone(), *c);
    assert_eq!(a.clone() * b.clone(), *c);

    let mut x = a.clone();
    x *= b;
    assert_eq!(x, *c);

    let mut x = a.clone();
    x *= b.clone();
    assert_eq!(x, *c);
}

#[test]
#[rustfmt::skip::macros(fbig)]
fn test_mul_binary() {
    let exact_cases = [
        (fbig!(0), fbig!(0), fbig!(0)),
        (fbig!(0), fbig!(1), fbig!(0)),
        (fbig!(0), fbig!(-1), fbig!(0)),
        (fbig!(0x1000), fbig!(0), fbig!(0)),
        (fbig!(1), fbig!(-1), fbig!(-1)),
        (fbig!(0x12p1), fbig!(0x34p-1), fbig!(0x75p3)),
        (fbig!(0x056p2), fbig!(0x078p4), fbig!(0x285p10)),
    ];

    for (a, b, c) in &exact_cases {
        test_mul(a, b, c);
        test_mul(b, a, c);

        if let Exact(v) = Context::max(a.context(), b.context()).mul(a.repr(), b.repr()) {
            assert_eq!(v, *c);
        } else {
            panic!("the result should be exact!")
        }
    }

    // for inexact division with rounding to zero, the rounding error is always NoOp
    let inexact_cases = [
        (fbig!(0xa), fbig!(0xb), fbig!(0xdp3)),
        (fbig!(0x13), fbig!(-0x17), fbig!(-0xdap1)),
        (fbig!(-0x56p2), fbig!(-0x78p4), fbig!(0xa1p12)),
    ];

    for (a, b, c) in &inexact_cases {
        test_mul(a, b, c);
        test_mul(b, a, c);

        if let Inexact(v, e) = Context::max(a.context(), b.context()).mul(a.repr(), b.repr()) {
            assert_eq!(v, *c);
            assert_eq!(e, NoOp);
        } else {
            panic!("the result should be inexact!")
        }
    }
}

#[test]
fn test_mul_decimal() {
    let exact_cases = [
        (dbig!(0), dbig!(0), dbig!(0)),
        (dbig!(0), dbig!(1), dbig!(0)),
        (dbig!(0), dbig!(-1), dbig!(0)),
        (dbig!(1000), dbig!(0), dbig!(0)),
        (dbig!(1), dbig!(-1), dbig!(-1)),
        (dbig!(012e1), dbig!(034e-1), dbig!(408)),
        (dbig!(0056e2), dbig!(0078e4), dbig!(4368e6)),
        (dbig!(25), dbig!(16), dbig!(4e2)),
        (dbig!(25), dbig!(40), dbig!(1e3)),
        (dbig!(25), dbig!(64), dbig!(16e2)),
    ];

    for (a, b, c) in &exact_cases {
        test_mul(a, b, c);
        test_mul(b, a, c);

        if let Exact(v) = Context::max(a.context(), b.context()).mul(a.repr(), b.repr()) {
            assert_eq!(v, *c);
        } else {
            panic!("the result should be exact!")
        }
    }

    // for inexact division with rounding to zero, the rounding error is always NoOp
    let inexact_cases = [
        (dbig!(7), dbig!(8), dbig!(6e1), AddOne),
        (dbig!(13), dbig!(-17), dbig!(-22e1), NoOp),
        (dbig!(-12e1), dbig!(34e-1), dbig!(-41e1), SubOne),
        (dbig!(-56e2), dbig!(-78e4), dbig!(44e8), AddOne),
    ];

    for (a, b, c, rnd) in &inexact_cases {
        test_mul(a, b, c);
        test_mul(b, a, c);

        if let Inexact(v, e) = Context::max(a.context(), b.context()).mul(a.repr(), b.repr()) {
            assert_eq!(v, *c);
            assert_eq!(e, *rnd);
        } else {
            panic!("the result should be inexact!")
        }
    }
}

#[test]
#[should_panic]
fn test_mul_by_inf() {
    let _ = dashu_float::DBig::ONE * dashu_float::DBig::INFINITY;
}

#[test]
fn test_mul_zero_precision() {
    let a = fbig!(0xff).with_precision(0).value();
    let b = fbig!(-0xff);
    test_mul(&a, &a, &fbig!(0xfe01));
    test_mul(&a, &b, &fbig!(-0xfep8));

    let a = dbig!(99).with_precision(0).value();
    let b = dbig!(-99);
    test_mul(&a, &a, &dbig!(9801));
    test_mul(&a, &b, &dbig!(-98e2));
}
