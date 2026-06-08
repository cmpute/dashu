//! IO benchmarks (printing and parsing).
//! Run: cargo bench -p dashu-float --bench io --features rand -- --quick

use criterion::{
    criterion_group, criterion_main, AxisScale, BenchmarkId, Criterion, PlotConfiguration,
};
use dashu_base::Sign;
use dashu_float::DBig;
use dashu_int::{IBig, UBig};
use rand_v08::prelude::*;
use std::fmt::Write;

const SEED: u64 = 1;

fn random_dbig<R>(precision: usize, rng: &mut R) -> DBig
where
    R: Rng + ?Sized,
{
    let precision_ub = UBig::from_word(10).pow(precision + 1);
    let precision_lb = UBig::from_word(10).pow(precision);
    let significand = rng.gen_range(precision_lb..precision_ub);
    let sign = Sign::from(rng.gen_bool(0.5));
    let exponent = rng.gen_range(-(precision as isize)..(precision as isize));
    DBig::from_parts(IBig::from_parts(sign, significand), exponent)
}

fn dbig_to_string(criterion: &mut Criterion) {
    let mut rng = StdRng::seed_from_u64(SEED);
    let mut group = criterion.benchmark_group("dbig_to_string");
    group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));

    for log_prec in 1..=5 {
        let precision = 10usize.pow(log_prec);
        let a = random_dbig(precision, &mut rng);
        let mut out = String::new();
        group.bench_with_input(BenchmarkId::from_parameter(precision), &a, |bencher, ta| {
            bencher.iter(|| {
                out.clear();
                write!(&mut out, "{}", ta).unwrap();
                out.len()
            })
        });
    }

    group.finish();
}

fn dbig_from_str(criterion: &mut Criterion) {
    let mut rng = StdRng::seed_from_u64(SEED);
    let mut group = criterion.benchmark_group("dbig_from_str");
    group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));

    for log_prec in 1..=5 {
        let precision = 10usize.pow(log_prec);
        let a = random_dbig(precision, &mut rng);
        let s = a.to_string();
        group.bench_with_input(BenchmarkId::from_parameter(precision), &s, |bencher, ts| {
            bencher.iter(|| ts.parse::<DBig>())
        });
    }

    group.finish();
}

fn dbig_scientific_fmt(criterion: &mut Criterion) {
    let mut rng = StdRng::seed_from_u64(SEED);
    let mut group = criterion.benchmark_group("dbig_scientific_fmt");
    group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));

    for log_prec in 1..=5 {
        let precision = 10usize.pow(log_prec);
        let a = random_dbig(precision, &mut rng);
        let mut out = String::new();
        group.bench_with_input(BenchmarkId::from_parameter(precision), &a, |bencher, ta| {
            bencher.iter(|| {
                out.clear();
                write!(&mut out, "{:e}", ta).unwrap();
                out.len()
            })
        });
    }

    group.finish();
}

criterion_group!(benches, dbig_to_string, dbig_from_str, dbig_scientific_fmt,);

criterion_main!(benches);
