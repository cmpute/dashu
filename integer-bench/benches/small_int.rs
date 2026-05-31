//! Fast-path / small-value benchmarks.
//!
//! The `primitive` bit-width sweep runs over 10..=10^4 bits and so under-covers
//! the small-integer range (≤ 128 bits, where most libraries keep the value
//! inline/on the fast path). This file fills that gap: each group runs across
//! every `ValueClass`, including `Zero`, `OneWord`, and `TwoWord` which the
//! bit-width sweep never reaches.
//!
//! Each bench body is generic over [`Backend`] and run for every backend, so
//! dashu and the comparison libraries land in one criterion report under the
//! backend name. The pure-Rust backends (dashu, ibig, num-bigint, malachite)
//! are always built; rug is added with `--features gmp` (needs the GMP
//! toolchain).
//!
//! Run:
//!   cargo bench --manifest-path integer-bench/Cargo.toml --bench small_int -- --quick
//! Include the rug backend too (needs the GMP toolchain): add `--features gmp`.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use criterion::measurement::WallTime;
use criterion::{
    black_box, criterion_group, criterion_main, BenchmarkGroup, BenchmarkId, Criterion,
};
use integer_bench::{
    seeded_rng, Backend, BenchInt, Dashu, Ibig, Malachite, Num, SignedInt, UnsignedInt, ValueClass,
};

#[cfg(feature = "gmp")]
use integer_bench::Rug;

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

// ---- construction from primitives ----

fn from_i64<B: Backend>(group: &mut BenchmarkGroup<WallTime>) {
    let mut rng = seeded_rng();
    let inputs: Vec<i64> = (0..256).map(|_| rand_v08::Rng::gen(&mut rng)).collect();
    group.bench_function(B::NAME, |b| {
        let mut i = 0usize;
        b.iter(|| {
            let v = inputs[i & 255];
            i = i.wrapping_add(1);
            B::Signed::from_i64(black_box(v))
        })
    });
}

fn from_i128<B: Backend>(group: &mut BenchmarkGroup<WallTime>) {
    let mut rng = seeded_rng();
    let inputs: Vec<i128> = (0..256).map(|_| rand_v08::Rng::gen(&mut rng)).collect();
    group.bench_function(B::NAME, |b| {
        let mut i = 0usize;
        b.iter(|| {
            let v = inputs[i & 255];
            i = i.wrapping_add(1);
            B::Signed::from_i128(black_box(v))
        })
    });
}

fn from_u64<B: Backend>(group: &mut BenchmarkGroup<WallTime>) {
    let mut rng = seeded_rng();
    let inputs: Vec<u64> = (0..256).map(|_| rand_v08::Rng::gen(&mut rng)).collect();
    group.bench_function(B::NAME, |b| {
        let mut i = 0usize;
        b.iter(|| {
            let v = inputs[i & 255];
            i = i.wrapping_add(1);
            B::Unsigned::from_u64(black_box(v))
        })
    });
}

fn from_u128<B: Backend>(group: &mut BenchmarkGroup<WallTime>) {
    let mut rng = seeded_rng();
    let inputs: Vec<u128> = (0..256).map(|_| rand_v08::Rng::gen(&mut rng)).collect();
    group.bench_function(B::NAME, |b| {
        let mut i = 0usize;
        b.iter(|| {
            let v = inputs[i & 255];
            i = i.wrapping_add(1);
            B::Unsigned::from_u128(black_box(v))
        })
    });
}

per_backend!(bench_from_i64, "ibig_from_i64", from_i64);
per_backend!(bench_from_i128, "ibig_from_i128", from_i128);
per_backend!(bench_from_u64, "ubig_from_u64", from_u64);
per_backend!(bench_from_u128, "ubig_from_u128", from_u128);

// ---- TryInto primitives (round-trip cost) ----

fn try_into_i128<B: Backend>(group: &mut BenchmarkGroup<WallTime>) {
    let mut rng = seeded_rng();
    let inputs: Vec<B::Signed> = (0..256)
        .map(|_| B::sample_signed(ValueClass::TwoWord, &mut rng))
        .collect();
    group.bench_function(B::NAME, |b| {
        let mut i = 0usize;
        b.iter(|| {
            let v = &inputs[i & 255];
            i = i.wrapping_add(1);
            black_box(v).try_to_i128()
        })
    });
}

per_backend!(bench_try_into_i128, "ibig_try_into_i128", try_into_i128);

