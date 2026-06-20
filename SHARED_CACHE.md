# Shared MathCache via Context

## Goal

Make existing `FBig` and `Context` methods (`ln`, `exp`, `pi`, `sin`, `convert_base`,
`to_f64`, etc.) automatically benefit from a shared `MathCache` — zero API duplication,
zero new methods on `FBig`.

## Approach

Embed an optional `Rc<RefCell<ConstCache>>` handle in `Context`. All internal
methods that compute expensive constants (π, ln2, ln10, acoth series) check the
cache before recomputing from scratch. Internal work contexts inherit the cache
via `Clone`, so a single `with_cache()` call covers the entire operation tree.

## Changes to Context

### repr.rs

- Remove `Copy` from `Context`; keep `Clone`.
- Add `cache: Option<Rc<RefCell<ConstCache>>>` field.
- Add builder:

```rust
impl<R: Round> Context<R> {
    pub fn with_cache(mut self, cache: &Rc<RefCell<ConstCache>>) -> Self {
        self.cache = Some(cache.clone());
        self
    }
}
```

- `Context::max` changes to take `&self, &Self` (the 16 callers pass references
  instead of copying — zero `.clone()` overhead).

### fbig.rs

- `FBig::context()` returns `&Context<R>` instead of `Context<R>` (keeps `const`).
- Internal constructors (`Self::new(repr, self.context)`) add `.clone()` — 5 sites.
  These are real ownership transfers: a new FBig owns its own Context.
- `FBig::clone` adds `.clone()` on the context field.

### math/mod.rs

- `FpResult::value/ok` change `*context` to `context.clone()` — 2 sites.

## How internal methods use the cache

Each constant-source method on `Context` checks the cache before computing:

```rust
impl<R: Round> Context<R> {
    fn ln2<const B: Word>(&self) -> FBig<R, B> {
        if let Some(ref c) = self.cache {
            return c.borrow().ln2::<B, R>(self.precision);
        }
        // existing from-scratch path (unchanged)
        4 * self.iacoth(6.into()) + 2 * self.iacoth(99.into())
    }

    fn ln_base<const B: Word>(&self) -> FBig<R, B> {
        if let Some(ref c) = self.cache {
            return c.borrow().ln_base::<B, R>(self.precision);
        }
        // existing from-scratch path (unchanged)
    }

    pub fn pi<const B: Word>(&self) -> Rounded<FBig<R, B>> {
        if let Some(ref c) = self.cache {
            return c.borrow().pi::<B, R>(self.precision);
        }
        // existing from-scratch path (unchanged)
    }

    pub fn exp<const B: Word>(&self, x: &Repr<B>) -> Rounded<FBig<R, B>> {
        // exp_internal already calls ln_base() — which now checks cache.
        // No changes needed inside exp_internal itself.
        self.exp_internal(x)
    }
}
```

The three key touchpoints are `ln2`, `ln_base`, and `pi`. Everything else
(`ln`, `exp`, `powf`, `convert_base`, trig reduction, etc.) flows through
these three and benefits transitively — no additional code.

### Why this covers everything

| Constant | Checked in... | Benefits these public functions |
|---|---|---|
| ln(2) | `Context::ln2()` | `ln`, `ln_1p`, `ln10`, `ln_base` |
| ln(B) | `Context::ln_base()` | `exp`, `exp_m1`, `powf`, `convert_base`, `to_f64`, `to_f32`, `to_decimal`, `to_binary`, `with_base` |
| π | `Context::pi()` | `sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `atan2` |

### How internal work contexts propagate the cache

Internal methods create work contexts at elevated precision by cloning `self`
rather than calling `Context::new()`:

```rust
// Before: fresh context — no cache
let work_context = Context::<R>::new(2 * self.precision);

