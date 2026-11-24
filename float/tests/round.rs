use core::str::FromStr;

use dashu_base::{Approximation::*, ParseError};
use dashu_float::{round::Rounding::*, DBig};

mod helper_macros;

#[test]
#[rustfmt::skip::macros(fbig)]
fn test_ceil_floor() {
    assert_eq!(fbig!(0x0).ceil(), fbig!(0x0));
    assert_eq!(fbig!(0x1p1).ceil(), fbig!(0x1p1));
    assert_eq!(fbig!(0x1).ceil(), fbig!(0x1));
    assert_eq!(fbig!(0x1p-1).ceil(), fbig!(0x1));
    assert_eq!(fbig!(-0x1p1).ceil(), fbig!(-0x1p1));
    assert_eq!(fbig!(-0x1).ceil(), fbig!(-0x1));
    assert_eq!(fbig!(-0x1p-1).ceil(), fbig!(0x0));

    assert_eq!(fbig!(0x0).floor(), fbig!(0x0));
    assert_eq!(fbig!(0x1p1).floor(), fbig!(0x1p1));
    assert_eq!(fbig!(0x1).floor(), fbig!(0x1));
    assert_eq!(fbig!(0x1p-1).floor(), fbig!(0x0));
    assert_eq!(fbig!(-0x1p1).floor(), fbig!(-0x1p1));
    assert_eq!(fbig!(-0x1).floor(), fbig!(-0x1));
    assert_eq!(fbig!(-0x1p-1).floor(), fbig!(-0x1));

    assert_eq!(dbig!(0).ceil(), dbig!(0));
    assert_eq!(dbig!(1e1).ceil(), dbig!(1e1));
    assert_eq!(dbig!(1).ceil(), dbig!(1));
    assert_eq!(dbig!(1e-1).ceil(), dbig!(1));
    assert_eq!(dbig!(-1e1).ceil(), dbig!(-1e1));
    assert_eq!(dbig!(-1).ceil(), dbig!(-1));
    assert_eq!(dbig!(-1e-1).ceil(), dbig!(0));

    assert_eq!(dbig!(0).floor(), dbig!(0));
    assert_eq!(dbig!(1e1).floor(), dbig!(1e1));
    assert_eq!(dbig!(1).floor(), dbig!(1));
    assert_eq!(dbig!(1e-1).floor(), dbig!(0));
    assert_eq!(dbig!(-1e1).floor(), dbig!(-1e1));
    assert_eq!(dbig!(-1).floor(), dbig!(-1));
    assert_eq!(dbig!(-1e-1).floor(), dbig!(-1));
}

