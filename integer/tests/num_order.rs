use dashu_int::{IBig, UBig};
use num_order::{NumOrd, NumHash};

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
        use std::hash::Hasher;
        use std::collections::hash_map::DefaultHasher;
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
        12i64, -123, 1234, -12345, 123456, -12345678, 1234567890, -12345678901234, 1234567890123456789
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
    let big_cases = [
        1e10f64,
        -1e20,
        1e30,
        -1e40,
        1e60,
        -1e100
    ];
    for v in big_cases {
        let i = IBig::try_from(v).unwrap();
        assert_eq!(hash(&v), hash(&i));

        if let Ok(u) = UBig::try_from(v) {
            assert_eq!(hash(&u), hash(&v));
            assert_eq!(hash(&u), hash(&i));
        }
    }
}
