use dashu_ratio::{rand::Uniform01, RBig, Relaxed};
use rand::{distributions::uniform::Uniform, prelude::*};

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