#[test]
#[rustfmt::skip::macros(fbig)]
fn test_trunc_fract() {
    // binary
    assert_eq!(fbig!(0x0).trunc(), fbig!(0x0));
    assert_eq!(fbig!(0x12p4).trunc(), fbig!(0x12p4));
    assert_eq!(fbig!(0x12).trunc(), fbig!(0x12));
    assert_eq!(fbig!(0x12p-4).trunc(), fbig!(0x1));
    assert_eq!(fbig!(0x12p-8).trunc(), fbig!(0x0));
    assert_eq!(fbig!(0x12p-12).trunc(), fbig!(0x0));
    assert_eq!(fbig!(-0x12p4).trunc(), fbig!(-0x12p4));
    assert_eq!(fbig!(-0x12).trunc(), fbig!(-0x12));
    assert_eq!(fbig!(-0x12p-4).trunc(), fbig!(-0x1));
    assert_eq!(fbig!(-0x12p-8).trunc(), fbig!(-0x0));
    assert_eq!(fbig!(-0x12p-12).trunc(), fbig!(0x0));

    assert_eq!(fbig!(0x0).fract(), fbig!(0x0));
    assert_eq!(fbig!(0x12p4).fract(), fbig!(0x0));
    assert_eq!(fbig!(0x12).fract(), fbig!(0x0));
    assert_eq!(fbig!(0x12p-4).fract(), fbig!(0x2p-4));
    assert_eq!(fbig!(0x12p-8).fract(), fbig!(0x12p-8));
    assert_eq!(fbig!(0x12p-12).fract(), fbig!(0x12p-12));
    assert_eq!(fbig!(-0x12p4).fract(), fbig!(0x0));
    assert_eq!(fbig!(-0x12).fract(), fbig!(0x0));
    assert_eq!(fbig!(-0x12p-4).fract(), fbig!(-0x2p-4));
    assert_eq!(fbig!(-0x12p-8).fract(), fbig!(-0x12p-8));
    assert_eq!(fbig!(-0x12p-12).fract(), fbig!(-0x12p-12));

    assert_eq!(fbig!(0x0).split_at_point(), (fbig!(0x0), fbig!(0x0)));
    assert_eq!(fbig!(0x12p4).split_at_point(), (fbig!(0x12p4), fbig!(0x0)));
    assert_eq!(fbig!(0x12).split_at_point(), (fbig!(0x12), fbig!(0x0)));
    assert_eq!(fbig!(0x12p-4).split_at_point(), (fbig!(0x1), fbig!(0x2p-4)));
    assert_eq!(fbig!(0x12p-8).split_at_point(), (fbig!(0x0), fbig!(0x12p-8)));
    assert_eq!(fbig!(0x12p-12).split_at_point(), (fbig!(0x0), fbig!(0x12p-12)));
    assert_eq!(fbig!(-0x12p4).split_at_point(), (fbig!(-0x12p4), fbig!(0x0)));
    assert_eq!(fbig!(-0x12).split_at_point(), (fbig!(-0x12), fbig!(0x0)));
    assert_eq!(fbig!(-0x12p-4).split_at_point(), (fbig!(-0x1), fbig!(-0x2p-4)));
    assert_eq!(fbig!(-0x12p-8).split_at_point(), (fbig!(0x0), fbig!(-0x12p-8)));
    assert_eq!(fbig!(-0x12p-12).split_at_point(), (fbig!(0x0), fbig!(-0x12p-12)));

    // decimal
    assert_eq!(dbig!(0).trunc(), dbig!(0));
    assert_eq!(dbig!(12e1).trunc(), dbig!(12e1));
    assert_eq!(dbig!(12).trunc(), dbig!(12));
    assert_eq!(dbig!(12e-1).trunc(), dbig!(1));
    assert_eq!(dbig!(12e-2).trunc(), dbig!(0));
    assert_eq!(dbig!(12e-3).trunc(), dbig!(0));
    assert_eq!(dbig!(-12e1).trunc(), dbig!(-12e1));
    assert_eq!(dbig!(-12).trunc(), dbig!(-12));
    assert_eq!(dbig!(-12e-1).trunc(), dbig!(-1));
    assert_eq!(dbig!(-12e-2).trunc(), dbig!(-0));
    assert_eq!(dbig!(-12e-3).trunc(), dbig!(0));

    assert_eq!(dbig!(0).fract(), dbig!(0));
    assert_eq!(dbig!(12e1).fract(), dbig!(0));
    assert_eq!(dbig!(12).fract(), dbig!(0));
    assert_eq!(dbig!(12e-1).fract(), dbig!(2e-1));
    assert_eq!(dbig!(12e-2).fract(), dbig!(12e-2));
    assert_eq!(dbig!(12e-3).fract(), dbig!(12e-3));
    assert_eq!(dbig!(-12e1).fract(), dbig!(0));
    assert_eq!(dbig!(-12).fract(), dbig!(0));
    assert_eq!(dbig!(-12e-1).fract(), dbig!(-2e-1));
    assert_eq!(dbig!(-12e-2).fract(), dbig!(-12e-2));
    assert_eq!(dbig!(-12e-3).fract(), dbig!(-12e-3));

    assert_eq!(dbig!(0).split_at_point(), (dbig!(0), dbig!(0)));
    assert_eq!(dbig!(12e1).split_at_point(), (dbig!(12e1), dbig!(0)));
    assert_eq!(dbig!(12).split_at_point(), (dbig!(12), dbig!(0)));
    assert_eq!(dbig!(12e-1).split_at_point(), (dbig!(1), dbig!(2e-1)));
    assert_eq!(dbig!(12e-2).split_at_point(), (dbig!(0), dbig!(12e-2)));
    assert_eq!(dbig!(12e-3).split_at_point(), (dbig!(0), dbig!(12e-3)));
    assert_eq!(dbig!(-12e1).split_at_point(), (dbig!(-12e1), dbig!(0)));
    assert_eq!(dbig!(-12).split_at_point(), (dbig!(-12), dbig!(0)));
    assert_eq!(dbig!(-12e-1).split_at_point(), (dbig!(-1), dbig!(-2e-1)));
    assert_eq!(dbig!(-12e-2).split_at_point(), (dbig!(0), dbig!(-12e-2)));
    assert_eq!(dbig!(-12e-3).split_at_point(), (dbig!(0), dbig!(-12e-3)));

    // precision test
    assert_eq!(dbig!(12).trunc().precision(), 2);
    assert_eq!(dbig!(12).fract().precision(), 0);
    assert_eq!(dbig!(12e-1).trunc().precision(), 1);
    assert_eq!(dbig!(12e-1).fract().precision(), 1);
    assert_eq!(dbig!(12e-2).trunc().precision(), 0);
    assert_eq!(dbig!(12e-2).fract().precision(), 2);
}

/// Numbers with |self| < 1 can be stored with a single significand digit and a
/// negative exponent (e.g. 0.009 = 9e-3). Rounding must use `-exponent` as the
/// fractional scale, not `context.precision`; otherwise 9 / 10^1 = 0.9 rounds up.
#[test]
fn test_round_smaller_than_one_uses_exponent_scale() -> Result<(), ParseError> {
    let a = DBig::from_str("0.009")?.with_precision(1).unwrap();
    assert_eq!(a.round(), DBig::ZERO);

    let b = DBig::from_str("0.09")?.with_precision(1).unwrap();
    assert_eq!(b.round(), DBig::ZERO);

    let c = DBig::from_str("1e-5")?.with_precision(3).unwrap();
    assert_eq!(c.round(), DBig::ZERO);
    assert_eq!(c.to_int(), Inexact(ibig!(0), NoOp));

    Ok(())
}

