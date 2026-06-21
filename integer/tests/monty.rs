//! Cross-validation of the Montgomery (`monty`) module against the Barrett-style
//! `modular::Reduced` implementation.

use dashu_int::{fast_div::ConstDivisor, monty::MontgomeryRepr, UBig};

/// A spread of odd moduli exercising the Single, Double and Large code paths.
fn moduli() -> Vec<UBig> {
    vec![
        UBig::from(3u8),
        UBig::from(101u8),
        UBig::from(0x1234_5679u32),
        UBig::from(u64::MAX),  // single word on 64-bit, double on 32-bit
        UBig::from(u128::MAX), // double word on 64-bit, large on 32-bit
        UBig::from(2u8).pow(127) - UBig::ONE, // Mersenne prime M127
        UBig::from(2u8).pow(521) - UBig::ONE, // Mersenne prime M521 (multi-word)
        UBig::from(2u8).pow(607) - UBig::ONE, // Mersenne prime M607 (multi-word)
        (UBig::from(2u8).pow(333) / 3u8) | UBig::ONE, // an arbitrary large odd modulus
    ]
}

/// Representative input values for a given modulus (some smaller, some larger than `m`).
fn values(m: &UBig) -> Vec<UBig> {
    vec![
        UBig::ZERO,
        UBig::ONE,
        UBig::from(2u8),
        UBig::from(7u8),
        UBig::from(123u32),
        m.clone() - UBig::ONE,
        m.clone() - UBig::from(2u8),
        m.clone() + UBig::from(5u8), // larger than m (reduce must wrap)
        m.clone() * 2u32 + UBig::from(3u8),
    ]
}

#[test]
fn cross_check_mul_sqr_add_sub() {
    for m in moduli() {
        let monty = MontgomeryRepr::new(m.clone());
        let barrett = ConstDivisor::new(m.clone());
        let vs = values(&m);

        for a in &vs {
            for b in &vs {
                let ma = monty.reduce(a.clone());
                let mb = monty.reduce(b.clone());
                let ba = barrett.reduce(a.clone());
                let bb = barrett.reduce(b.clone());

                assert_eq!((&ma * &mb).residue(), (&ba * &bb).residue(), "mul {a} {b} mod {m}");
                assert_eq!((&ma + &mb).residue(), (&ba + &bb).residue(), "add {a} {b} mod {m}");
                assert_eq!((&ma - &mb).residue(), (&ba - &bb).residue(), "sub {a} {b} mod {m}");
            }
            // squaring, negation, doubling
            let ma = monty.reduce(a.clone());
            let ba = barrett.reduce(a.clone());
            assert_eq!(ma.sqr().residue(), ba.sqr().residue(), "sqr {a} mod {m}");
            assert_eq!((-&ma).residue(), (-&ba).residue(), "neg {a} mod {m}");
            assert_eq!(ma.clone().dbl().residue(), ba.clone().dbl().residue(), "dbl {a} mod {m}");
        }
    }
}

#[test]
fn cross_check_pow() {
    let exponents = [
        UBig::ZERO,
        UBig::ONE,
        UBig::from(2u8),
        UBig::from(3u8),
        UBig::from(50u8),
    ];
    for m in moduli() {
        let monty = MontgomeryRepr::new(m.clone());
        let barrett = ConstDivisor::new(m.clone());
        for a in values(&m) {
            for e in &exponents {
                let ma = monty.reduce(a.clone());
                let ba = barrett.reduce(a.clone());
                assert_eq!(ma.pow(e).residue(), ba.pow(e).residue(), "pow {a}^{e} mod {m}");
            }
        }
    }
}

#[test]
fn cross_check_inv() {
    for m in moduli() {
        let monty = MontgomeryRepr::new(m.clone());
        let barrett = ConstDivisor::new(m.clone());
        for a in values(&m) {
            let ma = monty.reduce(a.clone());
            let ba = barrett.reduce(a.clone());
            match (ma.inv(), ba.inv()) {
                (Some(mi), Some(bi)) => {
                    assert_eq!(mi.residue(), bi.residue(), "inv {a} mod {m}");
                    // a * a^{-1} == 1
                    let one = monty.reduce(1u8);
                    assert_eq!((&ma * &mi).residue(), one.residue(), "inv product {a} mod {m}");
                }
                (None, None) => {}
                (mi, bi) => {
                    panic!("inv existence mismatch for {a} mod {m}: monty={mi:?} barrett={bi:?}")
                }
            }
        }
    }
}

#[test]
fn fermat_little_theorem() {
    // For a prime p and gcd(a, p) = 1: a^(p-1) == 1 (mod p).
    for p in [
        UBig::from(2u8).pow(127) - UBig::ONE,
        UBig::from(2u8).pow(607) - UBig::ONE,
    ] {
        let ring = MontgomeryRepr::new(p.clone());
        for a in [
            UBig::from(2u8),
            UBig::from(123u32),
            UBig::from(0xdeadbeefu32),
        ] {
            let ma = ring.reduce(a.clone());
            let result = ma.pow(&(p.clone() - UBig::ONE)).residue();
            assert_eq!(result, UBig::ONE, "Fermat failed for {a} mod {p}");
        }
    }
}

#[test]
fn round_trip_residue() {
    for m in moduli() {
        let ring = MontgomeryRepr::new(m.clone());
        for a in values(&m) {
            let expected = &a % &m;
            assert_eq!(ring.reduce(a.clone()).residue(), expected, "round-trip {a} mod {m}");
        }
    }
}

#[test]
fn mixed_rings_panic() {
    let m1 = UBig::from(101u8);
    let m2 = UBig::from(103u8);
    let r1 = MontgomeryRepr::new(m1);
    let r2 = MontgomeryRepr::new(m2);
    let a = r1.reduce(5u8);
    let b = r2.reduce(5u8);
    let result = std::panic::catch_unwind(|| {
        let _ = &a * &b;
    });
    assert!(result.is_err(), "multiplying across rings must panic");
}

#[test]
#[should_panic(expected = "Montgomery modulus must be odd")]
fn even_modulus_panics() {
    let _ = MontgomeryRepr::new(UBig::from(100u8));
}
