use dashu_ratio::{rand::Uniform01, RBig, Relaxed};
use rand_v08::{distributions::uniform::Uniform, prelude::*};

mod helper_macros;

#[test]
fn test_uniform01_rbig() {
    let mut rng = StdRng::seed_from_u64(1);
    let limit = ubig!(8);

    let distr = Uniform01::new(&limit);
    let x: RBig = (&mut rng).sample_iter(&distr).take(1000).min().unwrap();
    assert!(x >= rbig!(0) && x < rbig!(1 / 2));
    assert!(x.denominator() <= &limit);
    let x: RBig = (&mut rng).sample_iter(&distr).take(1000).max().unwrap();
    assert!(x < rbig!(1) && x > rbig!(1 / 2));
    assert!(x.denominator() <= &limit);

    let distr = Uniform01::new_closed(&limit);
    let x: RBig = (&mut rng).sample_iter(&distr).take(1000).min().unwrap();
    assert!(x >= rbig!(0) && x < rbig!(1 / 2));
    assert!(x.denominator() <= &limit);
    let x: RBig = (&mut rng).sample_iter(&distr).take(1000).max().unwrap();
    assert!(x <= rbig!(1) && x > rbig!(1 / 2));
    assert!(x.denominator() <= &limit);

    let distr = Uniform01::new_open(&limit);
    let x: RBig = (&mut rng).sample_iter(&distr).take(1000).min().unwrap();
    assert!(x > rbig!(0) && x < rbig!(1 / 2));
    assert!(x.denominator() <= &limit);
    let x: RBig = (&mut rng).sample_iter(&distr).take(1000).max().unwrap();
    assert!(x < rbig!(1) && x > rbig!(1 / 2));
    assert!(x.denominator() <= &limit);

    let distr = Uniform01::new_open_closed(&limit);
    let x: RBig = (&mut rng).sample_iter(&distr).take(1000).min().unwrap();
    assert!(x > rbig!(0) && x < rbig!(1 / 2));
    assert!(x.denominator() <= &limit);
    let x: RBig = (&mut rng).sample_iter(&distr).take(1000).max().unwrap();
    assert!(x <= rbig!(1) && x > rbig!(1 / 2));
    assert!(x.denominator() <= &limit);

    // test the standard distribution
    let x: RBig = rng.gen();
    assert!(x >= rbig!(0) && x < rbig!(1));
}

#[test]
fn test_uniform01_relaxed() {
    let mut rng = StdRng::seed_from_u64(1);
    let limit = ubig!(8);

    let distr = Uniform01::new(&limit);
    let x: Relaxed = (&mut rng).sample_iter(&distr).take(1000).min().unwrap();
    assert!(x >= rbig!(~0) && x < rbig!(~1/2));
    assert!(x.denominator() <= &limit);
    let x: Relaxed = (&mut rng).sample_iter(&distr).take(1000).max().unwrap();
    assert!(x < rbig!(~1) && x > rbig!(~1/2));
    assert!(x.denominator() <= &limit);

    let distr = Uniform01::new_closed(&limit);
    let x: Relaxed = (&mut rng).sample_iter(&distr).take(1000).min().unwrap();
    assert!(x >= rbig!(~0) && x < rbig!(~1/2));
    assert!(x.denominator() <= &limit);
    let x: Relaxed = (&mut rng).sample_iter(&distr).take(1000).max().unwrap();
    assert!(x <= rbig!(~1) && x > rbig!(~1/2));
    assert!(x.denominator() <= &limit);

    let distr = Uniform01::new_open(&limit);
    let x: Relaxed = (&mut rng).sample_iter(&distr).take(1000).min().unwrap();
    assert!(x > rbig!(~0) && x < rbig!(~1/2));
    assert!(x.denominator() <= &limit);
    let x: Relaxed = (&mut rng).sample_iter(&distr).take(1000).max().unwrap();
    assert!(x < rbig!(~1) && x > rbig!(~1/2));
    assert!(x.denominator() <= &limit);

    let distr = Uniform01::new_open_closed(&limit);
    let x: Relaxed = (&mut rng).sample_iter(&distr).take(1000).min().unwrap();
    assert!(x > rbig!(~0) && x < rbig!(~1/2));
    assert!(x.denominator() <= &limit);
    let x: Relaxed = (&mut rng).sample_iter(&distr).take(1000).max().unwrap();
    assert!(x <= rbig!(~1) && x > rbig!(~1/2));
    assert!(x.denominator() <= &limit);

    // test the standard distribution
    let x: Relaxed = rng.gen();
    assert!(x >= rbig!(~0) && x < rbig!(~1));
}

