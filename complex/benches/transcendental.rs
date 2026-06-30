//! Benchmarks for complex transcendentals (`exp`/`log`/`sin`/`cos`/`sqrt`/`abs`/`arg`).
//! Run: cargo bench -p dashu-cmplx --bench transcendental --features rand -- --quick
//!
//! Inputs are drawn from a modest range so the transcendentals stay well-conditioned and away
//! from branch cuts.

use criterion::{
    criterion_group, criterion_main, AxisScale, BenchmarkId, Criterion, PlotConfiguration,
};
use dashu_cmplx::CBig;
use dashu_float::FBig;
use rand_v09::prelude::*;

type C = CBig;

const SEED: u64 = 1;

/// Random base-2 complex number with modest magnitude (keeps the transcendentals well-conditioned).
fn random_cbig(precision: usize, rng: &mut impl Rng) -> C {
    let mut mk = || {
        let sig: i64 = rng.random_range(1..=i16::MAX as i64);
        let exp: isize = rng.random_range(-6i32..=-1i32) as isize;
        let sig = if rng.random_bool(0.5) { -sig } else { sig };
        FBig::from_parts(sig.into(), exp)
            .with_precision(precision)
            .value()
    };
    CBig::from_parts(mk(), mk())
}

macro_rules! unary_bench {
    ($name:ident, $method:ident) => {
        fn $name(criterion: &mut Criterion) {
            let mut rng = StdRng::seed_from_u64(SEED);
            let mut group = criterion.benchmark_group(stringify!($name));
            group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));
            for precision in [53, 113, 256] {
                let a = random_cbig(precision, &mut rng);
                group.bench_with_input(BenchmarkId::from_parameter(precision), &a, |bencher, a| {
                    bencher.iter(|| a.$method())
                });
            }
            group.finish();
        }
    };
}

unary_bench!(exp, exp);
unary_bench!(ln, ln);
unary_bench!(sin, sin);
unary_bench!(cos, cos);
unary_bench!(sqrt, sqrt);

fn abs_arg(criterion: &mut Criterion) {
    let mut rng = StdRng::seed_from_u64(SEED);
    let mut group = criterion.benchmark_group("abs_arg");
    group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));
    for precision in [53, 113, 256] {
        let a = random_cbig(precision, &mut rng);
        group.bench_with_input(BenchmarkId::new("abs", precision), &a, |bencher, a| {
            bencher.iter(|| a.abs())
        });
        group.bench_with_input(BenchmarkId::new("arg", precision), &a, |bencher, a| {
            bencher.iter(|| a.arg())
        });
    }
    group.finish();
}

criterion_group!(benches, exp, ln, sin, cos, sqrt, abs_arg);
criterion_main!(benches);
