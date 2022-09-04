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
        (fbig!(-0x2), ibig!(2), fbig!(0x1p2)),
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
        (fbig!(0x10001p-16), ibig!(-100), fbig!(0xff9c1p-20)),
        (fbig!(0x10001p-16), ibig!(100000), fbig!(0x932c1p-17)),
        (fbig!(0x10001p-16), ibig!(-100000), fbig!(0xdea69p-22)),
        (fbig!(0x10000001p-28), ibig!(100000), fbig!(0x800c3595p-31)),
        (fbig!(0x10000001p-28), ibig!(-100000), fbig!(0xffe79729p-32)),
        (fbig!(0x10000001p-28), ibig!(1000000000), fbig!(0xa5eedf2ep-26)),
        (fbig!(0x10000001p-28), ibig!(-1000000000), fbig!(0xc57a28c2p-37)),
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

#[test]
fn test_powi_decimal() {
    let exact_cases = [
        (dbig!(1), ibig!(0), dbig!(1)),
        (dbig!(1), ibig!(1), dbig!(1)),
        (dbig!(-10), ibig!(1), dbig!(-10)),
        (dbig!(-10), ibig!(-1), dbig!(-1e-1)),
        (dbig!(-10), ibig!(2), dbig!(1e2)),
        (dbig!(-10), ibig!(-2), dbig!(1e-2)),
        (dbig!(-03e-2), ibig!(3), dbig!(-27e-6)),
        (dbig!(-0005e2), ibig!(5), dbig!(-3125e10)),
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
        (dbig!(-123), ibig!(2), dbig!(151e2), NoOp),
        (dbig!(123), ibig!(-2), dbig!(661e-7), AddOne),
        (dbig!(10001e-4), ibig!(100), dbig!(10100e-4), NoOp),
        (dbig!(10001e-4), ibig!(-100), dbig!(99005e-5), NoOp),
        (dbig!(10001e-4), ibig!(10000), dbig!(27181e-4), NoOp),
        (dbig!(10001e-4), ibig!(-10000), dbig!(36790e-5), AddOne),
        (dbig!(10000001e-7), ibig!(10000), dbig!(10010005e-7), NoOp),
        (dbig!(10000001e-7), ibig!(-10000), dbig!(99900050e-8), AddOne),
        (dbig!(10000001e-7), ibig!(10000000), dbig!(27182817e-7), AddOne),
        (dbig!(10000001e-7), ibig!(-10000000), dbig!(36787946e-8), AddOne),
    ];

    for (base, exp, pow, rnd) in &inexact_cases {
        assert_eq!(base.powi(exp.clone()), *pow);
        if let Inexact(v, e) = base.context().powi(base, exp.clone()) {
            assert_eq!(v, *pow);
            assert_eq!(e, *rnd);
        } else {
            panic!("the result should be inexact!")
        }
    }
}

#[test]
fn test_exp_binary() {
    assert_eq!(fbig!(0).exp(), fbig!(1));

    let inexact_cases = [
        (fbig!(0x1), fbig!(0xap-2)),
        (fbig!(0x0001), fbig!(0xadf8p-14)),
        (fbig!(0x0000000000000001), fbig!(0xadf85458a2bb4a9ap-62)),
        (fbig!(1).with_precision(200).value(), fbig!(0xadf85458a2bb4a9aafdc5620273d3cf1d8b9c583ce2d3695a9p-198)),
        (fbig!(-0x1), fbig!(0xbp-5)),
        (fbig!(-0x0001), fbig!(0xbc5ap-17)),
        (fbig!(-0x0000000000000001), fbig!(0xbc5ab1b16779be35p-65)),
        (fbig!(-1).with_precision(200).value(), fbig!(0xbc5ab1b16779be3575bd8f0520a9f21bb5300b556ad8ee6660p-201)),
        (fbig!(0x12p-4), fbig!(0xc5p-6)),
        (fbig!(0x1234p-12), fbig!(0xc7a7p-14)),
        (fbig!(0x123456789p-32), fbig!(0xc7ab41d2cp-34)),
        (fbig!(0x123456789012345678901234567890123456789p-152), fbig!(0xc7ab41d2cef9900a0e4de4219dd6d2aaaee02fap-154)),
        (fbig!(-0x12p-4), fbig!(0xa6p-9)),
        (fbig!(-0x1234p-12), fbig!(0xa420p-17)),
        (fbig!(-0x123456789p-32), fbig!(0xa41c9392bp-37)),
        (fbig!(-0x123456789012345678901234567890123456789p-152), fbig!(0xa41c9392b0c8363d84145dd27bee3ffc01346adp-157)),
    ];
    for (exp, pow) in &inexact_cases {
        assert_eq!(exp.exp(), *pow);
        if let Inexact(v, e) = exp.context().exp(exp) {
            assert_eq!(v, *pow);
            assert_eq!(e, NoOp);
        } else {
            panic!("the result should be inexact!")
        }
    }
}

#[test]
fn test_exp_decimal() {
    assert_eq!(dbig!(0).exp(), dbig!(1));

    let inexact_cases = [
        (dbig!(1), dbig!(3), AddOne),
        (dbig!(0001), dbig!(2718e-3), NoOp),
        (dbig!(0000000000000001), dbig!(2718281828459045e-15), NoOp),
        (dbig!(1).with_precision(60).value(), dbig!(271828182845904523536028747135266249775724709369995957496697e-59), AddOne),
        (dbig!(-1), dbig!(4e-1), AddOne),
        (dbig!(-0001), dbig!(3679e-4), AddOne),
        (dbig!(-0000000000000001), dbig!(3678794411714423e-16), NoOp),
        (dbig!(-1).with_precision(60).value(), dbig!(367879441171442321595523770161460867445811131031767834507837e-60), AddOne),
        (dbig!(12e-1), dbig!(33e-1), NoOp),
        (dbig!(1234e-3), dbig!(3435e-3), AddOne),
        (dbig!(123456789e-8), dbig!(343689308e-8), NoOp),
        (dbig!(123456789012345678901234567890123456789e-38), dbig!(343689308434600800459142431476227568847e-38), NoOp),
        (dbig!(-12e-1), dbig!(30e-2), NoOp),
        (dbig!(-1234e-3), dbig!(2911e-4), NoOp),
        (dbig!(-123456789e-8), dbig!(290960462e-9), NoOp),
        (dbig!(-123456789012345678901234567890123456789e-38), dbig!(290960462097204229206318720257638673836e-39), AddOne),
    ];
    for (exp, pow, rnd) in &inexact_cases {
        assert_eq!(exp.exp(), *pow);
        if let Inexact(v, e) = exp.context().exp(exp) {
            assert_eq!(v, *pow);
            assert_eq!(e, *rnd);
        } else {
            panic!("the result should be inexact!")
        }
    }
}