/// `.fract()` must not inflate context precision to `-exponent` when trailing
/// zeros were normalized away from the significand.
#[test]
fn test_fract_preserves_context_precision() -> Result<(), ParseError> {
    let a = DBig::from_str("1e-5")?.with_precision(3).unwrap();
    assert_eq!(a.fract(), a);
    assert_eq!(a.fract().precision(), 3);

    let b = DBig::from_str("9e-5")?.with_precision(1).unwrap();
    assert_eq!(b.fract(), b);
    assert_eq!(b.fract().precision(), 1);

    Ok(())
}

#[test]
#[should_panic]
fn test_floor_inf() {
    let _ = DBig::INFINITY.floor();
}

#[test]
#[should_panic]
fn test_ceil_inf() {
    let _ = DBig::INFINITY.ceil();
}

#[test]
#[should_panic]
fn test_trunc_inf() {
    let _ = DBig::INFINITY.trunc();
}

#[test]
#[should_panic]
fn test_fract_inf() {
    let _ = DBig::INFINITY.fract();
}

#[test]
fn test_quantize_decimal() -> Result<(), ParseError> {
    let a = DBig::from_str("1.234")?; // precision 4, exponent -3

    // coarser quantum: rounding occurs (always inexact for a normalized input)
    let q = a.quantize(-2);
    assert_eq!(q, Inexact(DBig::from_str("1.23")?, NoOp));
    assert_eq!(q.value().precision(), 3);

    assert_eq!(a.quantize(0), Inexact(DBig::from_str("1")?, NoOp));
    assert_eq!(a.quantize(0).value().precision(), 1);

    // tie rounds away from zero under HalfAway (the DBig default)
    let b = DBig::from_str("7.325")?;
    assert_eq!(b.quantize(-2), Inexact(DBig::from_str("7.33")?, AddOne));
    assert_eq!(b.quantize(-2).value().precision(), 3);
    assert_eq!(b.quantize(0), Inexact(DBig::from_str("7")?, NoOp));

    // round up across a digit boundary
    assert_eq!(DBig::from_str("999")?.quantize(3), Inexact(DBig::from_str("1000")?, AddOne));
    assert_eq!(DBig::from_str("999")?.quantize(3).value().precision(), 1);
    assert_eq!(DBig::from_str("9.9")?.quantize(0), Inexact(DBig::from_str("10")?, AddOne));

    // round toward zero (result is zero -> unlimited precision, like `round()`)
    assert_eq!(DBig::from_str("10")?.quantize(2), Inexact(DBig::ZERO, NoOp));
    assert_eq!(DBig::from_str("0.4")?.quantize(0), Inexact(DBig::ZERO, NoOp));

    // finer-or-equal quantum: exact, value unchanged, precision increases
    assert_eq!(a.quantize(-3), Exact(DBig::from_str("1.234")?));
    assert_eq!(a.quantize(-3).value().precision(), 4);
    assert_eq!(a.quantize(-5), Exact(DBig::from_str("1.234")?));
    assert_eq!(a.quantize(-5).value().precision(), 6);
    assert_eq!(a.quantize(-10), Exact(DBig::from_str("1.234")?));
    assert_eq!(a.quantize(-10).value().precision(), 11);

    // zero input is exact
    assert_eq!(DBig::ZERO.quantize(3), Exact(DBig::ZERO));

    // negatives: odd/even behavior mirrors the sign
    let n = DBig::from_str("-1.234")?;
    assert_eq!(n.quantize(-1), Inexact(DBig::from_str("-1.2")?, NoOp));
    assert_eq!(n.quantize(-1).value().precision(), 2);

    Ok(())
}

#[test]
#[rustfmt::skip::macros(fbig)]
fn test_quantize_binary() {
    // binary FBig defaults to Zero (toward-zero) rounding.
    // 0x3 (3, sig 3 exp 0, 2 bits): an equal/finer quantum is exact.
    assert_eq!(fbig!(0x3).quantize(0), Exact(fbig!(0x3)));
    assert_eq!(fbig!(0x3).quantize(0).value().precision(), 2);
    assert_eq!(fbig!(0x3).quantize(-2), Exact(fbig!(0x3)));
    assert_eq!(fbig!(0x3).quantize(-2).value().precision(), 4);

    // 0x3p-2 (0.75) quantized toward zero:
    //   to exp -1 (nearest 0.5) -> 0.5 ; to exp 0 (integer) -> 0
    assert_eq!(fbig!(0x3p-2).quantize(-1), Inexact(fbig!(0x1p-1), NoOp));
    assert_eq!(fbig!(0x3p-2).quantize(-1).value().precision(), 1);
    assert_eq!(fbig!(0x3p-2).quantize(0), Inexact(fbig!(0x0), NoOp));
}

