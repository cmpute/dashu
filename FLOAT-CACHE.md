# Float Constant Cache Design

## Overview

Add an opt-in constant cache to `dashu-float` that stores **binary-splitting tree
state** for mathematical constants (π, ln2, ln10, …), enabling progressive
refinement: extend from 100 digits to 1000 digits without recomputing from
scratch.

The cache is a **standalone `MathCache<const B: Word>` type**. It is *not*
embedded in `Context` or `FBig`. Those types stay exactly as they are today
(`Copy`, `Send + Sync`, `context()` a `const fn` returning by value) — **zero
migration, zero breaking changes**. The user opts in by holding a `MathCache` and
calling its methods.

## Design decisions at a glance

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Storage location | **Standalone `MathCache<B>` type** | Leaves `Context`/`FBig` untouched (`Copy + Send + Sync`); only functions that actually reuse a constant need to take the cache |
| Base | **`const B: Word` type parameter** | Base is fixed per cache; cached term counts and finalized values are base-specific, and the return type is `FBig<R, B>` |
| Rounding mode | **Method generic `pi::<R>(precision)`** | Series state is rounding-independent; one cache serves all rounding modes |
| Sharing mechanism | `RefCell<ConstCache>` interior mutability | Per-thread ownership, no atomics on the compute path; `MathCache: Send + !Sync` |
| Cross-thread | User wraps `Arc<Mutex<MathCache<B>>>` | Standard Rust idiom; no bespoke export/import API |
| Storage | **Flattened `Option<CachedState>` fields** on `ConstCache` (`pi`, `iacoth_6`, `iacoth_9`, `iacoth_99`) | Only a handful of constants — a field per series beats a map and makes sub-series collisions impossible |
| Entries per series | **Single largest state only** (no per-precision map) | Lower precision ⇒ reuse the cached higher-precision state and round down; higher precision ⇒ extend it |
| **What is stored** | **Binary-splitting tree state `(P, Q, T, num_terms)`, never final float values** | Exact integers, losslessly extensible |
| Merge strategy | Universal `T' = T_l·Q_r + P_l·T_r` | Same merge for every series; only the leaf differs |

## Core principle: binary splitting makes every constant extensible

A cache that stores final `FBig` values cannot be extended — a 100-digit π result
is useless for computing 1000-digit π. A floating-point partial sum carries
guard-digit rounding that doesn't suffice when the target precision grows.

**Binary splitting** reduces a series to exact integer tree state `(P, Q, T)`.
The cached state is lossless, and extension is tree-merging — pure integer
arithmetic with no rounding.

### The merge is associative → extension is lossless

Define `combine((Pₗ,Qₗ,Tₗ), (Pᵣ,Qᵣ,Tᵣ)) = (Pₗ·Pᵣ, Qₗ·Qᵣ, Tₗ·Qᵣ + Pₗ·Tᵣ)`. This
operation is **associative**: `combine(combine(X,Y), Z) == combine(X, combine(Y,Z))`
(verified algebraically — all three `T` terms expand to the same
`Tₗ·Qₘ·Qᵣ + Tₘ·Pₗ·Qᵣ + Tᵣ·Pₗ·Pₘ`). Therefore `binary_split(0, N)` yields the
same `(P,Q,T)` regardless of how the recursion splits, and
`combine(binary_split(0,K), binary_split(K,N)) == binary_split(0,N)`.

That identity is what makes a cached state at `K` terms byte-for-byte identical to
the left half of the tree for `N > K` terms, and extension a pure tree-merge.

## Series definitions (corrected)

Every cached series is written as `Σ ρₖ` where consecutive terms satisfy a
rational ratio `rₖ/rₖ₋₁ = pₖ/qₖ`. Binary splitting over `[a, b)` returns:

- `P = Π_{a≤k<b} pₖ`, `Q = Π_{a≤k<b} qₖ`
- `T = Σ_{a≤k<b} (Π_{a≤j≤k} pⱼ)·(Π_{k<j<b} qⱼ)`
- merge: `T' = Tₗ·Qᵣ + Pₗ·Tᵣ` (universal)
- leaf at `k` (a≥1): `(pₖ, qₖ, pₖ)`

Using the **ratio** form (not independent `1/qₖ` terms) is what keeps `Q` linear
in precision — see "Bugs fixed" below.

