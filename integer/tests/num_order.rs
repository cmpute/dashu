use dashu_int::{IBig, UBig};
use num_order::{NumHash, NumOrd};

mod helper_macros;

#[test]
fn test_ord_with_float() {
    assert!(ubig!(0).num_eq(&0f32));
    assert!(ubig!(0).num_eq(&-0f32));
    assert!(ibig!(0).num_eq(&0f32));
    assert!(ibig!(0).num_eq(&-0f32));
    assert!(ubig!(1).num_eq(&1f32));
    assert!(ibig!(1).num_eq(&1f32));
    assert!(ubig!(1).num_ne(&-1f32));
    assert!(ibig!(-1).num_eq(&-1f32));

    assert!(ubig!(1).num_gt(&-1f32));
    assert!(ibig!(1).num_gt(&-1f32));
    assert!(ibig!(-1).num_gt(&-1.0001f32));
    assert!(ibig!(-100000).num_gt(&-100001f32));
    assert!(1f32.num_gt(&ibig!(-1)));
    assert!((-1.0001).num_le(&ibig!(-1)));
}

#[test]
fn test_ord_between_ubig_ibig() {
    assert!(ubig!(500).num_eq(&ibig!(500)));
    assert!(ubig!(500).num_ne(&ibig!(-500)));
    assert!(ibig!(500).num_eq(&ubig!(500)));
    assert!(ibig!(-500).num_ne(&ubig!(500)));

    assert!(ubig!(500).num_gt(&ibig!(499)));
    assert!(ibig!(500).num_gt(&ubig!(499)));
    assert!(ubig!(500).num_gt(&ibig!(-500)));
    assert!(ibig!(-500).num_le(&ubig!(500)));
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
    assert_eq!(hash(&ubig!(0)), hash(&ibig!(0)));
    assert_eq!(hash(&ubig!(1)), hash(&ibig!(1)));
    assert_ne!(hash(&ubig!(1)), hash(&ibig!(-1)));

    // small numbers
    let small_cases = [
        12i64,
        -123,
        1234,
        -12345,
        123456,
        -12345678,
        1234567890,
        -12345678901234,
        1234567890123456789,
    ];
    for v in small_cases {
        let i = IBig::from(v);
        assert_eq!(hash(&v), hash(&i));

        if let Ok(u) = UBig::try_from(v) {
            assert_eq!(hash(&u), hash(&v));
            assert_eq!(hash(&u), hash(&i));
        }
    }

    // large numbers
    let big_cases = [1e10f64, -1e20, 1e30, -1e40, 1e60, -1e100];
    for v in big_cases {
        let i = IBig::try_from(v).unwrap();
        assert_eq!(hash(&v), hash(&i));

        if let Ok(u) = UBig::try_from(v) {
            assert_eq!(hash(&u), hash(&v));
            assert_eq!(hash(&u), hash(&i));
        }
    }
}

