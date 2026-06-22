//! Tests for IEEE-754 signed zero (`-0`) propagation across operations.

use core::str::FromStr;

use dashu_base::{Abs, ParseError, Sign};
use dashu_float::{round::mode, Context, DBig, FBig, Repr};

mod helper_macros;

/// The default binary FBig (Zero rounding mode).
type F = FBig;

fn r2(significand: i32, exponent: isize) -> Repr<2> {
    Repr::new(significand.into(), exponent)
}

/// Helper: assert the value is a negative zero.
fn assert_neg_zero(v: &FBig) {
    assert!(v.repr().is_neg_zero(), "expected -0, got {:?}", v.repr());
}
/// Helper: assert the value is a positive zero.
fn assert_pos_zero(v: &FBig) {
    assert!(v.repr().is_zero(), "expected +0, got {:?}", v.repr());
}

#[test]
fn test_f64_round_trip() {
    // -0.0 round-trips through FBig
    let negz: F = FBig::try_from(-0.0f64).unwrap();
    assert_neg_zero(&negz);
    let posz: F = FBig::try_from(0.0f64).unwrap();
    assert_pos_zero(&posz);

    // back to f64 preserves the sign
    assert!(negz.to_f64().value().is_sign_negative());
    assert!(!posz.to_f64().value().is_sign_negative());
}

#[test]
fn test_equality_and_order() {
    let negz: F = FBig::try_from(-0.0f64).unwrap();
    let posz: F = FBig::try_from(0.0f64).unwrap();
    assert_eq!(negz, posz); // -0 == +0
    assert!(negz >= posz); // total order: -0 is not less than +0
    assert!(negz <= posz);
    // Repr equality too
    assert_eq!(Repr::<2>::neg_zero(), Repr::<2>::zero());
}

#[test]
fn test_neg_and_abs() {
    let negz: F = FBig::try_from(-0.0f64).unwrap();
    let posz: F = FBig::try_from(0.0f64).unwrap();
    assert_pos_zero(&-negz.clone()); // -(-0) = +0
    assert_neg_zero(&-posz.clone()); // -(+0) = -0
    assert_pos_zero(&negz.abs()); // abs(-0) = +0
    assert_pos_zero(&posz.abs());
}

#[test]
fn test_signum() {
    let negz: F = FBig::try_from(-0.0f64).unwrap();
    let posz: F = FBig::try_from(0.0f64).unwrap();
    // signum(±0) = +0
    assert_pos_zero(&negz.signum());
    assert_pos_zero(&posz.signum());
    assert_eq!(negz.sign(), Sign::Negative);
    assert_eq!(posz.sign(), Sign::Positive);
}

#[test]
fn test_mul_signed_zero() {
    let f = |x: f64| -> F { FBig::try_from(x).unwrap() };
    // -0 * 5 = -0 ; -0 * -5 = +0 ; +0 * 5 = +0 ; +0 * -5 = -0
    let r = f(-0.0) * f(5.0);
    assert_neg_zero(&r);
    let r = f(-0.0) * f(-5.0);
    assert_pos_zero(&r);
    let r = f(0.0) * f(5.0);
    assert_pos_zero(&r);
    let r = f(0.0) * f(-5.0);
    assert_neg_zero(&r);
}

#[test]
fn test_div_signed_zero() {
    let ctx = Context::<mode::HalfEven>::new(53);
    let negz = ctx
        .div::<2>(&Repr::<2>::neg_zero(), &r2(5, 0))
        .unwrap()
        .value();
    assert!(negz.repr().is_neg_zero()); // -0 / 5 = -0
    let posz = ctx.div::<2>(&Repr::<2>::zero(), &r2(5, 0)).unwrap().value();
    assert!(posz.repr().is_zero()); // +0 / 5 = +0
}

#[test]
fn test_sqrt_signed_zero() {
    let ctx = Context::<mode::HalfEven>::new(53);
    let negz = ctx.sqrt::<2>(&Repr::<2>::neg_zero()).unwrap().value();
    assert!(negz.repr().is_neg_zero()); // sqrt(-0) = -0
    let posz = ctx.sqrt::<2>(&Repr::<2>::zero()).unwrap().value();
    assert!(posz.repr().is_zero()); // sqrt(+0) = +0
}

