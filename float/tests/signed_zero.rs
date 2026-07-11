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
    assert!(v.repr().is_pos_zero(), "expected +0, got {:?}", v.repr());
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
    assert!(posz.repr().is_pos_zero()); // +0 / 5 = +0
}

#[test]
fn test_sqrt_signed_zero() {
    let ctx = Context::<mode::HalfEven>::new(53);
    let negz = ctx.sqrt::<2>(&Repr::<2>::neg_zero()).unwrap().value();
    assert!(negz.repr().is_neg_zero()); // sqrt(-0) = -0
    let posz = ctx.sqrt::<2>(&Repr::<2>::zero()).unwrap().value();
    assert!(posz.repr().is_pos_zero()); // sqrt(+0) = +0
}

#[test]
fn test_trig_signed_zero() {
    let ctx = Context::<mode::HalfEven>::new(53);
    let sin_neg0 = ctx.sin::<2>(&Repr::<2>::neg_zero(), None).unwrap().value();
    assert!(sin_neg0.repr().is_neg_zero()); // sin(-0) = -0
    let sin_pos0 = ctx.sin::<2>(&Repr::<2>::zero(), None).unwrap().value();
    assert!(sin_pos0.repr().is_pos_zero()); // sin(+0) = +0
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
    assert!(sum_up.repr().is_pos_zero(), "(-3)+3 under Up = +0");

    // subtraction a - a likewise
    let sub_down = down.sub::<10>(three.repr(), three.repr()).unwrap().value();
    assert!(sub_down.repr().is_neg_zero(), "3-3 under Down = -0");

    // The FBig operator path (Add/Sub traits) must agree: a - a under Down = -0.
    let a = FBig::<mode::Down, 10>::from_str("3")?;
    let a2 = FBig::<mode::Down, 10>::from_str("3")?;
    let diff = a - a2;
    assert!(diff.repr().is_neg_zero(), "FBig 3-3 under Down = -0");
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
    assert!(posz.repr().is_pos_zero()); // (-0)^2 = +0
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

#[test]
fn test_exp_m1_signed_zero() {
    let ctx = Context::<mode::HalfEven>::new(53);
    // IEEE 754 §9.2.1: exp_m1(-0) = -0, exp_m1(+0) = +0.
    let neg = ctx
        .exp_m1::<2>(&Repr::<2>::neg_zero(), None)
        .unwrap()
        .value();
    assert!(neg.repr().is_neg_zero());
    let pos = ctx.exp_m1::<2>(&Repr::<2>::zero(), None).unwrap().value();
    assert!(pos.repr().is_pos_zero() && !pos.repr().is_neg_zero());
}

#[test]
fn test_powf_neg_zero_exponent() {
    // pow(x, ±0) = 1 for any base, including a negative one (IEEE 754 §9.2.1). The float-exponent
    // path must treat `-0` like `+0`; otherwise a negative base falls through to OutOfDomain.
    let ctx = Context::<mode::HalfEven>::new(53);
    let one = FBig::<mode::HalfEven>::ONE.with_precision(53).unwrap();
    for base in [r2(5, 0), r2(-5, 0), Repr::<2>::neg_zero()] {
        let r_neg = ctx
            .powf::<2>(&base, &Repr::<2>::neg_zero(), None)
            .unwrap()
            .value();
        let r_pos = ctx
            .powf::<2>(&base, &Repr::<2>::zero(), None)
            .unwrap()
            .value();
        assert_eq!(r_neg, one, "powf({:?}, -0) should be 1", base);
        assert_eq!(r_pos, one, "powf({:?}, +0) should be 1", base);
    }
}

#[test]
fn test_quantize_preserves_neg_zero() {
    // quantize is sign-preserving: quantize(-0, q) stays -0 for any quantum.
    let negz = FBig::<mode::HalfEven, 2>::from_repr(Repr::<2>::neg_zero(), Context::new(53));
    for q in [0isize, 3, -1] {
        let r = negz.clone().quantize(q).value();
        assert!(r.repr().is_neg_zero(), "quantize(-0, {}) should keep -0, got {:?}", q, r.repr());
    }
}

#[test]
fn test_error_bounds_unlimited_precision_is_exact() {
    // ErrorBounds contract: unlimited precision (precision 0) ⇒ (ZERO, ZERO, true, true) for
    // every value, including `-0` and nonzero values. Verifies the Away/Zero asymmetry fix.
    use dashu_float::round::ErrorBounds;
    let mk = |repr: Repr<2>| FBig::<mode::Away, 2>::from_repr(repr, Context::new(0));
    for v in [
        mk(Repr::neg_zero()),
        mk(Repr::zero()),
        mk(r2(7, 0)),
        mk(r2(-7, 0)),
    ] {
        let (l, r, il, ir) = mode::Away::error_bounds(&v);
        assert_eq!(l, FBig::<mode::Away, 2>::ZERO, "lower bound nonzero for {:?}", v.repr());
        assert_eq!(r, FBig::<mode::Away, 2>::ZERO, "upper bound nonzero for {:?}", v.repr());
        assert!(il && ir, "bounds not inclusive for {:?}", v.repr());
    }
}

#[test]
fn test_error_bounds_signed_zero_is_one_sided() {
    // Interpretation (A): `+0` is the canonical zero (symmetric interval), while `-0` carries a
    // sign and gets the one-sided (Negative) interval. All three sign-aware modes now agree:
    // Zero, HalfAway and HalfEven return (false, false) for `+0` and (false, true) for `-0`.
    use dashu_float::round::ErrorBounds;
    let mk_z = |repr: Repr<2>| FBig::<mode::Zero, 2>::from_repr(repr, Context::new(8));
    let half = |repr: Repr<2>| FBig::<mode::HalfEven, 2>::from_repr(repr, Context::new(8));

    let (_, _, il, ir) = mode::Zero::error_bounds(&mk_z(Repr::zero()));
    assert_eq!((il, ir), (false, false), "Zero +0 symmetric");
    let (_, _, il, ir) = mode::Zero::error_bounds(&mk_z(Repr::neg_zero()));
    assert_eq!((il, ir), (false, true), "Zero -0 one-sided");

    let mk_a = |repr: Repr<2>| FBig::<mode::HalfAway, 2>::from_repr(repr, Context::new(8));
    let (_, _, il, ir) = mode::HalfAway::error_bounds(&mk_a(Repr::zero()));
    assert_eq!((il, ir), (false, false), "HalfAway +0 symmetric");
    let (_, _, il, ir) = mode::HalfAway::error_bounds(&mk_a(Repr::neg_zero()));
    assert_eq!((il, ir), (false, true), "HalfAway -0 one-sided");

    let (_, _, il, ir) = mode::HalfEven::error_bounds(&half(Repr::zero()));
    assert_eq!((il, ir), (false, false), "HalfEven +0 symmetric");
    let (_, _, il, ir) = mode::HalfEven::error_bounds(&half(Repr::neg_zero()));
    assert_eq!((il, ir), (false, true), "HalfEven -0 one-sided (aligned with HalfAway)");
}

#[cfg(feature = "num-order")]
#[test]
fn test_numord_neg_zero_equals_primitive_zero() {
    // `-0` is numerically equal to primitive `0.0`, so NumOrd must report Equal — not Less.
    use num_order::NumOrd;
    let negz = FBig::<mode::HalfEven, 2>::from_repr(Repr::<2>::neg_zero(), Context::new(53));
    assert_eq!(negz.num_partial_cmp(&0.0f64), Some(core::cmp::Ordering::Equal));
    assert_eq!(negz.num_partial_cmp(&-0.0f64), Some(core::cmp::Ordering::Equal));
    // sanity: a negative nonzero value still orders below zero
    let neg = FBig::<mode::HalfEven, 2>::from_repr(r2(-5, 0), Context::new(53));
    assert_eq!(neg.num_partial_cmp(&0.0f64), Some(core::cmp::Ordering::Less));
}

#[test]
fn test_asin_acos_unit_under_down() {
    // Under roundTowardNegative (Down), `1 - 1` cancels to `-0`, so `d = sqrt(1 - x²) = -0` at
    // |x| = 1. The `d`-is-zero branch must catch `-0` too, or the general path divides by `-0`
    // → `-∞` and panics (regression: asin/acos(±1) used to panic under Down). The exact value
    // (±π/2, 0/π) is mode-independent; only its last digit may round differently, so we check the
    // sign and that it is within 1 ULP of the round-to-nearest result rather than bit-equality.
    let down = Context::<mode::Down>::new(53);

    let asin1 = down.asin::<2>(&Repr::<2>::one(), None).unwrap().value();
    let asinm1 = down.asin::<2>(&Repr::<2>::neg_one(), None).unwrap().value();
    assert_eq!(asin1.sign(), Sign::Positive);
    assert_eq!(asinm1.sign(), Sign::Negative);
    assert!((asin1.to_f64().value() - core::f64::consts::FRAC_PI_2).abs() < 1e-15);
    assert!((asinm1.to_f64().value() + core::f64::consts::FRAC_PI_2).abs() < 1e-15);

    let acos1 = down.acos::<2>(&Repr::<2>::one(), None).unwrap().value();
    let acosm1 = down.acos::<2>(&Repr::<2>::neg_one(), None).unwrap().value();
    assert!(
        acos1.repr().is_pos_zero() || acos1.repr().is_neg_zero(),
        "acos(+1) = 0, got {:?}",
        acos1.repr()
    );
    assert!((acosm1.to_f64().value() - core::f64::consts::PI).abs() < 1e-14, "acos(-1) ≈ π");
}
