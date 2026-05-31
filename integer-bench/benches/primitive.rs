//! Bit-width-sweep arithmetic benchmarks (the dashu `primitive.rs` suite,
//! generalised to compare against other libraries).
//!
//! Run:
//!   cargo bench --manifest-path integer-bench/Cargo.toml --bench primitive -- --quick
//! Include the rug backend too (needs the GMP toolchain): add `--features gmp`.
//!
//! Note: these don't work on 16-bit machines.
//!
//! Each bench body is generic over [`PrimitiveBackend`] and run for every
//! backend, so dashu and the comparison libraries land in one criterion report
//! under the backend name. The pure-Rust backends (dashu, ibig, num-bigint,
//! malachite) are always built; rug is added with `--features gmp`.
//!
//! The `ubig_modulo_*` benches measure each library's plain multiply-then-reduce
//! and native modpow (nothing precomputed), so they're a like-for-like
//! comparison.

use criterion::measurement::WallTime;
use criterion::{
    criterion_group, criterion_main, AxisScale, BenchmarkGroup, BenchmarkId, Criterion,
    PlotConfiguration,
};
use integer_bench::{
    BenchInt, Dashu, Ibig, Malachite, Num, PrimitiveBackend, PrimitiveInt, UnsignedInt,
};
use rand_v08::prelude::*;
use std::fmt::Write;

#[cfg(feature = "gmp")]
use integer_bench::Rug;

const SEED: u64 = 1;

