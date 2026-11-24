use core::cmp::Ordering;
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

#[test]
#[rustfmt::skip::macros(fbig)]
#[allow(clippy::approx_constant)]
fn test_primitive_floats() {
    // trivial cases
    assert_eq!(fbig!(0).num_cmp(&0f32), Ordering::Equal);
    assert_eq!(fbig!(0).num_cmp(&1f32), Ordering::Less);
    assert_eq!(fbig!(0).num_cmp(&-1f32), Ordering::Greater);
    assert_eq!(fbig!(1).num_cmp(&1f32), Ordering::Equal);
    assert_eq!(fbig!(1).num_cmp(&-1f32), Ordering::Greater);
    assert_eq!(fbig!(-1).num_cmp(&1f32), Ordering::Less);
    assert_eq!(fbig!(-1).num_cmp(&-1f32), Ordering::Equal);
    assert_eq!(fbig!(0).num_cmp(&0f32), Ordering::Equal);
    assert_eq!(FBin::INFINITY.num_cmp(&f32::INFINITY), Ordering::Equal);
    assert_eq!(FBin::INFINITY.num_cmp(&0f32), Ordering::Greater);
    assert_eq!(FBin::INFINITY.num_cmp(&f32::NEG_INFINITY), Ordering::Greater);
    assert_eq!(FBin::NEG_INFINITY.num_cmp(&f32::INFINITY), Ordering::Less);
    assert_eq!(FBin::NEG_INFINITY.num_cmp(&0f32), Ordering::Less);
    assert_eq!(FBin::NEG_INFINITY.num_cmp(&f32::NEG_INFINITY), Ordering::Equal);
    assert_eq!(fbig!(0).num_partial_cmp(&f32::NAN), None);

    assert_eq!(dbig!(0).num_cmp(&0f64), Ordering::Equal);
    assert_eq!(dbig!(0).num_cmp(&1f64), Ordering::Less);
    assert_eq!(dbig!(0).num_cmp(&-1f64), Ordering::Greater);
    assert_eq!(dbig!(1).num_cmp(&1f64), Ordering::Equal);
    assert_eq!(dbig!(1).num_cmp(&-1f64), Ordering::Greater);
    assert_eq!(dbig!(-1).num_cmp(&1f64), Ordering::Less);
    assert_eq!(dbig!(-1).num_cmp(&-1f64), Ordering::Equal);
    assert_eq!(dbig!(0).num_cmp(&0f64), Ordering::Equal);
    assert_eq!(DBig::INFINITY.num_cmp(&f64::INFINITY), Ordering::Equal);
    assert_eq!(DBig::INFINITY.num_cmp(&0f64), Ordering::Greater);
    assert_eq!(DBig::INFINITY.num_cmp(&f64::NEG_INFINITY), Ordering::Greater);
    assert_eq!(DBig::NEG_INFINITY.num_cmp(&f64::INFINITY), Ordering::Less);
    assert_eq!(DBig::NEG_INFINITY.num_cmp(&0f64), Ordering::Less);
    assert_eq!(DBig::NEG_INFINITY.num_cmp(&f64::NEG_INFINITY), Ordering::Equal);
    assert_eq!(dbig!(0).num_partial_cmp(&f64::NAN), None);

    // numbers with very large difference in scale
    assert_eq!(fbig!(0x1p100).num_cmp(&1e10f32), Ordering::Greater);
    assert_eq!(fbig!(0x1p10).num_cmp(&1f32), Ordering::Greater);
    assert_eq!(fbig!(0x1).num_cmp(&1e-10f32), Ordering::Greater);
    assert_eq!(fbig!(0x1p-100).num_cmp(&1e-10f32), Ordering::Less);
    assert_eq!(fbig!(0x1p-10).num_cmp(&1f32), Ordering::Less);
    assert_eq!(fbig!(0x1).num_cmp(&1e10f32), Ordering::Less);
    assert_eq!(fbig!(-0x1p100).num_cmp(&1e-10f32), Ordering::Less);
    assert_eq!(fbig!(0x1p-100).num_cmp(&-1e10f32), Ordering::Greater);

    assert_eq!(dbig!(1e100).num_cmp(&1e10f64), Ordering::Greater);
    assert_eq!(dbig!(1e10).num_cmp(&1f64), Ordering::Greater);
    assert_eq!(dbig!(1).num_cmp(&1e-10f64), Ordering::Greater);
    assert_eq!(dbig!(1e-100).num_cmp(&1e-10f64), Ordering::Less);
    assert_eq!(dbig!(1e-10).num_cmp(&1f64), Ordering::Less);
    assert_eq!(dbig!(1).num_cmp(&1e10f64), Ordering::Less);
    assert_eq!(dbig!(-1e100).num_cmp(&1e-10f64), Ordering::Less);
    assert_eq!(dbig!(1e100).num_cmp(&-1e10f64), Ordering::Greater);

    // numbers that are close
    assert_eq!(fbig!(0x1921fb4p-23).num_cmp(&3.1415926f32), Ordering::Equal);
    assert_eq!(fbig!(0x1921fb4001p-35).num_cmp(&3.1415926f32), Ordering::Greater);
    assert_eq!(fbig!(0x1921fb3fffp-35).num_cmp(&3.1415926f32), Ordering::Less);
    assert_eq!(fbig!(0x1921fb54442d18p-51).num_cmp(&3.141592653589793f64), Ordering::Equal);
    assert_eq!(
        fbig!(0x1921fb54442d180000000001p-91).num_cmp(&3.141592653589793f64),
        Ordering::Greater
    );
    assert_eq!(
        fbig!(0x1921fb54442d17ffffffffffp-91).num_cmp(&3.141592653589793f64),
        Ordering::Less
    );

    // exact value 3.1415926f32 = 3.141592502593994140625​
    assert_eq!(dbig!(3.1415926).num_cmp(&3.1415926f32), Ordering::Greater);
    assert_eq!(dbig!(3.1415925).num_cmp(&3.1415926f32), Ordering::Less);
    // exact value 3.141592653589793f64 = 3.141592653589793115997963468544185161590576171875​​
    assert_eq!(dbig!(3.141592653589793).num_cmp(&3.141592653589793f64), Ordering::Less);
    assert_eq!(dbig!(3.141592653589793238).num_cmp(&3.141592653589793f64), Ordering::Greater);
}

