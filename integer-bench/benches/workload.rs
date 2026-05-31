//! Scenario benchmarks for a property-based-testing-style workload.
//!
//! These benchmarks are derived from profiling specific hot paths that show
//! up when profiling the test suite of
//! [hegel-rust](https://github.com/hegeldev/hegel-rust), but should mostly 
//! just be taken as interesting examples of realistic workloads. These ones
//! are particularly focused on hot paths that occur during generation.
//!
//! Each bench body is generic over [`Backend`] and run for every backend, so
//! dashu and the comparison libraries land in one criterion report under the
//! backend name. The pure-Rust backends (dashu, ibig, num-bigint, malachite)
//! are always built; rug is added with `--features gmp` (needs the GMP
//! toolchain).
//!
//! Run:
//!   cargo bench --manifest-path integer-bench/Cargo.toml --bench workload -- --quick
//! Include the rug backend too (needs the GMP toolchain): add `--features gmp`.

use criterion::measurement::WallTime;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkGroup, Criterion};
use integer_bench::{
    mixed_class, seeded_rng, Backend, BenchInt, Dashu, Ibig, Malachite, Num, SignedInt, ValueClass,
};
use rand_v08::Rng;

#[cfg(feature = "gmp")]
use integer_bench::Rug;

const N: usize = 4096;

/// Define a criterion entry point `$name` that opens group `$group` and runs
/// the generic body `$body` for every backend (dashu, ibig, num and malachite
/// always; rug when the `gmp` feature is on).
macro_rules! per_backend {
    ($name:ident, $group:literal, $body:ident) => {
        fn $name(c: &mut Criterion) {
            let mut group = c.benchmark_group($group);
            $body::<Dashu>(&mut group);
            $body::<Ibig>(&mut group);
            $body::<Num>(&mut group);
            $body::<Malachite>(&mut group);
            #[cfg(feature = "gmp")]
            $body::<Rug>(&mut group);
            group.finish();
        }
    };
}

fn build_mixed_inputs<B: Backend>() -> Vec<B::Signed> {
    let mut rng = seeded_rng();
    (0..N)
        .map(|_| B::sample_signed(mixed_class(&mut rng), &mut rng))
        .collect()
}

/// Small-only input distribution (every value ≤ 128 bits, on the inline /
/// fast path for most libraries). Roughly 1/3 TwoWord, 2/3 OneWord — a
/// small-but-not-trivial mix.
fn build_small_inputs<B: Backend>() -> Vec<B::Signed> {
    let mut rng = seeded_rng();
    (0..N)
        .map(|_| {
            let class = if rng.gen::<u32>() % 3 == 0 {
                ValueClass::TwoWord
            } else {
                ValueClass::OneWord
            };
            B::sample_signed(class, &mut rng)
        })
        .collect()
}

/// Sub-1-kbit mixed-class distribution: same shape as `mixed_class` (small
/// values dominate) but bounded to ≤ 256 bits, so no operand ever pushes the
/// accumulator past the 1-kbit regime. The 2 % `Mid` (1024-bit, right at the
/// boundary) and 1 % `Large` (100 kbit) slots of `mixed_class` get
/// redistributed to `JustOverInline` (192-bit) and the inline classes — that
/// keeps the mixed-scale flavour without inviting GMP's asymptotic kernels
/// into the bench.
fn under_1kbit_class<R: Rng>(rng: &mut R) -> ValueClass {
    let r: u32 = rng.gen_range(0..100);
    match r {
        0..=4 => ValueClass::Zero,       // 5 %
        5..=64 => ValueClass::OneWord,   // 60 %
        65..=89 => ValueClass::TwoWord,  // 25 %
        _ => ValueClass::JustOverInline, // 10 % (was 7 % + redirected Mid/Large)
    }
}

fn build_under_1kbit_inputs<B: Backend>() -> Vec<B::Signed> {
    let mut rng = seeded_rng();
    (0..N)
        .map(|_| B::sample_signed(under_1kbit_class(&mut rng), &mut rng))
        .collect()
}

/// Scenario 1: running-sum-and-compare.
///
/// Mirrors a targeting/score loop: every step adds the next value into an
/// accumulator and checks whether it crossed a bound. Most inputs fit in i64
/// (per `mixed_class`); the accumulator may grow.
fn running_sum_and_compare<B: Backend>(group: &mut BenchmarkGroup<WallTime>) {
    let inputs = build_mixed_inputs::<B>();
    let bound = B::Signed::from_i64(1).shl_ref(200);
    group.bench_function(B::NAME, |b| {
        b.iter(|| {
            let mut sum = B::Signed::from_i64(0);
            let mut hits = 0u32;
            for v in &inputs {
                sum.add_assign_ref(black_box(v));
                if black_box(&sum) >= black_box(&bound) {
                    hits += 1;
                    sum = B::Signed::from_i64(0);
                }
            }
            (sum, hits)
        })
    });
}

