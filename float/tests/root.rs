use dashu_base::{Approximation::*, Sign};
use dashu_float::{
    ops::CubicRoot,
    round::{mode, ErrorBounds, Round, Rounded, Rounding::*},
    Context, DBig, FBig, Repr, Word,
};
use dashu_int::IBig;

mod helper_macros;

#[test]
#[rustfmt::skip::macros(fbig)]
fn test_sqrt_binary() {
    let exact_cases = [
        (fbig!(0), fbig!(0)),
        (fbig!(1), fbig!(1)),
        (fbig!(0x9), fbig!(0x3)),
        (fbig!(0x100), fbig!(0x10)),
    ];
    for (x, sqrt) in &exact_cases {
        assert_eq!(x.sqrt(), *sqrt);
        if let Ok(Exact(v)) = x.context().sqrt(x.repr()) {
            assert_eq!(v, *sqrt);
        } else {
            panic!("the result should be exact!")
        }
    }

    let inexact_cases = [
        (fbig!(0x3), fbig!(0xdp-3)),
        (fbig!(0x0003), fbig!(0xddb3p-15)),
        (fbig!(0x0000000000000003), fbig!(0xddb3d742c265539dp-63)),
        (
            fbig!(0x3).with_precision(200).value(),
            fbig!(0xddb3d742c265539d92ba16b83c5c1dc492ec1a6629ed23cc63p-199),
        ),
        (fbig!(0x3000), fbig!(0xddb3p-9)),
        (fbig!(0x3000000000000000), fbig!(0xddb3d742c265539dp-33)),
        (fbig!(0xf), fbig!(0xfp-2)),
        (fbig!(0xffff), fbig!(0xffffp-8)),
    ];

    for (x, root) in &inexact_cases {
        assert_eq!(x.sqrt(), *root);
        if let Ok(Inexact(v, e)) = x.context().sqrt(x.repr()) {
            assert_eq!(v, *root);
            assert_eq!(e, NoOp);
        } else {
            panic!("the result should be inexact!")
        }
    }
}

#[test]
fn test_sqrt_decimal() {
    let exact_cases = [
        (dbig!(0), dbig!(0)),
        (dbig!(1), dbig!(1)),
        (dbig!(9), dbig!(3)),
        (dbig!(6561), dbig!(81)),
    ];
    for (x, sqrt) in &exact_cases {
        assert_eq!(x.sqrt(), *sqrt);
        if let Ok(Exact(v)) = x.context().sqrt(x.repr()) {
            assert_eq!(v, *sqrt);
        } else {
            panic!("the result should be exact!")
        }
    }

    let inexact_cases = [
        (dbig!(3), dbig!(2), AddOne),
        (dbig!(0003), dbig!(1732e-3), NoOp),
        (dbig!(0000000000000003), dbig!(1732050807568877e-15), NoOp),
        (
            dbig!(3).with_precision(60).value(),
            dbig!(173205080756887729352744634150587236694280525381038062805581e-59),
            AddOne,
        ),
        (dbig!(3000), dbig!(5477e-2), NoOp),
        (dbig!(3000000000000000), dbig!(5477225575051661e-8), NoOp),
        (dbig!(9999), dbig!(9999e-2), NoOp),
        (dbig!(9999e-4), dbig!(9999e-4), NoOp),
    ];
    for (x, sqrt, rnd) in &inexact_cases {
        assert_eq!(x.sqrt(), *sqrt);
        if let Ok(Inexact(v, e)) = x.context().sqrt(x.repr()) {
            assert_eq!(v, *sqrt);
            assert_eq!(e, *rnd);
        } else {
            panic!("the result should be inexact!")
        }
    }
}

#[test]
#[rustfmt::skip::macros(fbig)]
fn test_cbrt_binary() {
    let exact_cases = [
        (fbig!(0), fbig!(0)),
        (fbig!(1), fbig!(1)),
        (fbig!(0x8), fbig!(0x2)),
        (fbig!(0x1000), fbig!(0x10)), // 4096 = 16^3
        (fbig!(0x8000), fbig!(0x20)), // 32768 = 32^3
    ];
    for (x, cbrt) in &exact_cases {
        assert_eq!(x.cbrt(), *cbrt);
        if let Ok(Exact(v)) = x.context().cbrt(x.repr()) {
            assert_eq!(v, *cbrt);
        } else {
            panic!("the result should be exact!")
        }
    }

    let inexact_cases = [
        (fbig!(0x3), fbig!(0xbp-3)),
        (fbig!(0x0003), fbig!(0xb89bp-15)),
        (fbig!(0x0000000000000003), fbig!(0xb89ba24891f7b2e6p-63)),
        (
            fbig!(0x3).with_precision(200).value(),
            fbig!(0xb89ba24891f7b2e6ef3f8b62b71933e050c4a6157ab766ccfap-199),
        ),
        (fbig!(0x300000), fbig!(0x9285ffp-16)),
        (fbig!(0x300000000000000000000000), fbig!(0x9285ff0d8417a7cdc6a6742ep-64)),
    ];
    for (x, root) in &inexact_cases {
        assert_eq!(x.cbrt(), *root);
        if let Ok(Inexact(v, e)) = x.context().cbrt(x.repr()) {
            assert_eq!(v, *root);
            assert_eq!(e, NoOp);
        } else {
            panic!("the result should be inexact!")
        }
    }
}

