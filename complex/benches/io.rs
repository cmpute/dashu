//! Benchmarks for complex I/O (`Display` / `FromStr` in `a+bi` form).
//! Run: cargo bench -p dashu-cmplx --bench io --features rand -- --quick

use core::str::FromStr;
use criterion::{
    criterion_group, criterion_main, AxisScale, BenchmarkId, Criterion, PlotConfiguration,
};
use dashu_cmplx::CBig;
use dashu_float::FBig;
use rand_v09::prelude::*;

type C = CBig;

const SEED: u64 = 1;

fn random_cbig(precision: usize, rng: &mut impl Rng) -> C {
    let mut mk = || {
        let sig: i64 = rng.random_range(1..=i16::MAX as i64);
        let exp: isize = rng.random_range(-4i32..=4i32) as isize;
        let sig = if rng.random_bool(0.5) { -sig } else { sig };
        FBig::from_parts(sig.into(), exp)
            .with_precision(precision)
            .value()
    };
    CBig::from_parts(mk(), mk())
}

fn bench_io(criterion: &mut Criterion) {
    let mut rng = StdRng::seed_from_u64(SEED);
    let mut group = criterion.benchmark_group("io");
    group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));

    for precision in [53, 113, 256] {
        let z = random_cbig(precision, &mut rng);
        let s = z.to_string();
        group.bench_with_input(BenchmarkId::new("display", precision), &z, |bencher, z| {
            bencher.iter(|| z.to_string())
        });
        group.bench_with_input(BenchmarkId::new("parse", precision), &s, |bencher, s| {
            bencher.iter(|| C::from_str(s).unwrap())
        });
    }
    group.finish();
}

criterion_group!(benches, bench_io);
criterion_main!(benches);
