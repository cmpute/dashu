//! Property-based-testing shrinker workload.
//!
//! These benchmarks are derived from profiling specific hot paths that show
//! up when profiling the test suite of
//! [hegel-rust](https://github.com/hegeldev/hegel-rust), but should mostly 
//! just be taken as interesting examples of realistic workloads. These ones
//! are particularly focused on hot paths that occur during shrinking.
//!
//! Each bench body is generic over [`Backend`] and run for every backend, so
//! dashu and the comparison libraries land in one criterion report under the
//! backend name. The pure-Rust backends (dashu, ibig, num-bigint, malachite)
//! are always built; rug is added with `--features gmp` (needs the GMP
//! toolchain). Where dashu uses a truly-unsigned `UBig` for a sort-key
//! magnitude, single-signed-type backends model it via `Backend::magnitude`
//! (`abs`).
//!
//! Run:
//!   cargo bench --manifest-path integer-bench/Cargo.toml --bench shrinker
//! Include the rug backend too (needs the GMP toolchain): add `--features gmp`.

use criterion::measurement::WallTime;
use criterion::{
    black_box, criterion_group, criterion_main, BenchmarkGroup, BenchmarkId, Criterion,
};
use integer_bench::{
    seeded_rng, Backend, BenchInt, Dashu, Ibig, Malachite, Num, SignedInt, UnsignedInt, ValueClass,
};

#[cfg(feature = "gmp")]
use integer_bench::Rug;

/// A node-shaped tuple: (min, max, shrink-target, value), all signed.
type Node4<B> = (
    <B as Backend>::Signed,
    <B as Backend>::Signed,
    <B as Backend>::Signed,
    <B as Backend>::Signed,
);

/// A value→index scenario: (label, min, shrink-target, max).
type Scenario<B> = (
    &'static str,
    <B as Backend>::Signed,
    <B as Backend>::Signed,
    <B as Backend>::Signed,
);

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

// ---------------------------------------------------------------------------
// 1. Clone — typically the dominant cost.
//
// A shrinker holds a sequence of choice nodes, each carrying four integers
// (min, max, shrink-target, value), and clones the whole sequence for every
// candidate it evaluates, so per-integer clone cost is multiplied by
// 4 * n_nodes * n_candidates.
// ---------------------------------------------------------------------------