// ---- binops parameterised by class ----

fn ubig_add_by_class<B: Backend>(group: &mut BenchmarkGroup<WallTime>) {
    let mut rng = seeded_rng();
    for &class in ValueClass::ALL {
        let pairs: Vec<(B::Unsigned, B::Unsigned)> = (0..32)
            .map(|_| (B::sample_unsigned(class, &mut rng), B::sample_unsigned(class, &mut rng)))
            .collect();
        group.bench_with_input(BenchmarkId::new(B::NAME, class.label()), &pairs, |b, p| {
            let mut i = 0usize;
            b.iter(|| {
                let (a, c) = &p[i & 31];
                i = i.wrapping_add(1);
                black_box(a).add_ref(black_box(c))
            })
        });
    }
}

fn ubig_mul_by_class<B: Backend>(group: &mut BenchmarkGroup<WallTime>) {
    let mut rng = seeded_rng();
    for &class in ValueClass::ALL {
        let pairs: Vec<(B::Unsigned, B::Unsigned)> = (0..32)
            .map(|_| (B::sample_unsigned(class, &mut rng), B::sample_unsigned(class, &mut rng)))
            .collect();
        group.bench_with_input(BenchmarkId::new(B::NAME, class.label()), &pairs, |b, p| {
            let mut i = 0usize;
            b.iter(|| {
                let (a, c) = &p[i & 31];
                i = i.wrapping_add(1);
                black_box(a).mul_ref(black_box(c))
            })
        });
    }
}

fn ibig_add_by_class<B: Backend>(group: &mut BenchmarkGroup<WallTime>) {
    let mut rng = seeded_rng();
    for &class in ValueClass::ALL {
        let pairs: Vec<(B::Signed, B::Signed)> = (0..32)
            .map(|_| (B::sample_signed(class, &mut rng), B::sample_signed(class, &mut rng)))
            .collect();
        group.bench_with_input(BenchmarkId::new(B::NAME, class.label()), &pairs, |b, p| {
            let mut i = 0usize;
            b.iter(|| {
                let (a, c) = &p[i & 31];
                i = i.wrapping_add(1);
                black_box(a).add_ref(black_box(c))
            })
        });
    }
}

// Mixed-class: one operand drawn from a small class, the other from a larger
// one. Models the "running total += small constant" pattern that pure
// same-class benches miss.
fn ubig_add_mixed<B: Backend>(group: &mut BenchmarkGroup<WallTime>) {
    let mut rng = seeded_rng();
    for &bigger in &[
        ValueClass::JustOverInline,
        ValueClass::Mid,
        ValueClass::Large,
    ] {
        let pairs: Vec<(B::Unsigned, B::Unsigned)> = (0..32)
            .map(|_| {
                (
                    B::sample_unsigned(bigger, &mut rng),
                    B::sample_unsigned(ValueClass::OneWord, &mut rng),
                )
            })
            .collect();
        group.bench_with_input(BenchmarkId::new(B::NAME, bigger.label()), &pairs, |b, p| {
            let mut i = 0usize;
            b.iter(|| {
                let (a, c) = &p[i & 31];
                i = i.wrapping_add(1);
                black_box(a).add_ref(black_box(c))
            })
        });
    }
}

per_backend!(bench_ubig_add_by_class, "ubig_add_by_class", ubig_add_by_class);
per_backend!(bench_ubig_mul_by_class, "ubig_mul_by_class", ubig_mul_by_class);
per_backend!(bench_ibig_add_by_class, "ibig_add_by_class", ibig_add_by_class);
per_backend!(bench_ubig_add_mixed, "ubig_add_mixed", ubig_add_mixed);

// ---- assign-form binops ----
//
// The by-ref benches above exercise `Add` / `+`. The benches below exercise the
// in-place `+= &T` form, which is the entry point on the running-sum hot path
// and the natural place for an in-place specialisation.

fn ubig_add_assign_by_class<B: Backend>(group: &mut BenchmarkGroup<WallTime>) {
    let mut rng = seeded_rng();
    for &class in ValueClass::ALL {
        let starts: Vec<B::Unsigned> = (0..32)
            .map(|_| B::sample_unsigned(class, &mut rng))
            .collect();
        let rhs: Vec<B::Unsigned> = (0..32)
            .map(|_| B::sample_unsigned(class, &mut rng))
            .collect();
        group.bench_with_input(
            BenchmarkId::new(B::NAME, class.label()),
            &(starts, rhs),
            |b, (s, r)| {
                let mut i = 0usize;
                b.iter(|| {
                    let mut acc = s[i & 31].clone();
                    acc.add_assign_ref(black_box(&r[i & 31]));
                    i = i.wrapping_add(1);
                    acc
                })
            },
        );
    }
}