#[test]
fn test_uniform() {
    let mut rng = StdRng::seed_from_u64(1);

    let distr = Uniform::from(rbig!(3)..rbig!(7));
    let x = (&mut rng).sample_iter(&distr).take(1000).min().unwrap();
    assert!(x >= rbig!(3) && x < rbig!(5));
    assert!(x.denominator() <= &ubig!(21));
    let x = (&mut rng).sample_iter(&distr).take(1000).max().unwrap();
    assert!(x < rbig!(7) && x > rbig!(5));
    assert!(x.denominator() <= &ubig!(21));

    let distr = Uniform::from(rbig!(-7)..=rbig!(-3));
    let x = (&mut rng).sample_iter(&distr).take(1000).min().unwrap();
    assert!(x >= rbig!(-7) && x < rbig!(-5));
    assert!(x.denominator() <= &ubig!(21));
    let x = (&mut rng).sample_iter(&distr).take(1000).max().unwrap();
    assert!(x <= rbig!(-3) && x > rbig!(-5));
    assert!(x.denominator() <= &ubig!(21));

    let distr = Uniform::from(rbig!(1 / 7)..rbig!(1 / 3));
    let x = (&mut rng).sample_iter(&distr).take(1000).min().unwrap();
    assert!(x >= rbig!(1 / 7) && x < rbig!(1 / 5));
    assert!(x.denominator() <= &ubig!(21));
    let x = (&mut rng).sample_iter(&distr).take(1000).max().unwrap();
    assert!(x < rbig!(1 / 3) && x > rbig!(1 / 5));
    assert!(x.denominator() <= &ubig!(21));

    let distr = Uniform::from(rbig!(-1 / 3)..=rbig!(1 / 7));
    let x = (&mut rng).sample_iter(&distr).take(1000).min().unwrap();
    assert!(x >= rbig!(-1 / 3) && x < rbig!(0));
    assert!(x.denominator() <= &ubig!(21));
    let x = (&mut rng).sample_iter(&distr).take(1000).max().unwrap();
    assert!(x <= rbig!(1 / 7) && x > rbig!(0));
    assert!(x.denominator() <= &ubig!(21));
}

/// The distinct values among 200 draws, in ascending order.
fn uniform01_support(distr: &Uniform01) -> Vec<RBig> {
    let mut rng = StdRng::seed_from_u64(7);
    let mut values: Vec<RBig> = (&mut rng).sample_iter(distr).take(200).collect();
    let relaxed: Vec<Relaxed> = (&mut rng).sample_iter(distr).take(200).collect();
    values.extend(relaxed.into_iter().map(|x| x.canonicalize()));
    values.sort();
    values.dedup();
    values
}

#[test]
fn test_uniform01_smallest_limits() {
    let (one, two, three) = (ubig!(1), ubig!(2), ubig!(3));

    // [0, 1) and (0, 1]: `limit` values with the denominator `limit`
    assert_eq!(uniform01_support(&Uniform01::new(&one)), [rbig!(0)]);
    assert_eq!(uniform01_support(&Uniform01::new(&two)), [rbig!(0), rbig!(1 / 2)]);
    assert_eq!(uniform01_support(&Uniform01::new_open_closed(&one)), [rbig!(1)]);
    assert_eq!(uniform01_support(&Uniform01::new_open_closed(&two)), [rbig!(1 / 2), rbig!(1)]);

    // [0, 1]: `limit` values with the denominator `limit - 1`
    assert_eq!(uniform01_support(&Uniform01::new_closed(&two)), [rbig!(0), rbig!(1)]);
    assert_eq!(
        uniform01_support(&Uniform01::new_closed(&three)),
        [rbig!(0), rbig!(1 / 2), rbig!(1)]
    );

    // (0, 1): `limit - 1` values with the denominator `limit`
    assert_eq!(uniform01_support(&Uniform01::new_open(&two)), [rbig!(1 / 2)]);
    assert_eq!(uniform01_support(&Uniform01::new_open(&three)), [rbig!(1 / 3), rbig!(2 / 3)]);
}

// A limit that leaves no rational to choose from is rejected by the constructor: sampling
// from `new_open(&1)` used to spin forever, and `new_closed(&1)` returned 0 only (its
// denominator `limit - 1` is zero).

#[test]
#[should_panic(expected = "(0, 1) must be at least 2")]
fn test_uniform01_open_limit_one() {
    let _ = Uniform01::new_open(&ubig!(1));
}

#[test]
#[should_panic(expected = "(0, 1) must be at least 2")]
fn test_uniform01_open_limit_zero() {
    let _ = Uniform01::new_open(&ubig!(0));
}

#[test]
#[should_panic(expected = "[0, 1] must be at least 2")]
fn test_uniform01_closed_limit_one() {
    let _ = Uniform01::new_closed(&ubig!(1));
}

#[test]
#[should_panic(expected = "[0, 1] must be at least 2")]
fn test_uniform01_closed_limit_zero() {
    let _ = Uniform01::new_closed(&ubig!(0));
}

#[test]
#[should_panic(expected = "[0, 1) must be at least 1")]
fn test_uniform01_half_open_limit_zero() {
    let _ = Uniform01::new(&ubig!(0));
}

#[test]
#[should_panic(expected = "(0, 1] must be at least 1")]
fn test_uniform01_open_closed_limit_zero() {
    let _ = Uniform01::new_open_closed(&ubig!(0));
}
