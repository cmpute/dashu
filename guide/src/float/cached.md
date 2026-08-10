The [`CachedFBig`] type is an [`FBig`] that carries a shared handle to a
`Rc<RefCell<ConstCache>>`. The cache stores exact binary-splitting state for
mathematical constants (π, ln2, ln10), so that transcendental operations
(`ln`, `exp`, `sin`, `cos`, …, `pi`) reuse and progressively extend prior
work instead of recomputing from scratch.

## Creation

A `CachedFBig` is created by attaching a cache handle to an `FBig`:

```rust
use std::rc::Rc;
use core::cell::RefCell;
use dashu::float::{CachedFBig, ConstCache, FBig, Repr, Context};

let cache = Rc::new(RefCell::new(ConstCache::new()));

// From an FBig
let a = FBig::ONE.into_cached(cache.clone());

// From raw parts with a fresh cache
let b = CachedFBig::<_, 10>::with_cache(
    Repr::new(1234.into(), -3),
    Context::new(50),
);
```

Use `From<FBig> for CachedFBig` for one-off conversions (it creates a fresh
empty cache):

```rust
let c: CachedFBig = FBig::from(3u8).into();
```

To drop the cache and get back a plain `FBig`, use `into_fbig()` or the
`From<CachedFBig> for FBig` trait:

```rust
let plain: FBig = cached.into();  // or cached.into_fbig()
```

## Cache sharing

Binary operations between `CachedFBig` values preserve the cache handle in
the result: `(a + b).ln().exp()` keeps extending the same cache throughout.
When two operands carry different cache handles, the **left-hand side** cache
is preserved. For `FBig op CachedFBig`, the `CachedFBig` operand's cache is
preserved regardless of which side it is on.

Operations with plain `FBig` and primitives (`u8`, `i32`, `UBig`, etc.) also
work and preserve the `CachedFBig` operand's cache:

```rust
let cached = CachedFBig::<_, 10>::with_cache(
    Repr::new(2.into(), 0), Context::new(20),
);
let result = cached + 3u8;    // CachedFBig, cache preserved
let result = 10i32 * cached;  // CachedFBig, cache preserved
```

## Inspecting and clearing the cache

Use `cache()` to borrow the cache read-only and inspect its size:

```rust
let terms = cached.cache().total_terms();
let words = cached.cache().total_words();
```

Call `clear_cache()` to free all cached big-integer memory. The next
transcendental operation will recompute constants from scratch:

```rust
cached.clear_cache();
assert_eq!(cached.cache().total_terms(), 0);
```

## More constructors and accessors

Beyond `into_cached` / `with_cache` / `From<FBig>`, `CachedFBig` mirrors the rest of `FBig`'s construction surface while preserving the cache handle:

- `from_parts(significand, exponent)` — build from a significand and exponent, with a fresh cache.
- `with_rounding::<NewR>()` — change the rounding mode, keeping the cache handle.
- `as_fbig()` — borrow the inner `FBig` immutably (cheap; no cache detach).
- `from_repr(repr, context, cache)` / `into_repr()` — the raw-repr constructor/destructor that share a specific cache handle.

## Computing constants directly

The cache stores exact binary-splitting state for the constants π, ln2, and ln10, so the methods that produce them reuse and progressively extend prior work rather than recomputing from scratch. On `CachedFBig`, π is a single call:

```rust
use std::rc::Rc;
use core::cell::RefCell;
use dashu::float::{CachedFBig, ConstCache};
use dashu::float::round::mode::HalfAway;

let cache = Rc::new(RefCell::new(ConstCache::new()));
let _pi = CachedFBig::<HalfAway, 10>::pi(100, &cache);
// a later, higher-precision call extends the same cached state instead of restarting
let _pi_more = CachedFBig::<HalfAway, 10>::pi(1000, &cache);
```

You can also drive a bare `ConstCache` directly, without a `CachedFBig` — useful when you want the constants but not the per-value wrapper. The methods are generic over base and rounding mode, and a single cache serves any base:

```rust
use dashu::float::ConstCache;
use dashu::float::round::mode::HalfAway;

let mut cache = ConstCache::new();
let pi = cache.pi::<10, HalfAway>(100).value();       // computes from scratch
let pi_1000 = cache.pi::<10, HalfAway>(1000).value(); // extends the cached state
let ln2 = cache.ln2::<10, HalfAway>(100);
let ln10 = cache.ln10::<10, HalfAway>(100);
```