fn ibig_add_assign_by_class<B: Backend>(group: &mut BenchmarkGroup<WallTime>) {
    let mut rng = seeded_rng();
    for &class in ValueClass::ALL {
        let starts: Vec<B::Signed> = (0..32).map(|_| B::sample_signed(class, &mut rng)).collect();
        let rhs: Vec<B::Signed> = (0..32).map(|_| B::sample_signed(class, &mut rng)).collect();
        group.bench_with_input(
            BenchmarkId::new(B::NAME, class.label()),
            &(starts, rhs),
            |b, (s, r)| {
                let mut i = 0usize;
                b.iter(|| {
                    let mut acc = s[i & 31].clone();
                    acc.add_assign_ref(black_box(&r[i & 31]));
                    i = i.wrapping_add(1);
                    acc
                })
            },
        );
    }
}

fn ubig_sub_assign_by_class<B: Backend>(group: &mut BenchmarkGroup<WallTime>) {
    let mut rng = seeded_rng();
    // For the unsigned type, build acc = a + b and subtract b, so the result
    // is non-negative.
    for &class in ValueClass::ALL {
        let pairs: Vec<(B::Unsigned, B::Unsigned)> = (0..32)
            .map(|_| {
                let a = B::sample_unsigned(class, &mut rng);
                let b = B::sample_unsigned(class, &mut rng);
                (a.add_ref(&b), b)
            })
            .collect();
        group.bench_with_input(BenchmarkId::new(B::NAME, class.label()), &pairs, |b, p| {
            let mut i = 0usize;
            b.iter(|| {
                let (start, rhs) = &p[i & 31];
                let mut acc = start.clone();
                acc.sub_assign_ref(black_box(rhs));
                i = i.wrapping_add(1);
                acc
            })
        });
    }
}

fn ibig_sub_assign_by_class<B: Backend>(group: &mut BenchmarkGroup<WallTime>) {
    let mut rng = seeded_rng();
    for &class in ValueClass::ALL {
        let starts: Vec<B::Signed> = (0..32).map(|_| B::sample_signed(class, &mut rng)).collect();
        let rhs: Vec<B::Signed> = (0..32).map(|_| B::sample_signed(class, &mut rng)).collect();
        group.bench_with_input(
            BenchmarkId::new(B::NAME, class.label()),
            &(starts, rhs),
            |b, (s, r)| {
                let mut i = 0usize;
                b.iter(|| {
                    let mut acc = s[i & 31].clone();
                    acc.sub_assign_ref(black_box(&r[i & 31]));
                    i = i.wrapping_add(1);
                    acc
                })
            },
        );
    }
}

per_backend!(
    bench_ubig_add_assign_by_class,
    "ubig_add_assign_by_class",
    ubig_add_assign_by_class
);
per_backend!(
    bench_ibig_add_assign_by_class,
    "ibig_add_assign_by_class",
    ibig_add_assign_by_class
);
per_backend!(
    bench_ubig_sub_assign_by_class,
    "ubig_sub_assign_by_class",
    ubig_sub_assign_by_class
);
per_backend!(
    bench_ibig_sub_assign_by_class,
    "ibig_sub_assign_by_class",
    ibig_sub_assign_by_class
);

// Diagnostic for the "heap accumulator, small RHS" path: the accumulator is
// heap-resident every iteration and stays heap-resident (no shrink possible)
// — the case where per-step reduce/allocate finalisation is pure overhead. A
// successful in-place AddAssign specialisation should move this bench
// substantially while leaving the same-class benches above largely flat.
fn ubig_add_assign_heap_acc_small_rhs<B: Backend>(group: &mut BenchmarkGroup<WallTime>) {
    let mut rng = seeded_rng();
    for &acc_class in &[
        ValueClass::JustOverInline,
        ValueClass::Mid,
        ValueClass::Large,
    ] {
        let acc_starts: Vec<B::Unsigned> = (0..32)
            .map(|_| B::sample_unsigned(acc_class, &mut rng))
            .collect();
        let rhs: Vec<B::Unsigned> = (0..32)
            .map(|_| B::sample_unsigned(ValueClass::OneWord, &mut rng))
            .collect();
        group.bench_with_input(
            BenchmarkId::new(B::NAME, acc_class.label()),
            &(acc_starts, rhs),
            |b, (s, r)| {
                let mut i = 0usize;
                b.iter(|| {
                    let mut acc = s[i & 31].clone();
                    acc.add_assign_ref(black_box(&r[i & 31]));
                    i = i.wrapping_add(1);
                    acc
                })
            },
        );
    }
}

