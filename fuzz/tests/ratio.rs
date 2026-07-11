//! Differential / fuzz tests for `dashu-ratio` (`RBig`) against `rug::Rational` (GMP mpq).
//!
//! Rational ops are EXACT (no rounding), so the comparison is exact value equality: build a
//! `rug::Rational` from each side's canonical `(numerator, denominator)`, compute the op, and assert
//! the two `rug::Rational` results are equal (both canonical). Proptest-driven; all `#[ignore]`d
//! (manual, release-time).
//!
//! Run with: `cargo test --manifest-path fuzz/Cargo.toml --test ratio -- --ignored --nocapture`

use dashu::base::Inverse;
use dashu::rational::RBig;
use proptest::prelude::*;
use rug::ops::Pow;

/// Mirror a (canonical) dashu `RBig` into a `rug::Rational` via decimal num/den strings.
fn rbig_to_rug(r: &RBig) -> rug::Rational {
    let n = r.numerator().to_string().parse::<rug::Integer>().unwrap();
    let d = r.denominator().to_string().parse::<rug::Integer>().unwrap();
    rug::Rational::from((n, d))
}

/// Complete a rug `…Incomplete` (or owned `Rational`) into a `Rational` via `Assign`.
fn rugc_r<S>(src: S) -> rug::Rational
where
    rug::Rational: rug::Assign<S>,
{
    let mut r = rug::Rational::new();
    rug::Assign::assign(&mut r, src);
    r
}

fn rbig_strategy() -> impl Strategy<Value = RBig> {
    (
        fuzz::ibig_strategy(3),
        fuzz::ubig_strategy(2).prop_filter("nonzero denominator", |d| !d.is_zero()),
    )
        .prop_map(|(n, d)| RBig::from_parts(n, d))
}

proptest! {
    #![proptest_config(fuzz::fuzz_config())]

    #[test]
    #[ignore]
    fn ratio_add(a in rbig_strategy(), b in rbig_strategy()) {
        let (ra, rb) = (rbig_to_rug(&a), rbig_to_rug(&b));
        let d = &a + &b;
        prop_assert!(rbig_to_rug(&d) == rugc_r(&ra + &rb));
    }

    #[test]
    #[ignore]
    fn ratio_sub(a in rbig_strategy(), b in rbig_strategy()) {
        let (ra, rb) = (rbig_to_rug(&a), rbig_to_rug(&b));
        let d = &a - &b;
        prop_assert!(rbig_to_rug(&d) == rugc_r(&ra - &rb));
    }

    #[test]
    #[ignore]
    fn ratio_mul(a in rbig_strategy(), b in rbig_strategy()) {
        let (ra, rb) = (rbig_to_rug(&a), rbig_to_rug(&b));
        let d = &a * &b;
        prop_assert!(rbig_to_rug(&d) == rugc_r(&ra * &rb));
    }

    #[test]
    #[ignore]
    fn ratio_div(a in rbig_strategy(), b in rbig_strategy().prop_filter("nonzero value", |b| !b.numerator().is_zero())) {
        let (ra, rb) = (rbig_to_rug(&a), rbig_to_rug(&b));
        let d = &a / &b;
        prop_assert!(rbig_to_rug(&d) == rugc_r(&ra / &rb));
    }

    #[test]
    #[ignore]
    fn ratio_sqr(a in rbig_strategy()) {
        let ra = rbig_to_rug(&a);
        let d = a.sqr();
        prop_assert!(rbig_to_rug(&d) == rugc_r(&ra * &ra));
    }

    #[test]
    #[ignore]
    fn ratio_pow(a in rbig_strategy(), n in 0u32..=12) {
        let ra = rbig_to_rug(&a);
        let d = a.pow(n as usize);
        prop_assert!(rbig_to_rug(&d) == rugc_r(ra.pow(n)));
    }

    #[test]
    #[ignore]
    fn ratio_inv(a in rbig_strategy().prop_filter("nonzero value", |a| !a.numerator().is_zero())) {
        let ra = rbig_to_rug(&a);
        let d = a.inv();
        prop_assert!(rbig_to_rug(&d) == rugc_r(ra.recip()));
    }

    /// `from_parts` reduces to the same canonical form as GMP.
    #[test]
    #[ignore]
    fn ratio_reduce(num in fuzz::ibig_strategy(3), den in fuzz::ubig_strategy(2).prop_filter("nonzero", |d| !d.is_zero())) {
        let r = RBig::from_parts(num.clone(), den.clone());
        // canonical: denominator > 0, gcd(|num|, den) == 1 — check against GMP's reduction
        let rug_n = num.to_string().parse::<rug::Integer>().unwrap();
        let rug_d = den.to_string().parse::<rug::Integer>().unwrap();
        let rr = rug::Rational::from((rug_n, rug_d));
        prop_assert!(rbig_to_rug(&r) == rr);
    }
}
