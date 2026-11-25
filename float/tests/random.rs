use dashu_float::{
    rand::{Uniform01, UniformFBig},
    DBig,
};
use rand_v08::{distributions::uniform::Uniform, prelude::*};

mod helper_macros;

type FBig = dashu_float::FBig;

#[test]
#[rustfmt::skip::macros(fbig)]
fn test_uniform01_binary() {
    let mut rng = StdRng::seed_from_u64(1);

    let distr = Uniform01::new(8);
    let x: FBig = (&mut rng).sample_iter(&distr).take(1000).min().unwrap();
    assert!(x >= fbig!(0) && x < fbig!(0x1p-1));
    assert_eq!(x.precision(), 8);
    let x: FBig = (&mut rng).sample_iter(&distr).take(1000).max().unwrap();
    assert!(x < fbig!(1) && x > fbig!(0x1p-1));
    assert_eq!(x.precision(), 8);

    let distr = Uniform01::new_closed(8);
    let x: FBig = (&mut rng).sample_iter(&distr).take(1000).min().unwrap();
    assert!(x >= fbig!(0) && x < fbig!(0x1p-1));
    assert_eq!(x.precision(), 8);
    let x: FBig = (&mut rng).sample_iter(&distr).take(1000).max().unwrap();
    assert!(x <= fbig!(1) && x > fbig!(0x1p-1));
    assert_eq!(x.precision(), 8);

    let distr = Uniform01::new_open(8);
    let x: FBig = (&mut rng).sample_iter(&distr).take(1000).min().unwrap();
    assert!(x > fbig!(0) && x < fbig!(0x1p-1));
    assert_eq!(x.precision(), 8);
    let x: FBig = (&mut rng).sample_iter(&distr).take(1000).max().unwrap();
    assert!(x < fbig!(1) && x > fbig!(0x1p-1));
    assert_eq!(x.precision(), 8);

    let distr = Uniform01::new_open_closed(8);
    let x: FBig = (&mut rng).sample_iter(&distr).take(1000).min().unwrap();
    assert!(x > fbig!(0) && x < fbig!(0x1p-1));
    assert_eq!(x.precision(), 8);
    let x: FBig = (&mut rng).sample_iter(&distr).take(1000).max().unwrap();
    assert!(x <= fbig!(1) && x > fbig!(0x1p-1));
    assert_eq!(x.precision(), 8);

    // test the standard distribution
    let x: FBig = rng.gen();
    assert!(x >= fbig!(0) && x < fbig!(1));
}

#[test]
#[rustfmt::skip::macros(fbig)]
fn test_uniform_binary() {
    let mut rng = StdRng::seed_from_u64(1);

    let distr = Uniform::from(fbig!(0x3)..fbig!(0x07));
    let x = (&mut rng).sample_iter(&distr).take(1000).min().unwrap();
    assert!(x >= fbig!(0x3) && x < fbig!(0x5));
    assert_eq!(x.precision(), 8);
    let x = (&mut rng).sample_iter(&distr).take(1000).max().unwrap();
    assert!(x < fbig!(0x7) && x > fbig!(0x5));
    assert_eq!(x.precision(), 8);

    let distr = Uniform::from(fbig!(-0x07)..=fbig!(-0x3));
    let x = (&mut rng).sample_iter(&distr).take(1000).min().unwrap();
    assert!(x >= fbig!(-0x7) && x < fbig!(-0x5));
    assert_eq!(x.precision(), 8);
    let x = (&mut rng).sample_iter(&distr).take(1000).max().unwrap();
    assert!(x <= fbig!(-0x3) && x > fbig!(-0x5));
    assert_eq!(x.precision(), 8);

    let distr = UniformFBig::new(&fbig!(0x3), &fbig!(0x7), 8);
    let x = (&mut rng).sample_iter(&distr).take(1000).min().unwrap();
    assert!(x >= fbig!(0x3) && x < fbig!(0x5));
    assert_eq!(x.precision(), 8);
    let x = (&mut rng).sample_iter(&distr).take(1000).max().unwrap();
    assert!(x < fbig!(0x7) && x > fbig!(0x5));
    assert_eq!(x.precision(), 8);

    let distr = UniformFBig::new_inclusive(&fbig!(-0x7p-3), &fbig!(0x3p-3), 8);
    let x = (&mut rng).sample_iter(&distr).take(1000).min().unwrap();
    assert!(x >= fbig!(-0x7p-3) && x < fbig!(-0x5p-3));
    assert_eq!(x.precision(), 8);
    let x = (&mut rng).sample_iter(&distr).take(1000).max().unwrap();
    assert!(x <= fbig!(0x3p-3) && x > fbig!(-0x5p-3));
    assert_eq!(x.precision(), 8);
}

