//! Benchmarks for complex field arithmetic (`mul`/`div`/`sqr`).
//! Run: cargo bench -p dashu-cmplx --bench arith --features rand -- --quick

use criterion::{
    criterion_group, criterion_main, AxisScale, BenchmarkId, Criterion, PlotConfiguration,
};
use dashu_cmplx::CBig;
use dashu_float::FBig;
use rand_v09::prelude::*;

type C = CBig; // base-2, Zero rounding (the default)

const SEED: u64 = 1;

/// Random base-2 complex number at the given precision, with a random sign and modest magnitude.
fn random_cbig(precision: usize, rng: &mut impl Rng) -> C {
    let mut mk = || {
        let sig: i64 = rng.random_range(i16::MIN as i64..=i16::MAX as i64);
        if sig == 0 {
            return FBig::ZERO.with_precision(precision).value();
        }
        let exp: isize = rng.random_range(-8i32..=8i32) as isize;
        let sig = if rng.random_bool(0.5) { -sig } else { sig };
        FBig::from_parts(sig.into(), exp)
            .with_precision(precision)
            .value()
    };
    CBig::from_parts(mk(), mk())
}

fn bench_arith(criterion: &mut Criterion) {
    let mut rng = StdRng::seed_from_u64(SEED);
    let mut group = criterion.benchmark_group("arith");
    group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));

    for precision in [53, 113, 256, 1024] {
        let a = random_cbig(precision, &mut rng);
        let b = random_cbig(precision, &mut rng);
        group.bench_with_input(
            BenchmarkId::new("mul", precision),
            &(&a, &b),
            |bencher, &(a, b)| bencher.iter(|| a * b),
        );
        group.bench_with_input(
            BenchmarkId::new("div", precision),
            &(&a, &b),
            |bencher, &(a, b)| bencher.iter(|| a / b),
        );
        group.bench_with_input(BenchmarkId::new("sqr", precision), &a, |bencher, a| {
            bencher.iter(|| a.sqr())
        });
    }
    group.finish();
}

criterion_group!(benches, bench_arith);
criterion_main!(benches);
