use core::str::FromStr;

use dashu_base::Approximation::Inexact;
use dashu_base::Sign;
use dashu_float::ops::Abs;
use dashu_float::round::mode::{Away, Down, HalfAway, HalfEven, Up, Zero};
use dashu_float::round::Rounding::{AddOne, NoOp, SubOne};
use dashu_float::{round, Context, DBig, FBig, FpError, Repr, Word};
use dashu_int::IBig;

#[test]
fn test_pi() {
    let pi = DBig::pi(10);
    assert_eq!(pi.to_string(), "3.141592654");

    let pi20 = DBig::pi(20);
    assert_eq!(pi20.to_string(), "3.1415926535897932385");

    let pi100 = DBig::pi(100);
    assert_eq!(pi100.to_string(), "3.141592653589793238462643383279502884197169399375105820974944592307816406286208998628034825342117068");

    let pi500 = DBig::pi(500);
    assert_eq!(pi500.to_string(), "3.1415926535897932384626433832795028841971693993751058209749445923078164062862089986280348253421170679821480865132823066470938446095505822317253594081284811174502841027019385211055596446229489549303819644288109756659334461284756482337867831652712019091456485669234603486104543266482133936072602491412737245870066063155881748815209209628292540917153643678925903600113305305488204665213841469519415116094330572703657595919530921861173819326117931051185480744623799627495673518857527248912279381830119491");

    let pi_bin = FBig::<round::mode::HalfAway, 2>::pi(100);
    assert_eq!(pi_bin.to_string(), "11.00100100001111110110101010001000100001011010001100001000110100110001001100011001100010100010111");
}

#[test]
fn test_sin_cos() {
    let x = DBig::ZERO.with_precision(30).value();
    let (s, c) = x.sin_cos();
    assert_eq!(s, DBig::ZERO);
    assert_eq!(c, DBig::ONE.with_precision(30).value());

    let pi = DBig::pi(30);
    let (s, c) = pi.sin_cos();
    assert!(s.abs() < DBig::from_parts(1.into(), -29));
    let neg_one = -DBig::ONE.with_precision(30).value();
    assert!((c - neg_one).abs() < DBig::from_parts(1.into(), -29));
}

#[test]
fn test_tan() {
    let x = DBig::ZERO.with_precision(30).value();
    assert_eq!(x.tan(), DBig::ZERO);

    let pi = DBig::pi(30);
    let pi4: DBig = pi / 4;
    let tan_pi4 = pi4.tan();
    assert!((tan_pi4 - DBig::ONE).abs() < DBig::from_parts(1.into(), -29));
}

#[test]
fn test_atan() {
    let x = DBig::ZERO.with_precision(30).value();
    assert_eq!(x.atan(), DBig::ZERO);

    let one = DBig::ONE.with_precision(30).value();
    let pi = DBig::pi(30);
    let pi4: DBig = pi / 4;
    let atan_one = one.atan();
    assert!((atan_one - pi4).abs() < DBig::from_parts(1.into(), -29));
}

#[test]
fn test_asin_acos() {
    let x = DBig::ZERO.with_precision(30).value();
    assert_eq!(x.asin(), DBig::ZERO);

    let pi = DBig::pi(30);
    let half_pi: DBig = &pi / 2;
    assert!((x.acos() - half_pi).abs() < DBig::from_parts(1.into(), -29));

    let half = DBig::from_parts(5.into(), -1).with_precision(30).value();
    let asin_half = half.asin();
    // asin(0.5) = pi/6
    let pi6: DBig = &pi / 6;
    assert!((asin_half - pi6).abs() < DBig::from_parts(1.into(), -29));
}

#[test]
#[should_panic]
fn test_asin_out_of_domain_panics() {
    // asin(|x| > 1) is out of domain; the FBig convenience layer panics.
    let two = DBig::from_parts(2.into(), 0).with_precision(10).value();
    let _ = two.asin();
}

#[test]
fn test_atan2() {
    let zero = DBig::ZERO.with_precision(30).value();
    let one = DBig::ONE.with_precision(30).value();
    let neg_one = -one.clone();
    let pi = DBig::pi(30);

    // atan2(0, 1) = 0
    assert_eq!(zero.atan2(&one), zero);

    // atan2(1, 0) = pi/2
    let half_pi: DBig = &pi / 2;
    assert!((one.atan2(&zero) - half_pi.clone()).abs() < DBig::from_parts(1.into(), -29));

    // atan2(0, -1) = pi
    assert!((zero.atan2(&neg_one) - &pi).abs() < DBig::from_parts(1.into(), -29));

    // atan2(-1, 0) = -pi/2
    let m_half_pi: DBig = -half_pi;
    assert!((neg_one.atan2(&zero) - m_half_pi).abs() < DBig::from_parts(1.into(), -29));
}

