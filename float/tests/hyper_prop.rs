//! Property tests for the hyperbolic functions (pure-Rust identities).
//!
//! These run unconditionally in CI. The residual is checked at a tolerance of a
//! few ulp. Note `cosh²−sinh² = 1` is evaluated on a small range: for large |x|
//! both terms are ≈ e^{2|x|}/4, so subtracting them to get 1 loses ≈2·log₁₀(cosh x)
//! digits, which would exceed the tolerance.

use dashu_float::ops::Abs;
use dashu_float::DBig;
use dashu_int::IBig;
use proptest::prelude::*;

const P: usize = 50;

/// `10^(-P + slack)` — a tolerance in ulp at precision `P` (base 10).
fn tol(slack: isize) -> DBig {
    DBig::from_parts(IBig::from(1), -(P as isize) + slack)
}

/// Random `DBig` at precision `P` with value `m * 10^-4` for `m in [min, max]`.
fn x_in(min: i64, max: i64) -> impl Strategy<Value = DBig> {
    (min..=max).prop_map(|m| {
        DBig::from_parts(IBig::from(m), -4)
            .with_precision(P)
            .value()
    })
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 64, ..Default::default() })]

    /// cosh²(x) - sinh²(x) == 1  (small |x|: avoids the cancellation for large |x|)
    #[test]
    fn pythagorean(x in x_in(-10_000, 10_000)) {
        let s = x.sinh();
        let c = x.cosh();
        let resid = (&c * &c - &s * &s - DBig::ONE).abs();
        prop_assert!(resid < tol(2));
    }

    /// sinh(−x) == −sinh(x);  cosh(−x) == cosh(x);  tanh(−x) == −tanh(x)
    #[test]
    fn parity(x in x_in(-200_000, 200_000)) {
        let s = x.sinh();
        let c = x.cosh();
        let t = x.tanh();
        let nx = -x;
        prop_assert!((nx.sinh() + s).abs() < tol(2));
        prop_assert!((nx.cosh() - c).abs() < tol(2));
        prop_assert!((nx.tanh() + t).abs() < tol(2));
    }

    /// tanh(x) == sinh(x) / cosh(x)
    #[test]
    fn tanh_ratio(x in x_in(-200_000, 200_000)) {
        let s = x.sinh();
        let c = x.cosh();
        let t = x.tanh();
        prop_assert!((t - s / c).abs() < tol(3));
    }

    /// asinh(sinh(x)) == x
    #[test]
    fn round_trip_asinh(x in x_in(-200_000, 200_000)) {
        let y = x.sinh().asinh();
        prop_assert!((y - x).abs() < tol(2));
    }

    /// acosh(cosh(x)) == x  (x ≥ 0)
    #[test]
    fn round_trip_acosh(x in x_in(0, 200_000)) {
        let y = x.cosh().acosh();
        prop_assert!((y - x).abs() < tol(2));
    }

    /// atanh(tanh(x)) == x  (small |x|: tanh saturates toward ±1 for large |x|, so the
    /// inverse loses ≈0.87·|x| decimal digits regardless of precision)
    #[test]
    fn round_trip_atanh(x in x_in(-20_000, 20_000)) {
        let y = x.tanh().atanh();
        prop_assert!((y - x).abs() < tol(2));
    }
}
