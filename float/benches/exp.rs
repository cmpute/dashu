//! Benchmarks for transcendental / exponentiation operations.
//! Run: cargo bench -p dashu-float --bench exp --features rand -- --quick

use criterion::{
    criterion_group, criterion_main, AxisScale, BenchmarkId, Criterion, PlotConfiguration,
};
use dashu_base::{Abs, Sign};
use dashu_float::{DBig, FBig};
use dashu_int::{IBig, UBig};
use rand_v08::prelude::*;
use std::str::FromStr;

type FBin = FBig;

const SEED: u64 = 1;

fn random_fbig<R>(precision: usize, rng: &mut R) -> FBin
where
    R: Rng + ?Sized,
{
    let precision_ub = UBig::ONE << (precision + 1);
    let precision_lb = UBig::ONE << precision;
    let significand = rng.gen_range(precision_lb..precision_ub);
    let sign = Sign::from(rng.gen_bool(0.5));
    let exponent = rng.gen_range(-(precision as isize)..(precision as isize));
    FBin::from_parts(IBig::from_parts(sign, significand), exponent)
}

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

macro_rules! add_unary_benchmark {
    (fbig, $name:ident, $method:ident, $max_log_prec:literal, $gen:expr) => {
        fn $name(criterion: &mut Criterion) {
            let mut rng = StdRng::seed_from_u64(SEED);
            let mut group = criterion.benchmark_group(stringify!($name));
            group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));

            for log_prec in 1..=$max_log_prec {
                let precision = 10usize.pow(log_prec);
                let a: FBin = $gen(precision, &mut rng);
                group.bench_with_input(
                    BenchmarkId::from_parameter(format!("1e{}", log_prec)),
                    &a,
                    |bencher, ta| bencher.iter(|| ta.$method()),
                );
            }

            group.finish();
        }
    };
    (dbig, $name:ident, $method:ident, $max_log_prec:literal, $gen:expr) => {
        fn $name(criterion: &mut Criterion) {
            let mut rng = StdRng::seed_from_u64(SEED);
            let mut group = criterion.benchmark_group(stringify!($name));
            group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));

            for log_prec in 1..=$max_log_prec {
                let precision = 10usize.pow(log_prec);
                let a: DBig = $gen(precision, &mut rng);
                group.bench_with_input(
                    BenchmarkId::from_parameter(format!("1e{}", log_prec)),
                    &a,
                    |bencher, ta| bencher.iter(|| ta.$method()),
                );
            }

            group.finish();
        }
    };
}

// — exp — keep inputs in a moderate range so e^x doesn't blow up —
fn gen_fbig_exp(precision: usize, rng: &mut impl Rng) -> FBin {
    let signif = rng.gen_range(UBig::ONE << precision..UBig::ONE << (precision + 1));
    // exponent 0 → value in [0.5, 2.0), keeps exp result reasonable
    let exponent = -(precision as isize);
    FBin::from_parts(IBig::from(signif), exponent)
}
fn gen_dbig_exp(precision: usize, rng: &mut impl Rng) -> DBig {
    let signif =
        rng.gen_range(UBig::from_word(10).pow(precision)..UBig::from_word(10).pow(precision + 1));
    let exponent = -(precision as isize);
    DBig::from_parts(IBig::from(signif), exponent)
}

// — ln — inputs should be positive and away from 0 —
fn gen_fbig_ln(precision: usize, rng: &mut impl Rng) -> FBin {
    // value in [1, 2) → ln in [0, 0.69)
    let signif = rng.gen_range(UBig::ONE << precision..UBig::ONE << (precision + 1));
    FBin::from_parts(IBig::from(signif), -(precision as isize))
}
fn gen_dbig_ln(precision: usize, rng: &mut impl Rng) -> DBig {
    let signif =
        rng.gen_range(UBig::from_word(10).pow(precision)..UBig::from_word(10).pow(precision + 1));
    DBig::from_parts(IBig::from(signif), -(precision as isize))
}