#[test]
fn test_uniform01_decimal() {
    let mut rng = StdRng::seed_from_u64(1);

    let distr = Uniform01::new(2);
    let x: DBig = (&mut rng).sample_iter(&distr).take(1000).min().unwrap();
    assert!(x >= dbig!(0) && x < dbig!(0.5));
    assert_eq!(x.precision(), 2);
    let x: DBig = (&mut rng).sample_iter(&distr).take(1000).max().unwrap();
    assert!(x < dbig!(1) && x > dbig!(0.5));
    assert_eq!(x.precision(), 2);

    let distr = Uniform01::new_closed(2);
    let x: DBig = (&mut rng).sample_iter(&distr).take(1000).min().unwrap();
    assert!(x >= dbig!(0) && x < dbig!(0.5));
    assert_eq!(x.precision(), 2);
    let x: DBig = (&mut rng).sample_iter(&distr).take(1000).max().unwrap();
    assert!(x <= dbig!(1) && x > dbig!(0.5));
    assert_eq!(x.precision(), 2);

    let distr = Uniform01::new_open(2);
    let x: DBig = (&mut rng).sample_iter(&distr).take(1000).min().unwrap();
    assert!(x > dbig!(0) && x < dbig!(0.5));
    assert_eq!(x.precision(), 2);
    let x: DBig = (&mut rng).sample_iter(&distr).take(1000).max().unwrap();
    assert!(x < dbig!(1) && x > dbig!(0.5));
    assert_eq!(x.precision(), 2);

    let distr = Uniform01::new_open_closed(2);
    let x: DBig = (&mut rng).sample_iter(&distr).take(1000).min().unwrap();
    assert!(x > dbig!(0) && x < dbig!(0.5));
    assert_eq!(x.precision(), 2);
    let x: DBig = (&mut rng).sample_iter(&distr).take(1000).max().unwrap();
    assert!(x <= dbig!(1) && x > dbig!(0.5));
    assert_eq!(x.precision(), 2);

    // test the standard distribution
    let x: DBig = rng.gen();
    assert!(x >= dbig!(0) && x < dbig!(1));
}

#[test]
fn test_uniform_decimal() {
    let mut rng = StdRng::seed_from_u64(1);

    let distr = Uniform::from(dbig!(3)..dbig!(07));
    let x = (&mut rng).sample_iter(&distr).take(1000).min().unwrap();
    assert!(x >= dbig!(3) && x < dbig!(5));
    assert_eq!(x.precision(), 2);
    let x = (&mut rng).sample_iter(&distr).take(1000).max().unwrap();
    assert!(x < dbig!(7) && x > dbig!(5));
    assert_eq!(x.precision(), 2);

    let distr = Uniform::from(dbig!(-07)..=dbig!(-3));
    let x = (&mut rng).sample_iter(&distr).take(1000).min().unwrap();
    assert!(x >= dbig!(-7) && x < dbig!(-5));
    assert_eq!(x.precision(), 2);
    let x = (&mut rng).sample_iter(&distr).take(1000).max().unwrap();
    assert!(x <= dbig!(-3) && x > dbig!(-5));
    assert_eq!(x.precision(), 2);

    let distr = UniformFBig::new(&dbig!(0.3), &dbig!(0.7), 2);
    let x = (&mut rng).sample_iter(&distr).take(1000).min().unwrap();
    assert!(x >= dbig!(0.3) && x < dbig!(0.5));
    assert_eq!(x.precision(), 2);
    let x = (&mut rng).sample_iter(&distr).take(1000).max().unwrap();
    assert!(x < dbig!(0.7) && x > dbig!(0.5));
    assert_eq!(x.precision(), 2);

    let distr = UniformFBig::new_inclusive(&dbig!(-0.7), &dbig!(0.3), 2);
    let x = (&mut rng).sample_iter(&distr).take(1000).min().unwrap();
    assert!(x >= dbig!(-0.7) && x < dbig!(-0.5));
    assert_eq!(x.precision(), 2);
    let x = (&mut rng).sample_iter(&distr).take(1000).max().unwrap();
    assert!(x <= dbig!(0.3) && x > dbig!(-0.5));
    assert_eq!(x.precision(), 2);
}

