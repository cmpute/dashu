use dashu_ratio::{RBig, Relaxed};
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

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
    assert!(!(rbig!(0) > rbig!(0)));
    assert!(rbig!(1) > rbig!(-1));
    assert!(rbig!(-2) < rbig!(1 / 2));
    assert!(!(rbig!(~0) > rbig!(~0)));
    assert!(rbig!(~1) > rbig!(~-1));
    assert!(rbig!(~-2) < rbig!(~2/4));

    // case 2: compare integers
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