/// Define a criterion entry point `$name` that opens group `$group` and runs
/// the generic body `$body` for every backend (dashu, ibig, num and malachite
/// always; rug when the `gmp` feature is on).
macro_rules! per_backend {
    ($name:ident, $group:literal, $body:ident) => {
        fn $name(c: &mut Criterion) {
            let mut group = c.benchmark_group($group);
            group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));
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

/// Arithmetic binop bench bodies sharing the "b is built > a" setup, so that
/// subtraction never underflows.
macro_rules! binop_body {
    ($body:ident, $method:ident, $max_log_bits:literal) => {
        fn $body<B: PrimitiveBackend>(group: &mut BenchmarkGroup<WallTime>)
        where
            B::Unsigned: PrimitiveInt,
        {
            let mut rng = StdRng::seed_from_u64(SEED);
            for log_bits in 1..=$max_log_bits {
                let bits = 10usize.pow(log_bits);
                let a = B::sample_unsigned_bits(bits, &mut rng);
                let b = B::sample_unsigned_bits(bits, &mut rng).add_ref(&a); // b > a
                group.bench_with_input(
                    BenchmarkId::new(B::NAME, bits),
                    &(a, b),
                    |bencher, (ta, tb)| bencher.iter(|| tb.$method(ta)),
                );
            }
        }
    };
}

binop_body!(add_body, add_ref, 4);
binop_body!(sub_body, sub_ref, 4);
binop_body!(mul_body, mul_ref, 4);
binop_body!(div_body, div_ref, 4);
binop_body!(gcd_body, gcd, 4);

fn gcd_ext_body<B: PrimitiveBackend>(group: &mut BenchmarkGroup<WallTime>)
where
    B::Unsigned: PrimitiveInt,
{
    let mut rng = StdRng::seed_from_u64(SEED);
    for log_bits in 1..=4 {
        let bits = 10usize.pow(log_bits);
        let a = B::sample_unsigned_bits(bits, &mut rng);
        let b = B::sample_unsigned_bits(bits, &mut rng).add_ref(&a);
        group.bench_with_input(BenchmarkId::new(B::NAME, bits), &(a, b), |bencher, (ta, tb)| {
            bencher.iter(|| tb.gcd_ext_blackbox(ta))
        });
    }
}

per_backend!(ubig_add, "ubig_add", add_body);
per_backend!(ubig_sub, "ubig_sub", sub_body);
per_backend!(ubig_mul, "ubig_mul", mul_body);
per_backend!(ubig_div, "ubig_div", div_body);
per_backend!(ubig_gcd, "ubig_gcd", gcd_body);
per_backend!(ubig_gcd_ext, "ubig_gcd_ext", gcd_ext_body);

fn to_hex_body<B: PrimitiveBackend>(group: &mut BenchmarkGroup<WallTime>)
where
    B::Unsigned: PrimitiveInt,
{
    let mut rng = StdRng::seed_from_u64(SEED);
    for log_bits in 1..=4 {
        let bits = 10usize.pow(log_bits);
        let a = B::sample_unsigned_bits(bits, &mut rng);
        let mut out = String::with_capacity(bits / 4 + 1);
        group.bench_with_input(BenchmarkId::new(B::NAME, bits), &a, |bencher, ta| {
            bencher.iter(|| {
                out.clear();
                ta.write_hex(&mut out);
                out.len()
            })
        });
    }
}

fn to_dec_body<B: PrimitiveBackend>(group: &mut BenchmarkGroup<WallTime>)
where
    B::Unsigned: PrimitiveInt,
{
    let mut rng = StdRng::seed_from_u64(SEED);
    for log_bits in 1..=4 {
        let bits = 10usize.pow(log_bits);
        let a = B::sample_unsigned_bits(bits, &mut rng);
        let mut out = String::with_capacity(bits / 3 + 1);
        group.bench_with_input(BenchmarkId::new(B::NAME, bits), &a, |bencher, ta| {
            bencher.iter(|| {
                out.clear();
                write!(&mut out, "{}", ta).unwrap();
                out.len()
            })
        });
    }
}

fn from_hex_body<B: PrimitiveBackend>(group: &mut BenchmarkGroup<WallTime>)
where
    B::Unsigned: PrimitiveInt,
{
    let mut rng = StdRng::seed_from_u64(SEED);
    for log_bits in 1..=4 {
        let bits = 10usize.pow(log_bits);
        let a = B::sample_unsigned_bits(bits, &mut rng);
        let s = a.to_radix_string(16);
        group.bench_with_input(BenchmarkId::new(B::NAME, bits), &s, |bencher, ts| {
            bencher.iter(|| B::Unsigned::from_radix(ts, 16))
        });
    }
}

fn from_dec_body<B: PrimitiveBackend>(group: &mut BenchmarkGroup<WallTime>)
where
    B::Unsigned: PrimitiveInt,
{
    let mut rng = StdRng::seed_from_u64(SEED);
    for log_bits in 1..=4 {
        let bits = 10usize.pow(log_bits);
        let a = B::sample_unsigned_bits(bits, &mut rng);
        let s = a.to_radix_string(10);
        group.bench_with_input(BenchmarkId::new(B::NAME, bits), &s, |bencher, ts| {
            bencher.iter(|| B::Unsigned::from_radix(ts, 10))
        });
    }
}

fn pow_body<B: PrimitiveBackend>(group: &mut BenchmarkGroup<WallTime>)
where
    B::Unsigned: PrimitiveInt,
{
    for log_power in 1..=4 {
        let p = 10usize.pow(log_power);
        group.bench_with_input(BenchmarkId::new(B::NAME, p), &p, |bencher, p| {
            bencher.iter(|| B::Unsigned::from_u64(3).pow_exp(*p))
        });
    }
}

per_backend!(ubig_to_hex, "ubig_to_hex", to_hex_body);
per_backend!(ubig_to_dec, "ubig_to_dec", to_dec_body);
per_backend!(ubig_from_hex, "ubig_from_hex", from_hex_body);
per_backend!(ubig_from_dec, "ubig_from_dec", from_dec_body);
per_backend!(ubig_pow, "ubig_pow", pow_body);

fn modulo_mul_body<B: PrimitiveBackend>(group: &mut BenchmarkGroup<WallTime>)
where
    B::Unsigned: PrimitiveInt,
{
    let mut rng = StdRng::seed_from_u64(SEED);
    for log_bits in 1..=4 {
        let bits = 10usize.pow(log_bits);
        let m = B::sample_unsigned_bits(bits, &mut rng);
        let a = B::sample_unsigned_bits(bits, &mut rng);
        let b = B::sample_unsigned_bits(bits, &mut rng);
        group.bench_with_input(
            BenchmarkId::new(B::NAME, bits),
            &(a, b, m),
            |bencher, (a, b, m)| bencher.iter(|| B::mod_mul(a, b, m)),
        );
    }
}

fn modulo_pow_body<B: PrimitiveBackend>(group: &mut BenchmarkGroup<WallTime>)
where
    B::Unsigned: PrimitiveInt,
{
    let mut rng = StdRng::seed_from_u64(SEED);
    for log_bits in 1..=4 {
        let bits = 10usize.pow(log_bits);
        let m = B::sample_unsigned_bits(bits, &mut rng);
        let a = B::sample_unsigned_bits(2048, &mut rng);
        let b = B::sample_unsigned_bits(bits, &mut rng); // exponent
        group.bench_with_input(
            BenchmarkId::new(B::NAME, bits),
            &(a, b, m),
            |bencher, (a, b, m)| bencher.iter(|| B::mod_pow(a, b, m)),
        );
    }
}

per_backend!(ubig_modulo_mul, "ubig_modulo_mul", modulo_mul_body);
per_backend!(ubig_modulo_pow, "ubig_modulo_pow", modulo_pow_body);

criterion_group!(
    benches,
    ubig_add,
    ubig_sub,
    ubig_mul,
    ubig_div,
    ubig_gcd,
    ubig_gcd_ext,
    ubig_to_hex,
    ubig_to_dec,
    ubig_from_hex,
    ubig_from_dec,
    ubig_pow,
    ubig_modulo_mul,
    ubig_modulo_pow,
);

criterion_main!(benches);
