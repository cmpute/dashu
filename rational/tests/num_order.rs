use std::cmp::Ordering;
use std::str::FromStr;

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
        (rbig!(1 / 2), FBin::from_str("0x1p-1").unwrap(), Ordering::Equal),
        (rbig!(-9 / 2), FBin::from_str("-0x9p-1").unwrap(), Ordering::Equal),
        (rbig!(1 / 1024), FBin::from_str("0x1p-10").unwrap(), Ordering::Equal),
        (
            rbig!(1 / 1267650600228229401496703205376),
            FBin::from_str("0x1p-100").unwrap(),
            Ordering::Equal,
        ),
        (rbig!(1 / 3), FBin::from_str("0x55555p-20").unwrap(), Ordering::Greater),
        (rbig!(1 / 3), FBin::from_str("0x55556p-20").unwrap(), Ordering::Less),
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
        (rbig!(1 / 10), DBig::from_str("0.1").unwrap(), Ordering::Equal),
        (rbig!(-11 / 10), DBig::from_str("-1.1").unwrap(), Ordering::Equal),
        (rbig!(1 / 9765625), DBig::from_str("1.024e-7").unwrap(), Ordering::Equal),
        (
            rbig!(1 / 7888609052210118054117285652827862296732064351090230047702789306640625),
            DBig::from_str("1.267650600228229401496703205376e-70").unwrap(),
            Ordering::Equal,
        ),
        (rbig!(-1 / 3), DBig::from_str("-0.33334").unwrap(), Ordering::Greater),
        (rbig!(-1 / 3), DBig::from_str("-0.33333").unwrap(), Ordering::Less),
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

#[test]
fn test_primitive_floats() {
    // trivial cases
    assert_eq!(rbig!(0).num_cmp(&0f32), Ordering::Equal);
    assert_eq!(rbig!(0).num_cmp(&1f32), Ordering::Less);
    assert_eq!(rbig!(0).num_cmp(&-1f32), Ordering::Greater);
    assert_eq!(rbig!(1).num_cmp(&1f32), Ordering::Equal);
    assert_eq!(rbig!(1).num_cmp(&-1f32), Ordering::Greater);
    assert_eq!(rbig!(-1).num_cmp(&1f32), Ordering::Less);
    assert_eq!(rbig!(-1).num_cmp(&-1f32), Ordering::Equal);
    assert_eq!(rbig!(0).num_partial_cmp(&f32::NAN), None);

    assert_eq!(rbig!(~0).num_cmp(&0f64), Ordering::Equal);
    assert_eq!(rbig!(~0).num_cmp(&1f64), Ordering::Less);
    assert_eq!(rbig!(~0).num_cmp(&-1f64), Ordering::Greater);
    assert_eq!(rbig!(~1).num_cmp(&1f64), Ordering::Equal);
    assert_eq!(rbig!(~1).num_cmp(&-1f64), Ordering::Greater);
    assert_eq!(rbig!(~-1).num_cmp(&1f64), Ordering::Less);
    assert_eq!(rbig!(~-1).num_cmp(&-1f64), Ordering::Equal);
    assert_eq!(rbig!(~0).num_partial_cmp(&f64::NAN), None);

    // numbers with very large difference in scale
    assert_eq!(rbig!(0x10000000000).num_cmp(&1e10f32), Ordering::Greater);
    assert_eq!(rbig!(0x10).num_cmp(&1f32), Ordering::Greater);
    assert_eq!(rbig!(1).num_cmp(&1e-10f32), Ordering::Greater);
    assert_eq!(rbig!(1 / 0x10000000000).num_cmp(&1e-10f32), Ordering::Less);
    assert_eq!(rbig!(1 / 0x10).num_cmp(&1f32), Ordering::Less);
    assert_eq!(rbig!(1).num_cmp(&1e10f32), Ordering::Less);
    assert_eq!(rbig!(-0x10000000000).num_cmp(&1e-10f32), Ordering::Less);
    assert_eq!(rbig!(1 / 0x10000000000).num_cmp(&-1e10f32), Ordering::Greater);

    assert_eq!(rbig!(~0x10000000000).num_cmp(&1e10f64), Ordering::Greater);
    assert_eq!(rbig!(~0x10).num_cmp(&1f64), Ordering::Greater);
    assert_eq!(rbig!(~1).num_cmp(&1e-10f64), Ordering::Greater);
    assert_eq!(rbig!(~1/0x10000000000).num_cmp(&1e-10f64), Ordering::Less);
    assert_eq!(rbig!(~1/0x10).num_cmp(&1f64), Ordering::Less);
    assert_eq!(rbig!(~1).num_cmp(&1e10f64), Ordering::Less);
    assert_eq!(rbig!(~-0x10000000000).num_cmp(&1e-10f64), Ordering::Less);
    assert_eq!(rbig!(~1/0x10000000000).num_cmp(&-1e10f64), Ordering::Greater);

    // numbers that are close
    {
        use core::f32::consts::PI;
        assert_eq!(rbig!(355 / 113).num_cmp(&PI), Ordering::Greater);
        assert_eq!(rbig!(355 / 114).num_cmp(&PI), Ordering::Less);
    }
    {
        use core::f64::consts::PI;
        assert_eq!(rbig!(~165707065/52746197).num_cmp(&PI), Ordering::Greater);
        assert_eq!(rbig!(~165707065/52746198).num_cmp(&PI), Ordering::Less);
    }
}

