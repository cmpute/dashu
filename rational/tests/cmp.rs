use dashu_ratio::RBig;
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
fn test_rbig_eq_hash() {
    let r = RBig::from_parts(ibig!(-1) << 1000, ubig!(3).pow(250));
    let h = hash(&r);
    for i in 0..=250 {
        let r2 = RBig::from_parts(
            ibig!(-1) << (i * 4) << (1000 - i * 4),
            ubig!(3).pow(i) * ubig!(3).pow(250 - i),
        );
        assert_eq!(r2, r);

        let h2 = hash(&r2);
        assert_eq!(h2, h);
    }

    let r3 = RBig::from_parts(ibig!(-1) << 1000, ubig!(3).pow(25));
    assert_ne!(r3, r);

    let h3 = hash(&r3);
    assert_ne!(h3, h);
}