#[test]
fn test_trig_signed_zero() {
    let ctx = Context::<mode::HalfEven>::new(53);
    let sin_neg0 = ctx.sin::<2>(&Repr::<2>::neg_zero(), None).unwrap().value();
    assert!(sin_neg0.repr().is_neg_zero()); // sin(-0) = -0
    let sin_pos0 = ctx.sin::<2>(&Repr::<2>::zero(), None).unwrap().value();
    assert!(sin_pos0.repr().is_zero()); // sin(+0) = +0
    let tan_neg0 = ctx.tan::<2>(&Repr::<2>::neg_zero(), None).unwrap().value();
    assert!(tan_neg0.repr().is_neg_zero()); // tan(-0) = -0
    let cos_neg0 = ctx.cos::<2>(&Repr::<2>::neg_zero(), None).unwrap().value();
    assert_eq!(cos_neg0, FBig::<mode::HalfEven>::ONE); // cos(±0) = 1
}

#[test]
fn test_rounding_ops_signed_zero() -> Result<(), ParseError> {
    // trunc / round sign of zero
    let half_neg = DBig::from_str("-0.5")?;
    assert!(half_neg.trunc().repr().is_neg_zero(), "trunc(-0.5) = -0");
    let third_neg = DBig::from_str("-0.3")?;
    assert!(third_neg.round().repr().is_neg_zero(), "round(-0.3) = -0");

    // -0 passes through ceil/floor/trunc unchanged
    let neg_zero_d = -DBig::ZERO;
    assert!(neg_zero_d.repr().is_neg_zero());
    assert!(neg_zero_d.ceil().repr().is_neg_zero(), "ceil(-0) = -0");
    assert!(neg_zero_d.floor().repr().is_neg_zero(), "floor(-0) = -0");
    assert!(neg_zero_d.trunc().repr().is_neg_zero(), "trunc(-0) = -0");

    // fract of a negative integer is -0
    let neg_five = DBig::from_str("-5")?;
    assert!(neg_five.fract().repr().is_neg_zero(), "fract(-5) = -0");
    Ok(())
}

#[test]
fn test_cancellation_under_down() -> Result<(), ParseError> {
    // x + (-x) yields -0 only under roundTowardNegative (Down); +0 otherwise.
    let three = DBig::from_str("3")?;
    let neg_three = DBig::from_str("-3")?;

    let down = Context::<mode::Down>::new(10);
    let sum_down = down
        .add::<10>(three.repr(), neg_three.repr())
        .unwrap()
        .value();
    assert!(sum_down.repr().is_neg_zero(), "(-3)+3 under Down = -0");

    let up = Context::<mode::Up>::new(10);
    let sum_up = up
        .add::<10>(three.repr(), neg_three.repr())
        .unwrap()
        .value();
    assert!(sum_up.repr().is_zero(), "(-3)+3 under Up = +0");

    // subtraction a - a likewise
    let sub_down = down.sub::<10>(three.repr(), three.repr()).unwrap().value();
    assert!(sub_down.repr().is_neg_zero(), "3-3 under Down = -0");
    Ok(())
}

#[test]
fn test_powi_signed_zero() {
    let ctx = Context::<mode::HalfEven>::new(53);
    let negz = ctx
        .powi::<2>(&Repr::<2>::neg_zero(), 3.into())
        .unwrap()
        .value();
    assert!(negz.repr().is_neg_zero()); // (-0)^3 = -0
    let posz = ctx
        .powi::<2>(&Repr::<2>::neg_zero(), 2.into())
        .unwrap()
        .value();
    assert!(posz.repr().is_zero()); // (-0)^2 = +0
}

#[test]
fn test_num_traits_sign() {
    use dashu_base::Signed;
    let negz: F = FBig::try_from(-0.0f64).unwrap();
    let posz: F = FBig::try_from(0.0f64).unwrap();
    // is_positive/is_negative follow the sign bit (matching Rust's f64::is_sign_*):
    // -0 is negative-signed, +0 is positive-signed.
    assert!(!negz.is_positive());
    assert!(negz.is_negative());
    assert!(posz.is_positive());
    assert!(!posz.is_negative());
}

#[test]
fn test_ln_1p_signed_zero() {
    let ctx = Context::<mode::HalfEven>::new(53);
    let r = ctx
        .ln_1p::<2>(&Repr::<2>::neg_zero(), None)
        .unwrap()
        .value();
    assert!(r.repr().is_neg_zero()); // ln_1p(-0) = -0
}
