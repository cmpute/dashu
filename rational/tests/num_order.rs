use dashu_float::{DBig, FBig};
use dashu_ratio::RBig;
use num_order::{NumHash, NumOrd};

mod helper_macros;

type FBin = FBig;

#[test]
fn test_ord_with_ubig_ibig() {
    assert!(rbig!(0).num_eq(&ubig!(0)));
    assert!(rbig!(0).num_eq(&ibig!(0)));
    assert!(rbig!(0).num_le(&ubig!(1)));
    assert!(rbig!(0).num_le(&ibig!(1)));
    assert!(rbig!(0).num_ge(&ibig!(-1)));
    assert!(rbig!(1).num_eq(&ubig!(1)));
    assert!(rbig!(1).num_eq(&ibig!(1)));
    assert!(rbig!(1).num_ge(&ibig!(-1)));
    assert!(rbig!(-1).num_le(&ubig!(1)));
    assert!(rbig!(-1).num_le(&ibig!(1)));
    assert!(rbig!(-1).num_eq(&ibig!(-1)));
    assert!(rbig!(-1).num_eq(&ibig!(-1)));

    assert!(ubig!(0).num_eq(&rbig!(0)));
    assert!(ubig!(0).num_le(&rbig!(1)));
    assert!(ubig!(0).num_ge(&rbig!(-1)));
    assert!(ubig!(1).num_eq(&rbig!(1)));
    assert!(ubig!(1).num_ge(&rbig!(-1)));

    assert!(ibig!(0).num_eq(&rbig!(0)));
    assert!(ibig!(0).num_le(&rbig!(1)));
    assert!(ibig!(0).num_ge(&rbig!(-1)));
    assert!(ibig!(1).num_eq(&rbig!(1)));
    assert!(ibig!(1).num_ge(&rbig!(-1)));
    assert!(ibig!(-1).num_le(&rbig!(1)));
    assert!(ibig!(-1).num_eq(&rbig!(-1)));
    assert!(ibig!(-1).num_eq(&rbig!(-1)));

    assert!(rbig!(1 / 10).num_ge(&ubig!(0)));
    assert!(rbig!(1 / 10).num_ge(&ibig!(0)));
    assert!(rbig!(1 / 10).num_le(&ubig!(1)));
    assert!(rbig!(1 / 10).num_le(&ibig!(1)));
    assert!(rbig!(-1 / 10).num_ge(&ibig!(-1)));
    assert!(rbig!(-1 / 10).num_le(&ubig!(0)));
    assert!(rbig!(-1 / 10).num_le(&ibig!(0)));
    assert!(rbig!(999999 / 100).num_ge(&ubig!(9999)));
    assert!(rbig!(999999 / 100).num_le(&ubig!(10000)));
    assert!(rbig!(-999999 / 100).num_le(&ibig!(-9999)));
    assert!(rbig!(-999999 / 100).num_ge(&ibig!(-10000)));
}

#[test]
fn test_ord_with_fbig() {
    assert!(rbig!(0).num_eq(&FBin::ZERO));
    assert!(rbig!(0).num_eq(&DBig::ZERO));
    assert!(rbig!(0).num_le(&FBin::ONE));
    assert!(rbig!(0).num_le(&DBig::ONE));
    assert!(rbig!(0).num_ge(&FBin::NEG_ONE));
    assert!(rbig!(0).num_ge(&DBig::NEG_ONE));
    assert!(rbig!(1).num_eq(&FBin::ONE));
    assert!(rbig!(1).num_eq(&DBig::ONE));
    assert!(rbig!(1).num_ge(&FBin::NEG_ONE));
    assert!(rbig!(1).num_ge(&DBig::NEG_ONE));
    assert!(rbig!(-1).num_le(&FBin::ONE));
    assert!(rbig!(-1).num_le(&DBig::ONE));
    assert!(rbig!(-1).num_eq(&FBin::NEG_ONE));
    assert!(rbig!(-1).num_eq(&DBig::NEG_ONE));

    assert!(rbig!(1 / 2).num_eq(&FBin::from_str_native("0x1p-1").unwrap()));
    assert!(rbig!(-9 / 2).num_eq(&FBin::from_str_native("-0x9p-1").unwrap()));
    assert!(rbig!(1 / 1024).num_eq(&FBin::from_str_native("0x1p-10").unwrap()));
    assert!(rbig!(1 / 1267650600228229401496703205376)
        .num_eq(&FBin::from_str_native("0x1p-100").unwrap()));
    assert!(rbig!(1 / 10).num_eq(&DBig::from_str_native("0.1").unwrap()));
    assert!(rbig!(-11 / 10).num_eq(&DBig::from_str_native("-1.1").unwrap()));
    assert!(rbig!(1 / 9765625).num_eq(&DBig::from_str_native("1.024e-7").unwrap()));
    assert!(
        rbig!(1 / 7888609052210118054117285652827862296732064351090230047702789306640625)
            .num_eq(&DBig::from_str_native("1.267650600228229401496703205376e-70").unwrap())
    );

    assert!(rbig!(1 / 3).num_ge(&FBin::from_str_native("0x55555p-20").unwrap()));
    assert!(rbig!(1 / 3).num_le(&FBin::from_str_native("0x55556p-20").unwrap()));
    assert!(rbig!(-1 / 3).num_ge(&DBig::from_str_native("-0.33334").unwrap()));
    assert!(rbig!(-1 / 3).num_le(&DBig::from_str_native("-0.33333").unwrap()));
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
        dbg!(v, &r);
        assert_eq!(hash(&v), hash(&r));
    }
}
