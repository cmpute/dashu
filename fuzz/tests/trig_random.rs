//! Differential / fuzz tests for dashu-float transcendentals against `rug::Float` (MPFR).
//!
//! The broad random differentials (sin/cos/tan/atan2/asin/acos) are proptest-driven so a mismatch
//! shrinks to a minimal counterexample; the inherently-sweep tests (π over precision, asin near 1,
//! the pinned large-exponent tan regression) stay as deterministic loops. All are `#[ignore]`d and
//! run manually before a release.
//!
//! Run with: `cargo test --manifest-path fuzz/Cargo.toml --test trig_random -- --ignored --nocapture`

use core::str::FromStr;
use dashu::float::ops::Abs;
use dashu::float::round::mode::HalfAway;
use dashu::float::{Context, DBig, Repr};
use proptest::prelude::*;
use rug::Float;

/// MPFR working precision (bits) large enough to hold `x` and the result to `prec` decimal digits
/// with margin: `(|exponent| + significand_digits)·log₂10` for `x`'s magnitude + `prec·log₂10` + slack.
fn rug_bits(x: &Repr<10>, prec: usize) -> u32 {
    let x_mag = (x.exponent().unsigned_abs() + x.digits()) as f64;
    let x_bits = (x_mag * 3.322).ceil() as u32 + 500;
    let p_bits = ((prec.max(100) as f64) * 3.322).ceil() as u32;
    p_bits + x_bits
}

/// Tolerance of `100 · 10^{-prec}` (~100 ulp at `prec` decimal digits) — both libraries are
/// near-/correctly-rounded, so a few-ulp divergence is expected; this catches real bugs.
fn tol(prec: usize) -> DBig {
    DBig::from_parts(100.into(), -(prec as isize))
}

proptest! {
    #![proptest_config(fuzz::fuzz_config())]

    /// sin(x) ≈ MPFR sin(x) across precisions {10, 20, 50, 100}.
    #[test]
    #[ignore]
    fn sin_fuzz(x in fuzz::dbig_strategy(-50..=50)) {
        let x_str = format!("{x:e}");
        for prec in [10usize, 20, 50, 100] {
            let ctx = Context::<HalfAway>::new(prec);
            let sin_d = ctx.sin::<10>(x.repr(), None).unwrap().value();
            let bits = rug_bits(x.repr(), prec);
            let x_rug = match Float::parse(&x_str) {
                Ok(p) => Float::with_val(bits, p),
                Err(_) => return Ok(()),
            };
            let sin_r = x_rug.sin();
            let s_r: DBig = DBig::from_str(&sin_r.to_string_radix(10, Some(prec))).unwrap();
            prop_assert!(
                (sin_d.clone() - s_r).abs() <= tol(prec),
                "sin mismatch x={x_str} prec={prec}: dashu={sin_d} rug={sin_r}"
            );
        }
    }

    /// cos(x) ≈ MPFR cos(x) across precisions {10, 20, 50, 100}.
    #[test]
    #[ignore]
    fn cos_fuzz(x in fuzz::dbig_strategy(-50..=50)) {
        let x_str = format!("{x:e}");
        for prec in [10usize, 20, 50, 100] {
            let ctx = Context::<HalfAway>::new(prec);
            let cos_d = ctx.cos::<10>(x.repr(), None).unwrap().value();
            let bits = rug_bits(x.repr(), prec);
            let x_rug = match Float::parse(&x_str) {
                Ok(p) => Float::with_val(bits, p),
                Err(_) => return Ok(()),
            };
            let cos_r = x_rug.cos();
            let c_r: DBig = DBig::from_str(&cos_r.to_string_radix(10, Some(prec))).unwrap();
            prop_assert!(
                (cos_d.clone() - c_r).abs() <= tol(prec),
                "cos mismatch x={x_str} prec={prec}: dashu={cos_d} rug={cos_r}"
            );
        }
    }

    /// tan(x) ≈ MPFR tan(x), skipping arguments where |cos(x)| < 1e-5 (too close to a singularity).
    #[test]
    #[ignore]
    fn tan_fuzz(x in fuzz::dbig_strategy(-50..=50)) {
        let x_str = format!("{x:e}");
        for prec in [10usize, 20, 50, 100] {
            let ctx = Context::<HalfAway>::new(prec);
            let cos_d = ctx.cos::<10>(x.repr(), None).unwrap().value();
            if cos_d.abs() <= DBig::from_parts(1.into(), -5) {
                continue; // near a singularity — tan is ill-conditioned, skip this precision
            }
            let tan_d = ctx.tan::<10>(x.repr(), None).unwrap().value();
            let bits = rug_bits(x.repr(), prec);
            let x_rug = match Float::parse(&x_str) {
                Ok(p) => Float::with_val(bits, p),
                Err(_) => return Ok(()),
            };
            let tan_r = x_rug.tan();
            let t_r: DBig = DBig::from_str(&tan_r.to_string_radix(10, Some(prec))).unwrap();
            prop_assert!(
                (tan_d.clone() - t_r).abs() <= tol(prec),
                "tan mismatch x={x_str} prec={prec}: dashu={tan_d} rug={tan_r}"
            );
        }
    }

    /// atan2(y, x) ≈ MPFR atan2(y, x) across precisions {20, 50}.
    #[test]
    #[ignore]
    fn atan2_fuzz(y in fuzz::dbig_strategy(-50..=50), x in fuzz::dbig_strategy(-50..=50)) {
        let y_str = format!("{y:e}");
        let x_str = format!("{x:e}");
        for prec in [20usize, 50] {
            let ctx = Context::<HalfAway>::new(prec);
            // atan2(0,0) (and other indeterminate forms) report FpError — skip those; nothing to
            // compare. Finite in-domain inputs never error here.
            let atan2_d = match ctx.atan2::<10>(y.repr(), x.repr(), None) {
                Ok(v) => v.value(),
                Err(_) => return Ok(()),
            };
            let bits = (rug_bits(y.repr(), prec)).max(rug_bits(x.repr(), prec));
            let y_rug = Float::with_val(bits, Float::parse(&y_str).unwrap());
            let x_rug = Float::with_val(bits, Float::parse(&x_str).unwrap());
            let atan2_r = y_rug.atan2(&x_rug);
            let a_r: DBig = DBig::from_str(&atan2_r.to_string_radix(10, Some(prec))).unwrap();
            prop_assert!(
                (atan2_d.clone() - a_r).abs() <= tol(prec),
                "atan2 mismatch y={y_str} x={x_str} prec={prec}: dashu={atan2_d} rug={atan2_r}"
            );
        }
    }

    /// asin(x)/acos(x) ≈ MPFR for x in [-1, 1] across precisions {20, 50}.
    #[test]
    #[ignore]
    fn inv_trig_fuzz(x in fuzz::unit_dbig()) {
        let x_str = format!("{x:e}");
        for prec in [20usize, 50] {
            let ctx = Context::<HalfAway>::new(prec);
            let bits = (prec as u32) * 4 + 128;
            let x_rug = Float::with_val(bits, Float::parse(&x_str).unwrap());

            let asin_d = ctx.asin::<10>(x.repr(), None).unwrap().value();
            let asin_r = x_rug.clone().asin();
            let a_r: DBig = DBig::from_str(&asin_r.to_string_radix(10, Some(prec))).unwrap();
            prop_assert!(
                (asin_d.clone() - a_r).abs() <= tol(prec),
                "asin mismatch x={x_str} prec={prec}: dashu={asin_d} rug={asin_r}"
            );

            let acos_d = ctx.acos::<10>(x.repr(), None).unwrap().value();
            let acos_r = x_rug.acos();
            let a_r: DBig = DBig::from_str(&acos_r.to_string_radix(10, Some(prec))).unwrap();
            prop_assert!(
                (acos_d.clone() - a_r).abs() <= tol(prec),
                "acos mismatch x={x_str} prec={prec}: dashu={acos_d} rug={acos_r}"
            );
        }
    }
}

