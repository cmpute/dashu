use dashu_base::Approximation::*;
use dashu_float::round::Rounding::*;

mod helper_macros;

#[test]
fn test_powi_binary() {
    let exact_cases = [
        (fbig!(0x1), ibig!(0), fbig!(0x1)),
        (fbig!(0x1), ibig!(1), fbig!(0x1)),
        (fbig!(-0x2), ibig!(1), fbig!(-0x2)),
        (fbig!(-0x2), ibig!(-1), fbig!(-0x1p-1)),
        (fbig!(-0x2), ibig!(2), fbig!(0x4)),
        (fbig!(-0x2), ibig!(-2), fbig!(0x1p-2)),
        (fbig!(-0x03p-2), ibig!(3), fbig!(-0x1bp-6)),
        (fbig!(-0x005p2), ibig!(5), fbig!(-0xc35p10)),
    ];
    for (base, exp, pow) in &exact_cases {
        assert_eq!(base.powi(exp.clone()), *pow);
        if let Exact(v) = base.context().powi(base, exp.clone()) {
            assert_eq!(v, *pow);
        } else {
            panic!("the result should be exact!")
        }
    }

    let inexact_cases = [
        (fbig!(-0x123), ibig!(2), fbig!(0xa56p5)),
        (fbig!(0x123), ibig!(-2), fbig!(0xc61p-28)),
        (fbig!(0x10001p-16), ibig!(100), fbig!(0x80320p-19)),
        (fbig!(0x10001p-16), ibig!(100000), fbig!(0x932c1p-17)),
        (fbig!(0x10000001p-28), ibig!(100000), fbig!(0x800c3595p-31)),
        (fbig!(0x10000001p-28), ibig!(1000000000), fbig!(0xa5eedf2ep-26)),
    ];

    for (base, exp, pow) in &inexact_cases {
        assert_eq!(base.powi(exp.clone()), *pow);
        if let Inexact(v, e) = base.context().powi(base, exp.clone()) {
            assert_eq!(v, *pow);
            assert_eq!(e, NoOp);
        } else {
            panic!("the result should be inexact!")
        }
    }
}

// TODO: add more test cases for decimal cases

#[test]
fn test_exp_binary() {
    assert_eq!(fbig!(0).exp(), fbig!(1));

    let inexact_cases = [
        (fbig!(0x1), fbig!(0xap-2)),
        (fbig!(0x0001), fbig!(0xadf8p-14)),
        (fbig!(0x0000000000000001), fbig!(0xadf85458a2bb4a9ap-62)),
        (fbig!(1).with_precision(200).value(), fbig!(0xadf85458a2bb4a9aafdc5620273d3cf1d8b9c583ce2d3695a9p-198)),
        (fbig!(0x12p-4), fbig!(0xc5p-6)),
        (fbig!(0x1234p-12), fbig!(0xc7a7p-14)),
        (fbig!(0x123456789p-32), fbig!(0xc7ab41d2cp-34)),
        (fbig!(0x123456789012345678901234567890123456789p-152), fbig!(0xc7ab41d2cef9900a0e4de4219dd6d2aaaee02fap-154)),
        // TODO: add negative test cases after we have correct inverse rounding
        // (fbig!(-0x1), fbig!(0xbp-5)),
    ];
    for (exp, pow) in &inexact_cases {
        dbg!(exp, pow);
        assert_eq!(exp.exp(), *pow);
        if let Inexact(v, e) = exp.context().exp(exp) {
            assert_eq!(v, *pow);
            assert_eq!(e, NoOp);
        } else {
            panic!("the result should be inexact!")
        }
    }
}
