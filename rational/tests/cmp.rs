#![allow(clippy::nonminimal_bool)]

use dashu_base::AbsOrd;
use dashu_float::{DBig, FBig};
use dashu_ratio::{RBig, Relaxed};
use std::str::FromStr;
use std::{
    cmp::Ordering,
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

type FBin = FBig;

mod helper_macros;

fn hash<T>(x: &T) -> u64
where
    T: Hash,
{
    let mut hasher = DefaultHasher::new();
    x.hash(&mut hasher);
    hasher.finish()
}

#[test]
fn test_eq_hash() {
    // simple cases
    assert_eq!(rbig!(0), rbig!(0));
    assert_eq!(rbig!(~0), rbig!(~0));
    assert_eq!(rbig!(~0), rbig!(~0/4));
    assert_ne!(rbig!(0), rbig!(-1));
    assert_ne!(rbig!(~0), rbig!(~-1));
    assert_ne!(rbig!(~0/4), rbig!(~-1));
    assert_ne!(rbig!(0), rbig!(1 / 2));
    assert_ne!(rbig!(~0), rbig!(~1/2));
    assert_eq!(rbig!(1), rbig!(1));
    assert_eq!(rbig!(~1), rbig!(~1));
    assert_ne!(rbig!(1), rbig!(-1));
    assert_ne!(rbig!(~1), rbig!(~-1));
    assert_eq!(rbig!(-1), rbig!(-2 / 2));
    assert_eq!(rbig!(~-1), rbig!(~-2/2));
    assert_eq!(rbig!(~4/2), rbig!(~2/1));
    assert_eq!(rbig!(~4/2), rbig!(~2/1));
    assert_eq!(rbig!(~-2/4), rbig!(~-1/2));
    assert_eq!(rbig!(~9/3), rbig!(~3/1));
    assert_eq!(rbig!(~-3/9), rbig!(~-1/3));
    assert_ne!(rbig!(~3/10), rbig!(~1/3));

    // test with reduction
    let r = RBig::from_parts(ibig!(-1) << 1000, ubig!(3).pow(250));
    let h = hash(&r);
    for i in 0..=250 {
        // test eq on Relaxed
        let r2 = Relaxed::from_parts(
            (ibig!(-1) << (i * 4) << (1000 - i * 4)) * ibig!(5).pow(i / 2),
            ubig!(3).pow(i) * ubig!(3).pow(250 - i) * ubig!(5).pow(i / 2),
        );
        assert_eq!(r2, r.clone().relax());

        // test eq and hash on RBig
        let r2 = r2.canonicalize();
        assert_eq!(r2, r);

        let h2 = hash(&r2);
        assert_eq!(h2, h);
    }

    let r3 = RBig::from_parts(ibig!(-1) << 1000, ubig!(3).pow(25));
    assert_ne!(r3, r);

    let h3 = hash(&r3);
    assert_ne!(h3, h);
}

#[test]
fn test_cmp() {
    // case 1: compare sign
    assert!(rbig!(1) > rbig!(-1));
    assert!(rbig!(-2) < rbig!(1 / 2));
    assert!(rbig!(~1) > rbig!(~-1));
    assert!(rbig!(~-2) < rbig!(~2/4));

    // case 2: compare integers and with zero
    assert!(!(rbig!(0) > rbig!(0)));
    assert!(rbig!(1) > rbig!(0));
    assert!(rbig!(1 / 10) > rbig!(0));
    assert!(rbig!(~1/10) > rbig!(~0));
    assert!(rbig!(10) > rbig!(1));
    assert!(rbig!(-10) < rbig!(-1));
    assert!(rbig!(~-10) < rbig!(~-1));
    assert!(rbig!(1) < rbig!(10));
    assert!(rbig!(-1) > rbig!(-10));
    assert!(rbig!(~-1) > rbig!(~-10));

    // case 3: compare by bits
    assert!(rbig!(1 / 1000) < rbig!(1 / 10));
    assert!(rbig!(-1 / 1000) > rbig!(-1 / 10));
    assert!(rbig!(~1/1000) < rbig!(~1/10));
    assert!(rbig!(~-1/1000) > rbig!(~-1/10));
    assert!(rbig!(1000 / 3) > rbig!(300 / 5));
    assert!(rbig!(-1000 / 3) < rbig!(-300 / 5));
    assert!(rbig!(~1000/3) > rbig!(~300/5));
    assert!(rbig!(~-1000/3) < rbig!(~-300/5));

    // case 4: compare by multiplication
    assert!(rbig!(22 / 7) > rbig!(355 / 113));
    assert!(rbig!(113 / 36) < rbig!(355 / 113));
    assert!(rbig!(-22 / 7) < rbig!(-355 / 113));
    assert!(rbig!(-113 / 36) > rbig!(-355 / 113));
    assert!(rbig!(~-22/7) < rbig!(~-355/113));
    assert!(rbig!(~-113/36) > rbig!(~-355/113));
}

#[test]
fn test_abs_ord() {
    let ubig_cases = [
        (rbig!(0), ubig!(0), Ordering::Equal),
        (rbig!(0), ubig!(1), Ordering::Less),
        (rbig!(1), ubig!(1), Ordering::Equal),
        (rbig!(-1), ubig!(1), Ordering::Equal),
        (rbig!(1 / 10), ubig!(0), Ordering::Greater),
        (rbig!(1 / 10), ubig!(1), Ordering::Less),
        (rbig!(-1 / 10), ubig!(0), Ordering::Greater),
        (rbig!(-1 / 10), ubig!(1), Ordering::Less),
        (rbig!(999999 / 100), ubig!(9999), Ordering::Greater),
        (rbig!(999999 / 100), ubig!(10000), Ordering::Less),
        (rbig!(-999999 / 100), ubig!(9999), Ordering::Greater),
        (rbig!(-999999 / 100), ubig!(10000), Ordering::Less),
    ];

    for (r, i, order) in ubig_cases {
        assert_eq!(r.abs_cmp(&i), order);
        assert_eq!(i.abs_cmp(&r), order.reverse());
    }

    let ibig_cases = [
        (rbig!(0), ibig!(0), Ordering::Equal),
        (rbig!(0), ibig!(1), Ordering::Less),
        (rbig!(0), ibig!(-1), Ordering::Less),
        (rbig!(1), ibig!(1), Ordering::Equal),
        (rbig!(1), ibig!(-1), Ordering::Equal),
        (rbig!(-1), ibig!(1), Ordering::Equal),
        (rbig!(-1), ibig!(-1), Ordering::Equal),
        (rbig!(1 / 10), ibig!(0), Ordering::Greater),
        (rbig!(1 / 10), ibig!(1), Ordering::Less),
        (rbig!(-1 / 10), ibig!(0), Ordering::Greater),
        (rbig!(-1 / 10), ibig!(-1), Ordering::Less),
        (rbig!(-999999 / 100), ibig!(9999), Ordering::Greater),
        (rbig!(-999999 / 100), ibig!(10000), Ordering::Less),
        (rbig!(-999999 / 100), ibig!(-9999), Ordering::Greater),
        (rbig!(-999999 / 100), ibig!(-10000), Ordering::Less),
    ];

    for (r, i, order) in ibig_cases {
        assert_eq!(r.abs_cmp(&i), order);
        assert_eq!(i.abs_cmp(&r), order.reverse());
    }

    let fbig_cases = [
        (rbig!(0), FBin::ZERO, Ordering::Equal),
        (rbig!(0), FBin::ONE, Ordering::Less),
        (rbig!(0), FBin::NEG_ONE, Ordering::Less),
        (rbig!(1), FBin::ONE, Ordering::Equal),
        (rbig!(1), FBin::NEG_ONE, Ordering::Equal),
        (rbig!(-1), FBin::ONE, Ordering::Equal),
        (rbig!(-1), FBin::NEG_ONE, Ordering::Equal),
        (rbig!(1 / 2), FBin::from_str("-0x1p-1").unwrap(), Ordering::Equal),
        (rbig!(-9 / 2), FBin::from_str("-0x9p-1").unwrap(), Ordering::Equal),
        (rbig!(-1 / 1024), FBin::from_str("0x1p-10").unwrap(), Ordering::Equal),
        (
            rbig!(1 / 1267650600228229401496703205376),
            FBin::from_str("0x1p-100").unwrap(),
            Ordering::Equal,
        ),
    ];

    for (r, f, order) in fbig_cases {
        assert_eq!(r.abs_cmp(&f), order);
        assert_eq!(f.abs_cmp(&r), order.reverse());
    }

    let dbig_cases = [
        (rbig!(0), DBig::ZERO, Ordering::Equal),
        (rbig!(0), DBig::ONE, Ordering::Less),
        (rbig!(0), DBig::NEG_ONE, Ordering::Less),
        (rbig!(1), DBig::ONE, Ordering::Equal),
        (rbig!(1), DBig::NEG_ONE, Ordering::Equal),
        (rbig!(-1), DBig::ONE, Ordering::Equal),
        (rbig!(-1), DBig::NEG_ONE, Ordering::Equal),
        (rbig!(1 / 10), DBig::from_str("-0.1").unwrap(), Ordering::Equal),
        (rbig!(-11 / 10), DBig::from_str("-1.1").unwrap(), Ordering::Equal),
        (rbig!(-1 / 9765625), DBig::from_str("1.024e-7").unwrap(), Ordering::Equal),
        (
            rbig!(1 / 7888609052210118054117285652827862296732064351090230047702789306640625),
            DBig::from_str("1.267650600228229401496703205376e-70").unwrap(),
            Ordering::Equal,
        ),
    ];

    for (r, d, order) in dbig_cases {
        assert_eq!(r.abs_cmp(&d), order, "{}, {}", r, d);
        assert_eq!(d.abs_cmp(&r), order.reverse());
    }
}