/// `quantize` must honor the type-level rounding mode `R`.
#[test]
fn test_quantize_modes() -> Result<(), ParseError> {
    use dashu_float::{
        round::mode::{Down, Up},
        FBig,
    };

    let halfaway = DBig::from_str("7.325")?; // DBig == FBig<HalfAway, 10>
    let down: FBig<Down, 10> = FBig::<Down, 10>::from_str("7.325")?;
    let up: FBig<Up, 10> = FBig::<Up, 10>::from_str("7.325")?;

    // 7.325 is exactly halfway between 7.32 and 7.33
    assert_eq!(halfaway.quantize(-2), Inexact(DBig::from_str("7.33")?, AddOne));
    assert_eq!(down.quantize(-2), Inexact(FBig::<Down, 10>::from_str("7.32")?, NoOp));
    assert_eq!(up.quantize(-2), Inexact(FBig::<Up, 10>::from_str("7.33")?, AddOne));
    Ok(())
}

/// For any nonzero result, `quantize(exp).ulp()` must equal `BASE^exp` exactly
/// (this is how the result precision is defined).
#[test]
fn test_quantize_ulp_invariant() -> Result<(), ParseError> {
    let inputs = [
        "1.234",
        "7.325",
        "999",
        "0.5",
        "12345.6789",
        "0.001",
        "1e20",
    ];
    for s in &inputs {
        let x = DBig::from_str(s)?;
        for &exp in &[-10isize, -5, -2, -1, 0, 1, 2, 3, 5, 10] {
            let q = x.quantize(exp).value();
            // skip results that rounded to zero (they have unlimited precision)
            if q.precision() != 0 {
                let quantum = DBig::from_str(&format!("1e{exp}"))?;
                assert_eq!(q.ulp(), quantum, "ulp mismatch for {s}.quantize({exp})");
            }
        }
    }
    Ok(())
}

#[test]
#[should_panic]
fn test_quantize_inf() {
    let _ = DBig::INFINITY.quantize(0);
}

// ---------------------------------------------------------------------------
// Regressions for the rounding-operation reports of the Hegel triage
// (see triage/float-arith.md).
// ---------------------------------------------------------------------------

mod triage {
    use dashu_base::{Approximation, Sign};
    use dashu_float::{
        round::{mode, Round, Rounding},
        Context, DBig, FBig, Repr, Word,
    };
    use dashu_int::IBig;
    use std::panic::catch_unwind;

    fn fbig<R: Round, const B: Word>(sig: i64, exp: isize, precision: usize) -> FBig<R, B> {
        FBig::from_repr(Repr::new(sig.into(), exp), Context::new(0))
            .with_precision(precision)
            .value()
    }

    /// Assert two floats are identical: same value, same sign of zero, same precision.
    #[track_caller]
    fn assert_same<R: Round, const B: Word>(a: &FBig<R, B>, b: &FBig<R, B>) {
        assert_eq!(a.repr(), b.repr());
        assert_eq!(a.sign(), b.sign(), "{a:?} vs {b:?}");
        assert_eq!(a.precision(), b.precision(), "{a:?} vs {b:?}");
    }

    /// (significand, exponent) samples around the radix point: integers, values below one
    /// that the cheap magnitude estimate does and does not recognize as such, and mixed ones.
    const SAMPLES: [(i64, isize); 16] = [
        (0, 0),
        (1, 0),
        (12, 0),
        (12, 3),
        (1, -1),
        (5, -1),
        (3, -2),
        (99, -2),
        (5, -3),
        (1, -5),
        (15, -1),
        (125, -1),
        (1234, -3),
        (123456789, -4),
        (1, -70),
        (987654321, -9),
    ];

    /// Rounding a negative number with a zero integer part yields `-0` from every
    /// round-to-integer operation (IEEE 754: the sign of the operand is kept): `ceil` used to
    /// return `+0` where `trunc` and `round` returned `-0`.
    #[test]
    fn test_round_to_integer_keeps_sign_of_zero() {
        fn check<R: Round, const B: Word>() {
            for precision in [20usize, 50, 100, 500] {
                for (sig, exp) in SAMPLES {
                    for sign in [1, -1] {
                        let x = fbig::<R, B>(sign * sig, exp, precision);
                        let x = if sig == 0 && sign < 0 { -x } else { x };
                        for (name, y) in [
                            ("trunc", x.trunc()),
                            ("ceil", x.ceil()),
                            ("floor", x.floor()),
                            ("round", x.round()),
                        ] {
                            assert!(y.repr().exponent() >= 0 || y.repr().is_neg_zero());
                            if y.repr().significand().is_zero() {
                                assert_eq!(y.sign(), x.sign(), "{name}({x:?}) = {y:?}");
                            }
                        }
                        // the integer bracket is consistent
                        assert!(x.floor() <= x && x <= x.ceil());
                        assert!(x.ceil() - x.floor() <= FBig::<R, B>::ONE);
                    }
                }
            }
        }
        check::<mode::Zero, 2>();
        check::<mode::HalfAway, 10>();
        check::<mode::Down, 10>();
        check::<mode::Up, 2>();

        use core::str::FromStr;
        for s in ["-0.1", "-0.5", "-0.05", "-0.99"] {
            let x = DBig::from_str(s).unwrap();
            assert!(x.ceil().repr().is_neg_zero(), "ceil({s})");
            assert!(x.trunc().repr().is_neg_zero(), "trunc({s})");
        }
        assert_eq!(DBig::from_str("-1.5").unwrap().ceil(), DBig::from(-1));
        assert!(DBig::from_str("0.5").unwrap().floor().repr().is_pos_zero());
        assert!(DBig::from_str("-0").unwrap().ceil().repr().is_neg_zero());
    }

