use core::{
    fmt::Debug,
    ops::{Add, AddAssign, Sub, SubAssign},
};
use dashu_base::Approximation::*;
use dashu_float::{round::Rounding::*, Context};

mod helper_macros;

/// Test a + b = c in various ways.
fn test_add<'a, T>(a: &'a T, b: &'a T, c: &'a T)
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

/// Test a - b = c in various ways.
fn test_sub<'a, T>(a: &'a T, b: &'a T, c: &'a T)
where
    T: Sub<T, Output = T>,
    T: Sub<&'a T, Output = T>,
    &'a T: Sub<T, Output = T>,
    &'a T: Sub<&'a T, Output = T>,
    T: SubAssign<T>,
    T: SubAssign<&'a T>,
    T: Clone,
    T: Debug,
    T: Eq,
{
    assert_eq!(a - b, *c);
    assert_eq!(a.clone() - b, *c);
    assert_eq!(a - b.clone(), *c);
    assert_eq!(a.clone() - b.clone(), *c);

    let mut x = a.clone();
    x -= b;
    assert_eq!(x, *c);

    let mut x = a.clone();
    x -= b.clone();
    assert_eq!(x, *c);
}

#[test]
fn test_add_binary() {
    // cases without rounding
    let exact_cases = [
        (fbig!(0), fbig!(0), fbig!(0)),
        (fbig!(0), fbig!(1), fbig!(1)),
        (fbig!(0), fbig!(-1), fbig!(-1)),
        (fbig!(0x001), fbig!(0x100), fbig!(0x101)),
        (fbig!(0x00001p8), fbig!(0x00001p-8), fbig!(0x10001p-8)),
        (fbig!(0x123p2), fbig!(-0x123p2), fbig!(0)),
        (fbig!(0x123p2), fbig!(-0x023p2), fbig!(0x1p10)),
        (fbig!(0x123p2), fbig!(-0x234p-2), fbig!(0xffcp-2)),
        (fbig!(0x100), fbig!(-0x001p-1), fbig!(0x1ffp-1)),
        (fbig!(0x100), fbig!(0x001p-1), fbig!(0x201p-1)),
        (fbig!(0xff), fbig!(0x01), fbig!(0x1p8)),
    ];
    for (a, b, c) in &exact_cases {
        test_add(a, b, c);
        test_add(b, a, c);
        test_sub(c, a, b);
        test_sub(c, b, a);

        let context = Context::max(a.context(), b.context());
        match (context.add(a, b), context.sub(c, a), context.sub(c, b)) {
            (Exact(vc), Exact(vb), Exact(va)) => {
                assert_eq!(va, *a);
                assert_eq!(vb, *b);
                assert_eq!(vc, *c);
            }
            _ => panic!("the result should be exact!"),
        }
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
        test_add(a, b, c);
        test_add(b, a, c);

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
        (dbig!(0), dbig!(-1), dbig!(-1)),
        (dbig!(1), dbig!(100), dbig!(101)),
        (dbig!(00001e2), dbig!(00001e-2), dbig!(10001e-2)),
        (dbig!(123e2), dbig!(-123e2), dbig!(0)),
        (dbig!(995), dbig!(005), dbig!(100e1)),
    ];

    for (a, b, c) in &exact_cases {
        test_add(a, b, c);
        test_add(b, a, c);
        test_sub(c, a, b);
        test_sub(c, b, a);

        let context = Context::max(a.context(), b.context());
        match (context.add(a, b), context.sub(c, a), context.sub(c, b)) {
            (Exact(vc), Exact(vb), Exact(va)) => {
                assert_eq!(va, *a);
                assert_eq!(vb, *b);
                assert_eq!(vc, *c);
            }
            _ => panic!("the result should be exact!"),
        }
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
        test_add(a, b, c);
        test_add(b, a, c);

        if let Inexact(v, e) = Context::max(a.context(), b.context()).add(a, b) {
            assert_eq!(v, *c);
            assert_eq!(e, *rnd);
        } else {
            panic!("the result should be inexact!")
        }
    }
}

#[test]
fn test_sub_binary() {
    let inexact_cases = [
        (fbig!(0x100), fbig!(0x1p-10), fbig!(0xfffp-4), SubOne),
        (fbig!(0x100), fbig!(0x1p-100), fbig!(0xfffp-4), SubOne),
        (fbig!(0x100), fbig!(-0x1p-10), fbig!(0x100), NoOp),
        (fbig!(0x100), fbig!(-0x1p-100), fbig!(0x100), NoOp),
        (fbig!(0xff), fbig!(-0x1p-1), fbig!(0xff), NoOp),
    ];

    for (a, b, c, rnd) in &inexact_cases {
        test_sub(a, b, c);

        if let Inexact(v, e) = Context::max(a.context(), b.context()).sub(a, b) {
            assert_eq!(v, *c);
            assert_eq!(e, *rnd);
        } else {
            panic!("the result should be inexact!")
        }
    }
}

#[test]
fn test_sub_decimal() {
    // in these cases, the output significand can have one more digit than the precision
    let exact_cases = [
        (dbig!(100), dbig!(2e-1), dbig!(998e-1)),
        (dbig!(101), dbig!(2e-1), dbig!(1008e-1)),
        (dbig!(101), dbig!(5e-1), dbig!(1005e-1)),
        (dbig!(101), dbig!(8e-1), dbig!(1002e-1)),
        (dbig!(101), dbig!(12e-1), dbig!(998e-1)),
        (dbig!(102), dbig!(2e-1), dbig!(1018e-1)),
        (dbig!(100e1), dbig!(8), dbig!(992)),
        (dbig!(101e1), dbig!(13), dbig!(997)),
        (dbig!(999), dbig!(2e-1), dbig!(9988e-1)),
    ];

    for (a, b, c) in &exact_cases {
        test_sub(a, b, c);

        if let Exact(v) = Context::max(a.context(), b.context()).sub(a, b) {
            assert_eq!(v, *c);
        } else {
            panic!("the result should be exact!")
        }
    }

    let inexact_cases = [
        (dbig!(100), dbig!(1e-10), dbig!(100), NoOp),
        (dbig!(100), dbig!(1e-100), dbig!(100), NoOp),
        (dbig!(100), dbig!(-2e-2), dbig!(100), NoOp),
        (dbig!(100), dbig!(-5e-2), dbig!(100), NoOp),
        (dbig!(999e-1), dbig!(-8e-2), dbig!(100), AddOne),
        (dbig!(999e-1), dbig!(-12e-2), dbig!(100), NoOp),
        (dbig!(999e-1), dbig!(-15e-2), dbig!(100), NoOp),
        (dbig!(998e-1), dbig!(-18e-2), dbig!(100), AddOne),
        (dbig!(100), dbig!(-1e-10), dbig!(100), NoOp),
        (dbig!(100), dbig!(-1e-100), dbig!(100), NoOp),
    ];

    for (a, b, c, rnd) in &inexact_cases {
        test_sub(a, b, c);

        if let Inexact(v, e) = Context::max(a.context(), b.context()).sub(a, b) {
            assert_eq!(v, *c);
            assert_eq!(e, *rnd);
        } else {
            panic!("the result should be inexact!")
        }
    }
}

#[test]
#[should_panic]
fn test_add_by_inf() {
    let _ = dashu_float::DBig::ONE + dashu_float::DBig::INFINITY;
}

#[test]
#[should_panic]
fn test_sub_by_inf() {
    let _ = dashu_float::DBig::ONE - dashu_float::DBig::INFINITY;
}
