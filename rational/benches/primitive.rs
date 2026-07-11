//! Benchmarks.
//! Run: cargo bench -p dashu-float --bench primitive --features rand -- --quick

use criterion::{
    criterion_group, criterion_main, AxisScale, BenchmarkId, Criterion, PlotConfiguration,
};
use dashu_base::Sign;
use dashu_int::{IBig, UBig};
use dashu_ratio::RBig;
use rand_v08::prelude::*;
use std::ops::*;

const SEED: u64 = 1;

fn random_rbig<R>(bits: usize, rng: &mut R) -> RBig
where
    R: Rng + ?Sized,
{
    let sign = Sign::from(rng.gen_bool(0.5));
    let numerator =
        IBig::from_parts(sign, rng.gen_range(UBig::ONE << (bits - 1)..UBig::ONE << bits));
    let denominator = rng.gen_range(UBig::ONE << (bits - 1)..UBig::ONE << bits);
    RBig::from_parts(numerator, denominator)
}

fn random_ubig<R>(bits: usize, rng: &mut R) -> UBig
where
    R: Rng + ?Sized,
{
    rng.gen_range(UBig::ONE << (bits - 1)..UBig::ONE << bits)
}

macro_rules! add_binop_benchmark {
    ($name:ident, $method:ident, $max_log_bits:literal) => {
        fn $name(criterion: &mut Criterion) {
            let mut rng = StdRng::seed_from_u64(SEED);
            let mut group = criterion.benchmark_group(stringify!($name));
            group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));

            for log_bits in 1..=$max_log_bits {
                let bits = 10usize.pow(log_bits);
                let a = random_rbig(bits, &mut rng);
                let b = random_rbig(bits, &mut rng) + &a; // make b > a so that sub won't underflow
                group.bench_with_input(
                    BenchmarkId::from_parameter(format!("1e{}", log_bits)),
                    &(a, b),
                    |bencher, (ta, tb)| bencher.iter(|| tb.$method(ta)),
                );
            }

            group.finish();
        }
    };
}

add_binop_benchmark!(rbig_add, add, 6);
add_binop_benchmark!(rbig_sub, sub, 6);
add_binop_benchmark!(rbig_mul, mul, 6);
add_binop_benchmark!(rbig_div, div, 6);

/// Measure the GCD reduction in `RBig::from_parts`: numerator and denominator
/// share a large common factor, so canonicalization does real (not trivial) work.
fn rbig_reduction(criterion: &mut Criterion) {
    let mut rng = StdRng::seed_from_u64(SEED);
    let mut group = criterion.benchmark_group("rbig_reduction");
    group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));

    for log_bits in 1..=6 {
        let bits = 10usize.pow(log_bits);
        let common = random_ubig(bits, &mut rng);
        let n_mag = random_ubig(bits, &mut rng) * &common;
        let d = random_ubig(bits, &mut rng) * &common;
        let n = IBig::from_parts(Sign::from(rng.gen_bool(0.5)), n_mag);
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("1e{}", log_bits)),
            &(n, d),
            |bencher, (tn, td)| bencher.iter(|| RBig::from_parts(tn.clone(), td.clone())),
        );
    }

    group.finish();
}

criterion_group!(benches, rbig_add, rbig_sub, rbig_mul, rbig_div, rbig_reduction,);

criterion_main!(benches);
