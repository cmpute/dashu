//! Tests for the hyperbolic functions: fixture values, IEEE-754 special values,
//! identities, and domain handling.

use core::str::FromStr;

use dashu_base::Sign;
use dashu_float::ops::Abs;
use dashu_float::{round::mode, Context, DBig, Repr};
use dashu_int::IBig;

#[test]
fn test_values() {
    // Low-precision fixtures, verified against mpmath
    // (sinh(0.5)=0.5210953054937474, cosh(0.5)=1.1276259652063807, …).
    let x = DBig::from_str("0.5000000").unwrap();
    assert_eq!(x.sinh().to_string(), "0.52109531");
    assert_eq!(x.cosh().to_string(), "1.127626");
    assert_eq!(x.tanh().to_string(), "0.46211716");
    assert_eq!(x.asinh().to_string(), "0.48121183");
    assert_eq!(x.atanh().to_string(), "0.54930614");
    let two = DBig::from_str("2.000000").unwrap();
    assert_eq!(two.acosh().to_string(), "1.316958");
}

#[test]
fn test_signed_zeros() {
    let ctx = Context::<mode::HalfEven>::new(30);
    // odd functions preserve the sign of zero: sinh/tanh/asinh/atanh(−0) = −0
    assert!(ctx
        .sinh::<10>(&Repr::<10>::zero(), None)
        .unwrap()
        .value()
        .repr()
        .is_zero());
    assert!(ctx
        .sinh::<10>(&Repr::<10>::neg_zero(), None)
        .unwrap()
        .value()
        .repr()
        .is_neg_zero());
    assert!(ctx
        .tanh::<10>(&Repr::<10>::neg_zero(), None)
        .unwrap()
        .value()
        .repr()
        .is_neg_zero());
    assert!(ctx
        .asinh::<10>(&Repr::<10>::neg_zero(), None)
        .unwrap()
        .value()
        .repr()
        .is_neg_zero());
    assert!(ctx
        .atanh::<10>(&Repr::<10>::neg_zero(), None)
        .unwrap()
        .value()
        .repr()
        .is_neg_zero());
    // even function: cosh(±0) = 1
    assert_eq!(
        ctx.cosh::<10>(&Repr::<10>::neg_zero(), None)
            .unwrap()
            .value(),
        DBig::from_str("1").unwrap()
    );
}

#[test]
fn test_infinities() {
    let ctx = Context::<mode::HalfEven>::new(30);
    let one = DBig::from_str("1").unwrap();

    // sinh(±inf) = ±inf
    let r = ctx
        .sinh::<10>(&Repr::<10>::infinity(), None)
        .unwrap()
        .value();
    assert!(r.repr().is_infinite() && r.repr().sign() == Sign::Positive);
    let r = ctx
        .sinh::<10>(&Repr::<10>::neg_infinity(), None)
        .unwrap()
        .value();
    assert!(r.repr().is_infinite() && r.repr().sign() == Sign::Negative);

    // cosh(±inf) = +inf
    let r = ctx
        .cosh::<10>(&Repr::<10>::neg_infinity(), None)
        .unwrap()
        .value();
    assert!(r.repr().is_infinite() && r.repr().sign() == Sign::Positive);

    // tanh(±inf) = ±1 (finite values, not infinities)
    assert!(!ctx
        .tanh::<10>(&Repr::<10>::infinity(), None)
        .unwrap()
        .value()
        .repr()
        .is_infinite());
    assert_eq!(
        ctx.tanh::<10>(&Repr::<10>::infinity(), None)
            .unwrap()
            .value(),
        one.clone()
    );
    assert_eq!(
        ctx.tanh::<10>(&Repr::<10>::neg_infinity(), None)
            .unwrap()
            .value(),
        -one
    );

    // asinh(±inf) = ±inf
    let r = ctx
        .asinh::<10>(&Repr::<10>::neg_infinity(), None)
        .unwrap()
        .value();
    assert!(r.repr().is_infinite() && r.repr().sign() == Sign::Negative);

    // acosh(+inf) = +inf
    let r = ctx
        .acosh::<10>(&Repr::<10>::infinity(), None)
        .unwrap()
        .value();
    assert!(r.repr().is_infinite() && r.repr().sign() == Sign::Positive);
}

#[test]
fn test_pythagorean_identity() {
    let x = DBig::from_str("1.234").unwrap().with_precision(30).value();
    let s = x.sinh();
    let c = x.cosh();
    let resid = (&c * &c - &s * &s - DBig::ONE).abs();
    assert!(resid < DBig::from_parts(IBig::from(1), -27), "cosh²-sinh²-1 = {resid}");
}

#[test]
fn test_round_trips() {
    let x = DBig::from_str("1.5").unwrap().with_precision(30).value();
    let tol = DBig::from_parts(IBig::from(1), -27);
    assert!((x.sinh().asinh() - &x).abs() < tol);
    assert!((x.cosh().acosh() - &x).abs() < tol);
    assert!((x.tanh().atanh() - &x).abs() < tol);
}

#[test]
fn test_acosh_one_is_zero() {
    let ctx = Context::<mode::HalfEven>::new(30);
    let r = ctx.acosh::<10>(&Repr::<10>::one(), None).unwrap().value();
    assert!(r.repr().is_zero());
}

#[test]
fn test_atanh_one_is_infinity() {
    let ctx = Context::<mode::HalfEven>::new(30);
    let r = ctx.atanh::<10>(&Repr::<10>::one(), None).unwrap().value();
    assert!(r.repr().is_infinite() && r.repr().sign() == Sign::Positive);
    let r = ctx
        .atanh::<10>(&Repr::new(IBig::from(-1), 0), None)
        .unwrap()
        .value();
    assert!(r.repr().is_infinite() && r.repr().sign() == Sign::Negative);
}

#[test]
#[should_panic]
fn test_acosh_out_of_domain_panics() {
    let _ = DBig::from_str("0.5").unwrap().acosh();
}

#[test]
#[should_panic]
fn test_atanh_out_of_domain_panics() {
    let _ = DBig::from_str("2").unwrap().atanh();
}
