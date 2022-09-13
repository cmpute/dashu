use core::{
    fmt::Debug,
    ops::{Div, DivAssign},
};
use dashu_base::{Approximation::*, DivEuclid, DivRemEuclid, RemEuclid};
use dashu_float::{round::Rounding::*, Context};

mod helper_macros;

fn test_div<'a, T>(a: &'a T, b: &'a T, c: &'a T)
where
    T: Div<T, Output = T>,
    T: Div<&'a T, Output = T>,
    &'a T: Div<T, Output = T>,
    &'a T: Div<&'a T, Output = T>,
    T: DivAssign<T>,
    T: DivAssign<&'a T>,
    T: Clone,
    T: Debug,
    T: Eq,
{
    assert_eq!(a / b, *c);
    assert_eq!(a.clone() / b, *c);
    assert_eq!(a / b.clone(), *c);
    assert_eq!(a.clone() / b.clone(), *c);

    let mut x = a.clone();
    x /= b;
    assert_eq!(x, *c);

    let mut x = a.clone();
    x /= b.clone();
    assert_eq!(x, *c);
}

#[test]
fn test_div_binary() {
    let exact_cases = [
        (fbig!(0), fbig!(1), fbig!(0)),
        (fbig!(1), fbig!(1), fbig!(1)),
        (fbig!(0x1000), fbig!(0x1000), fbig!(1)),
        (fbig!(0x1000), fbig!(0x10), fbig!(0x100)),
        (fbig!(0x1000), fbig!(-0x10), fbig!(-0x100)),
        (fbig!(-0xffff), fbig!(-0xff), fbig!(0x101)),
        (fbig!(0x1b), fbig!(0x3), fbig!(0x9)),
    ];

    for (a, b, c) in &exact_cases {
        test_div(a, b, c);

        if let Exact(v) = Context::max(a.context(), b.context()).div(a.repr(), b.repr()) {
            assert_eq!(v, *c);
        } else {
            panic!("the result should be exact!")
        }
    }

    let inexact_cases = [
        (fbig!(0x43), fbig!(0x21), fbig!(0x81p-6)),
        (fbig!(0x654), fbig!(-0x321), fbig!(-0x817p-10)),
        (fbig!(-0x98765), fbig!(-0x43210), fbig!(0x915b1p-18)),
        (fbig!(0x1), fbig!(0x9), fbig!(0xep-7)),
        (fbig!(0x1), fbig!(0x09), fbig!(0xe3p-11)),
        (fbig!(0x1), fbig!(0x009), fbig!(0x1c7p-12)),
        (fbig!(0x13), fbig!(-0x9), fbig!(-0x87p-6)),
        (fbig!(0x169), fbig!(-0x9), fbig!(-0xa07p-6)),
    ];
    for (a, b, c) in &inexact_cases {
        test_div(a, b, c);

        if let Inexact(v, e) = Context::max(a.context(), b.context()).div(a.repr(), b.repr()) {
            assert_eq!(v, *c);
            assert_eq!(e, NoOp);
        } else {
            panic!("the result should be inexact!")
        }
    }
}

#[test]
fn test_div_decimal() {
    let exact_cases = [
        (dbig!(0), dbig!(1), dbig!(0)),
        (dbig!(1), dbig!(1), dbig!(1)),
        (dbig!(1000), dbig!(1000), dbig!(1)),
        (dbig!(1000), dbig!(10), dbig!(100)),
        (dbig!(1000), dbig!(-10), dbig!(-100)),
        (dbig!(-9999), dbig!(-99), dbig!(101)),
        (dbig!(27), dbig!(3), dbig!(9)),
    ];

    for (a, b, c) in &exact_cases {
        test_div(a, b, c);

        if let Exact(v) = Context::max(a.context(), b.context()).div(a.repr(), b.repr()) {
            assert_eq!(v, *c);
        } else {
            panic!("the result should be exact!")
        }
    }

    let inexact_cases = [
        (dbig!(43), dbig!(21), dbig!(2), NoOp),
        (dbig!(654), dbig!(-321), dbig!(-204e-2), SubOne),
        (dbig!(-98765), dbig!(-43210), dbig!(22857e-4), AddOne),
        (dbig!(1), dbig!(9), dbig!(1e-1), NoOp),
        (dbig!(1), dbig!(09), dbig!(11e-2), NoOp),
        (dbig!(1), dbig!(009), dbig!(111e-3), NoOp),
        (dbig!(13), dbig!(-9), dbig!(-14e-1), NoOp),
        (dbig!(169), dbig!(-9), dbig!(-188e-1), SubOne),
        (dbig!(1), dbig!(4), dbig!(3e-1), AddOne),
        (dbig!(1), dbig!(-4), dbig!(-3e-1), SubOne),
    ];
    for (a, b, c, rnd) in &inexact_cases {
        test_div(a, b, c);

        if let Inexact(v, e) = Context::max(a.context(), b.context()).div(a.repr(), b.repr()) {
            assert_eq!(v, *c);
            assert_eq!(e, *rnd);
        } else {
            panic!("the result should be inexact!")
        }
    }
}

