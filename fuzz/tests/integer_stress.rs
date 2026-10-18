//! Large integer campaigns moved out of the routine random integration tests.
//!
//! Each proptest case runs both the 100,000- and 1,000,000-bit sizes. GCD has a
//! separate budget so its quadratic cost cannot dominate the arithmetic campaign.
//! All properties are exact algebraic checks; no approximate oracle is used.
//!
//! Reproduce a smoke run with:
//! `FUZZ_CASES=1 FUZZ_SEED=3 cargo test --release --manifest-path fuzz/Cargo.toml \
//! --test integer_stress -- --ignored --test-threads=1`
//! Set `RUSTFLAGS='--cfg force_bits="16"'` (or 32/64) to exercise each Word width.

use dashu::base::{DivRem, ExtendedGcd, Gcd};
use dashu::integer::{UBig, fast_div::ConstDivisor};
use proptest::prelude::*;
use rand::{RngExt, SeedableRng, rngs::StdRng};

proptest! {
    #![proptest_config(fuzz::fuzz_config())]

    #[test]
    #[ignore]
    fn large_arithmetic(seed in any::<u64>()) {
        let mut rng = StdRng::seed_from_u64(seed);
        let p = UBig::from(1_000_000_007u32);
        for bits in [100_000, 1_000_000] {
            let a_bits = bits;
            let b_bits = rng.random_range(10..=bits);
            let a = fuzz::random_ubig_bits(&mut rng, a_bits);
            let b = fuzz::random_ubig_bits(&mut rng, b_bits);
            let c = fuzz::random_ubig_bits(&mut rng, a_bits) % &a;
            let radix = rng.random_range(2u8..=36);
            assert_eq!(
                UBig::from_str_radix(&a.in_radix(radix).to_string(), radix as u32).unwrap(),
                a
            );
            assert_eq!((&a + &b) % &p, ((&a % &p) + (&b % &p)) % &p);
            assert_eq!(&a + &b - &a, b);
            let product = &a * &b;
            assert_eq!(&product % &p, ((&a % &p) * (&b % &p)) % &p);
            assert_eq!((product + &c).div_rem(&a), (b.clone(), c));
            assert_eq!((&a + 1u8) * (&a - 1u8), a.sqr() - 1u8);
        }
        // The old loop repeated this same identity for every eighth sample.
        let exponent = 100_000;
        assert_eq!((UBig::from(5u8).pow(exponent) + 1u8).ilog(&UBig::from(25u8)), exponent / 2);
    }

    #[test]
    #[ignore]
    fn large_gcd(seed in any::<u64>()) {
        let mut rng = StdRng::seed_from_u64(seed);
        for bits in [100_000, 1_000_000] {
            let a = fuzz::random_ubig_bits(&mut rng, bits);
            let b = fuzz::random_ubig_bits(&mut rng, bits);
            let (g, ca, cb) = (&a).gcd_ext(&b);
            assert_eq!(g, (&a).gcd(&b));
            assert_eq!(&a % &g, UBig::ZERO);
            assert_eq!(&b % &g, UBig::ZERO);
            assert_eq!(a * ca + b * cb, g.into());
        }
    }

    #[test]
    #[ignore]
    fn large_modular_arithmetic(seed in any::<u64>()) {
        let mut rng = StdRng::seed_from_u64(seed);
        let p = ConstDivisor::new(UBig::from(1_000_000_007u32));
        let one = p.reduce(UBig::ONE);
        let zero = p.reduce(UBig::ZERO);
        for bits in [100_000, 1_000_000] {
            let a = p.reduce(fuzz::random_ubig_bits(&mut rng, bits));
            let b = p.reduce(fuzz::random_ubig_bits(&mut rng, bits));
            assert_eq!(&a + &b - &a, b);
            // Reduction can map a nonzero input to zero, which has no inverse.
            if a != zero {
                assert_eq!(&a * &b / &a, b);
            }
            assert_eq!((&a + &one) * (&a - &one), a.sqr() - &one);
        }
    }
}