/// `atan2` / `atan2_unit` take two operands and return the result at the higher of their
/// precisions, like `+`, `/`, `powf` and `hypot`. They used to keep the receiver's: a `1` of
/// precision 1 against an `x` of precision 40 gave `0.3`.
#[test]
fn test_atan2_takes_the_higher_precision() {
    let y = DBig::from(1);
    let x = DBig::from_str("3.000000000000000000000000000000000000001").unwrap();
    assert_eq!((y.precision(), x.precision()), (1, 40));
    let r = y.atan2(&x);
    assert_eq!(r.precision(), 40);
    assert_eq!(r.to_string(), "0.3217505543966421934014046143586613190207");
    assert_eq!(r, y.clone().with_precision(40).value().atan2(&x));
    // whichever operand is the wide one
    assert_eq!(x.atan2(&y).precision(), 40);
    let u = y.atan2_unit(&x, 360);
    assert_eq!(u.precision(), 40);
    assert_eq!(u, y.clone().with_precision(40).value().atan2_unit(&x, 360));
    assert_eq!(x.atan2_unit(&y, 360).precision(), 40);

    // the same value as the context-level call at the wider precision
    type F = FBig<HalfEven, 2>;
    for (py, px) in [
        (20usize, 50usize),
        (50, 20),
        (100, 500),
        (500, 100),
        (50, 50),
    ] {
        let y = F::from(3).with_precision(py).value();
        let x = F::from(-7).with_precision(px).value();
        let ctx = Context::<HalfEven>::new(py.max(px));
        let r = y.atan2(&x);
        assert_eq!(r.precision(), py.max(px));
        assert_eq!(r, ctx.atan2(y.repr(), x.repr(), None).unwrap().value());
        let r = y.atan2_unit(&x, 360);
        assert_eq!(r.precision(), py.max(px));
        assert_eq!(
            r,
            ctx.atan2_unit(y.repr(), x.repr(), 360, None)
                .unwrap()
                .value()
        );

        // the cached wrapper is a drop-in replacement
        let cache = std::rc::Rc::new(core::cell::RefCell::new(dashu_float::ConstCache::new()));
        let cy = dashu_float::CachedFBig::new(y.clone(), cache.clone());
        let cx = dashu_float::CachedFBig::new(x.clone(), cache);
        assert_eq!(cy.atan2(&cx).as_fbig(), &y.atan2(&x));
        assert_eq!(cy.atan2(&cx).as_fbig().precision(), py.max(px));
        assert_eq!(cy.atan2_unit(&cx, 360).as_fbig(), &y.atan2_unit(&x, 360));
        assert_eq!(cy.atan2_unit(&cx, 360).as_fbig().precision(), py.max(px));
    }
}

#[test]
#[should_panic]
fn test_atan2_zero_zero_panics() {
    // atan2(0, 0) is indeterminate; the FBig convenience layer panics.
    let z0 = DBig::ZERO.with_precision(10).value();
    let _ = z0.atan2(&z0);
}

#[test]
fn test_atan2_infinities() {
    let x = DBig::ZERO.with_precision(30).value();
    let ctx = x.context();
    let inf = Repr::infinity();
    let neg_inf = Repr::neg_infinity();
    let pi = ctx.pi::<10>(None).value();
    let pi_4 = &pi / 4;
    let pi_3_4 = &pi * 3 / 4;

    // atan2(+inf, +inf) = pi/4
    let res: DBig = ctx.atan2(&inf, &inf, None).unwrap().value();
    let diff: DBig = res - &pi_4;
    assert!(diff.abs() < DBig::from_parts(1.into(), -29));

    // atan2(+inf, -inf) = 3pi/4
    let res: DBig = ctx.atan2(&inf, &neg_inf, None).unwrap().value();
    let diff: DBig = res - &pi_3_4;
    assert!(diff.abs() < DBig::from_parts(1.into(), -29));

    // atan2(-inf, +inf) = -pi/4
    let res: DBig = ctx.atan2(&neg_inf, &inf, None).unwrap().value();
    let diff: DBig = res + &pi_4;
    assert!(diff.abs() < DBig::from_parts(1.into(), -29));

    // atan2(-inf, -inf) = -3pi/4
    let res: DBig = ctx.atan2(&neg_inf, &neg_inf, None).unwrap().value();
    let diff: DBig = res + &pi_3_4;
    assert!(diff.abs() < DBig::from_parts(1.into(), -29));
}

/// The fixed π-multiples `atan` / `atan2` return at infinite arguments and on the y axis are
/// rounded once, from a wider π — not derived from an already rounded π (which rounds twice:
/// `atan2(+inf, +inf)` at 10 digits used to be 0.7853981635, `atan(-inf)` under `Up` used to
/// round away from +∞).
#[test]
fn test_atan_atan2_pi_multiples_correctly_rounded() {
    // the reported cases
    let inf = DBig::from_repr(Repr::infinity(), Context::new(10));
    assert_eq!(inf.atan2(&inf).to_string(), "0.7853981634");
    let one = DBig::from_repr(Repr::one(), Context::new(10));
    assert_eq!(inf.atan2(&inf), one.atan2(&one));
    let ninf = FBig::<Up, 2>::from_repr(Repr::neg_infinity(), Context::new(4));
    assert_eq!(ninf.atan(), FBig::<Up, 2>::try_from(-1.5f64).unwrap());
    let pinf = FBig::<Up, 2>::from_repr(Repr::infinity(), Context::new(4));
    assert_eq!(pinf.atan(), FBig::<Up, 2>::try_from(1.625f64).unwrap());
    let inf2 = FBig::<HalfEven, 2>::from_repr(Repr::infinity(), Context::new(1));
    let ninf2 = FBig::<HalfEven, 2>::from_repr(Repr::neg_infinity(), Context::new(1));
    assert_eq!(inf2.atan2(&ninf2), FBig::<HalfEven, 2>::from(2u8));

    // Every special row against k·π/2^s formed exactly from a π that is 64 bits wider, then
    // rounded once under the mode (π is irrational: 64 spare bits settle these fixed cases).
    fn check<R: round::ErrorBounds>(p: usize) {
        let ctx = Context::<R>::new(p);
        let pi = Context::<HalfEven>::new(p + 64).pi::<2>(None).value();
        let want = |num: i32, shift: isize| {
            let exact = Repr::<2>::new(
                pi.repr().significand() * IBig::from(num),
                pi.repr().exponent() - shift,
            );
            FBig::<R, 2>::from_repr(exact, Context::new(0)).with_precision(p)
        };
        let (inf, ninf) = (Repr::<2>::infinity(), Repr::<2>::neg_infinity());
        let (one, mone) = (Repr::<2>::one(), Repr::<2>::neg_one());
        let zero = Repr::<2>::zero();

        assert_eq!(ctx.atan(&inf, None), Ok(want(1, 1)), "atan(+inf), p = {p}");
        assert_eq!(ctx.atan(&ninf, None), Ok(want(-1, 1)), "atan(-inf), p = {p}");
        for (y, x, num, shift) in [
            (&inf, &inf, 1, 2),
            (&inf, &ninf, 3, 2),
            (&ninf, &inf, -1, 2),
            (&ninf, &ninf, -3, 2),
            (&inf, &one, 1, 1),
            (&ninf, &mone, -1, 1),
            (&one, &ninf, 1, 0),
            (&mone, &ninf, -1, 0),
            (&one, &zero, 1, 1),
            (&mone, &zero, -1, 1),
        ] {
            assert_eq!(ctx.atan2(y, x, None), Ok(want(num, shift)), "atan2({y:?}, {x:?}), p = {p}");
        }
        // atan2(±finite, +inf) = ±0, exactly
        assert!(ctx
            .atan2(&one, &inf, None)
            .unwrap()
            .value()
            .repr()
            .is_pos_zero());
        assert!(ctx
            .atan2(&mone, &inf, None)
            .unwrap()
            .value()
            .repr()
            .is_neg_zero());
    }
    for p in [1usize, 2, 4, 20, 50, 100, 500] {
        check::<Zero>(p);
        check::<Away>(p);
        check::<Up>(p);
        check::<Down>(p);
        check::<HalfEven>(p);
        check::<HalfAway>(p);
    }
}

