//! Benchmarks for hyperbolic operations.
//! Run: cargo bench -p dashu-float --bench hyper --features rand -- --quick
//!
//! Inputs for sinh/cosh/tanh/asinh/atanh are drawn from (-1, 0) ∪ (0, 1) — strictly
//! inside atanh's |x| < 1 domain; acosh takes x ∈ [1, 2) (its x ≥ 1 domain).

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

/// Positive value in [1, 2) — acosh's `x ≥ 1` domain.
fn gen_fbig_above_one(precision: usize, rng: &mut impl Rng) -> FBin {
    let one = UBig::ONE << precision; // 2^p ≡ 1.0
    let frac = rng.gen_range(UBig::ZERO..one.clone());
    FBin::from_parts(IBig::from(one + frac), -(precision as isize))
}

fn gen_dbig_above_one(precision: usize, rng: &mut impl Rng) -> DBig {
    let one = UBig::from_word(10).pow(precision); // 10^p ≡ 1.0
    let frac = rng.gen_range(UBig::ZERO..one.clone());
    DBig::from_parts(IBig::from(one + frac), -(precision as isize))
}

macro_rules! add_unary_hyper_bench {
    ($name:ident, $method:ident, $gen:ident, $base:ty, $max_log:literal) => {
        fn $name(criterion: &mut Criterion) {
            let mut rng = StdRng::seed_from_u64(SEED);
            let mut group = criterion.benchmark_group(stringify!($name));
            group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));
            for log_prec in 1..=$max_log {
                let precision = 10usize.pow(log_prec);
                let a: $base = $gen(precision, &mut rng);
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

add_unary_hyper_bench!(fbig_sinh, sinh, gen_fbig, FBin, 4);
add_unary_hyper_bench!(fbig_cosh, cosh, gen_fbig, FBin, 4);
add_unary_hyper_bench!(fbig_tanh, tanh, gen_fbig, FBin, 4);
add_unary_hyper_bench!(fbig_asinh, asinh, gen_fbig, FBin, 4);
add_unary_hyper_bench!(fbig_acosh, acosh, gen_fbig_above_one, FBin, 4);
add_unary_hyper_bench!(fbig_atanh, atanh, gen_fbig, FBin, 4);
add_unary_hyper_bench!(dbig_sinh, sinh, gen_dbig, DBig, 3);
add_unary_hyper_bench!(dbig_cosh, cosh, gen_dbig, DBig, 3);
add_unary_hyper_bench!(dbig_tanh, tanh, gen_dbig, DBig, 3);
add_unary_hyper_bench!(dbig_asinh, asinh, gen_dbig, DBig, 3);
add_unary_hyper_bench!(dbig_acosh, acosh, gen_dbig_above_one, DBig, 3);
add_unary_hyper_bench!(dbig_atanh, atanh, gen_dbig, DBig, 3);

criterion_group!(
    benches, fbig_sinh, fbig_cosh, fbig_tanh, fbig_asinh, fbig_acosh, fbig_atanh, dbig_sinh,
    dbig_cosh, dbig_tanh, dbig_asinh, dbig_acosh, dbig_atanh,
);
criterion_main!(benches);
