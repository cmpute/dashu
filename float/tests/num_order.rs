use dashu_float::{DBig, FBig};
use num_order::{NumHash, NumOrd};

mod helper_macros;

type FBin = FBig;

#[test]
#[rustfmt::skip::macros(fbig)]
fn test_ord_between_fbig() {
    assert!(fbig!(0).num_eq(&dbig!(0)));
    assert!(fbig!(0).num_le(&dbig!(1)));
    assert!(fbig!(0).num_ge(&dbig!(-1)));
    assert!(fbig!(1).num_eq(&dbig!(1)));
    assert!(fbig!(1).num_ge(&dbig!(-1)));
    assert!(fbig!(-1).num_le(&dbig!(1)));
    assert!(fbig!(-1).num_eq(&dbig!(-1)));
    assert!(fbig!(-1).num_eq(&dbig!(-1)));
    assert!(FBin::INFINITY.num_eq(&DBig::INFINITY));
    assert!(FBin::INFINITY.num_ge(&dbig!(0)));
    assert!(FBin::INFINITY.num_ge(&DBig::NEG_INFINITY));
    assert!(FBin::NEG_INFINITY.num_le(&DBig::INFINITY));
    assert!(FBin::NEG_INFINITY.num_le(&dbig!(0)));
    assert!(FBin::NEG_INFINITY.num_eq(&DBig::NEG_INFINITY));

    assert!(fbig!(0x1p-1).num_eq(&dbig!(5e-1)));
    assert!(fbig!(0x1p-1).num_ge(&dbig!(1e-1)));
    assert!(fbig!(0x1p-1).num_ge(&dbig!(-5e-1)));
    assert!(fbig!(-0x1p-1).num_le(&dbig!(5e-1)));
    assert!(fbig!(-0x1p-1).num_le(&dbig!(-1e-1)));
    assert!(fbig!(-0x1p-1).num_eq(&dbig!(-5e-1)));

    assert!(fbig!(0x123456p-100).num_le(&dbig!(123456)));
    assert!(fbig!(0x123456p100).num_ge(&dbig!(123456)));
    assert!(fbig!(-0x123456p-100).num_le(&dbig!(123456)));
    assert!(fbig!(-0x123456p100).num_le(&dbig!(123456)));
    assert!(fbig!(-0x123456p-100).num_ge(&dbig!(-123456)));
    assert!(fbig!(-0x123456p100).num_le(&dbig!(-123456)));

    assert!(fbig!(0x1p-10).num_ge(&dbig!(9765624e-10)));
    assert!(fbig!(0x1p-10).num_eq(&dbig!(9765625e-10)));
    assert!(fbig!(0x1p-10).num_le(&dbig!(9765626e-10)));
    assert!(fbig!(0x1p-50).num_ge(&dbig!(88817841970012523233890533447265624e-50)));
    assert!(fbig!(0x1p-50).num_eq(&dbig!(88817841970012523233890533447265625e-50)));
    assert!(fbig!(0x1p-50).num_le(&dbig!(88817841970012523233890533447265626e-50)));
}

#[test]
#[rustfmt::skip::macros(fbig)]
fn test_ord_with_ubig_ibig() {
    assert!(fbig!(0).num_eq(&ubig!(0)));
    assert!(fbig!(0).num_eq(&ibig!(0)));
    assert!(fbig!(0).num_le(&ubig!(1)));
    assert!(fbig!(0).num_le(&ibig!(1)));
    assert!(fbig!(0).num_ge(&ibig!(-1)));
    assert!(fbig!(1).num_eq(&ubig!(1)));
    assert!(fbig!(1).num_eq(&ibig!(1)));
    assert!(fbig!(1).num_ge(&ibig!(-1)));
    assert!(fbig!(-1).num_le(&ubig!(1)));
    assert!(fbig!(-1).num_le(&ibig!(1)));
    assert!(fbig!(-1).num_eq(&ibig!(-1)));
    assert!(fbig!(-1).num_eq(&ibig!(-1)));
    assert!(FBin::INFINITY.num_ge(&ubig!(0)));
    assert!(FBin::INFINITY.num_ge(&ibig!(0)));
    assert!(FBin::NEG_INFINITY.num_le(&ubig!(0)));
    assert!(FBin::NEG_INFINITY.num_le(&ibig!(0)));

    assert!(ubig!(0).num_eq(&fbig!(0)));
    assert!(ubig!(0).num_le(&fbig!(1)));
    assert!(ubig!(0).num_ge(&fbig!(-1)));
    assert!(ubig!(1).num_eq(&fbig!(1)));
    assert!(ubig!(1).num_ge(&fbig!(-1)));
    assert!(ubig!(0).num_le(&FBin::INFINITY));
    assert!(ubig!(0).num_ge(&FBin::NEG_INFINITY));

    assert!(ibig!(0).num_eq(&fbig!(0)));
    assert!(ibig!(0).num_le(&fbig!(1)));
    assert!(ibig!(0).num_ge(&fbig!(-1)));
    assert!(ibig!(1).num_eq(&fbig!(1)));
    assert!(ibig!(1).num_ge(&fbig!(-1)));
    assert!(ibig!(-1).num_le(&fbig!(1)));
    assert!(ibig!(-1).num_eq(&fbig!(-1)));
    assert!(ibig!(-1).num_eq(&fbig!(-1)));
    assert!(ibig!(0).num_le(&FBin::INFINITY));
    assert!(ibig!(0).num_ge(&FBin::NEG_INFINITY));

    assert!(fbig!(0x1p-10).num_ge(&ubig!(0)));
    assert!(fbig!(0x1p-10).num_ge(&ibig!(0)));
    assert!(fbig!(0x1p-10).num_le(&ubig!(1)));
    assert!(fbig!(0x1p-10).num_le(&ibig!(1)));
    assert!(fbig!(-0x1p-10).num_ge(&ibig!(-1)));
    assert!(fbig!(-0x1p-10).num_le(&ubig!(0)));
    assert!(fbig!(-0x1p-10).num_le(&ibig!(0)));
    assert!(fbig!(0x1p10).num_ge(&ubig!(0x399)));
    assert!(fbig!(0x1p10).num_eq(&ubig!(0x400)));
    assert!(fbig!(0x1p10).num_le(&ubig!(0x401)));
    assert!(fbig!(-0x1p10).num_le(&ibig!(-0x399)));
    assert!(fbig!(-0x1p10).num_eq(&ibig!(-0x400)));
    assert!(fbig!(-0x1p10).num_ge(&ibig!(-0x401)));
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
    assert_eq!(hash(&fbig!(0)), hash(&ibig!(0)));
    assert_eq!(hash(&fbig!(1)), hash(&ibig!(1)));
    assert_ne!(hash(&fbig!(-1)), hash(&ibig!(1)));
    assert_eq!(hash(&FBin::INFINITY), hash(&f32::INFINITY));
    assert_eq!(hash(&FBin::NEG_INFINITY), hash(&f32::NEG_INFINITY));

    // f64 numbers
    let small_cases = [
        12f64,
        -12.3,
        1234.,
        -12345.,
        1.23456,
        -12345.678,
        12.34567890,
        -0.012345678901234,
    ];
    for v in small_cases {
        let i = FBin::try_from(v).unwrap();
        assert_eq!(hash(&v), hash(&i));
    }
}