    /// `split_at_point()` is `(trunc(), fract())` — value, sign of zero and precision — and,
    /// like them, rejects the infinities. Its zero parts used to be `+0` for negative inputs,
    /// and `+inf` came back as `(inf, 0)`.
    #[test]
    fn test_split_at_point_matches_trunc_fract() {
        fn check<R: Round, const B: Word>() {
            for precision in [20usize, 50, 100, 500] {
                for (sig, exp) in SAMPLES {
                    for sign in [1, -1] {
                        let x = fbig::<R, B>(sign * sig, exp, precision);
                        let x = if sig == 0 && sign < 0 { -x } else { x };
                        let (trunc, fract) = x.clone().split_at_point();
                        assert_same(&trunc, &x.trunc());
                        assert_same(&fract, &x.fract());
                        // both parts carry the sign of the input
                        assert_eq!(trunc.sign(), x.sign());
                        assert_eq!(fract.sign(), x.sign());
                        // and they add back to it exactly
                        let sum = Context::<R>::new(0)
                            .add(trunc.repr(), fract.repr())
                            .unwrap()
                            .value();
                        assert_eq!(sum.repr(), x.repr());
                    }
                }
            }
        }
        check::<mode::Zero, 2>();
        check::<mode::HalfAway, 10>();
        check::<mode::HalfEven, 3>();

        for x in [DBig::INFINITY, DBig::NEG_INFINITY] {
            assert!(catch_unwind(|| x.trunc()).is_err());
            assert!(catch_unwind(|| x.fract()).is_err());
            let y = x.clone();
            assert!(catch_unwind(move || y.split_at_point()).is_err());
        }
    }

    /// A number below one in magnitude is its own fractional part, precision included,
    /// whether or not the cheap magnitude estimate recognizes it as below one. `fract()` used
    /// to reset the precision to the number of fractional digits for values in `[0.01, 1)`.
    #[test]
    fn test_fract_below_one_is_identity() {
        fn check<R: Round, const B: Word>() {
            for precision in [1usize, 2, 3, 5, 20, 50, 100, 500] {
                for exp in [-1isize, -2, -3, -5, -40] {
                    for sig in [1i64, -1] {
                        let x = fbig::<R, B>(sig, exp, precision);
                        assert_same(&x.fract(), &x);
                        let t = x.trunc();
                        assert!(t.repr().significand().is_zero());
                        assert_eq!((t.sign(), t.precision()), (x.sign(), 0));
                    }
                }
            }
        }
        check::<mode::HalfAway, 10>();
        check::<mode::Zero, 2>();

        // a nonzero integral part still leaves the number of fractional digits as precision
        let x = fbig::<mode::HalfAway, 10>(1234, -3, 7);
        assert_eq!(x.fract().precision(), 3);
        assert_eq!(x.trunc().precision(), 4);
    }

    /// `quantize` keeps the sign of a negative value that rounds to zero (like `round()` and
    /// IEEE 754 `quantize`), and a zero result is never mistaken for a special value whatever
    /// the quantum exponent is (`-1` is the exponent of the `-0` encoding).
    #[test]
    fn test_quantize_zero_result_sign() {
        fn check<R: Round, const B: Word>(rounds_to_zero: bool) {
            for precision in [20usize, 50, 100, 500] {
                for exp in [-1isize, 0, 1, 5, isize::MAX - 1, isize::MAX] {
                    for sign in [1i64, -1] {
                        // far below half a quantum
                        let x = fbig::<R, B>(sign, exp.saturating_sub(40), precision);
                        let q = x.quantize(exp);
                        let (value, rounding) = match q {
                            Approximation::Inexact(v, r) => (v, r),
                            Approximation::Exact(v) => panic!("exact: {v:?}"),
                        };
                        if rounds_to_zero {
                            assert!(value.repr().significand().is_zero());
                            assert_eq!(value.sign(), x.sign(), "{x:?}.quantize({exp})");
                            assert_eq!(value.precision(), 0);
                            assert_eq!(rounding, Rounding::NoOp);
                        } else {
                            // rounded away from zero to one quantum
                            assert_eq!(value.repr(), &Repr::new(sign.into(), exp));
                            assert_eq!(value.precision(), 1);
                        }
                    }
                }
            }
        }
        check::<mode::Zero, 2>(true);
        check::<mode::HalfAway, 10>(true);
        check::<mode::HalfEven, 2>(true);
        check::<mode::Away, 10>(false);

        use core::str::FromStr;
        let x = DBig::from_str("-0.3").unwrap();
        assert!(x.round().repr().is_neg_zero());
        assert!(x.quantize(0).value().repr().is_neg_zero());
        assert!(DBig::from_str("-1")
            .unwrap()
            .quantize(1)
            .value()
            .repr()
            .is_neg_zero());
        assert!(DBig::from_str("0.03")
            .unwrap()
            .quantize(-1)
            .value()
            .repr()
            .is_pos_zero());
        let b = FBig::<mode::Zero, 2>::from_parts(IBig::from(-1), -2); // -0.25
        assert!(b.quantize(0).value().repr().is_neg_zero());
        // a nonzero result is unaffected
        assert_eq!(
            DBig::from_str("-7").unwrap().quantize(1),
            Approximation::Inexact(DBig::from_parts(IBig::from(-1), 1), Rounding::SubOne)
        );
    }

