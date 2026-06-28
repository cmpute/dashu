//! Differential / fuzz tests for dashu-float's non-trig transcendentals against `rug::Float` (MPFR).
//!
//! Companion to `trig_random.rs` (which covers sin/cos/tan/atan2/asin/acos/π). Here: exp, exp_m1,
//! ln, ln_1p, sqrt, cbrt, nth_root, hypot, atan, powf, powi, sinh, cosh, tanh, asinh, acosh, atanh.
//! Proptest-driven so a mismatch shrinks to a minimal counterexample; all `#[ignore]`d (manual,
//! release-time — they link `rug` and run long). Tolerance is `within_k_ulps(2)`: dashu is
//! near-correctly-rounded (guard digits), MPFR is Ziv-correct, so a ≤1-ulp divergence is legitimate
//! and `k=2` leaves margin; a >2-ulp divergence is a real bug to investigate.
//!
//! Run with: `cargo test --manifest-path fuzz/Cargo.toml --test transcendental -- --ignored --nocapture`

use core::str::FromStr;
use dashu::float::ops::Abs;
use dashu::float::round::mode::HalfAway;
use dashu::float::{Context, DBig, Repr};
use dashu::integer::IBig;
use proptest::prelude::*;
use rug::Float;
use rug::ops::Pow;

/// MPFR working precision (bits) sufficient to hold `x` and a `prec`-digit result with margin.
fn rug_bits(x: &Repr<10>, prec: usize) -> u32 {
    let x_mag = (x.exponent().unsigned_abs() + x.digits()) as f64;
    let x_bits = (x_mag * 3.322).ceil() as u32 + 500;
    let p_bits = ((prec.max(100) as f64) * 3.322).ceil() as u32;
    p_bits + x_bits
}

/// |dashu - rug| ≤ `k` ulps at dashu's precision.
fn within_k_ulps(d: &DBig, r: &DBig, k: i32) -> bool {
    let diff = (d.clone() - r).abs();
    // Exact agreement → no need to inspect ulps (also avoids .ulp() on
    // unlimited-precision constants like `FBig::ONE` from powi(x,0)=1).
    if diff.repr().significand().is_zero() {
        return true;
    }
    diff <= d.ulp() * k
}

/// Unwrap a `FpResult<FBig>` to its `FBig` value, or skip the whole case (`return Ok(())`) on error.
macro_rules! dashu_ok {
    ($e:expr) => {
        match $e {
            Ok(v) => v.value(),
            Err(_) => return Ok(()),
        }
    };
}

/// `x ∈ [-100, 100]` as `n/100` — bounded magnitude for exp/exp_m1/sinh/cosh/tanh so the result
/// doesn't overflow to infinity (which would skip the comparison).
fn small_x() -> impl Strategy<Value = DBig> {
    (-10000i32..=10000)
        .prop_map(|n| DBig::from_repr(Repr::<10>::new(n.into(), -2), Context::<HalfAway>::new(0)))
}

/// `x ∈ (lo, lo + 100]` — for ln_1p (lo = -1) so the domain `x > -1` holds.
fn small_x_above(lo: i32) -> impl Strategy<Value = DBig> {
    let lo = lo * 100;
    (1 + lo..=10000 + lo)
        .prop_map(|n| DBig::from_repr(Repr::<10>::new(n.into(), -2), Context::<HalfAway>::new(0)))
}

fn rug_at(x_str: &str, bits: u32) -> Option<Float> {
    match Float::parse(x_str) {
        Ok(p) => Some(Float::with_val(bits, p)),
        Err(_) => None,
    }
}