#[test]
fn test_cbrt_decimal() {
    let exact_cases = [
        (dbig!(0), dbig!(0)),
        (dbig!(1), dbig!(1)),
        (dbig!(8), dbig!(2)),
        (dbig!(27), dbig!(3)),
        (dbig!(1000), dbig!(10)),
        (dbig!(8000), dbig!(20)),
    ];
    for (x, cbrt) in &exact_cases {
        assert_eq!(x.cbrt(), *cbrt);
        if let Ok(Exact(v)) = x.context().cbrt(x.repr()) {
            assert_eq!(v, *cbrt);
        } else {
            panic!("the result should be exact!")
        }
    }

    let inexact_cases = [
        (dbig!(3), dbig!(1), NoOp),
        (dbig!(0003), dbig!(1442e-3), NoOp),
        (dbig!(0000000000000003), dbig!(1442249570307408e-15), NoOp),
        (
            dbig!(3).with_precision(60).value(),
            dbig!(144224957030740838232163831078010958839186925349935057754642e-59),
            AddOne,
        ),
        (dbig!(300000), dbig!(669433e-4), AddOne),
        (dbig!(300000000000000000000000), dbig!(669432950082169521882659e-16), NoOp),
        (dbig!(999999), dbig!(100), AddOne),
        (dbig!(999999e-6), dbig!(1.00000), AddOne),
    ];
    for (x, cbrt, rnd) in &inexact_cases {
        assert_eq!(x.cbrt(), *cbrt);
        if let Ok(Inexact(v, e)) = x.context().cbrt(x.repr()) {
            assert_eq!(v, *cbrt);
            assert_eq!(e, *rnd);
        } else {
            panic!("the result should be inexact!")
        }
    }
}

#[test]
fn test_nth_root_exact_decimal() {
    // (input, n, expected)
    let cases: &[(DBig, usize, DBig)] = &[
        (dbig!(1), 7, dbig!(1)),
        (dbig!(4), 2, dbig!(2)),
        (dbig!(27), 3, dbig!(3)),
        (dbig!(16), 4, dbig!(2)),
        (dbig!(1024), 5, dbig!(4)),  // 4^5 = 1024
        (dbig!(2187), 7, dbig!(3)),  // 3^7 = 2187
        (dbig!(4096), 12, dbig!(2)), // 2^12 = 4096
    ];
    for (x, n, expected) in cases {
        assert_eq!(x.nth_root(*n), *expected, "nth_root({n}) of {x:?}");
        match x.context().nth_root(*n, x.repr()) {
            Ok(Exact(v)) => assert_eq!(v, *expected),
            _ => panic!("the result should be exact!"),
        }
        // the identity case returns the value unchanged
        assert_eq!(x.nth_root(1), *x);
    }

    let inexact_cases = [
        // 20 digits
        (dbig!(00000000000000000003), 4, dbig!(1.3160740129524924608), NoOp),
        (dbig!(00000000000000000003), 5, dbig!(1.2457309396155173260), AddOne),
        (dbig!(00000000000000000003), 6, dbig!(1.2009369551760027267), AddOne),
        (dbig!(00000000000000000003), 7, dbig!(1.1699308127586868865), AddOne),
        (dbig!(00000000000000000003), 8, dbig!(1.1472026904398770895), AddOne),
        (dbig!(00000000000000000003), 9, dbig!(1.1298309639097530326), NoOp),
        (dbig!(00000000000000000003), 10, dbig!(1.1161231740339044344), NoOp),
        (dbig!(00000000000000000003), 11, dbig!(1.1050315033964666965), NoOp),
        (dbig!(00000000000000000003), 10000, dbig!(1.0001098672638326159), NoOp),
    ];
    for (x, n, root, rnd) in &inexact_cases {
        assert_eq!(x.nth_root(*n), *root);
        if let Ok(Inexact(v, e)) = x.context().nth_root(*n, x.repr()) {
            assert_eq!(v, *root);
            assert_eq!(e, *rnd);
        } else {
            panic!("the result should be inexact!")
        }
    }
}

#[test]
fn test_cbrt_and_nth_root_negative() {
    // odd roots of negative numbers are real and negative
    assert_eq!(fbig!(-0x8).cbrt(), fbig!(-0x2));
    assert_eq!(dbig!(-27).cbrt(), dbig!(-3));
    assert_eq!(dbig!(-1).cbrt(), dbig!(-1));
    assert_eq!(dbig!(-8).nth_root(3), dbig!(-2));
    assert_eq!(dbig!(-32).nth_root(5), dbig!(-2)); // (-2)^5 = -32
    assert_eq!(dbig!(-1).nth_root(9), dbig!(-1));

    // cbrt(-x) == -cbrt(x)
    let x = dbig!(2);
    assert_eq!(dbig!(-2).cbrt(), -x.cbrt());
}

#[test]
#[rustfmt::skip::macros(fbig)]
fn test_nth_root_equals_sqrt() {
    // nth_root(2, x) must match sqrt(x) exactly (value and rounding flag),
    // which exercises the full general nth-root path against the trusted sqrt.
    let binary_cases = [
        fbig!(0x3),
        fbig!(0x3000),
        fbig!(0xf),
        fbig!(0xffff),
        fbig!(0xfeed),
        fbig!(0x3).with_precision(200).value(),
        fbig!(0x1234).with_precision(80).value(),
    ];
    for x in &binary_cases {
        let by_sqrt = x.context().sqrt(x.repr());
        let by_nth = x.context().nth_root(2, x.repr());
        assert_eq!(by_sqrt, by_nth, "nth_root(2) != sqrt for {x:?}");
    }

    let decimal_cases = [
        dbig!(3),
        dbig!(2),
        dbig!(3000),
        dbig!(9999),
        dbig!(3).with_precision(60).value(),
    ];
    for x in &decimal_cases {
        let by_sqrt = x.context().sqrt(x.repr());
        let by_nth = x.context().nth_root(2, x.repr());
        assert_eq!(by_sqrt, by_nth, "nth_root(2) != sqrt for {x:?}");
    }
}