/// Scenario 2: string round-trip.
///
/// When bigints cross a serialization boundary as decimal strings, every value
/// is formatted to and parsed from decimal. This bench measures the
/// steady-state cost of that path on mostly-small values.
fn string_round_trip<B: Backend>(group: &mut BenchmarkGroup<WallTime>) {
    let inputs = build_mixed_inputs::<B>();
    group.bench_function(B::NAME, |b| {
        b.iter(|| {
            let mut last = B::Signed::from_i64(0);
            for v in &inputs {
                let s = black_box(v).to_string();
                last = B::Signed::parse(&s);
            }
            last
        })
    });
}

/// Sub-1-kbit string round-trip. Same shape as the mixed-input version
/// but every value ≤ 256 bits, so no input pulls the bench into GMP's
/// asymptotic-base-conversion regime.
fn string_round_trip_under_1kbit<B: Backend>(group: &mut BenchmarkGroup<WallTime>) {
    let inputs = build_under_1kbit_inputs::<B>();
    group.bench_function(B::NAME, |b| {
        b.iter(|| {
            let mut last = B::Signed::from_i64(0);
            for v in &inputs {
                let s = black_box(v).to_string();
                last = B::Signed::parse(&s);
            }
            last
        })
    });
}

/// Scenario 3: bounded arithmetic mix.
///
/// A scripted sequence of `+`, `-`, `*`, `<<`, `&` over a small working set.
/// Simulates one step of stateful test execution where most intermediate
/// values stay inline. The exact op sequence is fixed so successive runs
/// are comparable.
fn bounded_arithmetic_mix<B: Backend>(group: &mut BenchmarkGroup<WallTime>) {
    let inputs = build_mixed_inputs::<B>();
    group.bench_function(B::NAME, |b| {
        b.iter(|| {
            // Four live registers, refreshed periodically from `inputs`.
            let mut r0 = B::Signed::from_i64(0);
            let mut r1 = B::Signed::from_i64(1);
            let mut r2 = B::Signed::from_i64(-1);
            let mut r3 = B::Signed::from_i64(2);
            for (i, v) in inputs.iter().enumerate() {
                match i & 7 {
                    0 => r0 = r0.add_ref(black_box(v)),
                    1 => r1 = r1.sub_ref(black_box(v)),
                    2 => r2 = r2.mul_ref(black_box(v)),
                    3 => r3 = r3.add_ref(&r0),
                    4 => r0 = r0.bitxor_ref(&r1),
                    5 => r1 = r2.bitand_ref(black_box(v)),
                    6 => r2 = r3.shl_ref(1),
                    _ => r3 = r0.add_ref(&r2),
                }
            }
            (r0, r1, r2, r3)
        })
    });
}

/// Scenario 1-small: running-sum-and-compare over ≤ 128-bit inputs.
///
/// Every RHS is small but the accumulator can grow heap-resident, so the
/// dominant cost is the per-step reduce/allocate finalisation on the
/// AddAssign path — a headline detector for in-place AddAssign work.
fn running_sum_and_compare_small<B: Backend>(group: &mut BenchmarkGroup<WallTime>) {
    let inputs = build_small_inputs::<B>();
    let bound = B::Signed::from_i64(1).shl_ref(200);
    group.bench_function(B::NAME, |b| {
        b.iter(|| {
            let mut sum = B::Signed::from_i64(0);
            let mut hits = 0u32;
            for v in &inputs {
                sum.add_assign_ref(black_box(v));
                if black_box(&sum) >= black_box(&bound) {
                    hits += 1;
                    sum = B::Signed::from_i64(0);
                }
            }
            (sum, hits)
        })
    });
}

/// Scenario 1-under_1kbit: running-sum-and-compare strictly bounded to
/// values ≤ 256 bits. The `_small` variant covers the all-inline case; this
/// one covers the more interesting "mostly inline, occasionally just-over-
/// inline heap" regime that the user's < 1-kbit performance target is
/// directly about.
fn running_sum_and_compare_under_1kbit<B: Backend>(group: &mut BenchmarkGroup<WallTime>) {
    let inputs = build_under_1kbit_inputs::<B>();
    let bound = B::Signed::from_i64(1).shl_ref(200);
    group.bench_function(B::NAME, |b| {
        b.iter(|| {
            let mut sum = B::Signed::from_i64(0);
            let mut hits = 0u32;
            for v in &inputs {
                sum.add_assign_ref(black_box(v));
                if black_box(&sum) >= black_box(&bound) {
                    hits += 1;
                    sum = B::Signed::from_i64(0);
                }
            }
            (sum, hits)
        })
    });
}

