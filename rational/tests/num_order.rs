use std::cmp::Ordering;

use dashu_float::{DBig, FBig};
use dashu_ratio::RBig;
use num_order::{NumHash, NumOrd};

mod helper_macros;

type FBin = FBig;

#[test]
fn test_ord_with_ubig_ibig() {
    let ubig_cases = [
        (rbig!(0), ubig!(0), Ordering::Equal),
        (rbig!(0), ubig!(1), Ordering::Less),
        (rbig!(1), ubig!(1), Ordering::Equal),
        (rbig!(-1), ubig!(1), Ordering::Less),
        (rbig!(1 / 10), ubig!(0), Ordering::Greater),
        (rbig!(1 / 10), ubig!(1), Ordering::Less),
        (rbig!(-1 / 10), ubig!(0), Ordering::Less),
        (rbig!(999999 / 100), ubig!(9999), Ordering::Greater),
        (rbig!(999999 / 100), ubig!(10000), Ordering::Less),
    ];

    for (r, i, order) in ubig_cases {
        assert_eq!(r.num_cmp(&i), order, "{}, {}", r, i);
        assert_eq!(i.num_cmp(&r), order.reverse());
    }

    let ibig_cases = [
        (rbig!(0), ibig!(0), Ordering::Equal),
        (rbig!(0), ibig!(1), Ordering::Less),
        (rbig!(0), ibig!(-1), Ordering::Greater),
        (rbig!(1), ibig!(1), Ordering::Equal),
        (rbig!(1), ibig!(-1), Ordering::Greater),
        (rbig!(-1), ibig!(1), Ordering::Less),
        (rbig!(-1), ibig!(-1), Ordering::Equal),
        (rbig!(1 / 10), ibig!(0), Ordering::Greater),
        (rbig!(1 / 10), ibig!(1), Ordering::Less),
        (rbig!(-1 / 10), ibig!(0), Ordering::Less),
        (rbig!(-1 / 10), ibig!(-1), Ordering::Greater),
        (rbig!(999999 / 100), ibig!(9999), Ordering::Greater),
        (rbig!(999999 / 100), ibig!(10000), Ordering::Less),
        (rbig!(-999999 / 100), ibig!(-9999), Ordering::Less),
        (rbig!(-999999 / 100), ibig!(-10000), Ordering::Greater),
    ];

    for (r, i, order) in ibig_cases {
        assert_eq!(r.num_cmp(&i), order, "{}, {}", r, i);
        assert_eq!(i.num_cmp(&r), order.reverse());
    }
}

#[test]
fn test_ord_with_fbig() {
    let fbig_cases = [
        (rbig!(0), FBin::ZERO, Ordering::Equal),
        (rbig!(0), FBin::ONE, Ordering::Less),
        (rbig!(0), FBin::NEG_ONE, Ordering::Greater),
        (rbig!(1), FBin::ONE, Ordering::Equal),
        (rbig!(1), FBin::NEG_ONE, Ordering::Greater),
        (rbig!(-1), FBin::ONE, Ordering::Less),
        (rbig!(-1), FBin::NEG_ONE, Ordering::Equal),
        (rbig!(1 / 2), FBin::from_str_native("0x1p-1").unwrap(), Ordering::Equal),
        (rbig!(-9 / 2), FBin::from_str_native("-0x9p-1").unwrap(), Ordering::Equal),
        (rbig!(1 / 1024), FBin::from_str_native("0x1p-10").unwrap(), Ordering::Equal),
        (
            rbig!(1 / 1267650600228229401496703205376),
            FBin::from_str_native("0x1p-100").unwrap(),
            Ordering::Equal,
        ),
        (rbig!(1 / 3), FBin::from_str_native("0x55555p-20").unwrap(), Ordering::Greater),
        (rbig!(1 / 3), FBin::from_str_native("0x55556p-20").unwrap(), Ordering::Less),
    ];
    for (r, f, ord) in fbig_cases {
        assert_eq!(r.num_cmp(&f), ord);
        assert_eq!(f.num_cmp(&r), ord.reverse());
    }

    let dbig_cases = [
        (rbig!(0), DBig::ZERO, Ordering::Equal),
        (rbig!(0), DBig::ONE, Ordering::Less),
        (rbig!(0), DBig::NEG_ONE, Ordering::Greater),
        (rbig!(1), DBig::ONE, Ordering::Equal),
        (rbig!(1), DBig::NEG_ONE, Ordering::Greater),
        (rbig!(-1), DBig::ONE, Ordering::Less),
        (rbig!(-1), DBig::NEG_ONE, Ordering::Equal),
        (rbig!(1 / 10), DBig::from_str_native("0.1").unwrap(), Ordering::Equal),
        (rbig!(-11 / 10), DBig::from_str_native("-1.1").unwrap(), Ordering::Equal),
        (rbig!(1 / 9765625), DBig::from_str_native("1.024e-7").unwrap(), Ordering::Equal),
        (
            rbig!(1 / 7888609052210118054117285652827862296732064351090230047702789306640625),
            DBig::from_str_native("1.267650600228229401496703205376e-70").unwrap(),
            Ordering::Equal,
        ),
        (rbig!(-1 / 3), DBig::from_str_native("-0.33334").unwrap(), Ordering::Greater),
        (rbig!(-1 / 3), DBig::from_str_native("-0.33333").unwrap(), Ordering::Less),
    ];
    for (r, d, ord) in dbig_cases {
        assert_eq!(r.num_cmp(&d), ord);
        assert_eq!(d.num_cmp(&r), ord.reverse());
    }
}

#[test]
fn test_hash() {
    fn hash<T: NumHash>(value: &T) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::Hasher;
        let mut hasher = DefaultHasher::new();
        value.num_hash(&mut hasher);
        hasher.finish()
    }

    // trivial cases
    assert_eq!(hash(&rbig!(0)), hash(&ibig!(0)));
    assert_eq!(hash(&rbig!(1)), hash(&ibig!(1)));
    assert_ne!(hash(&rbig!(-1)), hash(&ibig!(1)));
    assert_eq!(hash(&rbig!(12345)), hash(&ibig!(12345)));

    // f64 numbers that are exact representable
    let small_cases = [
        -1.25,
        46.515625,
        -79808794.80078125,
        6343071834.078125,
        -13095725861.65625,
    ];
    for v in small_cases {
        let r = RBig::simplest_from_f64(v).unwrap();
        assert_eq!(hash(&v), hash(&r));
    }
}
