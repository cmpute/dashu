//! Transcendental identity property tests for sqrt / exp / log (and the sqrt self-oracle).

use dashu_base::{Abs, AbsOrd};
use dashu_cmplx::{CBig, Context, FBig};
use dashu_float::round::mode::HalfEven;
use proptest::prelude::*;

type C = CBig<HalfEven, 2>;
type F = FBig<HalfEven, 2>;

const P: usize = 53;

fn fbig_strategy() -> impl Strategy<Value = F> {
    // keep the real part non-negative so sqrt's principal branch is well away from the cut, and
    // magnitudes modest
    (1i64..(1i64 << 20), -6isize..6isize)
        .prop_map(|(sig, exp)| F::from_parts(sig.into(), exp).with_precision(P).value())
}

fn cbig_strategy() -> impl Strategy<Value = C> {
    (fbig_strategy(), fbig_strategy()).prop_map(|(re, im)| CBig::from_parts(re, im))
}

/// Modest-magnitude parts (≈ [0.25, 1.75], so `|re|,|im| < 2`): for the trig identities and
/// inverse-trig oracles. The pythagorean identity needs small `|im|` (else `cosh²y`/`sinh²y`
/// catastrophically cancel to 1), and `asin`'s `iz + sqrt(1-z²)` cancels for large `|z|`.
fn small_strategy() -> impl Strategy<Value = C> {
    (1i64..8, 1i64..8).prop_map(|(re_num, im_num)| {
        let re = F::from_parts(re_num.into(), -2).with_precision(P).value();
        let im = F::from_parts(im_num.into(), -2).with_precision(P).value();
        CBig::from_parts(re, im)
    })
}

/// Modest real part (away from the poles `π/2 + kπ`) with a moderate imaginary part up to ~40 —
/// large enough that the naive `sin z / cos z` division would catastrophically cancel in the real
/// part, small enough that `cosh(2·im)` stays cheap. For the cancellation-free `tan` oracle.
fn tan_strategy() -> impl Strategy<Value = C> {
    ((1i64..(1i64 << 20), -4isize..4isize), 1i64..40i64).prop_map(|((re_sig, re_exp), im_num)| {
        let re = F::from_parts(re_sig.into(), re_exp)
            .with_precision(P)
            .value();
        let im = F::from_parts(im_num.into(), 0).with_precision(P).value();
        CBig::from_parts(re, im)
    })
}

fn within_ulps(a: &F, b: &F, k: u32) -> bool {
    if a == b {
        return true;
    }
    let diff = (a.clone() - b.clone()).abs();
    diff.abs_cmp(&(a.ulp() * F::from(k))).is_le()
}

fn within_ulps_cbig(a: &C, b: &C, k: u32) -> bool {
    let (ar, ai) = a.clone().into_parts();
    let (br, bi) = b.clone().into_parts();
    within_ulps(&ar, &br, k) && within_ulps(&ai, &bi, k)
}

fn reround_hi(hi: C) -> C {
    let (re, im) = hi.into_parts();
    CBig::from_parts(re.with_precision(P).value(), im.with_precision(P).value())
}

