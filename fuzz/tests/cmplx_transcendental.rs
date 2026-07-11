//! Differential / fuzz tests for `dashu-cmplx::CBig` transcendentals against `rug::Complex` (GNU MPC)
//! at 53-bit precision.
//!
//! Companion to `cmplx_random.rs` (which covers field arithmetic mul/div/sqr). Here: exp, log, sqrt,
//! sin, cos, tan, asin, acos, atan, powf. rug has direct MPC methods for all of these (no gaps).
//! Reuses the shared `fuzz::cmplx` build/compare helpers (`pair`, `cbig_to_f64`, `rug_to_f64`,
//! `close`). All `#[ignore]`d (manual, release-time). Inputs are modest-magnitude finite `f64`
//! pairs, so results stay finite and `close`'s non-finite guard never trips in practice.
//!
//! Run with: `cargo test --manifest-path fuzz/Cargo.toml --test cmplx_transcendental -- --ignored --nocapture`

use fuzz::cmplx::*;
use proptest::prelude::*;
use rug::ops::Pow;

/// Unwrap a `CfpResult<CBig>` to its `CBig` value, or skip the case on error (e.g. tan at a zero of
/// cos, powf singularities).
macro_rules! cmplx_ok {
    ($e:expr) => {
        match $e {
            Ok(v) => v.value(),
            Err(_) => return Ok(()),
        }
    };
}

proptest! {
    #![proptest_config(fuzz::fuzz_config())]

    /// exp(z) ≈ MPC exp(z).
    #[test]
    #[ignore]
    fn mpc_exp(zre in f64_part(), zim in f64_part()) {
        let (z, rz) = pair(zre, zim);
        let d = cmplx_ok!(z.context().exp(&z, None));
        prop_assert!(close(cbig_to_f64(&d), rug_to_f64(&rz.exp())));
    }

    /// log(z) ≈ MPC ln(z).
    #[test]
    #[ignore]
    fn mpc_log(zre in f64_part(), zim in f64_part()) {
        let (z, rz) = pair(zre, zim);
        let d = cmplx_ok!(z.context().log(&z, None));
        prop_assert!(close(cbig_to_f64(&d), rug_to_f64(&rz.ln())));
    }

    /// sqrt(z) ≈ MPC sqrt(z).
    #[test]
    #[ignore]
    fn mpc_sqrt(zre in f64_part(), zim in f64_part()) {
        let (z, rz) = pair(zre, zim);
        let d = cmplx_ok!(z.context().sqrt(&z));
        prop_assert!(close(cbig_to_f64(&d), rug_to_f64(&rz.sqrt())));
    }

    /// sin(z) ≈ MPC sin(z).
    #[test]
    #[ignore]
    fn mpc_sin(zre in f64_part(), zim in f64_part()) {
        let (z, rz) = pair(zre, zim);
        let d = cmplx_ok!(z.context().sin(&z, None));
        prop_assert!(close(cbig_to_f64(&d), rug_to_f64(&rz.sin())));
    }

    /// cos(z) ≈ MPC cos(z).
    #[test]
    #[ignore]
    fn mpc_cos(zre in f64_part(), zim in f64_part()) {
        let (z, rz) = pair(zre, zim);
        let d = cmplx_ok!(z.context().cos(&z, None));
        prop_assert!(close(cbig_to_f64(&d), rug_to_f64(&rz.cos())));
    }

    /// tan(z) ≈ MPC tan(z) (skips zeros of cos, where tan is singular).
    #[test]
    #[ignore]
    fn mpc_tan(zre in f64_part(), zim in f64_part()) {
        let (z, rz) = pair(zre, zim);
        let d = cmplx_ok!(z.context().tan(&z, None));
        prop_assert!(close(cbig_to_f64(&d), rug_to_f64(&rz.tan())));
    }

    /// asin(z) ≈ MPC asin(z).
    #[test]
    #[ignore]
    fn mpc_asin(zre in f64_part(), zim in f64_part()) {
        let (z, rz) = pair(zre, zim);
        let d = cmplx_ok!(z.context().asin(&z, None));
        prop_assert!(close(cbig_to_f64(&d), rug_to_f64(&rz.asin())));
    }

    /// acos(z) ≈ MPC acos(z).
    #[test]
    #[ignore]
    fn mpc_acos(zre in f64_part(), zim in f64_part()) {
        let (z, rz) = pair(zre, zim);
        let d = cmplx_ok!(z.context().acos(&z, None));
        prop_assert!(close(cbig_to_f64(&d), rug_to_f64(&rz.acos())));
    }

    /// atan(z) ≈ MPC atan(z).
    #[test]
    #[ignore]
    fn mpc_atan(zre in f64_part(), zim in f64_part()) {
        let (z, rz) = pair(zre, zim);
        let d = cmplx_ok!(z.context().atan(&z, None));
        prop_assert!(close(cbig_to_f64(&d), rug_to_f64(&rz.atan())));
    }

    /// base^w ≈ MPC pow(base, w).
    #[test]
    #[ignore]
    fn mpc_powf(
        zre in f64_part(), zim in f64_part(),
        wre in f64_part(), wim in f64_part(),
    ) {
        let (z, rz) = pair(zre, zim);
        let (w, rw) = pair(wre, wim);
        let d = cmplx_ok!(z.context().powf(&z, &w, None));
        prop_assert!(close(cbig_to_f64(&d), rug_to_f64(&rz.pow(&rw))));
    }
}
