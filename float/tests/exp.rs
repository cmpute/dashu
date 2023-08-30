use dashu_base::Approximation::*;
use dashu_float::{round::Rounding::*, DBig};

mod helper_macros;

#[test]
#[rustfmt::skip::macros(fbig)]
fn test_powi_binary() {
    // simple cases
    assert_eq!(fbig!(0x0).powi(ibig!(0)), fbig!(0x1));
    assert_eq!(fbig!(0x0).powi(ibig!(10)), fbig!(0x0));

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
        if let Exact(v) = base.context().powi(base.repr(), exp.clone()) {
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
        if let Inexact(v, e) = base.context().powi(base.repr(), exp.clone()) {
            assert_eq!(v, *pow);
            assert_eq!(e, NoOp);
        } else {
            panic!("the result should be inexact!")
        }
    }
}

#[test]
fn test_powi_decimal() {
    // simple cases
    assert_eq!(dbig!(0).powi(ibig!(0)), dbig!(1));
    assert_eq!(dbig!(0).powi(ibig!(10)), dbig!(0));

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
        if let Exact(v) = base.context().powi(base.repr(), exp.clone()) {
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
        if let Inexact(v, e) = base.context().powi(base.repr(), exp.clone()) {
            assert_eq!(v, *pow);
            assert_eq!(e, *rnd);
        } else {
            panic!("the result should be inexact!")
        }
    }
}

#[test]
#[rustfmt::skip::macros(fbig)]
fn test_powi_unlimited_precision() {
    assert_eq!(fbig!(0x1p-3).with_precision(0).value().powi(ibig!(100)), fbig!(0x1p-300));
    assert_eq!(
        fbig!(0x11p-3).with_precision(0).value().powi(ibig!(100)),
        fbig!(0x1ad6e751d93a86b6ee122b6be4254d4ee2283adf63955e927dd2ccf8c9ed1fceec29ee2d0e93474283c3edae5b313516ad69c41p-300)
    );
}

#[test]
#[should_panic]
#[rustfmt::skip::macros(fbig)]
fn test_powi_unlimited_precision_neg_exp() {
    let _ = fbig!(0x3p-3).with_precision(0).value().powi(ibig!(-100));
}