| Constant | Term ratio `pₖ/qₖ` (k≥1) | Leaf `(pₖ, qₖ)` | Sum reconstruction |
|----------|--------------------------|------------------|--------------------|
| π (Chudnovsky) | see `chudnovsky_bs` (existing, unchanged) | `(6k-5)(2k-1)(6k-1)`, `k³·C` | `π = 426880·√10005·Q / T` |
| L(n) = acoth(n) | `(2k-1) / ((2k+1)·n²)` | `(2k-1, (2k+1)·n²)` | `L(n) = (Q + T) / (n·Q)` |

`r₀ = 1/n` is pulled out; the tree computes `Σ_{k=1}^{N-1} ρₖ = T/Q` over `[1, N)`,
then `L(n) = r₀·(1 + T/Q) = (Q + T)/(n·Q)`.

ln2 / ln10 are compositions of cached L(n) series:

- `ln2 = 4·L(6) + 2·L(99)`
- `ln10 = 3·ln2 + 2·L(9)`
- `ln_base(B)` = `ln2` (B=2), `ln10` (B=10), `ln2·log2(B)` (B power of two), else computed directly (and cached by precision)

## Bugs in the earlier draft — fixed here

1. **iacoth term count ignored `n`.** The draft used `required_terms = p / (log2(B)/2)`,
   which has no `n`. For B=10, n=6 that *under-counts* → silently wrong `DBig::ln(2)`.
   **Fix:** the term count must come from `rₖ < B⁻ᵖ` ⇒ `(2k+1)·log_B(n) > p`:
   ```
   required_terms = ceil(precision / (2 * log_B(n))) + GUARD,   GUARD ≈ 8
   ```
   (over-provisioned on purpose — too many terms only adds precision, never corrupts).

2. **iacoth base case used the independent-term form** `qₖ = n^(2k+1)·(2k+1)`, which
   makes `Q = Π qₖ` carry `n^{Σ(2k+1)} = n^{O(N²)}` → **O(p²)-digit intermediates**.
   That's a catastrophic slowdown, not a speedup. **Fix:** ratio form with
   `pₖ = 2k-1`, `qₖ = (2k+1)·n²`, so each leaf multiplies only small integers
   (`n²` is computed once) and `Q` stays O(p) digits.

3. **iacoth merge was wrong.** The draft wrote `T' = Tₗ·Qᵣ + Tᵣ` (dropping the left
   factor). Verified false on L(2) over 2 terms (gives 25/48; correct is 13/24).
   **Fix:** the universal `T' = Tₗ·Qᵣ + Pₗ·Tᵣ` — same as Chudnovsky. (The ratio form
   produces a non-trivial `P = Π(2k-1)`, so `Pₗ` is no longer 1.)

4. **Storage used dictionaries for a handful of constants.** The draft had
   `BTreeMap<ConstantKind, ConstantEntries>` with `ConstantEntries = BTreeMap<usize, _>`,
   plus a per-precision map per series — and `ln2`'s two sub-series (L(6), L(99))
   collided under one key. **Fix:** flatten to one `Option<CachedState>` field per
   series (`pi`, `iacoth_6`, `iacoth_9`, `iacoth_99`), and keep only the *single
   largest* state per series: a smaller-precision request reuses the cached
   higher-precision state and rounds down, so no per-precision map is needed at all.

5. **`FBig`/`Context` were made `!Send`/`!Copy`.** The draft put
   `Rc<RefCell<ConstCache>>` on `Context`, losing `Send+Sync` for the whole float
   type even when the cache is unused. **Fix:** the cache is a standalone
   `MathCache<B>`; `Context`/`FBig` are untouched and remain `Copy + Send + Sync`.

