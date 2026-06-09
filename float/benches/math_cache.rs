use std::hint::black_box;
use std::time::Instant;

use dashu_float::DBig;
use rand_v08::prelude::*;

fn main() {
    println!("Benchmarking Math Cache Scaling & Usage Patterns");

    // Test for different magnitude scales
    let scales = [100, 10000, 100000];

    println!(
        "{:<10} | {:<20} | {:<20} | {:<20} | {:<20}",
        "Scale (S)",
        "Random NO Cache",
        "Random WITH Cache",
        "Progressive NO Cache",
        "Progressive WITH Cache"
    );
    println!("{:-<10}-|-{:-<20}-|-{:-<20}-|-{:-<20}-|-{:-<20}-", "", "", "", "", "");

    let mut rng = rand_v08::rngs::StdRng::seed_from_u64(42);

    for &s in &scales {
        // Generate 10 random precisions around S (+/- 10%)
        let min_prec = (s as f64 * 0.9) as usize;
        let max_prec = (s as f64 * 1.1) as usize;
        let random_precs: Vec<usize> = (0..10)
            .map(|_| rng.gen_range(min_prec..=max_prec))
            .collect();

        // Generate 10 progressive steps from S/2 to S
        let step = s / 20;
        let progressive_precs: Vec<usize> = (0..10).map(|i| (s / 2) + i * step).collect();

        // 1. Random NO Cache
        let mut time_random_no_cache = 0.0;
        for &prec in &random_precs {
            dashu_float::math::consts::clear_math_caches();
            let start = Instant::now();
            black_box(DBig::pi(prec));
            time_random_no_cache += start.elapsed().as_secs_f64() * 1000.0;
        }

        // 2. Random WITH Cache
        dashu_float::math::consts::clear_math_caches();
        let start = Instant::now();
        for &prec in &random_precs {
            black_box(DBig::pi(prec));
        }
        let time_random_with_cache = start.elapsed().as_secs_f64() * 1000.0;

        // 3. Progressive NO Cache
        let mut time_prog_no_cache = 0.0;
        for &prec in &progressive_precs {
            dashu_float::math::consts::clear_math_caches();
            let start = Instant::now();
            black_box(DBig::pi(prec));
            time_prog_no_cache += start.elapsed().as_secs_f64() * 1000.0;
        }

        // 4. Progressive WITH Cache
        dashu_float::math::consts::clear_math_caches();
        let start = Instant::now();
        for &prec in &progressive_precs {
            black_box(DBig::pi(prec));
        }
        let time_prog_with_cache = start.elapsed().as_secs_f64() * 1000.0;

        println!(
            "{:<10} | {:<15.2} ms | {:<15.2} ms | {:<15.2} ms | {:<15.2} ms",
            s,
            time_random_no_cache,
            time_random_with_cache,
            time_prog_no_cache,
            time_prog_with_cache
        );
    }
}