/// For toward-zero rounding (the binary FBig default), the correctly rounded
/// nth root `r` satisfies `r^n <= x < (r + ulp(r))^n`. The powers are evaluated
/// at unlimited precision so the bracketing comparison is exact.
#[rustfmt::skip::macros(fbig)]
fn assert_root_bracketed_bin(x: FBig, n: usize) {
    let r = x.nth_root(n);
    let ulp = r.ulp();
    let r = r.with_precision(0).value();
    let ulp = ulp.with_precision(0).value();
    let x = x.with_precision(0).value();

    let lower = r.clone().powi(IBig::from(n));
    let upper = (r.clone() + ulp).powi(IBig::from(n));
    assert!(
        lower <= x && x < upper,
        "nth_root({n}) bracketing failed for input: expected {lower:?} <= x < {upper:?}"
    );
}

#[test]
#[rustfmt::skip::macros(fbig)]
fn test_nth_root_rounding_binary() {
    let inputs = [
        fbig!(0x2),
        fbig!(0x3),
        fbig!(0x5),
        fbig!(0x9),
        fbig!(0x100),
        fbig!(0x1ff),
        fbig!(0x3000),
        fbig!(0xfeed),
        fbig!(0xdeadbeef),
        fbig!(0x3).with_precision(100).value(),
        fbig!(0x2).with_precision(64).value(),
        fbig!(0xfeed).with_precision(37).value(),
    ];
    for x in &inputs {
        for n in [2usize, 3, 5, 7, 10, 13] {
            assert_root_bracketed_bin(x.clone(), n);
        }
    }
}

/// For nearest rounding (the decimal DBig default), the correctly rounded nth
/// root `r` lies within one ulp of the true root, so
/// `(r - ulp)^n < x < (r + ulp)^n`.
fn assert_root_within_ulp_dec(x: DBig, n: usize) {
    let r = x.nth_root(n);
    let ulp = r.ulp();
    let r = r.with_precision(0).value();
    let ulp = ulp.with_precision(0).value();
    let x = x.with_precision(0).value();

    let lower = (r.clone() - ulp.clone()).powi(IBig::from(n));
    let upper = (r + ulp).powi(IBig::from(n));
    assert!(
        lower < x && x < upper,
        "nth_root({n}) bracketing failed for input: expected {lower:?} < x < {upper:?}"
    );
}

#[test]
fn test_nth_root_rounding_decimal() {
    let inputs = [
        dbig!(2),
        dbig!(3),
        dbig!(10),
        dbig!(2000),
        dbig!(99999),
        dbig!(2).with_precision(50).value(),
        dbig!(123).with_precision(40).value(),
    ];
    for x in &inputs {
        for n in [2usize, 3, 5, 7] {
            assert_root_within_ulp_dec(x.clone(), n);
        }
    }
}

#[test]
#[should_panic]
fn test_sqrt_unlimited_precision() {
    let _ = dbig!(2).with_precision(0).value().sqrt();
}

#[test]
#[should_panic]
fn test_sqrt_inf() {
    let _ = DBig::INFINITY.sqrt();
}

#[test]
#[should_panic]
fn test_sqrt_negative() {
    let _ = DBig::NEG_ONE.sqrt();
}

#[test]
#[should_panic]
fn test_nth_root_zero_degree() {
    let _ = dbig!(8).nth_root(0);
}

#[test]
#[should_panic]
fn test_nth_root_even_of_negative() {
    let _ = dbig!(-4).nth_root(2);
}

#[test]
#[should_panic]
fn test_cbrt_inf() {
    let _ = DBig::INFINITY.cbrt();
}

#[test]
#[should_panic]
fn test_cbrt_unlimited_precision() {
    let _ = dbig!(2).with_precision(0).value().cbrt();
}

// ---------------------------------------------------------------------------------------------
// Exact oracle for a rounded square root: the floor neighbour is bracketed by exact squares and
// the rounding decision is the mode's own `round_low_part` on the exact midpoint comparison.
// ---------------------------------------------------------------------------------------------

/// The value of a `Repr` as an unlimited-precision float (exact arithmetic).
fn exact<const B: Word>(r: &Repr<B>) -> FBig<mode::Zero, B> {
    FBig::from_repr(r.clone(), Context::new(0))
}

fn exact_sqr<const B: Word>(r: &Repr<B>) -> FBig<mode::Zero, B> {
    let v = exact(r);
    &v * &v
}