/// Exact results at zero do not need a precision to round to: a zero carrying unlimited
/// precision (`FBig::ZERO`, `try_from(0.0)`) used to panic with "precision cannot be 0".
#[test]
fn test_exact_results_at_unlimited_precision_zero() {
    type F = FBig<HalfEven, 2>;
    for zero in [F::ZERO, F::try_from(0.0f64).unwrap()] {
        assert_eq!(zero.precision(), 0);
        assert_eq!(zero.sin(), F::ZERO);
        assert_eq!(zero.cos(), F::ONE);
        assert_eq!(zero.sin_cos(), (F::ZERO, F::ONE));
        assert_eq!(zero.tan(), F::ZERO);
        assert_eq!(zero.asin(), F::ZERO);
        assert_eq!(zero.atan(), F::ZERO);
        assert_eq!(zero.sin_pi(), F::ZERO);
        assert_eq!(zero.cos_pi(), F::ONE);
        assert_eq!(zero.sin_cos_pi(), (F::ZERO, F::ONE));
        assert_eq!(zero.tan_pi(), F::ZERO);
        assert_eq!(zero.sin_unit(360), F::ZERO);
        assert_eq!(zero.cos_unit(360), F::ONE);
        assert_eq!(zero.tan_unit(360), F::ZERO);
        assert_eq!(zero.asin_unit(360), F::ZERO);
        assert_eq!(zero.atan_unit(360), F::ZERO);
    }
    // the sign of zero survives through the odd functions
    let neg_zero = F::try_from(-0.0f64).unwrap();
    assert!(neg_zero.sin().repr().is_neg_zero());
    assert!(neg_zero.tan().repr().is_neg_zero());
    assert!(neg_zero.asin().repr().is_neg_zero());
    assert!(neg_zero.atan().repr().is_neg_zero());
    assert!(neg_zero.sin_pi().repr().is_neg_zero());
    assert!(neg_zero.tan_unit(8).repr().is_neg_zero());
    // acos(1) = 0 is the one exact row of acos
    assert_eq!(F::ONE.acos(), F::ZERO);
}

/// The exact rows of the ×u family need no precision to round to either: `0`, `±1`, and the
/// table ratios (`u/4`, `u/12`, `±1/2`, …) wherever they have a finite expansion. On the
/// unlimited-precision constants `acos_unit` used to panic with "precision cannot be 0" where
/// `acos` returned its exact `0`.
#[test]
fn test_unit_exact_rows_at_unlimited_precision() {
    type F = FBig<HalfEven, 2>;
    let f = |sig: i32, exp: isize| F::from_parts(sig.into(), exp).with_precision(0).value();
    let half = f(1, -1);
    assert_eq!(F::ONE.precision(), 0);
    assert_eq!(half.precision(), 0);

    // inverse functions
    assert_eq!(F::ONE.acos_unit(360), F::ZERO);
    assert_eq!(F::ZERO.acos_unit(360), F::from(90));
    assert_eq!(F::NEG_ONE.acos_unit(360), F::from(180));
    assert_eq!(half.acos_unit(360), F::from(60));
    assert_eq!((-half.clone()).acos_unit(360), F::from(120));
    assert_eq!(F::ONE.asin_unit(360), F::from(90));
    assert_eq!(F::NEG_ONE.asin_unit(360), F::from(-90));
    assert_eq!(half.asin_unit(360), F::from(30));
    assert_eq!(F::ONE.atan_unit(360), F::from(45));
    assert_eq!(F::NEG_ONE.atan_unit(360), F::from(-45));
    assert_eq!(F::INFINITY.atan_unit(360), F::from(90));
    assert_eq!(F::ONE.atan2_unit(&F::ONE, 360), F::from(45));
    assert_eq!(F::ONE.atan2_unit(&F::NEG_ONE, 360), F::from(135));
    assert_eq!(F::ONE.atan2_unit(&F::ZERO, 360), F::from(90));
    assert_eq!(F::ZERO.atan2_unit(&F::NEG_ONE, 360), F::from(180));
    assert_eq!(F::ZERO.atan2_unit(&F::ONE, 360), F::ZERO);
    // a ratio that is a binary fraction: u/4 = 1/4, u/8 = 5/8
    assert_eq!(F::ZERO.acos_unit(1), f(1, -2));
    assert_eq!(F::ONE.atan_unit(5), f(5, -3));

    // forward functions
    assert_eq!(f(90, 0).sin_unit(360), F::ONE);
    assert_eq!(f(270, 0).sin_unit(360), F::NEG_ONE);
    assert_eq!(f(180, 0).sin_unit(360), F::ZERO);
    assert_eq!(f(30, 0).sin_unit(360), half);
    assert_eq!(f(180, 0).cos_unit(360), F::NEG_ONE);
    assert_eq!(f(90, 0).cos_unit(360), F::ZERO);
    assert_eq!(f(120, 0).cos_unit(360), -half.clone());
    assert_eq!(f(90, 0).sin_cos_unit(360), (F::ONE, F::ZERO));
    assert_eq!(f(45, 0).tan_unit(360), F::ONE);
    assert_eq!(f(135, 0).tan_unit(360), F::NEG_ONE);
    assert_eq!(half.sin_pi(), F::ONE);
    assert_eq!(F::ONE.cos_pi(), F::NEG_ONE);
    assert_eq!(f(1, -2).tan_pi(), F::ONE);
    assert_eq!(f(1, -2).tan_pi().precision(), 0);

    // base 10 has the same rows
    assert_eq!(DBig::ONE.acos_unit(360), DBig::ZERO);
    assert_eq!(DBig::ZERO.acos_unit(360), DBig::from(90));
    assert_eq!(DBig::ONE.asin_unit(2), DBig::from_str("0.5").unwrap());
}

