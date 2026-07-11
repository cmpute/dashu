//! Benchmarks for shift operations (Shl/Shr) on UBig and IBig.
//! Run: cargo bench -p dashu-int --bench shift --features rand -- --quick
//!
//! Shifts by ~half the operand width, which exercises both the word-granular
//! fast path (large shifts) and the within-word bit path.

use criterion::{
    criterion_group, criterion_main, AxisScale, BenchmarkId, Criterion, PlotConfiguration,
};
use dashu_base::Sign;
use dashu_int::{IBig, UBig};
use rand_v08::prelude::*;

const SEED: u64 = 1;

fn random_ubig<R: Rng + ?Sized>(bits: usize, rng: &mut R) -> UBig {
    rng.gen_range(UBig::ONE << (bits - 1)..UBig::ONE << bits)
}

fn random_ibig<R: Rng + ?Sized>(bits: usize, rng: &mut R) -> IBig {
    let sign = Sign::from(rng.gen_bool(0.5));
    IBig::from_parts(sign, random_ubig(bits, rng))
}

macro_rules! add_shift_benchmark {
    ($name:ident, $op:tt, $t:ty, $gen:expr) => {
        fn $name(criterion: &mut Criterion) {
            let mut rng = StdRng::seed_from_u64(SEED);
            let mut group = criterion.benchmark_group(stringify!($name));
            group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));
            for log_bits in 1..=6 {
                let bits = 10usize.pow(log_bits);
                let a: $t = $gen(bits, &mut rng);
                let sh = bits / 2;
                group.bench_with_input(
                    BenchmarkId::from_parameter(format!("1e{}", log_bits)),
                    &(a, sh),
                    |bencher, (ta, ts)| bencher.iter(|| ta $op *ts),
                );
            }
            group.finish();
        }
    };
}

add_shift_benchmark!(ubig_shl, <<, UBig, random_ubig);
add_shift_benchmark!(ubig_shr, >>, UBig, random_ubig);
add_shift_benchmark!(ibig_shl, <<, IBig, random_ibig);
add_shift_benchmark!(ibig_shr, >>, IBig, random_ibig);

criterion_group!(benches, ubig_shl, ubig_shr, ibig_shl, ibig_shr);
criterion_main!(benches);
