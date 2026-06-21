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
