use dashu_base::Approximation::*;
use dashu_float::{round::Rounding::*, DBig, FBig};
use dashu_int::Word;

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
        if let Ok(Exact(v)) = base.context().powi(base.repr(), exp.clone()) {
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
        if let Ok(Inexact(v, e)) = base.context().powi(base.repr(), exp.clone()) {
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
        if let Ok(Exact(v)) = base.context().powi(base.repr(), exp.clone()) {
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
        if let Ok(Inexact(v, e)) = base.context().powi(base.repr(), exp.clone()) {
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
    assert_eq!(fbig!(0x1p-3).with_precision(0).unwrap().powi(ibig!(100)), fbig!(0x1p-300));
    assert_eq!(
        fbig!(0x11p-3).with_precision(0).unwrap().powi(ibig!(100)),
        fbig!(0x1ad6e751d93a86b6ee122b6be4254d4ee2283adf63955e927dd2ccf8c9ed1fceec29ee2d0e93474283c3edae5b313516ad69c41p-300)
    );
}

#[test]
#[should_panic]
#[rustfmt::skip::macros(fbig)]
fn test_powi_unlimited_precision_neg_exp() {
    let _ = fbig!(0x3p-3).with_precision(0).unwrap().powi(ibig!(-100));
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
            fbig!(1).with_precision(200).unwrap(),
            fbig!(0xadf85458a2bb4a9aafdc5620273d3cf1d8b9c583ce2d3695a9p-198),
        ),
        (fbig!(-0x1), fbig!(0xbp-5)),
        (fbig!(-0x0001), fbig!(0xbc5ap-17)),
        (fbig!(-0x0000000000000001), fbig!(0xbc5ab1b16779be35p-65)),
        (
            fbig!(-1).with_precision(200).unwrap(),
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
        if let Ok(Inexact(v, e)) = exp.context().exp(exp.repr(), None) {
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
            dbig!(1).with_precision(60).unwrap(),
            dbig!(271828182845904523536028747135266249775724709369995957496697e-59),
            AddOne,
        ),
        (dbig!(-1), dbig!(4e-1), AddOne),
        (dbig!(-0001), dbig!(3679e-4), AddOne),
        (dbig!(-0000000000000001), dbig!(3678794411714423e-16), NoOp),
        (
            dbig!(-1).with_precision(60).unwrap(),
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
        if let Ok(Inexact(v, e)) = exp.context().exp(exp.repr(), None) {
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
    let _ = dbig!(2).with_precision(0).unwrap().exp();
}

#[test]
fn test_exp_inf() {
    // exp(+inf) = +inf, exp(-inf) = +0
    assert_eq!(DBig::INFINITY.exp(), DBig::INFINITY);
    assert_eq!(DBig::NEG_INFINITY.exp(), DBig::ZERO);
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
            fbig!(1).with_precision(200).unwrap(),
            fbig!(0xdbf0a8b1457695355fb8ac404e7a79e3b1738b079c5a6d2b53p-199),
        ),
        (fbig!(-0x1), fbig!(-0xap-4)),
        (fbig!(-0x0001), fbig!(-0xa1d2p-16)),
        (fbig!(-0x0000000000000001), fbig!(-0xa1d2a7274c4320e5p-64)),
        (
            fbig!(-1).with_precision(200).unwrap(),
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
        if let Ok(Inexact(v, e)) = exp.context().exp_m1(exp.repr(), None) {
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
            dbig!(1).with_precision(60).unwrap(),
            dbig!(171828182845904523536028747135266249775724709369995957496697e-59),
            AddOne,
        ),
        (dbig!(-1), dbig!(-6e-1), NoOp),
        (dbig!(-0001), dbig!(-6321e-4), NoOp),
        (dbig!(-0000000000000001), dbig!(-6321205588285577e-16), SubOne),
        (
            dbig!(-1).with_precision(60).unwrap(),
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
        if let Ok(Inexact(v, e)) = exp.context().exp_m1(exp.repr(), None) {
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
    let _ = dbig!(2).with_precision(0).unwrap().exp_m1();
}

#[test]
fn test_exp_m1_inf() {
    // exp_m1(+inf) = +inf, exp_m1(-inf) = -1
    assert_eq!(DBig::INFINITY.exp_m1(), DBig::INFINITY);
    assert_eq!(DBig::NEG_INFINITY.exp_m1(), -DBig::ONE);
}

#[test]
#[rustfmt::skip::macros(fbig)]
fn test_powf_binary() {
    assert_eq!(fbig!(0).powf(&fbig!(0)), fbig!(1));
    assert_eq!(fbig!(1).powf(&fbig!(0)), fbig!(1));
    assert_eq!(fbig!(0).powf(&fbig!(1)), fbig!(0));
    assert_eq!(fbig!(1).powf(&fbig!(1)), fbig!(1));
    assert_eq!(fbig!(1).powf(&fbig!(-1)), fbig!(1));
    assert_eq!(fbig!(0).powf(&fbig!(0x2)), fbig!(0));
    assert_eq!(fbig!(1).powf(&fbig!(0x2)), fbig!(1));

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
        if let Ok(Inexact(v, e)) = x.context().powf(x.repr(), x.repr(), None) {
            assert_eq!(v, *pow);
            assert_eq!(e, NoOp);
        } else {
            panic!("the result should be inexact!")
        }
        if let Ok(Inexact(v, e)) = x.context().powf(x.repr(), (-x).repr(), None) {
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
    assert_eq!(dbig!(0).powf(&dbig!(2)), dbig!(0));
    assert_eq!(dbig!(1).powf(&dbig!(2)), dbig!(1));

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
        if let Ok(Inexact(v, e)) = x.context().powf(x.repr(), x.repr(), None) {
            assert_eq!(v, *pow);
            assert_eq!(e, *rnd);
        } else {
            panic!("the result should be inexact!")
        }
        if let Ok(Inexact(v, e)) = x.context().powf(x.repr(), (-x).repr(), None) {
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
        .unwrap()
        .powf(&dbig!(2.1).with_precision(0).unwrap());
}

#[test]
#[should_panic]
fn test_pow_inf() {
    let _ = DBig::INFINITY.powf(&dbig!(2));
}

#[test]
fn test_pow_with_rounding() {
    use dashu_base::Abs;
    use dashu_float::round::{mode::*, ErrorBounds, Round};

    fn test_powf_with_error<R: Round, OpR: ErrorBounds, const B: Word>(
        base: &FBig<R, B>,
        exp: &FBig<R, B>,
        target: &FBig<R, B>,
        atol: &FBig<R, B>,
    ) {
        let result = base
            .clone()
            .with_rounding::<OpR>()
            .powf(&exp.clone().with_rounding::<OpR>());
        let result_err = (result.with_rounding::<R>() - target).abs();
        assert!(result_err <= *atol, "{}^{}, err: {} (>{})", base, exp, result_err, atol);
    }

    let binary_cases = [
        // base, exp, target result
        (fbig!(0x0010), fbig!(0x0001p4), fbig!(0x0001p64)),
        (fbig!(0x0010), fbig!(-0x0001p4), fbig!(0x0001p - 64)),
        (fbig!(0x1234), fbig!(0x1234p - 16), fbig!(0xe960p - 15)),
        (fbig!(0x1234), fbig!(-0x1234p - 16), fbig!(0x8c69p - 16)),
    ];

    for (base, exp, target) in binary_cases {
        test_powf_with_error::<_, Zero, 2>(&base, &exp, &target, &target.ulp());
        test_powf_with_error::<_, Up, 2>(&base, &exp, &target, &target.ulp());
        test_powf_with_error::<_, Down, 2>(&base, &exp, &target, &target.ulp());
        test_powf_with_error::<_, HalfAway, 2>(&base, &exp, &target, &target.ulp());
        test_powf_with_error::<_, HalfEven, 2>(&base, &exp, &target, &target.ulp());
    }
}

// ---------------------------------------------------------------------------------------------
// Regressions from the correct-rounding certification (the Ziv containment test).
// ---------------------------------------------------------------------------------------------

/// A result just below a power of the base was certified against a preimage one binade too
/// wide: `exp`/`powi` of a value a hair below `0.9995` returned `1` at 3 digits, where the
/// correctly rounded value is `0.999`.
#[test]
fn test_exp_powi_round_just_below_power_of_base() {
    use core::str::FromStr;
    use dashu_float::{round::mode::HalfEven, Context};
    type D = FBig<HalfEven, 10>;

    let hi = Context::<HalfEven>::new(100);
    let ctx = Context::<HalfEven>::new(3);
    for (t, expected) in [
        ("0.99949999999999999999999999999999999999999", "0.999"),
        ("0.99950000000000000000000000000000000000001", "1"),
    ] {
        let t = D::from_str(t).unwrap();
        let expected = D::from_str(expected).unwrap();

        // x with x² ≈ t: the exact square (unlimited precision) is the oracle
        let x = hi.sqrt(t.repr()).unwrap().value();
        let x_exact = D::from_repr(x.repr().clone(), Context::new(0));
        assert_eq!((&x_exact * &x_exact).with_precision(3).value(), expected);
        assert_eq!(ctx.powi(x.repr(), 2.into()).unwrap().value(), expected);

        let l = hi.ln(t.repr(), None).unwrap().value();
        assert_eq!(ctx.exp(l.repr(), None).unwrap().value(), expected);
    }
}

/// An exact power that is a rounding tie (`5² = 25` at one digit) was rejected by the
/// certification and surfaced as `ZivRetryLimitExceeded` (a panic through `FBig::powi`).
#[test]
fn test_powi_exact_tie() {
    use core::str::FromStr;
    use dashu_float::{
        round::mode::{HalfAway, HalfEven},
        Context,
    };
    type D = FBig<HalfEven, 10>;

    for (x, p, even, away) in [
        ("5", 1usize, "20", "30"),
        ("1.5", 2, "2.2", "2.3"),
        ("3.5", 3, "12.2", "12.3"),
        ("5", 2, "25", "25"),
        ("1.4", 2, "2", "2"),
    ] {
        let a = D::from_str(x).unwrap();
        let got = Context::<HalfEven>::new(p).powi(a.repr(), 2.into());
        assert_eq!(got.unwrap().value(), D::from_str(even).unwrap(), "{x}^2 at {p} digits");
        let got = Context::<HalfAway>::new(p).powi(a.repr(), 2.into());
        assert_eq!(got.unwrap().value(), DBig::from_str(away).unwrap(), "{x}^2 at {p} digits");
    }
    // through the convenience API (this panicked)
    let five = D::from_str("5").unwrap().with_precision(1).value();
    assert_eq!(five.powi(2.into()), D::from_str("20").unwrap());
}

/// `powi` with a negative exponent agrees with the correctly rounded division `1 / baseⁿ`, flag
/// included. It used to report `Exact` whenever the working-precision value happened to round to
/// itself (`183⁻⁸` at 53 bits), and misround next to a power of two (`15⁻¹⁶` at 1 bit gave
/// `2⁻⁶²` for `0.70·2⁻⁶²`).
#[test]
fn test_powi_negative_exponent_matches_division() {
    use dashu_float::{
        round::{mode, ErrorBounds},
        Context, Repr,
    };
    use dashu_int::IBig;

    fn check<R: ErrorBounds, const B: Word>(p: usize, base: i64, n: usize) {
        let ctx = Context::<R>::new(p);
        let power = Repr::<B>::new(IBig::from(base).pow(n), 0);
        let want = ctx.div(&Repr::one(), &power);
        let got = ctx.powi(&Repr::<B>::new(base.into(), 0), -IBig::from(n));
        assert_eq!(got, want, "{base}^-{n} at {p} digits in base {B}");
    }
    fn check_all_modes<const B: Word>(p: usize, base: i64, n: usize) {
        check::<mode::Zero, B>(p, base, n);
        check::<mode::Away, B>(p, base, n);
        check::<mode::Down, B>(p, base, n);
        check::<mode::Up, B>(p, base, n);
        check::<mode::HalfEven, B>(p, base, n);
        check::<mode::HalfAway, B>(p, base, n);
    }

    // the reported inputs
    check::<mode::HalfEven, 2>(1, 15, 16);
    check::<mode::Away, 2>(3, 1187, 15);
    check::<mode::HalfEven, 2>(1, 13, 7);
    check::<mode::HalfEven, 2>(52, 30879, 6);
    check::<mode::HalfEven, 2>(53, 183, 8);

    for base in [-7i64, 2, 3, 5, 10, 15, 183, 1187, 30879] {
        for n in [2usize, 3, 7, 8, 15, 16] {
            for p in [1usize, 2, 3, 20, 50, 100, 500] {
                check_all_modes::<2>(p, base, n);
            }
            for p in [1usize, 2, 3, 6, 15, 30, 150] {
                check_all_modes::<10>(p, base, n);
            }
        }
    }
}

/// `powf(x, 0.5)` agrees with the correctly rounded `sqrt(x)` for a non-square `x`. At one bit of
/// precision every result is a power of two, and a value just below one (`√15.994 ≈ 3.9993`)
/// was certified as the next power up (`8` under `Away`/`Up`).
#[test]
fn test_powf_half_matches_sqrt() {
    use dashu_float::{
        round::{mode, ErrorBounds},
        Context, Repr,
    };
    use dashu_int::IBig;

    fn check<R: ErrorBounds>(p: usize, sig: i64, exp: isize) {
        let half = Repr::<2>::new(IBig::ONE, -1);
        let x = Repr::<2>::new(sig.into(), exp);
        let ctx = Context::<R>::new(p);
        let want = ctx.sqrt(&x);
        assert_eq!(ctx.powf(&x, &half, None), want, "({sig}·2^{exp})^0.5 at {p} bits");
    }
    for (sig, exp) in [
        (8189i64, -9isize),
        (8191, -9),
        (8187, -9),
        (16263, 80),
        (5, 0),
        (3, 1),
    ] {
        for p in [1usize, 2, 20, 50, 100, 500] {
            check::<mode::Zero>(p, sig, exp);
            check::<mode::Away>(p, sig, exp);
            check::<mode::Down>(p, sig, exp);
            check::<mode::Up>(p, sig, exp);
            check::<mode::HalfEven>(p, sig, exp);
            check::<mode::HalfAway>(p, sig, exp);
        }
    }
}

/// The exact special cases carry the context like every other result. They used to return the
/// precision-0 constants (`FBig::ONE` / `FBig::ZERO`), so the next limited-precision operation
/// on the result of e.g. `exp(0)` or `x.powi(0)` panicked with "precision cannot be 0".
#[test]
fn test_exact_shortcuts_keep_precision() {
    use dashu_float::{round::mode, Context, Repr};
    use dashu_int::IBig;

    fn check<const B: Word>(p: usize) {
        type F<const B: Word> = FBig<mode::HalfEven, B>;
        let zero = F::<B>::ZERO.with_precision(p).value();
        let one = F::<B>::ONE.with_precision(p).value();
        let three = F::<B>::from(3u8).with_precision(p).value();
        let exact_shortcuts = [
            (zero.exp(), F::<B>::ONE),
            (zero.exp_m1(), F::<B>::ZERO),
            ((-zero.clone()).exp_m1(), F::<B>::ZERO),
            (three.powi(IBig::ZERO), F::<B>::ONE),
            (zero.powi(IBig::ZERO), F::<B>::ONE),
            (three.powf(&zero), F::<B>::ONE),
            (three.powf(&-zero.clone()), F::<B>::ONE),
            (one.powf(&three.sqrt()), F::<B>::ONE),
            (zero.powf(&three), F::<B>::ZERO),
        ];
        for (i, (got, want)) in exact_shortcuts.iter().enumerate() {
            assert_eq!(got, want, "case {i} at {p} digits in base {B}");
            assert_eq!(got.precision(), p, "case {i} at {p} digits in base {B}");
        }
        assert!((-zero.clone()).exp_m1().repr().is_neg_zero());

        // chained operations must not panic
        assert_eq!(zero.exp().exp(), one.exp());
        assert_eq!(three.powi(IBig::ZERO).powi(IBig::from(-1)), one);
        assert_eq!(three.powi(IBig::ZERO).exp(), one.exp());
        assert_eq!(one.powf(&three).ln_1p(), (&one + &one).ln());

        // the Context layer reports them at its own precision
        let ctx = Context::<mode::HalfEven>::new(p);
        let exp_m1_neg_inf = ctx
            .exp_m1(&Repr::<B>::neg_infinity(), None)
            .unwrap()
            .value();
        assert_eq!(exp_m1_neg_inf, F::<B>::NEG_ONE);
        assert_eq!(exp_m1_neg_inf.precision(), p);
    }
    for p in [1, 20, 50, 100, 500] {
        check::<2>(p);
        check::<10>(p);
    }

    // an unlimited-precision zero still has its exact results (at unlimited precision)
    assert_eq!(DBig::ZERO.exp(), DBig::ONE);
    assert_eq!(DBig::ZERO.exp().precision(), 0);
    assert_eq!(DBig::ZERO.exp_m1(), DBig::ZERO);
}

// ---------------------------------------------------------------------------------------------
// exp over the whole argument range
// ---------------------------------------------------------------------------------------------

mod exp_range {
    use core::str::FromStr;
    use dashu_base::{Approximation::*, Sign};
    use dashu_float::{
        round::{mode, ErrorBounds, Rounding::*},
        Context, FBig, FpError, FpResult, Repr,
    };
    use dashu_int::{IBig, Word};

    /// `exp(x)` from `exp(x/2)²`: the half argument is exact in an even base and its exponential
    /// sits in the middle of the exponent range, where it is computed with 40 extra digits; the
    /// square is rounded once, on the significands, and the exponents are added with a range
    /// check — the library's convention being that a finite value has its (normalized) exponent
    /// strictly between `isize::MIN` and `isize::MAX`.
    fn exp_by_squaring<R: ErrorBounds, const B: Word>(
        p: usize,
        x: &Repr<B>,
    ) -> FpResult<FBig<R, B>> {
        assert!(B % 2 == 0);
        let half_x = Repr::<B>::new(x.significand() * IBig::from(B / 2), x.exponent() - 1);
        let half = Context::<mode::HalfEven>::new(p + 40)
            .exp(&half_x, None)
            .unwrap()
            .value();
        let unit = Repr::<B>::new(half.repr().significand().clone(), 0);
        let square = Context::<R>::new(p).mul(&unit, &unit).unwrap();
        let exponent =
            square.value_ref().repr().exponent() as i128 + 2 * half.repr().exponent() as i128;
        if exponent >= isize::MAX as i128 {
            Err(FpError::Overflow(Sign::Positive))
        } else if exponent <= isize::MIN as i128 {
            Err(FpError::Underflow(Sign::Positive))
        } else {
            Ok(square.map(|v| FBig::from_parts(v.repr().significand().clone(), exponent as isize)))
        }
    }

    fn check<R: ErrorBounds, const B: Word>(p: usize, x: &Repr<B>) {
        let got = Context::<R>::new(p).exp(x, None);
        assert_eq!(got, exp_by_squaring::<R, B>(p, x), "exp({x:?}) at {p} digits");
    }

    fn check_modes<const B: Word>(p: usize, x: &Repr<B>) {
        check::<mode::HalfEven, B>(p, x);
        check::<mode::Zero, B>(p, x);
        check::<mode::Up, B>(p, x);
    }

    /// Regressions, all for arguments around `±6e18` whose result is far inside or next to the
    /// end of the exponent range:
    /// - at a low precision the argument was rounded to the working precision before the
    ///   reduction and the result came back as `Err(Underflow)`;
    /// - a result within the guard digits of the smallest exponent did not terminate (from
    ///   222 bits on for `x = -6243314768165359·2^10`);
    /// - a result within the guard digits of the largest exponent was reported as an overflow,
    ///   or did not terminate just above `ln(10)·isize::MAX`.
    #[test]
    #[cfg(target_pointer_width = "64")]
    fn test_exp_huge_arguments() {
        let binary = [
            Repr::<2>::new(IBig::from(-5764607523034234881i64), 0),
            Repr::<2>::new(IBig::from(-6243314768165359i64), 10),
            Repr::<2>::new(IBig::from(6243314768165359i64), 10),
            Repr::<2>::new(IBig::from(-5), 60),
            Repr::<2>::new(IBig::from(5), 60),
            Repr::<2>::new(IBig::from(-12345), 3),
        ];
        for x in &binary {
            for p in [1usize, 2, 4, 20, 50, 53, 100, 221, 222, 230, 500] {
                check_modes::<2>(p, x);
            }
        }
        let decimal = [
            "21237598959199934314",
            "21237597959199934514",
            "21237598949199934514",
            "21237598959199934507",
            "21237598959199934508",
            "21237598959199934509",
            "21237598959199934510",
            "21237598959199934514",
            "21237598959199934600",
        ];
        for x in decimal {
            let x = Repr::<10>::new(IBig::from_str(x).unwrap(), 0);
            for p in [1usize, 3, 5, 6, 7, 30] {
                check_modes::<10>(p, &x);
                check_modes::<10>(p, &-x.clone());
            }
        }

        // spelled out: a 1-bit result nowhere near the limits, and the largest decimal results
        let x = Repr::<2>::new(IBig::from(-5764607523034234881i64), 0);
        let got = Context::<mode::HalfEven>::new(1).exp(&x, None);
        assert_eq!(got, Ok(Inexact(FBig::from_parts(1.into(), -8316570686152701559), NoOp)));
        let ctx = Context::<mode::HalfEven>::new(3);
        let x = Repr::<10>::new(IBig::from_str("21237598959199934314").unwrap(), 0);
        let got = ctx.exp(&x, None);
        assert_eq!(got, Ok(Inexact(FBig::from_parts(895.into(), isize::MAX - 87), AddOne)));
        // 6.47·10^(isize::MAX + 2): the exponent of the 3-digit significand is the sentinel
        let x = Repr::<10>::new(IBig::from_str("21237598959199934514").unwrap(), 0);
        assert_eq!(ctx.exp(&x, None), Err(FpError::Overflow(Sign::Positive)));
    }

    /// An argument beyond every finite result is a range error at once, whatever its exponent.
    /// Regression: the time to report it grew with the exponent (`exp(2^100000000)` took
    /// minutes), also through `sinh`/`cosh`.
    #[test]
    fn test_exp_astronomical_arguments() {
        let ctx = Context::<mode::Up>::new(2);
        for e in [100isize, 10_000, 100_000_000, isize::MAX - 1] {
            let x = Repr::<2>::new(IBig::ONE, e);
            let neg = -x.clone();
            assert_eq!(ctx.exp(&x, None), Err(FpError::Overflow(Sign::Positive)));
            assert_eq!(ctx.exp(&neg, None), Err(FpError::Underflow(Sign::Positive)));
            assert_eq!(ctx.exp_m1(&x, None), Err(FpError::Overflow(Sign::Positive)));
            // exp_m1(-huge) is just above -1: -0.75 is the next 2-bit value up
            let got = ctx.exp_m1(&neg, None);
            assert_eq!(got, Ok(Inexact(FBig::from_parts((-3).into(), -2), AddOne)));
            assert_eq!(ctx.sinh(&x, None), Err(FpError::Overflow(Sign::Positive)));
            assert_eq!(ctx.sinh(&neg, None), Err(FpError::Overflow(Sign::Negative)));
            assert_eq!(ctx.cosh(&neg, None), Err(FpError::Overflow(Sign::Positive)));

            let x = Repr::<10>::new(IBig::ONE, e);
            let ctx = Context::<mode::HalfEven>::new(7);
            assert_eq!(ctx.exp(&x, None), Err(FpError::Overflow(Sign::Positive)));
            assert_eq!(ctx.exp(&-x, None), Err(FpError::Underflow(Sign::Positive)));
        }
        // through the convenience API: the directed endpoints
        let x = FBig::<mode::Up, 10>::from_str("1e100000000").unwrap();
        assert!(x.exp().repr().is_infinite());
        assert_eq!((-x).exp().repr(), &Repr::new(IBig::ONE, isize::MIN));
    }

    /// A tiny argument: `exp(x) = 1 + x + …` and `exp_m1(x) = x + x²/2 + …` round like the
    /// leading term plus a sticky part, at a cost that does not depend on how tiny `x` is. (The
    /// oracle rounds the leading term with one unit added a few digits below the precision.)
    #[test]
    fn test_exp_tiny_arguments() {
        fn check<R: ErrorBounds, const B: Word>(p: usize, k: isize) {
            let ctx = Context::<R>::new(p);
            let pad = p + 3;
            // `v ± B^(v.exponent − pad)`, exactly, rounded to the precision
            let nudged = |v: &Repr<B>, unit: i32| {
                let sig = v.significand() * IBig::from(B).pow(pad) + IBig::from(unit);
                let exact = Repr::<B>::new(sig, v.exponent() - pad as isize);
                FBig::<R, B>::from_repr(exact, Context::new(0)).with_precision(p)
            };
            for sig in [1i32, -1, 7, -7] {
                let x = Repr::<B>::new(sig.into(), -k);
                // exp(x) rounds like 1 + x
                let want = nudged(&Repr::one(), sig.signum());
                assert_eq!(ctx.exp(&x, None), Ok(want), "exp({x:?}) at {p} digits");
                // exp_m1(x) rounds like x + x²/2
                let want = nudged(&x, 1);
                assert_eq!(ctx.exp_m1(&x, None), Ok(want), "exp_m1({x:?}) at {p} digits");
            }
        }
        for p in [1usize, 20, 50, 100, 500] {
            for k in [2 * p as isize + 8, 5_000, 1_000_000_000, isize::MAX / 4] {
                check::<mode::Zero, 2>(p, k);
                check::<mode::Away, 2>(p, k);
                check::<mode::Down, 2>(p, k);
                check::<mode::Up, 2>(p, k);
                check::<mode::HalfEven, 2>(p, k);
                check::<mode::HalfAway, 10>(p, k);
                check::<mode::Up, 10>(p, k);
                check::<mode::Down, 10>(p, k);
            }
        }

        // around the shortcut's threshold the direct rounding agrees with the series evaluated
        // at a much higher precision
        fn check_threshold<R: ErrorBounds, const B: Word>(p: usize) {
            let (ctx, hi) = (Context::<R>::new(p), Context::<mode::HalfEven>::new(3 * p + 60));
            let round =
                |v: FBig<mode::HalfEven, B>| v.with_rounding::<R>().with_precision(p).value();
            for k in (p as isize / 2).max(1)..p as isize + 8 {
                for sig in [1i32, -1, 9, -9] {
                    let x = Repr::<B>::new(sig.into(), -k);
                    let want = round(hi.exp(&x, None).unwrap().value());
                    let got = ctx.exp(&x, None).unwrap().value();
                    assert_eq!(got, want, "exp({x:?}) at {p} digits");
                    let want = round(hi.exp_m1(&x, None).unwrap().value());
                    let got = ctx.exp_m1(&x, None).unwrap().value();
                    assert_eq!(got, want, "exp_m1({x:?}) at {p} digits");
                }
            }
        }
        for p in [1usize, 20, 50, 100] {
            check_threshold::<mode::Zero, 2>(p);
            check_threshold::<mode::Up, 2>(p);
            check_threshold::<mode::HalfEven, 2>(p);
            check_threshold::<mode::HalfAway, 10>(p);
            check_threshold::<mode::Down, 10>(p);
        }
    }
}

// ---------------------------------------------------------------------------------------------
// powf / powi: exact results, huge exponents, and the ends of the exponent range
// ---------------------------------------------------------------------------------------------

mod pow_range {
    use core::str::FromStr;
    use dashu_base::{Approximation::*, Sign};
    use dashu_float::{
        round::{mode, ErrorBounds, Rounding::*},
        Context, DBig, FBig, FpError, Repr,
    };
    use dashu_int::{IBig, UBig, Word};

    /// `powf(z^d, n/d)` against `z^n` computed exactly (`n ≥ 0`) or as the correctly rounded
    /// quotient `1/z^|n|` (`n < 0`). `inv_d` is `1/d` as an exact value of the base.
    fn check_root_power<R: ErrorBounds, const B: Word>(
        p: usize,
        z: &Repr<B>,
        d: usize,
        inv_d: &Repr<B>,
        n: i32,
    ) {
        let ctx = Context::<R>::new(p);
        let exact = Context::<R>::new(0);
        let x = exact.powi(z, d.into()).unwrap().value();
        let y = Repr::<B>::new(inv_d.significand() * IBig::from(n), inv_d.exponent());
        let power = exact.powi(z, n.unsigned_abs().into()).unwrap().value();
        let want = if n >= 0 {
            Ok(power.with_precision(p))
        } else {
            ctx.div(&Repr::one(), power.repr())
        };
        let got = ctx.powf(x.repr(), &y, None);
        assert_eq!(got, want, "({z:?}^{d})^({n}/{d}) at {p} digits");
    }

    fn check_root_power_modes<const B: Word>(
        p: usize,
        z: &Repr<B>,
        d: usize,
        inv_d: &Repr<B>,
        n: i32,
    ) {
        check_root_power::<mode::Zero, B>(p, z, d, inv_d, n);
        check_root_power::<mode::Away, B>(p, z, d, inv_d, n);
        check_root_power::<mode::Down, B>(p, z, d, inv_d, n);
        check_root_power::<mode::Up, B>(p, z, d, inv_d, n);
        check_root_power::<mode::HalfEven, B>(p, z, d, inv_d, n);
        check_root_power::<mode::HalfAway, B>(p, z, d, inv_d, n);
    }

    /// A power with a non-integer exponent that is exactly representable (or an exact tie) is
    /// returned correctly rounded and flagged, in every mode. Regression: `powf(4, 0.5)` and
    /// `powf(0.25, 1.5)` did not terminate under `Down`/`Up`/`Zero` (the default mode of
    /// `FBig`), and under the nearest modes `2` came back flagged `Inexact`.
    #[test]
    fn test_powf_exact_results() {
        // base 2: d ∈ {2, 4, 8}
        for (d, inv_d) in [(2usize, (1, -1)), (4, (1, -2)), (8, (1, -3))] {
            let inv_d = Repr::<2>::new(inv_d.0.into(), inv_d.1);
            for sig in [1i32, 2, 3, 5, 7, 12] {
                for exp in [-3isize, 0, 2] {
                    let z = Repr::<2>::new(sig.into(), exp);
                    if z.is_one() {
                        continue;
                    }
                    for n in [-3i32, -1, 1, 3, 5] {
                        for p in [1usize, 2, 3, 20, 50, 100, 500] {
                            check_root_power_modes::<2>(p, &z, d, &inv_d, n);
                        }
                    }
                }
            }
        }
        // base 10: 1/2 = 0.5, 1/4 = 0.25, 1/5 = 0.2
        for (d, inv_d) in [(2usize, (5, -1)), (4, (25, -2)), (5, (2, -1))] {
            let inv_d = Repr::<10>::new(inv_d.0.into(), inv_d.1);
            for sig in [2i32, 5, 15, 25, 3] {
                for exp in [-2isize, 0, 1] {
                    let z = Repr::<10>::new(sig.into(), exp);
                    for n in [-3i32, -1, 1, 3, 7] {
                        if n.unsigned_abs() as usize % d == 0 {
                            continue;
                        }
                        for p in [1usize, 2, 3, 6, 15, 30, 150] {
                            check_root_power_modes::<10>(p, &z, d, &inv_d, n);
                        }
                    }
                }
            }
        }
        // bases with a square factor: 4^0.5 = 2 in base 4, 3 = 9^0.5 in base 9
        let half4 = Repr::<4>::new(2.into(), -1);
        check_root_power_modes::<4>(3, &Repr::new(2.into(), 0), 2, &half4, 1);
        check_root_power_modes::<4>(3, &Repr::new(6.into(), -1), 2, &half4, 3);
        let third9 = Repr::<9>::new(3.into(), -1);
        check_root_power_modes::<9>(3, &Repr::new(3.into(), 0), 3, &third9, 1);
        check_root_power_modes::<9>(3, &Repr::new(2.into(), 1), 3, &third9, 2);

        // the reported calls
        let x = FBig::<mode::Zero, 2>::from(4u32).with_precision(53).value();
        let half = FBig::<mode::Zero, 2>::from_parts(1.into(), -1);
        assert_eq!(x.powf(&half), FBig::<mode::Zero, 2>::from(2u32));
        let (four, half) = (Repr::<2>::new(4.into(), 0), Repr::<2>::new(1.into(), -1));
        let got = Context::<mode::Down>::new(2).powf(&four, &half, None);
        assert_eq!(got, Ok(Exact(FBig::from(2u8))));
        let got = Context::<mode::Up>::new(20).powf(&four, &half, None);
        assert_eq!(got, Ok(Exact(FBig::from(2u8))));
        let got = Context::<mode::HalfEven>::new(2).powf(&four, &half, None);
        assert_eq!(got, Ok(Exact(FBig::from(2u8))));
        let (quarter, y) = (Repr::<2>::new(1.into(), -2), Repr::<2>::new(3.into(), -1));
        let got = Context::<mode::Zero>::new(20).powf(&quarter, &y, None);
        assert_eq!(got, Ok(Exact(FBig::from_parts(1.into(), -3))));
        // an exact result beyond the exponent range is a range error, not a hang
        let x = Repr::<2>::new(1.into(), isize::MAX - 1);
        let y = Repr::<2>::new(3.into(), -1);
        let got = Context::<mode::Zero>::new(20).powf(&x, &y, None);
        assert_eq!(got, Err(FpError::Overflow(Sign::Positive)));
    }

    /// `(±1)^n` is `±1` for any exponent. Regression: with a base of `-1` and an exponent past
    /// `2^64` the call did not return under `Down`/`Up`.
    #[test]
    fn test_powi_unit_base_huge_exponent() {
        fn check<R: ErrorBounds, const B: Word>() {
            let ctx = Context::<R>::new(10);
            let (one, neg_one) = (Repr::<B>::one(), Repr::<B>::neg_one());
            let big = IBig::ONE << 64;
            for (n, odd) in [
                (big.clone(), false),
                (big.clone() + IBig::ONE, true),
                (IBig::from(1_000_000_001), true),
            ] {
                for n in [n.clone(), -n] {
                    assert_eq!(ctx.powi(&one, n.clone()), Ok(Exact(FBig::ONE)));
                    let want = if odd { FBig::NEG_ONE } else { FBig::ONE };
                    assert_eq!(ctx.powi(&neg_one, n.clone()), Ok(Exact(want)), "(-1)^{n}");
                }
            }
        }
        check::<mode::Down, 10>();
        check::<mode::Up, 10>();
        check::<mode::Zero, 2>();
        check::<mode::HalfEven, 2>();
    }

    /// An exponent past `i64` is not by itself a range error. Regression: `0.999^(2^63)` — about
    /// `10^(-4.0e15)`, one factor of `0.999` below the correctly computed `0.999^(2^63 - 1)` —
    /// was reported as an underflow (0 through `FBig::powi`), also through `powf`.
    #[test]
    #[cfg(target_pointer_width = "64")]
    fn test_powi_exponent_past_i64() {
        let x = DBig::from_str("0.999").unwrap();
        let hi = Context::<mode::HalfAway>::new(40);
        let two63 = IBig::ONE << 63;
        // x^(2^63) = x^(2^63 - 1) · x, and x^(2^64) = (x^(2^63))²
        let below = hi
            .powi(x.repr(), two63.clone() - IBig::ONE)
            .unwrap()
            .value();
        let want = (below * &x).with_precision(6).value();
        let at = x.clone().with_precision(6).value().powi(two63.clone());
        assert_eq!(at, want);
        assert_eq!(at.repr().exponent(), -4007663746158940);
        let y = Repr::<10>::new(two63.clone(), 0);
        let got = Context::<mode::HalfAway>::new(6).powf(x.repr(), &y, None);
        assert_eq!(got.unwrap().value(), want);

        let at_hi = hi.powi(x.repr(), two63.clone()).unwrap().value();
        let want = hi
            .sqr(at_hi.repr())
            .unwrap()
            .value()
            .with_precision(6)
            .value();
        let got = Context::<mode::HalfAway>::new(6).powi(x.repr(), two63 << 1);
        assert_eq!(got.unwrap().value(), want);

        // still a range error where it is one
        let got = Context::<mode::HalfAway>::new(6).powi(x.repr(), IBig::ONE << 80);
        assert_eq!(got, Err(FpError::Underflow(Sign::Positive)));
        let got = Context::<mode::HalfAway>::new(6).powi(x.repr(), -(IBig::ONE << 80));
        assert_eq!(got, Err(FpError::Overflow(Sign::Positive)));
    }

    /// `(1 + 10^-k)^(10^k + 1) ≈ e` for an exponent beyond the squaring chain. Regression: from
    /// `k = 20` (an exponent above `2^64`) the call took seconds to minutes and then panicked
    /// with "out of memory" at `k = 22` — an attempt whose working precision cannot resolve the
    /// base yet carries an astronomically wide error radius, and the certification test
    /// materialized it.
    #[test]
    fn test_powi_base_near_one_huge_exponent() {
        let e15 = DBig::from_str("2.71828182845905").unwrap();
        for k in [19usize, 20, 22, 40, 300] {
            let pow10 = UBig::from(10u8).pow(k);
            let x = Repr::<10>::new(IBig::from(pow10.clone() + UBig::ONE), -(k as isize));
            let n = IBig::from(pow10 + UBig::ONE);
            let got = Context::<mode::Up>::new(1).powi(&x, n.clone());
            assert_eq!(got, Ok(Inexact(FBig::from(3), AddOne)), "k = {k}");
            let got = Context::<mode::Zero>::new(1).powi(&x, n.clone());
            assert_eq!(got, Ok(Inexact(FBig::from(2), NoOp)), "k = {k}");
            let got = Context::<mode::HalfAway>::new(15).powi(&x, n.clone());
            assert_eq!(got.unwrap().value(), e15, "k = {k}");
            let got = Context::<mode::HalfAway>::new(15).powi(&x, -n);
            let want = Context::<mode::HalfAway>::new(15).div(&Repr::one(), e15.repr());
            assert_eq!(got.unwrap().value(), want.unwrap().value(), "k = {k}");
        }
    }

    /// `powi` does not depend on where the base sits in the exponent range: `(m·B^j)^n` is
    /// `m^n` with the exponent moved by `j·n`, or a range error exactly when that exponent is out
    /// of range. Regression: `(3·2^j)^-2` was reported as an underflow (0 through `FBig::powi`)
    /// when the result was within the working precision's guard digits of `isize::MIN`.
    #[test]
    fn test_powi_is_exponent_invariant() {
        fn check<R: ErrorBounds, const B: Word>(p: usize, m: i32, j: isize, n: i32) {
            let ctx = Context::<R>::new(p);
            let unit = ctx.powi(&Repr::<B>::new(m.into(), 0), n.into()).unwrap();
            let exponent = unit.value_ref().repr().exponent() as i128 + j as i128 * n as i128;
            let sign = unit.value_ref().repr().sign();
            let want = if exponent >= isize::MAX as i128 {
                Err(FpError::Overflow(sign))
            } else if exponent <= isize::MIN as i128 {
                Err(FpError::Underflow(sign))
            } else {
                Ok(unit
                    .map(|v| FBig::from_parts(v.repr().significand().clone(), exponent as isize)))
            };
            let got = ctx.powi(&Repr::<B>::new(m.into(), j), n.into());
            assert_eq!(got, want, "({m}·{B}^{j})^{n} at {p} digits");
        }
        for (m, n) in [
            (3i32, -2i32),
            (3, 2),
            (-3, 3),
            (-7, -3),
            (15, -16),
            (5, 4),
            (2, -2),
        ] {
            let half = isize::MAX / n.unsigned_abs() as isize;
            for offset in [-600isize, -74, -72, -40, -6, -2, 0, 2, 40, 600] {
                for j in [half + offset, -half + offset] {
                    for p in [1usize, 10, 20, 50, 100, 200, 500] {
                        check::<mode::Down, 2>(p, m, j, n);
                        check::<mode::HalfEven, 2>(p, m, j, n);
                        check::<mode::Up, 10>(p, m, j, n);
                        check::<mode::HalfAway, 10>(p, m, j, n);
                    }
                }
            }
        }

        // the reported shape: 1/(9·2^(2j)) just above the smallest exponent
        let ctx = Context::<mode::Down>::new(10);
        for k in [4isize, 72, 74, 1000] {
            let j = -((isize::MIN + k + 12) / 2);
            let base = Repr::<2>::new(3.into(), j);
            let expected = ctx
                .div::<2>(&Repr::one(), &Repr::new(9.into(), 2 * j))
                .unwrap();
            assert_eq!(expected.value_ref().repr().exponent(), isize::MIN + k);
            assert_eq!(ctx.powi::<2>(&base, (-2).into()), Ok(expected), "isize::MIN + {k}");
        }
    }

    /// `powf` next to the smallest exponent: `2^(isize::MIN + k + 1/2)` has the binary order of
    /// magnitude `isize::MIN + k` (or underflows when its 66-bit significand does not fit above
    /// the smallest exponent). Regression: for `k ≤ 260` the result was the same value near
    /// `2^(isize::MIN + 260)` — up to `2^260` times too large.
    #[test]
    fn test_powf_near_smallest_exponent() {
        let two = Repr::<2>::new(2.into(), 0);
        let sqrt2 = Context::<mode::Up>::new(66).sqrt(&two).unwrap().value();
        for k in [0i128, 60, 64, 65, 66, 100, 200, 260, 261, 1000] {
            let target = isize::MIN as i128 + k;
            let y = Repr::<2>::new(IBig::from(2 * target + 1), -1);
            let got = Context::<mode::Up>::new(66).powf(&two, &y, None);
            // √2·2^target: the (normalized) significand of √2 at 66 bits, moved by target
            if sqrt2.repr().exponent() as i128 + target <= isize::MIN as i128 {
                assert_eq!(got, Err(FpError::Underflow(Sign::Positive)), "k = {k}");
            } else {
                let want = FBig::<mode::Up, 2>::from_parts(
                    sqrt2.repr().significand().clone(),
                    sqrt2.repr().exponent() + target as isize,
                );
                assert_eq!(got, Ok(Inexact(want, AddOne)), "k = {k}");
            }
        }
        // through the convenience API the underflow is the directed endpoint
        let y = FBig::<mode::Up, 2>::from_parts(IBig::from(2 * (isize::MIN as i128) + 1), -1)
            .with_precision(66)
            .value();
        let r = FBig::<mode::Up, 2>::from(2u8)
            .with_precision(66)
            .value()
            .powf(&y);
        assert_eq!(r.repr(), &Repr::new(IBig::ONE, isize::MIN));
    }
}
