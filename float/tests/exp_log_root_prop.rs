//! Property tests for dashu-float exp / ln / powf / nth_root (pure-Rust).
//!
//! These run unconditionally in CI (no rug/GMP). They check inverses, the log
//! product rule, a correctly-rounded nth-root bracketing condition (structural),
//! and a correct-rounding self-oracle for `ln` (a 2p-precision result rounded
//! back to p agrees with the direct p-precision result within one ulp). The
//! strong rug/MPFR differential lives in the excluded `fuzz/` crate.

use dashu_float::ops::Abs;
use dashu_float::DBig;
use dashu_int::IBig;
use proptest::prelude::*;

// Precision in decimal digits. 10^50 ≈ 166 bits, so the significand exceeds the
// native u128 width and exercises the true arbitrary-precision code paths.
const P: usize = 50;

/// `10^(-P + slack)` — a tolerance in ulp at precision `P` (base 10).
fn tol(slack: isize) -> DBig {
    DBig::from_parts(IBig::from(1), -(P as isize) + slack)
}

/// Random precision-`P` `DBig` with value `m * 10^-4`.
fn x_from(m: i64) -> DBig {
    DBig::from_parts(IBig::from(m), -4)
        .with_precision(P)
        .value()
}

fn pos_x(max_m: i64) -> impl Strategy<Value = DBig> {
    (1..=max_m).prop_map(x_from)
}

fn signed_x(max_m: i64) -> impl Strategy<Value = DBig> {
    (-max_m..=max_m)
        .prop_map(x_from)
        .prop_filter("nonzero", |x| *x != DBig::ZERO)
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 64, ..Default::default() })]

    /// exp(ln(x)) == x   (x > 0)
    #[test]
    fn exp_ln_inverse(x in pos_x(100_000)) {
        let y = x.ln().exp();
        prop_assert!((y - x).abs() < tol(4));
    }

    /// ln(exp(x)) == x
    #[test]
    fn ln_exp_inverse(x in signed_x(2_000)) {
        let y = x.exp().ln();
        prop_assert!((y - x).abs() < tol(4));
    }

    /// ln(x*y) == ln(x) + ln(y)   (x, y > 0)
    #[test]
    fn ln_product(x in pos_x(100_000), y in pos_x(100_000)) {
        let lhs = (x.clone() * y.clone()).ln();
        let rhs = x.ln() + y.ln();
        prop_assert!((lhs - rhs).abs() < tol(3));
    }

    /// x.powf(1) == x   (x > 0)
    #[test]
    fn powf_identity(x in pos_x(50_000)) {
        let one = DBig::ONE.with_precision(P).value();
        let y = x.powf(&one);
        prop_assert!((y - x).abs() < tol(2));
    }

    /// nth_root is correctly rounded: for r = x.nth_root(n),
    ///   (r - ulp)^n < x < (r + ulp)^n   (structural, no tolerance).
    #[test]
    fn nth_root_within_ulp(x in pos_x(1_000_000), n in 2usize..=7) {
        let r = x.nth_root(n);
        let ulp = r.ulp();
        // promote to unlimited precision for the exact bracketing check
        let r = r.with_precision(0).value();
        let ulp = ulp.with_precision(0).value();
        let x = x.with_precision(0).value();
        let lower = (r.clone() - ulp.clone()).powi(IBig::from(n));
        let upper = (r.clone() + ulp).powi(IBig::from(n));
        prop_assert!(lower < x && x < upper);
    }

    /// Correct-rounding self-oracle for ln: the p-precision result must agree *exactly* with the
    /// 2p-precision result re-rounded to p (the Ziv loop guarantees both are the unique
    /// correctly-rounded value). Pre-Ziv this only held to within one ulp.
    #[test]
    fn ln_rounding_exact_oracle(x in pos_x(100_000)) {
        let r_p = x.ln();
        let x_2p = x.clone().with_precision(2 * P).value();
        let r_2p = x_2p.ln().with_precision(P).value();
        prop_assert_eq!(r_p, r_2p);
    }

    /// Correct-rounding self-oracle for exp: same construction as `ln_rounding_exact_oracle`.
    #[test]
    fn exp_rounding_exact_oracle(x in signed_x(2_000)) {
        let r_p = x.exp();
        let x_2p = x.clone().with_precision(2 * P).value();
        let r_2p = x_2p.exp().with_precision(P).value();
        prop_assert_eq!(r_p, r_2p);
    }

    /// Correct-rounding self-oracle for exp_m1.
    #[test]
    fn exp_m1_rounding_exact_oracle(x in signed_x(2_000)) {
        let r_p = x.exp_m1();
        let x_2p = x.clone().with_precision(2 * P).value();
        let r_2p = x_2p.exp_m1().with_precision(P).value();
        prop_assert_eq!(r_p, r_2p);
    }

    /// Correct-rounding self-oracle for ln_1p (domain x > -1; m·10⁻⁴ with m > -10000 keeps x > -1).
    #[test]
    fn ln_1p_rounding_exact_oracle(m in -9999i64..100_000i64) {
        let x = x_from(m);
        let r_p = x.ln_1p();
        let x_2p = x.clone().with_precision(2 * P).value();
        let r_2p = x_2p.ln_1p().with_precision(P).value();
        prop_assert_eq!(r_p, r_2p);
    }
}