#[test]
fn test_ord_zero_with_float() {
    // zero against nonzero floats of every magnitude, in particular below 1/4 where the
    // bit length of a zero numerator used to be taken as a magnitude estimate
    let f64_cases = [
        1.0f64,
        0.5,
        0.25,
        0.125,
        0.1,
        1e-30,
        f64::MIN_POSITIVE,
        f64::from_bits(1),
        1e300,
        f64::MAX,
        f64::INFINITY,
    ];
    for f in f64_cases {
        assert_eq!(rbig!(0).num_cmp(&f), Ordering::Less, "0 vs {f:e}");
        assert_eq!(rbig!(~0).num_cmp(&f), Ordering::Less, "~0 vs {f:e}");
        assert_eq!(rbig!(0).num_partial_cmp(&f), Some(Ordering::Less), "0 vs {f:e}");
        assert_eq!(f.num_cmp(&rbig!(0)), Ordering::Greater, "{f:e} vs 0");
        assert_eq!(f.num_cmp(&rbig!(~0)), Ordering::Greater, "{f:e} vs ~0");
        assert_eq!(rbig!(0).num_cmp(&-f), Ordering::Greater, "0 vs -{f:e}");
        assert_eq!(rbig!(~0).num_cmp(&-f), Ordering::Greater, "~0 vs -{f:e}");
        assert_eq!((-f).num_cmp(&rbig!(0)), Ordering::Less, "-{f:e} vs 0");
    }
    let f32_cases = [
        1.0f32,
        0.25,
        0.125,
        0.1,
        1e-30,
        f32::MIN_POSITIVE,
        f32::from_bits(1),
        f32::MAX,
        f32::INFINITY,
    ];
    for f in f32_cases {
        assert_eq!(rbig!(0).num_cmp(&f), Ordering::Less, "0 vs {f:e}");
        assert_eq!(rbig!(~0).num_cmp(&f), Ordering::Less, "~0 vs {f:e}");
        assert_eq!(f.num_cmp(&rbig!(0)), Ordering::Greater, "{f:e} vs 0");
        assert_eq!(rbig!(0).num_cmp(&-f), Ordering::Greater, "0 vs -{f:e}");
        assert_eq!((-f).num_cmp(&rbig!(~0)), Ordering::Less, "-{f:e} vs ~0");
    }
    assert_eq!(rbig!(0).num_cmp(&0f64), Ordering::Equal);
    assert_eq!(rbig!(0).num_cmp(&-0f32), Ordering::Equal);

    // the ordering against floats agrees with the exact ordering of the rationals
    let floats = [
        0.0f64, 1e-300, 0.1, 0.125, 0.2, 0.25, 0.3, 0.5, 1.0, 1.5, 3.0, 1e10, 1e300,
    ];
    let ratios = [
        rbig!(0),
        rbig!(1 / 0x10000000000000000000000000000),
        rbig!(1 / 10),
        rbig!(1 / 8),
        rbig!(1 / 5),
        rbig!(1 / 4),
        rbig!(1 / 3),
        rbig!(1 / 2),
        rbig!(1),
        rbig!(3 / 2),
        rbig!(3),
        rbig!(0x10000000000000000000000000000),
    ];
    for r in &ratios {
        for f in floats {
            for (r, f) in [
                (r.clone(), f),
                (-r.clone(), -f),
                (-r.clone(), f),
                (r.clone(), -f),
            ] {
                let exact = r.cmp(&RBig::try_from(f).unwrap());
                assert_eq!(r.num_cmp(&f), exact, "{r} vs {f:e}");
                assert_eq!(f.num_cmp(&r), exact.reverse(), "{f:e} vs {r}");
                assert_eq!(r.clone().relax().num_cmp(&f), exact, "~{r} vs {f:e}");
                let f = f as f32;
                if f.is_infinite() {
                    continue;
                }
                let exact = r.cmp(&RBig::try_from(f).unwrap());
                assert_eq!(r.num_cmp(&f), exact, "{r} vs {f:e}f32");
                assert_eq!(f.num_cmp(&r), exact.reverse(), "{f:e}f32 vs {r}");
            }
        }
    }
}
