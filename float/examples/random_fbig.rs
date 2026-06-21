//! Generating random `FBig` values with the default rand version (0.8).
//!
//! This is the single runnable demonstration of the `dashu_float::rand` distributions (it
//! replaces the per-version examples that used to live on the `rand_vXX` modules).
//!
//! Run with: `cargo run --example random_fbig --features rand`

use dashu_float::rand::Uniform01;
use dashu_float::FBig;
use rand_v08::{
    distributions::{Open01, OpenClosed01, Standard},
    rngs::StdRng,
    Rng, SeedableRng,
};

fn main() {
    let mut rng = StdRng::seed_from_u64(0x1234_5678_9abc_def0);

    // Uniform01 generates an FBig in [0, 1) at a precision you choose (here 20 bits).
    let a: FBig = rng.sample(Uniform01::new(20));
    let b: FBig = rng.sample(Uniform01::new_closed(20));
    println!("a = {a}   (Uniform01,        [0, 1), precision 20)");
    println!("b = {b}   (Uniform01 closed, [0, 1], precision 20)");

    // The builtin rand distributions pick the largest precision that fits a DoubleWord, so no
    // allocation is needed: Standard -> [0, 1), Open01 -> (0, 1), OpenClosed01 -> (0, 1].
    let c: FBig = rng.sample(Standard);
    let d: FBig = rng.sample(Open01);
    let e: FBig = rng.sample(OpenClosed01);
    println!("c = {c}   (Standard,      [0, 1))");
    println!("d = {d}   (Open01,        (0, 1))");
    println!("e = {e}   (OpenClosed01,  (0, 1])");

    // `rng.gen()` is sugar for sampling with `Standard`.
    let f: FBig = rng.gen();
    println!("f = {f}   (rng.gen(),     [0, 1))");
}