6. **Draft hand-waved iacoth BS performance** ("incremental q-term precomputation…
   competitive") while shipping a `n.pow(2k+1)`-per-leaf version. **Fix:** the ratio
   form *is* the incremental construction — no `pow` per leaf.

(Smaller fixes: `with_cache` helper is gone — `MathCache` owns the methods; manual
`Debug` avoids dumping MB-sized big-ints; `export`/`import` dropped in favor of
`Arc<Mutex<MathCache<B>>>`; `bound_base`/`check_base` panic machinery dropped — `B`
is a const generic param.)

## Architecture

### New module

```
float/src/math/
├── mod.rs
├── consts.rs        (existing — Context::pi, unchanged)
├── trig.rs          (existing)
└── cache.rs         (NEW — MathCache, ConstCache, CachedState)
```

### Types

```rust
// float/src/math/cache.rs

use core::cell::RefCell;
use dashu_int::{IBig, UBig};
use crate::repr::Word;

/// Binary-splitting tree state — exact integers, losslessly extensible.
///
/// Represents `binary_split(start, num_terms)` as the universal triple `(P, Q, T)`,
/// where `start` is 0 for π and 1 for L(n) (whose k=0 term 1/n is pulled out).
/// Pure integers: independent of base and rounding mode. To extend to
/// `new_terms > num_terms`, compute the right half over the new range and merge
/// with the universal `T' = T_l·Q_r + P_l·T_r`.
#[derive(Debug, Clone)]
pub(crate) struct CachedState {
    pub p: UBig,
    pub q: UBig,
    pub t: IBig,
    pub num_terms: usize,
}

/// The cache interior: one slot per series, holding only the **largest** state
/// computed so far for that series. Base is fixed by the `MathCache<B>` type
/// parameter, so there is no base-mismatch check.
///
/// A smaller-precision request reuses the cached (higher-precision) state and
/// rounds down at finalize time — no per-precision map is needed.
#[derive(Debug)]
struct ConstCache {
    pi: Option<CachedState>,
    /// L(6), L(9), L(99) — the sub-series used by ln2 / ln10.
    iacoth_6: Option<CachedState>,
    iacoth_9: Option<CachedState>,
    iacoth_99: Option<CachedState>,
    // future: e, ...
}

impl ConstCache {
    const fn new() -> Self {
        Self { pi: None, iacoth_6: None, iacoth_9: None, iacoth_99: None }
    }
}

/// Universal merge: `combine((Pl,Ql,Tl),(Pr,Qr,Tr))`.
fn merge(pl: &UBig, ql: &UBig, tl: &IBig, pr: &UBig, qr: &UBig, tr: &IBig) -> (UBig, UBig, IBig) {
    let p = pl * pr;
    let q = ql * qr;
    let t = IBig::from(qr) * tl + IBig::from(pl) * tr;
    (p, q, t)
}

/// Ensure `slot` holds state for at least `target` terms, then return `(P, Q, T)`
/// covering `target` terms (or more, when an existing higher-precision state
/// already covers `target` — finalize then rounds down to the requested precision).
///
/// `range_bs(a, b)` computes the leaf-merged state over `[a, b)` and must handle
/// `a == b` by returning the identity `(1, 1, 0)`.
fn extend_or_compute<F>(
    slot: &mut Option<CachedState>,
    start: usize,
    target: usize,
    range_bs: F,
) -> (UBig, UBig, IBig)
where
    F: Fn(usize, usize) -> (UBig, UBig, IBig),
{
    match slot {
        // Already have >= target terms: reuse (extra terms only add precision).
        Some(s) if s.num_terms >= target => (s.p.clone(), s.q.clone(), s.t.clone()),
        // Have fewer terms: extend the right half [num_terms, target) and merge.
        Some(s) => {
            let (pr, qr, tr) = range_bs(s.num_terms, target);
            let (p, q, t) = merge(&s.p, &s.q, &s.t, &pr, &qr, &tr);
            *slot = Some(CachedState { p: p.clone(), q: q.clone(), t: t.clone(), num_terms: target });
            (p, q, t)
        }
        // Cold: compute from `start`.
        None => {
            let (p, q, t) = range_bs(start, target);
            *slot = Some(CachedState { p: p.clone(), q: q.clone(), t: t.clone(), num_terms: target });
            (p, q, t)
        }
    }
}
```

### `MathCache` — the public type

```rust
/// An opt-in cache for mathematical constants, keyed by base `B`.
///
/// Holds exact binary-splitting tree state so that repeated calls at increasing
/// precision *extend* prior work instead of recomputing from scratch.
///
/// Owned per-thread; fill-on-miss via interior mutability. `Send + !Sync`.
/// To share one cache across threads, wrap it: `Arc<Mutex<MathCache<B>>>`.
///
/// `Context` and `FBig` are unaffected — `Context::pi()` etc. still recompute
/// from scratch. `MathCache` is purely additive.
#[derive(Debug, Default)]
pub struct MathCache<const B: Word = 2> {
    inner: RefCell<ConstCache>,
}
// Safety: all fields are Send (UBig/IBig/plain data). !Sync via RefCell.
// MathCache is therefore Send + !Sync: single-thread ownership, but movable
// between threads and wrappable in Arc<Mutex<..>> for sharing.

impl<const B: Word> MathCache<B> {
    pub const fn new() -> Self {
        Self { inner: RefCell::new(ConstCache::new()) }
    }

    // pi / ln2 / ln10 / ln_base live in the Integration section below; each calls
    // `extend_or_compute` on the matching `ConstCache` field, e.g.
    // `extend_or_compute(&mut cache.pi, 0, required_terms, chudnovsky_bs)`.
    // The RefCell borrow spans the binary-splitting compute, which is fine for a
    // per-thread cache; under `Arc<Mutex<MathCache<B>>>` the lock spans the compute
    // (acceptable — release-around-compute is a straightforward change if needed).
}
```

Callers pass the series `start` index to `extend_or_compute`: `0` for π (indexed
from 0) and `1` for L(n) (whose `k=0` term `1/n` is pulled out, so the tree spans
`[1, N)`).

### `Debug`

Manual `Debug` for `MathCache`/`ConstCache` prints each series slot →
`(num_terms, bit-length of P/Q/T)` — **not** the full big-integers (a cached π at
1000 digits would otherwise dump megabytes).

## API surface

```rust
impl<const B: Word> MathCache<B> {
    pub const fn new() -> Self;

    /// π at `precision` base-`B` digits, rounded per `R`. Extends prior π state.
    pub fn pi<R: Round>(&self, precision: usize) -> Rounded<FBig<R, B>>;

    /// ln(2), ln(10), ln(B) — reuse cached L(n) sub-series.
    pub fn ln2<R: Round>(&self, precision: usize) -> FBig<R, B>;
    pub fn ln10<R: Round>(&self, precision: usize) -> FBig<R, B>;
    pub fn ln_base<R: Round>(&self, precision: usize) -> FBig<R, B>;
}
```

Usage:

```rust
use dashu_float::{MathCache, DBig};
let cache: MathCache<10> = MathCache::new();
let _pi100  = cache.pi::<HalfAway>(100);   // computes from scratch
let _pi1000 = cache.pi::<HalfAway>(1000);  // extends the 100-digit state
```

`Context::pi`, `Context::ln2`, `FBig::pi`, etc. are **unchanged** and recomputed
from scratch. Trig/exp/etc. that currently call `Context::pi` internally keep doing
so; a future `MathCache::sin`/`cos` (which reuse `pi` for argument reduction) can
be added later — but most float functions do **not** benefit from the cache, so it
is deliberately not threaded through the whole API.

## Integration

### `MathCache::pi`

```rust
impl<const B: Word> MathCache<B> {
    pub fn pi<R: Round>(&self, precision: usize) -> Rounded<FBig<R, B>> {
        crate::error::assert_limited_precision(precision);

        let bits = bits_for_precision::<B>(precision); // = ceil(precision * log2(B))
        let required_terms = (bits * 100 / 4708) + 1;   // ~14.18 digits/term

        let (_p, q, t) = {
            let mut cache = self.inner.borrow_mut();
            extend_or_compute(&mut cache.pi, 0, required_terms, chudnovsky_bs)
        };

        // Finalize: π = 426880·√10005·Q / T  (identical to Context::pi)
        let guard_bits = required_terms.ilog2() as usize + 32;
        let work_precision = work_precision_for_bits::<B>(bits + guard_bits);
        let work = Context::<R>::new(work_precision);
        let q_f = work.convert_int::<B>(q.into()).value();
        let t_f = work.convert_int::<B>(t).value();
        let sqrt_10005 = work.sqrt(&work.convert_int::<B>(10005.into()).value().repr).value();
        let c = work.convert_int::<B>(426_880.into()).value();
        ((c * sqrt_10005 * q_f) / t_f).with_precision(precision)
    }
}
```

`chudnovsky_bs` is the existing free function in `consts.rs` (unchanged), now also
used by the cache. Add an `a >= b` guard returning `(1, 1, 0)` so
`extend_or_compute`'s extend path never underflows.

### `iacoth_bs` — ratio-form binary splitting (replaces the iterative loop)

```rust
/// Binary splitting for L(n) = Σ_{k≥0} 1/(n^{2k+1}(2k+1)) over [1, N).
///
/// Term ratio (k≥1): rₖ/rₖ₋₁ = pₖ/qₖ with pₖ = 2k-1, qₖ = (2k+1)·n².
/// The k=0 term r₀ = 1/n is pulled out; L(n) = (Q + T)/(n·Q).
///
/// Using the ratio form (not 1/qₖ) keeps Q = Π(2k+1)·n² at O(p) digits; each leaf
/// multiplies only small integers (n² computed once), no `pow` per leaf.
fn iacoth_bs(n: u32, a: usize, b: usize) -> (UBig, UBig, IBig) {
    if a >= b {
        return (UBig::ONE, UBig::ONE, IBig::ZERO); // identity
    }
    if b - a == 1 {
        // leaf at k = a (a ≥ 1): (pₐ, qₐ, pₐ)
        let pk = UBig::from(2 * a as u64 - 1);
        let qk = pk.clone() + UBig::from(2);               // 2a+1
        let qk = qk * (UBig::from(n) * UBig::from(n));     // (2a+1)·n²
        return (pk.clone(), qk, IBig::from(pk));           // (pₐ, qₐ, pₐ)
    }
    let mid = (a + b) / 2;
    let (pl, ql, tl) = iacoth_bs(n, a, mid);
    let (pr, qr, tr) = iacoth_bs(n, mid, b);
    merge(&pl, &ql, &tl, &pr, &qr, &tr) // universal merge
}
```

### `MathCache::ln2` / `ln10` / `ln_base`

```rust
impl<const B: Word> MathCache<B> {
    /// L(n) at `precision` digits, extending its cached series state.
    fn iacoth<R: Round>(&self, n: u32, precision: usize) -> FBig<R, B> {
        // terms until rₖ < B^{-p}: (2k+1)·log_B(n) > p
        let log_b_n = (n as f32).log2() / (B as f32).log2();
        let required_terms = ((precision as f32) / (2.0 * log_b_n)).ceil() as usize + 8;

        let (_p, q, t) = {
            let mut cache = self.inner.borrow_mut();
            let slot = match n {
                6 => &mut cache.iacoth_6,
                9 => &mut cache.iacoth_9,
                99 => &mut cache.iacoth_99,
                _ => unreachable!("iacoth only caches n ∈ {{6, 9, 99}}"),
            };
            extend_or_compute(slot, 1, required_terms, |a, b| iacoth_bs(n, a, b))
        };

        // L(n) = (Q + T) / (n·Q)
        let guard = (precision as f32).log2() as usize / (B as f32).log2() as usize + 2;
        let work = Context::<R>::new(precision + guard);
        let num   = work.convert_int::<B>(q.clone() + t).value();
        let denom = work.convert_int::<B>(IBig::from(n) * &q).value();
        (num / denom).value()
    }

    pub fn ln2<R: Round>(&self, precision: usize) -> FBig<R, B> {
        let n6  = self.iacoth::<R>(6, precision);
        let n99 = self.iacoth::<R>(99, precision);
        FBig::from(4) * n6 + FBig::from(2) * n99
    }

    pub fn ln10<R: Round>(&self, precision: usize) -> FBig<R, B> {
        FBig::from(3) * self.ln2::<R>(precision) + FBig::from(2) * self.iacoth::<R>(9, precision)
    }

    pub fn ln_base<R: Round>(&self, precision: usize) -> FBig<R, B> {
        match B {
            2 => self.ln2::<R>(precision),
            10 => self.ln10::<R>(precision),
            b if b.is_power_of_two() => self.ln2::<R>(precision) * b.trailing_zeros() as usize,
            _ => /* ln(B) via Context::ln on Repr::BASE, cached by precision */ todo!(),
        }
    }
}
```

Note: `Context::iacoth` (used by `Context::ln`) should switch to the same
`iacoth_bs` helper — this is the asymptotic speedup from TODO P19/P20, and removes
a second implementation. The existing exact-significand fixtures in `log.rs`
(`test_iacoth`, `test_ln2_ln10`) serve as regression guards for the rewrite.

## What does NOT change

| Item | Status |
|------|--------|
| `Context` | **Unchanged** — still `#[derive(Clone, Copy)]`, `Send + Sync` |
| `FBig` | **Unchanged** — still `Send + Sync`; `context()` still `const fn -> Context<R>` by value |
| `Context::max`, all binary-op call sites | **Unchanged** — no `&ref` churn |
| `Context::pi`, `Context::ln`, `FBig::pi` | **Unchanged** — recompute from scratch (no cache) |
| `chudnovsky_bs` | Reused as-is (add `a >= b` guard) |
| Migration footprint | **Zero** on existing code |

The only change to existing files: `Context::iacoth` switches from its iterative
loop to `iacoth_bs` (performance; behavior pinned by existing tests). Everything
else is the new `cache.rs` module plus a re-export.

## `no_std` and threading

- Uses only `core::cell::RefCell` and `alloc` (already declared in
  `float/src/lib.rs:66`) — no `BTreeMap`/dictionary anywhere. **No `std` feature gate.**
- `MathCache: Send + !Sync` (via `RefCell`): own one per thread, fill on miss.
- Cross-thread sharing: `Arc<Mutex<MathCache<B>>>` (user wraps; standard idiom).
  No bespoke `export`/`import` API, no `unsafe impl`.
- Base is a const generic param, so the old `bound_base`/`check_base` panic
  machinery is gone entirely.

## Extension semantics (recap)

Each series slot keeps only its **largest** state `(P, Q, T, K)`. A request for
`N` terms:

- `N ≤ K`: reuse the cached state as-is. The K-term sum is a *superset* of the
  N-term sum (the extra tail lies below the requested precision's rounding floor),
  so finalizing at the requested precision and rounding down yields the correct
  result — no recompute, no storage change.
- `N > K`: compute the right half `binary_split(K, N) → (Pᵣ, Qᵣ, Tᵣ)`, merge
  `P' = P·Pᵣ, Q' = Q·Qᵣ, T' = T·Qᵣ + P·Tᵣ`, and replace the slot with
  `(P', Q', T', N)`.

All intermediates are exact integers; the merge is an algebraic identity.

## Implementation order

### Phase 1 — Cache skeleton (no behavior change)
1. Add `float/src/math/cache.rs`: `MathCache<B>`, `ConstCache` (flattened fields),
   `CachedState`, `merge`, `extend_or_compute`, manual `Debug`.
2. Re-export `MathCache` from the crate. Add a `pi`-only path.
3. Add unit tests: cache miss → full compute; lower precision after a higher one
   → reuses cached state (no recompute); higher precision after a lower one
   → extends and is bit-identical to a from-scratch compute.

### Phase 2 — π
4. Wire `MathCache::pi` through `extend_or_compute(&mut cache.pi, 0, _, chudnovsky_bs)`;
   add the `a >= b` guard to `chudnovsky_bs`.
5. Test 100→1000 digit extension matches `FBig::pi(1000)`.

### Phase 3 — iacoth (ratio form), ln2/ln10/ln_base
6. Implement `iacoth_bs` (ratio form above); switch `Context::iacoth` to use it.
7. Add `MathCache::iacoth` / `ln2` / `ln10` / `ln_base`.
8. **Regression:** existing `test_iacoth` / `test_ln2_ln10` fixtures must still pass
   unchanged; add cross-checks `MathCache::pi/ln2/ln10` vs `Context::` equivalents
   at several bases/precisions.

### Phase 4 — Hardening
9. Bench iacoth BS vs the old iterative loop (expect a win at high precision);
   tune `GUARD` and the leaf-vs-block threshold.
10. Optionally cache `√10005` and finalized values by precision for O(1) repeats.
11. (Future) `MathCache::sin`/`cos` reusing cached `pi` for argument reduction.

## Documentation, changelog, tests

- **`float/CHANGELOG.md`** — add under `## Unreleased` → `### Add`: the `MathCache`
  type and its methods; under `### Change`: `Context::iacoth` now uses binary
  splitting.
- **Docs** — `MathCache` and every public method get `# Examples` with runnable
  code (per `AGENTS.md`). Document `Send + !Sync` and the `Arc<Mutex<..>>` pattern.
- **Tests** — algorithm tests live in `cache.rs` (`#[cfg(test)] mod tests`), per
  `AGENTS.md`; cross-cutting API tests can go under `tests/`.

## Open questions (resolved)

1. **Embed in `Context`?** → **No.** Standalone `MathCache<B>`; `Context`/`FBig`
   stay `Copy + Send + Sync` with zero migration.
2. **Base on the type?** → **Yes**, `MathCache<const B: Word>`. Fixes the base per
   cache; removes base-mismatch checking; return type is naturally `FBig<R, B>`.
3. **Rounding mode on the type?** → **No.** Series state is rounding-independent;
   methods are generic `pi::<R>(_)` so one cache serves all modes.
4. **Eviction?** None initially (typical use: a few precision steps, KB–MB each).
   LRU can be added later.
5. **iacoth merge / base case?** → Ratio form, universal merge (see "Bugs fixed").
