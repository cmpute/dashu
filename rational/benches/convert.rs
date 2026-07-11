//! Benchmark: cross-crate conversion RBig -> FBig.
//! Run: cargo bench -p dashu-ratio --bench convert --features dashu-float,rand -- --quick

use criterion::{
    criterion_group, criterion_main, AxisScale, BenchmarkId, Criterion, PlotConfiguration,
};
use dashu_base::Sign;
use dashu_float::round::mode::HalfAway;
use dashu_int::{IBig, UBig};
use dashu_ratio::RBig;
use rand_v08::prelude::*;

const SEED: u64 = 1;

fn random_rbig<R>(bits: usize, rng: &mut R) -> RBig
where
    R: Rng + ?Sized,
{
    let sign = Sign::from(rng.gen_bool(0.5));
    let numerator =
        IBig::from_parts(sign, rng.gen_range(UBig::ONE << (bits - 1)..UBig::ONE << bits));
    let denominator = rng.gen_range(UBig::ONE << (bits - 1)..UBig::ONE << bits);
    RBig::from_parts(numerator, denominator)
}

fn rbig_to_float(criterion: &mut Criterion) {
    let mut rng = StdRng::seed_from_u64(SEED);
    let mut group = criterion.benchmark_group("rbig_to_float");
    group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));

    for log_bits in 1..=6 {
        let bits = 10usize.pow(log_bits);
        let r = random_rbig(bits, &mut rng);
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("1e{}", log_bits)),
            &r,
            |bencher, tr| bencher.iter(|| tr.to_float::<HalfAway, 2>(bits)),
        );
    }

    group.finish();
}

criterion_group!(benches, rbig_to_float);
criterion_main!(benches);