    /// `quantize` accepts any `isize` quantum exponent: the distance to the stored exponent
    /// is computed without overflow (a finer quantum near `isize::MIN` used to panic with
    /// "attempt to subtract with overflow" in debug builds).
    #[test]
    fn test_quantize_extreme_exponents() {
        fn check<R: Round, const B: Word>() {
            for precision in [20usize, 50, 100, 500] {
                let one = fbig::<R, B>(1, 0, precision);
                for exp in [isize::MIN / 2, isize::MIN + 2, isize::MIN + 1, isize::MIN] {
                    let q = one.quantize(exp);
                    assert!(matches!(q, Approximation::Exact(_)));
                    let q = q.value();
                    assert_eq!(q, one);
                    assert_eq!(q.precision(), exp.unsigned_abs() + 1);
                }
                // the opposite ends of the exponent range
                let tiny = fbig::<R, B>(3, isize::MIN + 1, precision);
                assert!(matches!(tiny.quantize(isize::MAX), Approximation::Inexact(_, _)));
                let huge = fbig::<R, B>(3, isize::MAX - 1, precision);
                let q = huge.quantize(isize::MIN + 1);
                assert_eq!(q.clone().value(), huge);
                // the exponent distance (isize::MAX - 1) - (isize::MIN + 1), plus the digits
                assert_eq!(q.value().precision(), usize::MAX - 2 + huge.repr().digits());
            }
        }
        check::<mode::HalfAway, 10>();
        check::<mode::Zero, 2>();
        assert_eq!(Sign::Positive, DBig::ONE.quantize(isize::MIN).value().sign());
    }
}

// An unlimited-precision value is exact, so its error bounds are zero under every mode. The
// directed modes `Down` and `Up` used to ask for its ulp, which panics.
#[test]
fn test_error_bounds_unlimited_precision() {
    use dashu_float::{
        round::{mode, ErrorBounds},
        FBig, Repr,
    };

    fn check<R: ErrorBounds, const B: dashu_int::Word>() {
        for repr in [
            Repr::<B>::one(),
            Repr::neg_one(),
            Repr::zero(),
            Repr::neg_zero(),
            Repr::new(7.into(), 0),
            Repr::new((-7).into(), -3),
        ] {
            let f = FBig::<R, B>::from_repr(repr, dashu_float::Context::new(0));
            let (l, r, incl_l, incl_r) = R::error_bounds(&f);
            assert!(l.repr().is_pos_zero() && r.repr().is_pos_zero());
            assert!(incl_l && incl_r);
        }
    }
    check::<mode::Zero, 2>();
    check::<mode::Away, 2>();
    check::<mode::Down, 2>();
    check::<mode::Up, 2>();
    check::<mode::HalfEven, 2>();
    check::<mode::HalfAway, 2>();
    check::<mode::Down, 10>();
    check::<mode::Up, 10>();
    check::<mode::Down, 3>();
    check::<mode::Up, 3>();
}