// — powi — use a moderate integer exponent —
fn fbig_powi(criterion: &mut Criterion) {
    let mut rng = StdRng::seed_from_u64(SEED);
    let mut group = criterion.benchmark_group("fbig_powi");
    group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));

    let exp = IBig::from(7);
    for log_prec in 1..=4 {
        let precision = 10usize.pow(log_prec);
        let a = random_fbig(precision, &mut rng);
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("1e{}", log_prec)),
            &(a, exp.clone()),
            |bencher, (ta, te)| bencher.iter(|| ta.powi(te.clone())),
        );
    }

    group.finish();
}

fn dbig_powi(criterion: &mut Criterion) {
    let mut rng = StdRng::seed_from_u64(SEED);
    let mut group = criterion.benchmark_group("dbig_powi");
    group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));

    let exp = IBig::from(5);
    for log_prec in 1..=3 {
        let precision = 10usize.pow(log_prec);
        let a = random_dbig(precision, &mut rng);
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("1e{}", log_prec)),
            &(a, exp.clone()),
            |bencher, (ta, te)| bencher.iter(|| ta.powi(te.clone())),
        );
    }

    group.finish();
}

// — nth_root — sweep across n at a fixed moderate precision —
fn fbig_nth_root(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("fbig_nth_root");
    group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));

    let a: FBin = FBin::from_str("0x2").unwrap().with_precision(64).value();
    let ns = [2usize, 3, 7, 10, 100, 1000, 10000];
    for &n in &ns {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("n={n}")),
            &(a.clone(), n),
            |bencher, (ta, tn)| bencher.iter(|| ta.nth_root(*tn)),
        );
    }

    group.finish();
}

fn dbig_nth_root(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("dbig_nth_root");
    group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));

    let a = DBig::from_str("2").unwrap().with_precision(8).value();
    let ns = [2usize, 3, 7, 10, 100, 1000, 10000];
    for &n in &ns {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("n={n}")),
            &(a.clone(), n),
            |bencher, (ta, tn)| bencher.iter(|| ta.nth_root(*tn)),
        );
    }

    group.finish();
}

// — powf — use a fixed rational exponent (cube root) —
fn fbig_powf(criterion: &mut Criterion) {
    let mut rng = StdRng::seed_from_u64(SEED);
    let mut group = criterion.benchmark_group("fbig_powf");
    group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));

    let exp = FBin::from_str("0x1.5555555555555p-1").unwrap(); // ≈ 1/3
    for log_prec in 1..=4 {
        let precision = 10usize.pow(log_prec);
        // powf panics on negative bases with non-integer exponents
        let a = random_fbig(precision, &mut rng).abs();
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("1e{}", log_prec)),
            &(a, exp.clone()),
            |bencher, (ta, te)| bencher.iter(|| ta.powf(te)),
        );
    }

    group.finish();
}

fn dbig_powf(criterion: &mut Criterion) {
    let mut rng = StdRng::seed_from_u64(SEED);
    let mut group = criterion.benchmark_group("dbig_powf");
    group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));

    let exp = DBig::from_str("0.33333333333333333333")
        .unwrap()
        .with_precision(4)
        .value();
    for log_prec in 1..=3 {
        let precision = 10usize.pow(log_prec);
        let a = random_dbig(precision, &mut rng).abs();
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("1e{}", log_prec)),
            &(a, exp.clone()),
            |bencher, (ta, te)| bencher.iter(|| ta.powf(te)),
        );
    }

    group.finish();
}

add_unary_benchmark!(fbig, fbig_exp, exp, 4, gen_fbig_exp);
add_unary_benchmark!(fbig, fbig_ln, ln, 4, gen_fbig_ln);
add_unary_benchmark!(dbig, dbig_exp, exp, 3, gen_dbig_exp);
add_unary_benchmark!(dbig, dbig_ln, ln, 3, gen_dbig_ln);

criterion_group!(
    benches,
    fbig_exp,
    fbig_ln,
    fbig_powi,
    fbig_nth_root,
    fbig_powf,
    dbig_exp,
    dbig_ln,
    dbig_powi,
    dbig_nth_root,
    dbig_powf,
);

criterion_main!(benches);
