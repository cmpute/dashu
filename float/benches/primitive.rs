//! Benchmarks.
//! Run: cargo bench -p dashu-float --bench primitive --features rand -- --quick

use std::ops::*;
use criterion::{
    black_box, criterion_group, criterion_main, AxisScale, BenchmarkId, Criterion,
    PlotConfiguration,
};
use dashu_base::Sign;
use dashu_float::{
    FBig, DBig,
};
use dashu_int::{UBig, IBig};
use rand_v08::prelude::*;

type FBin = FBig;

const SEED: u64 = 1;

fn random_fbig<R>(precision: usize, rng: &mut R) -> FBin
where
    R: Rng + ?Sized,
{
    let precision_ub = UBig::ONE << (precision + 1);
    let precision_lb = UBig::ONE << precision;
    let significand = rng.gen_range(precision_lb .. precision_ub);
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
    let significand = rng.gen_range(precision_lb .. precision_ub);
    let sign = Sign::from(rng.gen_bool(0.5));
    let exponent = rng.gen_range(-(precision as isize)..(precision as isize));
    DBig::from_parts(IBig::from_parts(sign, significand), exponent)
}

macro_rules! add_binop_benchmark {
    (fbig, $name:ident, $method:ident, $max_log_prec:literal) => {
        fn $name(criterion: &mut Criterion) {
            let mut rng = StdRng::seed_from_u64(SEED);
            let mut group = criterion.benchmark_group(stringify!($name));
            group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));

            for log_prec in 1..=$max_log_prec {
                let precision = 10usize.pow(log_prec);
                let a = random_fbig(precision, &mut rng);
                let b = random_fbig(precision, &mut rng);
                group.bench_with_input(BenchmarkId::from_parameter(precision), &precision, |bencher, _| {
                    bencher.iter(|| black_box(&a).$method(black_box(&b)))
                });
            }

            group.finish();
        }
    };
    (dbig, $name:ident, $method:ident, $max_log_prec:literal) => {
        fn $name(criterion: &mut Criterion) {
            let mut rng = StdRng::seed_from_u64(SEED);
            let mut group = criterion.benchmark_group(stringify!($name));
            group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));

            for log_prec in 1..=$max_log_prec {
                let precision = 10usize.pow(log_prec);
                let a = random_dbig(precision, &mut rng);
                let b = random_dbig(precision, &mut rng);
                group.bench_with_input(BenchmarkId::from_parameter(precision), &precision, |bencher, _| {
                    bencher.iter(|| black_box(&a).$method(black_box(&b)))
                });
            }

            group.finish();
        }
    };
}
add_binop_benchmark!(fbig, fbig_add, add, 6);
add_binop_benchmark!(dbig, dbig_add, add, 5);
add_binop_benchmark!(fbig, fbig_sub, sub, 6);
add_binop_benchmark!(dbig, dbig_sub, sub, 5);
add_binop_benchmark!(fbig, fbig_mul, mul, 6);
add_binop_benchmark!(dbig, dbig_mul, mul, 5);
add_binop_benchmark!(fbig, fbig_div, div, 6);
add_binop_benchmark!(dbig, dbig_div, div, 5);

criterion_group!(
    benches,

    fbig_add,
    fbig_sub,
    fbig_mul,
    fbig_div,

    dbig_add,
    dbig_sub,
    dbig_mul,
    dbig_div,
);

criterion_main!(benches);
