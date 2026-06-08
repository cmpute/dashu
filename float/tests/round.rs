use core::str::FromStr;

use dashu_base::{Approximation::*, ParseError};
use dashu_float::{round::Rounding::NoOp, DBig};

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
