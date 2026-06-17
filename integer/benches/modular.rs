//! Benchmarks comparing the two modular-arithmetic backends:
//! Barrett reduction (`ConstDivisor` / `Reduced`) vs Montgomery reduction
//! (`MontgomeryRepr` / `Montgomery`).
//!
//! Run (full):  `cargo bench -p dashu-int --bench modular --features rand`
//! Run (quick): `cargo bench -p dashu-int --bench modular --features rand -- --sample-size 10 --warm-up-time 1 --measurement-time 1`

use criterion::{
    criterion_group, criterion_main, AxisScale, BenchmarkId, Criterion, PlotConfiguration,
};
use dashu_int::{fast_div::ConstDivisor, monty::MontgomeryRepr, UBig};
use rand_v08::prelude::*;

const SEED: u64 = 1;

/// Bit sizes of the moduli benchmarked. On a 64-bit target these span 4, 8, 16, 32, 64,
/// 128 and 256 words — covering the schoolbook, Karatsuba, Toom-3 and NTT regimes.
const BITS: &[usize] = &[256, 512, 1024, 2048, 4096, 8192, 16384];
/// A shorter list for the (expensive) pow benchmark.
const POW_BITS: &[usize] = &[256, 1024, 4096];

fn random_ubig<R: Rng + ?Sized>(bits: usize, rng: &mut R) -> UBig {
    rng.gen_range(UBig::ONE << (bits - 1)..UBig::ONE << bits)
}

/// An odd modulus in the given bit range (Montgomery requires an odd modulus).
fn random_odd_ubig<R: Rng + ?Sized>(bits: usize, rng: &mut R) -> UBig {
    random_ubig(bits, rng) | UBig::ONE
}

/// Benchmark a binary modular operation (`*`, `+`, `-`) for both backends.
macro_rules! binop_bench {
    ($group:ident, $op:tt) => {
        fn $group(c: &mut Criterion) {
            let mut rng = StdRng::seed_from_u64(SEED);
            let mut group = c.benchmark_group(stringify!($group));
            group.plot_config(
                PlotConfiguration::default().summary_scale(AxisScale::Logarithmic),
            );

            for &bits in BITS {
                let m = random_odd_ubig(bits, &mut rng);
                let barrett = ConstDivisor::new(m.clone());
                let monty = MontgomeryRepr::new(m.clone());
                let a = random_ubig(bits, &mut rng);
                let b = random_ubig(bits, &mut rng);
                let (ba, bb) = (barrett.reduce(a.clone()), barrett.reduce(b.clone()));
                let (ma, mb) = (monty.reduce(a.clone()), monty.reduce(b.clone()));

                group.bench_with_input(
                    BenchmarkId::new("barrett", bits),
                    &(ba, bb),
                    |bencher, (a, b)| bencher.iter(|| a $op b),
                );
                group.bench_with_input(
                    BenchmarkId::new("monty", bits),
                    &(ma, mb),
                    |bencher, (a, b)| bencher.iter(|| a $op b),
                );
            }

            group.finish();
        }
    };
}

binop_bench!(modular_mul, *);
binop_bench!(modular_add, +);
binop_bench!(modular_sub, -);

fn modular_sqr(c: &mut Criterion) {
    let mut rng = StdRng::seed_from_u64(SEED);
    let mut group = c.benchmark_group("modular_sqr");
    group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));

    for &bits in BITS {
        let m = random_odd_ubig(bits, &mut rng);
        let barrett = ConstDivisor::new(m.clone());
        let monty = MontgomeryRepr::new(m.clone());
        let a = random_ubig(bits, &mut rng);
        let ba = barrett.reduce(a.clone());
        let ma = monty.reduce(a.clone());

        group.bench_with_input(BenchmarkId::new("barrett", bits), &ba, |b, a| b.iter(|| a.sqr()));
        group.bench_with_input(BenchmarkId::new("monty", bits), &ma, |b, a| b.iter(|| a.sqr()));
    }

    group.finish();
}

fn modular_pow(c: &mut Criterion) {
    let mut rng = StdRng::seed_from_u64(SEED);
    let mut group = c.benchmark_group("modular_pow");
    group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));

    for &bits in POW_BITS {
        if bits >= 4096 {
            group.sample_size(10);
        }
        let m = random_odd_ubig(bits, &mut rng);
        let barrett = ConstDivisor::new(m.clone());
        let monty = MontgomeryRepr::new(m.clone());
        let a = random_ubig(bits, &mut rng);
        let e = random_ubig(bits, &mut rng);
        let ba = barrett.reduce(a.clone());
        let ma = monty.reduce(a.clone());

        group.bench_with_input(BenchmarkId::new("barrett", bits), &(ba, &e), |b, (a, e)| {
            b.iter(|| a.pow(e))
        });
        group.bench_with_input(BenchmarkId::new("monty", bits), &(ma, &e), |b, (a, e)| {
            b.iter(|| a.pow(e))
        });
    }

    group.finish();
}

fn modular_inv(c: &mut Criterion) {
    let mut rng = StdRng::seed_from_u64(SEED);
    let mut group = c.benchmark_group("modular_inv");
    group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));

    for &bits in BITS {
        let m = random_odd_ubig(bits, &mut rng);
        let barrett = ConstDivisor::new(m.clone());
        let monty = MontgomeryRepr::new(m.clone());
        // 2 is always coprime to an odd modulus, so the inverse always exists.
        let ba = barrett.reduce(2u8);
        let ma = monty.reduce(2u8);

        group.bench_with_input(BenchmarkId::new("barrett", bits), &ba, |b, a| {
            b.iter(|| a.clone().inv().unwrap())
        });
        group.bench_with_input(BenchmarkId::new("monty", bits), &ma, |b, a| {
            b.iter(|| a.clone().inv().unwrap())
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    modular_mul,
    modular_sqr,
    modular_add,
    modular_sub,
    modular_pow,
    modular_inv,
);
criterion_main!(benches);
