//! Benchmarks.
//! Run: cargo bench -p dashu-int --bench primitive --features rand -- --quick
//!
//! Note: these don't work on 16-bit machines.

use criterion::{
    criterion_group, criterion_main, AxisScale, BenchmarkId, Criterion, PlotConfiguration,
};
use dashu_base::{DivExact, DivExactAssign, DivRem, Sign};
use dashu_int::{
    fast_div::ConstDivisor,
    ops::{ExtendedGcd, Gcd},
    IBig, UBig,
};
use rand_v010::prelude::*;
use std::ops::*;

const SEED: u64 = 1;

fn random_ubig<R>(bits: usize, rng: &mut R) -> UBig
where
    R: Rng + ?Sized,
{
    rng.random_range(UBig::ONE << (bits - 1)..UBig::ONE << bits)
}

fn random_ibig<R>(bits: usize, rng: &mut R) -> IBig
where
    R: Rng + ?Sized,
{
    let sign = Sign::from(rng.random_bool(0.5));
    IBig::from_parts(sign, random_ubig(bits, rng))
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

/// Exact division (`div_exact`, Hensel) against the general division, for multi-word divisors.
/// The dividend is an exact multiple of the (odd) divisor. The `_word` groups compare the in-place
/// `div_exact_assign` (single-word divisor) with the general `div_rem`, for both the divisible
/// ("hit") and not-divisible ("miss", early probe exit) cases.
fn ubig_div_exact(criterion: &mut Criterion) {
    let mut rng = StdRng::seed_from_u64(SEED);

    let mut group = criterion.benchmark_group("ubig_div_exact");
    group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));
    for log_bits in 1..=6 {
        let bits = 10usize.pow(log_bits);
        let d = random_ubig(bits / 2, &mut rng) | UBig::ONE; // odd, multi-word on 64-bit
        let q = random_ubig(bits, &mut rng);
        let n = &d * &q;

        group.bench_with_input(
            BenchmarkId::new("div_exact", format!("1e{log_bits}")),
            &(n.clone(), d.clone()),
            |bencher, (n, d)| bencher.iter(|| n.clone().div_exact(d.clone(), &())),
        );
        group.bench_with_input(
            BenchmarkId::new("div", format!("1e{log_bits}")),
            &(n, d),
            |bencher, (n, d)| bencher.iter(|| &*n / &*d),
        );
    }
    group.finish();

    let mut group = criterion.benchmark_group("ubig_div_exact_word");
    group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));
    for log_bits in 1..=7 {
        let bits = 10usize.pow(log_bits);
        let n = random_ubig(bits, &mut rng) * 1001u32;

        group.bench_with_input(
            BenchmarkId::new("div_exact_assign", format!("1e{log_bits}")),
            &n,
            |bencher, n| {
                bencher.iter(|| {
                    let mut m = n.clone();
                    m.div_exact_assign(1001u32, &());
                    m
                })
            },
        );
        group.bench_with_input(
            BenchmarkId::new("div_rem", format!("1e{log_bits}")),
            &n,
            |bencher, n| bencher.iter(|| n.div_rem(&UBig::from(1001u32))),
        );
    }
    group.finish();

    let mut group = criterion.benchmark_group("ubig_div_exact_word_miss");
    group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));
    for log_bits in 1..=7 {
        let bits = 10usize.pow(log_bits);
        // A value unlikely to be divisible by 1001 (a large prime multiple plus 1).
        let n = random_ubig(bits, &mut rng) * 1001u32 + 1u8;

        group.bench_with_input(
            BenchmarkId::new("div_exact_assign", format!("1e{log_bits}")),
            &n,
            |bencher, n| {
                bencher.iter(|| {
                    let mut m = n.clone();
                    assert!(!m.div_exact_assign(1001u32, &()));
                    m
                })
            },
        );
        group.bench_with_input(
            BenchmarkId::new("div_rem", format!("1e{log_bits}")),
            &n,
            |bencher, n| bencher.iter(|| n.div_rem(&UBig::from(1001u32))),
        );
    }
    group.finish();
}

macro_rules! add_ibig_binop_benchmark {
    ($name:ident, $method:ident, $max_log_bits:literal) => {
        fn $name(criterion: &mut Criterion) {
            let mut rng = StdRng::seed_from_u64(SEED);
            let mut group = criterion.benchmark_group(stringify!($name));
            group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));

            for log_bits in 1..=$max_log_bits {
                let bits = 10usize.pow(log_bits);
                let a = random_ibig(bits, &mut rng);
                let b = random_ibig(bits, &mut rng);
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

// IBig arithmetic — sign handling is unbench'd by the UBig groups above.
add_ibig_binop_benchmark!(ibig_add, add, 6);
add_ibig_binop_benchmark!(ibig_sub, sub, 6);
add_ibig_binop_benchmark!(ibig_mul, mul, 7);
add_ibig_binop_benchmark!(ibig_div, div, 6);

criterion_group!(
    benches,
    ubig_add,
    ubig_sub,
    ubig_mul,
    ubig_div,
    ubig_div_exact,
    ubig_gcd,
    ubig_gcd_ext,
    ubig_pow,
    ubig_modulo_mul,
    ubig_modulo_pow,
    ubig_pow_large_base,
    ubig_ilog_large,
    ubig_mul_asymmetric,
    ubig_sqr,
    ibig_add,
    ibig_sub,
    ibig_mul,
    ibig_div,
);

criterion_main!(benches);