// A zero has no significand bits, which the magnitude estimate used against floats read as
// the value 1: zero then ranked above every positive float below 1/2.
#[test]
fn test_ord_zero_with_float() {
    use dashu_float::{round::mode::HalfEven, Context, Repr};

    let zeros_bin = [
        FBin::ZERO,
        FBin::try_from(-0.0f64).unwrap(),
        FBin::from_repr(Repr::zero(), Context::new(53)),
        FBin::from_repr(Repr::neg_zero(), Context::new(53)),
    ];
    let zeros_dec = [DBig::ZERO, -DBig::ZERO, DBig::from_repr(Repr::neg_zero(), Context::new(9))];
    let floats = [
        f64::MIN_POSITIVE,
        f64::from_bits(1),
        1e-300,
        1e-10,
        0.25,
        0.49,
        0.5,
        1.0,
        5.0,
        f64::MAX,
        f64::INFINITY,
    ];
    for x in floats {
        for z in &zeros_bin {
            assert_eq!(z.num_partial_cmp(&x), Some(Ordering::Less), "0 vs {x:e}");
            assert_eq!(z.num_partial_cmp(&-x), Some(Ordering::Greater), "0 vs {:e}", -x);
            assert_eq!(x.num_partial_cmp(z), Some(Ordering::Greater));
            assert_eq!((-x).num_partial_cmp(z), Some(Ordering::Less));
            assert_eq!(z.repr().num_partial_cmp(&x), Some(Ordering::Less));
            assert_eq!(z.num_cmp(&x), Ordering::Less);
            let x32 = x as f32;
            if x32 != 0.0 {
                assert_eq!(z.num_partial_cmp(&x32), Some(Ordering::Less), "0 vs {x32:e}f32");
                assert_eq!(z.num_partial_cmp(&-x32), Some(Ordering::Greater));
                assert_eq!(x32.num_partial_cmp(z), Some(Ordering::Greater));
            }
        }
        for z in &zeros_dec {
            assert_eq!(z.num_partial_cmp(&x), Some(Ordering::Less), "0 vs {x:e}");
            assert_eq!(z.num_partial_cmp(&-x), Some(Ordering::Greater), "0 vs {:e}", -x);
            assert_eq!(x.num_partial_cmp(z), Some(Ordering::Greater));
        }
    }
    for z in &zeros_bin {
        assert_eq!(z.num_partial_cmp(&0.0f64), Some(Ordering::Equal));
        assert_eq!(z.num_partial_cmp(&-0.0f32), Some(Ordering::Equal));
        assert_eq!(z.num_partial_cmp(&f64::NAN), None);
    }

    // nonzero values on both sides of the same floats, at the four standard precisions
    for precision in [20usize, 50, 100, 500] {
        let context = Context::<HalfEven>::new(precision);
        for x in [1e-300f64, 0.25, 0.5, 1.0, 3.0, 1e300] {
            let exact = dashu_float::FBig::<HalfEven, 2>::try_from(x).unwrap();
            let third = context.div::<2>(exact.repr(), &Repr::new(3.into(), 0)).unwrap().value();
            assert_eq!(exact.num_partial_cmp(&x), Some(Ordering::Equal));
            assert_eq!(third.num_partial_cmp(&x), Some(Ordering::Less));
            assert_eq!((&exact + &third).num_partial_cmp(&x), Some(Ordering::Greater));
            assert_eq!((-third).num_partial_cmp(&-x), Some(Ordering::Greater));
        }
    }
}