/// A row whose ratio has no finite expansion is not exact, and an ordinary argument is not a
/// row at all: both still need a precision.
#[test]
fn test_unit_inexact_at_unlimited_precision_panics() {
    fn panics(f: impl FnOnce() + std::panic::UnwindSafe) -> bool {
        match std::panic::catch_unwind(f) {
            Ok(()) => false,
            Err(e) => e
                .downcast_ref::<&str>()
                .map(|s| s.to_string())
                .or_else(|| e.downcast_ref::<String>().cloned())
                .is_some_and(|s| s.contains("precision cannot be 0")),
        }
    }
    type F = FBig<HalfEven, 2>;
    type F3 = FBig<HalfEven, 3>;
    // 1/4 and 1/2 in base 3, 1/3 in base 2
    assert!(panics(|| drop(F3::ZERO.acos_unit(1))));
    assert!(panics(|| drop(F3::ONE.asin_unit(2))));
    let half = F::from_parts(1.into(), -1).with_precision(0).value();
    assert!(panics(|| drop(half.asin_unit(4))));
    // sin(1°), acos_unit(1/4)
    assert!(panics(|| drop(F::ONE.sin_unit(360))));
    assert!(panics(|| drop(F::ONE.tan_unit(360))));
    let quarter = F::from_parts(1.into(), -2).with_precision(0).value();
    assert!(panics(|| drop(quarter.acos_unit(360))));
    assert!(panics(|| drop(
        F::from_parts(3.into(), 0)
            .with_precision(0)
            .value()
            .atan_unit(360)
    )));
}

/// An argument within ~1e-24 of a multiple of π that carries more digits than the context:
/// the reduced argument cancels onto a zero midpoint at the first working precision, and its
/// radius used to be dropped — `sin` returned an exact `0` (π to 77 bits at 26 bits, 10π to 79
/// digits at 28 digits).
#[test]
fn test_sin_next_to_multiple_of_pi_is_not_zero() {
    // x = π rounded to 77 bits; sin(x) = sin(π − x) = (π − x)·(1 − O(1e-47))
    let m: IBig = "118686014801135580649625".parse().unwrap();
    let x = Repr::<2>::new(m, -75);
    let up = Context::<Up>::new(26).sin(&x, None).unwrap();
    assert_eq!(up, Inexact(FBig::from_parts(IBig::from(-27096191), -102), NoOp));

    fn check<R: round::ErrorBounds>(x: &Repr<2>, p: usize) {
        // oracle: π − x from a 400-bit π, exact subtraction, one rounding under the mode
        let pi = Context::<HalfEven>::new(400).pi::<2>(None).value();
        let exact = pi.with_precision(0).value() - FBig::from_repr(x.clone(), Context::new(0));
        let want = FBig::<R, 2>::from_repr(exact.into_repr(), Context::new(0)).with_precision(p);
        assert_eq!(Context::<R>::new(p).sin(x, None), Ok(want), "sin, p = {p}");
        // the other entry points share the reduction
        let (s, c) = Context::<R>::new(p).sin_cos(x, None);
        assert_eq!(s.unwrap().value(), Context::<R>::new(p).sin(x, None).unwrap().value());
        assert_eq!(c.unwrap().value(), Context::<R>::new(p).cos(x, None).unwrap().value());
        let t = Context::<R>::new(p).tan(x, None).unwrap().value();
        assert!(!t.repr().significand().is_zero(), "tan, p = {p}");
    }
    for p in [20usize, 26, 50, 53, 100] {
        check::<Up>(&x, p);
        check::<Down>(&x, p);
        check::<HalfEven>(&x, p);
        check::<Zero>(&x, p);
    }

    // base 10: 10π rounded to 79 digits
    let xd = DBig::from_str(
        "31.41592653589793238462643383279502884197169399375105820974944592307816406286209",
    )
    .unwrap();
    let r28 = Context::<Up>::new(28).sin(xd.repr(), None).unwrap().value();
    let r40 = Context::<Up>::new(40).sin(xd.repr(), None).unwrap().value();
    assert!(!r28.repr().significand().is_zero());
    // the 28-digit result is the 40-digit one rounded up once more
    assert_eq!(r28, r40.with_precision(28).value());
}

