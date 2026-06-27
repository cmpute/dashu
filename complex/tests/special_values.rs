//! Exact, deterministic Annex G / Kahan special-value vectors for the arithmetic ops (no proptest).
//!
//! These exercise the context-layer short-circuits: `0·∞` / `0/0` / `∞/∞` map to
//! [`FpError::Indeterminate`], `z/0`/`∞·finite` map to the Riemann point at infinity, and
//! `finite/∞` / `0/finite` map to zero.

use dashu_base::Sign;
use dashu_cmplx::{CBig, Context, FBig, FpError};
use dashu_float::round::mode::HalfEven;

type C = CBig<HalfEven, 2>;
type F = FBig<HalfEven, 2>;

fn ctx() -> Context<HalfEven> {
    Context::new(53)
}

fn real(v: i64) -> C {
    CBig::from(F::from(v))
}

fn inf() -> C {
    CBig::from(F::INFINITY)
}

fn is_riemann(r: &C) -> bool {
    r.re().is_infinite() && r.re().sign() == Sign::Positive && r.imag().is_zero()
}

#[test]
fn mul_zero_infinity_is_indeterminate() {
    assert_eq!(ctx().mul(&real(0), &inf()), Err(FpError::Indeterminate));
    assert_eq!(ctx().mul(&inf(), &real(0)), Err(FpError::Indeterminate));
}

#[test]
fn mul_infinity_infinity_is_riemann() {
    let r = ctx().mul(&inf(), &inf()).unwrap().value();
    assert!(is_riemann(&r));
}

#[test]
fn mul_infinity_finite_is_riemann() {
    let r = ctx().mul(&real(3), &inf()).unwrap().value();
    assert!(is_riemann(&r));
}

#[test]
fn div_zero_zero_is_indeterminate() {
    assert_eq!(ctx().div(&real(0), &real(0)), Err(FpError::Indeterminate));
}

#[test]
fn div_inf_inf_is_indeterminate() {
    assert_eq!(ctx().div(&inf(), &inf()), Err(FpError::Indeterminate));
}

#[test]
fn div_by_zero_is_riemann() {
    let r = ctx().div(&real(3), &real(0)).unwrap().value();
    assert!(is_riemann(&r));
}

#[test]
fn div_inf_by_finite_is_riemann() {
    let r = ctx().div(&inf(), &real(3)).unwrap().value();
    assert!(is_riemann(&r));
}

#[test]
fn div_finite_by_inf_is_zero() {
    let r = ctx().div(&real(3), &inf()).unwrap().value();
    assert!(r.is_zero());
}

#[test]
fn div_zero_by_finite_is_zero() {
    let r = ctx().div(&real(0), &real(3)).unwrap().value();
    assert!(r.is_zero());
}

#[test]
fn inv_zero_is_riemann() {
    let r = ctx().inv(&real(0)).unwrap().value();
    assert!(is_riemann(&r));
}

#[test]
fn inv_inf_is_zero() {
    let r = ctx().inv(&inf()).unwrap().value();
    assert!(r.is_zero());
}

#[test]
fn mul_context_inexactness_flags() {
    // 2/3 · 3: exercises the CRounded path and its per-axis (Rounding, Rounding) flags.
    use dashu_float::round::Rounding;
    let two_thirds = F::from_parts(2.into(), -1).with_precision(53).value();
    let z = CBig::from(two_thirds);
    let w = CBig::from(F::from(3));
    let r = ctx().mul(&z, &w).unwrap();
    let _: (Rounding, Rounding) = match r {
        dashu_base::Approximation::Inexact(_, flags) => flags,
        dashu_base::Approximation::Exact(_) => (Rounding::NoOp, Rounding::NoOp),
    };
}

// --- sqrt / exp / log special values (M3) ---

#[test]
fn sqrt_pos_infinity() {
    let s = ctx().sqrt(&inf()).unwrap().value();
    assert!(is_riemann(&s));
}

#[test]
fn sqrt_zero_is_zero() {
    let s = ctx().sqrt(&real(0)).unwrap().value();
    assert!(s.is_zero());
}

