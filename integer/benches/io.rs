//! IO benchmarks (printing and parsing).
//! Run: cargo bench -p dashu-int --bench io --features rand -- --quick

use criterion::{
    criterion_group, criterion_main, AxisScale, BenchmarkId, Criterion, PlotConfiguration,
};
use dashu_int::UBig;
use rand_v08::prelude::*;
use std::fmt::Write;

const SEED: u64 = 1;

fn random_ubig<R>(bits: usize, rng: &mut R) -> UBig
where
    R: Rng + ?Sized,
{
    rng.gen_range(UBig::ONE << (bits - 1)..UBig::ONE << bits)
}

fn ubig_to_hex(criterion: &mut Criterion) {
    let mut rng = StdRng::seed_from_u64(SEED);
    let mut group = criterion.benchmark_group("ubig_to_hex");
    group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));

    for log_bits in 1..=6 {
        let bits = 10usize.pow(log_bits);
        let a = random_ubig(bits, &mut rng);
        let mut out = String::with_capacity(bits / 4 + 1);
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("1e{}", log_bits)),
            &a,
            |bencher, ta| {
                bencher.iter(|| {
                    out.clear();
                    write!(&mut out, "{:x}", &ta).unwrap();
                    out.len()
                })
            },
        );
    }

    group.finish();
}

fn ubig_to_dec(criterion: &mut Criterion) {
    let mut rng = StdRng::seed_from_u64(SEED);
    let mut group = criterion.benchmark_group("ubig_to_dec");
    group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));

    for log_bits in 1..=6 {
        let bits = 10usize.pow(log_bits);
        let a = random_ubig(bits, &mut rng);
        let mut out = String::with_capacity(bits / 3 + 1);
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("1e{}", log_bits)),
            &a,
            |bencher, ta| {
                bencher.iter(|| {
                    out.clear();
                    write!(&mut out, "{}", &ta).unwrap();
                    out.len()
                })
            },
        );
    }

    group.finish();
}

fn ubig_from_hex(criterion: &mut Criterion) {
    let mut rng = StdRng::seed_from_u64(SEED);
    let mut group = criterion.benchmark_group("ubig_from_hex");
    group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));

    for log_bits in 1..=6 {
        let bits = 10usize.pow(log_bits);
        let a = random_ubig(bits, &mut rng);
        let s = a.in_radix(16).to_string();
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("1e{}", log_bits)),
            &s,
            |bencher, ts| bencher.iter(|| UBig::from_str_radix(ts, 16)),
        );
    }

    group.finish();
}

fn ubig_from_dec(criterion: &mut Criterion) {
    let mut rng = StdRng::seed_from_u64(SEED);
    let mut group = criterion.benchmark_group("ubig_from_dec");
    group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));

    for log_bits in 1..=6 {
        let bits = 10usize.pow(log_bits);
        let a = random_ubig(bits, &mut rng);
        let s = a.in_radix(10).to_string();
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("1e{}", log_bits)),
            &s,
            |bencher, ts| bencher.iter(|| UBig::from_str_radix(ts, 10)),
        );
    }

    group.finish();
}

criterion_group!(benches, ubig_to_hex, ubig_to_dec, ubig_from_hex, ubig_from_dec,);

criterion_main!(benches);