// The width of the interval used to be rounded to the precision of the bounds (`6 - (-9)` is
// `2e1` at one digit), and the offset was added at the sampling precision even when the lower
// bound did not fit in it, so samples could land well outside the interval.
#[test]
fn test_uniform_stays_in_range() {
    use core::str::FromStr;
    use dashu_float::round::mode::{HalfAway, HalfEven, Up, Zero};

    let mut rng = StdRng::seed_from_u64(7);
    let d = |s: &str| DBig::from_str(s).unwrap();

    // through rand's `Uniform`: the precision comes from the bounds
    for (low, high) in [
        ("-9", ".6"),
        ("-9", "0.6"),
        ("-9", "6"),
        ("-9.0", "6.0"),
        ("-1e2", "-1"),
        ("-0.7", "0.3"),
        ("-0.1245", "1"),
        ("1", "9"),
        ("-11", "0"),
        ("99", "101"),
        ("1e-30", "1"),
    ] {
        let (low, high) = (d(low), d(high));
        let precision = low.precision().max(high.precision());
        let closed = Uniform::from(low.clone()..=high.clone());
        let open = Uniform::from(low.clone()..high.clone());
        let mut interior = false;
        for _ in 0..2000 {
            let x = rng.sample(&closed);
            assert!(low <= x && x <= high, "{x} outside [{low}, {high}]");
            assert!(x.repr().digits() <= x.precision());
            assert!(x.precision() >= precision);
            let x = rng.sample(&open);
            assert!(low <= x && x < high, "{x} outside [{low}, {high})");
            interior |= x > low;
        }
        assert!(interior, "[{low}, {high}) only ever gave its lower bound");
    }
    for _ in 0..2000 {
        let x = rng.gen_range(d("-9")..=d("6"));
        assert!(d("-9") <= x && x <= d("6"), "{x}");
    }

    // through the constructors, at precisions below, at and above what the bounds need
    fn check<R: dashu_float::round::Round, const B: dashu_int::Word>(
        rng: &mut StdRng,
        low: i64,
        high: i64,
    ) {
        let low = dashu_float::FBig::<R, B>::from(low);
        let high = dashu_float::FBig::<R, B>::from(high);
        let digits = low.repr().digits().max(high.repr().digits());
        for precision in [1usize, 2, 5, 20, 50, 100, 500] {
            let open = UniformFBig::new(&low, &high, precision);
            let closed = UniformFBig::new_inclusive(&low, &high, precision);
            for _ in 0..300 {
                let x = rng.sample(&open);
                assert!(low <= x && x < high, "[{low}, {high}) at {precision} gave {x}");
                assert_eq!(x.precision(), precision.max(digits));
                assert!(x.repr().digits() <= x.precision());
                let x = rng.sample(&closed);
                assert!(low <= x && x <= high, "[{low}, {high}] at {precision} gave {x}");
                assert_eq!(x.precision(), precision.max(digits));
            }
        }
    }
    for (low, high) in [
        (-9, 6),
        (-11, 0),
        (-3, 0),
        (0, 7),
        (99, 101),
        (-1000001, -999999),
    ] {
        check::<HalfAway, 10>(&mut rng, low, high);
        check::<Zero, 2>(&mut rng, low, high);
        check::<HalfEven, 2>(&mut rng, low, high);
        check::<Up, 3>(&mut rng, low, high);
    }

    // the closed interval reaches both ends
    let closed = UniformFBig::new_inclusive(&d("-9"), &d("6"), 1);
    let samples: Vec<DBig> = (&mut rng).sample_iter(&closed).take(2000).collect();
    assert_eq!(samples.iter().min().unwrap(), &d("-9"));
    assert_eq!(samples.iter().max().unwrap(), &d("6"));
    // a single point is a valid closed interval
    let point = UniformFBig::new_inclusive(&d("2.5"), &d("2.5"), 4);
    assert_eq!(rng.sample(&point), d("2.5"));
}