// The magnitude estimate multiplies the exponent by the bit length of the base, which
// overflowed `isize` (a panic in debug builds, a wrong order in release builds) for exponents
// beyond `isize::MAX / bit_len(base)`.
#[test]
fn test_ord_with_float_extreme_exponent() {
    use dashu_int::IBig;

    let floats = [f64::MIN_POSITIVE, f64::from_bits(1), 1.0, f64::MAX];
    for (exp, expected) in [
        (isize::MAX / 2, Ordering::Greater),
        (isize::MAX / 2 + 1, Ordering::Greater),
        (isize::MAX - 1, Ordering::Greater),
        (isize::MIN / 2, Ordering::Less),
        (isize::MIN / 2 - 1, Ordering::Less),
        (isize::MIN + 1, Ordering::Less),
    ] {
        let x = FBin::from_parts(IBig::ONE, exp);
        let neg = FBin::from_parts(IBig::NEG_ONE, exp);
        for f in floats {
            assert_eq!(x.num_partial_cmp(&f), Some(expected), "2^{exp} vs {f:e}");
            assert_eq!(f.num_partial_cmp(&x), Some(expected.reverse()));
            assert_eq!(x.num_partial_cmp(&-f), Some(Ordering::Greater));
            assert_eq!(neg.num_partial_cmp(&-f), Some(expected.reverse()), "-2^{exp} vs -{f:e}");
        }
        for f in [f32::from_bits(1), f32::MIN_POSITIVE, 1.0, f32::MAX] {
            assert_eq!(x.num_partial_cmp(&f), Some(expected), "2^{exp} vs {f:e}f32");
            assert_eq!(f.num_partial_cmp(&neg), Some(Ordering::Greater));
        }
    }
    for (exp, expected) in [
        (isize::MAX / 4 + 1, Ordering::Greater),
        (isize::MAX - 1, Ordering::Greater),
        (isize::MIN / 4 - 1, Ordering::Less),
        (isize::MIN + 1, Ordering::Less),
    ] {
        let d = DBig::from_parts(IBig::from(7), exp);
        for f in floats {
            assert_eq!(d.num_partial_cmp(&f), Some(expected), "7e{exp} vs {f:e}");
            assert_eq!(f.num_partial_cmp(&d), Some(expected.reverse()));
        }
    }
}

// Comparing two numbers of the same base scaled both by the full power of the base, so equal
// (or merely close) values with a large exponent tried to allocate that power.
#[test]
fn test_ord_same_base_large_exponent() {
    use dashu_float::round::mode::{HalfAway, Zero};
    use dashu_int::IBig;

    for exp in [1isize << 20, 1 << 40, isize::MAX - 200, -(1 << 40), isize::MIN + 200] {
        let x = dashu_float::FBig::<Zero, 2>::from_parts(IBig::from(5), exp);
        let same = dashu_float::FBig::<HalfAway, 2>::from_parts(IBig::from(5), exp);
        let more = dashu_float::FBig::<HalfAway, 2>::from_parts(IBig::from(11), exp - 1);
        let less = dashu_float::FBig::<HalfAway, 2>::from_parts(IBig::from(39), exp - 3);
        assert_eq!(x.num_cmp(&same), Ordering::Equal);
        assert_eq!(x.num_cmp(&more), Ordering::Less);
        assert_eq!(x.num_cmp(&less), Ordering::Greater);
        assert_eq!((-x.clone()).num_cmp(&-more), Ordering::Greater);
        assert_eq!(x.repr().num_cmp(same.repr()), Ordering::Equal);
        assert_eq!(x.num_partial_cmp(&less), Some(Ordering::Greater));
        assert!(x.num_eq(&same));

        let d = DBig::from_parts(IBig::from(5), exp);
        assert_eq!(d.num_cmp(&d.clone()), Ordering::Equal);
        assert_eq!(d.num_cmp(&DBig::from_parts(IBig::from(51), exp - 1)), Ordering::Less);
    }
}