`ln_base::<B, R>(precision)` dispatches to the cached ln2 / ln10 when `B` is 2 or 10 (or a power of two), and falls back to a direct `ln(B)` otherwise.

## Thread safety

`CachedFBig` carries its cache as `Rc<RefCell<ConstCache>>`, so it is **`!Send + !Sync`** — a cached value cannot move across threads. `FBig` itself stays `Copy + Send + Sync` (which is why `static_fbig!` keeps working); only the cached wrapper is non-thread-safe. `ConstCache` is a plain struct of big integers and is itself `Send + Sync`, so to share one cache across threads, wrap a `ConstCache` (or a `CachedFBig`) in `Arc<Mutex<ConstCache>>`. The underlying `Context` methods accept `Option<&mut ConstCache>` regardless of the container, so this needs no API change.

## Worked example: reusing constants across a chain

Because every value-producing operation preserves the cache handle, a chain of transcendentals reuses the same constants throughout. Building several results from one shared handle pays for each constant once:

```rust
use std::rc::Rc;
use core::cell::RefCell;
use dashu::float::{CachedFBig, ConstCache, Context, Repr};
use dashu::float::round::mode::HalfAway;

type F = CachedFBig<HalfAway, 10>;
let cache = Rc::new(RefCell::new(ConstCache::new()));

// π is computed into the shared cache...
let _pi_50 = F::pi(50, &cache);
// ...and a later, higher-precision call extends it instead of restarting
let _pi_1000 = F::pi(1000, &cache);

// an arithmetic chain built on the same handle keeps it end to end
let a = F::from_repr(Repr::new(2.into(), 0), Context::new(50), cache.clone());
let b = F::from_repr(Repr::new(3.into(), 0), Context::new(50), cache.clone());
let _ = (a + b).ln().exp();

assert!(cache.borrow().total_terms() > 0);
```

## Complex numbers: `CachedCBig`

The complex type [`CachedCBig`](https://docs.rs/dashu-cmplx/latest/dashu_cmplx/struct.CachedCBig.html)
is the exact twin of `CachedFBig`: it wraps a
[`CBig`](https://docs.rs/dashu-cmplx/latest/dashu_cmplx/struct.CBig.html) plus the same shared
`Rc<RefCell<ConstCache>>` handle, and threads it through the complex transcendentals (`ln`, `exp`,
`sin`/`cos`/`tan`/`sin_cos`, `asin`/`acos`/`atan`, `powf`, `arg`). The complex transcendentals are
built entirely from real `FBig` operations, so **the same `ConstCache` (π, ln2, ln10) is reused
unchanged** — there are no complex-specific constants to store.

```rust
use dashu::complex::{CBig, CachedCBig};
use dashu::float::FBig;

// build a cached 1+1i from a plain CBig (fresh cache)
let z = CachedCBig::from(CBig::from_parts(FBig::from(1), FBig::from(1)));

// ln / exp reuse the shared real-constant cache end to end
let _ = z.clone().ln().exp();
```

`CachedCBig` mirrors `CBig`'s full always-on surface (formatting, ordering, conversions, the binary
operators including cross-type ops against both `CBig` and `FBig`, `Neg`/`Inverse`, `Sum`/`Product`),
and is `!Send + !Sync` just like `CachedFBig` (so `CBig` stays `Send + Sync` and `static_cbig!` is
unaffected). One intentional divergence: `CachedCBig::into_parts` returns `(CachedFBig, CachedFBig)`
**sharing the handle**, so transcendentals on either part stay cached — distinct from
`CBig::into_parts`, which returns `(FBig, FBig)`. Third-party traits (serde/num-traits/num-order/
num-complex/rand) are not mirrored; reach them via `.as_cbig()`.

## The `Fast*` aliases

For transcendental-heavy code, the meta-crate exposes the cached variants under short aliases so the
faster type is easy to reach by name:

| Alias | Type | Notes |
|-------|------|-------|
| [`FastReal`](https://docs.rs/dashu/latest/dashu/index.html#fastreal) | `dashu_float::CachedFBig` | base 2, Zero — fast `Real` |
| [`FastDecimal`](https://docs.rs/dashu/latest/dashu/index.html#fastdecimal) | `dashu_float::CachedFBig<HalfAway, 10>` | fast `Decimal` |
| [`FastComplex`](https://docs.rs/dashu/latest/dashu/index.html#fastcomplex) | `dashu_cmplx::CachedCBig` | base 2, Zero — fast `Complex` |

All three are `!Send + !Sync`; the non-cached `Real`/`Decimal`/`Complex` remain the `Send + Sync`
baseline.