// The error bounds describe the values that round to a float, so rounding either end of the
// reported interval (or anything just inside it) at the float's precision must give the float
// back. In an odd base half an ulp has no finite expansion, and the round-to-nearest bounds
// used to be rounded *up* to the next digit (2/3 of an ulp in base 3), reaching into the
// neighbors' territory.
#[test]
fn test_error_bounds_round_back() {
    use dashu_float::{
        round::{mode, ErrorBounds},
        Context, FBig,
    };
    use dashu_int::IBig;

    fn check<R: ErrorBounds, const B: dashu_int::Word>() {
        let exact = Context::<R>::new(0);
        for precision in [1usize, 2, 3, 20, 50, 100, 500] {
            let context = Context::<R>::new(precision);
            // significands with `precision` digits, at a few exponents and both signs; they
            // include the power of the base, below which the floats are denser
            let top = IBig::from(B).pow(precision);
            let low = IBig::from(B).pow(precision - 1);
            let mut sigs = vec![
                top.clone() - 1,
                low.clone() + 1,
                (top.clone() + low.clone()) / 2,
                low.clone(),
            ];
            if precision == 1 {
                sigs = (1..B as i32).map(IBig::from).collect();
            }
            for sig in sigs {
                for exp in [-7isize, 0, 5] {
                    for sign in [1, -1] {
                        let f = FBig::<R, B>::from_repr(
                            dashu_float::Repr::new(sig.clone() * sign, exp),
                            context,
                        );
                        let (l, r, incl_l, incl_r) = R::error_bounds(&f);
                        let ulp = f.ulp();
                        // a point 1/B^3 of an ulp inside each bound, and the bound itself if
                        // it is included
                        let step = dashu_float::Repr::new(IBig::ONE, ulp.repr().exponent() - 3);
                        let lower = exact.sub(f.repr(), l.repr()).unwrap().value();
                        let upper = exact.add(f.repr(), r.repr()).unwrap().value();
                        let mut points = Vec::new();
                        if !l.repr().is_pos_zero() {
                            points.push(exact.add(lower.repr(), &step).unwrap().value());
                        }
                        if !r.repr().is_pos_zero() {
                            points.push(exact.sub(upper.repr(), &step).unwrap().value());
                        }
                        if incl_l {
                            points.push(lower.clone());
                        }
                        if incl_r {
                            points.push(upper.clone());
                        }
                        for x in points {
                            let rounded = x.clone().with_precision(precision).value();
                            assert_eq!(rounded, f, "base {B}, precision {precision}: {x:?}");
                        }
                        // and the bounds are not needlessly narrow: together they span at
                        // least 99% of an ulp (away from the powers of the base, where the
                        // side facing zero is shorter)
                        if sig == low {
                            continue;
                        }
                        let width = exact.add(l.repr(), r.repr()).unwrap().value();
                        let hundred = dashu_float::Repr::new(100.into(), 0);
                        let ninety_nine = dashu_float::Repr::new(99.into(), 0);
                        assert!(
                            exact.mul(width.repr(), &hundred).unwrap().value()
                                >= exact.mul(ulp.repr(), &ninety_nine).unwrap().value(),
                            "base {B}, precision {precision}"
                        );
                    }
                }
            }
        }
    }
    check::<mode::HalfAway, 3>();
    check::<mode::HalfEven, 3>();
    check::<mode::HalfAway, 7>();
    check::<mode::HalfEven, 7>();
    check::<mode::HalfAway, 35>();
    check::<mode::HalfEven, 35>();
    check::<mode::HalfAway, 2>();
    check::<mode::HalfEven, 2>();
    check::<mode::HalfAway, 10>();
    check::<mode::HalfEven, 10>();
    check::<mode::Zero, 3>();
    check::<mode::Away, 3>();
    check::<mode::Down, 3>();
    check::<mode::Up, 3>();
    check::<mode::Down, 10>();
    check::<mode::Up, 2>();

    // the reported case: in base 3 at one digit, 44/27 = 1.6296... rounds to 2, so it must
    // lie outside the interval reported for 1
    let one = FBig::<mode::HalfAway, 3>::from_repr(dashu_float::Repr::one(), Context::new(1));
    let (_, r, _, _) = mode::HalfAway::error_bounds(&one);
    let x = FBig::<mode::HalfAway, 3>::from_parts(IBig::from(44), -3);
    assert!(x > exact_sum(&one, &r));
    fn exact_sum<const B: dashu_int::Word>(
        a: &FBig<mode::HalfAway, B>,
        b: &FBig<mode::HalfAway, B>,
    ) -> FBig<mode::HalfAway, B> {
        Context::<mode::HalfAway>::new(0)
            .add(a.repr(), b.repr())
            .unwrap()
            .value()
    }
}