#[test]
fn exp_pos_infinity_is_riemann() {
    let r = ctx().exp(&inf(), None).unwrap().value();
    assert!(is_riemann(&r));
}

#[test]
fn exp_neg_infinity_is_zero() {
    let neg_inf = CBig::from(F::NEG_INFINITY);
    let r = ctx().exp(&neg_inf, None).unwrap().value();
    assert!(r.is_zero());
}

#[test]
fn exp_imag_infinity_is_indeterminate() {
    let im_inf = CBig::from_parts(F::ZERO, F::INFINITY);
    assert_eq!(ctx().exp(&im_inf, None), Err(FpError::Indeterminate));
}

#[test]
fn log_zero_is_neg_infinity() {
    let r = ctx().log(&real(0), None).unwrap().value();
    assert!(r.re().is_infinite());
    assert_eq!(r.re().sign(), Sign::Negative);
}

#[test]
fn log_infinity_is_riemann() {
    let r = ctx().log(&inf(), None).unwrap().value();
    assert!(is_riemann(&r));
}

// --- proj / conj / arg / signed-zero branch-cut specials (M5 hardening) ---

#[test]
fn proj_infinity_is_riemann() {
    // proj collapses any infinity to +∞ + i·0
    assert!(is_riemann(&ctx().proj(&inf()).unwrap().value()));
    let im_inf = CBig::from_parts(F::ZERO, F::INFINITY);
    assert!(is_riemann(&ctx().proj(&im_inf).unwrap().value()));
}

#[test]
fn proj_finite_unchanged() {
    let z = real(3);
    let p = ctx().proj(&z).unwrap().value();
    assert!(p == z);
}

#[test]
fn conj_infinity_flips_imag_sign() {
    // conj(+inf + i·inf) = +inf - i·inf (the real part keeps its sign)
    let z = CBig::from_parts(F::INFINITY, F::INFINITY);
    let c = ctx().conj(&z).unwrap().value();
    assert!(c.re().is_infinite());
    assert!(c.imag().is_infinite());
    assert_eq!(c.imag().sign(), Sign::Negative);
}

#[test]
fn arg_of_imaginary_infinity_is_half_pi() {
    // arg(0 + i·inf) = π/2 > 0; arg(0 - i·inf) = -π/2 < 0
    let pos = CBig::from_parts(F::ZERO, F::INFINITY);
    let neg = CBig::from_parts(F::ZERO, F::NEG_INFINITY);
    assert!(ctx().arg(&pos, None).unwrap().value() > F::ZERO);
    assert!(ctx().arg(&neg, None).unwrap().value() < F::ZERO);
}

#[test]
fn log_negative_real_branch_cut() {
    // log(-r ± i·0) = ln r ± i·π: the sign of the imaginary zero selects the side of the cut.
    use dashu_float::{Context as FloatCtx, Repr};
    let f = FloatCtx::<HalfEven>::new(53);
    let neg_r = F::from(-4);
    let pos_zero = CBig::from_parts(neg_r.clone(), F::from_repr(Repr::zero(), f));
    let neg_zero = CBig::from_parts(neg_r, F::from_repr(Repr::neg_zero(), f));

    let (re_p, im_p) = ctx().log(&pos_zero, None).unwrap().value().into_parts();
    let (re_n, im_n) = ctx().log(&neg_zero, None).unwrap().value().into_parts();
    // both real parts = ln 4; imaginary parts are ±π
    assert!(re_p == re_n);
    assert!(im_p > F::ZERO); // +i·π
    assert!(im_n < F::ZERO); // -i·π
}

#[test]
fn sqrt_neg_infinity_is_imaginary_infinity() {
    // sqrt(-inf + i·0) = +0 + i·inf
    let neg_inf = CBig::from(F::NEG_INFINITY);
    let s = ctx().sqrt(&neg_inf).unwrap().value();
    assert!(s.re().is_zero());
    assert!(s.imag().is_infinite());
    assert_eq!(s.imag().sign(), Sign::Positive);
}
