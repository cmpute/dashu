//! Differential / fuzz tests for `dashu-cmplx::CBig` transcendentals against `rug::Complex` (GNU MPC)
//! across the [`fuzz::fuzz_precisions_bits`] sweep (default 50/100/500/1000 bits).
//!
//! Companion to `cmplx_random.rs` (which covers field arithmetic mul/div/sqr). Here: exp, log, sqrt,
//! sin, cos, tan, asin, acos, atan, powf. rug has direct MPC methods for all of these (no gaps).
//! Reuses the shared `fuzz::cmplx` build/compare helpers (`pair`, `complex_finite`, `close_at`).
//! `close_at` compares per-component to `CLOSE_K × 2^-prec × scale` ulps (not via f64, which would
//! be meaningless above 53 bits). All `#[ignore]`d (manual, release-time). Inputs are modest-magnitude
//! finite `f64` pairs, so results stay finite and `complex_finite` rarely skips.
//!
//! Run with: `cargo test --manifest-path fuzz/Cargo.toml --test cmplx_transcendental -- --ignored --nocapture`
//! (override the precision sweep with `FUZZ_PRECISIONS=53`, case count with `PROPTEST_CASES=N`.)

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
}