fn ibig_clone_by_class<B: Backend>(group: &mut BenchmarkGroup<WallTime>) {
    let mut rng = seeded_rng();
    for &class in ValueClass::ALL {
        let inputs: Vec<B::Signed> = (0..32).map(|_| B::sample_signed(class, &mut rng)).collect();
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

/// Clone a node-shaped struct: 4 integer fields (min, max, shrink-target,
/// value). This is the atomic unit the shrinker clones; measuring it directly
/// captures the aggregate overhead better than per-field clones.
fn choice_node_clone<B: Backend>(group: &mut BenchmarkGroup<WallTime>) {
    let mut rng = seeded_rng();
    for &class in &[ValueClass::OneWord, ValueClass::TwoWord] {
        let nodes: Vec<Node4<B>> = (0..32)
            .map(|_| {
                let min = B::sample_signed(class, &mut rng);
                let max = B::sample_signed(class, &mut rng);
                let towards = B::Signed::from_i64(0);
                let value = B::sample_signed(class, &mut rng);
                (min, max, towards, value)
            })
            .collect();
        group.bench_with_input(BenchmarkId::new(B::NAME, class.label()), &nodes, |b, n| {
            let mut i = 0usize;
            b.iter(|| {
                let (min, max, towards, value) = &n[i & 31];
                i = i.wrapping_add(1);
                (
                    black_box(min).clone(),
                    black_box(max).clone(),
                    black_box(towards).clone(),
                    black_box(value).clone(),
                )
            })
        });
    }
}

// ---------------------------------------------------------------------------
// 2. Drop — paired with clone: every cloned value is eventually dropped.
//    The shrinker clones the node sequence, evaluates it, then drops it.
// ---------------------------------------------------------------------------

fn ibig_drop_by_class<B: Backend>(group: &mut BenchmarkGroup<WallTime>) {
    let mut rng = seeded_rng();
    for &class in ValueClass::ALL {
        let templates: Vec<B::Signed> =
            (0..32).map(|_| B::sample_signed(class, &mut rng)).collect();
        group.bench_with_input(BenchmarkId::new(B::NAME, class.label()), &templates, |b, t| {
            let mut i = 0usize;
            b.iter(|| {
                let x = t[i & 31].clone();
                i = i.wrapping_add(1);
                drop(black_box(x));
            })
        });
    }
}

// ---------------------------------------------------------------------------
// 3. sort_key pattern: `(value - target).magnitude()`.
//    Computed once per node per candidate, so ~n_nodes * n_candidates times
//    per shrink run.
// ---------------------------------------------------------------------------

fn ibig_sub_magnitude<B: Backend>(group: &mut BenchmarkGroup<WallTime>) {
    let mut rng = seeded_rng();
    for &class in &[
        ValueClass::OneWord,
        ValueClass::TwoWord,
        ValueClass::JustOverInline,
    ] {
        let pairs: Vec<(B::Signed, B::Signed)> = (0..32)
            .map(|_| (B::sample_signed(class, &mut rng), B::sample_signed(class, &mut rng)))
            .collect();
        group.bench_with_input(BenchmarkId::new(B::NAME, class.label()), &pairs, |b, p| {
            let mut i = 0usize;
            b.iter(|| {
                let (value, target) = &p[i & 31];
                i = i.wrapping_add(1);
                B::magnitude(black_box(value).sub_ref(black_box(target)))
            })
        });
    }
}

// ---------------------------------------------------------------------------
// 4. Clamp a value into a range: `value.clamp(min, max)`.
//    Uses Ord::clamp, which does two comparisons plus one clone.
// ---------------------------------------------------------------------------

fn ibig_clamp<B: Backend>(group: &mut BenchmarkGroup<WallTime>) {
    let mut rng = seeded_rng();
    for &class in &[ValueClass::OneWord, ValueClass::TwoWord] {
        let triples: Vec<(B::Signed, B::Signed, B::Signed)> = (0..32)
            .map(|_| {
                let mut vals = [
                    B::sample_signed(class, &mut rng),
                    B::sample_signed(class, &mut rng),
                    B::sample_signed(class, &mut rng),
                ];
                vals.sort();
                let [min, value, max] = vals;
                (min, value, max)
            })
            .collect();
        group.bench_with_input(BenchmarkId::new(B::NAME, class.label()), &triples, |b, t| {
            let mut i = 0usize;
            b.iter(|| {
                let (min, value, max) = &t[i & 31];
                i = i.wrapping_add(1);
                black_box(value).clone().clamp(min.clone(), max.clone())
            })
        });
    }
}

// ---------------------------------------------------------------------------
// 5. Range validation: `min <= value && value <= max`.
//    Two comparisons per check, run on every candidate.
// ---------------------------------------------------------------------------

fn ibig_double_cmp<B: Backend>(group: &mut BenchmarkGroup<WallTime>) {
    let mut rng = seeded_rng();
    for &class in &[
        ValueClass::OneWord,
        ValueClass::TwoWord,
        ValueClass::JustOverInline,
    ] {
        let triples: Vec<(B::Signed, B::Signed, B::Signed)> = (0..32)
            .map(|_| {
                let a = B::sample_signed(class, &mut rng);
                let b = B::sample_signed(class, &mut rng);
                let c = B::sample_signed(class, &mut rng);
                (a, b, c)
            })
            .collect();
        group.bench_with_input(BenchmarkId::new(B::NAME, class.label()), &triples, |b, t| {
            let mut i = 0usize;
            b.iter(|| {
                let (min, value, max) = &t[i & 31];
                i = i.wrapping_add(1);
                black_box(min) <= black_box(value) && black_box(value) <= black_box(max)
            })
        });
    }
}

// ---------------------------------------------------------------------------
// 6. Construct integers from small primitives — `from(0)`, `from(1)`,
//    `from(n)` — done constantly throughout a shrink run.
// ---------------------------------------------------------------------------

fn ibig_from_small_consts<B: Backend>(group: &mut BenchmarkGroup<WallTime>) {
    group.bench_function(format!("{}/zero", B::NAME), |b| {
        b.iter(|| B::Signed::from_i64(black_box(0)))
    });
    group.bench_function(format!("{}/one", B::NAME), |b| {
        b.iter(|| B::Signed::from_i64(black_box(1)))
    });
    group.bench_function(format!("{}/minus_one", B::NAME), |b| {
        b.iter(|| B::Signed::from_i64(black_box(-1)))
    });
}

// ---------------------------------------------------------------------------
// 7. Unsigned compare — comparing sort-key magnitudes (unsigned distances)
//    while ordering candidate sequences.
// ---------------------------------------------------------------------------

fn ubig_cmp_by_class<B: Backend>(group: &mut BenchmarkGroup<WallTime>) {
    let mut rng = seeded_rng();
    for &class in &[
        ValueClass::OneWord,
        ValueClass::TwoWord,
        ValueClass::JustOverInline,
    ] {
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

// ---------------------------------------------------------------------------
// 8. Shift-right descent — a binary search that probes
//    `lo + (dist >> k as usize)` where k grows geometrically.
// ---------------------------------------------------------------------------

fn ibig_shift_right_descent<B: Backend>(group: &mut BenchmarkGroup<WallTime>) {
    let mut rng = seeded_rng();
    for &class in &[ValueClass::OneWord, ValueClass::TwoWord] {
        let pairs: Vec<(B::Signed, B::Signed)> = (0..32)
            .map(|_| {
                let lo = B::sample_signed(class, &mut rng);
                let dist = B::magnitude(B::sample_signed(class, &mut rng));
                (lo, B::unsigned_to_signed(dist))
            })
            .collect();
        group.bench_with_input(BenchmarkId::new(B::NAME, class.label()), &pairs, |b, p| {
            let mut i = 0usize;
            b.iter(|| {
                let (lo, dist) = &p[i & 31];
                i = i.wrapping_add(1);
                // Inner loop of the descent search: lo + (dist >> k as usize)
                // for k = 1, 2, 4, 8, 16
                let mut last = lo.clone();
                for k in [1usize, 2, 4, 8, 16] {
                    last = lo.add_ref(&black_box(dist).shr_ref(k));
                }
                last
            })
        });
    }
}

// ---------------------------------------------------------------------------
// 9. Combined shrinker step — one candidate evaluation's hot path: clone n
//    nodes (each 4 integers), compute a sort key for each (sub + magnitude),
//    then compare the sort-key sequences lexicographically.
//
//    The top-level scenario bench that combines all of the above.
// ---------------------------------------------------------------------------

fn shrinker_consider_workload<B: Backend>(group: &mut BenchmarkGroup<WallTime>) {
    let mut rng = seeded_rng();
    for n_nodes in [4, 16, 64] {
        // Build n nodes: each has (min, max, shrink-target, value).
        let nodes: Vec<Node4<B>> = (0..n_nodes)
            .map(|_| {
                let class = if rand_v08::Rng::gen::<bool>(&mut rng) {
                    ValueClass::OneWord
                } else {
                    ValueClass::TwoWord
                };
                let a = B::sample_signed(class, &mut rng);
                let b = B::sample_signed(class, &mut rng);
                let (min, max) = if a <= b { (a, b) } else { (b, a) };
                let towards = B::Signed::from_i64(0);
                let value = B::sample_signed(class, &mut rng);
                (min, max, towards, value)
            })
            .collect();

        group.bench_with_input(BenchmarkId::new(B::NAME, n_nodes), &nodes, |b, nodes| {
            b.iter(|| {
                // Phase 1: clone all nodes (usually the dominant cost).
                let cloned: Vec<_> = nodes
                    .iter()
                    .map(|(min, max, towards, value)| {
                        (min.clone(), max.clone(), towards.clone(), value.clone())
                    })
                    .collect();

                // Phase 2: compute sort_key for each: sub + magnitude.
                let sort_keys: Vec<(B::Unsigned, bool)> = cloned
                    .iter()
                    .map(|(_min, _max, towards, value)| {
                        let target = towards.clone();
                        let distance = B::magnitude(black_box(value).sub_ref(&target));
                        let below = *value < target;
                        (distance, below)
                    })
                    .collect();

                // Phase 3: lexicographic comparison of sort key sequences.
                let mut total_order = std::cmp::Ordering::Equal;
                for i in 0..sort_keys.len() {
                    let cmp = sort_keys[i].cmp(black_box(&sort_keys[sort_keys.len() - 1 - i]));
                    if cmp != std::cmp::Ordering::Equal {
                        total_order = cmp;
                        break;
                    }
                }
                (cloned, sort_keys, total_order)
            })
        });
    }
}

// ---------------------------------------------------------------------------
// 10. Index→value binary-search step — maps an index to a value with
//     unsigned arithmetic: mid = lo + ((hi - lo) >> 1), then
//     min(mid, above) + min(mid, below) comparisons.
// ---------------------------------------------------------------------------

fn ubig_binary_search_step<B: Backend>(group: &mut BenchmarkGroup<WallTime>) {
    let mut rng = seeded_rng();
    for &class in &[
        ValueClass::OneWord,
        ValueClass::TwoWord,
        ValueClass::JustOverInline,
    ] {
        let triples: Vec<(B::Unsigned, B::Unsigned, B::Unsigned)> = (0..32)
            .map(|_| {
                let a = B::sample_unsigned(class, &mut rng);
                let b = B::sample_unsigned(class, &mut rng);
                let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
                let above = B::sample_unsigned(class, &mut rng);
                (lo, hi, above)
            })
            .collect();
        group.bench_with_input(BenchmarkId::new(B::NAME, class.label()), &triples, |b, t| {
            let mut i = 0usize;
            b.iter(|| {
                let (lo, hi, above) = &t[i & 31];
                i = i.wrapping_add(1);
                let mid = lo.add_ref(&hi.sub_ref(lo).shr_ref(1));
                let total = std::cmp::min(&mid, black_box(above))
                    .add_ref(std::cmp::min(&mid, black_box(above)));
                (mid, total)
            })
        });
    }
}

// ---------------------------------------------------------------------------
// 11. HashMap insert+lookup workload — signed integers are sometimes used as
//     deterministic-id keys in shrinker-adjacent data structures. Exercises
//     the Hash + Eq impls on inline values.
// ---------------------------------------------------------------------------

fn ibig_hashmap_keys<B: Backend>(group: &mut BenchmarkGroup<WallTime>) {
    use std::collections::HashMap;

    let mut rng = seeded_rng();
    for &class in &[ValueClass::OneWord, ValueClass::TwoWord] {
        let keys: Vec<B::Signed> = (0..128)
            .map(|_| B::sample_signed(class, &mut rng))
            .collect();
        // Pre-populate the map.
        let mut map: HashMap<B::Signed, u32> = HashMap::with_capacity(keys.len());
        for (i, k) in keys.iter().enumerate() {
            map.insert(k.clone(), i as u32);
        }
        group.bench_with_input(
            BenchmarkId::new(B::NAME, class.label()),
            &(keys, map),
            |b, (ks, m)| {
                let mut i = 0usize;
                b.iter(|| {
                    let k = &ks[i & 127];
                    i = i.wrapping_add(1);
                    m.get(black_box(k)).copied()
                })
            },
        );
    }
}

// ---------------------------------------------------------------------------
// 12. Full index→value binary search — often the single most expensive
//     operation. Maps an index to a value over a full i128-range choice:
//     binary search with mid = lo + ((hi - lo) >> 1),
//     total = min(mid, above) + min(mid, below), ~128 iterations for
//     i128::MIN..i128::MAX.
// ---------------------------------------------------------------------------

fn from_index_full_search<B: Backend>(group: &mut BenchmarkGroup<WallTime>) {
    // i128 range: above = i128::MAX, below = i128::MIN.abs() = i128::MAX + 1
    // The common case: a choice over {min: i128::MIN+1, max: i128::MAX, shrink-target: 0}.
    let above = B::Unsigned::from_u128(i128::MAX as u128);
    let below = B::Unsigned::from_u128(i128::MAX as u128 + 1);

    for target_frac in [0.0f64, 0.25, 0.5, 0.75, 1.0] {
        let target_idx = {
            let max_idx = above.add_ref(&below);
            let frac_bits = (target_frac * 1000.0) as u128;
            max_idx
                .mul_ref(&B::Unsigned::from_u128(frac_bits))
                .div_ref(&B::Unsigned::from_u64(1000))
        };

        group.bench_with_input(
            BenchmarkId::new(B::NAME, format!("frac_{:.0}pct", target_frac * 100.0)),
            &target_idx,
            |b, idx| {
                b.iter(|| {
                    let one = B::Unsigned::from_u64(1);
                    let mut lo = one.clone();
                    let mut hi = std::cmp::max(&above, &below).clone();
                    while lo < hi {
                        let mid = lo.add_ref(&hi.sub_ref(&lo).shr_ref(1));
                        let total =
                            std::cmp::min(&mid, &above).add_ref(std::cmp::min(&mid, &below));
                        if total >= *black_box(idx) {
                            hi = mid;
                        } else {
                            lo = mid.add_ref(&one);
                        }
                    }
                    lo
                })
            },
        );
    }
}

// ---------------------------------------------------------------------------
// 13. By-ref unsigned add/sub — the index search operates on references, not
//     owned values. The benches above cover owned operands; this covers the
//     by-reference path.
// ---------------------------------------------------------------------------

fn ubig_ref_add_by_class<B: Backend>(group: &mut BenchmarkGroup<WallTime>) {
    let mut rng = seeded_rng();
    for &class in &[
        ValueClass::OneWord,
        ValueClass::TwoWord,
        ValueClass::JustOverInline,
    ] {
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

fn ubig_ref_sub_by_class<B: Backend>(group: &mut BenchmarkGroup<WallTime>) {
    let mut rng = seeded_rng();
    for &class in &[
        ValueClass::OneWord,
        ValueClass::TwoWord,
        ValueClass::JustOverInline,
    ] {
        let pairs: Vec<(B::Unsigned, B::Unsigned)> = (0..32)
            .map(|_| {
                let a = B::sample_unsigned(class, &mut rng);
                let b = B::sample_unsigned(class, &mut rng);
                // Ensure a >= b so subtraction doesn't panic.
                if a >= b {
                    (a, b)
                } else {
                    (b, a)
                }
            })
            .collect();
        group.bench_with_input(BenchmarkId::new(B::NAME, class.label()), &pairs, |b, p| {
            let mut i = 0usize;
            b.iter(|| {
                let (a, c) = &p[i & 31];
                i = i.wrapping_add(1);
                black_box(a).sub_ref(black_box(c))
            })
        });
    }
}

per_backend!(bench_ibig_clone_by_class, "ibig_clone", ibig_clone_by_class);
per_backend!(bench_choice_node_clone, "choice_node_clone", choice_node_clone);
per_backend!(bench_ibig_drop_by_class, "ibig_drop", ibig_drop_by_class);
per_backend!(bench_ibig_sub_magnitude, "ibig_sub_magnitude", ibig_sub_magnitude);
per_backend!(bench_ibig_clamp, "ibig_clamp", ibig_clamp);
per_backend!(bench_ibig_double_cmp, "ibig_double_cmp", ibig_double_cmp);
per_backend!(bench_ibig_from_small_consts, "ibig_from_const", ibig_from_small_consts);
per_backend!(bench_ubig_cmp_by_class, "ubig_cmp_shrinker", ubig_cmp_by_class);
per_backend!(bench_ibig_shift_right_descent, "ibig_shr_descent", ibig_shift_right_descent);
per_backend!(
    bench_shrinker_consider_workload,
    "shrinker_consider",
    shrinker_consider_workload
);
per_backend!(
    bench_ubig_binary_search_step,
    "ubig_binary_search_step",
    ubig_binary_search_step
);
per_backend!(bench_ibig_hashmap_keys, "ibig_hashmap_keys", ibig_hashmap_keys);
per_backend!(bench_from_index_full_search, "from_index_full_search", from_index_full_search);
per_backend!(bench_ubig_ref_add_by_class, "ubig_ref_add", ubig_ref_add_by_class);
per_backend!(bench_ubig_ref_sub_by_class, "ubig_ref_sub", ubig_ref_sub_by_class);

// ---------------------------------------------------------------------------
// 14. Boundary-value sort — build a Vec of ~258 boundary values
//     (0, ±1, ±2^k for k in 0..=128, min, max), then dedup + sort. Exercises
//     the comparison path on a mix of inline and just-over-inline values.
// ---------------------------------------------------------------------------

fn ibig_boundary_sort<B: Backend>(group: &mut BenchmarkGroup<WallTime>) {
    let mut rng = seeded_rng();
    let min = B::Signed::from_i128(i128::MIN + 1);
    let max = B::Signed::from_i128(i128::MAX);
    group.bench_function(B::NAME, |b| {
        b.iter(|| {
            let mut values = vec![min.clone(), max.clone(), B::Signed::from_i64(0)];
            for sign in [1i128, -1] {
                for exp in 0..=128u32 {
                    let v = B::Signed::from_i128(sign)
                        .mul_ref(&B::Signed::from_u128(1u128 << exp.min(127)));
                    values.push(v);
                }
            }
            values.push(B::Signed::from_i64(rand_v08::Rng::gen_range(&mut rng, -10i64..10)));
            values.sort();
            values.dedup();
            black_box(values.len())
        })
    });
}

per_backend!(bench_ibig_boundary_sort, "ibig_boundary_sort", ibig_boundary_sort);

// ---------------------------------------------------------------------------
// 15. Unsigned min — used heavily in the index search: std::cmp::min(&mid,
//     &above). Each binary-search iteration does 2× min (Ord::cmp + branch).
//     This micro-bench isolates unsigned comparison cost on inline values.
// ---------------------------------------------------------------------------

fn ubig_min_inline<B: Backend>(group: &mut BenchmarkGroup<WallTime>) {
    let mut rng = seeded_rng();
    for &class in &[ValueClass::OneWord, ValueClass::TwoWord] {
        let pairs: Vec<(B::Unsigned, B::Unsigned)> = (0..32)
            .map(|_| (B::sample_unsigned(class, &mut rng), B::sample_unsigned(class, &mut rng)))
            .collect();
        group.bench_with_input(BenchmarkId::new(B::NAME, class.label()), &pairs, |b, p| {
            let mut i = 0usize;
            b.iter(|| {
                let (a, c) = &p[i & 31];
                i = i.wrapping_add(1);
                std::cmp::min(black_box(a), black_box(c))
            })
        });
    }
}

per_backend!(bench_ubig_min_inline, "ubig_min", ubig_min_inline);

// ---------------------------------------------------------------------------
// 16. value→index lookup — the forward direction, inverse of
//     `from_index_full_search`, with the same sub/magnitude/min/add shape.
//     For a choice over (min, s, max) the index of `value` is:
//
//       above = (max - s).magnitude()
//       below = (s - min).magnitude()
//       d_abs = (value - s).magnitude()
//       d_minus_one = d_abs - 1
//       count = min(d_minus_one, above) + min(d_minus_one, below)
//       (+ 1 or 2 depending on sign / d_abs vs above)
//
//     This bench drives the body for a fixed (min, s, max) over a Vec of
//     pre-sampled values, capturing the per-value cost.
// ---------------------------------------------------------------------------

fn integer_choice_to_index<B: Backend>(group: &mut BenchmarkGroup<WallTime>) {
    let mut rng = seeded_rng();

    // Two ranges: a small i128-bracket (the common case, matches the
    // `from_index_full_search` setup) and a heap-only range, so the bench
    // reflects both the inline and just-over-inline paths.
    let scenarios: [Scenario<B>; 2] = [
        (
            "i128_range",
            B::Signed::from_i128(i128::MIN + 1),
            B::Signed::from_i64(0),
            B::Signed::from_i128(i128::MAX),
        ),
        (
            "heap_range",
            B::Signed::from_i64(0),
            B::Signed::from_i64(0),
            B::Signed::from_u128(1).shl_ref(200),
        ),
    ];

    for (label, min_v, s, max_v) in scenarios.iter() {
        // Pre-sample 32 in-range values. Mix nasty pool with random draws
        // so we don't end up only exercising one branch (`d_abs <= above`).
        let values: Vec<B::Signed> = (0..32)
            .map(|i| {
                let class = match i % 4 {
                    1 => ValueClass::TwoWord,
                    _ => ValueClass::OneWord,
                };
                let mag = B::sample_signed(class, &mut rng);
                // Clamp into range so the lookup doesn't have to reject.
                if &mag > max_v {
                    max_v.clone()
                } else if &mag < min_v {
                    min_v.clone()
                } else {
                    mag
                }
            })
            .collect();
        group.bench_with_input(
            BenchmarkId::new(B::NAME, label),
            &(min_v.clone(), s.clone(), max_v.clone(), values),
            |b, (min_v, s, max_v, values)| {
                let mut i = 0usize;
                let one = B::Unsigned::from_u64(1);
                b.iter(|| {
                    let v = &values[i & 31];
                    i = i.wrapping_add(1);
                    // The value→index body, inlined.
                    if v == s {
                        B::Unsigned::from_u64(0)
                    } else {
                        let above = B::magnitude(max_v.sub_ref(s));
                        let below = B::magnitude(s.sub_ref(min_v));
                        let d_abs = B::magnitude(v.sub_ref(s));
                        let d_minus_one = d_abs.sub_ref(&one);
                        let mut count = std::cmp::min(&d_minus_one, &above)
                            .add_ref(std::cmp::min(&d_minus_one, &below));
                        if v > s {
                            return count.add_ref(&one);
                        }
                        if d_abs <= above {
                            count.add_assign_ref(&one);
                        }
                        count.add_ref(&one)
                    }
                })
            },
        );
    }
}

per_backend!(
    bench_integer_choice_to_index,
    "integer_choice_to_index",
    integer_choice_to_index
);

// ---------------------------------------------------------------------------
// 17. Lazy lexicographic sort-key compare of two node sequences.
//
//     A shrinker compares pre/post candidate sequences by walking both in
//     lockstep and computing per-node sort keys on the fly. The per-node key
//     is `(value - shrink_towards).magnitude(), value < shrink_towards`
//     (an allocated unsigned magnitude + a bool).
//
//     The lazy variant is meaningfully different from
//     `shrinker_consider_workload` (which eagerly materialises all sort keys
//     into a `Vec`): when the sequences differ early, the lazy form does far
//     less work, and the per-iteration allocation cost is what the real
//     shrinker pays. Two parameterisations:
//
//     * `same_prefix` — sequences agree for the first half, differ in the
//       middle. Exercises the typical "small change to a long shrunk
//       sequence" path.
//     * `differ_at_zero` — sequences differ at position 0. Tests the early-
//       exit fast path where we only allocate two sort keys.
// ---------------------------------------------------------------------------

// Each "node" is a (value, shrink_towards) pair; the sort key is
// ((value - shrink_towards).magnitude(), value < shrink_towards).
fn sort_key<B: Backend>(node: &(B::Signed, B::Signed)) -> (B::Unsigned, bool) {
    let (value, target) = node;
    (B::magnitude(value.sub_ref(target)), value < target)
}

fn lex_cmp<B: Backend>(
    a: &[(B::Signed, B::Signed)],
    b: &[(B::Signed, B::Signed)],
) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    match a.len().cmp(&b.len()) {
        Ordering::Equal => {}
        ord => return ord,
    }
    for (x, y) in a.iter().zip(b.iter()) {
        let key_x = sort_key::<B>(x);
        let key_y = sort_key::<B>(y);
        match (&key_x.0, key_x.1).cmp(&(&key_y.0, key_y.1)) {
            Ordering::Equal => continue,
            ord => return ord,
        }
    }
    Ordering::Equal
}

fn nodes_sort_key_lex_cmp<B: Backend>(group: &mut BenchmarkGroup<WallTime>) {
    let mut rng = seeded_rng();
    type Node<B> = (<B as Backend>::Signed, <B as Backend>::Signed);

    for n_nodes in [4usize, 16, 64] {
        // Scenario A: sequences share the first half, differ in the middle.
        let a: Vec<Node<B>> = (0..n_nodes)
            .map(|_| (B::sample_signed(ValueClass::OneWord, &mut rng), B::Signed::from_i64(0)))
            .collect();
        let mut b = a.clone();
        let mid = n_nodes / 2;
        b[mid].0 = b[mid].0.add_ref(&B::Signed::from_i64(1));

        group.bench_with_input(
            BenchmarkId::new(format!("{}/same_prefix", B::NAME), n_nodes),
            &(a, b),
            |bn, (a, b)| {
                bn.iter(|| lex_cmp::<B>(black_box(a), black_box(b)));
            },
        );

        // Scenario B: differ at index 0 — cmp returns after one pair of
        // sort_key allocations.
        let a: Vec<Node<B>> = (0..n_nodes)
            .map(|_| (B::sample_signed(ValueClass::TwoWord, &mut rng), B::Signed::from_i64(0)))
            .collect();
        let mut b = a.clone();
        b[0].0 = b[0].0.add_ref(&B::Signed::from_i64(1));
        group.bench_with_input(
            BenchmarkId::new(format!("{}/differ_at_zero", B::NAME), n_nodes),
            &(a, b),
            |bn, (a, b)| {
                bn.iter(|| lex_cmp::<B>(black_box(a), black_box(b)));
            },
        );
    }
}

per_backend!(bench_nodes_sort_key_lex_cmp, "nodes_sort_key_lex_cmp", nodes_sort_key_lex_cmp);

// ---------------------------------------------------------------------------
// 18. Descent step — the inner loop of an integer shrink-towards-zero search:
//     try candidates `base - (2 * n)` for n = 1, 2, 4, 8, ... until a
//     predicate fails, then binary-search the bracket.
//
//     The integer work is candidate construction + range validation:
//     `from(small_const)`, `&base - that`, `cand >= min && cand <= max` —
//     a sub plus two compares per probe. The bench drives a fixed step
//     sequence so the cost reflects the per-probe overhead.
// ---------------------------------------------------------------------------

fn shrinker_descent_subtract<B: Backend>(group: &mut BenchmarkGroup<WallTime>) {
    let mut rng = seeded_rng();
    for &class in &[ValueClass::OneWord, ValueClass::TwoWord] {
        // 32 distinct (base, min, max) triples so the bench loop sees varied
        // inputs; magnitudes match the inline workload the shrinker actually
        // touches in tests.
        let triples: Vec<(B::Signed, B::Signed, B::Signed)> = (0..32)
            .map(|_| {
                let base = B::sample_signed(class, &mut rng);
                let min = base.sub_ref(&B::Signed::from_i64(1024));
                let max = base.add_ref(&B::Signed::from_i64(1024));
                (base, min, max)
            })
            .collect();
        group.bench_with_input(BenchmarkId::new(B::NAME, class.label()), &triples, |b, t| {
            let mut i = 0usize;
            // Exponential probe sequence used by the descent search:
            // 1, 2, 3, 4 then geometric (8, 16, 32, ...).
            const STEPS: [u64; 9] = [1, 2, 3, 4, 8, 16, 32, 64, 128];
            b.iter(|| {
                let (base, min, max) = &t[i & 31];
                i = i.wrapping_add(1);
                let mut valid_count = 0u32;
                for n in STEPS {
                    // `&base - (2 * n)` is the shrink-by-multiples-of-2
                    // probe; the linear-1 probe is `&base - n`.
                    let cand = base.sub_ref(&B::Signed::from_u64(2 * n));
                    if &cand >= black_box(min) && &cand <= black_box(max) {
                        valid_count += 1;
                    }
                    let cand = base.sub_ref(&B::Signed::from_u64(n));
                    if &cand >= black_box(min) && &cand <= black_box(max) {
                        valid_count += 1;
                    }
                }
                valid_count
            })
        });
    }
}

per_backend!(
    bench_shrinker_descent_subtract,
    "shrinker_descent_subtract",
    shrinker_descent_subtract
);

criterion_group!(
    benches,
    bench_ibig_clone_by_class,
    bench_choice_node_clone,
    bench_ibig_drop_by_class,
    bench_ibig_sub_magnitude,
    bench_ibig_clamp,
    bench_ibig_double_cmp,
    bench_ibig_from_small_consts,
    bench_ubig_cmp_by_class,
    bench_ibig_shift_right_descent,
    bench_shrinker_consider_workload,
    bench_ubig_binary_search_step,
    bench_ibig_hashmap_keys,
    bench_from_index_full_search,
    bench_ubig_ref_add_by_class,
    bench_ubig_ref_sub_by_class,
    bench_ibig_boundary_sort,
    bench_ubig_min_inline,
    bench_integer_choice_to_index,
    bench_nodes_sort_key_lex_cmp,
    bench_shrinker_descent_subtract,
);

criterion_main!(benches);