// After: inherit cache from self
let mut work_context = self.clone();
work_context.precision = 2 * self.precision;
```

Or add a helper:

```rust
impl<R: Round> Context<R> {
    fn with_precision(&self, precision: usize) -> Self {
        Self { precision, cache: self.cache.clone(), _marker: PhantomData }
    }
}
```

Then all `Context::new(precision)` calls inside `Context` methods change to
`self.with_precision(precision)`. This ensures nested work contexts inherit
the cache and all sub-computations benefit.

## Changes to MathCache

`MathCache` becomes a thin `Rc<RefCell<ConstCache>>` factory. The existing
public methods (`pi`, `ln2`, `ln10`, `ln_base`) stay — they're the API for:

1. Direct use without `Context`: `cache.pi::<10, HalfAway>(100)`
2. Internal use by `Context` methods: called via `c.borrow().ln2(...)` 

### math/cache.rs

```rust
impl MathCache {
    /// Consume self and return a shareable handle for Context::with_cache.
    pub fn into_handle(self) -> Rc<RefCell<ConstCache>> {
        Rc::new(self.inner)
    }
}
```

## User experience

### No cache (identical to today)

```rust
let x = DBig::from_str("3.14")?.ln();  // Context::new(), no cache
```

### With cache — per-thread

```rust
let cache = MathCache::new();
let handle = cache.into_handle();

let ctx = Context::<HalfAway>::new(100).with_cache(&handle);
let x = FBig::new(parse_repr, ctx);
let ln = x.ln();       // cached ln2 inside
let exp = x.exp();     // cached ln_base inside
let f64 = x.to_f64();  // cached ln(B) + ln(2) inside convert_base
```

### With cache — across an entire thread (std only)

```rust
thread_local! {
    static MATH_CACHE: RefCell<Option<Rc<RefCell<ConstCache>>>> = const { RefCell::new(None) };
}

impl<R: Round> Context<R> {
    fn ln2<const B: Word>(&self) -> FBig<R, B> {
        // Check the handle stored in Context first, then fall back to thread-local
        let cache = self.cache.as_ref().or_else(|| {
            MATH_CACHE.with(|tc| tc.borrow().as_ref())
        });
        if let Some(c) = cache {
            return c.borrow().ln2::<B, R>(self.precision);
        }
        // from-scratch path
    }
}
```

This makes every `FBig` operation automatically cached after a single
`MATH_CACHE.set(Some(handle))` at program start — zero API changes.

## Files changed

| File | Lines changed | Nature |
|---|---|---|
| `float/src/repr.rs` | ~10 | `Context`: drop `Copy`, add `cache` field, add `with_cache`, `with_precision`, change `max` to references |
| `float/src/fbig.rs` | ~8 | 5 `.clone()` additions, `context()` returns `&Context` |
| `float/src/math/cache.rs` | ~5 | `into_handle()` method |
| `float/src/math/mod.rs` | ~2 | `FpResult` `.clone()` |
| `float/src/log.rs` | ~15 | `ln2`/`ln_base` cache check, `with_precision` propagation |
| `float/src/exp.rs` | ~5 | `with_precision` propagation |
| `float/src/convert.rs` | ~5 | `with_precision` propagation |
| `float/src/math/trig.rs` | ~5 | `pi` cache check |
| `float/src/add.rs` | ~4 | `Context::max` takes `&` |
| `float/src/div.rs` | ~6 | `Context::max` takes `&` |
| `float/src/mul.rs` | ~4 | `Context::max` takes `&` |
| `float/src/exp.rs` | ~1 | `Context::max` takes `&` |
| **Total** | **~70** | Mechanical |

## What this does NOT do

- No new methods on `FBig` — existing API is automatically accelerated
- No lifetime parameter on `FBig` — the `Rc` handle is owned, not borrowed
- No `std` requirement — `Rc`/`RefCell` are `alloc`, same tier as the rest of
  the crate's `no_std` support
- No `Send + Sync` regression — `Rc` is `!Send`, but `Context` was already
  `!Sync` (and `Send` is preserved by wrapping in `Arc<Mutex<..>>` externally)