/// `acos` / `asin` of `1 − m·2^-k` held exactly, under a context much narrower than the
/// argument: at the first working precision `1 − x²` is all rounding error, which used to trip
/// a debug assertion in the ball square root (`x = 1 − 3·2^-77` at 26 bits).
#[test]
fn test_acos_asin_next_to_one_wide_argument() {
    let x = FBig::<HalfAway>::from_parts((IBig::ONE << 77) - IBig::from(3u8), -77);
    let ctx = Context::<HalfAway>::new(26);
    // acos(1 − δ) = √(2δ)·(1 + δ/12 + …) with δ = 3·2^-77
    let acos = ctx.acos(x.repr(), None).unwrap().value();
    let want = (6.0f64).sqrt() * (2.0f64).powf(-38.5);
    assert!((acos.to_f64().value() / want - 1.0).abs() < 1e-7, "acos = {acos:?}");
    let asin = ctx.asin(x.repr(), None).unwrap().value();
    assert!((asin.to_f64().value() - (core::f64::consts::FRAC_PI_2 - want)).abs() < 1e-7);

    // A small sweep against the same function at a precision that holds the argument exactly
    // (so the first attempt already takes the general path), rounded once more.
    for p in [20usize, 26, 50, 100] {
        for k in [p + 10, 2 * p + 1, 3 * p - 1] {
            for m in [1u8, 3, 5, 7] {
                let x = Repr::<2>::new((IBig::ONE << k) - IBig::from(m), -(k as isize));
                let wide = Context::<HalfEven>::new(p + k + 80);
                let narrow = Context::<HalfEven>::new(p);
                let acos = narrow.acos(&x, None).unwrap().value();
                let asin = narrow.asin(&x, None).unwrap().value();
                let acos_wide = wide.acos(&x, None).unwrap().value();
                let asin_wide = wide.asin(&x, None).unwrap().value();
                assert_eq!(acos, acos_wide.with_precision(p).value(), "acos p={p} k={k} m={m}");
                assert_eq!(asin, asin_wide.with_precision(p).value(), "asin p={p} k={k} m={m}");
                // and the mirror image at −1
                let neg = -x.clone();
                let asin_neg = narrow.asin(&neg, None).unwrap().value();
                assert_eq!(asin_neg, -asin, "asin(-x) p={p} k={k} m={m}");
            }
        }
    }
}

/// `cos(x) < 1` and `|sin(x)| < |x|` for every nonzero `x`: the result of a small argument is
/// never `Exact`, even when the working precision cannot tell it from `1` or `x`.
#[test]
fn test_sin_cos_small_argument_is_inexact() {
    type F = FBig<HalfEven, 2>;
    for (sig, exp) in [(5, -29), (1, -40), (1, -10), (1, -1)] {
        let x = Repr::<2>::new(IBig::from(sig), exp);
        for p in 1..=4 {
            let ctx = Context::<HalfEven>::new(p);
            let (s, c) = ctx.sin_cos(&x, None);
            for r in [ctx.sin(&x, None), ctx.cos(&x, None), s, c] {
                assert!(matches!(r, Ok(Inexact(..))), "{sig}·2^{exp} at {p} bits: {r:?}");
            }
        }
    }
    for (sig, exp) in [(1, -40), (5, -29)] {
        let x = Repr::<2>::new(IBig::from(sig), exp);
        for p in 1..=3 {
            let ctx = Context::<HalfEven>::new(p);
            assert_eq!(ctx.cos(&x, None), Ok(Inexact(F::ONE, AddOne)));
            assert_eq!(ctx.sin_cos(&x, None).1, Ok(Inexact(F::ONE, AddOne)));
        }
        // where x is representable, sin(x) rounds back up to it
        for p in 3..=4 {
            let want = F::from_parts(sig.into(), exp);
            assert_eq!(Context::<HalfEven>::new(p).sin(&x, None), Ok(Inexact(want, AddOne)));
        }
    }
    // 5·2^-29 = 101b·2^-29 at 1 bit: sin(x) is just below x, which truncates to 100b
    let x = Repr::<2>::new(IBig::from(5), -29);
    assert_eq!(
        Context::<HalfEven>::new(1).sin(&x, None),
        Ok(Inexact(F::from_parts(1.into(), -27), NoOp))
    );
}

/// A tiny argument under a directed mode: `sin(x)` is the float just inside `x` and `cos(x)`
/// the float just below `1`, decided without raising the working precision to `2·|log2 x|`
/// bits (which took seconds at `2^-1000000` and cannot finish at all further down).
#[test]
fn test_sin_cos_tiny_argument_directed() {
    let p = 53usize;
    let ones = (IBig::ONE << p) - IBig::ONE;
    for e in [
        -1000isize,
        -1_000_000,
        isize::MIN / 4,
        -(isize::MAX / 2) - 10,
    ] {
        let x = Repr::<2>::new(IBig::ONE, e);
        // the float just inside ±x: all ones, one binade down
        fn just_inside<R: round::Round>(ones: &IBig, neg: bool, exp: isize) -> FBig<R, 2> {
            FBig::from_parts(if neg { -ones.clone() } else { ones.clone() }, exp)
        }
        let inside = e - p as isize;
        let down = Context::<Down>::new(p);
        let up = Context::<Up>::new(p);
        assert_eq!(
            down.sin(&x, None),
            Ok(Inexact(just_inside(&ones, false, inside), NoOp)),
            "e = {e}"
        );
        assert_eq!(up.sin(&x, None), Ok(Inexact(FBig::from_parts(1.into(), e), AddOne)));
        assert_eq!(up.sin(&-x.clone(), None), Ok(Inexact(just_inside(&ones, true, inside), NoOp)));
        assert_eq!(
            down.sin(&-x.clone(), None),
            Ok(Inexact(FBig::from_parts((-1).into(), e), SubOne))
        );
        assert_eq!(up.cos(&x, None), Ok(Inexact(FBig::ONE, AddOne)));
        assert_eq!(
            down.cos(&x, None),
            Ok(Inexact(FBig::from_parts(ones.clone(), -(p as isize)), NoOp))
        );
        let (s, c) = down.sin_cos(&x, None);
        assert_eq!((s, c), (down.sin(&x, None), down.cos(&x, None)));
        // tan and asin move away from zero, atan towards it
        assert_eq!(down.tan(&x, None), Ok(Inexact(FBig::from_parts(1.into(), e), NoOp)));
        assert_eq!(down.asin(&x, None), Ok(Inexact(FBig::from_parts(1.into(), e), NoOp)));
        assert_eq!(down.atan(&x, None), Ok(Inexact(just_inside(&ones, false, inside), NoOp)));
        // atan2 with a power of the base for x is the same atan
        let four = Repr::<2>::new(IBig::ONE, 2);
        let quarter = Repr::<2>::new(IBig::ONE, e - 2);
        assert_eq!(down.atan2(&x, &four, None), down.atan(&quarter, None));
    }
}

