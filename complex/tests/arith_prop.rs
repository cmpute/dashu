//! Arithmetic identity property tests (exact identities for finite operands).
//!
//! Tolerance-based correctness (the self-oracle) lives in `rounding_prop.rs`.

use dashu_cmplx::{CBig, FBig};
use dashu_float::round::mode::HalfEven;
use proptest::prelude::*;

type C = CBig<HalfEven, 2>;
type F = FBig<HalfEven, 2>;

const P: usize = 53;

fn fbig_strategy() -> impl Strategy<Value = F> {
    (-(1i64 << 20)..(1i64 << 20), -10isize..10isize).prop_map(|(sig, exp)| {
        if sig == 0 {
            F::ZERO.with_precision(P).value()
        } else {
            F::from_parts(sig.into(), exp).with_precision(P).value()
        }
    })
}

fn cbig_strategy() -> impl Strategy<Value = C> {
    (fbig_strategy(), fbig_strategy()).prop_map(|(re, im)| CBig::from_parts(re, im))
}

proptest! {
    #[test]
    fn add_commutes((z, w) in (cbig_strategy(), cbig_strategy())) {
        prop_assert!(&z + &w == &w + &z);
    }

    #[test]
    fn add_zero_identity(z in cbig_strategy()) {
        let zero = CBig::from(F::ZERO);
        prop_assert!(&z + &zero == z);
    }

    #[test]
    fn sub_self_is_zero(z in cbig_strategy()) {
        prop_assert!((&z - &z).is_zero());
    }

    #[test]
    fn mul_commutes((z, w) in (cbig_strategy(), cbig_strategy())) {
        // the 4-mul formula is symmetric, so z·w and w·z round identically
        prop_assert!(&z * &w == &w * &z);
    }

    #[test]
    fn mul_one_identity(z in cbig_strategy()) {
        prop_assert!(&z * &CBig::ONE == z);
    }

    #[test]
    fn mul_zero_is_zero(z in cbig_strategy()) {
        let zero = CBig::from(F::ZERO);
        prop_assert!((&z * &zero).is_zero());
    }

    #[test]
    fn mul_i_fourth_is_identity(z in cbig_strategy()) {
        let id = z.mul_i(false).mul_i(false).mul_i(false).mul_i(false);
        prop_assert!(id == z);
    }

    #[test]
    fn conj_involution(z in cbig_strategy()) {
        prop_assert!(z.conj().conj() == z);
    }

    #[test]
    fn proj_idempotent_finite(z in cbig_strategy()) {
        prop_assert!(z.proj().proj() == z.proj());
    }

    #[test]
    fn neg_is_additive_inverse(z in cbig_strategy()) {
        prop_assert!((&z + &(-&z)).is_zero());
    }
}
