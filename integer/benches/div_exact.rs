//! Benchmarks for exact division (Hensel) vs general division.
//!
//! Run (full):  `cargo bench -p dashu-int --bench div_exact --features rand`
//! Run (quick): `cargo bench -p dashu-int --bench div_exact --features rand -- --sample-size 10 --warm-up-time 1 --measurement-time 1`
//!
//! `div_exact` uses Hensel (2-adic) division — multiplies and subtracts only, no reciprocal or
//! normalization — so it should beat the general division whenever the dividend is known to be
//! divisible. The `_word` groups compare the in-place `div_exact_const` (single-word divisor) with
//! the general `div_rem`, for both the divisible ("hit") and not-divisible ("miss", early probe
//! exit) cases.

use criterion::{
    criterion_group, criterion_main, AxisScale, BenchmarkId, Criterion, PlotConfiguration,
};
use dashu_base::{DivExact, DivRem};
use dashu_int::UBig;
use rand_v010::{prelude::*, Rng, RngExt, SeedableRng};

const SEED: u64 = 1;

fn random_ubig<R>(bits: usize, rng: &mut R) -> UBig
where
    R: Rng + ?Sized,
{
    rng.random_range(UBig::ONE << (bits - 1)..UBig::ONE << bits)
}

/// Exact division by a multi-word divisor, against the general division on the same operands.
/// The dividend is an exact multiple of the (odd) divisor.
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
            |bencher, (n, d)| bencher.iter(|| n.clone().div_exact(d.clone())),
        );
        group.bench_with_input(
            BenchmarkId::new("div", format!("1e{log_bits}")),
            &(n, d),
            |bencher, (n, d)| bencher.iter(|| &*n / &*d),
        );
    }

    group.finish();
}

/// Exact division by a single word (`div_exact_const`, in place), against the general
/// `div_rem`. The dividend is a multiple of the word divisor.
fn ubig_div_exact_word(criterion: &mut Criterion) {
    let mut rng = StdRng::seed_from_u64(SEED);
    let mut group = criterion.benchmark_group("ubig_div_exact_word");
    group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));

    for log_bits in 1..=7 {
        let bits = 10usize.pow(log_bits);
        let n = random_ubig(bits, &mut rng) * 1001u32;

        group.bench_with_input(
            BenchmarkId::new("div_exact_const", format!("1e{log_bits}")),
            &n,
            |bencher, n| {
                bencher.iter(|| {
                    let mut m = n.clone();
                    m.div_exact_const(1001);
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

/// The not-divisible case for a single-word divisor: `div_exact_const` exits via the read-only
/// Hensel probe, while the general division computes a full remainder.
fn ubig_div_exact_word_miss(criterion: &mut Criterion) {
    let mut rng = StdRng::seed_from_u64(SEED);
    let mut group = criterion.benchmark_group("ubig_div_exact_word_miss");
    group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));

    for log_bits in 1..=7 {
        let bits = 10usize.pow(log_bits);
        // A value unlikely to be divisible by 1001 (a large prime multiple plus 1).
        let n = random_ubig(bits, &mut rng) * 1001u32 + 1u8;

        group.bench_with_input(
            BenchmarkId::new("div_exact_const", format!("1e{log_bits}")),
            &n,
            |bencher, n| {
                bencher.iter(|| {
                    let mut m = n.clone();
                    assert!(!m.div_exact_const(1001));
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

criterion_group!(benches, ubig_div_exact, ubig_div_exact_word, ubig_div_exact_word_miss,);

criterion_main!(benches);
