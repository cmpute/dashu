use dashu_base::{Approximation::*, EstimatedLog2};
use dashu_float::{round::Rounding::*, DBig, FBig};
use dashu_int::Word;

mod helper_macros;

#[test]
#[rustfmt::skip::macros(fbig)]
fn test_ln_binary() {
    assert_eq!(fbig!(1).ln(), fbig!(0));

    let inexact_cases = [
        (fbig!(0x3), fbig!(0x8p-3)),
        (fbig!(0x0003), fbig!(0x8c9fp-15)),
        (fbig!(0x0000000000000003), fbig!(0x8c9f53d5681854bbp-63)),
        (
            fbig!(0x3).with_precision(200).unwrap(),
            fbig!(0x8c9f53d5681854bb520cc6aa829dbe5adf0a216cdbf046f81ep-199),
        ),
        (fbig!(0x3000), fbig!(0x96a9p-12)),
        (fbig!(0x3000000000000000), fbig!(0xaabff116fff344b6p-58)),
        (fbig!(0xf), fbig!(0xap-2)),
        (fbig!(0xffff), fbig!(0xb172p-12)),
        (fbig!(0xffffp-16), fbig!(-0x1p-16)),
        (fbig!(0xffffp-32), fbig!(-0xb172p-12)),
        (fbig!(0xffffffffffffffff), fbig!(0xb17217f7d1cf79abp-58)),
        (fbig!(0xffff000000000000p-64), fbig!(-0x800040002aaacaaap-79)),
        (fbig!(0xffff000000000000p-128), fbig!(-0xb1721bf7d3cf7b01p-58)),
        (fbig!(0xffffffffffffffffp-64), fbig!(-0x1p-64)),
        (fbig!(0xf0f0f0f0f0f0f0f0), fbig!(0xb134039651acb45fp-58)),
        (fbig!(0xf0f0f0f0f0f0f0f0p-64), fbig!(-0xf85186008b15331bp-68)),
        (fbig!(0xf0f0f0f0f0f0f0f0p-128), fbig!(-0xb1b02c5951f23ef8p-58)),
        (
            fbig!(0xf0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0p-128),
            fbig!(-0xf85186008b15330be64b8b775997899dp-132),
        ),
    ];
    for (x, ln) in &inexact_cases {
        assert_eq!(x.ln(), *ln);
        if let Inexact(v, e) = x.context().ln(x.repr()) {
            assert_eq!(v, *ln);
            assert_eq!(e, NoOp);
        } else {
            panic!("the result should be inexact!")
        }
    }
}

#[test]
fn test_ln_decimal() {
    assert_eq!(dbig!(1).ln(), dbig!(0));

    let inexact_cases = [
        (dbig!(3), dbig!(1), NoOp),
        (dbig!(0003), dbig!(1099e-3), AddOne),
        (dbig!(0000000000000003), dbig!(1098612288668110e-15), AddOne),
        (
            dbig!(3).with_precision(60).unwrap(),
            dbig!(109861228866810969139524523692252570464749055782274945173469e-59),
            NoOp,
        ),
        (dbig!(3000), dbig!(8006e-3), NoOp),
        (dbig!(3000000000000000), dbig!(3563738868357879e-14), NoOp),
        (dbig!(9), dbig!(2), NoOp),
        (dbig!(9999), dbig!(9210e-3), NoOp),
        (dbig!(9999e-4), dbig!(-1e-4), NoOp),
        (dbig!(9999e-8), dbig!(-9210e-3), NoOp),
        (dbig!(9999999999999999), dbig!(3684136148790473e-14), NoOp),
        (dbig!(9999000000000000e-16), dbig!(-1000050003333583e-19), NoOp),
        (dbig!(9999000000000000e-32), dbig!(-3684146149290506e-14), NoOp),
        (dbig!(9999999999999999e-16), dbig!(-1e-16), NoOp),
        (dbig!(9090909090909090), dbig!(3674605130810041e-14), AddOne),
        (dbig!(9090909090909090e-16), dbig!(-9531017980432496e-17), NoOp),
        (dbig!(9090909090909090e-32), dbig!(-3693667166770906e-14), SubOne),
        (
            dbig!(90909090909090909090909090909090e-32),
            dbig!(-95310179804324860043952123280775e-33),
            NoOp,
        ),
    ];
    for (x, ln, rnd) in &inexact_cases {
        assert_eq!(x.ln(), *ln);
        if let Inexact(v, e) = x.context().ln(x.repr()) {
            assert_eq!(v, *ln);
            assert_eq!(e, *rnd);
        } else {
            panic!("the result should be inexact!")
        }
    }
}

