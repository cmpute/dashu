use core::{
    fmt::Debug,
    ops::{Add, AddAssign, Sub, SubAssign},
};

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

    assert_eq!(c - a, *b);
    assert_eq!(c.clone() - a, *b);
    assert_eq!(c - a.clone(), *b);
    assert_eq!(c.clone() - a.clone(), *b);

    let mut x = c.clone();
    x -= a;
    assert_eq!(x, *b);

    let mut x = c.clone();
    x -= a.clone();
    assert_eq!(x, *b);
}

#[test]
fn test_add_rbig() {
    let test_cases = [
        (rbig!(0), rbig!(0), rbig!(0)),
        (rbig!(0), rbig!(1), rbig!(1)),
        (rbig!(0), rbig!(-1), rbig!(-1)),
        (rbig!(1), rbig!(-1), rbig!(0)),
        (rbig!(1), rbig!(1), rbig!(2)),
        (rbig!(1 / 2), rbig!(1 / 2), rbig!(1)),
        (rbig!(1 / 2), rbig!(-1 / 2), rbig!(0)),
        (rbig!(1 / 3), rbig!(1 / 2), rbig!(5 / 6)),
        (rbig!(1 / 3), rbig!(-1 / 2), rbig!(-1 / 6)),
        (rbig!(1 / 6), rbig!(5 / 6), rbig!(1)),
    ];

    for (a, b, c) in &test_cases {
        test_add_sub(a, b, c);
        test_add_sub(b, a, c);
    }
}

#[test]
fn test_add_relaxed() {
    let relaxed_cases = [
        (rbig!(~0), rbig!(~0), rbig!(~0)),
        (rbig!(~0), rbig!(~1), rbig!(~1)),
        (rbig!(~0), rbig!(~-1), rbig!(~-1)),
        (rbig!(~1), rbig!(~-1), rbig!(~0)),
        (rbig!(~1), rbig!(~1), rbig!(~2)),
        (rbig!(~1 / 2), rbig!(~1 / 2), rbig!(~1)),
        (rbig!(~1 / 2), rbig!(~-1 / 2), rbig!(~0)),
        (rbig!(~1 / 3), rbig!(~1 / 2), rbig!(~5 / 6)),
        (rbig!(~1 / 3), rbig!(~-1 / 2), rbig!(~-1 / 6)),
        (rbig!(~1 / 6), rbig!(~5 / 6), rbig!(~1)),
    ];

    for (a, b, c) in &relaxed_cases {
        test_add_sub(a, b, c);
        test_add_sub(b, a, c);
    }
}
