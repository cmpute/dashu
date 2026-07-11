//! Differential / fuzz tests for `dashu-int` (`UBig`/`IBig`) against `rug::Integer` (GMP).
//!
//! Integer ops are EXACT (no rounding), so the comparison is exact equality via a decimal-string
//! round-trip (`dashu.to_string()` → `rug::Integer` parse, compute, rug result → string →
//! `dashu::from_str_radix`). This validates dashu's bignum algorithms against the GMP reference.
//! Proptest-driven; all `#[ignore]`d (manual, release-time).
//!
//! Run with: `cargo test --manifest-path fuzz/Cargo.toml --test integer -- --ignored --nocapture`

use dashu::base::SquareRoot;
use dashu::base::ring::{DivRem, Gcd};
use dashu::integer::{IBig, UBig};
use proptest::prelude::*;
use rug::ops::Pow;

fn u_to_rug(x: &UBig) -> rug::Integer {
    x.to_string().parse::<rug::Integer>().unwrap()
}
fn i_to_rug(x: &IBig) -> rug::Integer {
    x.to_string().parse::<rug::Integer>().unwrap()
}
fn rug_to_u(i: &rug::Integer) -> UBig {
    UBig::from_str_radix(&i.to_string_radix(10), 10).unwrap()
}
fn rug_to_i(i: &rug::Integer) -> IBig {
    IBig::from_str_radix(&i.to_string_radix(10), 10).unwrap()
}

/// Complete a rug `…Incomplete` (or an owned `Integer`) into an `Integer` via `Assign`.
fn rugc<S>(src: S) -> rug::Integer
where
    rug::Integer: rug::Assign<S>,
{
    let mut r = rug::Integer::new();
    rug::Assign::assign(&mut r, src);
    r
}

