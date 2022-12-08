use core::fmt::Debug;
use core::ops::{Div, DivAssign, Rem, RemAssign};

use dashu_base::{DivEuclid, DivRemEuclid, RemEuclid};

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

fn test_rem<'a, T>(a: &'a T, b: &'a T, c: &'a T)
where
    T: Rem<T, Output = T>,
    T: Rem<&'a T, Output = T>,
    &'a T: Rem<T, Output = T>,
    &'a T: Rem<&'a T, Output = T>,
    T: RemAssign<T>,
    T: RemAssign<&'a T>,
    T: Clone,
    T: Debug,
    T: Eq,
{
    assert_eq!(a % b, *c);
    assert_eq!(a.clone() % b, *c);
    assert_eq!(a % b.clone(), *c);
    assert_eq!(a.clone() % b.clone(), *c);

    let mut x = a.clone();
    x %= b;
    assert_eq!(x, *c);

    let mut x = a.clone();
    x %= b.clone();
    assert_eq!(x, *c);
}

#[test]
fn test_div_rbig() {
    let test_cases = [
        (rbig!(1), rbig!(1), rbig!(1)),
        (rbig!(1), rbig!(-1), rbig!(-1)),
        (rbig!(-1), rbig!(-1), rbig!(1)),
        (rbig!(-1 / 2), rbig!(1 / 3), rbig!(-3 / 2)),
        (rbig!(1 / 2), rbig!(-2 / 3), rbig!(-3 / 4)),
        (rbig!(-10 / 9), rbig!(-15 / 4), rbig!(8 / 27)),
    ];

    for (a, b, c) in &test_cases {
        test_div(a, b, c);
    }
}

#[test]
#[should_panic]
fn test_divide_by_0_rbig() {
    let _ = rbig!(1) / rbig!(0);
}

#[test]
fn test_rem_rbig() {
    let test_cases = [
        (rbig!(1), rbig!(1), rbig!(0)),
        (rbig!(1), rbig!(-1), rbig!(0)),
        (rbig!(-1), rbig!(-1), rbig!(0)),
        (rbig!(-1 / 2), rbig!(1 / 3), rbig!(1 / 6)),
        (rbig!(1 / 2), rbig!(-2 / 3), rbig!(-1 / 6)),
        (rbig!(-10 / 9), rbig!(-15 / 4), rbig!(-10 / 9)),
    ];

    for (a, b, c) in &test_cases {
        test_rem(a, b, c);
    }
}

#[test]
#[should_panic]
fn test_rem_by_0_rbig() {
    let _ = rbig!(1) % rbig!(0);
}

#[test]
fn test_div_relaxed() {
    let test_cases = [
        (rbig!(~1), rbig!(~1), rbig!(~1)),
        (rbig!(~1), rbig!(~-1), rbig!(~-1)),
        (rbig!(~-1), rbig!(~-1), rbig!(~1)),
        (rbig!(~1/2), rbig!(~-2/3), rbig!(~-3/4)),
        (rbig!(~-10/9), rbig!(~-15/4), rbig!(~8/27)),
    ];

    for (a, b, c) in &test_cases {
        test_div(a, b, c);
    }
}

#[test]
#[should_panic]
fn test_divide_by_0_relaxed() {
    let _ = rbig!(~1) / rbig!(~0);
}

#[test]
fn test_rem_relaxed() {
    let test_cases = [
        (rbig!(~1), rbig!(~1), rbig!(~0)),
        (rbig!(~1), rbig!(~-1), rbig!(~0)),
        (rbig!(~-1), rbig!(~-1), rbig!(~0)),
        (rbig!(~-1/2), rbig!(~1/3), rbig!(~1/6)),
        (rbig!(~1/2), rbig!(~-2/3), rbig!(~-1/6)),
        (rbig!(~-10/9), rbig!(~-15/4), rbig!(~-10/9)),
    ];

    for (a, b, c) in &test_cases {
        test_rem(a, b, c);
    }
}

#[test]
fn test_div_rem_euclid_rbig() {
    // (n, d, quotient, remainder)
    let test_cases = [
        (rbig!(1), rbig!(1), ibig!(1), rbig!(0)),
        (rbig!(1), rbig!(-1), ibig!(-1), rbig!(0)),
        (rbig!(-1), rbig!(-1), ibig!(1), rbig!(0)),
        (rbig!(-1 / 2), rbig!(1 / 3), ibig!(-2), rbig!(1 / 6)),
        (rbig!(1 / 2), rbig!(-2 / 3), ibig!(0), rbig!(1 / 2)),
        (rbig!(-10 / 9), rbig!(-15 / 4), ibig!(1), rbig!(95 / 36)),
    ];

    for (n, d, q, r) in &test_cases {
        assert_eq!(n.div_euclid(d), *q);
        assert_eq!(n.rem_euclid(d), *r);
        assert_eq!(n.div_rem_euclid(d), (q.clone(), r.clone()));
    }
}

#[test]
fn test_div_rem_euclid_relaxed() {
    // (n, d, quotient, remainder)
    let test_cases = [
        (rbig!(~1), rbig!(~1), ibig!(1), rbig!(~0)),
        (rbig!(~1), rbig!(~-1), ibig!(-1), rbig!(~0)),
        (rbig!(~-1), rbig!(~-1), ibig!(1), rbig!(~0)),
        (rbig!(~-1/2), rbig!(~1/3), ibig!(-2), rbig!(~1/6)),
        (rbig!(~1/2), rbig!(~-2/3), ibig!(0), rbig!(~1/2)),
        (rbig!(~-10/9), rbig!(~-15/4), ibig!(1), rbig!(~95/36)),
    ];

    for (n, d, q, r) in &test_cases {
        assert_eq!(n.div_euclid(d), *q);
        assert_eq!(n.rem_euclid(d), *r);
        assert_eq!(n.div_rem_euclid(d), (q.clone(), r.clone()));
    }
}
