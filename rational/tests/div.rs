mod helper_macros;

#[test]
fn test_div_rbig() {
    let test_cases = [
        (rbig!(1), rbig!(1), rbig!(1)),
        (rbig!(1), rbig!(-1), rbig!(-1)),
        (rbig!(-1), rbig!(-1), rbig!(1)),
        (rbig!(1 / 2), rbig!(-2 / 3), rbig!(-3 / 4)),
        (rbig!(-10 / 9), rbig!(-15 / 4), rbig!(8 / 27)),
    ];

    for (a, b, c) in &test_cases {
        assert_eq!(a / b, *c);
    }
}

#[test]
#[should_panic]
fn test_divide_by_0_rbig() {
    let _ = rbig!(1) / rbig!(0);
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
        assert_eq!(a / b, *c);
    }
}

#[test]
#[should_panic]
fn test_divide_by_0_relaxed() {
    let _ = rbig!(~1) / rbig!(~0);
}
