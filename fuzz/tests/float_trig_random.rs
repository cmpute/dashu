//! Differential / fuzz tests for dashu-float transcendentals against `rug::Float` (MPFR).
//!
//! The broad random differentials (sin/cos/tan/atan2/asin/acos) are proptest-driven so a mismatch
//! shrinks to a minimal counterexample; the inherently-sweep tests (π over precision, asin near 1,
//! the pinned large-exponent tan regression) stay as deterministic loops. All are `#[ignore]`d and
//! run manually before a release.
//!
//! Run with: `cargo test --manifest-path fuzz/Cargo.toml --test float_trig_random -- --ignored --nocapture`

use core::str::FromStr;
use dashu::float::ops::Abs;
use dashu::float::round::mode::HalfAway;
use dashu::float::{Context, DBig, FpError, Repr};
use dashu::integer::IBig;
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
    fn fbig_sin_fuzz(x in fuzz::dbig_strategy(-50..=50)) {
        let x_str = format!("{x:e}");
        for prec in fuzz::sampled_precisions_decimal(fuzz::case_key(&[&x])) {
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
    fn fbig_cos_fuzz(x in fuzz::dbig_strategy(-50..=50)) {
        let x_str = format!("{x:e}");
        for prec in fuzz::sampled_precisions_decimal(fuzz::case_key(&[&x])) {
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
    fn fbig_tan_fuzz(x in fuzz::dbig_strategy(-50..=50)) {
        let x_str = format!("{x:e}");
        for prec in fuzz::sampled_precisions_decimal(fuzz::case_key(&[&x])) {
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
    fn fbig_atan2_fuzz(y in fuzz::dbig_strategy(-50..=50), x in fuzz::dbig_strategy(-50..=50)) {
        // `+` flag so the sign of `±0` round-trips into rug — `atan2(±0, x<0) = ±π` is the one
        // trig result whose magnitude flips on the sign of a zero input (sin/cos/tan of `±0`
        // differ only by a zero magnitude, so they need no special handling).
        let y_str = format!("{y:+e}");
        let x_str = format!("{x:+e}");
        for prec in fuzz::sampled_precisions_decimal(fuzz::case_key(&[&y, &x])) {
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
    fn fbig_inv_trig_fuzz(x in fuzz::unit_dbig()) {
        let x_str = format!("{x:e}");
        for prec in fuzz::sampled_precisions_decimal(fuzz::case_key(&[&x])) {
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

    /// sin_pi(x) ≈ MPFR sin_pi(x) across the decimal precision sweep.
    #[test]
    #[ignore]
    fn fbig_sin_pi_fuzz(x in fuzz::dbig_strategy(-50..=50)) {
        let x_str = format!("{x:e}");
        for prec in fuzz::fuzz_precisions_decimal() {
            let ctx = Context::<HalfAway>::new(prec);
            let sin_d = ctx.sin_pi::<10>(x.repr(), None).unwrap().value();
            let bits = rug_bits(x.repr(), prec);
            let x_rug = match Float::parse(&x_str) {
                Ok(p) => Float::with_val(bits, p),
                Err(_) => return Ok(()),
            };
            let sin_r = x_rug.sin_pi();
            let s_r: DBig = DBig::from_str(&sin_r.to_string_radix(10, Some(prec))).unwrap();
            prop_assert!(
                (sin_d.clone() - s_r).abs() <= tol(prec),
                "sin_pi mismatch x={x_str} prec={prec}: dashu={sin_d} rug={sin_r}"
            );
        }
    }

    /// cos_pi(x) ≈ MPFR cos_pi(x) across the decimal precision sweep.
    #[test]
    #[ignore]
    fn fbig_cos_pi_fuzz(x in fuzz::dbig_strategy(-50..=50)) {
        let x_str = format!("{x:e}");
        for prec in fuzz::fuzz_precisions_decimal() {
            let ctx = Context::<HalfAway>::new(prec);
            let cos_d = ctx.cos_pi::<10>(x.repr(), None).unwrap().value();
            let bits = rug_bits(x.repr(), prec);
            let x_rug = match Float::parse(&x_str) {
                Ok(p) => Float::with_val(bits, p),
                Err(_) => return Ok(()),
            };
            let cos_r = x_rug.cos_pi();
            let c_r: DBig = DBig::from_str(&cos_r.to_string_radix(10, Some(prec))).unwrap();
            prop_assert!(
                (cos_d.clone() - c_r).abs() <= tol(prec),
                "cos_pi mismatch x={x_str} prec={prec}: dashu={cos_d} rug={cos_r}"
            );
        }
    }

    /// tan_pi(x) ≈ MPFR tan_pi(x), skipping arguments where |cos_pi(x)| < 1e-5 (too close to a
    /// pole). The exact poles (odd half-integers) are `Err(Indeterminate)` on our side and ±∞ on
    /// MPFR's — that cross-convention is pinned in `fbig_pi_lattice_fuzz` below.
    #[test]
    #[ignore]
    fn fbig_tan_pi_fuzz(x in fuzz::dbig_strategy(-50..=50)) {
        let x_str = format!("{x:e}");
        for prec in fuzz::fuzz_precisions_decimal() {
            let ctx = Context::<HalfAway>::new(prec);
            let cos_d = ctx.cos_pi::<10>(x.repr(), None).unwrap().value();
            if cos_d.abs() <= DBig::from_parts(1.into(), -5) {
                continue; // near a pole — tan is ill-conditioned, skip this precision
            }
            let tan_d = match ctx.tan_pi::<10>(x.repr(), None) {
                Ok(v) => v.value(),
                Err(_) => continue, // exact pole (convention pinned in the lattice test)
            };
            let bits = rug_bits(x.repr(), prec);
            let x_rug = match Float::parse(&x_str) {
                Ok(p) => Float::with_val(bits, p),
                Err(_) => return Ok(()),
            };
            let tan_r = x_rug.tan_pi();
            let t_r: DBig = DBig::from_str(&tan_r.to_string_radix(10, Some(prec))).unwrap();
            prop_assert!(
                (tan_d.clone() - t_r).abs() <= tol(prec),
                "tan_pi mismatch x={x_str} prec={prec}: dashu={tan_d} rug={tan_r}"
            );
        }
    }

    /// `sin_cos_pi` agrees exactly with the separate `sin_pi`/`cos_pi` evaluations (the shared
    /// Ziv certification must produce identical values), across the decimal precision sweep.
    #[test]
    #[ignore]
    fn fbig_sin_cos_pi_fuzz(x in fuzz::dbig_strategy(-50..=50)) {
        for prec in fuzz::fuzz_precisions_decimal() {
            let ctx = Context::<HalfAway>::new(prec);
            let (s, c) = ctx.sin_cos_pi::<10>(x.repr(), None);
            let s = s.unwrap().value();
            let c = c.unwrap().value();
            let s_sep = ctx.sin_pi::<10>(x.repr(), None).unwrap().value();
            let c_sep = ctx.cos_pi::<10>(x.repr(), None).unwrap().value();
            prop_assert!(s == s_sep, "sin_cos_pi sin disagrees at prec={prec}, x={x:e}");
            prop_assert!(c == c_sep, "sin_cos_pi cos disagrees at prec={prec}, x={x:e}");
        }
    }

    /// The ×u forward family ≈ MPFR's `sin_u`/`cos_u`/`tan_u`, across a u sweep with shared
    /// factors (u % 3 == 0 hits the ±1/2 sixth rows), coprime u (7, 11, 13 — no exact rows
    /// beyond the axes), and the degrees case (360).
    #[test]
    #[ignore]
    fn fbig_sin_unit_fuzz(x in fuzz::dbig_strategy(-50..=50), u in prop::sample::select(vec![1u32, 2, 3, 4, 6, 7, 8, 11, 12, 13, 24, 360])) {
        let x_str = format!("{x:e}");
        for prec in fuzz::fuzz_precisions_decimal() {
            let ctx = Context::<HalfAway>::new(prec);
            let bits = rug_bits(x.repr(), prec);
            let x_rug = match Float::parse(&x_str) {
                Ok(p) => Float::with_val(bits, p),
                Err(_) => return Ok(()),
            };

            let d = ctx.sin_unit::<10>(x.repr(), u as usize, None).unwrap().value();
            let r = x_rug.clone().sin_u(u);
            let r_d: DBig = DBig::from_str(&r.to_string_radix(10, Some(prec))).unwrap();
            prop_assert!(
                (d.clone() - r_d).abs() <= tol(prec),
                "sin_unit u={u} x={x_str} prec={prec}: dashu={d} rug={r}"
            );

            let d = ctx.cos_unit::<10>(x.repr(), u as usize, None).unwrap().value();
            let r = x_rug.cos_u(u);
            let r_d: DBig = DBig::from_str(&r.to_string_radix(10, Some(prec))).unwrap();
            prop_assert!(
                (d.clone() - r_d).abs() <= tol(prec),
                "cos_unit u={u} x={x_str} prec={prec}: dashu={d} rug={r}"
            );

            // tan: the poles (odd multiples of u/4) are Err(Indeterminate) here and ±∞ at
            // MPFR's — skip those; also skip near-poles like the tan fuzz above
            let c = ctx.cos_unit::<10>(x.repr(), u as usize, None).unwrap().value();
            if c.abs() > DBig::from_parts(1.into(), -5) {
                let d = match ctx.tan_unit::<10>(x.repr(), u as usize, None) {
                    Ok(v) => v.value(),
                    Err(_) => continue, // exact pole (cross-convention pinned in the lattice test)
                };
                let r = Float::parse(&x_str).map(|p| Float::with_val(bits, p)).unwrap().tan_u(u);
                let r_d: DBig = DBig::from_str(&r.to_string_radix(10, Some(prec))).unwrap();
                prop_assert!(
                    (d.clone() - r_d).abs() <= tol(prec),
                    "tan_unit u={u} x={x_str} prec={prec}: dashu={d} rug={r}"
                );
            }

            // sin_cos_unit agrees exactly with the separate evaluations
            let (s, c) = ctx.sin_cos_unit::<10>(x.repr(), u as usize, None);
            let s = s.unwrap().value();
            let c = c.unwrap().value();
            prop_assert!(s == ctx.sin_unit::<10>(x.repr(), u as usize, None).unwrap().value());
            prop_assert!(c == ctx.cos_unit::<10>(x.repr(), u as usize, None).unwrap().value());
        }
    }

    /// The inverse ×u family ≈ MPFR's `asin_u`/`acos_u`/`atan_u` across the same u sweep.
    /// Out-of-domain inputs (|x| > 1) are NaN at MPFR's and errors on ours — skipped.
    #[test]
    #[ignore]
    fn fbig_asin_unit_fuzz(x in fuzz::unit_dbig(), u in prop::sample::select(vec![1u32, 2, 3, 4, 6, 7, 8, 11, 12, 13, 24, 360])) {
        let x_str = format!("{x:e}");
        for prec in fuzz::fuzz_precisions_decimal() {
            let ctx = Context::<HalfAway>::new(prec);
            let bits = rug_bits(x.repr(), prec);
            let x_rug = match Float::parse(&x_str) {
                Ok(p) => Float::with_val(bits, p),
                Err(_) => return Ok(()),
            };

            let d = ctx.asin_unit::<10>(x.repr(), u as usize, None).unwrap().value();
            let r = x_rug.clone().asin_u(u);
            let r_d: DBig = DBig::from_str(&r.to_string_radix(10, Some(prec))).unwrap();
            prop_assert!(
                (d.clone() - r_d).abs() <= tol(prec),
                "asin_unit u={u} x={x_str} prec={prec}: dashu={d} rug={r}"
            );

            let d = ctx.acos_unit::<10>(x.repr(), u as usize, None).unwrap().value();
            let r = x_rug.acos_u(u);
            let r_d: DBig = DBig::from_str(&r.to_string_radix(10, Some(prec))).unwrap();
            prop_assert!(
                (d.clone() - r_d).abs() <= tol(prec),
                "acos_unit u={u} x={x_str} prec={prec}: dashu={d} rug={r}"
            );
        }
    }

    /// `atan_unit`/`atan2_unit` ≈ MPFR's `atan_u`/`atan2_u`.
    #[test]
    #[ignore]
    fn fbig_atan_unit_fuzz(x in fuzz::dbig_strategy(-50..=50), y in fuzz::dbig_strategy(-50..=50), u in prop::sample::select(vec![1u32, 2, 3, 7, 12, 360])) {
        let x_str = format!("{x:e}");
        let y_str = format!("{y:e}");
        for prec in fuzz::fuzz_precisions_decimal() {
            let ctx = Context::<HalfAway>::new(prec);
            let bits = (rug_bits(x.repr(), prec)).max(rug_bits(y.repr(), prec));
            let x_rug = match Float::parse(&x_str) {
                Ok(p) => Float::with_val(bits, p),
                Err(_) => return Ok(()),
            };

            let d = ctx.atan_unit::<10>(x.repr(), u as usize, None).unwrap().value();
            let r = x_rug.clone().atan_u(u);
            let r_d: DBig = DBig::from_str(&r.to_string_radix(10, Some(prec))).unwrap();
            prop_assert!(
                (d.clone() - r_d).abs() <= tol(prec),
                "atan_unit u={u} x={x_str} prec={prec}: dashu={d} rug={r}"
            );

            // atan2: skip the (0, 0) indeterminate and the axis/diagonal exact rows are
            // covered by the unit tests — here the general path
            let y_d = ctx.tan_unit::<10>(y.repr(), u as usize, None);
            let _ = y_d;
            let y_rug = Float::with_val(bits, Float::parse(&y_str).unwrap());
            let d = match ctx.atan2_unit::<10>(y.repr(), x.repr(), u as usize, None) {
                Ok(v) => v.value(),
                Err(_) => continue, // (0, 0)
            };
            let r = y_rug.atan2_u(&x_rug, u);
            let r_d: DBig = DBig::from_str(&r.to_string_radix(10, Some(prec))).unwrap();
            prop_assert!(
                (d.clone() - r_d).abs() <= tol(prec),
                "atan2_unit u={u} y={y_str} x={x_str} prec={prec}: dashu={d} rug={r}"
            );
        }
    }

    /// sinh_pi(x)/cosh_pi(x) ≈ MPFR sinh/cosh(x·π) — the hyperbolic ×π pair has no direct MPFR
    /// counterpart, so the reference pre-multiplies a full-precision π. Overflows (|x| ≳ 10¹⁴)
    /// saturate to ±∞/∞ on both sides and are skipped.
    #[test]
    #[ignore]
    fn fbig_sinh_cosh_pi_fuzz(x in fuzz::dbig_strategy(-50..=50)) {
        let x_str = format!("{x:e}");
        for prec in fuzz::fuzz_precisions_decimal() {
            let ctx = Context::<HalfAway>::new(prec);
            let sinh_d = match ctx.sinh_pi::<10>(x.repr(), None) {
                Ok(v) => v.value(),
                Err(_) => continue, // overflow → ±∞ (saturated at the convenience layer)
            };
            let bits = rug_bits(x.repr(), prec);
            let x_rug = match Float::parse(&x_str) {
                Ok(p) => Float::with_val(bits, p),
                Err(_) => return Ok(()),
            };
            let arg = x_rug * Float::with_val(bits, rug::float::Constant::Pi);
            let sinh_r = arg.clone().sinh();
            let cosh_r = arg.cosh();
            // `inf`/`nan` don't parse as DBig — the reference overflowed at this precision;
            // dashu holds the huge value finitely (its exponent range is wider), so there is
            // nothing bit-comparable left: skip this precision.
            let s_r = match DBig::from_str(&sinh_r.to_string_radix(10, Some(prec))) {
                Ok(v) => v,
                Err(_) => continue,
            };
            prop_assert!(
                (sinh_d.clone() - s_r).abs() <= tol(prec),
                "sinh_pi mismatch x={x_str} prec={prec}: dashu={sinh_d} rug={sinh_r}"
            );

            let cosh_d = match ctx.cosh_pi::<10>(x.repr(), None) {
                Ok(v) => v.value(),
                Err(_) => continue,
            };
            let c_r = match DBig::from_str(&cosh_r.to_string_radix(10, Some(prec))) {
                Ok(v) => v,
                Err(_) => continue,
            };
            prop_assert!(
                (cosh_d.clone() - c_r).abs() <= tol(prec),
                "cosh_pi mismatch x={x_str} prec={prec}: dashu={cosh_d} rug={cosh_r}"
            );
        }
    }
}

/// The exact-case lattice vs MPFR across u: for x = v/8 (v an integer) every forward result
/// hits a table row (`8x/u` integral) or stays general; for x = v (integers) the u % 3 == 0
/// sixth rows appear. MPFR returns the same exact values, and the tan poles are ±∞ on its
/// side vs `Err(Indeterminate)` on ours. The u = 2 rows pin the ×π family.
#[test]
#[ignore]
fn fbig_pi_lattice_fuzz() {
    for u in [1u32, 2, 3, 4, 5, 6, 7, 8, 10, 12, 24, 360] {
        // x = v/8 (exact in decimal as 125v·10^-6) covers the eighth/quarter lattice; the
        // integer lattice (v·10^0) reaches the u % 3 == 0 sixth rows
        for lattice in [(-96i64..=96, -3isize), (-24i64..=24, 0isize)] {
            let (range, exp) = lattice;
            for v in range {
                let sig = if exp == -3 {
                    IBig::from(125 * v)
                } else {
                    IBig::from(v)
                };
                let x = DBig::from_parts(sig, exp);
                let x_str = format!("{x:e}");
                for &prec in &[20usize, 50] {
                    let ctx = Context::<HalfAway>::new(prec);
                    let bits = rug_bits(x.repr(), prec);
                    let mk_rug = || Float::with_val(bits, Float::parse(&x_str).unwrap());

                    for (name, d, r) in [
                        (
                            "sin",
                            ctx.sin_unit::<10>(x.repr(), u as usize, None)
                                .unwrap()
                                .value(),
                            mk_rug().sin_u(u),
                        ),
                        (
                            "cos",
                            ctx.cos_unit::<10>(x.repr(), u as usize, None)
                                .unwrap()
                                .value(),
                            mk_rug().cos_u(u),
                        ),
                    ] {
                        let r_d: DBig = DBig::from_str(&r.to_string_radix(10, Some(prec))).unwrap();
                        assert!(
                            (d.clone() - r_d).abs() <= tol(prec),
                            "lattice {name}_unit u={u} v={v}e{exp} prec={prec}: dashu={d} rug={r}"
                        );
                    }

                    // tan: at the poles (8x/u ≡ 2 or 6 mod 8) MPFR reports ±∞ and we report
                    // Indeterminate — pin the cross-convention; elsewhere compare numerically.
                    match ctx.tan_unit::<10>(x.repr(), u as usize, None) {
                        Err(FpError::Indeterminate) => {
                            let t_rug = mk_rug().tan_u(u);
                            assert!(
                                t_rug.is_infinite(),
                                "MPFR tan_u at pole u={u} v={v}e{exp} must be ±∞"
                            );
                        }
                        Ok(t) => {
                            let t_rug = mk_rug().tan_u(u);
                            let t_r: DBig =
                                DBig::from_str(&t_rug.to_string_radix(10, Some(prec))).unwrap();
                            let t = t.value();
                            assert!(
                                (t.clone() - t_r).abs() <= tol(prec),
                                "lattice tan_unit u={u} v={v}e{exp} prec={prec}: dashu={t} rug={t_rug}"
                            );
                        }
                        Err(e) => panic!("unexpected tan_unit error at u={u} v={v}: {e:?}"),
                    }
                }
            }
        }
    }
}

/// π at every precision matches MPFR's π to within 1 ulp. (Deterministic precision sweep.)
#[test]
#[ignore]
fn fbig_pi_fuzz() {
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
fn fbig_asin_near_one_fuzz() {
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
fn fbig_tan_large_exponent_regression() {
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
