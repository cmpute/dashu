//! Benchmarks.

use criterion::{
    black_box, criterion_group, criterion_main, AxisScale, BenchmarkId, Criterion,
    PlotConfiguration,
};

use dashu_base::{Gcd, ExtendedGcd, RootRem};
use rand::prelude::*;

const SEED: u64 = 1;

macro_rules! uop_case {
    ($t:ty, $bits:literal, $method:ident, $rng:ident, $group:ident) => {
        let bits = $bits;
        let a: $t = $rng.gen_range(0..1 << $bits);
        $group.bench_with_input(BenchmarkId::from_parameter(bits), &bits, |bencher, _| {
            bencher.iter(|| black_box(a).$method())
        });
    };
}

macro_rules! binop_case {
    ($t:ty, $bits:literal, $method:ident, $rng:ident, $group:ident) => {
        let bits = $bits;
        let a: $t = $rng.gen_range(0..1 << $bits);
        let b: $t = $rng.gen_range(0..1 << $bits);
        $group.bench_with_input(BenchmarkId::from_parameter(bits), &bits, |bencher, _| {
            bencher.iter(|| black_box(a).$method(black_box(b)))
        });
    };
}

fn bench_gcd(criterion: &mut Criterion) {
    let mut rng = StdRng::seed_from_u64(SEED);
    let mut group = criterion.benchmark_group("gcd");
    group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));

    binop_case!(u16, 10, gcd, rng, group);
    binop_case!(u32, 20, gcd, rng, group);
    binop_case!(u64, 40, gcd, rng, group);
    binop_case!(u128, 80, gcd, rng, group);
    binop_case!(u128, 120, gcd, rng, group);

    group.finish();
    
    let mut group = criterion.benchmark_group("gcd_ext");
    group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));
    
    binop_case!(u16, 10, gcd_ext, rng, group);
    binop_case!(u32, 20, gcd_ext, rng, group);
    binop_case!(u64, 40, gcd_ext, rng, group);
    binop_case!(u128, 80, gcd_ext, rng, group);
    binop_case!(u128, 120, gcd_ext, rng, group);

    group.finish();
}

fn bench_roots(criterion: &mut Criterion) {
    let mut rng = StdRng::seed_from_u64(SEED);
    let mut group = criterion.benchmark_group("sqrt");
    group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));

    uop_case!(u16, 10, sqrt_rem, rng, group);
    uop_case!(u32, 20, sqrt_rem, rng, group);
    uop_case!(u64, 40, sqrt_rem, rng, group);
    uop_case!(u128, 80, sqrt_rem, rng, group);
    uop_case!(u128, 120, sqrt_rem, rng, group);

    group.finish();

    let mut group = criterion.benchmark_group("cbrt");
    group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));

    uop_case!(u16, 10, cbrt_rem, rng, group);
    uop_case!(u32, 20, cbrt_rem, rng, group);
    uop_case!(u64, 40, cbrt_rem, rng, group);
    uop_case!(u128, 80, cbrt_rem, rng, group);
    uop_case!(u128, 120, cbrt_rem, rng, group);

    group.finish();
}

criterion_group!(
    benches,
    bench_gcd,
    bench_roots,
);

criterion_main!(benches);