#[test]
fn test_ord_with_small_float() {
    use core::cmp::Ordering::*;

    // floats in the range (0, 1)
    for f in [
        0.75f64,
        0.5,
        0.25,
        0.1,
        1e-300,
        f64::MIN_POSITIVE,
        f64::from_bits(1),
    ] {
        assert_eq!(ubig!(0).num_partial_cmp(&f), Some(Less), "0 vs {}", f);
        assert_eq!(ibig!(0).num_partial_cmp(&f), Some(Less), "0 vs {}", f);
        assert_eq!(f.num_partial_cmp(&ubig!(0)), Some(Greater), "{} vs 0", f);
        assert_eq!(f.num_partial_cmp(&ibig!(0)), Some(Greater), "{} vs 0", f);
        assert_eq!(ubig!(1).num_partial_cmp(&f), Some(Greater), "1 vs {}", f);
        assert_eq!(ibig!(1).num_partial_cmp(&f), Some(Greater), "1 vs {}", f);
        assert_eq!(ibig!(-1).num_partial_cmp(&f), Some(Less), "-1 vs {}", f);

        // and in the range (-1, 0)
        assert_eq!(ubig!(0).num_partial_cmp(&-f), Some(Greater), "0 vs {}", -f);
        assert_eq!(ibig!(0).num_partial_cmp(&-f), Some(Greater), "0 vs {}", -f);
        assert_eq!(ibig!(-1).num_partial_cmp(&-f), Some(Less), "-1 vs {}", -f);
        assert_eq!((-f).num_partial_cmp(&ibig!(-1)), Some(Greater), "{} vs -1", -f);
        assert_eq!(ibig!(1).num_partial_cmp(&-f), Some(Greater), "1 vs {}", -f);
    }
    for f in [
        0.75f32,
        0.5,
        0.25,
        0.1,
        1e-30,
        f32::MIN_POSITIVE,
        f32::from_bits(1),
    ] {
        assert_eq!(ubig!(0).num_partial_cmp(&f), Some(Less), "0 vs {}", f);
        assert_eq!(ibig!(0).num_partial_cmp(&f), Some(Less), "0 vs {}", f);
        assert_eq!(ubig!(1).num_partial_cmp(&f), Some(Greater), "1 vs {}", f);
        assert_eq!(ibig!(-1).num_partial_cmp(&-f), Some(Less), "-1 vs {}", -f);
        assert_eq!(ibig!(0).num_partial_cmp(&-f), Some(Greater), "0 vs {}", -f);
    }
    assert!(ibig!(0).num_lt(&f64::MIN_POSITIVE));
}

#[test]
fn test_ord_with_infinite_float() {
    use core::cmp::Ordering::*;

    let big = IBig::ONE << 2000;
    for x in [
        ibig!(0),
        ibig!(1),
        ibig!(5),
        ibig!(-1),
        ibig!(-5),
        big.clone(),
        -big,
    ] {
        assert_eq!(x.num_partial_cmp(&f64::INFINITY), Some(Less), "{:?} vs +inf", x);
        assert_eq!(x.num_partial_cmp(&f64::NEG_INFINITY), Some(Greater), "{:?} vs -inf", x);
        assert_eq!(x.num_partial_cmp(&f32::INFINITY), Some(Less), "{:?} vs +inf", x);
        assert_eq!(x.num_partial_cmp(&f32::NEG_INFINITY), Some(Greater), "{:?} vs -inf", x);
        assert_eq!(f64::INFINITY.num_partial_cmp(&x), Some(Greater), "+inf vs {:?}", x);
        assert_eq!(f64::NEG_INFINITY.num_partial_cmp(&x), Some(Less), "-inf vs {:?}", x);
        assert_eq!(x.num_partial_cmp(&f64::NAN), None);
    }
    for x in [ubig!(0), ubig!(5), UBig::ONE << 2000] {
        assert_eq!(x.num_partial_cmp(&f64::INFINITY), Some(Less));
        assert_eq!(x.num_partial_cmp(&f64::NEG_INFINITY), Some(Greater));
        assert_eq!(f32::INFINITY.num_partial_cmp(&x), Some(Greater));
        assert_eq!(f32::NEG_INFINITY.num_partial_cmp(&x), Some(Less));
    }
}

#[test]
fn test_ord_with_float_matches_exact_conversion() {
    // every integer here is exactly representable by f64, so the
    // comparison between floats is an exact reference
    let ints = [-1025i64, -1024, -3, -2, -1, 0, 1, 2, 3, 1024, 1025, 1 << 40];
    let floats = [
        -1e300, -1025.5, -1024.0, -2.5, -1.0, -0.5, -1e-300, 1e-300, 0.5, 1.0, 2.5, 1024.0, 1024.5,
        1e300,
    ];
    for i in ints {
        for f in floats {
            let expected = (i as f64).partial_cmp(&f);
            assert_eq!(IBig::from(i).num_partial_cmp(&f), expected, "{} vs {}", i, f);
            assert_eq!(f.num_partial_cmp(&IBig::from(i)), expected.map(|o| o.reverse()));
            if i >= 0 {
                assert_eq!(UBig::from(i as u64).num_partial_cmp(&f), expected, "{} vs {}", i, f);
            }
        }
    }
}
