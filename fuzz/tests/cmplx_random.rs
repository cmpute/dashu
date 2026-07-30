//! Differential / fuzz test: `dashu-cmplx::CBig` field arithmetic against `rug::Complex` (GNU MPC)
//! across the [`fuzz::fuzz_precisions_bits`] sweep (default 50/100/500/1000 bits).
//!
//! For random finite inputs, `mul`/`div`/`sqr` are computed in both libraries and must agree to
//! within `CLOSE_K × 2^-prec × scale` per component (see `close_at`) — both are (near-)correctly
//! rounded, and field arithmetic is MPC's hardest-to-round class (the spec's top risk). Non-finite
//! results (per precision) are skipped. Proptest-driven so a mismatch shrinks to a minimal
//! counterexample. Shared build/compare helpers live in `fuzz::cmplx`.
//!
//! Run with: `cargo test --manifest-path fuzz/Cargo.toml --test cmplx_random -- --ignored --nocapture`
//! (override the precision sweep with `FUZZ_PRECISIONS=53`, case count with `PROPTEST_CASES=N`.)

use fuzz::cmplx::*;
use proptest::prelude::*;

proptest! {
    #![proptest_config(fuzz::fuzz_config())]

    #[test]
    #[ignore]
    fn cbig_mul_div_sqr_fuzz(
        zre in f64_part(), zim in f64_part(),
        wre in f64_part(), wim in f64_part(),
    ) {
        for prec in fuzz::fuzz_precisions_bits() {
            let (z, rz) = pair(zre, zim, prec as usize);
            let (w, rw) = pair(wre, wim, prec as usize);
            let p = prec as usize;

            // mul
            let dm = &z * &w;
            let rm = rz.clone() * rw.clone();
            if complex_finite(&dm, &rm) {
                prop_assert!(close_at(&dm, &rm, p), "mul zre={zre} zim={zim} wre={wre} wim={wim} prec={prec}");
            }

            // sqr
            let ds = z.sqr();
            let rs = rz.clone() * rz.clone();
            if complex_finite(&ds, &rs) {
                prop_assert!(close_at(&ds, &rs, p), "sqr zre={zre} zim={zim} prec={prec}");
            }

            // div (skip a zero denominator)
            if !w.is_zero() {
                let dd = &z / &w;
                let rd = rz / rw;
                if complex_finite(&dd, &rd) {
                    prop_assert!(close_at(&dd, &rd, p), "div zre={zre} zim={zim} wre={wre} wim={wim} prec={prec}");
                }
            }
        }
    }
}
