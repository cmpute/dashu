use dashu_float::{
    rand::{Uniform01, UniformFBig},
    DBig,
};
use rand_v08::{distributions::uniform::Uniform, prelude::*};

mod helper_macros;

type FBig = dashu_float::FBig;

#[test]
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
