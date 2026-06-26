//! Property tests for dashu-ratio arithmetic identities (pure-Rust, exact).
//!
//! Rationals are exact, so these laws hold with exact equality (no tolerance).
//! They exercise add/sub/mul/div, sign handling, automatic GCD reduction, and the
//! `Relaxed` value view.

use dashu_int::{IBig, UBig};
use dashu_ratio::RBig;
use proptest::prelude::*;

fn rbig() -> impl Strategy<Value = RBig> {
    (any::<i64>(), any::<u64>())
        .prop_filter("nonzero denominator", |(_, d)| *d != 0)
        .prop_map(|(n, d)| RBig::from_parts(IBig::from(n), UBig::from(d)))
}

fn nonzero_rbig() -> impl Strategy<Value = RBig> {
    rbig().prop_filter("nonzero value", |c| c.numerator() != &IBig::ZERO)
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 128, ..Default::default() })]

    /// a + c == c + a,  a * c == c * a.
    #[test]
    fn commutative(a in rbig(), c in rbig()) {
        prop_assert_eq!(a.clone() + c.clone(), c.clone() + a.clone());
        prop_assert_eq!(a.clone() * c.clone(), c.clone() * a.clone());
    }

    /// (a + c) + e == a + (c + e),  and likewise for multiplication.
    #[test]
    fn associative(a in rbig(), c in rbig(), e in rbig()) {
        prop_assert_eq!((a.clone() + c.clone()) + e.clone(), a.clone() + (c.clone() + e.clone()));
        prop_assert_eq!((a.clone() * c.clone()) * e.clone(), a.clone() * (c.clone() * e.clone()));
    }

    /// a + 0 == a,  a - a == 0,  a * 1 == a.
    #[test]
    fn identities(a in rbig()) {
        prop_assert_eq!(a.clone() + RBig::ZERO, a.clone());
        prop_assert_eq!(a.clone() - a.clone(), RBig::ZERO);
        prop_assert_eq!(a.clone() * RBig::ONE, a.clone());
    }

    /// (a / c) * c == a   (c != 0).
    #[test]
    fn div_mul_inverse(a in rbig(), c in nonzero_rbig()) {
        prop_assert_eq!((a.clone() / c.clone()) * c.clone(), a.clone());
    }

    /// Scaling numerator and denominator by a common factor yields the same
    /// canonical value (reduction idempotence), and the `Relaxed` view agrees.
    #[test]
    fn reduction_and_relaxed(n in -1000i64..=1000, d in 1u64..=1000, k in 1u64..=100) {
        let reduced = RBig::from_parts(IBig::from(n), UBig::from(d));
        let scaled = RBig::from_parts(IBig::from(n) * IBig::from(k as i64), UBig::from(d) * UBig::from(k));
        prop_assert_eq!(reduced.clone(), scaled.clone());
        prop_assert_eq!(reduced.as_relaxed(), scaled.as_relaxed());
    }
}