/// A finite argument whose exponent is in the lower half of the `isize` range: `x²` is not
/// representable there, and the series kernels used to report its underflow as the result
/// (`cos`, `sin`, `tan`, `atan`, `asin` and `acos` all returned 0).
#[test]
fn test_trig_argument_with_exponent_near_half_isize_min() {
    let one = DBig::ONE.with_precision(10).value();
    let half_pi = DBig::ZERO.with_precision(10).value().acos();
    assert_eq!(half_pi.to_string(), "1.570796327");
    for e in [isize::MIN / 4, -(isize::MAX / 2) - 10, isize::MIN + 100] {
        for m in [1i64, -1, 123456789] {
            let x = DBig::from_parts(IBig::from(m), e)
                .with_precision(10)
                .value();
            assert_eq!(x.cos(), one, "cos({m}e{e})");
            assert_eq!(x.sin(), x, "sin({m}e{e})");
            assert_eq!(x.sin_cos(), (x.clone(), one.clone()));
            assert_eq!(x.tan(), x, "tan({m}e{e})");
            assert_eq!(x.atan(), x, "atan({m}e{e})");
            assert_eq!(x.asin(), x, "asin({m}e{e})");
            assert_eq!(x.acos(), half_pi, "acos({m}e{e})");
            assert_eq!(x.atan2(&one), x, "atan2({m}e{e}, 1)");
            // the ×u family: cos is 1, acos a quarter turn, and sin / tan / asin / atan scale
            // the argument by 2π/u or u/(2π) — the same digits as for an ordinary exponent
            assert_eq!(x.cos_unit(360), one, "cos_unit({m}e{e})");
            assert_eq!(x.acos_unit(360), DBig::from(90), "acos_unit({m}e{e})");
            let y = DBig::from_parts(IBig::from(m), -50)
                .with_precision(10)
                .value();
            let shift = e + 50;
            let scaled = |r: DBig| {
                let (sig, exp) = r.into_repr().into_parts();
                DBig::from_parts(sig, exp + shift)
            };
            assert_eq!(x.sin_unit(360), scaled(y.sin_unit(360)), "sin_unit({m}e{e})");
            assert_eq!(x.tan_unit(360), scaled(y.tan_unit(360)), "tan_unit({m}e{e})");
            assert_eq!(x.asin_unit(360), scaled(y.asin_unit(360)), "asin_unit({m}e{e})");
            assert_eq!(x.atan_unit(360), scaled(y.atan_unit(360)), "atan_unit({m}e{e})");
        }
    }
}

/// Right at the bottom of the exponent range the neighbours of `x` have no representable last
/// place, so the functions that return `x ± tiny` report an underflow (promptly, under every
/// mode) while `cos` is still the float next to 1.
#[test]
fn test_trig_argument_at_bottom_of_exponent_range() {
    let x = Repr::<2>::new(IBig::ONE, isize::MIN + 10);
    let neg = -x.clone();
    let down = Context::<Down>::new(53);
    let even = Context::<HalfEven>::new(53);
    assert_eq!(down.sin(&x, None), Err(FpError::Underflow(Sign::Positive)));
    assert_eq!(even.sin(&neg, None), Err(FpError::Underflow(Sign::Negative)));
    assert_eq!(down.tan(&x, None), Err(FpError::Underflow(Sign::Positive)));
    assert_eq!(down.asin(&neg, None), Err(FpError::Underflow(Sign::Negative)));
    assert_eq!(even.atan(&x, None), Err(FpError::Underflow(Sign::Positive)));
    assert_eq!(even.cos(&x, None), Ok(Inexact(FBig::ONE, AddOne)));
    let (s, c) = down.sin_cos(&x, None);
    assert_eq!(s, Err(FpError::Underflow(Sign::Positive)));
    assert!(matches!(c, Ok(Inexact(_, NoOp))));
}