/// The correctly rounded `sqrt(sum)` at `p` digits under `R`, given any `p`-digit value `near`
/// within one ulp of the root. `sum` is an exact unlimited-precision value.
fn sqrt_oracle<R: Round, const B: Word>(
    p: usize,
    sum: &FBig<mode::Zero, B>,
    near: &Repr<B>,
) -> Rounded<Repr<B>> {
    let digits = near.digits();
    assert!(digits <= p, "{near:?} has more than {p} digits");
    let base = IBig::from(B);
    let mut sig = near.significand() * base.clone().pow(p - digits);
    let mut exp = near.exponent() - (p - digits) as isize;
    if exact_sqr(near) > *sum {
        // step down to the floor neighbour (into the finer binade below a power of the base)
        if sig == base.clone().pow(p - 1) {
            sig = base.pow(p) - IBig::ONE;
            exp -= 1;
        } else {
            sig -= IBig::ONE;
        }
    }
    let floor = Repr::<B>::new(sig.clone(), exp);
    let next = Repr::<B>::new(sig.clone() + IBig::ONE, exp);
    assert!(
        exact_sqr(&floor) <= *sum && *sum < exact_sqr(&next),
        "{near:?} is not within an ulp"
    );
    if exact_sqr(&floor) == *sum {
        return Exact(floor);
    }
    // sqrt(sum) vs (floor + next)/2  <=>  4·sum vs (floor + next)²
    let twice_mid = exact(&floor) + exact(&next);
    let four_sum = sum * exact(&Repr::<B>::new(4.into(), 0));
    let adjust = R::round_low_part(&sig, Sign::Positive, || {
        four_sum.partial_cmp(&(&twice_mid * &twice_mid)).unwrap()
    });
    Inexact(Repr::new(sig + adjust, exp), adjust)
}

fn assert_sqrt_correct<R: ErrorBounds, const B: Word>(p: usize, x: &Repr<B>) {
    let got = Context::<R>::new(p).sqrt(x).unwrap().map(|v| v.into_repr());
    let want = sqrt_oracle::<R, B>(p, &exact(x), got.value_ref());
    assert_eq!(got, want, "sqrt({x:?}) at {p} digits");
}

fn assert_sqrt_correct_all_modes<const B: Word>(p: usize, x: &Repr<B>) {
    assert_sqrt_correct::<mode::Zero, B>(p, x);
    assert_sqrt_correct::<mode::Away, B>(p, x);
    assert_sqrt_correct::<mode::Down, B>(p, x);
    assert_sqrt_correct::<mode::Up, B>(p, x);
    assert_sqrt_correct::<mode::HalfEven, B>(p, x);
    assert_sqrt_correct::<mode::HalfAway, B>(p, x);
}

fn sweep_sqrt<const B: Word>() {
    for p in 1..=3 {
        for sig in 1..=700 {
            for exp in -2..=2 {
                assert_sqrt_correct_all_modes::<B>(p, &Repr::new(sig.into(), exp));
            }
        }
    }
}

/// `sqrt` is correctly rounded, with the right `Exact`/`Inexact` flag, in every base and mode:
/// exhaustively over small significands at tiny precisions (where exact roots, exact ties and
/// `p+1`-digit roots are dense).
#[test]
fn test_sqrt_exact_oracle_sweep() {
    sweep_sqrt::<2>();
    sweep_sqrt::<3>();
    sweep_sqrt::<4>();
    sweep_sqrt::<9>();
    sweep_sqrt::<10>();
    sweep_sqrt::<16>();
}

/// The same oracle at the standard binary precisions (and their decimal counterparts).
#[test]
fn test_sqrt_exact_oracle_standard_precisions() {
    let ten20_1 = IBig::from(10u8).pow(20) + IBig::ONE;
    for sig in [IBig::from(2), IBig::from(3), IBig::from(225), ten20_1] {
        for exp in [-41isize, -2, 0, 1, 40] {
            for p in [20usize, 50, 100, 500] {
                assert_sqrt_correct_all_modes::<2>(p, &Repr::new(sig.clone(), exp));
            }
            for p in [6usize, 15, 30, 150] {
                assert_sqrt_correct_all_modes::<10>(p, &Repr::new(sig.clone(), exp));
            }
        }
    }
}

/// Regression: an exact root that is a rounding tie in a non-power-of-two base did not terminate
/// (the certification loop could not accept a tie), e.g. `sqrt(225) = 15` at one decimal digit.
#[test]
fn test_sqrt_exact_tie_decimal() {
    // (precision, significand, exponent, expected significand, expected exponent)
    let cases: [(usize, i64, isize, i64, isize); 5] = [
        (1, 196, 0, 1, 1),      // sqrt = 14   -> 1e1 (not a tie)
        (1, 3025, -2, 6, 0),    // sqrt = 5.5  -> 6
        (1, 225, 0, 2, 1),      // sqrt = 15   -> 2e1
        (1, 625, -2, 2, 0),     // sqrt = 2.5  -> 2
        (2, 15625, -4, 12, -1), // sqrt = 1.25 -> 1.2
    ];
    for (p, s, e, ws, we) in cases {
        let x = Repr::<10>::new(s.into(), e);
        let got = Context::<mode::HalfEven>::new(p).sqrt(&x).unwrap();
        assert!(matches!(got, Inexact(..)), "sqrt({s}e{e}) at {p} digits is rounded");
        assert_eq!(got.value().repr(), &Repr::<10>::new(ws.into(), we), "sqrt({s}e{e})");
    }
    // ties away from zero
    let x = Repr::<10>::new(225.into(), 0);
    let got = Context::<mode::HalfAway>::new(1).sqrt(&x).unwrap();
    assert_eq!(got, Inexact(FBig::from_parts(2.into(), 1), AddOne));
}