proptest! {
    #[test]
    fn sqrt_conj_identity(z in cbig_strategy()) {
        // sqrt(conj z) == conj(sqrt z): the magnitude path is identical, only the im sign differs
        prop_assert!(z.conj().sqrt() == z.sqrt().conj());
    }

    #[test]
    fn sqrt_self_oracle(z in cbig_strategy()) {
        let lo = Context::new(P);
        let hi = Context::new(2 * P);
        let rp = lo.sqrt(&z).unwrap().value();
        let r2 = reround_hi(hi.sqrt(&z).unwrap().value());
        // sqrt is correctly rounded via Ziv, so 1 ULP.
        prop_assert!(within_ulps_cbig(&rp, &r2, 1));
    }

    #[test]
    fn tan_self_oracle(z in tan_strategy()) {
        // tan uses the cancellation-free double-angle form, so it stays correct for moderate-large
        // |Im z| (where the naive sin/cos division would cancel). 1 ULP.
        let lo = Context::new(P);
        let hi = Context::new(2 * P);
        let rp = lo.tan(&z, None).unwrap().value();
        let r2 = reround_hi(hi.tan(&z, None).unwrap().value());
        prop_assert!(within_ulps_cbig(&rp, &r2, 1));
    }

    #[test]
    fn exp_self_oracle(z in cbig_strategy()) {
        let lo = Context::new(P);
        let hi = Context::new(2 * P);
        let rp = lo.exp(&z, None).unwrap().value();
        let r2 = reround_hi(hi.exp(&z, None).unwrap().value());
        // exp is correctly rounded via Ziv, so the low- and (re-rounded) high-precision results
        // agree to within 1 ULP (they match exactly except at rare near-tie double-rounding cases).
        prop_assert!(within_ulps_cbig(&rp, &r2, 1));
    }

    #[test]
    fn log_self_oracle(z in cbig_strategy()) {
        let lo = Context::new(P);
        let hi = Context::new(2 * P);
        let rp = lo.log(&z, None).unwrap().value();
        let r2 = reround_hi(hi.log(&z, None).unwrap().value());
        // log is correctly rounded via Ziv (its real part even carries an amplification term near
        // |z|=1), so the results agree to within 1 ULP.
        prop_assert!(within_ulps_cbig(&rp, &r2, 1));
    }

    #[test]
    fn log_imag_is_arg(z in cbig_strategy()) {
        // the imaginary part of log z equals arg z ∈ ]-π, π]; the real part equals ln|z|
        let (lr, li) = z.ln().into_parts();
        let arg = z.arg();
        prop_assert!(within_ulps(&li, &arg, 16));
        let abs = z.abs();
        let ln_abs = abs.ln();
        prop_assert!(within_ulps(&lr, &ln_abs, 16));
    }

    #[test]
    fn sin_cos_self_oracle(z in cbig_strategy()) {
        let lo = Context::new(P);
        let hi = Context::new(2 * P);
        let sp = lo.sin(&z, None).unwrap().value();
        let cp = lo.cos(&z, None).unwrap().value();
        let s2 = reround_hi(hi.sin(&z, None).unwrap().value());
        let c2 = reround_hi(hi.cos(&z, None).unwrap().value());
        // sin/cos are correctly rounded via a shared Ziv loop, so 1 ULP.
        prop_assert!(within_ulps_cbig(&sp, &s2, 1));
        prop_assert!(within_ulps_cbig(&cp, &c2, 1));
    }

    #[test]
    fn abs_self_oracle(z in cbig_strategy()) {
        // abs delegates directly to the (Ziv-correctly-rounded) real hypot, so abs@p agrees with
        // abs@2p re-rounded to p exactly (0 ULP) save for rare near-tie double-rounding (≤1 ULP).
        let lo = Context::new(P);
        let hi = Context::new(2 * P);
        let lp = lo.abs(&z).unwrap().value();
        let hp = hi.abs(&z).unwrap().value().with_precision(P).value();
        prop_assert!(within_ulps(&lp, &hp, 1));
    }

    #[test]
    fn pythagorean_identity(z in small_strategy()) {
        // sin²z + cos²z = 1: the real part is ~1, the imaginary part is a small residual of O(1)
        // terms, so compare it to ulp(1) (not its own tiny ulp). F::ONE has unlimited precision, so
        // take the ulp from a precision-P one.
        let s = z.sin();
        let c = z.cos();
        let sum = &s.sqr() + &c.sqr();
        let (re, im) = sum.into_parts();
        let one = F::ONE.with_precision(P).value();
        let tol = one.ulp() * F::from(16u32);
        prop_assert!(im.abs_cmp(&tol).is_le());
        prop_assert!((re.clone() - F::ONE).abs_cmp(&tol).is_le());
    }

    #[test]
    fn asin_self_oracle(z in small_strategy()) {
        let lo = Context::new(P);
        let hi = Context::new(2 * P);
        let rp = lo.asin(&z, None).unwrap().value();
        let r2 = reround_hi(hi.asin(&z, None).unwrap().value());
        // asin is correctly rounded via Ziv (well-conditioned regime), 1 ULP.
        prop_assert!(within_ulps_cbig(&rp, &r2, 1));
    }

    #[test]
    fn atan_self_oracle(z in small_strategy()) {
        let lo = Context::new(P);
        let hi = Context::new(2 * P);
        let rp = lo.atan(&z, None).unwrap().value();
        let r2 = reround_hi(hi.atan(&z, None).unwrap().value());
        // atan is correctly rounded via Ziv (well-conditioned regime), 1 ULP.
        prop_assert!(within_ulps_cbig(&rp, &r2, 1));
    }

    #[test]
    fn powf_self_oracle((base, w) in (small_strategy(), small_strategy())) {
        // modest base/exponent keep `w·log base` bounded (no overflow); base has positive real part
        // so `log` stays off its branch cut. powf is correctly rounded via Ziv, 1 ULP.
        let lo = Context::new(P);
        let hi = Context::new(2 * P);
        let rp = lo.powf(&base, &w, None).unwrap().value();
        let r2 = reround_hi(hi.powf(&base, &w, None).unwrap().value());
        prop_assert!(within_ulps_cbig(&rp, &r2, 1));
    }
}
