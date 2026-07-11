//! Correct-rounding self-oracle: each op computed at precision `p` is recomputed at `2p` and
//! re-rounded to `p`; the two must agree to within 1 ulp per component (the near-correctly-rounded
//! guarantee class). Also covers the approximate algebraic identities.

use dashu_base::{Abs, AbsOrd};
use dashu_cmplx::{CBig, Context, FBig};
use dashu_float::round::mode::HalfEven;
use proptest::prelude::*;

type C = CBig<HalfEven, 2>;
type F = FBig<HalfEven, 2>;

const P: usize = 53;

fn fbig_strategy() -> impl Strategy<Value = F> {
    // keep magnitudes modest so div denominators stay well-conditioned
    (1i64..(1i64 << 20), -8isize..8isize)
        .prop_map(|(sig, exp)| F::from_parts(sig.into(), exp).with_precision(P).value())
}

fn cbig_strategy() -> impl Strategy<Value = C> {
    (fbig_strategy(), fbig_strategy()).prop_map(|(re, im)| CBig::from_parts(re, im))
}

/// True when `a` and `b` agree to within `k` ulps of `a` (both must have limited precision).
fn within_ulps(a: &F, b: &F, k: u32) -> bool {
    if a == b {
        return true;
    }
    let diff = (a.clone() - b.clone()).abs();
    let bound = a.ulp() * F::from(k);
    diff.abs_cmp(&bound).is_le()
}

fn within_ulps_cbig(a: &C, b: &C, k: u32) -> bool {
    let (ar, ai) = a.clone().into_parts();
    let (br, bi) = b.clone().into_parts();
    within_ulps(&ar, &br, k) && within_ulps(&ai, &bi, k)
}

/// Recompute a binary/unary op at `2p` and re-round each part back to `p`.
fn reround_hi(hi: C) -> C {
    let (re, im) = hi.into_parts();
    CBig::from_parts(re.with_precision(P).value(), im.with_precision(P).value())
}

proptest! {
    #[test]
    fn mul_self_oracle((z, w) in (cbig_strategy(), cbig_strategy())) {
        let lo = Context::new(P);
        let hi = Context::new(2 * P);
        let rp = lo.mul(&z, &w).unwrap().value();
        let r2 = reround_hi(hi.mul(&z, &w).unwrap().value());
        prop_assert!(within_ulps_cbig(&rp, &r2, 1));
    }

    #[test]
    fn sqr_self_oracle(z in cbig_strategy()) {
        let lo = Context::new(P);
        let hi = Context::new(2 * P);
        let rp = lo.sqr(&z).unwrap().value();
        let r2 = reround_hi(hi.sqr(&z).unwrap().value());
        prop_assert!(within_ulps_cbig(&rp, &r2, 1));
    }

    #[test]
    fn div_self_oracle((z, w) in (cbig_strategy(), cbig_strategy())) {
        let lo = Context::new(P);
        let hi = Context::new(2 * P);
        let rp = lo.div(&z, &w).unwrap().value();
        let r2 = reround_hi(hi.div(&z, &w).unwrap().value());
        prop_assert!(within_ulps_cbig(&rp, &r2, 1));
    }

    #[test]
    fn abs_self_oracle(z in cbig_strategy()) {
        // abs returns a real FBig; compare at p vs 2p re-rounded
        let lo = Context::new(P);
        let hi = Context::new(2 * P);
        let rp = lo.abs(&z).unwrap().value();
        let r2 = hi.abs(&z).unwrap().value().with_precision(P).value();
        prop_assert!(within_ulps(&rp, &r2, 1));
    }

    #[test]
    fn mul_conj_is_norm(z in cbig_strategy()) {
        // z·conj(z) is purely real and equals norm(z) = |z|²
        let p = &z * &z.conj();
        let (re, im) = p.into_parts();
        let norm = z.norm();
        prop_assert!(within_ulps(&im, &F::ZERO, 4));
        prop_assert!(within_ulps(&re, &norm, 4));
    }
}
