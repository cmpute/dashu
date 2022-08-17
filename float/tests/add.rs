use core::{
    fmt::Debug,
    ops::{Add, AddAssign, Sub, SubAssign},
};
use dashu_base::{Approximation::*, DivRem};
use dashu_float::{round::Rounding::*, Context};

mod helper_macros;

/// Test a + b = c in various ways.
fn test_add_sub<'a, T>(a: &'a T, b: &'a T, c: &'a T)
where
    T: Add<T, Output = T>,
    T: Add<&'a T, Output = T>,
    &'a T: Add<T, Output = T>,
    &'a T: Add<&'a T, Output = T>,
    T: AddAssign<T>,
    T: AddAssign<&'a T>,
    T: Clone,
    T: Debug,
    T: Eq,
{
    assert_eq!(a + b, *c);
    assert_eq!(a.clone() + b, *c);
    assert_eq!(a + b.clone(), *c);
    assert_eq!(a.clone() + b.clone(), *c);

    let mut x = a.clone();
    x += b;
    assert_eq!(x, *c);

    let mut x = a.clone();
    x += b.clone();
    assert_eq!(x, *c);
}

#[test]
fn test_add_binary() {
    // cases without rounding
    let exact_cases = [
        (fbig!(0), fbig!(0), fbig!(0)),
        (fbig!(0), fbig!(1), fbig!(1)),
        (fbig!(0), fbig!(-1), fbig!(-1)),
        (fbig!(0x1), fbig!(0x100), fbig!(0x101)),
        (fbig!(0x00001p8), fbig!(0x00001p-8), fbig!(0x10001p-8)),
        (fbig!(0x123p2), fbig!(-0x123p2), fbig!(0)),
        (fbig!(0x123p2), fbig!(-0x23p2), fbig!(0x1p10)),
        (fbig!(0x123p2), fbig!(-0x234p-2), fbig!(0xffcp-2)),
        (fbig!(0x100), fbig!(-0x1p-1), fbig!(0x1ffp-1)),
        (fbig!(0x100), fbig!(0x1p-1), fbig!(0x201p-1)),
        (fbig!(0xff), fbig!(0x1), fbig!(0x1p8)),
    ];
    for (a, b, c) in &exact_cases {
        test_add_sub(a, b, c);
        test_add_sub(b, a, c);
        assert!(matches!(Context::max(a.context(), b.context()).add(a, b), Exact(_)));
    }

    // cases with rounding
    let inexact_cases = [
        (fbig!(0x100), fbig!(0x1p-10), fbig!(0x100), NoOp),
        (fbig!(0x100), fbig!(0x1p-100), fbig!(0x100), NoOp),
        (fbig!(0x100), fbig!(-0x1p-10), fbig!(0xfffp-4), SubOne),
        (fbig!(0x100), fbig!(-0x1p-100), fbig!(0xfffp-4), SubOne),
        (fbig!(0xff), fbig!(0x1p-1), fbig!(0xff), NoOp),
    ];

    for (a, b, c, rnd) in &inexact_cases {
        test_add_sub(a, b, c);
        test_add_sub(b, a, c);

        if let Inexact(v, e) = Context::max(a.context(), b.context()).add(a, b) {
            assert_eq!(v, *c);
            assert_eq!(e, *rnd);
        } else {
            panic!("the result should be inexact!")
        }
    }
}

#[test]
fn test_add_decimal() {
    // cases without rounding
    let exact_cases = [
        (dbig!(0), dbig!(0), dbig!(0)),
        (dbig!(0), dbig!(1), dbig!(1)),
        (dbig!(1), dbig!(100), dbig!(101)),
        (dbig!(00001e2), dbig!(00001e-2), dbig!(10001e-2)),
        (dbig!(123e2), dbig!(-123e2), dbig!(0)),
        (dbig!(995), dbig!(5), dbig!(100e1)),
    ];

    for (a, b, c) in &exact_cases {
        test_add_sub(a, b, c);
        test_add_sub(b, a, c);
        // assert!(matches!(Context::max(a.context(), b.context()).add(a, b), Exact(_)));
    }

    // cases with rounding
    let inexact_cases = [
        (dbig!(100), dbig!(2e-1), dbig!(100), NoOp),
        (dbig!(100), dbig!(5e-1), dbig!(101), AddOne),
        (dbig!(100), dbig!(8e-1), dbig!(101), AddOne),
        (dbig!(100), dbig!(12e-1), dbig!(101), NoOp),
        (dbig!(100), dbig!(15e-1), dbig!(102), AddOne),
        (dbig!(100), dbig!(18e-1), dbig!(102), AddOne),
        (dbig!(100), dbig!(1e-10), dbig!(100), NoOp),
        (dbig!(100), dbig!(1e-100), dbig!(100), NoOp),
        (dbig!(100), dbig!(-2e-2), dbig!(100), NoOp),
        (dbig!(100), dbig!(-5e-2), dbig!(100), NoOp),
        (dbig!(100), dbig!(-8e-2), dbig!(999e-1), SubOne),
        (dbig!(100), dbig!(-12e-2), dbig!(999e-1), NoOp),
        (dbig!(100), dbig!(-15e-2), dbig!(999e-1), NoOp),
        (dbig!(100), dbig!(-18e-2), dbig!(998e-1), SubOne),
        (dbig!(100), dbig!(-1e-10), dbig!(100), NoOp),
        (dbig!(100), dbig!(-1e-100), dbig!(100), NoOp),
        (dbig!(995), dbig!(8), dbig!(100e1), NoOp),
        (dbig!(995), dbig!(10), dbig!(101e1), AddOne),
        (dbig!(995), dbig!(13), dbig!(101e1), AddOne),
        (dbig!(999), dbig!(2e-1), dbig!(999), NoOp),
        (dbig!(999), dbig!(5e-1), dbig!(1e3), AddOne),
        (dbig!(999), dbig!(8e-1), dbig!(1e3), AddOne),
    ];

    for (a, b, c, rnd) in &inexact_cases {
        test_add_sub(a, b, c);
        test_add_sub(b, a, c);

        if let Inexact(v, e) = Context::max(a.context(), b.context()).add(a, b) {
            assert_eq!(v, *c);
            assert_eq!(e, *rnd);
        } else {
            panic!("the result should be inexact!")
        }
    }
}