/// Regression: a perfect square whose root fits the precision is `Exact` in every base. In a
/// base with a square factor the normalized significand can make the integer root one digit
/// wider than the precision with a zero last digit (`36 = 9·4¹`): that was reported `Inexact`
/// (`Up` returned 7) in base 4 and did not terminate in base 9.
#[test]
fn test_sqrt_perfect_square_in_square_factor_base() {
    fn check<R: ErrorBounds, const B: Word>(k: u32, p: usize) {
        let x = Repr::<B>::new((k * k).into(), 0);
        let got = Context::<R>::new(p).sqrt(&x).unwrap();
        let want = FBig::<R, B>::from_parts(k.into(), 0);
        assert_eq!(got, Exact(want), "sqrt({}) in base {B} at {p} digits", k * k);
    }
    check::<mode::Down, 4>(6, 2);
    check::<mode::Up, 4>(6, 2);
    check::<mode::HalfEven, 4>(6, 2);
    check::<mode::Down, 4>(5, 2);
    check::<mode::Up, 4>(6, 3);
    check::<mode::Down, 16>(12, 1);
    check::<mode::Down, 9>(21, 2);
    check::<mode::Up, 9>(21, 2);
    check::<mode::Zero, 9>(20, 2);
    check::<mode::Down, 10>(6, 1);
}

/// Regression: a root that merely *rounds* to a short value was reported `Exact` in a
/// non-power-of-two base (`sqrt(10^20 + 1)` at 4 digits, `sqrt(1 + 4·3^-34)` at 36 digits).
#[test]
fn test_sqrt_inexact_root_is_not_exact() {
    let x = Repr::<10>::new(IBig::from(10u8).pow(20) + IBig::ONE, 0);
    let got = Context::<mode::HalfEven>::new(4).sqrt(&x).unwrap();
    assert_eq!(got, Inexact(FBig::from_parts(1.into(), 10), NoOp));

    let x = Repr::<3>::new(IBig::from(3u8).pow(34) + IBig::from(4), -34);
    let got = Context::<mode::HalfEven>::new(36).sqrt(&x).unwrap();
    assert_eq!(
        got,
        Inexact(FBig::from_parts(IBig::from(3u8).pow(34) + IBig::from(2), -34), AddOne)
    );
}

/// Regression: an exponent next to `isize::MIN` overflowed the exponent arithmetic (a debug
/// panic, and a huge *positive* exponent in release builds).
#[test]
fn test_sqrt_extreme_exponents() {
    for e in [
        isize::MIN + 2,
        isize::MIN + 18,
        isize::MIN + 20,
        isize::MAX - 1,
        isize::MAX - 19,
    ] {
        let x = Repr::<10>::new(1.into(), e);
        let got = Context::<mode::Down>::new(1).sqrt(&x).unwrap();
        assert_eq!(got, Exact(FBig::from_parts(1.into(), e / 2)), "sqrt(1e{e})");
        let x = Repr::<2>::new(1.into(), e);
        let got = Context::<mode::Down>::new(53).sqrt(&x).unwrap();
        assert_eq!(got, Exact(FBig::from_parts(1.into(), e / 2)), "sqrt(2^{e})");
    }
    // an odd exponent next to the limits: sqrt(10·10^(e-1))
    for e in [isize::MIN + 3, isize::MAX - 2] {
        let x = Repr::<10>::new(1.into(), e);
        let got = Context::<mode::Down>::new(3).sqrt(&x).unwrap().value();
        assert_eq!(got.repr(), &Repr::<10>::new(316.into(), (e - 1) / 2 - 2), "sqrt(1e{e})");
    }
}

// ---------------------------------------------------------------------------------------------
// hypot, against the same exact oracle (the exact sum of squares is the radicand)
// ---------------------------------------------------------------------------------------------

fn assert_hypot_correct<R: ErrorBounds, const B: Word>(p: usize, a: &Repr<B>, b: &Repr<B>) {
    let got = Context::<R>::new(p)
        .hypot(a, b)
        .unwrap_or_else(|e| panic!("hypot({a:?}, {b:?}) at {p} digits: {e:?}"))
        .map(|v| v.into_repr());
    let sum = exact_sqr(a) + exact_sqr(b);
    let want = sqrt_oracle::<R, B>(p, &sum, got.value_ref());
    assert_eq!(got, want, "hypot({a:?}, {b:?}) at {p} digits");
}

fn assert_hypot_correct_all_modes<const B: Word>(p: usize, a: &Repr<B>, b: &Repr<B>) {
    assert_hypot_correct::<mode::Zero, B>(p, a, b);
    assert_hypot_correct::<mode::Away, B>(p, a, b);
    assert_hypot_correct::<mode::Down, B>(p, a, b);
    assert_hypot_correct::<mode::Up, B>(p, a, b);
    assert_hypot_correct::<mode::HalfEven, B>(p, a, b);
    assert_hypot_correct::<mode::HalfAway, B>(p, a, b);
}

fn sweep_hypot<const B: Word>() {
    for p in 1..=3 {
        for a in 1..=40 {
            for b in 1..=a {
                for exp in [-2isize, 0, 3] {
                    let (a, b) = (Repr::new(a.into(), 0), Repr::new(b.into(), exp));
                    assert_hypot_correct_all_modes::<B>(p, &a, &b);
                }
            }
        }
    }
}

/// `hypot` is correctly rounded with the right `Exact`/`Inexact` flag in every mode: over small
/// operands at tiny precisions (Pythagorean triples, exact ties and near-representable results
/// are dense there), and at the standard precisions.
#[test]
fn test_hypot_exact_oracle_sweep() {
    sweep_hypot::<2>();
    sweep_hypot::<3>();
    sweep_hypot::<10>();
}

