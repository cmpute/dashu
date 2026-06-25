//! Tests for the `FpResult` contract: infinite inputs → `Err`, infinite outputs → `Ok(±inf)`,
//! and domain/indeterminate errors.

use dashu_base::Approximation::*;
use dashu_base::Sign;
use dashu_float::{
    math::{FpError, FpResult},
    round::mode,
    Context, FBig, Repr,
};
use dashu_int::IBig;

fn r2(sig: i32, exp: isize) -> Repr<2> {
    Repr::new(sig.into(), exp)
}

#[test]
fn test_div_by_zero_is_infinity() {
    let ctx = Context::<mode::HalfEven>::new(53);
    // finite / 0 = ±inf (a value, not an error); sign = XOR
    let pos = ctx.div::<2>(&r2(1, 0), &Repr::<2>::zero()).unwrap().value();
    assert!(pos.repr().is_infinite());
    assert_eq!(pos.repr().sign(), Sign::Positive);

    let neg = ctx
        .div::<2>(&r2(-1, 0), &Repr::<2>::zero())
        .unwrap()
        .value();
    assert_eq!(neg.repr().sign(), Sign::Negative);

    // 1 / -0 = -inf
    let neg2 = ctx
        .div::<2>(&r2(1, 0), &Repr::<2>::neg_zero())
        .unwrap()
        .value();
    assert_eq!(neg2.repr().sign(), Sign::Negative);
}

#[test]
fn test_zero_over_zero_is_indeterminate() {
    let ctx = Context::<mode::HalfEven>::new(53);
    assert_eq!(
        ctx.div::<2>(&Repr::<2>::zero(), &Repr::<2>::zero()),
        Err(FpError::Indeterminate)
    );
}

#[test]
fn test_inv_zero_is_infinity() {
    let ctx = Context::<mode::HalfEven>::new(53);
    let r = ctx.inv::<2>(&Repr::<2>::zero()).unwrap().value();
    assert!(r.repr().is_infinite());
    assert_eq!(r.repr().sign(), Sign::Positive);
}

#[test]
fn test_ln_zero_is_neg_infinity() {
    let ctx = Context::<mode::HalfEven>::new(53);
    let r = ctx.ln::<2>(&Repr::<2>::zero(), None).unwrap().value();
    assert!(r.repr().is_infinite());
    assert_eq!(r.repr().sign(), Sign::Negative);
}

#[test]
fn test_domain_errors() {
    let ctx = Context::<mode::HalfEven>::new(53);
    assert_eq!(ctx.sqrt::<2>(&r2(-1, 0)), Err(FpError::OutOfDomain));
    assert_eq!(ctx.ln::<2>(&r2(-1, 0), None), Err(FpError::OutOfDomain));
    assert_eq!(ctx.asin::<2>(&r2(2, 0), None), Err(FpError::OutOfDomain));
    assert_eq!(
        ctx.atan2::<2>(&Repr::<2>::zero(), &Repr::<2>::zero(), None),
        Err(FpError::OutOfDomain)
    );
}

#[test]
fn test_infinite_input_is_error() {
    let ctx = Context::<mode::HalfEven>::new(53);
    let inf = Repr::<2>::infinity();
    assert_eq!(ctx.add::<2>(&inf, &r2(1, 0)), Err(FpError::InfiniteInput));
    assert_eq!(ctx.mul::<2>(&inf, &r2(1, 0)), Err(FpError::InfiniteInput));
    assert_eq!(ctx.sqrt::<2>(&inf), Err(FpError::InfiniteInput));
    // exp(+inf) = +inf, exp(-inf) = +0
    assert!(ctx.unwrap_fp(ctx.exp::<2>(&inf, None)).repr().is_infinite());
    assert_eq!(ctx.exp::<2>(&inf, None).unwrap().value().repr().sign(), Sign::Positive);
    assert!(ctx
        .unwrap_fp(ctx.exp::<2>(&Repr::<2>::neg_infinity(), None)).repr()
        .is_zero());
    assert_eq!(ctx.sin::<2>(&inf, None), Err(FpError::InfiniteInput));
}

