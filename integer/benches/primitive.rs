//! Benchmarks.
//! Run: cargo bench -p dashu-int --bench primitive --features rand -- --quick
//!
//! Note: these don't work on 16-bit machines.

use criterion::{
    criterion_group, criterion_main, AxisScale, BenchmarkId, Criterion, PlotConfiguration,
};
use dashu_int::{
    fast_div::ConstDivisor,
    ops::{ExtendedGcd, Gcd},
    UBig,
};
use rand_v08::prelude::*;
use std::fmt::Write;
use std::ops::*;

const SEED: u64 = 1;

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
                let a = random_ubig(bits, &mut rng);
                let b = random_ubig(bits, &mut rng) + &a; // make b > a so that sub won't underflow
                group.bench_with_input(
                    BenchmarkId::from_parameter(bits),
                    &(a, b),
                    |bencher, (ta, tb)| bencher.iter(|| tb.$method(ta)),
                );
            }

            group.finish();
        }
    };
}

add_binop_benchmark!(ubig_add, add, 6);
add_binop_benchmark!(ubig_sub, sub, 6);
add_binop_benchmark!(ubig_mul, mul, 6);
add_binop_benchmark!(ubig_div, div, 6);
add_binop_benchmark!(ubig_gcd, gcd, 6);
add_binop_benchmark!(ubig_gcd_ext, gcd_ext, 5);

fn ubig_to_hex(criterion: &mut Criterion) {
    let mut rng = StdRng::seed_from_u64(SEED);
    let mut group = criterion.benchmark_group("ubig_to_hex");
    group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));

    for log_bits in 1..=6 {
        let bits = 10usize.pow(log_bits);
        let a = random_ubig(bits, &mut rng);
        let mut out = String::with_capacity(bits / 4 + 1);
        group.bench_with_input(BenchmarkId::from_parameter(bits), &a, |bencher, ta| {
            bencher.iter(|| {
                out.clear();
                write!(&mut out, "{:x}", &ta).unwrap();
                out.len()
            })
        });
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
        group.bench_with_input(BenchmarkId::from_parameter(bits), &a, |bencher, ta| {
            bencher.iter(|| {
                out.clear();
                write!(&mut out, "{}", &ta).unwrap();
                out.len()
            })
        });
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
        group.bench_with_input(BenchmarkId::from_parameter(bits), &s, |bencher, ts| {
            bencher.iter(|| UBig::from_str_radix(ts, 16))
        });
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
        group.bench_with_input(BenchmarkId::from_parameter(bits), &s, |bencher, ts| {
            bencher.iter(|| UBig::from_str_radix(ts, 10))
        });
    }

    group.finish();
}

fn ubig_pow(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("ubig_pow");
    group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));

    for log_power in 1..=6 {
        let p = 10usize.pow(log_power);
        group.bench_with_input(BenchmarkId::from_parameter(p), &p, |bencher, p| {
            bencher.iter(|| UBig::from(3u8).pow(*p))
        });
    }

    group.finish();
}

fn ubig_modulo_mul(criterion: &mut Criterion) {
    let mut rng = StdRng::seed_from_u64(SEED);
    let mut group = criterion.benchmark_group("ubig_modulo_mul");
    group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));

    for log_bits in 1..=6 {
        let bits = 10usize.pow(log_bits);
        let m = random_ubig(bits, &mut rng);
        let ring = ConstDivisor::new(m);
        let a = ring.reduce(random_ubig(bits, &mut rng));
        let b = ring.reduce(random_ubig(bits, &mut rng));
        group.bench_with_input(BenchmarkId::from_parameter(bits), &(a, b), |bencher, (ta, tb)| {
            bencher.iter(|| ta * tb)
        });
    }

    group.finish();
}

fn ubig_modulo_pow(criterion: &mut Criterion) {
    let mut rng = StdRng::seed_from_u64(SEED);
    let mut group = criterion.benchmark_group("ubig_modulo_pow");
    group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));

    for log_bits in 1..=4 {
        if log_bits == 4 {
            group.sample_size(10);
        }
        let bits = 10usize.pow(log_bits);
        let m = random_ubig(bits, &mut rng);
        let ring = ConstDivisor::new(m);
        let a = ring.reduce(random_ubig(2048, &mut rng));
        let b = random_ubig(bits, &mut rng);
        group.bench_with_input(BenchmarkId::from_parameter(bits), &(a, b), |bencher, (ta, tb)| {
            bencher.iter(|| ta.pow(tb))
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    ubig_add,
    ubig_sub,
    ubig_mul,
    ubig_div,
    ubig_gcd,
    ubig_gcd_ext,
    ubig_to_hex,
    ubig_to_dec,
    ubig_from_hex,
    ubig_from_dec,
    ubig_pow,
    ubig_modulo_mul,
    ubig_modulo_pow,
);

criterion_main!(benches);
