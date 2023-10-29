//! Benchmarks.
//! Run: cargo bench -p dashu-float --bench primitive --features rand -- --quick

use std::ops::*;
use criterion::{
    black_box, criterion_group, criterion_main, AxisScale, BenchmarkId, Criterion,
    PlotConfiguration,
};
use dashu_base::Sign;
use dashu_ratio::RBig;
use dashu_int::{UBig, IBig};
use rand_v08::prelude::*;

const SEED: u64 = 1;

fn random_rbig<R>(bits: usize, rng: &mut R) -> RBig
where
    R: Rng + ?Sized,
{
    let sign = Sign::from(rng.gen_bool(0.5));
    let numerator = IBig::from_parts(sign, rng.gen_range(UBig::ONE << (bits - 1)..UBig::ONE << bits));
    let denominator = rng.gen_range(UBig::ONE << (bits - 1)..UBig::ONE << bits);
    RBig::from_parts(numerator, denominator)
}

macro_rules! add_binop_benchmark {
    ($name:ident, $method:ident, $max_log_bits:literal) => {
        fn $name(criterion: &mut Criterion) {
            let mut rng = StdRng::seed_from_u64(SEED);
            let mut group = criterion.benchmark_group(stringify!($name));
            group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));

            for log_bits in 1..=$max_log_bits {
                let bits = 10usize.pow(log_bits);
                let a = random_rbig(bits, &mut rng);
                let b = random_rbig(bits, &mut rng) + &a; // make b > a so that sub won't underflow
                group.bench_with_input(BenchmarkId::from_parameter(bits), &bits, |bencher, _| {
                    bencher.iter(|| black_box(&b).$method(black_box(&a)))
                });
            }

            group.finish();
        }
    };
}

add_binop_benchmark!(rbig_add, add, 6);
add_binop_benchmark!(rbig_sub, sub, 6);
add_binop_benchmark!(rbig_mul, mul, 6);
add_binop_benchmark!(rbig_div, div, 6);

criterion_group!(
    benches,
    rbig_add,
    rbig_sub,
    rbig_mul,
    rbig_div,
);

criterion_main!(benches);