#[test]
fn test_hypot_exact_oracle_standard_precisions() {
    let big = (IBig::ONE << 40) - IBig::ONE;
    for (a, b) in [
        (1i64, 1i64),
        (3, 4),
        (-5, 12),
        (1, 32),
        (1, 4095),
        (7, -4),
        (1, 1 << 40),
    ] {
        for exp in [-45isize, -20, 0, 7] {
            for p in [1usize, 20, 50, 53, 100, 500] {
                let (a, b) = (Repr::<2>::new(a.into(), 0), Repr::<2>::new(b.into(), exp));
                assert_hypot_correct_all_modes::<2>(p, &a, &b);
                assert_hypot_correct_all_modes::<2>(p, &a, &Repr::new(big.clone(), exp));
            }
            for p in [1usize, 6, 15, 30, 150] {
                let (a, b) = (Repr::<10>::new(a.into(), 0), Repr::<10>::new(b.into(), exp));
                assert_hypot_correct_all_modes::<10>(p, &a, &b);
            }
        }
    }
}

/// Regression: a result that merely rounds to a short value was flagged `Exact`
/// (`hypot(1, 2^40)` at 53 bits, `hypot(1, 32)` at 1 bit).
#[test]
fn test_hypot_inexact_result_is_not_exact() {
    let r2 = |v: u64| Repr::<2>::new(v.into(), 0);
    let ctx = Context::<mode::HalfEven>::new(53);
    let got = ctx.hypot(&r2(1 << 40), &r2(1)).unwrap();
    assert_eq!(got, Inexact(FBig::from_parts(1.into(), 40), NoOp));
    let got = Context::<mode::HalfAway>::new(53).hypot(&r2(1 << 40), &r2(1));
    assert!(matches!(got, Ok(Inexact(_, NoOp))));

    let ctx1 = Context::<mode::HalfEven>::new(1);
    assert_eq!(
        ctx1.hypot(&r2(1), &r2(32)).unwrap(),
        Inexact(FBig::from_parts(1.into(), 5), NoOp)
    );
    assert_eq!(
        ctx1.hypot(&r2(1), &r2(31)).unwrap(),
        Inexact(FBig::from_parts(1.into(), 5), AddOne)
    );
    assert_eq!(
        ctx1.hypot(&r2(1), &r2(4095)).unwrap(),
        Inexact(FBig::from_parts(1.into(), 12), AddOne)
    );
    // still exact where the root is exact
    assert_eq!(ctx.hypot(&r2(3), &r2(4)).unwrap(), Exact(FBig::from(5u8)));
}

/// Regression: `hypot(1, 2·3^-17)` at 35 base-3 digits under `Zero` returned `1 + 2 ulp`, above
/// the true value `√(1 + 4·3^-34) < 1 + 2·3^-34` (and likewise in base 10).
#[test]
fn test_hypot_toward_zero_does_not_round_up() {
    type F3 = FBig<mode::Zero, 3>;
    let a = F3::from_parts(1.into(), 0).with_precision(1).value();
    let b = F3::from_parts(2.into(), -17).with_precision(35).value();
    let expected = F3::from_parts(IBig::from(3u8).pow(34) + IBig::ONE, -34);
    assert_eq!(a.hypot(&b).repr(), expected.repr());
    let b = F3::from_parts(1.into(), -17).with_precision(35).value();
    assert_eq!(a.hypot(&b).repr(), &Repr::one());

    type F10 = FBig<mode::Zero, 10>;
    let a = F10::from_parts(1.into(), 0).with_precision(1).value();
    let b = F10::from_parts(2.into(), -18).with_precision(37).value();
    let expected = F10::from_parts(IBig::from(10u8).pow(36) + IBig::ONE, -36);
    assert_eq!(a.hypot(&b).repr(), expected.repr());
}

/// The root of a zero is exactly zero, so it needs no precision: `cbrt`, `nth_root` and
/// `hypot(0, 0)` on an unlimited-precision zero (`FBig::try_from(0.0)`, `DBig::ZERO`) used to
/// panic with "precision cannot be 0", while `sqrt` returned 0.
#[test]
fn test_roots_of_unlimited_precision_zero() {
    let zero: FBig = FBig::try_from(0.0f64).unwrap();
    assert_eq!(zero.precision(), 0);
    assert_eq!(zero.sqrt(), <FBig>::ZERO);
    assert_eq!(zero.cbrt(), <FBig>::ZERO);
    assert_eq!(zero.nth_root(5), <FBig>::ZERO);
    assert_eq!(zero.nth_root(2), <FBig>::ZERO);
    assert_eq!(zero.hypot(&zero), <FBig>::ZERO);
    assert!((-zero.clone()).cbrt().repr().is_neg_zero());
    assert!((-zero.clone()).hypot(&zero).repr().is_pos_zero());

    assert_eq!(DBig::ZERO.cbrt(), DBig::ZERO);
    assert_eq!(DBig::ZERO.nth_root(1), DBig::ZERO);
    assert_eq!(DBig::ZERO.hypot(&DBig::ZERO), DBig::ZERO);
    // the first root is the value itself, exact at any precision
    assert_eq!(DBig::ONE.nth_root(1), DBig::ONE);

    // a limited-precision operand still decides the precision of the result
    let three = dbig!(3).with_precision(10).value();
    assert_eq!(three.hypot(&DBig::ZERO), three);
    assert_eq!(three.hypot(&DBig::ZERO).precision(), 10);
}