#[test]
#[should_panic]
fn test_ln_unlimited_precision() {
    let _ = dbig!(2).with_precision(0).unwrap().ln();
}

#[test]
#[should_panic]
fn test_ln_inf() {
    let _ = DBig::INFINITY.ln();
}

#[test]
#[rustfmt::skip::macros(fbig)]
fn test_ln_1p_binary() {
    assert_eq!(fbig!(0).ln_1p(), fbig!(0));

    let inexact_cases = [
        (fbig!(0x1), fbig!(0xbp-4)),
        (fbig!(0x0001), fbig!(0xb172p-16)),
        (fbig!(0x0000000000000001), fbig!(0xb17217f7d1cf79abp-64)),
        (
            fbig!(1).with_precision(200).unwrap(),
            fbig!(0xb17217f7d1cf79abc9e3b39803f2f6af40f343267298b62d8ap-200),
        ),
        (fbig!(-0xfp-4), fbig!(-0xbp-2)),
        (fbig!(-0xffffp-16), fbig!(-0xb172p-12)),
        (fbig!(-0xffffffffffffffffp-64), fbig!(-0xb17217f7d1cf79abp-58)),
        (fbig!(0x12p-8), fbig!(0x8bp-11)),
        (fbig!(0x1234p-16), fbig!(0x8caep-19)),
        (fbig!(0x123456789p-36), fbig!(0x8cb0c4597p-39)),
        (
            fbig!(0x123456789012345678901234567890123456789p-156),
            fbig!(0x8cb0c45979137478e2c15022d0ec8a3bc2bd96fp-159),
        ),
        (fbig!(-0x12p-8), fbig!(-0x95p-11)),
        (fbig!(-0x1234p-16), fbig!(-0x970fp-19)),
        (fbig!(-0x123456789p-36), fbig!(-0x9712b5164p-39)),
        (
            fbig!(-0x123456789012345678901234567890123456789p-156),
            fbig!(-0x9712b51649e8249cb09abdb00a6a0fa1977d537p-159),
        ),
    ];
    for (x, ln) in &inexact_cases {
        assert_eq!(x.ln_1p(), *ln);
        if let Inexact(v, e) = x.context().ln_1p(x.repr()) {
            assert_eq!(v, *ln);
            assert_eq!(e, NoOp);
        } else {
            panic!("the result should be inexact!")
        }
    }
}