#[test]
#[should_panic]
fn test_div_by_inf() {
    let _ = dashu_float::DBig::ONE / dashu_float::DBig::INFINITY;
}

#[test]
#[should_panic]
fn test_div_by_0() {
    let _ = dashu_float::DBig::ONE / dashu_float::DBig::ZERO;
}

#[test]
#[should_panic]
fn test_div_by_unlimited_precision() {
    let _ = dbig!(1).with_precision(0).value() / dbig!(3).with_precision(0).value();
}

#[test]
fn test_div_rem_euclid_binary() {
    // test cases: n, d, quotient, remainder
    let test_cases = [
        (fbig!(0), fbig!(1), ibig!(0), fbig!(0)),
        (fbig!(1), fbig!(1), ibig!(1), fbig!(0)),
        (fbig!(0x1000), fbig!(0x1000), ibig!(1), fbig!(0)),
        (fbig!(0x1000), fbig!(0x10), ibig!(0x100), fbig!(0)),
        (fbig!(0x1000), fbig!(-0x10), ibig!(-0x100), fbig!(0)),
        (fbig!(-0xffff), fbig!(-0xff), ibig!(0x101), fbig!(0)),
        (fbig!(0x1b), fbig!(0x3), ibig!(9), fbig!(0)),
        (fbig!(0x43), fbig!(0x21), ibig!(2), fbig!(0x1)),
        (fbig!(0x654), fbig!(-0x321), ibig!(-2), fbig!(0x12)),
        (fbig!(-0x98765), fbig!(-0x43210), ibig!(3), fbig!(0x30ecb)),
        (fbig!(0x1), fbig!(0x9), ibig!(0), fbig!(0x1)),
        (fbig!(0x1), fbig!(0x9p-4), ibig!(1), fbig!(0x7p-4)),
        (fbig!(0x1), fbig!(0x9p-8), ibig!(28), fbig!(0x4p-8)),
        (fbig!(0x13), fbig!(-0x9), ibig!(-2), fbig!(0x1)),
        (fbig!(0x169), fbig!(-0x9), ibig!(-40), fbig!(0x1)),
    ];

    for (n, d, q, r) in &test_cases {
        assert_eq!(n.div_euclid(d), *q);
        assert_eq!(n.rem_euclid(d), *r);
        let (quotient, remainder) = n.div_rem_euclid(d);
        assert_eq!(quotient, *q);
        assert_eq!(remainder, *r);
    }
}

#[test]
fn test_div_rem_euclid_decimal() {
    // test cases: n, d, quotient, remainder
    let test_cases = [
        (dbig!(0), dbig!(1), ibig!(0), dbig!(0)),
        (dbig!(1), dbig!(1), ibig!(1), dbig!(0)),
        (dbig!(1000), dbig!(1000), ibig!(1), dbig!(0)),
        (dbig!(1000), dbig!(10), ibig!(100), dbig!(0)),
        (dbig!(1000), dbig!(-10), ibig!(-100), dbig!(0)),
        (dbig!(-9999), dbig!(-99), ibig!(101), dbig!(0)),
        (dbig!(27), dbig!(3), ibig!(9), dbig!(0)),
        (dbig!(43), dbig!(21), ibig!(2), dbig!(1)),
        (dbig!(654), dbig!(-321), ibig!(-2), dbig!(12)),
        (dbig!(-98765), dbig!(-43210), ibig!(3), dbig!(30865)),
        (dbig!(1), dbig!(9), ibig!(0), dbig!(1)),
        (dbig!(1), dbig!(9e-1), ibig!(1), dbig!(1e-1)),
        (dbig!(1), dbig!(9e-2), ibig!(11), dbig!(1e-2)),
        (dbig!(13), dbig!(-9), ibig!(-1), dbig!(4)),
        (dbig!(169), dbig!(-9), ibig!(-18), dbig!(7)),
    ];

    for (n, d, q, r) in &test_cases {
        assert_eq!(n.div_euclid(d), *q);
        assert_eq!(n.rem_euclid(d), *r);
        let (quotient, remainder) = n.div_rem_euclid(d);
        assert_eq!(quotient, *q);
        assert_eq!(remainder, *r);
    }
}
