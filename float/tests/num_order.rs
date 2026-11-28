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

    // The exponent `isize::MIN` can sit next to a nonzero significand: the hash must not
    // negate it, and must agree with the same value written in another base.
    for e in [isize::MIN, isize::MIN + 2, isize::MAX - 1] {
        let d = DBig::from_parts(ibig!(3), e);
        let c = FBig::<dashu_float::round::mode::HalfEven, 100>::from_parts(ibig!(3), e / 2);
        assert_eq!(hash(&d), hash(&c), "3 * 10^{e}");
        let b = FBin::from_parts(ibig!(3), e);
        let q = FBig::<dashu_float::round::mode::Zero, 4>::from_parts(ibig!(3), e / 2);
        assert_eq!(hash(&b), hash(&q), "3 * 2^{e}");
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
    let zeros_dec = [
        DBig::ZERO,
        -DBig::ZERO,
        DBig::from_repr(Repr::neg_zero(), Context::new(9)),
    ];
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
            let third = context
                .div::<2>(exact.repr(), &Repr::new(3.into(), 0))
                .unwrap()
                .value();
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

    for exp in [
        1isize << 20,
        1 << 40,
        isize::MAX - 200,
        -(1 << 40),
        isize::MIN + 200,
    ] {
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

/// The order of two numbers in different bases with large exponents does not need the powers
/// `B^|e|` themselves: it used to take time and memory proportional to the exponents.
#[test]
fn test_ord_cross_base_large_exponent() {
    use dashu_float::round::mode::{Down, Up, Zero};
    use dashu_int::IBig;
    type F<R, const B: dashu_int::Word> = dashu_float::FBig<R, B>;

    // 10^-n lies between 2^-(m + 1) and 2^-m for m = floor(n * log2(10))
    for n in [1000isize, 1_000_000, 1_000_000_000] {
        let m = (n as f64 * core::f64::consts::LOG2_10).floor() as isize;
        let d = DBig::from_parts(IBig::ONE, -n);
        let above = FBin::from_parts(IBig::ONE, -m);
        let below = FBin::from_parts(IBig::ONE, -(m + 1));
        assert_eq!(d.num_cmp(&above), Ordering::Less, "{n}");
        assert_eq!(above.num_cmp(&d), Ordering::Greater, "{n}");
        assert_eq!(d.num_cmp(&below), Ordering::Greater, "{n}");
        assert_eq!(below.num_cmp(&d), Ordering::Less, "{n}");
        // negative numbers order the other way
        assert_eq!((-d.clone()).num_cmp(&-above.clone()), Ordering::Greater, "{n}");
        assert_eq!((-below.clone()).num_cmp(&-d.clone()), Ordering::Greater, "{n}");
        assert_eq!((-d.clone()).num_cmp(&below), Ordering::Less, "{n}");
        // and the same far above one
        let d = DBig::from_parts(IBig::ONE, n);
        let below = FBin::from_parts(IBig::ONE, m);
        let above = FBin::from_parts(IBig::ONE, m + 1);
        assert_eq!(d.num_cmp(&above), Ordering::Less, "{n}");
        assert_eq!(d.num_cmp(&below), Ordering::Greater, "{n}");
        assert_eq!(d.repr().num_cmp(below.repr()), Ordering::Greater, "{n}");
    }

    // The two binary neighbours of a decimal number, at the four standard precisions: they
    // differ from it by less than one part in 2^precision. The neighbours are constructed
    // through `with_base_and_precision`, whose current implementation is wrong or does not
    // terminate for extreme exponents, so the loop covers only the exponent range it
    // supports; the extreme rows belong with the base-conversion fixes.
    for precision in [20usize, 50, 100, 500] {
        for (signif, exp) in [(3i32, -5000isize), (7, 5000)] {
            let d = F::<Zero, 10>::from_parts(signif.into(), exp);
            let lower = d
                .clone()
                .with_rounding::<Down>()
                .with_base_and_precision::<2>(precision)
                .value();
            let upper = d
                .clone()
                .with_rounding::<Up>()
                .with_base_and_precision::<2>(precision)
                .value();
            assert!(lower < upper.clone().with_rounding::<Down>());
            assert_eq!(d.num_cmp(&lower), Ordering::Greater, "{signif}e{exp} at {precision}");
            assert_eq!(lower.num_cmp(&d), Ordering::Less, "{signif}e{exp} at {precision}");
            assert_eq!(d.num_cmp(&upper), Ordering::Less, "{signif}e{exp} at {precision}");
            assert_eq!(upper.num_cmp(&d), Ordering::Greater, "{signif}e{exp} at {precision}");
        }
    }

    // other pairs of independent bases
    let a = F::<Zero, 3>::from_parts(IBig::ONE, 1_000_000_000);
    let m = (1e9 * 3f64.log2()).floor() as isize;
    assert_eq!(a.num_cmp(&FBin::from_parts(IBig::ONE, m)), Ordering::Greater);
    assert_eq!(a.num_cmp(&FBin::from_parts(IBig::ONE, m + 1)), Ordering::Less);
    let b = F::<Zero, 6>::from_parts(5.into(), -1_000_000_000_000);
    let c = F::<Zero, 12>::from_parts(5.into(), -1_000_000_000_000);
    assert_eq!(b.num_cmp(&c), Ordering::Greater);
    assert_eq!(c.num_cmp(&b), Ordering::Less);

    // bases that are powers of one integer are compared exactly, equal numbers included
    for e in [
        1000isize,
        1_000_000_000_000,
        -1_000_000_000_000,
        isize::MAX / 4,
    ] {
        let two = FBin::from_parts(5.into(), 3 * e);
        let eight = F::<Zero, 8>::from_parts(5.into(), e);
        assert_eq!(two.num_cmp(&eight), Ordering::Equal, "{e}");
        assert_eq!(eight.num_cmp(&two), Ordering::Equal, "{e}");
        assert_eq!((-two.clone()).num_cmp(&-eight.clone()), Ordering::Equal, "{e}");
        assert_eq!(FBin::from_parts(5.into(), 3 * e + 1).num_cmp(&eight), Ordering::Greater);
        assert_eq!(FBin::from_parts(5.into(), 3 * e - 1).num_cmp(&eight), Ordering::Less);
        assert_eq!(FBin::from_parts(9.into(), 3 * e - 1).num_cmp(&eight), Ordering::Less);
        assert_eq!(FBin::from_parts(11.into(), 3 * e - 1).num_cmp(&eight), Ordering::Greater);
        // 5 * 2^(3e - 1) = 20 * 8^(e - 1), and 4^(3e) = 8^(2e)
        let eight = F::<Zero, 8>::from_parts(20.into(), e - 1);
        assert_eq!(FBin::from_parts(5.into(), 3 * e - 1).num_cmp(&eight), Ordering::Equal);
        let four = F::<Zero, 4>::from_parts(7.into(), 3 * (e / 2));
        let eight = F::<Zero, 8>::from_parts(7.into(), 2 * (e / 2));
        assert_eq!(four.num_cmp(&eight), Ordering::Equal, "{e}");
        assert_eq!((four.clone() + four.ulp()).num_cmp(&eight), Ordering::Greater, "{e}");
    }
}

/// Numbers in independent bases can be equal only while their exponents are small next to
/// their significands; those are still recognized, however the comparison gets there.
#[test]
fn test_ord_cross_base_equal_values() {
    use dashu_float::round::mode::Zero;
    use dashu_int::{IBig, UBig};
    type F<const B: dashu_int::Word> = dashu_float::FBig<Zero, B>;

    for e in [1usize, 40, 300, 3000, 20000] {
        // s * 10^e = (s * 5^e) * 2^e
        for s in [1i32, 3, -7, 12345] {
            let d = F::<10>::from_parts(s.into(), e as isize);
            let binary_signif = IBig::from(s) * IBig::from(UBig::from(5u8).pow(e));
            let b = F::<2>::from_parts(binary_signif.clone(), e as isize);
            assert_eq!(d.num_cmp(&b), Ordering::Equal, "{s}e{e}");
            assert_eq!(b.num_cmp(&d), Ordering::Equal, "{s}e{e}");
            // one unit in the last place of the long significand apart
            let next = F::<2>::from_parts(binary_signif.clone() + IBig::ONE, e as isize);
            let prev = F::<2>::from_parts(binary_signif - IBig::ONE, e as isize);
            assert_eq!(d.num_cmp(&next), Ordering::Less, "{s}e{e}");
            assert_eq!(d.num_cmp(&prev), Ordering::Greater, "{s}e{e}");
            assert_eq!(next.num_cmp(&d), Ordering::Greater, "{s}e{e}");
        }
        // s * 2^-e = (s * 5^e) * 10^-e
        let b = F::<2>::from_parts(3.into(), -(e as isize));
        let decimal_signif = IBig::from(3) * IBig::from(UBig::from(5u8).pow(e));
        let d = F::<10>::from_parts(decimal_signif.clone(), -(e as isize));
        assert_eq!(b.num_cmp(&d), Ordering::Equal, "3b-{e}");
        let next = F::<10>::from_parts(decimal_signif + IBig::ONE, -(e as isize));
        assert_eq!(b.num_cmp(&next), Ordering::Less, "3b-{e}");
    }
}

/// The comparison agrees with the one made by scaling both numbers to integers, over a sweep
/// of bases, significand sizes and exponents on both sides of the switch between the methods.
#[test]
fn test_ord_cross_base_matches_scaled_integers() {
    use dashu_float::round::mode::Zero;
    use dashu_int::{IBig, UBig};
    type F<const B: dashu_int::Word> = dashu_float::FBig<Zero, B>;

    fn oracle<const B1: dashu_int::Word, const B2: dashu_int::Word>(
        a: &F<B1>,
        b: &F<B2>,
    ) -> Ordering {
        let power = |base: dashu_int::Word, e: isize| {
            IBig::from(UBig::from_word(base).pow(e.unsigned_abs()))
        };
        let (mut left, mut right) =
            (a.repr().significand().clone(), b.repr().significand().clone());
        match a.repr().exponent() {
            e if e < 0 => right *= power(B1, e),
            e => left *= power(B1, e),
        }
        match b.repr().exponent() {
            e if e < 0 => left *= power(B2, e),
            e => right *= power(B2, e),
        }
        left.cmp(&right)
    }

    fn check<const B1: dashu_int::Word, const B2: dashu_int::Word>(state: &mut u64) {
        let mut next = || {
            // a 64-bit linear congruential generator is plenty for a sweep
            *state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            *state >> 16
        };
        let log_ratio = (B1 as f64).log2() / (B2 as f64).log2();
        for round in 0..60 {
            let bits = [3u32, 20, 50, 100, 500][round % 5];
            let e1 = (next() % 4001) as isize - 2000;
            let mut s1 = IBig::ONE;
            for _ in 0..(bits + 47) / 48 {
                s1 = (s1 << 48) + IBig::from(next());
            }
            let s1 = (s1 >> ((bits + 47) / 48 * 48 + 1 - bits) as usize) + IBig::ONE;
            let a = F::<B1>::from_parts(if round % 3 == 0 { -s1 } else { s1 }, e1);
            // a number of about the same magnitude in the other base
            let e2 = (e1 as f64 * log_ratio) as isize + (next() % 5) as isize - 2;
            for s2 in [
                1i64,
                3,
                (next() % 1000) as i64 + 1,
                (next() >> 1) as i64 | 1,
            ] {
                let s2 = if round % 3 == 0 { -s2 } else { s2 };
                let b = F::<B2>::from_parts(s2.into(), e2);
                let expected = oracle(&a, &b);
                assert_eq!(a.num_cmp(&b), expected, "{:?} vs {:?}", a, b);
                assert_eq!(b.num_cmp(&a), expected.reverse(), "{:?} vs {:?}", b, a);
                assert_eq!(a.repr().num_cmp(b.repr()), expected);
            }
        }
    }

    let mut state = 0x9E37_79B9_7F4A_7C15;
    check::<10, 2>(&mut state);
    check::<2, 10>(&mut state);
    check::<3, 10>(&mut state);
    check::<6, 12>(&mut state);
    check::<2, 8>(&mut state);
    check::<8, 4>(&mut state);
    check::<16, 10>(&mut state);
    check::<36, 7>(&mut state);
}