/// π at every precision matches MPFR's π to within 1 ulp. (Deterministic precision sweep.)
#[test]
#[ignore]
fn pi_fuzz() {
    for prec in (10..1000).step_by(53) {
        let pi_dashu = DBig::pi(prec);
        let bits = (prec * 3322).div_ceil(1000) + 32;
        let pi_rug = Float::with_val(bits as u32, rug::float::Constant::Pi);
        let s_r: DBig = DBig::from_str(&pi_rug.to_string_radix(10, Some(prec))).unwrap();
        assert!(
            (pi_dashu.clone() - s_r).abs() <= DBig::from_parts(1.into(), -(prec as isize)),
            "Pi mismatch at prec={prec}: dashu={pi_dashu}, rug={pi_rug}"
        );
    }
}

/// asin near 1 (where it → π/2, most sensitive) for x = 1 - 10^-k. (Deterministic k sweep.)
#[test]
#[ignore]
fn asin_near_one_fuzz() {
    for k in 1u32..=15 {
        let eps = DBig::from_str(&format!("1e-{k}")).unwrap();
        let x = DBig::ONE - eps;
        let x_str = format!("{x:e}");
        for &prec in &[30usize, 100] {
            let ctx = Context::<HalfAway>::new(prec);
            let asin_d = ctx.asin::<10>(x.repr(), None).unwrap().value();
            let bits = (prec as u32) * 4 + 256;
            let x_rug = Float::with_val(bits, Float::parse(&x_str).unwrap());
            let asin_r = x_rug.asin();
            let a_r: DBig = DBig::from_str(&asin_r.to_string_radix(10, Some(prec))).unwrap();
            assert!(
                (asin_d.clone() - a_r).abs() <= tol(prec),
                "asin-near-1 mismatch k={k} prec={prec}: dashu={asin_d} rug={asin_r}"
            );
        }
    }
}

/// Regression: tan of a pinned very-large-exponent argument must match MPFR. (Deterministic.)
#[test]
#[ignore]
fn tan_large_exponent_regression() {
    let x_str = "-3.67225387623341113999117300261402819219640608e511";
    for prec in [20usize, 50] {
        let x = DBig::from_str(x_str).unwrap();
        let ctx = Context::<HalfAway>::new(prec);
        let tan_d = ctx.tan::<10>(x.repr(), None).unwrap().value();
        let bits = (prec as u32) * 4 + 512 + 1700; // extra bits for the large exponent
        let x_rug = Float::with_val(bits, Float::parse(x_str).unwrap());
        let tan_r = x_rug.tan();
        let t_r: DBig = DBig::from_str(&tan_r.to_string_radix(10, Some(prec))).unwrap();
        assert!(
            (tan_d.clone() - t_r).abs() <= tol(prec),
            "large-exponent tan regression failed at prec={prec}: dashu={tan_d}, rug={tan_r}"
        );
    }
}