fn ibig_add_assign_heap_acc_small_rhs<B: Backend>(group: &mut BenchmarkGroup<WallTime>) {
    let mut rng = seeded_rng();
    for &acc_class in &[
        ValueClass::JustOverInline,
        ValueClass::Mid,
        ValueClass::Large,
    ] {
        let acc_starts: Vec<B::Signed> = (0..32)
            .map(|_| B::sample_signed(acc_class, &mut rng))
            .collect();
        let rhs: Vec<B::Signed> = (0..32)
            .map(|_| B::sample_signed(ValueClass::OneWord, &mut rng))
            .collect();
        group.bench_with_input(
            BenchmarkId::new(B::NAME, acc_class.label()),
            &(acc_starts, rhs),
            |b, (s, r)| {
                let mut i = 0usize;
                b.iter(|| {
                    let mut acc = s[i & 31].clone();
                    acc.add_assign_ref(black_box(&r[i & 31]));
                    i = i.wrapping_add(1);
                    acc
                })
            },
        );
    }
}

per_backend!(
    bench_ubig_add_assign_heap_acc_small_rhs,
    "ubig_add_assign_heap_acc_small_rhs",
    ubig_add_assign_heap_acc_small_rhs
);
per_backend!(
    bench_ibig_add_assign_heap_acc_small_rhs,
    "ibig_add_assign_heap_acc_small_rhs",
    ibig_add_assign_heap_acc_small_rhs
);

// Primitive-RHS AddAssign benches — the recommendation explicitly mentions
// `<i64>` / `<u64>` / `<i128>` / `<u128>` variants of the specialised path.
// Each starts from a heap-resident accumulator so the per-step finalisation
// cost is visible; lifting it via specialisation should be measurable here.

fn ibig_add_assign_i64_into_heap_acc<B: Backend>(group: &mut BenchmarkGroup<WallTime>) {
    let mut rng = seeded_rng();
    let acc_starts: Vec<B::Signed> = (0..32)
        .map(|_| B::sample_signed(ValueClass::Mid, &mut rng))
        .collect();
    let rhs: Vec<i64> = (0..32).map(|_| rand_v08::Rng::gen(&mut rng)).collect();
    group.bench_function(B::NAME, |b| {
        let mut i = 0usize;
        b.iter(|| {
            let mut acc = acc_starts[i & 31].clone();
            acc.add_assign_i64(black_box(rhs[i & 31]));
            i = i.wrapping_add(1);
            acc
        })
    });
}

fn ibig_add_assign_i128_into_heap_acc<B: Backend>(group: &mut BenchmarkGroup<WallTime>) {
    let mut rng = seeded_rng();
    let acc_starts: Vec<B::Signed> = (0..32)
        .map(|_| B::sample_signed(ValueClass::Mid, &mut rng))
        .collect();
    let rhs: Vec<i128> = (0..32).map(|_| rand_v08::Rng::gen(&mut rng)).collect();
    group.bench_function(B::NAME, |b| {
        let mut i = 0usize;
        b.iter(|| {
            let mut acc = acc_starts[i & 31].clone();
            acc.add_assign_i128(black_box(rhs[i & 31]));
            i = i.wrapping_add(1);
            acc
        })
    });
}

fn ubig_add_assign_u64_into_heap_acc<B: Backend>(group: &mut BenchmarkGroup<WallTime>) {
    let mut rng = seeded_rng();
    let acc_starts: Vec<B::Unsigned> = (0..32)
        .map(|_| B::sample_unsigned(ValueClass::Mid, &mut rng))
        .collect();
    let rhs: Vec<u64> = (0..32).map(|_| rand_v08::Rng::gen(&mut rng)).collect();
    group.bench_function(B::NAME, |b| {
        let mut i = 0usize;
        b.iter(|| {
            let mut acc = acc_starts[i & 31].clone();
            acc.add_assign_u64(black_box(rhs[i & 31]));
            i = i.wrapping_add(1);
            acc
        })
    });
}

