//! Property tests for dashu-float trigonometric functions (pure-Rust identities).
//!
//! These run unconditionally in CI (no rug/GMP). A correct implementation must
//! satisfy these identities; the residual is checked at a tolerance of a few ulp.
//! The strong rug/MPFR differential lives in the excluded `fuzz/` crate and is run
//! manually before a release.

use dashu_float::ops::Abs;
use dashu_float::DBig;
use dashu_int::IBig;
use proptest::prelude::*;

// Precision in decimal digits. 10^50 ≈ 166 bits, so the significand exceeds the
// native u128 width and exercises the true arbitrary-precision code paths.
const P: usize = 50;

/// `10^(-P + slack)` — a tolerance measured in ulp at precision `P` (base 10).
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

fn one() -> DBig {
    DBig::ONE.with_precision(P).value()
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 64, ..Default::default() })]

    /// sin^2(x) + cos^2(x) == 1
    #[test]
    fn pythagorean(x in x_in(-200_000, 200_000)) {
        let (s, c) = x.sin_cos();
        let resid = (s.clone() * s.clone() + c.clone() * c.clone() - one()).abs();
        prop_assert!(resid < tol(2));
    }

    /// sin(2x) == 2 sin(x) cos(x);  cos(2x) == cos^2(x) - sin^2(x)
    #[test]
    fn double_angle(x in x_in(-200_000, 200_000)) {
        let (s, c) = x.clone().sin_cos();
        let two_x = x.clone() + x.clone();
        let (s2, c2) = two_x.sin_cos();
        let two_sc = (s.clone() * c.clone()) + (s.clone() * c.clone());
        prop_assert!((s2 - two_sc).abs() < tol(2));
        let cos2x = (c.clone() * c.clone()) - (s.clone() * s);
        prop_assert!((c2 - cos2x).abs() < tol(2));
    }

    /// atan(x) + atan(1/x) == pi/2  (x > 0)
    #[test]
    fn atan_reciprocal(x in x_in(1, 200_000)) {
        let half_pi = DBig::pi(P) / 2i32;
        let inv = one() / x.clone();
        let resid = (x.atan() + inv.atan()) - half_pi;
        prop_assert!(resid.abs() < tol(2));
    }

    /// sin(-x) == -sin(x);  cos(-x) == cos(x)
    #[test]
    fn parity(x in x_in(-200_000, 200_000)) {
        let s = x.clone().sin();
        let c = x.clone().cos();
        let nx = -x.clone();
        prop_assert!((nx.clone().sin() + s).abs() < tol(2));
        prop_assert!((nx.cos() - c).abs() < tol(2));
    }

    /// sin_cos agrees with the standalone sin / cos
    #[test]
    fn sin_cos_consistency(x in x_in(-200_000, 200_000)) {
        let (s, c) = x.clone().sin_cos();
        prop_assert!((s - x.clone().sin()).abs() < tol(2));
        prop_assert!((c - x.cos()).abs() < tol(2));
    }
}