#[test]
fn test_ln_1p_decimal() {
    assert_eq!(dbig!(0).ln_1p(), dbig!(0));

    let inexact_cases = [
        (dbig!(1), dbig!(7e-1), AddOne),
        (dbig!(0001), dbig!(6931e-4), NoOp),
        (dbig!(0000000000000001), dbig!(6931471805599453e-16), NoOp),
        (
            dbig!(1).with_precision(60).unwrap(),
            dbig!(693147180559945309417232121458176568075500134360255254120680e-60),
            NoOp,
        ),
        (dbig!(-9e-1), dbig!(-2), NoOp),
        (dbig!(-9999e-4), dbig!(-9210e-3), NoOp),
        (dbig!(-9999999999999999e-16), dbig!(-3684136148790473e-14), NoOp),
        (dbig!(98e-3), dbig!(93e-3), NoOp),
        (dbig!(9876e-5), dbig!(9418e-5), NoOp),
        (dbig!(987654321e-10), dbig!(941872151e-10), AddOne),
        (
            dbig!(987654321098765432109876543210987654321e-40),
            dbig!(941872150698134322883564967798628924618e-40),
            NoOp,
        ),
        (dbig!(-98e-3), dbig!(-10e-2), NoOp),
        (dbig!(-9876e-5), dbig!(-1040e-4), SubOne),
        (dbig!(-987654321e-10), dbig!(-103989714e-9), SubOne),
        (
            dbig!(-987654321098765432109876543210987654321e-40),
            dbig!(-103989713536376403613468637446294968790e-39),
            SubOne,
        ),
    ];
    for (x, ln, rnd) in &inexact_cases {
        assert_eq!(x.ln_1p(), *ln);
        if let Inexact(v, e) = x.context().ln_1p(x.repr()) {
            assert_eq!(v, *ln);
            assert_eq!(e, *rnd);
        } else {
            panic!("the result should be inexact!")
        }
    }
}

#[test]
#[should_panic]
fn test_ln_1p_unlimited_precision() {
    let _ = dbig!(2).with_precision(0).unwrap().ln_1p();
}

#[test]
#[should_panic]
fn test_ln_1p_inf() {
    let _ = DBig::INFINITY.ln_1p();
}

#[test]
fn test_ln_with_rounding() {
    use dashu_base::Abs;
    use dashu_float::round::{mode::*, Round};

    fn test_ln_with_error<R: Round, OpR: Round, const B: Word>(
        base: &FBig<R, B>,
        target: &FBig<R, B>,
        atol: &FBig<R, B>,
    ) {
        let result = base.clone().with_rounding::<OpR>().ln();
        let result_err = (result.with_rounding::<R>() - target).abs();
        assert!(result_err <= *atol, "ln({}), err: {} (>{})", base, result_err, atol);
    }

    let binary_cases = [
        // base, target result
        (fbig!(0x0010), fbig!(0xb172p - 14)),
        (fbig!(0x001f), fbig!(0xdbc6p - 14)),
        (fbig!(0x1234), fbig!(0x8726p - 12)),
        (fbig!(0x1abc), fbig!(0x8d4cp - 12)),
    ];

    for (base, target) in binary_cases {
        test_ln_with_error::<_, Zero, 2>(&base, &target, &target.ulp());
        test_ln_with_error::<_, Up, 2>(&base, &target, &target.ulp());
        test_ln_with_error::<_, Down, 2>(&base, &target, &target.ulp());
        test_ln_with_error::<_, HalfAway, 2>(&base, &target, &target.ulp());
        test_ln_with_error::<_, HalfEven, 2>(&base, &target, &target.ulp());
    }
}

#[test]
#[rustfmt::skip::macros(fbig)]
fn test_log2_fbig() {
    let test_cases = [
        (fbig!(0x3), 1.584962500721156),
        (fbig!(0x55p-4), 2.4093909361377017),
        (fbig!(-0x1234567890123456789p-123), -50.81378119141385),
        (fbig!(0x1) << 16, 16.0),
        (fbig!(0x1) << 32, 32.0),
        (fbig!(0x1) << 64, 64.0),
        (fbig!(0x1) << 128, 128.0),
        (fbig!(0xffffffff), 31.999999999664098),
        (fbig!(0xffffffffffffffffffffffffffffffff), 128.0),
        (fbig!(0xffffffff) << 96, 127.999999999664),
    ];

    const ERR_BOUND: f64 = 1. / 256.;
    for (n, log2) in test_cases {
        let (lb, ub) = n.log2_bounds();
        let (lb, ub) = (lb as f64, ub as f64);
        assert!(lb <= log2 && (log2 - lb) / log2 < ERR_BOUND);
        assert!(ub >= log2 && (ub - log2) / log2 < ERR_BOUND);
    }
}