fn ubig_add_assign_u128_into_heap_acc<B: Backend>(group: &mut BenchmarkGroup<WallTime>) {
    let mut rng = seeded_rng();
    let acc_starts: Vec<B::Unsigned> = (0..32)
        .map(|_| B::sample_unsigned(ValueClass::Mid, &mut rng))
        .collect();
    let rhs: Vec<u128> = (0..32).map(|_| rand_v08::Rng::gen(&mut rng)).collect();
    group.bench_function(B::NAME, |b| {
        let mut i = 0usize;
        b.iter(|| {
            let mut acc = acc_starts[i & 31].clone();
            acc.add_assign_u128(black_box(rhs[i & 31]));
            i = i.wrapping_add(1);
            acc
        })
    });
}

per_backend!(
    bench_ibig_add_assign_i64_into_heap_acc,
    "ibig_add_assign_i64_into_heap_acc",
    ibig_add_assign_i64_into_heap_acc
);
per_backend!(
    bench_ibig_add_assign_i128_into_heap_acc,
    "ibig_add_assign_i128_into_heap_acc",
    ibig_add_assign_i128_into_heap_acc
);
per_backend!(
    bench_ubig_add_assign_u64_into_heap_acc,
    "ubig_add_assign_u64_into_heap_acc",
    ubig_add_assign_u64_into_heap_acc
);
per_backend!(
    bench_ubig_add_assign_u128_into_heap_acc,
    "ubig_add_assign_u128_into_heap_acc",
    ubig_add_assign_u128_into_heap_acc
);

// One bitwise-assign bench to verify the same-shape claim ("Sub, BitAnd,
// BitOr, BitXor are all the same pattern") will land — full coverage of all
// three bitwise ops can pile on once the Add specialisation lands.
fn ubig_bitxor_assign_by_class<B: Backend>(group: &mut BenchmarkGroup<WallTime>) {
    let mut rng = seeded_rng();
    for &class in ValueClass::ALL {
        let starts: Vec<B::Unsigned> = (0..32)
            .map(|_| B::sample_unsigned(class, &mut rng))
            .collect();
        let rhs: Vec<B::Unsigned> = (0..32)
            .map(|_| B::sample_unsigned(class, &mut rng))
            .collect();
        group.bench_with_input(
            BenchmarkId::new(B::NAME, class.label()),
            &(starts, rhs),
            |b, (s, r)| {
                let mut i = 0usize;
                b.iter(|| {
                    let mut acc = s[i & 31].clone();
                    acc.bitxor_assign_ref(black_box(&r[i & 31]));
                    i = i.wrapping_add(1);
                    acc
                })
            },
        );
    }
}

per_backend!(
    bench_ubig_bitxor_assign_by_class,
    "ubig_bitxor_assign_by_class",
    ubig_bitxor_assign_by_class
);

// ---- comparison / hash / clone (cheap operations that dominate hot loops) ----

fn ubig_eq_same_class<B: Backend>(group: &mut BenchmarkGroup<WallTime>) {
    let mut rng = seeded_rng();
    for &class in ValueClass::ALL {
        // Half identical pairs, half non-equal pairs, so the benchmark sees
        // both branches of the eq fast path.
        let pairs: Vec<(B::Unsigned, B::Unsigned)> = (0..64)
            .map(|i| {
                let a = B::sample_unsigned(class, &mut rng);
                let b = if i % 2 == 0 {
                    a.clone()
                } else {
                    B::sample_unsigned(class, &mut rng)
                };
                (a, b)
            })
            .collect();
        group.bench_with_input(BenchmarkId::new(B::NAME, class.label()), &pairs, |b, p| {
            let mut i = 0usize;
            b.iter(|| {
                let (a, c) = &p[i & 63];
                i = i.wrapping_add(1);
                black_box(a) == black_box(c)
            })
        });
    }
}

fn ubig_cmp_same_class<B: Backend>(group: &mut BenchmarkGroup<WallTime>) {
    let mut rng = seeded_rng();
    for &class in ValueClass::ALL {
        let pairs: Vec<(B::Unsigned, B::Unsigned)> = (0..32)
            .map(|_| (B::sample_unsigned(class, &mut rng), B::sample_unsigned(class, &mut rng)))
            .collect();
        group.bench_with_input(BenchmarkId::new(B::NAME, class.label()), &pairs, |b, p| {
            let mut i = 0usize;
            b.iter(|| {
                let (a, c) = &p[i & 31];
                i = i.wrapping_add(1);
                black_box(a).cmp(black_box(c))
            })
        });
    }
}

