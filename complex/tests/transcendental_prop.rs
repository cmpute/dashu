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
        prop_assert!(within_ulps_cbig(&rp, &r2, 2));
    }

    #[test]
    fn exp_self_oracle(z in cbig_strategy()) {
        let lo = Context::new(P);
        let hi = Context::new(2 * P);
        let rp = lo.exp(&z, None).unwrap().value();
        let r2 = reround_hi(hi.exp(&z, None).unwrap().value());
        prop_assert!(within_ulps_cbig(&rp, &r2, 2));
    }

    #[test]
    fn log_self_oracle(z in cbig_strategy()) {
        let lo = Context::new(P);
        let hi = Context::new(2 * P);
        let rp = lo.log(&z, None).unwrap().value();
        let r2 = reround_hi(hi.log(&z, None).unwrap().value());
        prop_assert!(within_ulps_cbig(&rp, &r2, 2));
    }

    #[test]
    fn log_imag_is_arg(z in cbig_strategy()) {
        // the imaginary part of log z equals arg z ∈ ]-π, π]; the real part equals ln|z|
        let (lr, li) = z.log().into_parts();
        let arg = z.arg();
        prop_assert!(within_ulps(&li, &arg, 16));
        let abs = z.abs();
        let ln_abs = abs.ln();
        prop_assert!(within_ulps(&lr, &ln_abs, 16));
    }
}