// Bounds of unlimited precision, like the `ZERO` and `ONE` constants, used to give a sampler
// with zero digits, which could only return the lower bound.
#[test]
fn test_uniform_unlimited_precision_bounds() {
    use std::collections::BTreeSet;

    let mut rng = StdRng::seed_from_u64(11);
    assert_eq!(DBig::ZERO.precision(), 0);

    let xs: Vec<DBig> = (0..200)
        .map(|_| rng.gen_range(DBig::ZERO..DBig::ONE))
        .collect();
    assert!(xs
        .iter()
        .all(|x| *x >= DBig::ZERO && *x < DBig::ONE && x.precision() > 0));
    assert!(
        xs.iter()
            .map(|x| x.to_string())
            .collect::<BTreeSet<_>>()
            .len()
            > 190
    );

    let xs: Vec<DBig> = (0..200)
        .map(|_| rng.gen_range(DBig::ZERO..=DBig::ONE))
        .collect();
    assert!(xs.iter().all(|x| *x >= DBig::ZERO && *x <= DBig::ONE));
    assert!(xs.iter().any(|x| *x > DBig::ZERO && *x < DBig::ONE));

    let xs: Vec<FBig> = (0..200)
        .map(|_| rng.gen_range(FBig::ZERO..FBig::ONE))
        .collect();
    assert!(xs
        .iter()
        .all(|x| *x >= FBig::ZERO && *x < FBig::ONE && x.precision() > 0));
    assert!(xs.iter().filter(|x| **x > FBig::ZERO).count() > 190);

    let distr = UniformFBig::new(&FBig::NEG_ONE, &FBig::ONE, 0);
    let xs: Vec<FBig> = (&mut rng).sample_iter(&distr).take(200).collect();
    assert!(xs.iter().all(|x| *x >= FBig::NEG_ONE && *x < FBig::ONE));
    assert!(xs.iter().any(|x| *x < FBig::ZERO) && xs.iter().any(|x| *x > FBig::ZERO));
}

#[test]
#[should_panic(expected = "empty range")]
fn test_uniform_empty_range() {
    let _ = UniformFBig::new(&dbig!(5), &dbig!(5), 2);
}

#[test]
#[should_panic(expected = "empty range")]
fn test_uniform_reversed_range() {
    let _ = UniformFBig::new_inclusive(&dbig!(6), &dbig!(5), 2);
}

// With no digits there is no number strictly between 0 and 1. Sampling used to loop forever
// in base 2 and to trip an internal assertion in other bases.
#[test]
#[should_panic(expected = "precision cannot be 0")]
fn test_uniform01_open_zero_precision_binary() {
    let _ = Uniform01::<2>::new_open(0);
}

#[test]
#[should_panic(expected = "precision cannot be 0")]
fn test_uniform01_open_zero_precision_decimal() {
    let _ = Uniform01::<10>::new_open(0);
}

#[test]
fn test_uniform01_zero_precision() {
    // the other three intervals contain an endpoint, which is all a zero-digit sample can be
    let mut rng = StdRng::seed_from_u64(3);
    let x: FBig = rng.sample(Uniform01::<2>::new(0));
    assert_eq!(x, FBig::ZERO);
    let x: DBig = rng.sample(Uniform01::<10>::new_open_closed(0));
    assert_eq!(x, DBig::ONE);
    let x: DBig = rng.sample(Uniform01::<10>::new_closed(0));
    assert!(x == DBig::ZERO || x == DBig::ONE);
    // and one digit is enough for the open interval
    let x: FBig = rng.sample(Uniform01::<2>::new_open(1));
    assert_eq!(x, fbig!(0x1p - 1));
}