/// Scripted arithmetic mix where every register stays bounded.
///
/// The earlier `bounded_arithmetic_mix*` benches grow `r2` unboundedly via
/// `r2 = &r2 * v` (and `r3` via `r3 << 1`), so by the end of a single
/// `b.iter` invocation `r2` is ~32 kbit — well outside the user's < 1 kbit
/// target. Here every reassignment writes a result whose magnitude is
/// bounded by `O(input_size)`: multiplication is between two fresh inputs
/// (≤ 512 bits), shifts and bitwise ops can only grow by one bit per op,
/// and the chained sums/diffs use freshly-drawn inputs as one operand. All
/// four registers therefore stay under 1 kbit for the entire loop.
fn bounded_arithmetic_mix_under_1kbit<B: Backend>(group: &mut BenchmarkGroup<WallTime>) {
    let inputs = build_under_1kbit_inputs::<B>();
    group.bench_function(B::NAME, |b| {
        b.iter(|| {
            let mut r0 = inputs[0].clone();
            let mut r1 = inputs[1].clone();
            let mut r2 = inputs[2].clone();
            let mut r3 = inputs[3].clone();
            for (i, v) in inputs.iter().enumerate() {
                let w = &inputs[i.wrapping_add(7) & (N - 1)];
                match i & 7 {
                    0 => r0 = r1.sub_ref(black_box(v)),
                    1 => r1 = r0.bitxor_ref(&r2),
                    2 => r2 = black_box(v).sub_ref(&r3),
                    3 => r3 = r0.bitand_ref(black_box(v)),
                    4 => r0 = r2.add_ref(black_box(v)),
                    5 => r1 = r3.shl_ref(1),
                    6 => r2 = black_box(v).mul_ref(w),
                    _ => r3 = r1.sub_ref(&r0),
                }
            }
            (r0, r1, r2, r3)
        })
    });
}

/// Same shape as `bounded_arithmetic_mix_under_1kbit`, but inputs strictly
/// inline (≤ 128 bits). Every register stays bounded for the same reasons.
fn bounded_arithmetic_mix_small<B: Backend>(group: &mut BenchmarkGroup<WallTime>) {
    let inputs = build_small_inputs::<B>();
    group.bench_function(B::NAME, |b| {
        b.iter(|| {
            let mut r0 = inputs[0].clone();
            let mut r1 = inputs[1].clone();
            let mut r2 = inputs[2].clone();
            let mut r3 = inputs[3].clone();
            for (i, v) in inputs.iter().enumerate() {
                let w = &inputs[i.wrapping_add(7) & (N - 1)];
                match i & 7 {
                    0 => r0 = r1.sub_ref(black_box(v)),
                    1 => r1 = r0.bitxor_ref(&r2),
                    2 => r2 = black_box(v).sub_ref(&r3),
                    3 => r3 = r0.bitand_ref(black_box(v)),
                    4 => r0 = r2.add_ref(black_box(v)),
                    5 => r1 = r3.shl_ref(1),
                    6 => r2 = black_box(v).mul_ref(w),
                    _ => r3 = r1.sub_ref(&r0),
                }
            }
            (r0, r1, r2, r3)
        })
    });
}

// TODO: a fourth scenario derived from a real `generic-ints` trace once the
// repo is available locally.

per_backend!(
    bench_running_sum_and_compare,
    "running_sum_and_compare",
    running_sum_and_compare
);
per_backend!(
    bench_running_sum_and_compare_small,
    "running_sum_and_compare_small",
    running_sum_and_compare_small
);
per_backend!(
    bench_running_sum_and_compare_under_1kbit,
    "running_sum_and_compare_under_1kbit",
    running_sum_and_compare_under_1kbit
);
per_backend!(bench_string_round_trip, "string_round_trip", string_round_trip);
per_backend!(
    bench_string_round_trip_under_1kbit,
    "string_round_trip_under_1kbit",
    string_round_trip_under_1kbit
);
per_backend!(bench_bounded_arithmetic_mix, "bounded_arithmetic_mix", bounded_arithmetic_mix);
per_backend!(
    bench_bounded_arithmetic_mix_small,
    "bounded_arithmetic_mix_small",
    bounded_arithmetic_mix_small
);
per_backend!(
    bench_bounded_arithmetic_mix_under_1kbit,
    "bounded_arithmetic_mix_under_1kbit",
    bounded_arithmetic_mix_under_1kbit
);

criterion_group!(
    benches,
    bench_running_sum_and_compare,
    bench_running_sum_and_compare_small,
    bench_running_sum_and_compare_under_1kbit,
    bench_string_round_trip,
    bench_string_round_trip_under_1kbit,
    bench_bounded_arithmetic_mix,
    bench_bounded_arithmetic_mix_small,
    bench_bounded_arithmetic_mix_under_1kbit,
);

criterion_main!(benches);