/// A non-trivial root still needs a precision.
#[test]
#[should_panic]
fn test_nth_root_unlimited_precision() {
    let _ = dbig!(2).with_precision(0).value().nth_root(5);
}

#[test]
#[should_panic]
fn test_hypot_unlimited_precision() {
    let one = DBig::ONE;
    let _ = one.hypot(&one);
}

/// `hypot` does not depend on where the operands sit in the exponent range: scaling both by the
/// same power of the base scales the result. Regressions: operands near `2^(isize::MIN/2)` gave
/// `Err(Underflow)`, 0 or a huge positive exponent (a debug overflow panic); operands near
/// `2^(isize::MAX/2)` with a significand wider than the precision gave `Err(Overflow)`.
#[test]
fn test_hypot_is_exponent_invariant() {
    fn check<R: ErrorBounds, const B: Word>(p: usize, a: i64, b: i64, gap: isize, e: isize) {
        let ctx = Context::<R>::new(p);
        let at = |e: isize| {
            ctx.hypot(&Repr::<B>::new(a.into(), e), &Repr::<B>::new(b.into(), e + gap))
                .unwrap_or_else(|err| panic!("hypot({a}·B^{e}, {b}·B^({e}+{gap})): {err:?}"))
        };
        let want = at(0).map(|v| {
            let r = v.into_repr();
            FBig::<R, B>::from_parts(r.significand().clone(), r.exponent() + e)
        });
        assert_eq!(at(e), want, "hypot({a}·B^{e}, {b}·B^({e}+{gap})) at {p} digits");
    }
    let exponents = [
        isize::MIN / 2 - 70,
        isize::MIN / 2,
        isize::MIN / 2 + 64,
        isize::MIN / 4,
        isize::MAX / 2 - 3,
        isize::MAX / 2,
        isize::MAX / 2 + 10,
        isize::MAX - 2000,
        isize::MIN + 2000,
    ];
    for e in exponents {
        for (a, b, gap) in [
            (1i64, 1i64, 0isize),
            (1, 5, 0),
            (1, 47, 0),
            (1, 1023, 0),
            (1, (1 << 40) - 1, 0),
            (3, 4, 0),
            (7, 4, 0),
            (7, 5, -30),
            (1, 1, 300),
        ] {
            for p in [1usize, 2, 10, 20, 50, 53, 100, 500] {
                check::<mode::HalfEven, 2>(p, a, b, gap, e);
                check::<mode::Down, 2>(p, a, b, gap, e);
                check::<mode::Up, 2>(p, a, b, gap, e);
                check::<mode::HalfAway, 10>(p, a, b, gap, e);
                check::<mode::Zero, 10>(p, a, b, gap, e);
            }
        }
    }

    // the reported shapes, spelled out
    let ctx = Context::<mode::HalfEven>::new(53);
    let one = Repr::<2>::one();
    let sqrt2 = ctx.hypot(&one, &one).unwrap().value();
    for e in [isize::MIN / 2, isize::MIN / 2 + 64] {
        let tiny = Repr::<2>::new(1.into(), e);
        let r = ctx.hypot(&one, &tiny).unwrap();
        assert_eq!(r, Inexact(FBig::ONE, NoOp), "hypot(1, 2^{e})");
        let r = ctx.hypot(&tiny, &tiny).unwrap().value();
        let expect =
            Repr::<2>::new(sqrt2.repr().significand().clone(), sqrt2.repr().exponent() + e);
        assert_eq!(r.repr(), &expect, "hypot(2^{e}, 2^{e})");
        // through the convenience API (this returned 0)
        let (a, b) = (
            FBig::<mode::HalfEven, 2>::ONE,
            FBig::<mode::HalfEven, 2>::from_parts(1.into(), e),
        );
        assert_eq!(a.with_precision(53).value().hypot(&b), FBig::<mode::HalfEven, 2>::ONE);
    }
}

/// At the very edge of the exponent range the result itself can be out of range, and only then
/// is a range error reported.
#[test]
fn test_hypot_range_errors() {
    use dashu_float::FpError;
    let ctx = Context::<mode::HalfEven>::new(10);
    // 2^(MAX-1)·√2 has a 10-bit significand at exponent MAX-10: representable
    let big = Repr::<2>::new(1.into(), isize::MAX - 1);
    assert!(ctx.hypot(&big, &big).is_ok());
    // hypot(x, tiny) = x (inexact) for the largest power of two
    let r = ctx.hypot(&big, &Repr::one()).unwrap();
    assert_eq!(r, Inexact(FBig::from_parts(1.into(), isize::MAX - 1), NoOp));
    // a 10-bit significand times 2^(MAX-1) is beyond the range
    let wide = Repr::<2>::new(1023.into(), isize::MAX - 1);
    assert_eq!(ctx.hypot(&wide, &wide), Err(FpError::Overflow(Sign::Positive)));
    // 2^(MIN+1)·√2 needs a 10-bit significand below the smallest exponent
    let small = Repr::<2>::new(1.into(), isize::MIN + 1);
    assert_eq!(ctx.hypot(&small, &small), Err(FpError::Underflow(Sign::Positive)));
    assert_eq!(
        ctx.hypot(&small, &Repr::zero()),
        Ok(Exact(FBig::from_parts(1.into(), isize::MIN + 1)))
    );
}