#[test]
#[rustfmt::skip::macros(fbig)]
fn test_exp_binary() {
    assert_eq!(fbig!(0).exp(), fbig!(1));

    let inexact_cases = [
        (fbig!(0x1), fbig!(0xap-2)),
        (fbig!(0x0001), fbig!(0xadf8p-14)),
        (fbig!(0x0000000000000001), fbig!(0xadf85458a2bb4a9ap-62)),
        (
            fbig!(1).with_precision(200).value(),
            fbig!(0xadf85458a2bb4a9aafdc5620273d3cf1d8b9c583ce2d3695a9p-198),
        ),
        (fbig!(-0x1), fbig!(0xbp-5)),
        (fbig!(-0x0001), fbig!(0xbc5ap-17)),
        (fbig!(-0x0000000000000001), fbig!(0xbc5ab1b16779be35p-65)),
        (
            fbig!(-1).with_precision(200).value(),
            fbig!(0xbc5ab1b16779be3575bd8f0520a9f21bb5300b556ad8ee6660p-201),
        ),
        (fbig!(0x12p-4), fbig!(0xc5p-6)),
        (fbig!(0x1234p-12), fbig!(0xc7a7p-14)),
        (fbig!(0x123456789p-32), fbig!(0xc7ab41d2cp-34)),
        (
            fbig!(0x123456789012345678901234567890123456789p-152),
            fbig!(0xc7ab41d2cef9900a0e4de4219dd6d2aaaee02fap-154),
        ),
        (fbig!(-0x12p-4), fbig!(0xa6p-9)),
        (fbig!(-0x1234p-12), fbig!(0xa420p-17)),
        (fbig!(-0x123456789p-32), fbig!(0xa41c9392bp-37)),
        (
            fbig!(-0x123456789012345678901234567890123456789p-152),
            fbig!(0xa41c9392b0c8363d84145dd27bee3ffc01346adp-157),
        ),
    ];
    for (exp, pow) in &inexact_cases {
        assert_eq!(exp.exp(), *pow);
        if let Inexact(v, e) = exp.context().exp(exp.repr()) {
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
        (
            dbig!(1).with_precision(60).value(),
            dbig!(271828182845904523536028747135266249775724709369995957496697e-59),
            AddOne,
        ),
        (dbig!(-1), dbig!(4e-1), AddOne),
        (dbig!(-0001), dbig!(3679e-4), AddOne),
        (dbig!(-0000000000000001), dbig!(3678794411714423e-16), NoOp),
        (
            dbig!(-1).with_precision(60).value(),
            dbig!(367879441171442321595523770161460867445811131031767834507837e-60),
            AddOne,
        ),
        (dbig!(12e-1), dbig!(33e-1), NoOp),
        (dbig!(1234e-3), dbig!(3435e-3), AddOne),
        (dbig!(123456789e-8), dbig!(343689308e-8), NoOp),
        (
            dbig!(123456789012345678901234567890123456789e-38),
            dbig!(343689308434600800459142431476227568847e-38),
            NoOp,
        ),
        (dbig!(-12e-1), dbig!(30e-2), NoOp),
        (dbig!(-1234e-3), dbig!(2911e-4), NoOp),
        (dbig!(-123456789e-8), dbig!(290960462e-9), NoOp),
        (
            dbig!(-123456789012345678901234567890123456789e-38),
            dbig!(290960462097204229206318720257638673836e-39),
            AddOne,
        ),
    ];
    for (exp, pow, rnd) in &inexact_cases {
        assert_eq!(exp.exp(), *pow);
        if let Inexact(v, e) = exp.context().exp(exp.repr()) {
            assert_eq!(v, *pow);
            assert_eq!(e, *rnd);
        } else {
            panic!("the result should be inexact!")
        }
    }
}

#[test]
#[should_panic]
fn test_exp_unlimited_precision() {
    let _ = dbig!(2).with_precision(0).value().exp();
}

#[test]
#[should_panic]
fn test_exp_inf() {
    let _ = DBig::INFINITY.exp();
}

#[test]
#[rustfmt::skip::macros(fbig)]
fn test_exp_m1_binary() {
    assert_eq!(fbig!(0).exp_m1(), fbig!(0));

    let inexact_cases = [
        (fbig!(0x1), fbig!(0xdp-3)),
        (fbig!(0x0001), fbig!(0xdbf0p-15)),
        (fbig!(0x0000000000000001), fbig!(0xdbf0a8b145769535p-63)),
        (
            fbig!(1).with_precision(200).value(),
            fbig!(0xdbf0a8b1457695355fb8ac404e7a79e3b1738b079c5a6d2b53p-199),
        ),
        (fbig!(-0x1), fbig!(-0xap-4)),
        (fbig!(-0x0001), fbig!(-0xa1d2p-16)),
        (fbig!(-0x0000000000000001), fbig!(-0xa1d2a7274c4320e5p-64)),
        (
            fbig!(-1).with_precision(200).value(),
            fbig!(-0xa1d2a7274c4320e54521387d6fab06f22567fa554a9388cccfp-200),
        ),
        (fbig!(0x12p-8), fbig!(0x95p-11)),
        (fbig!(0x1234p-16), fbig!(0x96edp-19)),
        (fbig!(0x123456789p-36), fbig!(0x96f04c405p-39)),
        (
            fbig!(0x123456789012345678901234567890123456789p-156),
            fbig!(0x96f04c405335d8e869e647249066a2580d2819ap-159),
        ),
        (fbig!(-0x12p-8), fbig!(-0x8bp-11)),
        (fbig!(-0x1234p-16), fbig!(-0x8c91p-19)),
        (fbig!(-0x123456789p-36), fbig!(-0x8c93f7504p-39)),
        (
            fbig!(-0x123456789012345678901234567890123456789p-156),
            fbig!(-0x8c93f7504e1183b008f2ee19d5e1b53169f2458p-159),
        ),
    ];

    for (exp, pow) in &inexact_cases {
        assert_eq!(exp.exp_m1(), *pow);
        if let Inexact(v, e) = exp.context().exp_m1(exp.repr()) {
            assert_eq!(v, *pow);
            assert_eq!(e, NoOp);
        } else {
            panic!("the result should be inexact!")
        }
    }
}

#[test]
fn test_exp_m1_decimal() {
    assert_eq!(dbig!(0).exp_m1(), dbig!(0));

    let inexact_cases = [
        (dbig!(1), dbig!(2), AddOne),
        (dbig!(0001), dbig!(1718e-3), NoOp),
        (dbig!(0000000000000001), dbig!(1718281828459045e-15), NoOp),
        (
            dbig!(1).with_precision(60).value(),
            dbig!(171828182845904523536028747135266249775724709369995957496697e-59),
            AddOne,
        ),
        (dbig!(-1), dbig!(-6e-1), NoOp),
        (dbig!(-0001), dbig!(-6321e-4), NoOp),
        (dbig!(-0000000000000001), dbig!(-6321205588285577e-16), SubOne),
        (
            dbig!(-1).with_precision(60).value(),
            dbig!(-632120558828557678404476229838539132554188868968232165492163e-60),
            NoOp,
        ),
        (dbig!(98e-3), dbig!(10e-2), NoOp),
        (dbig!(9876e-5), dbig!(1038e-4), NoOp),
        (dbig!(987654321e-10), dbig!(103807351e-9), NoOp),
        (
            dbig!(987654321098765432109876543210987654321e-40),
            dbig!(103807351428083631009452051637976395305e-39),
            AddOne,
        ),
        (dbig!(-98e-3), dbig!(-93e-3), NoOp),
        (dbig!(-9876e-5), dbig!(-9404e-5), SubOne),
        (dbig!(-987654321e-10), dbig!(-940448089e-10), SubOne),
        (
            dbig!(-987654321098765432109876543210987654321e-40),
            dbig!(-940448089005565861082145972642612421058e-40),
            NoOp,
        ),
    ];

    for (exp, pow, rnd) in &inexact_cases {
        assert_eq!(exp.exp_m1(), *pow);
        if let Inexact(v, e) = exp.context().exp_m1(exp.repr()) {
            assert_eq!(v, *pow);
            assert_eq!(e, *rnd);
        } else {
            panic!("the result should be inexact!")
        }
    }
}

#[test]
#[should_panic]
fn test_exp_m1_unlimited_precision() {
    let _ = dbig!(2).with_precision(0).value().exp_m1();
}

#[test]
#[should_panic]
fn test_exp_m1_inf() {
    let _ = DBig::INFINITY.exp_m1();
}

#[test]
#[rustfmt::skip::macros(fbig)]
fn test_powf_binary() {
    assert_eq!(fbig!(0).powf(&fbig!(0)), fbig!(1));
    assert_eq!(fbig!(1).powf(&fbig!(0)), fbig!(1));
    assert_eq!(fbig!(0).powf(&fbig!(1)), fbig!(0));
    assert_eq!(fbig!(1).powf(&fbig!(1)), fbig!(1));
    assert_eq!(fbig!(1).powf(&fbig!(-1)), fbig!(1));

    // cases for x^x and x^-x
    let xx_inexact_cases = [
        (fbig!(0x12p-8), fbig!(0xd4p-8), fbig!(0x9ap-7)),
        (fbig!(0x12p-4), fbig!(0x92p-7), fbig!(0xe0p-8)),
        (fbig!(0x1234p-16), fbig!(0xd421p-16), fbig!(0x9a78p-15)),
        (fbig!(0x1234p-8), fbig!(0x9311p61), fbig!(0xdecep-92)),
        (fbig!(0x123456789p-36), fbig!(0xd42103860p-36), fbig!(0x9a78d9b71p-35)),
        (fbig!(0x123456789p-24), fbig!(0xa9f5a6d63p2349), fbig!(0xc0cc7d326p-2420)),
        (
            fbig!(0x123456789012345678901234567890123456789p-156),
            fbig!(0xd42103860f3571cd2a460fb6b4ea8d9b7c731f2p-156),
            fbig!(0x9a78d9b718a5e6b0df4da5a7ae7442e43f3d092p-155),
        ),
        (
            fbig!(0x123456789012345678901234567890123456789p-142),
            fbig!(0xdfa5a59d0656d300e096909463f09b52c76104ap11712),
            fbig!(0x92843df7e9a9b00a08c246f466f9783f1f6f463p-12023),
        ),
    ];

    for (x, pow, npow) in &xx_inexact_cases {
        assert_eq!(x.powf(x), *pow);
        assert_eq!(x.powf(&-x), *npow);
        if let Inexact(v, e) = x.context().powf(x.repr(), x.repr()) {
            assert_eq!(v, *pow);
            assert_eq!(e, NoOp);
        } else {
            panic!("the result should be inexact!")
        }
        if let Inexact(v, e) = x.context().powf(x.repr(), (-x).repr()) {
            assert_eq!(v, *npow);
            assert_eq!(e, NoOp);
        } else {
            panic!("the result should be inexact!")
        }
    }
}

#[test]
fn test_powf_decimal() {
    assert_eq!(dbig!(0).powf(&dbig!(0)), dbig!(1));
    assert_eq!(dbig!(1).powf(&dbig!(0)), dbig!(1));
    assert_eq!(dbig!(0).powf(&dbig!(1)), dbig!(0));
    assert_eq!(dbig!(1).powf(&dbig!(1)), dbig!(1));
    assert_eq!(dbig!(1).powf(&dbig!(-1)), dbig!(1));

    // cases for x^x and x^-x
    let xx_inexact_cases = [
        (dbig!(98e-3), dbig!(80e-2), AddOne, dbig!(13e-1), AddOne),
        (dbig!(9876e-5), dbig!(7956e-4), NoOp, dbig!(1257e-3), AddOne),
        (dbig!(987654321e-10), dbig!(795612734e-9), AddOne, dbig!(125689291e-8), AddOne),
        (
            dbig!(987654321098765432109876543210987654321e-40),
            dbig!(795612733503722716297318660203562998522e-39),
            AddOne,
            dbig!(125689290516530042979504040185410636781e-38),
            NoOp,
        ),
    ];

    for (x, pow, rnd, npow, nrnd) in &xx_inexact_cases {
        assert_eq!(x.powf(x), *pow);
        assert_eq!(x.powf(&-x), *npow);
        if let Inexact(v, e) = x.context().powf(x.repr(), x.repr()) {
            assert_eq!(v, *pow);
            assert_eq!(e, *rnd);
        } else {
            panic!("the result should be inexact!")
        }
        if let Inexact(v, e) = x.context().powf(x.repr(), (-x).repr()) {
            assert_eq!(v, *npow);
            assert_eq!(e, *nrnd);
        } else {
            panic!("the result should be inexact!")
        }
    }
}

#[test]
#[should_panic]
fn test_pow_unlimited_precision() {
    let _ = dbig!(2)
        .with_precision(0)
        .value()
        .powf(&dbig!(2.1).with_precision(0).value());
}

#[test]
#[should_panic]
fn test_pow_inf() {
    let _ = DBig::INFINITY.powf(&dbig!(2));
}
