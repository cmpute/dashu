//! Differential / fuzz tests for dashu-float's non-trig transcendentals against `rug::Float` (MPFR).
//!
//! Companion to `float_trig_random.rs` (which covers sin/cos/tan/atan2/asin/acos/π). Here: exp, exp_m1,
//! ln, ln_1p, sqrt, cbrt, nth_root, hypot, atan, powf, powi, sinh, cosh, sinh_cosh, tanh, asinh, acosh, atanh.
//! Proptest-driven so a mismatch shrinks to a minimal counterexample; all `#[ignore]`d (manual,
//! release-time — they link `rug` and run long). Tolerance is `within_k_ulps(2)`: dashu is
//! near-correctly-rounded (guard digits), MPFR is Ziv-correct, so a ≤1-ulp divergence is legitimate
//! and `k=2` leaves margin; a >2-ulp divergence is a real bug to investigate.
//!
//! Run with: `cargo test --manifest-path fuzz/Cargo.toml --test float_transcendental -- --ignored --nocapture`

use core::str::FromStr;
use dashu::float::ops::Abs;
use dashu::float::round::Round;
use dashu::float::round::mode::{Down, HalfAway, HalfEven, Up, Zero};
use dashu::float::{Context, DBig, FBig, Repr};
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

/// Round the high-precision MPFR value to `prec` decimal digits under `R`, via dashu's
/// `with_precision` (a trusted, separately-validated rounding primitive) on an unlimited-precision
/// parse of the full decimal expansion. This avoids the double-rounding hazard of formatting a
/// bit-rounded value to decimal digits — exact → digits directly, so a digit-level tie is resolved
/// correctly.
fn round_to_prec<R: Round>(rug_hi: &Float, prec: usize) -> FBig<R, 10> {
    let hi = FBig::<R, 10>::from_str(&rug_hi.to_string_radix(10, None)).unwrap();
    hi.with_precision(prec).value()
}

/// Is dashu's `prec`-digit result under a directed mode the correct rounding of the exact value?
///
/// `up` / `down` are MPFR's upward- and downward-rounded approximations at `rug_bits ≫ prec`
/// digits. Each is rounded to `prec` digits under `R` via [`round_to_prec`]. For a value that is
/// exactly a `prec`-digit number, or exactly at a digit-level tie, the two approximations both
/// land on the same digit value (the value is representable at `rug_bits`), so dashu must match it
/// exactly; in the rare straddle (both give adjacent digits) either is accepted.
fn directed_eq<R: Round>(d: &FBig<R, 10>, up: &Float, down: &Float, prec: usize) -> bool {
    let r_up = round_to_prec::<R>(up, prec);
    let r_down = round_to_prec::<R>(down, prec);
    d.repr() == r_up.repr() || (r_up.repr() != r_down.repr() && d.repr() == r_down.repr())
}

/// Compare `sqrt`'s result within 1 ulp of the correct `prec`-digit rounding. Unlike the other
/// roots, `sqrt` is deliberately **not** Ziv-certified (it uses the integer `sqrt_rem` + a guard
/// adjustment in `root.rs`, so it is near-correct only), so a 1-ulp deviation is legitimate.
fn directed_sqrt_ok<R: Round>(d: &FBig<R, 10>, up: &Float, prec: usize) -> bool {
    let r = round_to_prec::<R>(up, prec);
    let diff = (d.clone() - r).abs();
    diff.repr().significand().is_zero() || diff <= d.ulp()
}

/// Compare one cache-taking transcendental under one directed mode against MPFR's `*_round`
/// (computing both the `Up` and `Down` approximations). Skips (via `continue`) a result that
/// overflows to infinity.
macro_rules! directed_check {
    ($op:ident, $rug_method:ident, $mode:ident, $x:ident, $xr:ident, $prec:ident, $xs:ident) => {
        {
            let d = dashu_ok!(Context::<$mode>::new($prec).$op::<10>($x.repr(), None));
            if d.repr().is_infinite() {
                continue;
            }
            let mut up = $xr.clone();
            up.$rug_method(rug::float::Round::Up);
            let mut down = $xr.clone();
            down.$rug_method(rug::float::Round::Down);
            prop_assert!(
                directed_eq::<$mode>(&d, &up, &down, $prec),
                concat!(stringify!($op), " ", stringify!($mode), " x={} prec={}: dashu={}"),
                $xs,
                $prec,
                d
            );
        }
    };
}