fn ubig_hash_same_class<B: Backend>(group: &mut BenchmarkGroup<WallTime>) {
    let mut rng = seeded_rng();
    for &class in ValueClass::ALL {
        let inputs: Vec<B::Unsigned> = (0..32)
            .map(|_| B::sample_unsigned(class, &mut rng))
            .collect();
        group.bench_with_input(BenchmarkId::new(B::NAME, class.label()), &inputs, |b, v| {
            let mut i = 0usize;
            b.iter(|| {
                let x = &v[i & 31];
                i = i.wrapping_add(1);
                let mut h = DefaultHasher::new();
                black_box(x).hash(&mut h);
                h.finish()
            })
        });
    }
}

fn ubig_clone_same_class<B: Backend>(group: &mut BenchmarkGroup<WallTime>) {
    let mut rng = seeded_rng();
    for &class in ValueClass::ALL {
        let inputs: Vec<B::Unsigned> = (0..32)
            .map(|_| B::sample_unsigned(class, &mut rng))
            .collect();
        group.bench_with_input(BenchmarkId::new(B::NAME, class.label()), &inputs, |b, v| {
            let mut i = 0usize;
            b.iter(|| {
                let x = &v[i & 31];
                i = i.wrapping_add(1);
                black_box(x).clone()
            })
        });
    }
}

per_backend!(bench_ubig_eq_same_class, "ubig_eq", ubig_eq_same_class);
per_backend!(bench_ubig_cmp_same_class, "ubig_cmp", ubig_cmp_same_class);
per_backend!(bench_ubig_hash_same_class, "ubig_hash", ubig_hash_same_class);
per_backend!(bench_ubig_clone_same_class, "ubig_clone", ubig_clone_same_class);

// ---- string round-trip (decimal-string interchange) ----

fn ibig_display_small<B: Backend>(group: &mut BenchmarkGroup<WallTime>) {
    let mut rng = seeded_rng();
    let inputs: Vec<B::Signed> = (0..128)
        .map(|_| B::sample_signed(ValueClass::OneWord, &mut rng))
        .collect();
    group.bench_function(B::NAME, |b| {
        let mut i = 0usize;
        b.iter(|| {
            let v = &inputs[i & 127];
            i = i.wrapping_add(1);
            black_box(v).to_string()
        })
    });
}

fn ibig_from_str_small<B: Backend>(group: &mut BenchmarkGroup<WallTime>) {
    let mut rng = seeded_rng();
    let inputs: Vec<String> = (0..128)
        .map(|_| B::sample_signed(ValueClass::OneWord, &mut rng).to_string())
        .collect();
    group.bench_function(B::NAME, |b| {
        let mut i = 0usize;
        b.iter(|| {
            let s = &inputs[i & 127];
            i = i.wrapping_add(1);
            B::Signed::parse(black_box(s))
        })
    });
}

per_backend!(bench_ibig_display_small, "ibig_display_small", ibig_display_small);
per_backend!(bench_ibig_from_str_small, "ibig_from_str_small", ibig_from_str_small);

criterion_group!(
    benches,
    bench_from_i64,
    bench_from_i128,
    bench_from_u64,
    bench_from_u128,
    bench_try_into_i128,
    bench_ubig_add_by_class,
    bench_ubig_mul_by_class,
    bench_ibig_add_by_class,
    bench_ubig_add_mixed,
    bench_ubig_add_assign_by_class,
    bench_ibig_add_assign_by_class,
    bench_ubig_sub_assign_by_class,
    bench_ibig_sub_assign_by_class,
    bench_ubig_add_assign_heap_acc_small_rhs,
    bench_ibig_add_assign_heap_acc_small_rhs,
    bench_ibig_add_assign_i64_into_heap_acc,
    bench_ibig_add_assign_i128_into_heap_acc,
    bench_ubig_add_assign_u64_into_heap_acc,
    bench_ubig_add_assign_u128_into_heap_acc,
    bench_ubig_bitxor_assign_by_class,
    bench_ubig_eq_same_class,
    bench_ubig_cmp_same_class,
    bench_ubig_hash_same_class,
    bench_ubig_clone_same_class,
    bench_ibig_display_small,
    bench_ibig_from_str_small,
);

criterion_main!(benches);
