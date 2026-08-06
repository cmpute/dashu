//! Exact, deterministic Annex G / Kahan special-value vectors for the arithmetic ops (no proptest).
//!
//! These exercise the context-layer short-circuits. Infinity is a **terminal value**: any
//! arithmetic that takes an infinite operand is rejected ([`FpError::InfiniteInput`], matching
//! `dashu-float`), while finite operands that blow up produce the Riemann point at infinity —
//! `z/0 → ∞` — and `0/0` maps to [`FpError::Indeterminate`].

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
    r.re().is_infinite() && r.re().sign() == Sign::Positive && r.im().is_pos_zero()
}

#[test]
fn mul_with_infinity_is_infinite_input() {
    // ∞ is terminal: any multiplication that takes an infinite operand is rejected.
    assert_eq!(ctx().mul(&real(0), &inf()), Err(FpError::InfiniteInput));
    assert_eq!(ctx().mul(&inf(), &real(0)), Err(FpError::InfiniteInput));
    assert_eq!(ctx().mul(&inf(), &inf()), Err(FpError::InfiniteInput));
    assert_eq!(ctx().mul(&real(3), &inf()), Err(FpError::InfiniteInput));
}

#[test]
fn div_zero_zero_is_indeterminate() {
    assert_eq!(ctx().div(&real(0), &real(0)), Err(FpError::Indeterminate));
}

#[test]
fn div_with_infinity_is_infinite_input() {
    // ∞ is terminal: any division that takes an infinite operand is rejected.
    assert_eq!(ctx().div(&inf(), &inf()), Err(FpError::InfiniteInput));
    assert_eq!(ctx().div(&inf(), &real(3)), Err(FpError::InfiniteInput));
    assert_eq!(ctx().div(&real(3), &inf()), Err(FpError::InfiniteInput));
}

#[test]
fn div_by_zero_is_riemann() {
    // Finite nonzero ÷ ±0 = ∞ (a terminal output).
    let r = ctx().div(&real(3), &real(0)).unwrap().value();
    assert!(is_riemann(&r));
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
fn inv_infinity_is_infinite_input() {
    // ∞ is terminal: `1/∞` is rejected rather than computed.
    assert_eq!(ctx().inv(&inf()), Err(FpError::InfiniteInput));
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
fn sqrt_infinity_is_infinite_input() {
    // ∞ is terminal: `sqrt(∞)` is rejected (matching dashu-float's `sqrt`).
    assert_eq!(ctx().sqrt(&inf()), Err(FpError::InfiniteInput));
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
fn log_infinity_is_infinite_input() {
    // ∞ is terminal: `log(∞)` is rejected (matching dashu-float's `ln`, which rejects ∞ inputs).
    assert_eq!(ctx().log(&inf(), None), Err(FpError::InfiniteInput));
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
    assert!(c.im().is_infinite());
    assert_eq!(c.im().sign(), Sign::Negative);
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
fn sqrt_neg_infinity_is_infinite_input() {
    // ∞ is terminal: `sqrt(-∞ + i·0)` is rejected, like `sqrt(+∞)`.
    let neg_inf = CBig::from(F::NEG_INFINITY);
    assert_eq!(ctx().sqrt(&neg_inf), Err(FpError::InfiniteInput));
}