/// A negligible second operand costs one rounding of the first, in every mode. Regression: under
/// `Up`/`Down` the cost grew with the exponent gap (`hypot(1, 10^10000000)` took two minutes).
#[test]
fn test_hypot_negligible_operand() {
    let ten37 = IBig::from(10u8).pow(37);
    for gap in [40isize, 10_000, 10_000_000, isize::MAX / 2, isize::MAX - 2] {
        let small = Repr::<10>::one();
        let large = Repr::<10>::new(1.into(), gap);
        for (a, b) in [(&small, &large), (&large, &small)] {
            let up = Context::<mode::Up>::new(38).hypot(a, b).unwrap();
            assert_eq!(up, Inexact(FBig::from_parts(ten37.clone() + IBig::ONE, gap - 37), AddOne));
            let away = Context::<mode::Away>::new(38).hypot(a, b).unwrap();
            assert_eq!(
                away,
                Inexact(FBig::from_parts(ten37.clone() + IBig::ONE, gap - 37), AddOne)
            );
            let down = Context::<mode::Down>::new(38).hypot(a, b).unwrap();
            assert_eq!(down, Inexact(FBig::from_parts(1.into(), gap), NoOp));
            let zero = Context::<mode::Zero>::new(38).hypot(a, b).unwrap();
            assert_eq!(zero, Inexact(FBig::from_parts(1.into(), gap), NoOp));
            let even = Context::<mode::HalfEven>::new(38).hypot(a, b).unwrap();
            assert_eq!(even, Inexact(FBig::from_parts(1.into(), gap), NoOp));
        }
    }
    // a first operand wider than the precision: the sticky part breaks an exact tie upward
    let a = Repr::<10>::new(125.into(), 0);
    let b = Repr::<10>::new(1.into(), -1000);
    let r = Context::<mode::HalfEven>::new(2).hypot(&a, &b).unwrap();
    assert_eq!(r, Inexact(FBig::from_parts(13.into(), 1), AddOne));
    // just outside the shortcut: the gap is too small to be negligible at the precision
    for p in [1usize, 20, 50, 100, 500] {
        for gap in [
            p as isize / 2 - 1,
            p as isize / 2,
            p as isize / 2 + 1,
            p as isize / 2 + 3,
            p as isize,
        ] {
            let (a, b) = (Repr::<2>::new(3.into(), gap), Repr::<2>::new(5.into(), 0));
            assert_hypot_correct_all_modes::<2>(p, &a, &b);
            let (a, b) = (Repr::<10>::new(3.into(), gap), Repr::<10>::new(5.into(), 0));
            assert_hypot_correct_all_modes::<10>(p, &a, &b);
        }
    }
}

/// `nth_root` does not depend on where the operand sits in the exponent range: moving the
/// exponent by a multiple of `n` moves the root's exponent by that multiple. Regression: for an
/// exponent next to `isize::MIN`/`isize::MAX` the alignment arithmetic left the `isize` range —
/// an overflow panic in debug builds, and in release builds a root with the wrong exponent
/// (`cbrt(8·10^(isize::MIN+2))` came back as `2·10^3074457345618258603`) or the wrong
/// significand (`cbrt(27·10^(isize::MAX−1))`).
#[test]
fn test_nth_root_extreme_exponents() {
    // the reported calls: both cube roots are exact
    let ctx = Context::<mode::HalfEven>::new(10);
    let e = isize::MIN + 2;
    let got = ctx.cbrt(&Repr::<10>::new(8.into(), e)).unwrap();
    assert_eq!(got, Exact(FBig::from_parts(2.into(), e / 3)), "cbrt(8e{e})");
    let e = isize::MAX - 1;
    let got = ctx.cbrt(&Repr::<10>::new(27.into(), e)).unwrap();
    assert_eq!(got, Exact(FBig::from_parts(3.into(), e / 3)), "cbrt(27e{e})");

    fn check<R: Round, const B: Word>(p: usize, n: usize, sig: i32, e: isize) {
        let ctx = Context::<R>::new(p);
        // the same significand with the exponent reduced into `0..n`
        let near = e.rem_euclid(n as isize);
        let moved = (e as i128 - near as i128) / n as i128;
        let got = ctx.nth_root(n, &Repr::<B>::new(sig.into(), e)).unwrap();
        let want = ctx.nth_root(n, &Repr::<B>::new(sig.into(), near)).unwrap();
        let want = want.map(|v| {
            let (signif, exp) = v.into_repr().into_parts();
            FBig::from_parts(signif, (exp as i128 + moved) as isize)
        });
        assert_eq!(got, want, "nth_root({n}, {sig}·{B}^{e}) at precision {p}");
    }
    for n in [3usize, 4, 5, 7] {
        for d in 1..=(2 * n as isize + 1) {
            for e in [isize::MIN + d, isize::MAX - d] {
                for p in [20, 50, 100, 500] {
                    // 3^n has an exact root at an aligned exponent, 7 never has one
                    check::<mode::HalfEven, 2>(p, n, 3i32.pow(n as u32), e);
                    check::<mode::Down, 2>(p, n, 7, e);
                    check::<mode::Up, 2>(p, n, 7, e);
                }
                check::<mode::HalfAway, 10>(10, n, 3i32.pow(n as u32), e);
                check::<mode::Zero, 10>(10, n, 7, e);
                check::<mode::Away, 3>(10, n, 7, e);
                // a negative operand has an odd root
                if n % 2 == 1 {
                    check::<mode::Down, 10>(10, n, -7, e);
                    check::<mode::HalfEven, 2>(50, n, -(3i32.pow(n as u32)), e);
                }
            }
        }
    }
}
