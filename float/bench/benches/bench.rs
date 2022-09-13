//! Benchmarks.
//!
//! Note: these don't work on 16-bit machines.

use criterion::{criterion_group, criterion_main, Criterion};
use dashu_float::DBig;
use twofloat::TwoFloat;
use bigdecimal::BigDecimal;
use num_bigfloat::BigFloat;
use rug::Float;

type FBig = dashu_float::FBig;

fn bench_dashu(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("dashu");

    group.bench_function("exp (binary, 100 bits)", |b| {
        b.iter(|| (1..=12)
        .map(|n| FBig::from(n).with_precision(100).value().exp())
        .fold(FBig::ZERO, |acc, x| acc + x))
    });
    
    group.bench_function("exp (binary, 1000 bits)", |b| {
        b.iter(|| (1..=12)
        .map(|n| FBig::from(n).with_precision(1000).value().exp())
        .fold(FBig::ZERO, |acc, x| acc + x))
    });

    group.bench_function("exp (decimal, 40 digits)", |b| {
        b.iter(|| (1..=12)
        .map(|n| DBig::from(n).with_precision(40).value().exp())
        .fold(DBig::ZERO, |acc, x| acc + x))
    });
    
    println!("{}", DBig::from(12).with_precision(100).value().exp());

    group.bench_function("exp (decimal, 100 digits)", |b| {
        b.iter(|| (1..=12)
        .map(|n| DBig::from(n).with_precision(100).value().exp())
        .fold(DBig::ZERO, |acc, x| acc + x))
    });
    group.finish();
}

fn bench_num_bigfloat(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("num-bigfloat");

    println!("{}", BigFloat::from(12).exp());

    group.bench_function("exp (decimal, 40 digits)", |b| {
        b.iter(|| (1..=12)
        .map(|n| BigFloat::from(n).exp())
        .fold(BigFloat::from(0), |acc, x| acc + x))
    });
    group.finish();
}

fn bench_twofloat(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("twofloat");

    println!("{}", TwoFloat::from(12).exp());

    group.bench_function("exp (binary, 106 bits)", |b| {
        b.iter(|| (1..=12)
        .map(|n| TwoFloat::from(n).exp())
        .fold(TwoFloat::from(0), |acc, x| acc + x))
    });
    group.finish();
}

fn bench_bigdecimal(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("bigdecimal");

    println!("{}", BigDecimal::from(12).exp());

    group.bench_function("exp (decimal, 100 digits)", |b| {
        b.iter(|| (1..=12)
        .map(|n| BigDecimal::from(n).exp())
        .fold(BigDecimal::from(0), |acc, x| acc + x))
    });
    group.finish();
}

fn bench_rug(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("rug");

    println!("{}", Float::with_val(100, 12).exp());

    group.bench_function("exp (binary, 100 digits)", |b| {
        b.iter(|| (1..=12)
        .map(|n| Float::with_val(100, n).exp())
        .fold(Float::new(100), |acc, x| acc + x))
    });
    
    group.bench_function("exp (binary, 1000 digits)", |b| {
        b.iter(|| (1..=12)
        .map(|n| Float::with_val(1000, n).exp())
        .fold(Float::new(1000), |acc, x| acc + x))
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_dashu,
    bench_bigdecimal,
    bench_num_bigfloat,
    bench_twofloat,
    bench_rug,
);
criterion_main!(benches);
