//! Differential / fuzz tests for `dashu-cmplx::CBig` transcendentals against `rug::Complex` (GNU MPC)
//! across the [`fuzz::fuzz_precisions_bits`] sweep (default 50/100/500/1000 bits).
//!
//! Companion to `cmplx_random.rs` (which covers field arithmetic mul/div/sqr). Here: exp, log, sqrt,
//! sin, cos, tan, asin, acos, atan, the hyperbolic sinh/cosh/sinh_cosh/tanh/asinh/acosh/atanh, and
//! powf. rug has direct MPC methods for all of these (no gaps).
//! Reuses the shared `fuzz::cmplx` build/compare helpers (`pair`, `complex_finite`, `close_at`).
//! `close_at` compares per-component to `CLOSE_K × 2^-prec × scale` ulps (not via f64, which would
//! be meaningless above 53 bits). All `#[ignore]`d (manual, release-time). Inputs are modest-magnitude
//! finite `f64` pairs, so results stay finite and `complex_finite` rarely skips.
//!
//! Run with: `cargo test --manifest-path fuzz/Cargo.toml --test cmplx_transcendental -- --ignored --nocapture`
//! (override the precision sweep with `FUZZ_PRECISIONS=53`, case count with `PROPTEST_CASES=N`.)

use dashu::base::Sign;
use dashu::integer::IBig;
use fuzz::cmplx::*;
use proptest::prelude::*;
use rug::ops::Pow;

/// Unwrap a `CfpResult<CBig>` to its `CBig` value, or skip this precision on error (e.g. tan at a
/// zero of cos, powf singularities).
macro_rules! cmplx_ok {
    ($e:expr) => {
        match $e {
            Ok(v) => v.value(),
            Err(_) => continue,
        }
    };
}

/// CBig with both parts ±0 (all four sign combinations) — reaches the Annex-G signed-zero
/// shortcuts that the nonzero `f64_part` strategy never hits.
fn zero_pair() -> impl Strategy<Value = (f64, f64)> {
    (prop::bool::ANY, prop::bool::ANY).prop_map(|(neg_re, neg_im)| {
        (
            if neg_re { -0.0f64 } else { 0.0f64 },
            if neg_im { -0.0f64 } else { 0.0f64 },
        )
    })
}

