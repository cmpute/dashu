//! IO benchmarks (printing and parsing).
//! Run: cargo bench -p dashu-ratio --bench io --features rand -- --quick

use criterion::{
    criterion_group, criterion_main, AxisScale, BenchmarkId, Criterion, PlotConfiguration,
};
use dashu_base::Sign;
use dashu_int::{IBig, UBig};
use dashu_ratio::RBig;
use rand_v08::prelude::*;
use std::fmt::Write;

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

fn rbig_to_string(criterion: &mut Criterion) {
    let mut rng = StdRng::seed_from_u64(SEED);
    let mut group = criterion.benchmark_group("rbig_to_string");
    group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));

    for log_bits in 1..=6 {
        let bits = 10usize.pow(log_bits);
        let a = random_rbig(bits, &mut rng);
        let mut out = String::new();
        group.bench_with_input(BenchmarkId::from_parameter(bits), &a, |bencher, ta| {
            bencher.iter(|| {
                out.clear();
                write!(&mut out, "{}", ta).unwrap();
                out.len()
            })
        });
    }

    group.finish();
}

fn rbig_from_str(criterion: &mut Criterion) {
    let mut rng = StdRng::seed_from_u64(SEED);
    let mut group = criterion.benchmark_group("rbig_from_str");
    group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));

    for log_bits in 1..=6 {
        let bits = 10usize.pow(log_bits);
        let a = random_rbig(bits, &mut rng);
        let s = a.to_string();
        group.bench_with_input(BenchmarkId::from_parameter(bits), &s, |bencher, ts| {
            bencher.iter(|| ts.parse::<RBig>())
        });
    }

    group.finish();
}

fn rbig_in_radix_fmt(criterion: &mut Criterion) {
    let mut rng = StdRng::seed_from_u64(SEED);
    let mut group = criterion.benchmark_group("rbig_in_radix_fmt");
    group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));

    for &radix in &[3, 16] {
        for log_bits in 1..=6 {
            let bits = 10usize.pow(log_bits);
            let a = random_rbig(bits, &mut rng);
            let mut out = String::new();
            let param = format!("radix={},bits={}", radix, bits);
            group.bench_with_input(
                BenchmarkId::from_parameter(param),
                &(a, radix),
                |bencher, (ta, radix)| {
                    bencher.iter(|| {
                        out.clear();
                        write!(&mut out, "{}", ta.in_radix(*radix)).unwrap();
                        out.len()
                    })
                },
            );
        }
    }

    group.finish();
}

fn rbig_in_expanded_fmt(criterion: &mut Criterion) {
    let mut rng = StdRng::seed_from_u64(SEED);
    let mut group = criterion.benchmark_group("rbig_in_expanded_fmt");
    group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));

    // Use moderate-size rationals and vary output precision.
    let a = random_rbig(1000, &mut rng);
    for &prec in &[10, 100, 1000, 10000] {
        let mut out = String::new();
        group.bench_with_input(BenchmarkId::from_parameter(prec), &prec, |bencher, prec| {
            bencher.iter(|| {
                out.clear();
                write!(&mut out, "{:.prec$}", a.in_expanded(10), prec = prec).unwrap();
                out.len()
            })
        });
    }

    group.finish();
}

fn rbig_in_expanded_scientific(criterion: &mut Criterion) {
    let mut rng = StdRng::seed_from_u64(SEED);
    let mut group = criterion.benchmark_group("rbig_in_expanded_scientific");
    group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));

    let a = random_rbig(1000, &mut rng);
    for &prec in &[10, 100, 1000, 10000] {
        let mut out = String::new();
        group.bench_with_input(BenchmarkId::from_parameter(prec), &prec, |bencher, prec| {
            bencher.iter(|| {
                out.clear();
                write!(&mut out, "{:.prec$e}", a.in_expanded(10), prec = prec).unwrap();
                out.len()
            })
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    rbig_to_string,
    rbig_from_str,
    rbig_in_radix_fmt,
    rbig_in_expanded_fmt,
    rbig_in_expanded_scientific,
);

criterion_main!(benches);