/// The inverse ×u functions next to an axis: the angle is an exactly representable quarter
/// turn displaced by an amount far below the precision, on a known side. `acos_unit(2^-52, 3)`
/// at 1 bit is `0.75 − tiny` — below the midpoint of 0.5 and 1 — and used to come back as 1;
/// at 2 bits it is 0.75, but not exactly.
#[test]
fn test_inverse_unit_next_to_quarter_turn() {
    type F = FBig<HalfEven, 2>;
    for k in [10isize, 30, 52, 100, 1_000_000] {
        let x = Repr::<2>::new(IBig::ONE, -k);
        let (half, three_q) = (F::from_parts(1.into(), -1), F::from_parts(3.into(), -2));
        let one_bit = Context::<HalfEven>::new(1);
        let two_bits = Context::<HalfEven>::new(2);
        assert_eq!(one_bit.acos_unit(&x, 3, None), Ok(Inexact(half, NoOp)), "k = {k}");
        assert_eq!(two_bits.acos_unit(&x, 3, None), Ok(Inexact(three_q.clone(), AddOne)));
        // a negative argument sits just above 0.75
        let neg = -x;
        assert_eq!(one_bit.acos_unit(&neg, 3, None), Ok(Inexact(F::ONE, AddOne)));
        assert_eq!(two_bits.acos_unit(&neg, 3, None), Ok(Inexact(three_q, NoOp)));
    }

    // degrees, 10 digits, rounding down: just below / just above the axis
    let d = |s: &str| {
        FBig::<Down, 10>::from_str(s)
            .unwrap()
            .with_precision(10)
            .value()
    };
    let below_90 = d("89.99999999");
    for tiny in ["1e-30", "1e-1000000"] {
        assert_eq!(d(tiny).acos_unit(360), below_90);
        assert_eq!(d(&format!("-{tiny}")).acos_unit(360), d("90"));
        // atan2_unit: next to the y axis, and next to the negative x axis
        assert_eq!(d("1").atan2_unit(&d(tiny), 360), below_90);
        assert_eq!(d("1").atan2_unit(&d(&format!("-{tiny}")), 360), d("90"));
        assert_eq!(d("-1").atan2_unit(&d(tiny), 360), d("-90"));
        assert_eq!(d("-1").atan2_unit(&d(&format!("-{tiny}")), 360), d("-90.00000001"));
        assert_eq!(d(tiny).atan2_unit(&d("-1"), 360), d("179.9999999"));
        assert_eq!(d(&format!("-{tiny}")).atan2_unit(&d("-1"), 360), d("-180"));
    }
    for huge in ["1e30", "1e1000000"] {
        assert_eq!(d(huge).atan_unit(360), below_90);
        assert_eq!(d(&format!("-{huge}")).atan_unit(360), d("-90"));
    }
    // cos_unit next to a whole turn: 1 − tiny, whatever the argument's own width
    let near_turn = FBig::<Down, 10>::from_str("360.0000000000000000000000000001").unwrap();
    let ctx = Context::<Down>::new(10);
    let nines = d("0.9999999999");
    assert_eq!(ctx.cos_unit(near_turn.repr(), 360, None), Ok(Inexact(nines.clone(), NoOp)));
    let (s, c) = ctx.sin_cos_unit(near_turn.repr(), 360, None);
    assert_eq!(c, Ok(Inexact(nines, NoOp)));
    assert_eq!(s, ctx.sin_unit(near_turn.repr(), 360, None));
    assert_eq!(
        Context::<Up>::new(10).cos_unit(near_turn.repr(), 360, None),
        Ok(Inexact(FBig::ONE, AddOne))
    );

    // in an odd base a quarter turn of 2 has no finite expansion: nothing to hug, the Ziv
    // loop settles it — and the result is the plain rounding of 1/2
    let x = FBig::<HalfEven, 3>::from_parts(1.into(), -40)
        .with_precision(10)
        .value();
    let half = FBig::<HalfEven, 3>::ONE.with_precision(10).value() / 2;
    assert_eq!(x.acos_unit(2), half);
}

/// In an odd base every half-integer is the midpoint of two neighbouring floats (`1/2` is
/// `0.111…` in base 3), so an inverse ×u function next to an axis whose angle is a half-integer
/// is a near-tie under the round-to-nearest modes. The loop used to run forever once the
/// result was within `3^-129` ulps of the midpoint (`acos_unit(3^-140, 2)`; `3^-130` worked):
/// the half-ulp bound of an odd base is cut off there.
#[test]
fn test_inverse_unit_next_to_half_in_odd_base() {
    // the two neighbours of `q = k/2` (k odd) at `p` digits: (B^p ∓ 1)/2 · B^(e − p) for a
    // `q` whose leading digit has exponent `e − 1`
    fn neighbours<R: round::Round, const B: Word>(k: u32, p: usize) -> (FBig<R, B>, FBig<R, B>) {
        // k/2 = (k·B^s ∓ 1)/2 · B^-s, with s chosen so the significand has exactly p digits
        let base = IBig::from(B);
        let mut s = 0usize;
        while (IBig::from(k) * base.pow(s) - IBig::ONE) / IBig::from(2) < base.pow(p - 1) {
            s += 1;
        }
        let lo: IBig = (IBig::from(k) * base.pow(s) - IBig::ONE) / IBig::from(2);
        let exp = -(s as isize);
        (FBig::from_parts(lo.clone(), exp), FBig::from_parts(lo + IBig::ONE, exp))
    }
    fn check<R: round::ErrorBounds, const B: Word>(u: usize, p: usize, k: isize) {
        let ctx = Context::<R>::new(p);
        let (lo, hi) = neighbours::<R, B>(u as u32 / 2, p);
        let tiny = Repr::<B>::new(IBig::ONE, -k);
        let huge = Repr::<B>::new(IBig::ONE, k);
        let one = Repr::<B>::one();
        let tag = format!("base {B}, u = {u}, p = {p}, k = {k}");
        // u/4 − δ: the lower neighbour, a plain truncation
        let below = Ok(Inexact(lo.clone(), NoOp));
        // u/4 + δ: the upper neighbour
        let above = Ok(Inexact(hi.clone(), AddOne));
        assert_eq!(ctx.acos_unit(&tiny, u, None), below, "acos_unit, {tag}");
        assert_eq!(ctx.acos_unit(&-tiny.clone(), u, None), above, "acos_unit(-), {tag}");
        assert_eq!(ctx.atan_unit(&huge, u, None), below, "atan_unit, {tag}");
        assert_eq!(
            ctx.atan_unit(&-huge.clone(), u, None),
            Ok(Inexact(-lo.clone(), NoOp)),
            "atan_unit(-), {tag}"
        );
        assert_eq!(ctx.atan2_unit(&one, &tiny, u, None), below, "atan2_unit, {tag}");
        assert_eq!(ctx.atan2_unit(&one, &-tiny.clone(), u, None), above, "atan2_unit(-x), {tag}");
        assert_eq!(
            ctx.atan2_unit(&-one.clone(), &-tiny, u, None),
            Ok(Inexact(-hi, SubOne)),
            "atan2_unit(-y, -x), {tag}"
        );
    }
    // 10 is below the shortcut's reach (the loop decides), 140 is the reported case, and the
    // rest could never be reached by raising the working precision
    for k in [10isize, 16, 40, 130, 140, 1000, 1_000_000, isize::MAX / 4] {
        for p in [1usize, 10, 20, 50] {
            if k < p as isize + 6 {
                continue; // the displacement is not below the last place
            }
            check::<HalfEven, 3>(2, p, k); // next to 1/2
            check::<HalfAway, 3>(2, p, k);
            check::<HalfEven, 3>(6, p, k); // next to 3/2
            check::<HalfEven, 5>(2, p, k);
            check::<HalfAway, 7>(10, p, k); // next to 5/2
        }
    }

    // A quarter that is not a half-integer (1/4 in base 3 is 0.0202…) is no rounding boundary
    // at all: the result is the plain rounding of the quarter under every mode.
    fn check_plain<R: round::ErrorBounds>(p: usize, k: isize) {
        let ctx = Context::<R>::new(p);
        let quarter = ctx
            .div(&Repr::<3>::one(), &Repr::<3>::new(4.into(), 0))
            .unwrap()
            .value();
        let tiny = Repr::<3>::new(IBig::ONE, -k);
        assert_eq!(ctx.acos_unit(&tiny, 1, None).unwrap().value(), quarter, "p = {p}, k = {k}");
        assert_eq!(ctx.acos_unit(&-tiny, 1, None).unwrap().value(), quarter, "p = {p}, k = {k}");
    }
    for k in [60isize, 140, 1_000_000] {
        for p in [10usize, 20, 50] {
            check_plain::<HalfEven>(p, k);
            check_plain::<Up>(p, k);
            check_plain::<Down>(p, k);
        }
    }

    // the directed modes never saw a boundary at 1/2
    let tiny = Repr::<3>::new(IBig::ONE, -140);
    let (lo, _) = neighbours::<Down, 3>(1, 10);
    let down = Context::<Down>::new(10);
    assert_eq!(down.acos_unit(&tiny, 2, None), Ok(Inexact(lo.clone(), NoOp)));
    assert_eq!(down.acos_unit(&-tiny.clone(), 2, None), Ok(Inexact(lo, NoOp)));
    let (_, hi) = neighbours::<Up, 3>(1, 10);
    assert_eq!(Context::<Up>::new(10).acos_unit(&tiny, 2, None), Ok(Inexact(hi, AddOne)));
}

