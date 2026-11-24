use core::str::FromStr;

use dashu_float::ops::Abs;
use dashu_float::round::mode::{Away, Down, HalfAway, HalfEven, Up, Zero};
use dashu_float::{round, Context, DBig, FBig, Repr, Word};
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
            assert_eq!(
                ctx.atan2(y, x, None),
                Ok(want(num, shift)),
                "atan2({y:?}, {x:?}), p = {p}"
            );
        }
        // atan2(±finite, +inf) = ±0, exactly
        assert!(ctx.atan2(&one, &inf, None).unwrap().value().repr().is_pos_zero());
        assert!(ctx.atan2(&mone, &inf, None).unwrap().value().repr().is_neg_zero());
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
