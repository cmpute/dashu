//! Benchmarks.

use criterion::{criterion_group, criterion_main, Criterion};

fn bench_dashu(criterion: &mut Criterion) {
    use dashu_ratio::RBig;
    let mut group = criterion.benchmark_group("dashu");

    fn fib(n: u32) -> RBig {
        let mut a = RBig::ONE;
        let mut b = RBig::ONE;
        for _ in 0..n {
            let next = a + RBig::ONE / &b;
            a = b;
            b = next;
        }
        b
    }

    group.bench_function("fib 2000", |b| {
        b.iter(|| fib(2000))
    });
}

fn bench_num_rational(criterion: &mut Criterion) {
    use num_rational::BigRational;
    let mut group = criterion.benchmark_group("num-rational");

    fn fib(n: u32) -> BigRational {
        let one = BigRational::from_integer(1.into());
        let mut a = one.clone();
        let mut b = one.clone();
        for _ in 0..n {
            let next = a + &one / &b;
            a = b;
            b = next;
        }
        b
    }

    group.bench_function("fib 2000", |b| {
        b.iter(|| fib(2000))
    });
}

fn bench_rug(criterion: &mut Criterion) {
    use rug::{Rational, Complete};
    let mut group = criterion.benchmark_group("rug");

    fn fib(n: u32) -> Rational {
        let mut a = Rational::from(1);
        let mut b = Rational::from(1);
        let one = Rational::from(1);
        for _ in 0..n {
            let next = a + (&one / &b).complete();
            a = b;
            b = next;
        }
        b
    }

    group.bench_function("fib 2000", |b| {
        b.iter(|| fib(2000))
    });
}

fn bench_malachite(criterion: &mut Criterion) {
    use malachite_q::Rational;
    let mut group = criterion.benchmark_group("malachite");

    fn fib(n: u32) -> Rational {
        let mut a = Rational::from(1);
        let mut b = Rational::from(1);
        let one = Rational::from(1);
        for _ in 0..n {
            let next = a + &one / &b;
            a = b;
            b = next;
        }
        b
    }

    group.bench_function("fib 2000", |b| {
        b.iter(|| fib(2000))
    });
}

criterion_group!(
    benches,
    bench_dashu,
    bench_num_rational,
    bench_malachite,
    bench_rug,
);
criterion_main!(benches);