proptest! {
    #![proptest_config(fuzz::fuzz_config())]

    // ---- UBig ----


    #[test]
    #[ignore]
    fn ubig_mul(a in fuzz::ubig_strategy(4), b in fuzz::ubig_strategy(4)) {
        let d = &a * &b;
        let r = rugc(u_to_rug(&a) * u_to_rug(&b));
        prop_assert_eq!(d, rug_to_u(&r));
    }

    #[test]
    #[ignore]
    fn ubig_sqr(a in fuzz::ubig_strategy(5)) {
        let d = a.sqr();
        let ar = u_to_rug(&a);
        let r = rugc(&ar * &ar);
        prop_assert_eq!(d, rug_to_u(&r));
    }

    #[test]
    #[ignore]
    fn ubig_gcd((a, b) in (fuzz::ubig_strategy(4), fuzz::ubig_strategy(4)).prop_filter("not both zero (gcd(0,0) is undefined → panic)", |(a, b)| !(a.is_zero() && b.is_zero()))) {
        let r = u_to_rug(&a).gcd(&u_to_rug(&b));
        let d = a.gcd(&b);
        prop_assert_eq!(d, rug_to_u(&r));
    }

    #[test]
    #[ignore]
    fn ubig_div_rem(a in fuzz::ubig_strategy(4), b in fuzz::ubig_strategy(2).prop_filter("nonzero", |b| !b.is_zero())) {
        let (rq, rr) = u_to_rug(&a).div_rem(u_to_rug(&b));
        let (dq, dr) = a.div_rem(&b);
        prop_assert_eq!(dq, rug_to_u(&rq));
        prop_assert_eq!(dr, rug_to_u(&rr));
    }

    #[test]
    #[ignore]
    fn ubig_pow(a in fuzz::ubig_strategy(3), n in 0u32..=16) {
        let d = a.pow(n as usize);
        let r = rugc(u_to_rug(&a).pow(n));
        prop_assert_eq!(d, rug_to_u(&r));
    }

    #[test]
    #[ignore]
    fn ubig_sqrt(a in fuzz::ubig_strategy(6)) {
        let d = a.sqrt();
        let r = u_to_rug(&a).sqrt();
        prop_assert_eq!(d, rug_to_u(&r));
    }

    #[test]
    #[ignore]
    fn ubig_nth_root(a in fuzz::ubig_strategy(6), n in 2u32..=6) {
        let d = a.nth_root(n as usize);
        let r = u_to_rug(&a).root(n);
        prop_assert_eq!(d, rug_to_u(&r));
    }

    #[test]
    #[ignore]
    fn ubig_bit_ops(a in fuzz::ubig_strategy(4), b in fuzz::ubig_strategy(4)) {
        let (ar, br) = (u_to_rug(&a), u_to_rug(&b));
        prop_assert_eq!(&a & &b, rug_to_u(&rugc(&ar & &br)));
        prop_assert_eq!(&a | &b, rug_to_u(&rugc(&ar | &br)));
        prop_assert_eq!(&a ^ &b, rug_to_u(&rugc(&ar ^ &br)));
    }

    #[test]
    #[ignore]
    fn ubig_shifts(a in fuzz::ubig_strategy(4), n in 0u32..=200) {
        let ar = u_to_rug(&a);
        prop_assert_eq!(&a << (n as usize), rug_to_u(&rugc(&ar << n)));
        prop_assert_eq!(&a >> (n as usize), rug_to_u(&rugc(&ar >> n)));
    }

    // ---- IBig ----

    #[test]
    #[ignore]
    fn ibig_mul(a in fuzz::ibig_strategy(4), b in fuzz::ibig_strategy(4)) {
        let d = &a * &b;
        let r = rugc(i_to_rug(&a) * i_to_rug(&b));
        prop_assert_eq!(d, rug_to_i(&r));
    }

    #[test]
    #[ignore]
    fn ibig_gcd((a, b) in (fuzz::ibig_strategy(4), fuzz::ibig_strategy(4)).prop_filter("not both zero (gcd(0,0) is undefined → panic)", |(a, b)| !(a.is_zero() && b.is_zero()))) {
        // gcd is non-negative for both
        let r = i_to_rug(&a).gcd(&i_to_rug(&b));
        let d: UBig = a.gcd(&b);
        prop_assert_eq!(d, rug_to_u(&r));
    }

    #[test]
    #[ignore]
    fn ibig_div_rem(a in fuzz::ibig_strategy(4), b in fuzz::ibig_strategy(2).prop_filter("nonzero", |b| !b.is_zero())) {
        let (rq, rr) = i_to_rug(&a).div_rem(i_to_rug(&b));
        let (dq, dr) = a.div_rem(&b);
        prop_assert_eq!(dq, rug_to_i(&rq));
        prop_assert_eq!(dr, rug_to_i(&rr));
    }

    #[test]
    #[ignore]
    fn ibig_pow(a in fuzz::ibig_strategy(3), n in 0u32..=12) {
        let d = a.pow(n as usize);
        let r = rugc(i_to_rug(&a).pow(n));
        prop_assert_eq!(d, rug_to_i(&r));
    }

    #[test]
    #[ignore]
    fn ibig_bit_ops(a in fuzz::ibig_strategy(4), b in fuzz::ibig_strategy(4)) {
        let (ar, br) = (i_to_rug(&a), i_to_rug(&b));
        prop_assert_eq!(&a & &b, rug_to_i(&rugc(&ar & &br)));
        prop_assert_eq!(&a | &b, rug_to_i(&rugc(&ar | &br)));
        prop_assert_eq!(&a ^ &b, rug_to_i(&rugc(&ar ^ &br)));
    }

    #[test]
    #[ignore]
    fn ibig_shifts(a in fuzz::ibig_strategy(4), n in 0u32..=200) {
        let ar = i_to_rug(&a);
        prop_assert_eq!(&a << (n as usize), rug_to_i(&rugc(&ar << n)));
        prop_assert_eq!(&a >> (n as usize), rug_to_i(&rugc(&ar >> n)));
    }
}
