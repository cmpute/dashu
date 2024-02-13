use core::{
    fmt::Debug,
    ops::{Mul, MulAssign},
};

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
fn test_mul_rbig() {
    let test_cases = [
        (rbig!(0), rbig!(0), rbig!(0)),
        (rbig!(1), rbig!(1), rbig!(1)),
        (rbig!(1), rbig!(-1), rbig!(-1)),
        (rbig!(-1), rbig!(-1), rbig!(1)),
        (rbig!(1 / 2), rbig!(-2 / 3), rbig!(-1 / 3)),
        (rbig!(10 / 9), rbig!(15 / 4), rbig!(25 / 6)),
    ];

    for (a, b, c) in &test_cases {
        test_mul(a, b, c);
    }
}

#[test]
fn test_mul_relaxed() {
    let test_cases = [
        (rbig!(~0), rbig!(~0), rbig!(~0)),
        (rbig!(~1), rbig!(~1), rbig!(~1)),
        (rbig!(~1), rbig!(~-1), rbig!(~-1)),
        (rbig!(~-1), rbig!(~-1), rbig!(~1)),
        (rbig!(~1/2), rbig!(~-2/3), rbig!(~-1/3)),
        (rbig!(~10/9), rbig!(~15/4), rbig!(~75/18)),
    ];

    for (a, b, c) in &test_cases {
        test_mul(a, b, c);
    }
}

#[test]
fn test_add_with_ibig() {
    let test_cases = [
        (rbig!(0), ibig!(0), rbig!(0)),
        (rbig!(1), ibig!(1), rbig!(1)),
        (rbig!(1), ibig!(-1), rbig!(-1)),
        (rbig!(-1), ibig!(-1), rbig!(1)),
        (rbig!(1 / 2), ibig!(-2), rbig!(-1)),
        (rbig!(1 / 2), ibig!(-4), rbig!(-2)),
        (rbig!(10 / 7), ibig!(7), rbig!(10)),
        (rbig!(-7 / 6), ibig!(9), rbig!(-21 / 2)),
    ];

    for (a, b, c) in &test_cases {
        assert_eq!(a * b, *c);
        assert_eq!(b * a, *c);

        let r = &a.clone().relax();
        assert_eq!(r * b, c.clone().relax());
        assert_eq!(b * r, c.clone().relax());
    }
}

#[test]
fn test_mul_with_int() {
    assert_eq!(rbig!(0) * ubig!(1), rbig!(0));
    assert_eq!(rbig!(~0) * ubig!(1), rbig!(~0));
    assert_eq!(rbig!(1) * ubig!(1), rbig!(1));
    assert_eq!(rbig!(~1) * ubig!(1), rbig!(~1)); 
    assert_eq!(rbig!(-1/2) * ibig!(-2), rbig!(1));
    assert_eq!(rbig!(~-1/2) * ibig!(-2), rbig!(~1));
    assert_eq!(rbig!(5/12) * ibig!(-3), rbig!(-5/4));
    assert_eq!(rbig!(~5/12) * ibig!(-3), rbig!(~-5/4));
    
    assert_eq!(ubig!(0) * rbig!(1), rbig!(0));
    assert_eq!(ubig!(0) * rbig!(~1), rbig!(~0));
    assert_eq!(ubig!(1) * rbig!(1), rbig!(1));
    assert_eq!(ubig!(1) * rbig!(~1), rbig!(~1));
    assert_eq!(ibig!(-2) * rbig!(-1/2), rbig!(1));
    assert_eq!(ibig!(-2) * rbig!(~-1/2), rbig!(~1));
    assert_eq!(ibig!(-3) * rbig!(5/12), rbig!(-5/4));
    assert_eq!(ibig!(-3) * rbig!(~5/12), rbig!(~-5/4));
}