proptest! {
    #![proptest_config(fuzz::fuzz_config())]

    /// exp(z) ≈ MPC exp(z).
    #[test]
    #[ignore]
    fn cbig_exp_fuzz(zre in f64_part(), zim in f64_part()) {
        for prec in fuzz::fuzz_precisions_bits() {
            let (z, rz) = pair(zre, zim, prec as usize);
            let d = cmplx_ok!(z.context().exp(&z, None));
            let r = rz.exp();
            if !complex_finite(&d, &r) { continue; }
            prop_assert!(close_at(&d, &r, prec as usize), "exp zre={zre} zim={zim} prec={prec}");
        }
    }

    /// log(z) ≈ MPC ln(z).
    #[test]
    #[ignore]
    fn cbig_log_fuzz(zre in f64_part(), zim in f64_part()) {
        for prec in fuzz::fuzz_precisions_bits() {
            let (z, rz) = pair(zre, zim, prec as usize);
            let d = cmplx_ok!(z.context().log(&z, None));
            let r = rz.ln();
            if !complex_finite(&d, &r) { continue; }
            prop_assert!(close_at(&d, &r, prec as usize), "log zre={zre} zim={zim} prec={prec}");
        }
    }

    /// sqrt(z) ≈ MPC sqrt(z).
    #[test]
    #[ignore]
    fn cbig_sqrt_fuzz(zre in f64_part(), zim in f64_part()) {
        for prec in fuzz::fuzz_precisions_bits() {
            let (z, rz) = pair(zre, zim, prec as usize);
            let d = cmplx_ok!(z.context().sqrt(&z));
            let r = rz.sqrt();
            if !complex_finite(&d, &r) { continue; }
            prop_assert!(close_at(&d, &r, prec as usize), "sqrt zre={zre} zim={zim} prec={prec}");
        }
    }

    /// sin(z) ≈ MPC sin(z).
    #[test]
    #[ignore]
    fn cbig_sin_fuzz(zre in f64_part(), zim in f64_part()) {
        for prec in fuzz::fuzz_precisions_bits() {
            let (z, rz) = pair(zre, zim, prec as usize);
            let d = cmplx_ok!(z.context().sin(&z, None));
            let r = rz.sin();
            if !complex_finite(&d, &r) { continue; }
            prop_assert!(close_at(&d, &r, prec as usize), "sin zre={zre} zim={zim} prec={prec}");
        }
    }

    /// cos(z) ≈ MPC cos(z).
    #[test]
    #[ignore]
    fn cbig_cos_fuzz(zre in f64_part(), zim in f64_part()) {
        for prec in fuzz::fuzz_precisions_bits() {
            let (z, rz) = pair(zre, zim, prec as usize);
            let d = cmplx_ok!(z.context().cos(&z, None));
            let r = rz.cos();
            if !complex_finite(&d, &r) { continue; }
            prop_assert!(close_at(&d, &r, prec as usize), "cos zre={zre} zim={zim} prec={prec}");
        }
    }

    /// tan(z) ≈ MPC tan(z) (skips zeros of cos / errored precisions, where tan is singular).
    #[test]
    #[ignore]
    fn cbig_tan_fuzz(zre in f64_part(), zim in f64_part()) {
        for prec in fuzz::fuzz_precisions_bits() {
            let (z, rz) = pair(zre, zim, prec as usize);
            let d = cmplx_ok!(z.context().tan(&z, None));
            let r = rz.tan();
            if !complex_finite(&d, &r) { continue; }
            prop_assert!(close_at(&d, &r, prec as usize), "tan zre={zre} zim={zim} prec={prec}");
        }
    }

    /// asin(z) ≈ MPC asin(z).
    #[test]
    #[ignore]
    fn cbig_asin_fuzz(zre in f64_part(), zim in f64_part()) {
        for prec in fuzz::fuzz_precisions_bits() {
            let (z, rz) = pair(zre, zim, prec as usize);
            let d = cmplx_ok!(z.context().asin(&z, None));
            let r = rz.asin();
            if !complex_finite(&d, &r) { continue; }
            prop_assert!(close_at(&d, &r, prec as usize), "asin zre={zre} zim={zim} prec={prec}");
        }
    }

    /// acos(z) ≈ MPC acos(z).
    #[test]
    #[ignore]
    fn cbig_acos_fuzz(zre in f64_part(), zim in f64_part()) {
        for prec in fuzz::fuzz_precisions_bits() {
            let (z, rz) = pair(zre, zim, prec as usize);
            let d = cmplx_ok!(z.context().acos(&z, None));
            let r = rz.acos();
            if !complex_finite(&d, &r) { continue; }
            prop_assert!(close_at(&d, &r, prec as usize), "acos zre={zre} zim={zim} prec={prec}");
        }
    }

    /// atan(z) ≈ MPC atan(z).
    #[test]
    #[ignore]
    fn cbig_atan_fuzz(zre in f64_part(), zim in f64_part()) {
        for prec in fuzz::fuzz_precisions_bits() {
            let (z, rz) = pair(zre, zim, prec as usize);
            let d = cmplx_ok!(z.context().atan(&z, None));
            let r = rz.atan();
            if !complex_finite(&d, &r) { continue; }
            prop_assert!(close_at(&d, &r, prec as usize), "atan zre={zre} zim={zim} prec={prec}");
        }
    }

    /// sinh(z) ≈ MPC sinh(z). (dashu evaluates `sinh z = -i·sin(i·z)`, and MPC uses the same
    /// rotation identity, so the differential shares the trig fuzz's agreement.)
    #[test]
    #[ignore]
    fn cbig_sinh_fuzz(zre in f64_part(), zim in f64_part()) {
        for prec in fuzz::fuzz_precisions_bits() {
            let (z, rz) = pair(zre, zim, prec as usize);
            let d = cmplx_ok!(z.context().sinh(&z, None));
            let r = rz.sinh();
            if !complex_finite(&d, &r) { continue; }
            prop_assert!(close_at(&d, &r, prec as usize), "sinh zre={zre} zim={zim} prec={prec}");
        }
    }

    /// cosh(z) ≈ MPC cosh(z).
    #[test]
    #[ignore]
    fn cbig_cosh_fuzz(zre in f64_part(), zim in f64_part()) {
        for prec in fuzz::fuzz_precisions_bits() {
            let (z, rz) = pair(zre, zim, prec as usize);
            let d = cmplx_ok!(z.context().cosh(&z, None));
            let r = rz.cosh();
            if !complex_finite(&d, &r) { continue; }
            prop_assert!(close_at(&d, &r, prec as usize), "cosh zre={zre} zim={zim} prec={prec}");
        }
    }

    /// sinh_cosh(z) ≈ (MPC sinh(z), MPC cosh(z)) — the shared-eval pair must agree with the singles.
    #[test]
    #[ignore]
    fn cbig_sinh_cosh_fuzz(zre in f64_part(), zim in f64_part()) {
        for prec in fuzz::fuzz_precisions_bits() {
            let (z, rz) = pair(zre, zim, prec as usize);
            let (ds, dc) = z.context().sinh_cosh(&z, None);
            let ds = cmplx_ok!(ds);
            let dc = cmplx_ok!(dc);
            let rs = rz.clone().sinh();
            let rc = rz.cosh();
            if !complex_finite(&ds, &rs) || !complex_finite(&dc, &rc) { continue; }
            prop_assert!(close_at(&ds, &rs, prec as usize), "sinh_cosh[0] zre={zre} zim={zim} prec={prec}");
            prop_assert!(close_at(&dc, &rc, prec as usize), "sinh_cosh[1] zre={zre} zim={zim} prec={prec}");
        }
    }

    /// tanh(z) ≈ MPC tanh(z).
    #[test]
    #[ignore]
    fn cbig_tanh_fuzz(zre in f64_part(), zim in f64_part()) {
        for prec in fuzz::fuzz_precisions_bits() {
            let (z, rz) = pair(zre, zim, prec as usize);
            let d = cmplx_ok!(z.context().tanh(&z, None));
            let r = rz.tanh();
            if !complex_finite(&d, &r) { continue; }
            prop_assert!(close_at(&d, &r, prec as usize), "tanh zre={zre} zim={zim} prec={prec}");
        }
    }

    /// asinh(z) ≈ MPC asinh(z).
    #[test]
    #[ignore]
    fn cbig_asinh_fuzz(zre in f64_part(), zim in f64_part()) {
        for prec in fuzz::fuzz_precisions_bits() {
            let (z, rz) = pair(zre, zim, prec as usize);
            let d = cmplx_ok!(z.context().asinh(&z, None));
            let r = rz.asinh();
            if !complex_finite(&d, &r) { continue; }
            prop_assert!(close_at(&d, &r, prec as usize), "asinh zre={zre} zim={zim} prec={prec}");
        }
    }

    /// acosh(z) ≈ MPC acosh(z).
    #[test]
    #[ignore]
    fn cbig_acosh_fuzz(zre in f64_part(), zim in f64_part()) {
        for prec in fuzz::fuzz_precisions_bits() {
            let (z, rz) = pair(zre, zim, prec as usize);
            let d = cmplx_ok!(z.context().acosh(&z, None));
            let r = rz.acosh();
            if !complex_finite(&d, &r) { continue; }
            prop_assert!(close_at(&d, &r, prec as usize), "acosh zre={zre} zim={zim} prec={prec}");
        }
    }

    /// atanh(z) ≈ MPC atanh(z).
    #[test]
    #[ignore]
    fn cbig_atanh_fuzz(zre in f64_part(), zim in f64_part()) {
        for prec in fuzz::fuzz_precisions_bits() {
            let (z, rz) = pair(zre, zim, prec as usize);
            let d = cmplx_ok!(z.context().atanh(&z, None));
            let r = rz.atanh();
            if !complex_finite(&d, &r) { continue; }
            prop_assert!(close_at(&d, &r, prec as usize), "atanh zre={zre} zim={zim} prec={prec}");
        }
    }

    /// base^w ≈ MPC pow(base, w).
    #[test]
    #[ignore]
    fn cbig_powf_fuzz(
        zre in f64_part(), zim in f64_part(),
        wre in f64_part(), wim in f64_part(),
    ) {
        for prec in fuzz::fuzz_precisions_bits() {
            let (z, rz) = pair(zre, zim, prec as usize);
            let (w, rw) = pair(wre, wim, prec as usize);
            let d = cmplx_ok!(z.context().powf(&z, &w, None));
            let r = rz.pow(&rw);
            if !complex_finite(&d, &r) { continue; }
            prop_assert!(close_at(&d, &r, prec as usize), "powf zre={zre} zim={zim} wre={wre} wim={wim} prec={prec}");
        }
    }

    /// z^n ≈ MPC z^n for integer exponents (incl. negative, via dashu's reciprocal path and the
    /// Ziv squaring chain) — `mpc_pow` is single-valued for integer exponents, so it agrees with
    /// `z^n` even across the branch cut.
    #[test]
    #[ignore]
    fn cbig_powi_fuzz(zre in f64_part(), zim in f64_part(), n in -12i32..=12) {
        for prec in fuzz::fuzz_precisions_bits() {
            let (z, rz) = pair(zre, zim, prec as usize);
            let d = cmplx_ok!(z.context().powi(&z, IBig::from(n)));
            let r = rz.pow(&rug::Complex::with_val(prec as u32, (n as f64, 0.0)));
            if !complex_finite(&d, &r) { continue; }
            prop_assert!(close_at(&d, &r, prec as usize), "powi zre={zre} zim={zim} n={n} prec={prec}");
        }
    }

    /// Annex-G signed-zero checks at `±0 + i·±0`. `close_at` is sign-blind, so these assert the
    /// per-part component signs explicitly.
    #[test]
    #[ignore]
    fn cbig_signed_zero_fuzz((re, im) in zero_pair()) {
        let (c, _rz) = pair(re, im, 50);
        let ctx = c.context();

        let re_sign = if re.is_sign_negative() { Sign::Negative } else { Sign::Positive };
        let im_sign = if im.is_sign_negative() { Sign::Negative } else { Sign::Positive };
        // cos / sqr imaginary zero is the signed product x·y: -0 iff the parts have opposite signs.
        let product_sign = if re_sign != im_sign { Sign::Negative } else { Sign::Positive };

        // sin(±0 + i·0) = ±0 + i·0, cos(±0 + i·0) = 1 + i·(signed x·y).
        let (s, co) = ctx.sin_cos(&c, None);
        let s = s.unwrap().value();
        let co = co.unwrap().value();
        prop_assert_eq!(s.re().sign(), re_sign, "sin re sign z=({}, {})", re, im);
        prop_assert_eq!(s.im().sign(), im_sign, "sin im sign z=({}, {})", re, im);
        prop_assert!(co.re().significand().is_one(), "cos re = 1 z=({}, {})", re, im);
        prop_assert_eq!(co.im().sign(), product_sign, "cos im sign z=({}, {})", re, im);

        // sqr(±0 + i·0) = +0 + i·(signed 2·x·y).
        let sq = ctx.sqr(&c).unwrap().value();
        prop_assert_eq!(sq.re().sign(), Sign::Positive, "sqr re sign z=({}, {})", re, im);
        prop_assert_eq!(sq.im().sign(), product_sign, "sqr im sign z=({}, {})", re, im);

        // log(±0 + i·0) = -∞ + i·(±π if re negative, ±0 if re positive) — the im sign follows the input.
        let lg = ctx.log(&c, None).unwrap().value();
        prop_assert!(lg.re().is_infinite(), "log re -inf z=({}, {})", re, im);
        if re.is_sign_negative() {
            prop_assert!(!lg.im().significand().is_zero(), "log im = ±π z=({}, {})", re, im);
        } else {
            prop_assert!(lg.im().significand().is_zero(), "log im = ±0 z=({}, {})", re, im);
        }
        prop_assert_eq!(lg.im().sign(), im_sign, "log im sign z=({}, {})", re, im);

        // tan / tanh(±0 + i·0) = ±0 + i·0 — parts carry the input zeros' signs.
        let t = ctx.tan(&c, None).unwrap().value();
        prop_assert_eq!(t.re().sign(), re_sign, "tan re sign z=({}, {})", re, im);
        prop_assert_eq!(t.im().sign(), im_sign, "tan im sign z=({}, {})", re, im);
        let th = ctx.tanh(&c, None).unwrap().value();
        prop_assert_eq!(th.re().sign(), re_sign, "tanh re sign z=({}, {})", re, im);
        prop_assert_eq!(th.im().sign(), im_sign, "tanh im sign z=({}, {})", re, im);
    }
}
