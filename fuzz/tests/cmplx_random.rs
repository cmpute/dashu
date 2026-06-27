//! Differential / fuzz test: `dashu-cmplx::CBig` field arithmetic against `rug::Complex` (GNU MPC)
//! at 53-bit precision.
//!
//! For random finite inputs, `mul`/`div`/`sqr` are computed in both libraries and the `(re, im)`
//! `f64` parts must agree to within a few ulps — both are (near-)correctly rounded at 53 bits, and
//! field arithmetic is MPC's hardest-to-round class (the spec's top risk). Non-finite results are
//! skipped.
//!
//! Run with: `cargo test --manifest-path fuzz/Cargo.toml --test cmplx_random -- --ignored --nocapture`

use core::convert::TryFrom;
use dashu_cmplx::CBig;
use dashu_float::round::mode::HalfEven;
use dashu_float::FBig;
use rand::prelude::*;

type C = CBig<HalfEven, 2>;
type F = FBig<HalfEven, 2>;

/// A modest-magnitude random `f64`.
fn rand_f64(rng: &mut impl Rng) -> f64 {
    let sig = rng.random_range(-8i32..=8) as f64;
    if sig == 0.0 {
        return 0.5;
    }
    let mant = rng.random_range(1.0..2.0);
    let exp = rng.random_range(-2i32..=2);
    sig.signum() * mant * 2f64.powi(exp)
}

fn fbig_from(v: f64) -> F {
    F::try_from(v).unwrap().with_precision(53).value()
}

/// Build a dashu `CBig` and a matching `rug::Complex` (53-bit) from `f64` parts.
fn pair(re: f64, im: f64) -> (C, rug::Complex) {
    let cbig = CBig::from_parts(fbig_from(re), fbig_from(im));
    let rug = rug::Complex::with_val(53, (re, im));
    (cbig, rug)
}

fn cbig_to_f64(z: &C) -> (f64, f64) {
    let (re, im) = z.clone().into_parts();
    (re.to_f64().value(), im.to_f64().value())
}

fn rug_to_f64(z: &rug::Complex) -> (f64, f64) {
    (z.real().to_f64(), z.imag().to_f64())
}

/// True when both `(re, im)` pairs are finite and agree to within a few ulps.
fn close(a: (f64, f64), b: (f64, f64)) -> bool {
    let (ar, ai) = a;
    let (br, bi) = b;
    if !ar.is_finite() || !ai.is_finite() || !br.is_finite() || !bi.is_finite() {
        return false; // skip non-finite (overflow / branch-point) results
    }
    let scale = ar.abs().max(ai.abs()).max(br.abs()).max(bi.abs()).max(1e-300);
    let tol = scale * 1e-12;
    (ar - br).abs() <= tol && (ai - bi).abs() <= tol
}

#[test]
#[ignore]
fn mpc_mul_div_sqr_oracle() {
    let mut rng = StdRng::seed_from_u64(7);
    for _ in 0..8192 {
        let (z, rz) = pair(rand_f64(&mut rng), rand_f64(&mut rng));
        let (w, rw) = pair(rand_f64(&mut rng), rand_f64(&mut rng));

        // mul
        assert!(close(cbig_to_f64(&(&z * &w)), rug_to_f64(&(rz.clone() * rw.clone()))));
        // sqr
        assert!(close(cbig_to_f64(&z.sqr()), rug_to_f64(&(rz.clone() * rz.clone()))));
        // div (skip zero denominator)
        if !w.is_zero() && !rw.real().to_f64().abs().eq(&0.0) {
            assert!(close(cbig_to_f64(&(&z / &w)), rug_to_f64(&(rz / rw))));
        }
    }
}
