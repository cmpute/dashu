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
        if let Ok(Inexact(v, e)) = x.context().ln(x.repr(), None) {
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
        if let Ok(Inexact(v, e)) = x.context().ln(x.repr(), None) {
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
        if let Ok(Inexact(v, e)) = x.context().ln_1p(x.repr(), None) {
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
        if let Ok(Inexact(v, e)) = x.context().ln_1p(x.repr(), None) {
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
    use dashu_float::round::{mode::*, ErrorBounds, Round};

    fn test_ln_with_error<R: Round, OpR: ErrorBounds, const B: Word>(
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

// The exponent used to be converted to f32 before it was multiplied by log2(base). Above 2^24
// that conversion is itself off by whole units, so the "upper bound" could land below the
// true logarithm (971397245e16777285 got 55732964 for a log2 of 55732964.25).
#[test]
fn test_log2_bounds_large_exponent() {
    use dashu_base::BitTest;
    use dashu_int::{IBig, UBig};

    // the reported case: floor(log2(971397245 * 10^16777285)) = 55732964 (the fraction is
    // about 0.2529), and its neighbor, whose floor is 55732960 (fraction about 0.9310)
    for (exp, floor_log2) in [(16_777_285isize, 55732964f64), (16_777_284, 55732960f64)] {
        for sig in [971397245, -971397245] {
            let (lb, ub) = DBig::from_parts(IBig::from(sig), exp).log2_bounds();
            assert!((lb as f64) < floor_log2 + 1.0, "exponent {exp}: lower {lb}");
            assert!((ub as f64) > floor_log2, "exponent {exp}: upper {ub}");
            assert!(ub - lb <= 64.0);
        }
    }
    // a moderately large exponent, against the exact bit length of the integer
    for exp in [99_999usize, 100_000, 1_000_000] {
        let floor_log2 = (UBig::from(971397245u32) * UBig::from(10u8).pow(exp)).bit_len() - 1;
        let (lb, ub) = DBig::from_parts(IBig::from(971397245), exp as isize).log2_bounds();
        assert!((lb as f64) < (floor_log2 + 1) as f64, "exponent {exp}: lower {lb}");
        assert!((ub as f64) > floor_log2 as f64, "exponent {exp}: upper {ub}");
    }

    // any exponent: check against log2(sig) + exp * log2(base) evaluated in f64, whose error
    // (about 1e-15 relative) is far below the resolution of the f32 bounds
    fn check<const B: dashu_int::Word>(sig: i64, exp: isize) {
        let x = dashu_float::FBig::<dashu_float::round::mode::Zero, B>::from_parts(sig.into(), exp);
        let (lb, ub) = x.log2_bounds();
        let log2 = (sig.unsigned_abs() as f64).log2() + exp as f64 * (B as f64).log2();
        let slack = log2.abs() * 1e-14;
        assert!(lb as f64 <= log2 + slack, "{sig} * {B}^{exp}: lower {lb} > {log2}");
        assert!(ub as f64 >= log2 - slack, "{sig} * {B}^{exp}: upper {ub} < {log2}");
        // tight: within a few f32 ulps of the true value
        assert!((ub as f64 - lb as f64) <= 1e-2 + log2.abs() * 1e-6, "{sig} * {B}^{exp}");
    }
    for exp in [
        0isize,
        1,
        1000,
        16_777_217,
        16_777_285,
        (1 << 25) + 1,
        (1 << 31) + 12345,
        (1 << 40) + 1,
        (1 << 53) + 1,
        (1 << 60) + 12345,
        isize::MAX - 1,
    ] {
        for sign in [1, -1] {
            for sig in [1, 3, 971397245, i64::MAX] {
                check::<10>(sig, sign * exp);
                check::<2>(sig, sign * exp);
                check::<3>(sig, sign * exp);
                check::<16>(sig, sign * exp);
                check::<36>(sig, sign * exp);
            }
        }
    }
    assert_eq!(DBig::ZERO.log2_bounds(), (f32::NEG_INFINITY, f32::NEG_INFINITY));
}

// Correct rounding in an odd base under the round-to-nearest modes. The certification of a
// rounded result relied on a "half ulp" that was rounded up to 2/3 of an ulp in base 3, so a
// result up to 1/6 of an ulp on the wrong side of a midpoint was accepted: ln(1296) =
// 7.16703787... lies above the midpoint 43/6 of 7 and 22/3, yet came out as 7.
#[test]
fn test_ln_odd_base_nearest() {
    use dashu_float::round::mode::{HalfAway, HalfEven};
    use dashu_int::IBig;

    let x = FBig::<HalfAway, 3>::from_parts(IBig::from(1296), 0)
        .with_precision(3)
        .value();
    assert_eq!(x.ln(), FBig::<HalfAway, 3>::from_parts(IBig::from(22), -1));
    let x = FBig::<HalfEven, 3>::from_parts(IBig::from(1296), 0)
        .with_precision(3)
        .value();
    assert_eq!(x.ln(), FBig::<HalfEven, 3>::from_parts(IBig::from(22), -1));

    // against the same logarithm computed with many more digits and rounded once
    fn sweep<R: dashu_float::round::ErrorBounds, const B: Word>() {
        for precision in [1usize, 2, 3, 5, 13, 32, 63] {
            for n in (2..400).step_by(7).chain([1296, 59049, 1_000_003]) {
                let x = FBig::<R, B>::from_parts(IBig::from(n), 0)
                    .with_precision(0)
                    .value();
                let got = x.clone().with_precision(precision + 20).value();
                let got = dashu_float::Context::<R>::new(precision)
                    .ln(got.repr(), None)
                    .unwrap()
                    .value();
                let reference = dashu_float::Context::<R>::new(precision + 40)
                    .ln(x.repr(), None)
                    .unwrap()
                    .value()
                    .with_precision(precision)
                    .value();
                assert_eq!(got, reference, "ln({n}) in base {B} at precision {precision}");
            }
        }
    }
    sweep::<HalfAway, 3>();
    sweep::<HalfEven, 3>();
    sweep::<HalfAway, 5>();
    sweep::<HalfEven, 7>();
}

/// The `Exact`/`Inexact` flag of a logarithm describes the *true* value, not the working
/// approximation: an irrational logarithm is never `Exact`, and the rounding direction matches
/// the side of the result the true value is on (checked against a much higher precision).
///
/// Regression: `ln(55)` and `log10(31)` at 2 bits came back `Exact`, and `ln(2764)` at 24 bits
/// was rounded up but flagged `NoOp`.
#[test]
fn test_log_rounding_flag() {
    use dashu_float::{
        round::{mode::HalfEven, Rounded},
        Context, Repr,
    };
    type F = FBig<HalfEven, 2>;

    fn check(name: &str, n: u32, p: usize, got: Rounded<F>, truth: F) {
        let value = got.value_ref().clone().with_precision(0).value();
        let truth = truth.with_precision(0).value();
        match got {
            Exact(_) => panic!("{name}({n}) at {p} bits is flagged Exact"),
            Inexact(_, NoOp) => assert!(value < truth, "{name}({n}) at {p} bits: NoOp"),
            Inexact(_, AddOne) => assert!(value > truth, "{name}({n}) at {p} bits: AddOne"),
            Inexact(_, SubOne) => panic!("{name}({n}) at {p} bits: SubOne for a positive value"),
        }
    }

    // the reported cases
    let ctx = Context::<HalfEven>::new(2);
    let r = ctx.ln(&Repr::<2>::new(55.into(), 0), None).unwrap();
    assert_eq!(r, Inexact(F::from(4u8), NoOp));
    let r = ctx.log10(&Repr::<2>::new(31.into(), 0), None).unwrap();
    assert_eq!(r, Inexact(F::from_parts(3.into(), -1), AddOne));
    let r = Context::<HalfEven>::new(24).ln(&Repr::<2>::new(2764.into(), 0), None);
    assert!(matches!(r, Ok(Inexact(_, AddOne))));

    for p in [2usize, 20, 24, 50, 100, 500] {
        let ctx = Context::<HalfEven>::new(p);
        let hi = Context::<HalfEven>::new(p + 200);
        let ns: &[u32] = if p == 500 {
            &[3, 55, 2764]
        } else {
            &[3, 5, 6, 7, 30, 31, 54, 55, 99, 1000, 2764, 65535, 65537]
        };
        for &n in ns {
            let x = Repr::<2>::new(n.into(), 0);
            let (got, truth) = (ctx.ln(&x, None).unwrap(), hi.ln(&x, None).unwrap());
            check("ln", n, p, got, truth.value());
            let (got, truth) = (ctx.log2(&x, None).unwrap(), hi.log2(&x, None).unwrap());
            check("log2", n, p, got, truth.value());
            if n != 1000 {
                let (got, truth) = (ctx.log10(&x, None).unwrap(), hi.log10(&x, None).unwrap());
                check("log10", n, p, got, truth.value());
            }
        }
    }
}

/// The exact special cases (`ln(1)`, `log2(1)`, `log10(1)`, `ln_1p(±0)`) carry the context like
/// every other result. They used to return the precision-0 `FBig::ZERO`, so a following
/// limited-precision operation (`one.ln().sin()`) panicked with "precision cannot be 0".
#[test]
fn test_exact_shortcuts_keep_precision() {
    use dashu_float::round::mode::HalfEven;

    fn check<const B: Word>(p: usize) {
        type F<const B: Word> = FBig<HalfEven, B>;
        let zero = F::<B>::ZERO.with_precision(p).value();
        let one = F::<B>::ONE.with_precision(p).value();
        for (i, got) in [one.ln(), one.log2(), one.log10(), zero.ln_1p()]
            .iter()
            .enumerate()
        {
            assert!(got.repr().is_pos_zero(), "case {i} at {p} digits in base {B}");
            assert_eq!(got.precision(), p, "case {i} at {p} digits in base {B}");
        }
        let neg = (-zero.clone()).ln_1p();
        assert!(neg.repr().is_neg_zero());
        assert_eq!(neg.precision(), p);

        // chained operations must not panic
        assert_eq!(one.ln().exp(), one);
        assert_eq!(one.ln().sin(), zero);
        assert_eq!(one.log2().exp_m1(), zero);
    }
    for p in [1, 20, 50, 100, 500] {
        check::<2>(p);
        check::<10>(p);
    }
    // an unlimited-precision one still has its exact logarithm
    assert_eq!(DBig::ONE.ln(), DBig::ZERO);
}

/// `log2` of an exact power of two and `log10` of an exact power of ten are the exact integer in
/// every base and every rounding mode. Regression: in base 10 a negative power of two
/// (`0.5 = 5·10⁻¹`, `0.25`, `0.125`) was not recognized, and the certification loop cannot
/// terminate on an exactly representable result under `Down`/`Up`/`Zero`.
#[test]
fn test_log_exact_integer_results() {
    use dashu_float::{
        round::{mode, ErrorBounds},
        Context, Repr,
    };
    use dashu_int::IBig;

    fn check<R: ErrorBounds, const B: Word>(p: usize) {
        let ctx = Context::<R>::new(p);
        let exact = Context::<R>::new(0);
        let int = |k: i32| Exact(FBig::<R, B>::from(k).with_precision(p).value());
        let (two, ten) = (Repr::<B>::new(2.into(), 0), Repr::<B>::new(10.into(), 0));
        for k in 1..=12 {
            // 2^k and 10^k are integers in every base
            let pow2 = exact.powi(&two, k.into()).unwrap().value();
            assert_eq!(ctx.log2(pow2.repr(), None), Ok(int(k)), "log2(2^{k}) in base {B}");
            let pow10 = exact.powi(&ten, k.into()).unwrap().value();
            assert_eq!(ctx.log10(pow10.repr(), None), Ok(int(k)), "log10(10^{k}) in base {B}");
            // negative powers are representable when the base has the prime factors
            if B % 2 == 0 {
                let x = Context::<R>::new(200).powi(&two, (-k).into()).unwrap();
                assert!(matches!(x, Exact(_)));
                let got = ctx.log2(x.value().repr(), None);
                assert_eq!(got, Ok(int(-k)), "log2(2^-{k}) in base {B}");
            }
            if B == 10 {
                let x = Repr::<B>::new(IBig::ONE, -(k as isize));
                assert_eq!(ctx.log10(&x, None), Ok(int(-k)), "log10(10^-{k}) in base {B}");
            }
        }
    }
    fn check_all_modes<const B: Word>(p: usize) {
        check::<mode::Zero, B>(p);
        check::<mode::Away, B>(p);
        check::<mode::Down, B>(p);
        check::<mode::Up, B>(p);
        check::<mode::HalfEven, B>(p);
        check::<mode::HalfAway, B>(p);
    }
    for p in [5, 20, 50, 100, 500] {
        check_all_modes::<2>(p);
        check_all_modes::<10>(p);
    }
    check_all_modes::<3>(8);
    check_all_modes::<16>(8);
    check_all_modes::<30>(8);

    // the reported calls
    let half = Repr::<10>::new(5.into(), -1);
    let got = Context::<mode::Down>::new(5).log2(&half, None);
    assert_eq!(got, Ok(Exact(FBig::from(-1))));
    let quarter = Repr::<10>::new(25.into(), -2);
    let got = Context::<mode::Up>::new(5).log2(&quarter, None);
    assert_eq!(got, Ok(Exact(FBig::from(-2))));
    let eighth = Repr::<10>::new(125.into(), -3);
    let got = Context::<mode::Zero>::new(5).log2(&eighth, None);
    assert_eq!(got, Ok(Exact(FBig::from(-3))));
    // near misses stay inexact
    let x = Repr::<10>::new(126.into(), -3);
    assert!(matches!(Context::<mode::Zero>::new(5).log2(&x, None), Ok(Inexact(..))));
    let x = Repr::<10>::new(5.into(), -2);
    assert!(matches!(Context::<mode::Down>::new(5).log2(&x, None), Ok(Inexact(..))));
    // base 30: 0.1 = 3·30⁻¹
    let tenth = Repr::<30>::new(3.into(), -1);
    let got = Context::<mode::Down>::new(5).log10(&tenth, None);
    assert_eq!(got, Ok(Exact(FBig::from(-1))));
}

/// The logarithms accept the whole exponent range. Regression: `ln(1·2^isize::MIN)` — the value
/// a directed-mode underflow of `exp` saturates to — panicked ("attempt to negate with overflow"
/// in debug builds, "out of memory" in release builds).
#[test]
fn test_ln_extreme_exponents() {
    use dashu_float::{round::mode, Context, Repr};
    use dashu_int::IBig;

    fn check<const B: Word>(p: usize, e: isize) {
        let ctx = Context::<mode::HalfEven>::new(p);
        // ln(B^e) = e·ln(B): compare with the product computed at a higher precision
        let x = Repr::<B>::new(IBig::ONE, e);
        let got = ctx.ln(&x, None).unwrap().value();
        let hi = Context::<mode::HalfEven>::new(p + 40);
        let ln_base = hi.ln(&Repr::<B>::new(B.into(), 0), None).unwrap().value();
        let want = hi
            .mul(ln_base.repr(), &Repr::new(e.into(), 0))
            .unwrap()
            .value();
        assert_eq!(got, want.with_precision(p).value(), "ln({B}^{e}) at {p} digits");
    }
    for e in [
        isize::MIN,
        isize::MIN + 1,
        isize::MIN / 2,
        isize::MAX - 1,
        isize::MAX,
    ] {
        for p in [20, 50, 100, 500] {
            check::<2>(p, e);
        }
        check::<10>(2, e);
        check::<10>(30, e);
        check::<3>(30, e);
    }

    // exp(-2^63) under Up saturates to the smallest positive value; its logarithm is finite
    let x = FBig::<mode::Up, 2>::from_parts(IBig::from(-1), 63)
        .with_precision(53)
        .value()
        .exp();
    assert_eq!(x.repr().exponent(), isize::MIN);
    let f = x.ln().to_f64().value();
    let expected = isize::MIN as f64 * core::f64::consts::LN_2;
    assert!(((f - expected) / expected).abs() < 1e-12);
    // log2 of the same value is the exact integer
    assert_eq!(x.log2().to_f64().value(), isize::MIN as f64);
}