/// `atan2_unit` on the four diagonals: the angle is `±u/8` / `±3u/8` with the sign of `y`
/// and the quadrant of `x` (the two negative-`y` diagonals used to be swapped).
#[test]
fn test_atan2_unit_diagonals() {
    let d = |s: &str| DBig::from_str(s).unwrap();
    for (y, x, want) in [
        ("1.000", "1.000", 45),
        ("1.000", "-1.000", 135),
        ("-1.000", "1.000", -45),
        ("-1.000", "-1.000", -135),
        ("-2.50", "2.50", -45),
        ("-2.50", "-2.50", -135),
    ] {
        assert_eq!(d(y).atan2_unit(&d(x), 360), DBig::from(want), "atan2_unit({y}, {x}, 360)");
    }
    // the same rows in base 2, where k/8 is exact: u = 8 gives the octant index itself
    type F = FBig<HalfEven, 2>;
    for p in [20usize, 50, 100, 500] {
        let f = |v: i32| F::from(v).with_precision(p).value();
        for (y, x, want) in [(3, 3, 1), (3, -3, 3), (-3, 3, -1), (-3, -3, -3)] {
            assert_eq!(f(y).atan2_unit(&f(x), 8), F::from(want), "p = {p}, ({y}, {x})");
            // and the radian atan2 agrees on the sign
            assert_eq!(f(y).atan2(&f(x)).sign(), f(want).sign());
        }
    }
}

/// The `±1/2` exact row of `asin_unit` / `acos_unit` is matched exactly. In an odd base `1/2`
/// is not representable, and arguments such as 2/3 (base 3) or 2/5 (base 5) used to be
/// mistaken for it (a rounded `1/2` constant was compared instead).
#[test]
fn test_asin_acos_unit_half_row_is_exact() {
    fn check<R: round::ErrorBounds, const B: Word>(sig: i32, exp: isize) {
        let xf = sig as f64 * (B as f64).powi(exp as i32);
        for p in [10usize, 20, 50] {
            let x = FBig::<R, B>::from_parts(sig.into(), exp)
                .with_precision(p)
                .value();
            let a = x.asin_unit(360).to_f64().value();
            let c = x.acos_unit(360).to_f64().value();
            let (ea, ec) = (xf.asin().to_degrees(), xf.acos().to_degrees());
            // 10 base-B digits carry at least 10·log2(3) > 15 bits
            assert!((a - ea).abs() < 1e-2, "asin_unit({sig}·{B}^{exp}) = {a}, expected {ea}");
            assert!((c - ec).abs() < 1e-2, "acos_unit({sig}·{B}^{exp}) = {c}, expected {ec}");
        }
    }
    check::<HalfEven, 3>(2, -1); // 2/3
    check::<HalfEven, 3>(-2, -1);
    check::<HalfEven, 3>(7, -2); // 7/9
    check::<Down, 3>(1, -1); // 1/3
    check::<HalfEven, 5>(2, -1); // 2/5
    check::<Down, 5>(3, -1); // 3/5
    check::<HalfEven, 7>(3, -1);
    check::<HalfAway, 7>(4, -1);

    // the genuine 1/2 keeps resolving exactly in the even bases, whatever its spelling
    let half = DBig::from_str("0.5000").unwrap();
    assert_eq!(half.asin_unit(360), DBig::from(30));
    assert_eq!(half.acos_unit(360), DBig::from(60));
    assert_eq!((-half.clone()).asin_unit(360), DBig::from(-30));
    assert_eq!((-half).acos_unit(360), DBig::from(120));
    type F = FBig<HalfEven, 2>;
    for p in [20usize, 50, 100, 500] {
        let half = F::from_parts(1.into(), -1).with_precision(p).value();
        assert_eq!(half.asin_unit(12), F::ONE);
        assert_eq!(half.acos_unit(12), F::from(2));
        // a neighbour of 1/2 is not on the row
        let near = F::from_parts((IBig::ONE << (p - 1)) + IBig::ONE, -(p as isize))
            .with_precision(p)
            .value();
        assert!(near.asin_unit(12) > F::ONE);
        assert!(near.acos_unit(12) < F::from(2));
    }
}
