//! Benchmarks for trigonometric operations.
//! Run: cargo bench -p dashu-float --bench trig --features rand -- --quick
//!
//! All inputs are drawn from (-1, 0) ∪ (0, 1): well away from the tan/atan
//! singularities, strictly inside the asin/acos domain, and never the (0, 0)
//! origin that atan2 rejects.

use criterion::{
    criterion_group, criterion_main, AxisScale, BenchmarkId, Criterion, PlotConfiguration,
};
use dashu_base::Sign;
use dashu_float::{DBig, FBig};
use dashu_int::{IBig, UBig};
use rand_v08::prelude::*;

type FBin = FBig;

const SEED: u64 = 1;

/// Nonzero value with magnitude in (0, 1) and random sign.
fn gen_fbig(precision: usize, rng: &mut impl Rng) -> FBin {
    let signif = rng.gen_range(UBig::ONE..UBig::ONE << precision);
    let sign = Sign::from(rng.gen_bool(0.5));
    FBin::from_parts(IBig::from_parts(sign, signif), -(precision as isize))
}

fn gen_dbig(precision: usize, rng: &mut impl Rng) -> DBig {
    let signif = rng.gen_range(UBig::ONE..UBig::from_word(10).pow(precision));
    let sign = Sign::from(rng.gen_bool(0.5));
    DBig::from_parts(IBig::from_parts(sign, signif), -(precision as isize))
}

macro_rules! add_unary_trig_bench {
    (fbig, $name:ident, $method:ident) => {
        fn $name(criterion: &mut Criterion) {
            let mut rng = StdRng::seed_from_u64(SEED);
            let mut group = criterion.benchmark_group(stringify!($name));
            group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));
            for log_prec in 1..=4 {
                let precision = 10usize.pow(log_prec);
                let a: FBin = gen_fbig(precision, &mut rng);
                group.bench_with_input(
                    BenchmarkId::from_parameter(format!("1e{}", log_prec)),
                    &a,
                    |bencher, ta| bencher.iter(|| ta.$method()),
                );
            }
            group.finish();
        }
    };
    (dbig, $name:ident, $method:ident) => {
        fn $name(criterion: &mut Criterion) {
            let mut rng = StdRng::seed_from_u64(SEED);
            let mut group = criterion.benchmark_group(stringify!($name));
            group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));
            for log_prec in 1..=3 {
                let precision = 10usize.pow(log_prec);
                let a: DBig = gen_dbig(precision, &mut rng);
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

fn fbig_atan2(criterion: &mut Criterion) {
    let mut rng = StdRng::seed_from_u64(SEED);
    let mut group = criterion.benchmark_group("fbig_atan2");
    group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));
    for log_prec in 1..=4 {
        let precision = 10usize.pow(log_prec);
        let y = gen_fbig(precision, &mut rng);
        let x = gen_fbig(precision, &mut rng);
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("1e{}", log_prec)),
            &(y, x),
            |bencher, (ty, tx)| bencher.iter(|| ty.atan2(tx)),
        );
    }
    group.finish();
}

add_unary_trig_bench!(fbig, fbig_sin, sin);
add_unary_trig_bench!(fbig, fbig_cos, cos);
add_unary_trig_bench!(fbig, fbig_tan, tan);
add_unary_trig_bench!(fbig, fbig_asin, asin);
add_unary_trig_bench!(fbig, fbig_acos, acos);
add_unary_trig_bench!(fbig, fbig_atan, atan);
add_unary_trig_bench!(dbig, dbig_sin, sin);
add_unary_trig_bench!(dbig, dbig_cos, cos);
add_unary_trig_bench!(dbig, dbig_tan, tan);
add_unary_trig_bench!(dbig, dbig_asin, asin);
add_unary_trig_bench!(dbig, dbig_acos, acos);
add_unary_trig_bench!(dbig, dbig_atan, atan);

criterion_group!(
    benches, fbig_sin, fbig_cos, fbig_tan, fbig_asin, fbig_acos, fbig_atan, fbig_atan2, dbig_sin,
    dbig_cos, dbig_tan, dbig_asin, dbig_acos, dbig_atan,
);
criterion_main!(benches);