proptest! {
    #![proptest_config(fuzz::fuzz_config())]

    /// exp(x) ≈ MPFR exp(x).
    #[test]
    #[ignore]
    fn exp_fuzz(x in small_x()) {
        let xs = format!("{x:e}");
        for prec in [20usize, 50, 100] {
            let ctx = Context::<HalfAway>::new(prec);
            let d = dashu_ok!(ctx.exp::<10>(x.repr(), None));
            if d.repr().is_infinite() { continue; }
            let xr = rug_at(&xs, rug_bits(x.repr(), prec)).unwrap();
            let r: DBig = DBig::from_str(&xr.exp().to_string_radix(10, Some(prec))).unwrap();
            prop_assert!(within_k_ulps(&d, &r, 2), "exp x={xs} prec={prec}: dashu={d} rug={r}");
        }
    }


    /// exp(x) − 1 ≈ MPFR exp_m1 (cancellation-free near zero).
    #[test]
    #[ignore]
    fn exp_m1_fuzz(x in small_x()) {
        let xs = format!("{x:e}");
        for prec in [20usize, 50, 100] {
            let ctx = Context::<HalfAway>::new(prec);
            let d = dashu_ok!(ctx.exp_m1::<10>(x.repr(), None));
            if d.repr().is_infinite() { continue; }
            let xr = rug_at(&xs, rug_bits(x.repr(), prec)).unwrap();
            let r: DBig = DBig::from_str(&xr.exp_m1().to_string_radix(10, Some(prec))).unwrap();
            prop_assert!(within_k_ulps(&d, &r, 2), "exp_m1 x={xs} prec={prec}: dashu={d} rug={r}");
        }
    }


    /// ln(x) ≈ MPFR ln(x), x > 0.
    #[test]
    #[ignore]
    fn ln_fuzz(x in fuzz::pos_dbig_strategy(-50..=50)) {
        let xs = format!("{x:e}");
        for prec in [20usize, 50, 100] {
            let ctx = Context::<HalfAway>::new(prec);
            let d = dashu_ok!(ctx.ln::<10>(x.repr(), None));
            if d.repr().is_infinite() { continue; }
            let xr = rug_at(&xs, rug_bits(x.repr(), prec)).unwrap();
            let r: DBig = DBig::from_str(&xr.ln().to_string_radix(10, Some(prec))).unwrap();
            prop_assert!(within_k_ulps(&d, &r, 2), "ln x={xs} prec={prec}: dashu={d} rug={r}");
        }
    }


    /// ln(1 + x) ≈ MPFR ln_1p, x > −1.
    #[test]
    #[ignore]
    fn ln_1p_fuzz(x in small_x_above(-1)) {
        let xs = format!("{x:e}");
        for prec in [20usize, 50, 100] {
            let ctx = Context::<HalfAway>::new(prec);
            let d = dashu_ok!(ctx.ln_1p::<10>(x.repr(), None));
            if d.repr().is_infinite() { continue; }
            let xr = rug_at(&xs, rug_bits(x.repr(), prec)).unwrap();
            let r: DBig = DBig::from_str(&xr.ln_1p().to_string_radix(10, Some(prec))).unwrap();
            prop_assert!(within_k_ulps(&d, &r, 2), "ln_1p x={xs} prec={prec}: dashu={d} rug={r}");
        }
    }


    /// sqrt(x) ≈ MPFR sqrt(x), x ≥ 0.
    #[test]
    #[ignore]
    fn sqrt_fuzz(x in fuzz::pos_dbig_strategy(-50..=50)) {
        let xs = format!("{x:e}");
        for prec in [20usize, 50, 100] {
            let ctx = Context::<HalfAway>::new(prec);
            let d = dashu_ok!(ctx.sqrt::<10>(x.repr()));
            let xr = rug_at(&xs, rug_bits(x.repr(), prec)).unwrap();
            let r: DBig = DBig::from_str(&xr.sqrt().to_string_radix(10, Some(prec))).unwrap();
            prop_assert!(within_k_ulps(&d, &r, 2), "sqrt x={xs} prec={prec}: dashu={d} rug={r}");
        }
    }


    /// cbrt(x) ≈ MPFR cbrt(x), all real.
    #[test]
    #[ignore]
    fn cbrt_fuzz(x in fuzz::dbig_strategy(-50..=50)) {
        let xs = format!("{x:e}");
        for prec in [20usize, 50, 100] {
            let ctx = Context::<HalfAway>::new(prec);
            let d = dashu_ok!(ctx.cbrt::<10>(x.repr()));
            let xr = rug_at(&xs, rug_bits(x.repr(), prec)).unwrap();
            let r: DBig = DBig::from_str(&xr.cbrt().to_string_radix(10, Some(prec))).unwrap();
            prop_assert!(within_k_ulps(&d, &r, 2), "cbrt x={xs} prec={prec}: dashu={d} rug={r}");
        }
    }


    /// nth_root(n, x) ≈ MPFR root(n), x > 0, n ∈ 2..=6.
    #[test]
    #[ignore]
    fn nth_root_fuzz(x in fuzz::pos_dbig_strategy(-50..=50), n in 2u32..=6) {
        let xs = format!("{x:e}");
        for prec in [20usize, 50, 100] {
            let ctx = Context::<HalfAway>::new(prec);
            let d = dashu_ok!(ctx.nth_root::<10>(n as usize, x.repr()));
            let xr = rug_at(&xs, rug_bits(x.repr(), prec)).unwrap();
            let r: DBig = DBig::from_str(&xr.root(n).to_string_radix(10, Some(prec))).unwrap();
            prop_assert!(within_k_ulps(&d, &r, 2), "nth_root n={n} x={xs} prec={prec}: dashu={d} rug={r}");
        }
    }


    /// hypot(a, b) = sqrt(a² + b²) ≈ MPFR (computed as such; inputs bounded so no overflow).
    #[test]
    #[ignore]
    fn hypot_fuzz(a in small_x(), b in small_x()) {
        let (as_, bs) = (format!("{a:e}"), format!("{b:e}"));
        for prec in [20usize, 50, 100] {
            let ctx = Context::<HalfAway>::new(prec);
            let d = dashu_ok!(ctx.hypot::<10>(a.repr(), b.repr()));
            let bits = rug_bits(a.repr(), prec).max(rug_bits(b.repr(), prec));
            let ar = rug_at(&as_, bits).unwrap();
            let br = rug_at(&bs, bits).unwrap();
            let hr = (ar.pow(2u32) + br.pow(2u32)).sqrt();
            let r: DBig = DBig::from_str(&hr.to_string_radix(10, Some(prec))).unwrap();
            prop_assert!(within_k_ulps(&d, &r, 2), "hypot a={as_} b={bs} prec={prec}: dashu={d} rug={r}");
        }
    }


    /// atan(x) ≈ MPFR atan(x), all real.
    #[test]
    #[ignore]
    fn atan_fuzz(x in fuzz::dbig_strategy(-50..=50)) {
        let xs = format!("{x:e}");
        for prec in [20usize, 50, 100] {
            let ctx = Context::<HalfAway>::new(prec);
            let d = dashu_ok!(ctx.atan::<10>(x.repr(), None));
            let xr = rug_at(&xs, rug_bits(x.repr(), prec)).unwrap();
            let r: DBig = DBig::from_str(&xr.atan().to_string_radix(10, Some(prec))).unwrap();
            prop_assert!(within_k_ulps(&d, &r, 2), "atan x={xs} prec={prec}: dashu={d} rug={r}");
        }
    }


    /// base^exp ≈ MPFR pow, base > 0.
    #[test]
    #[ignore]
    fn powf_fuzz(base in fuzz::pos_dbig_strategy(-5..=5), exp in small_x()) {
        let (bs, es) = (format!("{base:e}"), format!("{exp:e}"));
        for prec in [20usize, 50, 100] {
            let ctx = Context::<HalfAway>::new(prec);
            let d = dashu_ok!(ctx.powf::<10>(base.repr(), exp.repr(), None));
            if d.repr().is_infinite() { continue; }
            let bits = rug_bits(base.repr(), prec).max(rug_bits(exp.repr(), prec));
            let br = rug_at(&bs, bits).unwrap();
            let er = rug_at(&es, bits).unwrap();
            let r: DBig = DBig::from_str(&br.pow(&er).to_string_radix(10, Some(prec))).unwrap();
            prop_assert!(within_k_ulps(&d, &r, 2), "powf base={bs} exp={es} prec={prec}: dashu={d} rug={r}");
        }
    }


    /// base^n ≈ MPFR pow(n), n ∈ 0..=16 (rug takes u32).
    #[test]
    #[ignore]
    fn powi_fuzz(base in fuzz::dbig_strategy(-20..=20), n in 0u32..=16) {
        let bs = format!("{base:e}");
        for prec in [20usize, 50, 100] {
            let ctx = Context::<HalfAway>::new(prec);
            let d = dashu_ok!(ctx.powi::<10>(base.repr(), IBig::from(n)));
            if d.repr().is_infinite() { continue; }
            let br = rug_at(&bs, rug_bits(base.repr(), prec)).unwrap();
            let r: DBig = DBig::from_str(&br.pow(n).to_string_radix(10, Some(prec))).unwrap();
            prop_assert!(within_k_ulps(&d, &r, 2), "powi base={bs} n={n} prec={prec}: dashu={d} rug={r}");
        }
    }


    /// sinh(x) ≈ MPFR sinh(x).
    #[test]
    #[ignore]
    fn sinh_fuzz(x in small_x()) {
        let xs = format!("{x:e}");
        for prec in [20usize, 50, 100] {
            let ctx = Context::<HalfAway>::new(prec);
            let d = dashu_ok!(ctx.sinh::<10>(x.repr(), None));
            if d.repr().is_infinite() { continue; }
            let xr = rug_at(&xs, rug_bits(x.repr(), prec)).unwrap();
            let r: DBig = DBig::from_str(&xr.sinh().to_string_radix(10, Some(prec))).unwrap();
            prop_assert!(within_k_ulps(&d, &r, 2), "sinh x={xs} prec={prec}: dashu={d} rug={r}");
        }
    }


    /// cosh(x) ≈ MPFR cosh(x).
    #[test]
    #[ignore]
    fn cosh_fuzz(x in small_x()) {
        let xs = format!("{x:e}");
        for prec in [20usize, 50, 100] {
            let ctx = Context::<HalfAway>::new(prec);
            let d = dashu_ok!(ctx.cosh::<10>(x.repr(), None));
            if d.repr().is_infinite() { continue; }
            let xr = rug_at(&xs, rug_bits(x.repr(), prec)).unwrap();
            let r: DBig = DBig::from_str(&xr.cosh().to_string_radix(10, Some(prec))).unwrap();
            prop_assert!(within_k_ulps(&d, &r, 2), "cosh x={xs} prec={prec}: dashu={d} rug={r}");
        }
    }


    /// tanh(x) ≈ MPFR tanh(x).
    #[test]
    #[ignore]
    fn tanh_fuzz(x in small_x()) {
        let xs = format!("{x:e}");
        for prec in [20usize, 50, 100] {
            let ctx = Context::<HalfAway>::new(prec);
            let d = dashu_ok!(ctx.tanh::<10>(x.repr(), None));
            let xr = rug_at(&xs, rug_bits(x.repr(), prec)).unwrap();
            let r: DBig = DBig::from_str(&xr.tanh().to_string_radix(10, Some(prec))).unwrap();
            prop_assert!(within_k_ulps(&d, &r, 2), "tanh x={xs} prec={prec}: dashu={d} rug={r}");
        }
    }


    /// asinh(x) ≈ MPFR asinh(x), all real.
    #[test]
    #[ignore]
    fn asinh_fuzz(x in fuzz::dbig_strategy(-50..=50)) {
        let xs = format!("{x:e}");
        for prec in [20usize, 50, 100] {
            let ctx = Context::<HalfAway>::new(prec);
            let d = dashu_ok!(ctx.asinh::<10>(x.repr(), None));
            let xr = rug_at(&xs, rug_bits(x.repr(), prec)).unwrap();
            let r: DBig = DBig::from_str(&xr.asinh().to_string_radix(10, Some(prec))).unwrap();
            prop_assert!(within_k_ulps(&d, &r, 2), "asinh x={xs} prec={prec}: dashu={d} rug={r}");
        }
    }


    /// acosh(x) ≈ MPFR acosh(x), x ≥ 1 (pos_dbig_strategy(0..=50) keeps x ≥ 1).
    #[test]
    #[ignore]
    fn acosh_fuzz(x in fuzz::pos_dbig_strategy(0..=50)) {
        let xs = format!("{x:e}");
        for prec in [20usize, 50, 100] {
            let ctx = Context::<HalfAway>::new(prec);
            let d = dashu_ok!(ctx.acosh::<10>(x.repr(), None));
            let xr = rug_at(&xs, rug_bits(x.repr(), prec)).unwrap();
            let r: DBig = DBig::from_str(&xr.acosh().to_string_radix(10, Some(prec))).unwrap();
            prop_assert!(within_k_ulps(&d, &r, 2), "acosh x={xs} prec={prec}: dashu={d} rug={r}");
        }
    }


    /// atanh(x) ≈ MPFR atanh(x), |x| < 1 (unit_dbig is [-1,1]; the ±1 endpoints yield ±∞ and are
    /// skipped by the `is_infinite` check inside the loop).
    #[test]
    #[ignore]
    fn atanh_fuzz(x in fuzz::unit_dbig()) {
        let xs = format!("{x:e}");
        for prec in [20usize, 50, 100] {
            let ctx = Context::<HalfAway>::new(prec);
            let d = dashu_ok!(ctx.atanh::<10>(x.repr(), None));
            if d.repr().is_infinite() { continue; }
            let xr = rug_at(&xs, rug_bits(x.repr(), prec)).unwrap();
            let r: DBig = DBig::from_str(&xr.atanh().to_string_radix(10, Some(prec))).unwrap();
            prop_assert!(within_k_ulps(&d, &r, 2), "atanh x={xs} prec={prec}: dashu={d} rug={r}");
        }
    }

}