// rand 0.9 and 0.10 report an empty or non-finite range as an error from `Uniform::new`,
// where the float sampler used to accept `new(x, x)` (and then return the excluded bound) and
// to panic on reversed bounds.
macro_rules! test_uniform_errors {
    ($name:ident, $feature:literal, $rand:ident) => {
        #[cfg(feature = $feature)]
        #[test]
        fn $name() {
            use $rand::distr::uniform::{Error, Uniform};

            for (five, six, inf) in [(dbig!(5), dbig!(6), DBig::INFINITY)] {
                assert!(Uniform::new(five.clone(), six.clone()).is_ok());
                assert!(Uniform::new_inclusive(five.clone(), six.clone()).is_ok());
                assert!(Uniform::new_inclusive(five.clone(), five.clone()).is_ok());
                assert_eq!(Uniform::new(five.clone(), five.clone()).err(), Some(Error::EmptyRange));
                assert_eq!(Uniform::new(six.clone(), five.clone()).err(), Some(Error::EmptyRange));
                assert_eq!(
                    Uniform::new_inclusive(six.clone(), five.clone()).err(),
                    Some(Error::EmptyRange)
                );
                assert_eq!(Uniform::new(five.clone(), inf.clone()).err(), Some(Error::NonFinite));
                assert_eq!(Uniform::new_inclusive(-inf, five).err(), Some(Error::NonFinite));
            }
            let (three, seven) = (fbig!(0x3), fbig!(0x7));
            assert!(Uniform::new(three.clone(), seven.clone()).is_ok());
            assert_eq!(Uniform::new(three.clone(), three.clone()).err(), Some(Error::EmptyRange));
            assert_eq!(Uniform::new(seven, three).err(), Some(Error::EmptyRange));
        }
    };
}
test_uniform_errors!(test_uniform_errors_v09, "rand_v09", rand_v09);
test_uniform_errors!(test_uniform_errors_v010, "rand_v010", rand_v010);

/// A sample is the exact value of `low + unit * (high - low)` rounded down to the precision,
/// where `unit` is what the unit sampler draws from the same random stream.
#[test]
fn test_uniform_matches_exact_formula() {
    use dashu_float::round::mode::{Down, HalfAway, Zero};
    use dashu_float::{round::Round, Context};

    fn check<R: Round, const B: dashu_int::Word>(
        seed: u64,
        low: (i64, isize),
        high: (i64, isize),
        precision: usize,
        closed: bool,
    ) {
        type F<R, const B: dashu_int::Word> = dashu_float::FBig<R, B>;
        let low = F::<R, B>::from_parts(low.0.into(), low.1);
        let high = F::<R, B>::from_parts(high.0.into(), high.1);
        let effective = precision.max(low.repr().digits()).max(high.repr().digits());
        let (range, unit) = if closed {
            (
                UniformFBig::new_inclusive(&low, &high, precision),
                Uniform01::<B>::new_closed(effective),
            )
        } else {
            (UniformFBig::new(&low, &high, precision), Uniform01::<B>::new(effective))
        };
        let mut rng = StdRng::seed_from_u64(seed);
        let mut unit_rng = StdRng::seed_from_u64(seed);
        let exact = Context::<Down>::new(0);
        for _ in 0..40 {
            let x: F<R, B> = rng.sample(&range);
            let u: F<Down, B> = unit_rng.sample(&unit);
            let width = exact.sub(high.repr(), low.repr()).unwrap().value();
            let scaled = exact.mul(u.repr(), width.repr()).unwrap().value();
            let sum = exact.add(scaled.repr(), low.repr()).unwrap().value();
            let expected = sum.with_precision(effective).value();
            assert_eq!(x.repr().significand(), expected.repr().significand());
            if !x.repr().significand().is_zero() {
                assert_eq!(x.repr().exponent(), expected.repr().exponent());
            }
            assert_eq!(x.precision(), effective);
            assert!(!x.repr().is_neg_zero());
        }
    }

    let bounds = [
        ((-1, 0), (1, 0)),
        ((0, 0), (2, 0)),
        ((-9, 0), (6, 0)),
        ((-11, 0), (0, 0)),
        ((1, -30), (1, 0)),
        ((1, -200), (3, 5)),
        ((-7, 40), (5, -40)),
        ((-123456789, -3), (-123456788, -3)),
        ((99, 0), (101, 0)),
        ((3, 300), (5, 300)),
    ];
    for (seed, (low, high)) in bounds.into_iter().enumerate() {
        // the precisions include the four standard binary ones
        for precision in [1usize, 2, 7, 20, 50, 100, 500] {
            for closed in [false, true] {
                check::<Zero, 2>(seed as u64, low, high, precision, closed);
                check::<HalfAway, 10>(seed as u64, low, high, precision, closed);
                check::<Down, 3>(seed as u64, low, high, precision, closed);
            }
        }
    }
}