#[test]
fn test_atan_infinity_is_preserved() {
    let ctx = Context::<mode::HalfEven>::new(53);
    // atan(±inf) = ±π/2 — a finite result, preserved (not an error)
    let r = ctx.atan::<2>(&Repr::<2>::infinity(), None).unwrap().value();
    assert!(r.repr().sign() == Sign::Positive);
    // it should be approximately π/2
    assert!(r > FBig::<mode::HalfEven>::ONE);
}

#[test]
fn test_fbig_div_zero_produces_infinity() {
    // FBig convenience layer: 1 / 0 yields an infinity-valued FBig (no panic).
    let one = FBig::<mode::HalfEven>::try_from(1.0f64).unwrap();
    let zero = FBig::<mode::HalfEven>::try_from(0.0f64).unwrap();
    let inf = one / zero;
    assert!(inf.repr().is_infinite());
}

#[test]
#[should_panic]
fn test_fbig_zero_over_zero_panics() {
    // 0 / 0 is indeterminate; the FBig layer panics.
    let zero = FBig::<mode::HalfEven>::try_from(0.0f64).unwrap();
    let _ = zero.clone() / zero;
}

#[test]
#[should_panic]
fn test_fbig_sqrt_negative_panics() {
    // sqrt(-1) is out of domain; the FBig layer panics.
    let neg_one = FBig::<mode::HalfEven>::try_from(-1.0f64).unwrap();
    use dashu_base::SquareRoot;
    let _ = neg_one.sqrt();
}

#[test]
fn test_fpresult_type_alias() {
    // FpResult is Result<Rounded<T>, FpError>.
    let r: FpResult<FBig> = Ok(Exact(FBig::ZERO));
    assert!(r.is_ok());
}

#[test]
fn test_exp_overflow_is_infinity() {
    let ctx = Context::<mode::HalfEven>::new(53);
    // exp(huge) overflows the isize exponent range -> Overflow error at Context level,
    // converted to +inf by the FBig convenience layer.
    // Need x large enough that floor(x/ln2) > isize::MAX, i.e. x > ~2^62.5.
    let huge = Repr::new(IBig::from(1) << 63, 0);
    assert_eq!(ctx.exp::<2>(&huge, None), Err(FpError::Overflow(Sign::Positive)));

    // exp(huge negative) underflows to +0
    let neg = Repr::new(-(IBig::from(1) << 63), 0);
    assert_eq!(ctx.exp::<2>(&neg, None), Err(FpError::Underflow(Sign::Positive)));

    // exp_m1(huge negative) -> -1 (a finite value, not an error)
    let m1 = ctx.exp_m1::<2>(&neg, None).unwrap().value();
    assert_eq!(m1, -FBig::<mode::HalfEven>::ONE);
}

#[test]
fn test_powf_zero_base() {
    use dashu_float::DBig;
    // powf with a float exponent returns the *positive* result on a zero base
    // (matching the common float-pow convention); use powi for the signed result.
    let ctx = Context::<mode::HalfEven>::new(53);
    // powf(-0, 2.0) = +0 (NOT -0)
    let r = ctx
        .powf::<2>(&Repr::<2>::neg_zero(), &Repr::new(2.into(), 0), None)
        .unwrap()
        .value();
    assert!(r.repr().is_zero(), "expected +0");
    assert!(!r.repr().is_neg_zero(), "powf(-0, x) should be +0, not -0");
    // powf(0, -1) = +inf
    let r = ctx
        .powf::<2>(&Repr::<2>::zero(), &Repr::new((-1i32).into(), 0), None)
        .unwrap()
        .value();
    assert!(r.repr().is_infinite());
    assert_eq!(r.repr().sign(), Sign::Positive);

    // powi(-0, 3) = -0 (the sign-correct, integer-exponent variant)
    let r = ctx
        .powi::<2>(&Repr::<2>::neg_zero(), 3.into())
        .unwrap()
        .value();
    assert!(r.repr().is_neg_zero());
    let _ = DBig::ZERO;
}

#[test]
fn test_shr_assign_shifts_once() {
    // Regression: shr_assign previously subtracted rhs twice.
    let mut x = FBig::<mode::HalfEven>::try_from(8.0f64).unwrap(); // 2^3
    x >>= 1; // 2^2 = 4
    let y = FBig::<mode::HalfEven>::try_from(4.0f64).unwrap();
    assert_eq!(x, y);
}
