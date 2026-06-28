//! Differential / fuzz test: `dashu-cmplx::CBig` field arithmetic against `rug::Complex` (GNU MPC)
//! at 53-bit precision.
//!
//! For random finite inputs, `mul`/`div`/`sqr` are computed in both libraries and the `(re, im)`
//! `f64` parts must agree to within a few ulps — both are (near-)correctly rounded at 53 bits, and
//! field arithmetic is MPC's hardest-to-round class (the spec's top risk). Non-finite results are
//! skipped. Proptest-driven so a mismatch shrinks to a minimal counterexample. Shared
//! build/compare helpers live in `fuzz::cmplx`.
//!
//! Run with: `cargo test --manifest-path fuzz/Cargo.toml --test cmplx_random -- --ignored --nocapture`

use fuzz::cmplx::*;
use proptest::prelude::*;

proptest! {
    #![proptest_config(fuzz::fuzz_config())]

    #[test]
    #[ignore]
    fn mpc_mul_div_sqr_oracle(
        zre in f64_part(), zim in f64_part(),
        wre in f64_part(), wim in f64_part(),
    ) {
        let (z, rz) = pair(zre, zim);
        let (w, rw) = pair(wre, wim);

        // mul
        prop_assert!(close(cbig_to_f64(&(&z * &w)), rug_to_f64(&(rz.clone() * rw.clone()))));
        // sqr
        prop_assert!(close(cbig_to_f64(&z.sqr()), rug_to_f64(&(rz.clone() * rz.clone()))));
        // div (skip a zero denominator)
        if !w.is_zero() {
            prop_assert!(close(cbig_to_f64(&(&z / &w)), rug_to_f64(&(rz / rw))));
        }
    }
}