/// A sample that is zero is `+0`: neither bound of a range that contains zero is `-0`.
#[test]
fn test_uniform_zero_sample_is_positive() {
    use dashu_float::round::mode::{Down, HalfAway, Zero};

    let mut rng = StdRng::seed_from_u64(1);

    // a range that straddles zero, at a precision where zero comes up often
    let binary = UniformFBig::<Zero, 2>::new(&FBig::from(-1), &FBig::from(1), 1);
    let decimal = UniformFBig::<HalfAway, 10>::new(&DBig::from(-1), &DBig::from(1), 1);
    let directed = UniformFBig::<Down, 10>::new_inclusive(
        &dashu_float::FBig::from(-2),
        &dashu_float::FBig::from(2),
        1,
    );
    let (mut binary_zeros, mut decimal_zeros, mut directed_zeros) = (0, 0, 0);
    for _ in 0..500 {
        let x = rng.sample(&binary);
        assert!(!x.repr().is_neg_zero(), "{x}");
        binary_zeros += x.repr().is_pos_zero() as usize;
        let x = rng.sample(&decimal);
        assert!(!x.repr().is_neg_zero(), "{x}");
        decimal_zeros += x.repr().is_pos_zero() as usize;
        let x = rng.sample(&directed);
        assert!(!x.repr().is_neg_zero(), "{x}");
        directed_zeros += x.repr().is_pos_zero() as usize;
        let x: DBig = rng.gen_range(DBig::from(-1)..DBig::from(1));
        assert!(!x.repr().is_neg_zero(), "{x}");
    }
    assert!(binary_zeros > 0 && decimal_zeros > 0 && directed_zeros > 0);

    // ranges with zero at an end, and the single point zero
    let d = |n: i32| DBig::from(n);
    for (low, high) in [(0, 0), (-1, 0), (0, 1), (-3, 0)] {
        let closed = UniformFBig::<HalfAway, 10>::new_inclusive(&d(low), &d(high), 5);
        for _ in 0..200 {
            let x = rng.sample(&closed);
            assert!(!x.repr().is_neg_zero(), "[{low}, {high}] gave {x}");
        }
    }
    let point = UniformFBig::<HalfAway, 10>::new_inclusive(&DBig::ZERO, &DBig::ZERO, 5);
    assert!(rng.sample(&point).repr().is_pos_zero());
    let x: DBig = rng.gen_range(DBig::ZERO..=DBig::ZERO);
    assert!(x.repr().is_pos_zero());
}

/// The cost of a sample depends on the precision, not on how far apart the exponents of the
/// bounds are: the exact width of `[B^-g, 1)` has `g` digits, and so had every sample before
/// it was rounded.
#[test]
fn test_uniform_distant_exponents() {
    use dashu_float::round::mode::Zero;

    fn check<const B: dashu_int::Word>(rng: &mut StdRng, low_exp: isize, high_exp: isize) {
        type F<const B: dashu_int::Word> = dashu_float::FBig<Zero, B>;
        let low = F::<B>::from_parts(1.into(), low_exp);
        let high = F::<B>::from_parts(1.into(), high_exp);
        let neg_high = -low.clone();
        let neg_low = -high.clone();
        for precision in [10usize, 20, 50, 100, 500] {
            let open = UniformFBig::new(&low, &high, precision);
            let closed = UniformFBig::new_inclusive(&low, &high, precision);
            let negative = UniformFBig::new(&neg_low, &neg_high, precision);
            let straddling = UniformFBig::new(&neg_low, &high, precision);
            for _ in 0..50 {
                let x = rng.sample(&open);
                assert!(x >= low && x < high);
                assert!(x.repr().digits() <= precision);
                let x = rng.sample(&closed);
                assert!(x >= low && x <= high);
                let x = rng.sample(&negative);
                assert!(x >= neg_low && x < neg_high);
                assert!(x.repr().digits() <= precision);
                let x = rng.sample(&straddling);
                assert!(x >= neg_low && x < high);
            }
        }
    }

    let mut rng = StdRng::seed_from_u64(1);
    // the reported gaps, and gaps that no exact evaluation could ever materialize
    check::<2>(&mut rng, -10_000_000, 0);
    check::<10>(&mut rng, -1_000_000, 0);
    check::<2>(&mut rng, isize::MIN / 2, 0);
    check::<10>(&mut rng, isize::MIN / 2, 3);
    check::<2>(&mut rng, -5, isize::MAX / 2);
    check::<10>(&mut rng, isize::MIN / 4, isize::MAX / 4);
    check::<3>(&mut rng, -1_000_000_000_000, 1_000_000_000_000);
}
