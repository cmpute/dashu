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
                    BenchmarkId::from_parameter(format!("1e{}", log_bits)),
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
add_binop_benchmark!(ubig_mul, mul, 7);
add_binop_benchmark!(ubig_div, div, 6);
add_binop_benchmark!(ubig_gcd, gcd, 6);
add_binop_benchmark!(ubig_gcd_ext, gcd_ext, 5);

/// Division with operands of very different sizes (asymmetric): a large dividend
/// divided by a much smaller divisor, producing a large quotient.
///
/// This is the regime where the Newton (reciprocal based) division pays off:
/// the cost of computing the divisor's reciprocal is amortized across many
/// quotient blocks, each produced by plain multiplications. The symmetric
/// `ubig_div` benchmark above produces only a tiny quotient, so it stays on the
/// schoolbook path regardless of operand size.
fn ubig_div_asymmetric(criterion: &mut Criterion) {
    let mut rng = StdRng::seed_from_u64(SEED);
    let mut group = criterion.benchmark_group("ubig_div_asymmetric");
    group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));

    // divisor bit sizes large enough to reach the Newton regime, crossed with
    // the dividend/divisor ratio.
    for &divisor_bits in &[100_000usize, 300_000, 500_000, 1_000_000] {
        for &ratio in &[4usize, 64] {
            let divisor = random_ubig(divisor_bits, &mut rng);
            let dividend = random_ubig(divisor_bits * ratio, &mut rng);
            // dividend is far larger than the divisor, so the quotient has
            // ~divisor_bits * (ratio-1) bits.
            group.bench_with_input(
                BenchmarkId::from_parameter(format!("{divisor_bits}/{ratio}")),
                &(dividend, divisor),
                |bencher, (n, d)| bencher.iter(|| n.div(d)),
            );
        }
    }

    group.finish();
}

fn ubig_pow(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("ubig_pow");
    group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));

    for log_power in 1..=6 {
        let p = 10usize.pow(log_power);
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("1e{}", log_power)),
            &p,
            |bencher, p| bencher.iter(|| UBig::from(3u8).pow(*p)),
        );
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
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("1e{}", log_bits)),
            &(a, b),
            |bencher, (ta, tb)| bencher.iter(|| ta * tb),
        );
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
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("1e{}", log_bits)),
            &(a, b),
            |bencher, (ta, tb)| bencher.iter(|| ta.pow(tb)),
        );
    }

    group.finish();
}

fn ubig_pow_large_base(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("ubig_pow_large_base");
    group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));

    let base = UBig::from(12345u32);
    for log_exp in 1..=6usize {
        let exp = 10usize.pow(log_exp as u32);
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("1e{}", log_exp)),
            &exp,
            |bencher, exp| bencher.iter(|| base.pow(*exp)),
        );
    }

    group.finish();
}

fn ubig_ilog_large(criterion: &mut Criterion) {
    let mut rng = StdRng::seed_from_u64(SEED);
    let mut group = criterion.benchmark_group("ubig_ilog_large");
    group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));

    let base = UBig::from(3u8);
    for log_bits in 1..=6usize {
        let bits = 10usize.pow(log_bits as u32);
        let n = random_ubig(bits, &mut rng);
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("1e{}", log_bits)),
            &n,
            |bencher, tn| bencher.iter(|| tn.ilog(&base)),
        );
    }

    group.finish();
}

fn ubig_mul_asymmetric(criterion: &mut Criterion) {
    let mut rng = StdRng::seed_from_u64(SEED);
    let mut group = criterion.benchmark_group("ubig_mul_asymmetric");
    group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));

    // b just above the NTT threshold (4 000 words = 256 kbits → use 500 kbits).
    let b_bits = 500_000;
    let b = random_ubig(b_bits, &mut rng);

    // a ranges from 1 kbit (below Karatsuba threshold) to heavily
    // asymmetric (10×), exercising all chunked-mul code paths.
    for &a_bits in &[
        1_000, 10_000, 100_000, 500_000, 1_000_000, 2_000_000, 5_000_000,
    ] {
        let a = random_ubig(a_bits, &mut rng);
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{a_bits}/{b_bits}")),
            &(a, &b),
            |bencher, (ta, tb)| bencher.iter(|| ta * *tb),
        );
    }

    group.finish();
}

fn ubig_sqr(criterion: &mut Criterion) {
    let mut rng = StdRng::seed_from_u64(SEED);
    let mut group = criterion.benchmark_group("ubig_sqr");
    group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));

    for log_bits in 1..=6 {
        if log_bits >= 5 {
            group.sample_size(10);
        }
        let bits = 10usize.pow(log_bits);
        let a = random_ubig(bits, &mut rng);
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("1e{}", log_bits)),
            &a,
            |bencher, ta| bencher.iter(|| ta.sqr()),
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    ubig_add,
    ubig_sub,
    ubig_mul,
    ubig_div,
    ubig_div_asymmetric,
    ubig_gcd,
    ubig_gcd_ext,
    ubig_pow,
    ubig_modulo_mul,
    ubig_modulo_pow,
    ubig_pow_large_base,
    ubig_ilog_large,
    ubig_mul_asymmetric,
    ubig_sqr,
);

criterion_main!(benches);