/// `ErrorBounds::error_bounds` describes the rounding preimage of a value: the bounds themselves
/// round to it exactly when flagged inclusive, points just inside always do, and points just
/// outside never do.
///
/// Regression: at a power of the base the neighbour toward zero lies in the finer binade below,
/// but the bound on that side was a full ulp (so e.g. at 3 decimal digits `[0.995, 1.005)` was
/// claimed for `1.00` instead of `[0.9995, 1.005)`), and the ties-to-even inclusion flags read
/// the parity of the normalized significand instead of the significand at the precision.
#[test]
fn test_error_bounds_are_the_rounding_preimage() {
    use dashu_float::{
        round::{mode, ErrorBounds},
        Context, FBig, Repr,
    };
    use dashu_int::{IBig, Word};

    fn check<R: ErrorBounds, const B: Word>(p: usize, y: &Repr<B>) {
        let exact = |r: Repr<B>| FBig::<R, B>::from_repr(r, Context::new(0));
        let rounds_to_y = |v: &FBig<R, B>| v.clone().with_precision(p).value().repr() == y;
        let (l, r, incl_l, incl_r) =
            R::error_bounds(&FBig::<R, B>::from_repr(y.clone(), Context::new(p)));
        let y_exact = exact(y.clone());
        assert!(rounds_to_y(&y_exact));
        for (bound, incl, left) in [(l, incl_l, true), (r, incl_r, false)] {
            let bound = exact(bound.into_repr());
            let step = |v: &FBig<R, B>, eps: &FBig<R, B>, outward: bool| {
                if left == outward {
                    v - eps
                } else {
                    v + eps
                }
            };
            if bound.repr().significand().is_zero() {
                // one-sided preimage: the value itself is the (inclusive) edge
                assert!(incl, "{y:?} at {p} digits: a zero bound must be inclusive");
                let eps = exact(Repr::new(IBig::ONE, y.exponent() - p as isize - 5));
                assert!(!rounds_to_y(&step(&y_exact, &eps, true)), "{y:?} at {p} digits");
            } else {
                let eps = exact(Repr::new(
                    bound.repr().significand().clone(),
                    bound.repr().exponent() - 3,
                ));
                let edge = step(&y_exact, &bound, true);
                assert_eq!(rounds_to_y(&edge), incl, "{y:?} at {p} digits, edge {edge:?}");
                assert!(rounds_to_y(&step(&edge, &eps, false)), "{y:?} at {p} digits: inside");
                assert!(!rounds_to_y(&step(&edge, &eps, true)), "{y:?} at {p} digits: outside");
            }
        }
    }

    fn sweep<const B: Word>() {
        for p in 1..=3usize {
            let hi = IBig::from(B).pow(p);
            let mut sig = IBig::ONE;
            while sig < hi {
                for exp in -2..=2 {
                    for s in [sig.clone(), -sig.clone()] {
                        let y = Repr::<B>::new(s, exp);
                        check::<mode::Zero, B>(p, &y);
                        check::<mode::Away, B>(p, &y);
                        check::<mode::Down, B>(p, &y);
                        check::<mode::Up, B>(p, &y);
                        check::<mode::HalfEven, B>(p, &y);
                        check::<mode::HalfAway, B>(p, &y);
                    }
                }
                sig += IBig::ONE;
            }
        }
    }
    // (even bases only: an odd base has no representable half-ulp)
    sweep::<2>();
    sweep::<4>();
    sweep::<10>();
}

/// `ErrorBounds::error_bounds` describes the set of reals that round to a value. At a power of
/// the base that set is lopsided: the representable values just inside it (towards zero) are
/// `B` times denser, so it reaches only `ulp/B` (half of that to nearest) on that side. The
/// bounds used to be symmetric there, which let the Ziv loop certify a value from the binade
/// below as the power of the base (`acos_unit(2^-52, 3)` at 1 bit returned 1, not 0.5).
#[test]
fn test_error_bounds_at_power_of_base() {
    use dashu_float::round::{mode, ErrorBounds};
    use dashu_float::FBig;

    // A point just inside each nonzero bound must round to `y`, a point just outside must not.
    fn check<R: ErrorBounds, const B: dashu_float::Word>(sig: i32, exp: isize, p: usize) {
        let y = FBig::<R, B>::from_parts(sig.into(), exp)
            .with_precision(p)
            .unwrap();
        let (l, r, _, _) = R::error_bounds(&y);
        let y0 = y.clone().with_precision(0).value();
        for (bound, below) in [(l, true), (r, false)] {
            if bound.repr().significand().is_zero() {
                continue;
            }
            let bound = bound.with_precision(0).value();
            let delta = bound.clone() >> 4;
            for (offset, inside) in [(&bound - &delta, true), (&bound + &delta, false)] {
                let point = if below { &y0 - offset } else { &y0 + offset };
                let rounded = point.clone().with_precision(p).value();
                assert_eq!(
                    rounded == y,
                    inside,
                    "{} at {p} digits (base {B}): {} rounds to {}",
                    y,
                    point,
                    rounded
                );
            }
        }
    }
    fn check_all<const B: dashu_float::Word>(sig: i32, exp: isize, p: usize) {
        check::<mode::Zero, B>(sig, exp, p);
        check::<mode::Away, B>(sig, exp, p);
        check::<mode::Up, B>(sig, exp, p);
        check::<mode::Down, B>(sig, exp, p);
        check::<mode::HalfEven, B>(sig, exp, p);
        check::<mode::HalfAway, B>(sig, exp, p);
    }
    for p in [1usize, 2, 5, 20] {
        for exp in [-3isize, 0, 4] {
            // powers of the base, both signs
            check_all::<2>(1, exp, p);
            check_all::<2>(-1, exp, p);
            check_all::<10>(1, exp, p);
            check_all::<10>(-1, exp, p);
        }
    }
    // ordinary values keep their symmetric bounds
    for p in [2usize, 5, 20] {
        check_all::<2>(3, -1, p);
        check_all::<2>(-3, 2, p);
        check_all::<10>(7, 0, p);
        check_all::<10>(-25, -1, p);
    }
}