/// The same comparison for `sqrt`, whose context method takes no cache argument.
macro_rules! directed_check_sqrt {
    ($mode:ident, $x:ident, $xr:ident, $prec:ident, $xs:ident) => {
        {
            let d = dashu_ok!(Context::<$mode>::new($prec).sqrt::<10>($x.repr()));
            if d.repr().is_infinite() {
                continue;
            }
            let mut up = $xr.clone();
            up.sqrt_round(rug::float::Round::Up);
            prop_assert!(
                directed_sqrt_ok::<$mode>(&d, &up, $prec),
                concat!("sqrt ", stringify!($mode), " x={} prec={}: dashu={}"),
                $xs,
                $prec,
                d
            );
        }
    };
}

proptest! {
    #![proptest_config(fuzz::fuzz_config())]

    /// exp(x) ≈ MPFR exp(x).
    #[test]
    #[ignore]
    fn fbig_exp_fuzz(x in small_x()) {
        let xs = format!("{x:e}");
        for prec in fuzz::fuzz_precisions_decimal() {
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
    fn fbig_exp_m1_fuzz(x in small_x()) {
        let xs = format!("{x:e}");
        for prec in fuzz::fuzz_precisions_decimal() {
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
    fn fbig_ln_fuzz(x in fuzz::pos_dbig_strategy(-50..=50)) {
        let xs = format!("{x:e}");
        for prec in fuzz::fuzz_precisions_decimal() {
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
    fn fbig_ln_1p_fuzz(x in small_x_above(-1)) {
        let xs = format!("{x:e}");
        for prec in fuzz::fuzz_precisions_decimal() {
            let ctx = Context::<HalfAway>::new(prec);
            let d = dashu_ok!(ctx.ln_1p::<10>(x.repr(), None));
            if d.repr().is_infinite() { continue; }
            let xr = rug_at(&xs, rug_bits(x.repr(), prec)).unwrap();
            let r: DBig = DBig::from_str(&xr.ln_1p().to_string_radix(10, Some(prec))).unwrap();
            prop_assert!(within_k_ulps(&d, &r, 2), "ln_1p x={xs} prec={prec}: dashu={d} rug={r}");
        }
    }


    /// log2(x) == MPFR log2(x), x > 0.
    ///
    /// Unlike the other transcendentals here (which use the lenient `within_k_ulps(2)`) this
    /// asserts bit-exact agreement: dashu's `log2` is Ziv-correct (via the Ball-based pilot) and
    /// MPFR is Ziv-correct, so correctly rounded means identical. For non-power-of-two `x` the
    /// result is irrational, so the HalfAway/nearest-even rounding-mode difference at exact ties
    /// never arises. This is the independent soundness net for the Ball error propagation.
    #[test]
    #[ignore]
    fn fbig_log2_fuzz(x in fuzz::pos_dbig_strategy(-200..=200)) {
        let xs = format!("{x:e}");
        for prec in fuzz::fuzz_precisions_decimal() {
            let ctx = Context::<HalfAway>::new(prec);
            let d = dashu_ok!(ctx.log2::<10>(x.repr(), None));
            if d.repr().is_infinite() { continue; }
            let xr = rug_at(&xs, rug_bits(x.repr(), prec)).unwrap();
            let r: DBig = DBig::from_str(&xr.log2().to_string_radix(10, Some(prec))).unwrap();
            prop_assert!(d == r, "log2 x={xs} prec={prec}: dashu={d} rug={r}");
        }
    }


    /// log10(x) == MPFR log10(x), x > 0. Bit-exact like `log2` (both dashu and MPFR are
    /// Ziv-correct; for non-power-of-ten `x` the result is irrational so the HalfAway/nearest-even
    /// difference at exact ties never arises). Exact powers of ten are covered by the shortcut.
    #[test]
    #[ignore]
    fn fbig_log10_fuzz(x in fuzz::pos_dbig_strategy(-200..=200)) {
        let xs = format!("{x:e}");
        for prec in fuzz::fuzz_precisions_decimal() {
            let ctx = Context::<HalfAway>::new(prec);
            let d = dashu_ok!(ctx.log10::<10>(x.repr(), None));
            if d.repr().is_infinite() { continue; }
            let xr = rug_at(&xs, rug_bits(x.repr(), prec)).unwrap();
            let r: DBig = DBig::from_str(&xr.log10().to_string_radix(10, Some(prec))).unwrap();
            prop_assert!(d == r, "log10 x={xs} prec={prec}: dashu={d} rug={r}");
        }
    }


    /// sqrt(x) ≈ MPFR sqrt(x), x ≥ 0.
    #[test]
    #[ignore]
    fn fbig_sqrt_fuzz(x in fuzz::pos_dbig_strategy(-50..=50)) {
        let xs = format!("{x:e}");
        for prec in fuzz::fuzz_precisions_decimal() {
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
    fn fbig_cbrt_fuzz(x in fuzz::dbig_strategy(-50..=50)) {
        let xs = format!("{x:e}");
        for prec in fuzz::fuzz_precisions_decimal() {
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
    fn fbig_nth_root_fuzz(x in fuzz::pos_dbig_strategy(-50..=50), n in 2u32..=6) {
        let xs = format!("{x:e}");
        for prec in fuzz::fuzz_precisions_decimal() {
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
    fn fbig_hypot_fuzz(a in small_x(), b in small_x()) {
        let (as_, bs) = (format!("{a:e}"), format!("{b:e}"));
        for prec in fuzz::fuzz_precisions_decimal() {
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
    fn fbig_atan_fuzz(x in fuzz::dbig_strategy(-50..=50)) {
        let xs = format!("{x:e}");
        for prec in fuzz::fuzz_precisions_decimal() {
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
    fn fbig_powf_fuzz(base in fuzz::pos_dbig_strategy(-5..=5), exp in small_x()) {
        let (bs, es) = (format!("{base:e}"), format!("{exp:e}"));
        for prec in fuzz::fuzz_precisions_decimal() {
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
    fn fbig_powi_fuzz(base in fuzz::dbig_strategy(-20..=20), n in 0u32..=16) {
        let bs = format!("{base:e}");
        for prec in fuzz::fuzz_precisions_decimal() {
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
    fn fbig_sinh_fuzz(x in small_x()) {
        let xs = format!("{x:e}");
        for prec in fuzz::fuzz_precisions_decimal() {
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
    fn fbig_cosh_fuzz(x in small_x()) {
        let xs = format!("{x:e}");
        for prec in fuzz::fuzz_precisions_decimal() {
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
    fn fbig_tanh_fuzz(x in small_x()) {
        let xs = format!("{x:e}");
        for prec in fuzz::fuzz_precisions_decimal() {
            let ctx = Context::<HalfAway>::new(prec);
            let d = dashu_ok!(ctx.tanh::<10>(x.repr(), None));
            let xr = rug_at(&xs, rug_bits(x.repr(), prec)).unwrap();
            let r: DBig = DBig::from_str(&xr.tanh().to_string_radix(10, Some(prec))).unwrap();
            prop_assert!(within_k_ulps(&d, &r, 2), "tanh x={xs} prec={prec}: dashu={d} rug={r}");
        }
    }


    /// sinh_cosh(x) ≈ (MPFR sinh(x), MPFR cosh(x)).
    #[test]
    #[ignore]
    fn fbig_sinh_cosh_fuzz(x in small_x()) {
        let xs = format!("{x:e}");
        for prec in fuzz::fuzz_precisions_decimal() {
            let ctx = Context::<HalfAway>::new(prec);
            let (ds, dc) = ctx.sinh_cosh::<10>(x.repr(), None);
            let d_sinh = dashu_ok!(ds);
            let d_cosh = dashu_ok!(dc);
            if d_sinh.repr().is_infinite() || d_cosh.repr().is_infinite() { continue; }
            let bits = rug_bits(x.repr(), prec);
            let r_sinh: DBig =
                DBig::from_str(&rug_at(&xs, bits).unwrap().sinh().to_string_radix(10, Some(prec))).unwrap();
            let r_cosh: DBig =
                DBig::from_str(&rug_at(&xs, bits).unwrap().cosh().to_string_radix(10, Some(prec))).unwrap();
            prop_assert!(within_k_ulps(&d_sinh, &r_sinh, 2), "sinh_cosh sinh x={xs} prec={prec}: dashu={d_sinh} rug={r_sinh}");
            prop_assert!(within_k_ulps(&d_cosh, &r_cosh, 2), "sinh_cosh cosh x={xs} prec={prec}: dashu={d_cosh} rug={r_cosh}");
        }
    }


    /// asinh(x) ≈ MPFR asinh(x), all real.
    #[test]
    #[ignore]
    fn fbig_asinh_fuzz(x in fuzz::dbig_strategy(-50..=50)) {
        let xs = format!("{x:e}");
        for prec in fuzz::fuzz_precisions_decimal() {
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
    fn fbig_acosh_fuzz(x in fuzz::pos_dbig_strategy(0..=50)) {
        let xs = format!("{x:e}");
        for prec in fuzz::fuzz_precisions_decimal() {
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
    fn fbig_atanh_fuzz(x in fuzz::unit_dbig()) {
        let xs = format!("{x:e}");
        for prec in fuzz::fuzz_precisions_decimal() {
            let ctx = Context::<HalfAway>::new(prec);
            let d = dashu_ok!(ctx.atanh::<10>(x.repr(), None));
            if d.repr().is_infinite() { continue; }
            let xr = rug_at(&xs, rug_bits(x.repr(), prec)).unwrap();
            let r: DBig = DBig::from_str(&xr.atanh().to_string_radix(10, Some(prec))).unwrap();
            prop_assert!(within_k_ulps(&d, &r, 2), "atanh x={xs} prec={prec}: dashu={d} rug={r}");
        }
    }

    // ---- directed rounding modes (bit-exact vs MPFR under the same mode) ----
    //
    // The HalfAway tests above only exercise the Ziv loop under nearest. These check that the
    // loop certifies correctly under Up / Down / Zero / HalfEven too, compared bit-exactly against
    // MPFR rounded in the matching direction.

    #[test]
    #[ignore]
    fn fbig_exp_directed_fuzz(x in small_x()) {
        let xs = format!("{x:e}");
        for prec in fuzz::fuzz_precisions_decimal() {
            let xr = rug_at(&xs, rug_bits(x.repr(), prec)).unwrap();
            directed_check!(exp, exp_round, Up, x, xr, prec, xs);
            directed_check!(exp, exp_round, Down, x, xr, prec, xs);
            directed_check!(exp, exp_round, Zero, x, xr, prec, xs);
            directed_check!(exp, exp_round, HalfEven, x, xr, prec, xs);
        }
    }

    #[test]
    #[ignore]
    fn fbig_ln_directed_fuzz(x in fuzz::pos_dbig_strategy(-50..=50)) {
        let xs = format!("{x:e}");
        for prec in fuzz::fuzz_precisions_decimal() {
            let xr = rug_at(&xs, rug_bits(x.repr(), prec)).unwrap();
            directed_check!(ln, ln_round, Up, x, xr, prec, xs);
            directed_check!(ln, ln_round, Down, x, xr, prec, xs);
            directed_check!(ln, ln_round, Zero, x, xr, prec, xs);
            directed_check!(ln, ln_round, HalfEven, x, xr, prec, xs);
        }
    }

    #[test]
    #[ignore]
    fn fbig_log2_directed_fuzz(x in fuzz::pos_dbig_strategy(-200..=200)) {
        let xs = format!("{x:e}");
        for prec in fuzz::fuzz_precisions_decimal() {
            let xr = rug_at(&xs, rug_bits(x.repr(), prec)).unwrap();
            directed_check!(log2, log2_round, Up, x, xr, prec, xs);
            directed_check!(log2, log2_round, Down, x, xr, prec, xs);
            directed_check!(log2, log2_round, Zero, x, xr, prec, xs);
            directed_check!(log2, log2_round, HalfEven, x, xr, prec, xs);
        }
    }

    #[test]
    #[ignore]
    fn fbig_log10_directed_fuzz(x in fuzz::pos_dbig_strategy(-200..=200)) {
        let xs = format!("{x:e}");
        for prec in fuzz::fuzz_precisions_decimal() {
            let xr = rug_at(&xs, rug_bits(x.repr(), prec)).unwrap();
            directed_check!(log10, log10_round, Up, x, xr, prec, xs);
            directed_check!(log10, log10_round, Down, x, xr, prec, xs);
            directed_check!(log10, log10_round, Zero, x, xr, prec, xs);
            directed_check!(log10, log10_round, HalfEven, x, xr, prec, xs);
        }
    }

    #[test]
    #[ignore]
    fn fbig_sqrt_directed_fuzz(x in fuzz::pos_dbig_strategy(-50..=50)) {
        let xs = format!("{x:e}");
        for prec in fuzz::fuzz_precisions_decimal() {
            let xr = rug_at(&xs, rug_bits(x.repr(), prec)).unwrap();
            directed_check_sqrt!(Up, x, xr, prec, xs);
            directed_check_sqrt!(Down, x, xr, prec, xs);
            directed_check_sqrt!(Zero, x, xr, prec, xs);
            directed_check_sqrt!(HalfEven, x, xr, prec, xs);
        }
    }

}
