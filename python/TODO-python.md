# Implementation Plan — dashu Python Package

This document is the concrete implementation plan for the dashu Python bindings. Each section describes *how* to implement the feature, not just what's missing.

---

## Architecture

### Global constant cache + bare FBig/CBig

**Decision: `FPy`/`DPy`/`CPy` wrap the *bare* `FBig`/`CBig`, and the module owns a single global `ConstCache`. Transcendentals borrow that cache and call the panic-free `Context` layer directly, mapping `FpError` to Python exceptions.**

We deliberately do **not** wrap `CachedFBig`/`CachedCBig`. Those types bundle `Rc<RefCell<ConstCache>>` inside the value, which (a) makes the wrapper `!Send + !Sync`, forcing `#[pyclass(unsendable)]` and breaking free-threaded Python, and (b) hides the cache behind a `pub(crate)` field, so external code can't reach the panic-free `Context::sin(repr, cache)` path and is stuck with the panicking convenience methods. Putting the cache in the module (not the value) sidesteps both problems.

| Python type | Rust inner type | Cache | Send? |
|---|---|---|---|
| `UPy` | `UBig` | — | yes |
| `IPy` | `IBig` | — | yes |
| `RPy` | `RBig` | — | yes |
| `FPy` | `FBig<mode::Zero, 2>` | global | **yes** |
| `DPy` | `FBig<mode::HalfAway, 10>` | global | **yes** |
| `CPy` | `CBig<mode::Zero, 2>` | global | **yes** |

All six wrappers are `Send + Sync`, so **none need `#[pyclass(unsendable)]`** and the binding is free-threaded-Python compatible.

**Why a global cache:** every `sin`/`cos`/`exp`/`ln` recomputes π/ln2/ln10 from scratch (O(N²)) unless a `ConstCache` is threaded in. The cache is base-free and rounding-mode-free (just big-integer constants), so **one cache serves FPy (base 2), DPy (base 10), and CPy (complex) at once**, and accumulates precision across calls. Keeping it in the module means the *value* stays a plain, `Send`, operator-bearing `FBig`/`CBig`.

**Backing store:** `thread_local! { static CONST_CACHE: RefCell<ConstCache> }`, initialized with `ConstCache::new()` (a `pub const fn` on a `Send + Sync` struct). Thread-local = zero locking, zero contention; under free-threaded Python each thread gets its own cache and recomputes constants once. A `with_cache(|c| ...)` helper borrows it mutably for one call.

**Error mapping (replaces the panicking `Context::unwrap_fp`):** the `Context` layer returns `FpResult<Rounded<T>>` (= `Result`). We map it ourselves:
- `Err(Overflow(sign))` → `Repr::infinity_with_sign(sign)` (graceful, matches Python float)
- `Err(Underflow(sign))` → `Repr::zero_with_sign(sign)` (graceful)
- `Err(InfiniteInput)` → `ValueError("arithmetic with infinity")`
- `Err(OutOfDomain)` → `ValueError("math domain error")` (matches `math.sqrt(-1)`)
- `Err(Indeterminate)` → `ZeroDivisionError` (this is `0/0`)

This is *strictly better* than the convenience layer, which `panic!`s on the last three and surfaces in Python as `pyo3_runtime.PanicException` (a `BaseException` subclass that escapes `except Exception:` and crashes the session).

**Scope of the fix:** this fully fixes **transcendentals** (`sin`, `exp`, `ln`, `sqrt`, `powf`, trig/hyperbolic — all take `cache: Option<&mut ConstCache>` at the `Context` layer). **Arithmetic** operators on bare `FBig`/`CBig` still `panic!` on infinite input (e.g. `exp(1000) + 1`, since `exp(1000)→+∞` then `+` asserts finite) and on `0/0`. Those are rarer than transcendental domain errors, so arithmetic keeps the operator impls for the MVP; routing it through the `Context` layer (or guarding) is a follow-up.

### Key patterns to follow

**UniInput dispatch** — `python/src/convert.rs`: The `UniInput` enum has 11 variants covering all Python numeric types (Uint, Int, BUint, BInt, OBInt, Float, BFloat, BDecimal, OBDecimal, BRational, OBRational). Its `FromPyObject` impl already extracts any Python number into the right variant. We add conversion methods (`into_fpy`, `into_dpy`, `into_rpy`, `into_cpy`) that collapse any variant into the target dashu type, making arithmetic functions simple one-liners.

**Comparison** — `num_order::NumOrd` trait provides cross-type comparison for all dashu types: FBig↔UBig, FBig↔IBig, FBig↔RBig, FBig↔f64, RBig↔UBig, RBig↔IBig, etc. The `__richcmp__` pattern is already established in UPy/IPy: `op.matches(self.0.num_cmp(&rhs))`.

**Arithmetic macros** — The existing `impl_binops!` macro in `int.rs` generates forward/reverse dispatch functions by matching each `UniInput` variant. For FPy/DPy/RPy/CPy we create simpler versions that first convert the operand to the target type via `into_fpy()`/etc, then call the Rust operator (bare FBig/CBig keep all operator trait impls).

**Transcendental dispatch** — transcendentals do NOT use the convenience methods; they call `self.0.context().sin(self.0.repr(), cache)` through `with_cache`, then map the `FpResult`. This is the single most important pattern in the binding — it's what gives clean Python exceptions instead of panics.

### Rust API facts (verified)

```
ConstCache  (dashu_float::ConstCache; Send + Sync; pub const fn new())
  Owns memoized pi/ln2/ln10 binary-splitting state. Base-free, rounding-free.
  One instance serves FPy (base 2), DPy (base 10), CPy (complex) simultaneously.
  pub fn clear(&mut self), pub fn total_terms(&self), pub fn total_words(&self).

FBig<R, B>  (the value FPy/DPy wrap; Send + Sync)
  Public accessors: .repr() -> &Repr<B>,  .context() -> Context<R> (Copy),  .into_repr() -> Repr<B>.
  Context layer (panic-free, returns FpResult<Rounded<FBig>>):
    .add/.sub/.mul/.div/.inv/.sqrt/.cbrt/.nth_root/.powi/.hypot  -- no cache param
    .exp/.exp_m1/.ln/.ln_1p/.sin/.cos/.tan/.asin/.acos/.atan/.atan2/.powf/.sinh/.cosh/.tanh/.asinh/.acosh/.atanh
        -- all take cache: Option<&mut ConstCache>
    .pi(precision, cache) -> Rounded<FBig>
  Convenience layer (FBig::sin etc.) -- PANICS on InfiniteInput/OutOfDomain/Indeterminate; DO NOT USE in bindings.

CBig<R, B>  (the value CPy wraps; Send + Sync)
  Public accessors: .re() -> &Repr<B>, .im() -> &Repr<B>, .context() -> dashu_cmplx::Context<R> (Copy).
  Complex Context layer (panic-free, returns CfpResult<R,B>):
    .sqrt/.powi  -- no cache param
    .ln/.exp/.sin/.cos/.tan/.asin/.acos/.atan/.powf  -- all take cache: Option<&mut ConstCache>
    .abs/.arg/.norm  -- return real FBig (abs via hypot; arg threads cache).
  Convenience layer (CBig::sin etc.) -- PANICS; DO NOT USE in bindings.

RBig methods (uncached):
  .numerator() -> &IBig, .denominator() -> &UBig
  .trunc() -> IBig, .floor() -> IBig, .ceil() -> IBig, .round() -> IBig  (NOT RBig!)
  .fract() -> RBig
  .split_at_point() -> (IBig, RBig)
  .sqr(), .cubic(), .pow(n: usize)
  .is_int(), .sign() -> Sign, .signum() -> RBig

UBig methods (uncached):
  .sqrt() -> UBig, .cbrt() -> UBig, .nth_root(n) -> UBig
  .sqr() -> UBig, .cubic() -> UBig
  .ilog(&UBig) -> usize
  .count_ones() -> usize, .count_zeros() -> Option<usize>
  .trailing_zeros() -> Option<usize>, .trailing_ones() -> Option<usize>
  .is_power_of_two() -> bool, .next_power_of_two() -> UBig
  .is_multiple_of(&UBig) -> bool, .remove(&UBig) -> Option<usize>

IBig methods (uncached):
  .sqrt() -> UBig (always unsigned!), .cbrt() -> IBig, .nth_root(n) -> IBig
  .sqr() -> UBig (always unsigned!), .cubic() -> IBig
  .ilog(&UBig) -> usize  (takes &UBig, not &IBig!)
  .trailing_zeros() -> Option<usize>, .trailing_ones() -> Option<usize>
  .sign() -> Sign, .signum() -> IBig
  .into_parts() -> (Sign, UBig)
  IBig::from_parts(Sign, UBig) -> IBig

Traits (need `use` imports):
  dashu_base::ring::Gcd::gcd(a, b)       -- UBig.gcd(UBig) -> UBig
  dashu_base::ring::ExtendedGcd::gcd_ext(a, b) -> (UBig, IBig, IBig)
  dashu_base::DivEuclid::div_euclid(a, b), DivRemEuclid::div_rem_euclid(a, b)

IBig::is_negative()/is_positive() -- NOT inherent! Use self.sign() == Sign::Negative instead.

Conversions (int/float/rational — bare FBig, since FPy/DPy wrap bare FBig):
  FBig::try_from(f64) -- base 2 ONLY (FPy works; DPy does not)
  FBig::from(UBig), FBig::from(IBig) -- infallible, any base
  FBig::try_from(RBig) -- exact only, returns Err(ConversionError) on precision loss
  RBig::try_from(FBig) -- exact only
  RBig::try_from(f64) -- exact only (f64 has limited precision anyway)
  DPy from f64 -- go through string: format!("{:e}", x) then FBig::from_str

Approximation::value() extracts T from both Exact(T) and Inexact(T, E) variants.
```

---

## Step 0: Upgrade PyO3 from 0.20 to 0.29

**Files: `python/Cargo.toml`, `python/pyproject.toml`**

### Rationale

PyO3 0.20 uses the old `gil-refs` API (`&PyAny`, `&PyList`, `.into_py(py)`). Version 0.23 introduced the modern `Bound` API and `IntoPyObject`, and 0.29 is the current latest. The MSRV constraint does **not** apply to `dashu-python` (per AGENTS.md — only core crates are bounded by MSRV).

### 0a: Update Cargo.toml — dependencies and metadata

```toml
[dependencies]
# Add complex numbers support:
dashu-cmplx = { version = "0.4.5", path = "../complex", features = ["std", "num-order"] }
```

```toml
[dependencies.pyo3]
# Change:
version = "0.20"
features = ["extension-module"]
# To:
version = "0.29"
# "extension-module" feature is deprecated in 0.28+ and removed in 0.29.
# maturin handles linking automatically.
```

Also bump `rust-version` and `edition`:
```toml
# Change:
rust-version = "1.68"
edition = "2021"
# To:
rust-version = "1.85"
edition = "2024"
```

Remove `categories = ["mathematics", "no-std"]` — `no-std` is misleading for a Python binding. Replace with `categories = ["mathematics", "science"]`.

### 0b: Update pyproject.toml

PyO3 0.26+ dropped Python 3.7 support:
```toml
# Change:
requires-python = ">=3.7"
# To:
requires-python = ">=3.8"
```

### 0c: `#[pymodule]` signature — `lib.rs`

```rust
// Before (0.20):
fn dashu(_py: Python<'_>, m: &PyModule) -> PyResult<()> {

// After (0.29):
fn dashu(m: &Bound<'_, PyModule>) -> PyResult<()> {
```

Remove the now-unused `_py: Python<'_>` parameter.

### 0d: `#[pyclass]` enum — `types.rs`

PyO3 0.22+ requires explicit `eq` for cross-type enum comparison:

```rust
// Before:
#[pyclass]
pub enum PySign { Positive, Negative }

// After:
#[pyclass(eq, eq_int)]
#[derive(PartialEq, Clone, Copy)]
pub enum PySign { Positive, Negative }
```

### 0d2: Wrapper types store bare FBig/CBig (all Send) — `types.rs`

Per the Architecture decision, `FPy`/`DPy`/`CPy` wrap the **bare** `FBig`/`CBig` (not the cached variants). The constant cache lives in the module (Step 2c), not in the value. All wrappers are `Send + Sync`, so **none use `unsendable`**.

```rust
// types.rs — final shape (after Steps 0d2 + 11):
type FBig2   = dashu_float::FBig<dashu_float::round::mode::Zero, 2>;
type DBig10  = dashu_float::FBig<dashu_float::round::mode::HalfAway, 10>;
type CBig2   = dashu_cmplx::CBig<dashu_float::round::mode::Zero, 2>;

#[pyclass(name = "FBig")] pub struct FPy(pub FBig2);
#[pyclass(name = "DBig")] pub struct DPy(pub DBig10);
#[pyclass(name = "CBig")] pub struct CPy(pub CBig2);   // Step 11

#[pyclass(name = "UBig")]  pub struct UPy(pub dashu_int::UBig);   // unchanged
#[pyclass(name = "IBig")]  pub struct IPy(pub dashu_int::IBig);   // unchanged
#[pyclass(name = "RBig")]  pub struct RPy(pub dashu_ratio::RBig); // unchanged
```

The `UniInput` enum's `BFloat`/`BDecimal` variants keep field names `BFloat(PyRef<'a, FPy>)` etc. — only the inner type behind `FPy` changes (from cached to bare). Add `BComplex(PyRef<'a, CPy>)` for Step 11.

The `UniInput` enum's `BFloat`/`BDecimal` variants change their `PyRef` target types to the new cached wrappers (the field names `BFloat(PyRef<'a, FPy>)` etc. stay the same; only the inner type behind `FPy` changes). Add `BComplex(PyRef<'a, CPy>)` for Step 11.

### 0e: `FromPyObject` for `UniInput` — `convert.rs`

PyO3 0.27 restructured `FromPyObject` with dual lifetimes and `Borrowed`:

```rust
// Before (0.20):
impl<'source> FromPyObject<'source> for UniInput<'source> {
    fn extract(ob: &'source PyAny) -> PyResult<Self> {

// After (0.29):
impl<'a, 'py> FromPyObject<'a, 'py> for UniInput<'a> {
    type Error = PyErr;
    fn extract(ob: Borrowed<'a, 'py, PyAny>) -> PyResult<Self> {
```

`Borrowed<'a, 'py, PyAny>` derefs to `&Bound<'py, PyAny>`, so `ob.is_instance_of::<PyLong>()`, `ob.py()`, etc. all work as before.

**Extracting `PyRef` from `Borrowed`:** The pattern changes from:
```rust
// 0.20:
<PyRef<'source, UPy> as FromPyObject>::extract(ob)
```
To (0.29):
```rust
ob.extract::<PyRef<'a, UPy>>()
```

### 0f: `.cast()` replaces `.downcast()` — `int.rs`, `words.rs`

PyO3 0.27 renamed downcast methods:

```rust
// Before (0.20):
index.downcast::<PySlice>()?
// After (0.29):
index.cast::<PySlice>()?
```

Also: `PySlice::indices()` now expects `isize` instead of `i32` for the length parameter in some contexts. The current code passes `.try_into().map_err(...)` — this should be reviewed.

### 0g: `.into_py(py)` → `.into_pyobject(py)` — all files

PyO3 0.23 replaced `IntoPy` with `IntoPyObject`. The new trait returns `PyResult`:

```rust
// Before (0.20):
UPy(value).into_py(py)
// After (0.29):
UPy(value).into_pyobject(py)
// Returns PyResult<Bound<'py, PyAny>>
// Use .unbind() to get Py<PyAny> (= PyObject):
UPy(value).into_pyobject(py).map(|b| b.unbind())
```

For functions that currently return `PyObject`, convert to `PyResult<PyObject>` and use:

```rust
// Before:
fn upy_add(lhs: &UPy, rhs: UniInput<'_>, py: Python) -> PyObject {
    // ...
    UPy(result).into_py(py)
}
// After:
fn upy_add(lhs: &UPy, rhs: UniInput<'_>, py: Python) -> PyResult<PyObject> {
    // ...
    Ok(UPy(result).into_py_any(py)?)
}
```

`into_py_any(py)` is shorthand for `.into_pyobject(py).map(|b| b.unbind())` — it returns `PyResult<Py<PyAny>>` (= `PyResult<PyObject>`).

For `__repr__`, `__str__` returning `String` — no change needed. For `__hash__` returning `u64` — no change needed. PyO3 handles those primitive types natively.

### 0h: `intern!` macro

The macro returns `&Bound<'py, PyString>`. Usage like `py.import(intern!(py, "decimal"))?` works transparently (Bound derefs). No code changes needed.

### 0i: `wrap_pyfunction!` 

No change needed — stable across 0.20→0.29.

### Summary of API changes (0.20 → 0.29)

| Pattern | 0.20 | 0.29 |
|---------|------|------|
| `#[pymodule]` | `fn(py, m: &PyModule)` | `fn(m: &Bound<PyModule>)` |
| `FromPyObject` | `extract(ob: &'source PyAny)` | `extract(ob: Borrowed<'a, 'py, PyAny>)` with `type Error` |
| `.downcast::<T>()` | present | `.cast::<T>()` |
| Value→Python | `.into_py(py)` | `.into_py_any(py)?` or `.into_pyobject(py)` |
| `#[pyclass]` enum | `#[pyclass]` | `#[pyclass(eq, eq_int)]` |
| `extension-module` | required in features | removed (maturin handles it) |
| rust-version | 1.68 | 1.83 (PyO3 0.29 MSRV) |
| requires-python | ≥3.7 | ≥3.8 (Python 3.7 dropped in 0.26+) |

---

## Step 1: Fix `todo!()` panics in existing code

**File: `python/src/int.rs`**

### 1a: Fix `upy_mod` and `ipy_mod`

Both `upy_mod` (line 140) and `ipy_mod` (line 150) have `_ => todo!()` for the float/rational variants. Fill the missing arms by converting to the corresponding float/rational type:

```rust
fn upy_mod(lhs: &UPy, rhs: UniInput<'_>, py: Python) -> PyResult<PyObject> {
    let result: Py<PyAny> = match rhs {
        UniInput::Uint(x) => UPy((&lhs.0).rem(x).into()).into_py(py),
        UniInput::Int(x) => UPy((&lhs.0).rem(IBig::from(x))).into_py(py),
        UniInput::BUint(x) => UPy((&lhs.0).rem(&x.0)).into_py(py),
        UniInput::BInt(x) => UPy((&lhs.0).rem(&x.0)).into_py(py),
        UniInput::OBInt(x) => UPy((&lhs.0).rem(x)).into_py(py),
        UniInput::Float(x) => FPy((&lhs.0).rem(FBig::try_from(x).unwrap())).into_py(py),
        UniInput::BFloat(x) => FPy((&lhs.0).rem(&x.0)).into_py(py),
        UniInput::BDecimal(x) => DPy((&lhs.0).rem(&x.0)).into_py(py),
        UniInput::OBDecimal(x) => DPy((&lhs.0).rem(x)).into_py(py),
        UniInput::BRational(x) => RPy((&lhs.0).rem(&x.0)).into_py(py),
        UniInput::OBRational(x) => RPy((&lhs.0).rem(x)).into_py(py),
    };
    Ok(result)
}
```

Same pattern for `ipy_mod`. Change return type from `PyObject` to `PyResult<PyObject>` and update the `__mod__` wrappers at lines 493 and 655 to propagate with `?`.

### 1b: Fix `ipy_pow` todo!() branches

Replace the three `_ => todo!()` arms with proper error returns:

```rust
// For modulus parameter (line 174):
_ => return Err(PyTypeError::new_err("modulus must be an integer")),

// For exponent in modulus branch (line 186):
_ => return Err(PyTypeError::new_err("modular exponentiation requires an integer exponent")),

// For exponent in non-modulus branch (line 191):
_ => return Err(PyTypeError::new_err("integer power requires a non-negative integer exponent")),
```

---

## Step 2: Add `UniInput` conversion helpers

**File: `python/src/convert.rs`**

Add four methods to `impl<'a> UniInput<'a>` that convert any numeric variant to the target dashu type. Since `FPy`/`DPy`/`CPy` wrap **bare** `FBig`/`CBig`, these are the plain `FBig`/`CBig` conversions (no cache is attached — the cache lives in the module, see Step 2c).

### `into_fpy(self) -> PyResult<FPy>`

`FPy` wraps `FBig<Zero,2>`.

| Input variant | Conversion |
|---|---|
| `Uint(x)` | `FBig::from(x)` |
| `Int(x)` | `FBig::from(IBig::from(x))` |
| `BUint(x)` | `FBig::from(x.0.clone())` |
| `BInt(x)` | `FBig::from(x.0.clone())` |
| `OBInt(x)` | `FBig::from(x)` |
| `Float(x)` | `FBig::try_from(x)` — map error to PyValueError |
| `BFloat(x)` | `x.0.clone()` (already a `FPy` = bare FBig) |
| `BDecimal/OBDecimal` | `Err(PyTypeError("decimal cannot be mixed with binary float; convert explicitly"))` |
| `BRational(x)` | `FBig::try_from(x.0.clone())` — map ConversionError to PyTypeError |
| `OBRational(x)` | `FBig::try_from(x)` — same |

### `into_dpy(self) -> PyResult<DPy>`

`DPy` wraps `FBig<HalfAway,10>`. For `Float(x)` use `format!("{:e}", x)` → `FBig::from_str` (no direct `TryFrom<f64>` for base 10). For `BFloat` return type error. For `BRational`/`OBRational` use `FBig::try_from` (DBig is `FBig<HalfAway,10>`).

### `into_rpy(self) -> PyResult<RPy>`

`RPy` wraps `RBig` (no transcendental ops).

| Input variant | Conversion |
|---|---|
| `Uint(x)` | `RBig::from(x)` |
| `Int(x)` | `RBig::from(IBig::from(x))` |
| `BUint(x)` | `RBig::from(x.0.clone())` |
| `BInt(x)` | `RBig::from(x.0.clone())` |
| `OBInt(x)` | `RBig::from(x)` |
| `Float(x)` | `RBig::try_from(x)` — map error to PyValueError |
| `BFloat(x)` | `RBig::try_from(x.0.clone())` |
| `BDecimal(x)` | `RBig::try_from(x.0.clone())` |
| `OBDecimal(x)` | `RBig::try_from(x)` |
| `BRational(x)` | `x.0.clone()` |
| `OBRational(x)` | `x` |

### `into_cpy(self) -> PyResult<CPy>` (Step 11)

`CPy` wraps bare `CBig<Zero,2>`. `CBig::From<FBig>` and `From<UBig/IBig>` exist; ints/floats embed as `z + 0i`. `BFloat`/`BDecimal` convert via their inner FBig (`x.0.clone()` then `CBig::from`). Cross-base (FPy↔DPy) and rational→complex raise type errors for the MVP.

Add imports at top of `convert.rs`:
```rust
use crate::types::{FPy, CPy};
use dashu_float::FBig;
use dashu_cmplx::CBig;
use dashu_float::round::mode;
use dashu_base::ConversionError;
use std::str::FromStr;
```

---

## Step 2b: Broaden `__new__` to accept any Python number

**Problem:** today `FBig(...)`, `DBig(...)`, `RBig(...)` only accept their "native" Python type or a string. `FBig(7)`, `FBig(Decimal("1.5"))`, `FBig(Fraction(1,3))`, `DBig(1.5)`, `RBig(1.5)` all fail at construction. The `into_*` helpers (Step 2) fixed the *arithmetic* case but not construction. Workaround today is `auto(x)`, but the type constructors should be permissive too.

**Design:** keep arithmetic dispatch strict (matches Python — `1.5 + Decimal("0.5")` raises `TypeError`), but make `__new__` permissive by giving it its own conversion path. Add permissive `construct_*` helpers to `UniInput` (or as free functions in `convert.rs`) that differ from `into_*` only in the cross-type rules:

```rust
impl<'a> UniInput<'a> {
    /// Permissive construction of FBig from any Python number (for __new__).
    /// Unlike into_fpy, this ACCEPTS Decimal/Fraction by routing through string / RBig.
    pub fn construct_fbig(self) -> PyResult<FBig> {
        match self {
            Self::Uint(x)    => Ok(FBig::from(x)),
            Self::Int(x)     => Ok(FBig::from(IBig::from(x))),
            Self::BUint(x)   => Ok(FBig::from(x.0.clone())),
            Self::BInt(x)    => Ok(FBig::from(x.0.clone())),
            Self::OBInt(x)   => Ok(FBig::from(x)),
            Self::Float(x)   => FBig::try_from(x).map_err(conversion_error_to_py),
            Self::BFloat(x)  => Ok(x.0.clone()),
            // Decimal/Fraction -> round-trip through string (Decimal.__format__ gives
            // scientific notation; Fraction via RBig::try_from is exact-or-error).
            Self::BDecimal(x) | Self::OBDecimal(_) => {
                let s = decimal_to_str(&self)?;   // helper: format Decimal as decimal string
                FBig::from_str(&s).map_err(parse_error_to_py)
            }
            Self::BRational(x) => FBig::try_from(x.0.clone()).map_err(conversion_error_to_py),
            Self::OBRational(x) => FBig::try_from(x).map_err(conversion_error_to_py),
        }
    }
    // construct_dbig, construct_rbig, construct_cbig: same permissive pattern
}
```

Then each `__new__` becomes (FPy example):

```rust
#[new]
fn __new__(ob: &PyAny, radix: Option<u32>) -> PyResult<Self> {
    if let Ok(s) = ob.extract::<String>() {
        // string with optional radix (existing path)
        let f = match radix {
            Some(r) => FBig::from_str_radix(s, r),
            None    => FBig::from_str(s),
        };
        return Ok(FPy(f.map_err(parse_error_to_py)?));
    }
    if radix.is_some() {
        return Err(PyTypeError::new_err("radix only valid with string input"));
    }
    if let Ok(other) = <PyRef<Self> as FromPyObject>::extract(ob) {
        return Ok(FPy(other.0.clone()));   // copy
    }
    // any other Python number -> permissive construction
    Ok(FPy(UniInput::extract(ob)?.construct_fbig()?))
}
```

**Per-type construction matrix** (what each `__new__` accepts after this step):

| `__new__` accepts | `FBig` | `DBig` | `RBig` | `CBig` |
|---|---|---|---|---|
| `str` (± radix) | ✅ | ✅ | ✅ | ✅ (`"a+bi"`) |
| same-type copy | ✅ | ✅ | ✅ | ✅ |
| `int` | ✅ `FBig::from` | ✅ `DBig::from` | ✅ `RBig::from` | ✅ `CBig::from` (z+0i) |
| `float` | ✅ `try_from` | ✅ via `{:e}` str | ✅ `try_from` (exact) | ✅ re+im |
| `Decimal` | ✅ via str | ✅ `parse_to_dbig` | ✅ via RBig | via FBig |
| `Fraction` | ✅ `try_from` (exact) | ✅ `try_from` (exact) | ✅ `parse_to_rbig` | via FBig |
| `(re, im)` tuple | — | — | `(num, den)` → from_parts | ✅ `from_parts` |
| `complex` | — | — | — | ✅ |

**Note on exact-vs-lossy:** `Decimal`/`Fraction` → `FBig`/`DBig` via `try_from` is **exact-only** (errors on precision loss, e.g. `1/3` → base 2). For lossy correctly-rounded conversion, users should call the explicit `to_float()` method (Step 6c). The constructor stays strict to avoid silent precision loss — consistent with how `int(1.5)` is explicit in Python.

**`decimal_to_str` helper:** format a `decimal.Decimal` as a plain decimal string. Reuse the existing `parse_to_dbig` round-trip logic in reverse (Decimal → `__format__` → string). This is the same path `DPy::unwrap` already uses.

---

## Step 2c: Global constant cache — new file `python/src/cache.rs`

This module owns the single `ConstCache` shared by all float/decimal/complex transcendentals, and provides the `FpError → PyErr` mapping that replaces the panicking `Context::unwrap_fp`.

```rust
// python/src/cache.rs
use std::cell::RefCell;
use dashu_float::{ConstCache, Repr, Sign};
use dashu_float::error::{FpError, Rounding};
use pyo3::exceptions::{PyValueError, PyZeroDivisionError};
use pyo3::PyErr;

thread_local! {
    /// One cache per thread — zero locking, zero contention. Base-free, so it serves
    /// FPy (base 2), DPy (base 10), and CPy (complex) simultaneously. Accumulates
    /// precision across calls; never needs explicit invalidation.
    static CONST_CACHE: RefCell<ConstCache> = RefCell::new(ConstCache::new());
}

/// Borrow the thread-local cache mutably for one transcendental call.
pub fn with_cache<R>(f: impl FnOnce(&mut ConstCache) -> R) -> R {
    CONST_CACHE.with(|c| f(&mut c.borrow_mut()))
}

/// Optional Python-facing handle for cache inspection/clearing (see Step 2c-bottom).
#[pyclass(name = "Cache")]
pub struct PyCache;
#[pymethods]
impl PyCache {
    /// Drop all memoized constants (rarely needed — the cache only grows usefully).
    #[staticmethod]
    fn clear() { with_cache(|c| c.clear()); }
    #[staticmethod]
    fn total_terms() -> usize { with_cache(|c| c.total_terms()) }
    #[staticmethod]
    fn total_words() -> usize { with_cache(|c| c.total_words()) }
}

/// Map a float `FpResult<Rounded<FBig>>` to a `PyResult<FBig>`:
/// Overflow/Underflow → signed ∞/0 (graceful, like Python float);
/// InfiniteInput/OutOfDomain → ValueError; Indeterminate → ZeroDivisionError.
/// `val` extracts the FBig from the Rounded wrapper (Exact|Inexact).
pub fn unwrap_float<B>(
    res: dashu_float::FpResult<dashu_float::FBig<dashu_float::round::mode::Zero, B>>,
) -> PyResult<dashu_float::FBig<dashu_float::round::mode::Zero, B>>
where
    usize: dashu_float::Base<B>,
{
    use dashu_base::Approximation;
    match res {
        Ok(rounded) => Ok(rounded.value()),
        Err(FpError::Overflow(sign)) => Ok(dashu_float::FBig::from_repr(
            Repr::infinity_with_sign(sign), /* context */ unreachable!())),
        // NOTE: the overflow/underflow branches need the value's context to rebuild
        // an FBig; in practice build them via Repr + the caller's context — see the
        // per-call helpers in float.rs/complex.rs which receive `self.0.context()`.
        Err(FpError::Underflow(sign)) => /* signed zero */ todo!(),
        Err(FpError::InfiniteInput) => Err(PyValueError::new_err("arithmetic with infinity")),
        Err(FpError::OutOfDomain)   => Err(PyValueError::new_err("math domain error")),
        Err(FpError::Indeterminate) => Err(PyZeroDivisionError::new_err("indeterminate form (0/0)")),
    }
}
```

**Rebuilding ∞/0 with the right context:** `FBig::from_repr` needs a `Context`. Since each call site has `self.0.context()`, the cleanest shape is a *context-aware* mapper `unwrap_float(res, ctx) -> PyResult<FBig>` rather than the simplified one above. Concretely, the transcendental methods in `float.rs` look like:

```rust
fn sin(&self) -> PyResult<FPy> {
    let ctx = self.0.context();
    let res = with_cache(|c| ctx.sin(self.0.repr(), Some(c))); // FpResult<Rounded<FBig>>
    Ok(FPy(unwrap_float(res, ctx)?))
}
```

For **complex**, the parallel `unwrap_complex(res, ctx) -> PyResult<CBig>` maps the same `FpError` variants but rebuilds `CBig::overflow(ctx, sign)` / `CBig::underflow(ctx, sign)` on Overflow/Underflow (these constructors exist on CBig — see `complex/src/repr.rs`).

**Register in `lib.rs`:** `m.add_class::<cache::PyCache>()?;` so Python users can call `dashu.Cache.clear()`.

### Why this design

- **Panic-free transcendentals:** every `sin`/`cos`/`exp`/`ln`/`sqrt`/`powf`/trig/hyperbolic goes through `Context::<op>(repr, cache) -> FpResult`, never the panicking convenience method.
- **Caching preserved:** `with_cache` threads the *same* growing cache into every call, exactly like `CachedFBig` would — but without `Rc<RefCell<..>>` baked into the value.
- **`Send` preserved:** `FPy`/`DPy`/`CPy` hold bare `FBig`/`CBig`, so no `#[pyclass(unsendable)]`, and the binding works under free-threaded Python (each thread gets its own thread-local cache).

---

## Step 3: Add arithmetic operators to FPy and DPy

**File: `python/src/float.rs`**

### 3a: Comparison

Add `__richcmp__` using the same pattern as UPy/IPy (int.rs lines 248–263):

```rust
use num_order::NumOrd;
use pyo3::basic::CompareOp;

#[pymethods]
impl FPy {
    fn __richcmp__(&self, other: UniInput<'_>, op: CompareOp) -> bool {
        let order = match other {
            UniInput::Uint(x) => self.0.num_cmp(&x),
            UniInput::Int(x) => self.0.num_cmp(&x),
            UniInput::BUint(x) => self.0.num_cmp(&x.0),
            UniInput::BInt(x) => self.0.num_cmp(&x.0),
            UniInput::OBInt(x) => self.0.num_cmp(&x),
            UniInput::Float(x) => self.0.num_cmp(&x),
            UniInput::BFloat(x) => self.0.cmp(&x.0),
            UniInput::BDecimal(x) => self.0.num_cmp(&x.0),
            UniInput::OBDecimal(x) => self.0.num_cmp(&x),
            UniInput::BRational(x) => self.0.num_cmp(&x.0),
            UniInput::OBRational(x) => self.0.num_cmp(&x),
        };
        op.matches(order)
    }
}
```

Same for DPy (DBig is FBig<HalfAway, 10> so NumOrd works identically).

### 3b: Bool

```rust
fn __bool__(&self) -> bool {
    !self.0.repr().is_zero()
}
```

### 3c: Arithmetic macro

Create a helper macro that uses `into_fpy()` for forward/reverse dispatch.
**Note:** PyO3 0.24 uses `.into_py_any(py)?` instead of `.into_py(py)`:

```rust
macro_rules! impl_fpy_binops {
    // Commutative (add, mul) — __radd__ and __rmul__ reuse the forward function
    ($method:ident, $rs_method:ident) => {
        fn $method(lhs: &FPy, rhs: UniInput<'_>, py: Python) -> PyResult<PyObject> {
            let rhs = rhs.into_fpy()?;
            Ok(FPy((&lhs.0).$rs_method(&rhs.0)).into_py_any(py)?)
        }
    };
    // Non-commutative (sub, div, rem) — also generate reverse function
    ($method:ident, $rev_method:ident, $rs_method:ident) => {
        impl_fpy_binops!($method, $rs_method);
        fn $rev_method(lhs: UniInput<'_>, rhs: &FPy, py: Python) -> PyResult<PyObject> {
            let lhs = lhs.into_fpy()?;
            Ok(FPy(lhs.0.$rs_method(&rhs.0)).into_py_any(py)?)
        }
    };
}

impl_fpy_binops!(fpy_add, add);
impl_fpy_binops!(fpy_mul, mul);
impl_fpy_binops!(fpy_sub, fpy_rsub, sub);
impl_fpy_binops!(fpy_div, fpy_rdiv, div);
impl_fpy_binops!(fpy_mod, fpy_rmod, rem);
```

Wire in `#[pymethods] impl FPy`:

```rust
fn __add__(&self, other: UniInput<'_>, py: Python) -> PyResult<PyObject> { fpy_add(self, other, py) }
fn __radd__(&self, other: UniInput<'_>, py: Python) -> PyResult<PyObject> { fpy_add(self, other, py) }
fn __sub__(&self, other: UniInput<'_>, py: Python) -> PyResult<PyObject> { fpy_sub(self, other, py) }
fn __rsub__(&self, other: UniInput<'_>, py: Python) -> PyResult<PyObject> { fpy_rsub(other, self, py) }
fn __mul__(&self, other: UniInput<'_>, py: Python) -> PyResult<PyObject> { fpy_mul(self, other, py) }
fn __rmul__(&self, other: UniInput<'_>, py: Python) -> PyResult<PyObject> { fpy_mul(self, other, py) }
fn __truediv__(&self, other: UniInput<'_>, py: Python) -> PyResult<PyObject> { fpy_div(self, other, py) }
fn __rtruediv__(&self, other: UniInput<'_>, py: Python) -> PyResult<PyObject> { fpy_rdiv(other, self, py) }
fn __mod__(&self, other: UniInput<'_>, py: Python) -> PyResult<PyObject> { fpy_mod(self, other, py) }
fn __rmod__(&self, other: UniInput<'_>, py: Python) -> PyResult<PyObject> { fpy_rmod(other, self, py) }
fn __neg__(&self) -> FPy { FPy(-&self.0) }
fn __pos__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> { slf }
fn __abs__(&self) -> FPy { FPy(self.0.abs()) }
```

Same pattern for DPy: create an identical macro using `into_dpy()` instead of `into_fpy()`.

### 3d: Imports to add at top of `float.rs`

```rust
use num_order::NumOrd;
use pyo3::basic::CompareOp;
use crate::convert::parse_error_to_py;
```

---

## Step 4: Add arithmetic operators to RPy

**File: `python/src/ratio.rs`**

Same pattern as Step 3c but using `into_rpy()`:

```rust
macro_rules! impl_rpy_binops {
    ($method:ident, $rs_method:ident) => {
        fn $method(lhs: &RPy, rhs: UniInput<'_>, py: Python) -> PyResult<PyObject> {
            let rhs = rhs.into_rpy()?;
            Ok(RPy((&lhs.0).$rs_method(&rhs.0)).into_py_any(py)?)
        }
    };
    ($method:ident, $rev_method:ident, $rs_method:ident) => {
        impl_rpy_binops!($method, $rs_method);
        fn $rev_method(lhs: UniInput<'_>, rhs: &RPy, py: Python) -> PyResult<PyObject> {
            let lhs = lhs.into_rpy()?;
            Ok(RPy(lhs.0.$rs_method(&rhs.0)).into_py_any(py)?)
        }
    };
}

impl_rpy_binops!(rpy_add, add);
impl_rpy_binops!(rpy_mul, mul);
impl_rpy_binops!(rpy_sub, rpy_rsub, sub);
impl_rpy_binops!(rpy_div, rpy_rdiv, div);
impl_rpy_binops!(rpy_mod, rpy_rmod, rem);
```

Wire `__add__`/`__radd__`/`__sub__`/`__rsub__`/`__mul__`/`__rmul__`/`__truediv__`/`__rtruediv__`/`__mod__`/`__rmod__`/`__neg__`/`__pos__`/`__abs__`.

Add `__richcmp__` (same pattern as FPy but using RBig's NumOrd impls) and `__bool__`.

Add imports:
```rust
use num_order::NumOrd;
use pyo3::basic::CompareOp;
use std::ops::{Add, Sub, Mul, Div, Rem};
```

---

## Step 5: Complete integer type methods

**File: `python/src/int.rs`**

### 5a: Add number theory + bit ops to UPy

Add to `#[pymethods] impl UPy`:

```rust
// Number theory
fn sqrt(&self) -> Self { UPy(self.0.sqrt()) }
fn cbrt(&self) -> Self { UPy(self.0.cbrt()) }
fn nth_root(&self, n: usize) -> Self { UPy(self.0.nth_root(n)) }
fn sqr(&self) -> Self { UPy(self.0.sqr()) }
fn cubic(&self) -> Self { UPy(self.0.cubic()) }
fn ilog(&self, base: &Self) -> PyResult<usize> {
    if base.0 <= UBig::ONE {
        return Err(PyValueError::new_err("base must be > 1"));
    }
    Ok(self.0.ilog(&base.0))
}
fn is_multiple_of(&self, divisor: &Self) -> bool { self.0.is_multiple_of(&divisor.0) }
fn remove(&mut self, factor: &Self) -> PyResult<usize> {
    self.0.remove(&factor.0).ok_or_else(|| PyValueError::new_err("factor does not divide this number"))
}
fn gcd(&self, other: &Self) -> Self {
    use dashu_base::ring::Gcd;
    UPy(Gcd::gcd(&self.0, &other.0))
}
fn gcd_ext(&self, other: &Self) -> (Self, IPy, IPy) {
    use dashu_base::ring::ExtendedGcd;
    let (g, s, t) = ExtendedGcd::gcd_ext(&self.0, &other.0);
    (UPy(g), IPy(s), IPy(t))
}

// Bit operations
fn count_ones(&self) -> usize { self.0.count_ones() }
fn count_zeros(&self) -> Option<usize> { self.0.count_zeros() }
fn trailing_zeros(&self) -> Option<usize> { self.0.trailing_zeros() }
fn trailing_ones(&self) -> Option<usize> { self.0.trailing_ones() }
fn is_power_of_two(&self) -> bool { self.0.is_power_of_two() }
fn next_power_of_two(slf: PyRef<'_, Self>) -> Self { UPy(slf.0.clone().next_power_of_two()) }

// Accessors
fn is_one(&self) -> bool { self.0.is_one() }
#[staticmethod]
fn ones(n: usize) -> Self { UPy(UBig::ones(n)) }
```

### 5b: Add missing operators to UPy

```rust
// Floor division
fn __floordiv__(&self, other: UniInput<'_>, py: Python) -> PyResult<PyObject> {
    use dashu_base::ring::DivEuclid;
    match other {
        UniInput::Uint(x) => Ok(UPy(self.0.div_euclid(&UBig::from(x))).into_py(py)),
        UniInput::BUint(x) => Ok(UPy(self.0.div_euclid(&x.0)).into_py(py)),
        UniInput::Int(x) => Ok(IPy(self.0.as_ibig().clone().div_euclid(IBig::from(x))).into_py(py)),
        UniInput::BInt(x) => Ok(IPy(self.0.as_ibig().clone().div_euclid(x.0.clone())).into_py(py)),
        UniInput::OBInt(x) => Ok(IPy(self.0.as_ibig().clone().div_euclid(x)).into_py(py)),
        _ => Err(PyTypeError::new_err("floor division requires an integer divisor")),
    }
}
fn __rfloordiv__(&self, other: UniInput<'_>, py: Python) -> PyResult<PyObject> {
    // other // self — reverse dispatch
    self.__floordiv__(other, py) // reuse for now; will be handled correctly via radd pattern
    // NOTE: to do this properly, need a reverse dispatch like rsub.
    // For MVP, raise NotImplementedError for complex cases
    todo!("proper __rfloordiv__ dispatch")
}

// Divmod
fn __divmod__(&self, other: UniInput<'_>, py: Python) -> PyResult<PyObject> {
    use dashu_base::ring::DivRemEuclid;
    match other {
        UniInput::Uint(x) => {
            let (q, r) = self.0.div_rem_euclid(&UBig::from(x));
            Ok((UPy(q), UPy(r)).into_py(py))
        }
        UniInput::BUint(x) => {
            let (q, r) = self.0.div_rem_euclid(&x.0);
            Ok((UPy(q), UPy(r)).into_py(py))
        }
        UniInput::Int(x) => {
            let (q, r) = self.0.as_ibig().clone().div_rem_euclid(IBig::from(x));
            Ok((IPy(q), UPy(r)).into_py(py))
        }
        UniInput::BInt(x) => {
            let (q, r) = self.0.as_ibig().clone().div_rem_euclid(x.0.clone());
            Ok((IPy(q), UPy(r)).into_py(py))
        }
        UniInput::OBInt(x) => {
            let (q, r) = self.0.as_ibig().clone().div_rem_euclid(x);
            Ok((IPy(q), UPy(r)).into_py(py))
        }
        _ => Err(PyTypeError::new_err("divmod requires an integer divisor")),
    }
}
```

### 5c: Add number theory + bit ops + accessors to IPy

```rust
// Number theory (note return types — sqrt/sqr return unsigned!)
fn sqrt(&self) -> PyResult<UPy> {
    if self.0.is_negative() {
        return Err(PyValueError::new_err("cannot compute sqrt of negative number"));
    }
    Ok(UPy(self.0.abs().sqrt()))
}
fn cbrt(&self) -> Self { IPy(self.0.cbrt()) }
fn nth_root(&self, n: usize) -> PyResult<Self> {
    if n % 2 == 0 && self.0.sign() == Sign::Negative {
        return Err(PyValueError::new_err("cannot compute even root of negative number"));
    }
    Ok(IPy(self.0.nth_root(n)))
}
fn sqr(&self) -> UPy { UPy(self.0.sqr()) }
fn cubic(&self) -> Self { IPy(self.0.cubic()) }
fn ilog(&self, base: &UPy) -> PyResult<usize> {
    if base.0 <= UBig::ONE {
        return Err(PyValueError::new_err("base must be > 1"));
    }
    Ok(self.0.ilog(&base.0))
}

// Bit operations
fn trailing_zeros(&self) -> Option<usize> { self.0.trailing_zeros() }
fn trailing_ones(&self) -> Option<usize> { self.0.trailing_ones() }
fn __invert__(&self) -> Self { IPy(!&self.0) }

// Accessors
fn is_one(&self) -> bool { self.0.is_one() }
fn sign(&self) -> PySign {
    match self.0.sign() {
        Sign::Positive => PySign::Positive,
        Sign::Negative => PySign::Negative,
    }
}
fn signum(&self) -> Self { IPy(self.0.signum()) }
fn is_negative(&self) -> bool { self.0.sign() == Sign::Negative }
fn is_positive(&self) -> bool { self.0.sign() == Sign::Positive }
fn to_parts(&self) -> (PySign, UPy) {
    let (sign, mag) = self.0.clone().into_parts();
    let py_sign = match sign {
        Sign::Positive => PySign::Positive,
        Sign::Negative => PySign::Negative,
    };
    (py_sign, UPy(mag))
}
#[staticmethod]
fn from_parts(sign: &PySign, magnitude: &UPy) -> Self {
    let sign = match sign {
        PySign::Positive => Sign::Positive,
        PySign::Negative => Sign::Negative,
    };
    IPy(IBig::from_parts(sign, magnitude.0.clone()))
}
fn as_ubig(&self) -> Option<UPy> {
    self.0.as_ubig().cloned().map(UPy)
}
```

### 5d: IBig interop parity with UBig

Add to IPy (copy patterns from UPy at int.rs lines 379–457):

```rust
fn to_words(&self) -> PyWords {
    let (_, words) = self.0.as_sign_words();
    PyWords(words.to_vec())
}
#[staticmethod]
fn from_words(ob: &PyAny) -> PyResult<Self> {
    if let Ok(vec) = <Vec<Word> as FromPyObject>::extract(ob) {
        Ok(IPy(IBig::from_words(&vec)))
    } else if let Ok(words) = <PyRef<PyWords> as FromPyObject>::extract(ob) {
        Ok(IPy(IBig::from_words(&words.0)))
    } else {
        Err(PyTypeError::new_err("only list of integers or Words instance can be used"))
    }
}
fn to_chunks(&self, chunk_bits: usize, py: Python) -> PyResult<PyObject> {
    if chunk_bits == 0 {
        Err(PyValueError::new_err("chunk size must not be zero"))
    } else {
        let iter = self.0.to_chunks(chunk_bits).into_vec().into_iter().map(|u| UPy(u).into_py(py));
        Ok(PyTuple::new(py, iter).into_py(py))
    }
}
#[staticmethod]
fn from_chunks(chunks: &PyAny, chunk_bits: usize) -> PyResult<Self> {
    // Same pattern as UPy::from_chunks, but collect into Vec<IBig> then join
    if chunk_bits == 0 {
        return Err(PyValueError::new_err("chunk size must not be zero"));
    }
    let mut input = Vec::new();
    if let Ok(list) = chunks.downcast::<PyList>() {
        input.reserve_exact(list.len());
        for item in list {
            input.push(UniInput::extract(item)?.to_ubig()?);
        }
    } else if let Ok(tuple) = chunks.downcast::<PyTuple>() {
        input.reserve_exact(tuple.len());
        for item in tuple {
            input.push(UniInput::extract(item)?.to_ubig()?);
        }
    } else if let Ok(iter) = chunks.downcast::<PyIterator>() {
        for item in iter {
            input.push(UniInput::extract(item?)?.to_ubig()?);
        }
    } else {
        return Err(PyTypeError::new_err("chunks must be a list, tuple, or iterator"));
    }
    Ok(IPy(IBig::from_chunks(input.iter(), chunk_bits)))
}
```

Also copy the bit-slice `__getitem__`, `__setitem__`, `__delitem__` implementations from UPy (lines 269–371) to IPy, adapting from `UBig` to `IBig`. The core bit-manipulation logic uses `split_bits`, `clear_high_bits`, `set_bit`, `clear_bit` which exist on IBig as well.

### 5e: Add in-place operators to UPy/IPy

For all types, in-place operators take `&mut self`:

```rust
fn __iadd__(&mut self, other: &Self) { self.0 += &other.0; }
fn __isub__(&mut self, other: &Self) { self.0 -= &other.0; }
fn __imul__(&mut self, other: &Self) { self.0 *= &other.0; }
fn __iand__(&mut self, other: &Self) { self.0 &= &other.0; }
fn __ior__(&mut self, other: &Self) { self.0 |= &other.0; }
fn __ixor__(&mut self, other: &Self) { self.0 ^= &other.0; }
fn __ilshift__(&mut self, other: usize) { self.0 <<= other; }
fn __irshift__(&mut self, other: usize) { self.0 >>= other; }
```

Note: these are same-type-only for the MVP. Python's data model falls back to `__add__` + assignment when `__iadd__` is not defined, so cross-type in-place is handled automatically.

---

## Step 6: Float & rational predicates, conversions, rounding

**File: `python/src/float.rs`** — add to `#[pymethods] impl FPy` (and replicate for DPy):

```rust
// Predicates (repr-based)
fn is_zero(&self) -> bool { self.0.repr().is_zero() }
fn is_finite(&self) -> bool { self.0.repr().is_finite() }
fn is_infinite(&self) -> bool { self.0.repr().is_infinite() }
fn is_nan(&self) -> bool { self.0.repr().is_nan() }

// Sign
fn sign(&self) -> PySign {
    match self.0.repr().sign() {
        Sign::Positive => PySign::Positive,
        Sign::Negative => PySign::Negative,
    }
}
fn signum(&self) -> Self { FPy(self.0.signum()) }

// Rounding — direct on bare FBig (pure significand/exponent ops, no cache needed).
fn trunc(&self) -> Self { FPy(self.0.trunc()) }
fn floor(&self) -> Self { FPy(self.0.floor()) }
fn ceil(&self) -> Self { FPy(self.0.ceil()) }
fn round(&self) -> Self { FPy(self.0.round()) }
fn fract(&self) -> Self { FPy(self.0.fract()) }

// Conversion (FBig::to_int returns Rounded<IBig>)
fn to_int(&self) -> PyResult<IPy> {
    let int: IBig = self.0.clone().to_int().value().try_into()
        .map_err(|e: ConversionError| PyValueError::new_err(format!("cannot convert to integer: {}", e)))?;
    Ok(IPy(int))
}
fn __int__(&self, py: Python) -> PyResult<PyObject> {
    let ipy = self.to_int()?;
    convert_from_ibig(&ipy.0, py)
}

// Precision (FBig::with_precision -> Rounded<Self>)
fn with_precision(&self, precision: usize) -> Self {
    FPy(self.0.clone().with_precision(precision).value())
}
fn precision(&self) -> usize { self.0.context().precision() }
fn digits(&self) -> usize { self.0.digits() }

// Float-specific constructor
#[staticmethod]
fn from_parts(significand: &IPy, exponent: isize) -> Self {
    FPy(FBig::from_parts(significand.0.clone(), exponent))
}
```

**For DPy** the same methods apply — bare `FBig<HalfAway,10>` supports them identically.

### 6b: Transcendentals — route through the global cache + Context layer

This is the critical part: transcendentals do **NOT** call `self.0.sin()` (the convenience method panics on domain errors). Instead they call the panic-free `Context` layer with the module cache and map the result:

```rust
use crate::cache::{with_cache, unwrap_float};

#[pymethods]
impl FPy {
    fn sin(&self) -> PyResult<FPy> {
        let ctx = self.0.context();
        let res = with_cache(|c| ctx.sin(self.0.repr(), Some(c)));     // FpResult<Rounded<FBig>>
        Ok(FPy(unwrap_float(res, ctx)?))
    }
    fn cos(&self) -> PyResult<FPy> { /* ctx.cos(...) */ }
    fn tan(&self) -> PyResult<FPy> { /* ctx.tan(...) */ }
    fn asin(&self) -> PyResult<FPy> { /* ctx.asin(...) */ }
    fn acos(&self) -> PyResult<FPy> { /* ctx.acos(...) */ }
    fn atan(&self) -> PyResult<FPy> { /* ctx.atan(...) */ }
    fn sinh(&self) -> PyResult<FPy> { /* ctx.sinh(...) */ }
    fn cosh(&self) -> PyResult<FPy> { /* ctx.cosh(...) */ }
    fn tanh(&self) -> PyResult<FPy> { /* ctx.tanh(...) */ }
    fn asinh(&self) -> PyResult<FPy> { /* ctx.asinh(...) */ }
    fn acosh(&self) -> PyResult<FPy> { /* ctx.acosh(...) */ }
    fn atanh(&self) -> PyResult<FPy> { /* ctx.atanh(...) */ }
    fn exp(&self) -> PyResult<FPy> { /* ctx.exp(...) */ }
    fn exp_m1(&self) -> PyResult<FPy> { /* ctx.exp_m1(...) */ }
    fn ln(&self) -> PyResult<FPy> { /* ctx.ln(...) */ }
    fn ln_1p(&self) -> PyResult<FPy> { /* ctx.ln_1p(...) */ }
    // sqrt/cbrt take NO cache (they're algebraic) but still go through Context for clean errors:
    fn sqrt(&self) -> PyResult<FPy> { let ctx = self.0.context(); Ok(FPy(unwrap_float(ctx.sqrt(self.0.repr()), ctx)?)) }
    fn cbrt(&self) -> PyResult<FPy> { /* ctx.cbrt / ctx.nth_root(3, ...) */ }
    fn powf(&self, w: &Self) -> PyResult<FPy> { let ctx = self.0.context(); Ok(FPy(unwrap_float(ctx.powf(self.0.repr(), w.0.repr(), with_cache_arg), ctx)?)) }
}
```

Note `powf` needs the cache threaded into `with_cache`; the helper can be extended to `with_cache(|c| ctx.powf(a, b, Some(c)))`. Every domain error (`sqrt(-1)`, `asin(2)`, `ln(-1)`, `acosh(0.5)`, `atanh(2)`) now raises a Python `ValueError` instead of crashing the session.

**File: `python/src/ratio.rs`** — add to `#[pymethods] impl RPy`:

```rust
// Properties
#[getter]
fn numerator(&self) -> IPy { IPy(self.0.numerator().clone()) }
#[getter]
fn denominator(&self) -> UPy { UPy(self.0.denominator().clone()) }

// Predicates
fn is_int(&self) -> bool { self.0.is_int() }
fn is_one(&self) -> bool { self.0.is_one() }
fn sign(&self) -> PySign {
    match self.0.sign() {
        Sign::Positive => PySign::Positive,
        Sign::Negative => PySign::Negative,
    }
}
fn signum(&self) -> Self { RPy(self.0.signum()) }

// Rounding (return IBig — NOT RBig!)
fn trunc(&self) -> IPy { IPy(self.0.trunc()) }
fn floor(&self) -> IPy { IPy(self.0.floor()) }
fn ceil(&self) -> IPy { IPy(self.0.ceil()) }
fn round(&self) -> IPy { IPy(self.0.round()) }
fn fract(&self) -> Self { RPy(self.0.fract()) }
fn split_at_point(&self) -> (IPy, Self) {
    let (int_part, frac_part) = self.0.clone().split_at_point();
    (IPy(int_part), RPy(frac_part))
}

// Powers
fn sqr(&self) -> Self { RPy(self.0.sqr()) }
fn cubic(&self) -> Self { RPy(self.0.cubic()) }
fn pow(&self, n: usize) -> Self { RPy(self.0.pow(n)) }

// Constructor
#[staticmethod]
fn from_parts(numerator: &IPy, denominator: &UPy) -> Self {
    RPy(RBig::from_parts(numerator.0.clone(), denominator.0.clone()))
}

// Conversion
fn to_int(&self) -> IPy { IPy(self.0.trunc()) }
fn __int__(&self, py: Python) -> PyResult<PyObject> {
    let ipy = IPy(self.0.trunc());
    convert_from_ibig(&ipy.0, py)
}

// Simplification
#[staticmethod]
fn simplest_from_float(f: &FPy) -> Option<Self> {
    RBig::simplest_from_float(&f.0).map(RPy)
}
```

Add import to `float.rs`: `use dashu_base::{Sign, ConversionError};`
Add imports to `ratio.rs`: `use dashu_base::Sign;`

---

## Step 6c: Cross-type conversions (base change, float↔rational)

Expose the base-change and float↔rational conversions that already exist on bare `FBig`/`RBig`. These let users move between `FPy` (base 2), `DPy` (base 10), `RPy`, and the corresponding Python native types without round-tripping through strings.

**File: `python/src/float.rs`** — add to `FPy` and `DPy`:

```rust
// Base conversion (FBig::to_decimal / to_binary return Rounded<FBig<...>>)
fn to_decimal(&self) -> DPy { DPy(self.0.to_decimal().value()) }   // FPy(base 2) -> DPy(base 10)
fn to_binary(&self) -> FPy { FPy(self.0.to_binary().value()) }     // DPy(base 10) -> FPy(base 2); identity on FPy

// Float -> Rational (exact only; errors if the float isn't exactly representable as a fraction)
fn to_rational(&self) -> PyResult<RPy> {
    RBig::try_from(self.0.clone()).map(RPy).map_err(conversion_error_to_py)
}

// Precision/base introspection + arbitrary-radix formatting (no new Python type for base != 2,10,
// so with_base is exposed only as a string formatter via in_radix — see Step 8/__format__)
```

**File: `python/src/ratio.rs`** — add to `RPy`:

```rust
// Rational -> Float, correctly rounded (lossy). RBig::to_float::<R,B>(precision) is the
// dashu-float lossy conversion; pick base 2 (FPy) and base 10 (DPy) variants.
fn to_float(&self, precision: usize) -> FPy {
    FPy(self.0.to_float::<mode::Zero, 2>(precision).value())
}
fn to_decimal(&self, precision: usize) -> DPy {
    DPy(self.0.to_float::<mode::HalfAway, 10>(precision).value())
}
```

**Conversion summary** (after Steps 2b + 6c):

| from → to | `FPy` | `DPy` | `RPy` | Python native |
|---|---|---|---|---|
| `FPy` | — | `.to_decimal()` | `.to_rational()` (exact) | `float()`, `int()`, `.to_decimal().unwrap()`→Decimal |
| `DPy` | `.to_binary()` | — | `.to_rational()` (exact) | `float()`, `int()`, `.unwrap()`→Decimal |
| `RPy` | `.to_float(p)` (lossy) | `.to_decimal(p)` (lossy) | — | `float()`, `int()`, `.unwrap()`→Fraction |
| `UPy`/`IPy` | `FBig::from` | `DBig::from` | `RBig::from` | `int()`, `.unwrap()` |

**Notes:**
- `to_decimal`/`to_binary` return `Rounded<...>` in Rust — use `.value()` to drop the rounding flag (we don't expose per-call rounding flags to Python yet).
- `RBig::to_float` requires an explicit precision argument (rationals are generally non-terminating in base 2/10). This is the lossy counterpart to the exact `FBig → RBig` path.
- `with_base(radix)` for radix ∉ {2,10} has no Python wrapper type, so it's exposed only as a string formatter (`in_radix` / `__format__`, Step 8), not as a new numeric type.
- For `FPy → Decimal` directly: compose `f.to_decimal().unwrap()` (DPy → Decimal already exists). No separate method needed.

---

## Step 7: Create math module

**New file: `python/src/math.rs`**

```rust
use pyo3::prelude::*;
use crate::types::{FPy, DPy, UPy, IPy};

macro_rules! impl_math_func {
    ($name:ident, ($($arg:ident: $arg_ty:ty),*) -> $ret:ty, $body:expr) => {
        #[pyfunction]
        pub fn $name($($arg: $arg_ty),*) -> $ret {
            $body
        }
    };
}

// Trigonometric
impl_math_func!(sin, (x: &FPy) -> FPy, FPy(x.0.sin()));
impl_math_func!(cos, (x: &FPy) -> FPy, FPy(x.0.cos()));
impl_math_func!(tan, (x: &FPy) -> FPy, FPy(x.0.tan()));
impl_math_func!(asin, (x: &FPy) -> FPy, FPy(x.0.asin()));
impl_math_func!(acos, (x: &FPy) -> FPy, FPy(x.0.acos()));
impl_math_func!(atan, (x: &FPy) -> FPy, FPy(x.0.atan()));
#[pyfunction]
pub fn atan2(y: &FPy, x: &FPy) -> FPy { FPy(y.0.atan2(&x.0)) }
#[pyfunction]
pub fn sincos(x: &FPy) -> (FPy, FPy) {
    let (s, c) = x.0.sin_cos();
    (FPy(s), FPy(c))
}

// Hyperbolic
impl_math_func!(sinh, (x: &FPy) -> FPy, FPy(x.0.sinh()));
impl_math_func!(cosh, (x: &FPy) -> FPy, FPy(x.0.cosh()));
impl_math_func!(tanh, (x: &FPy) -> FPy, FPy(x.0.tanh()));
impl_math_func!(asinh, (x: &FPy) -> FPy, FPy(x.0.asinh()));
impl_math_func!(acosh, (x: &FPy) -> FPy, FPy(x.0.acosh()));
impl_math_func!(atanh, (x: &FPy) -> FPy, FPy(x.0.atanh()));

// Exponential and log
impl_math_func!(exp, (x: &FPy) -> FPy, FPy(x.0.exp()));
impl_math_func!(exp_m1, (x: &FPy) -> FPy, FPy(x.0.exp_m1()));
impl_math_func!(ln, (x: &FPy) -> FPy, FPy(x.0.ln()));
impl_math_func!(ln_1p, (x: &FPy) -> FPy, FPy(x.0.ln_1p()));

// Roots
impl_math_func!(sqrt, (x: &FPy) -> FPy, FPy(x.0.sqrt()));
impl_math_func!(cbrt, (x: &FPy) -> FPy, FPy(x.0.cbrt()));
#[pyfunction]
pub fn nth_root(x: &FPy, n: usize) -> FPy { FPy(x.0.nth_root(n)) }

// Integer number theory
#[pyfunction]
pub fn gcd(a: &UPy, b: &UPy) -> UPy {
    use dashu_base::ring::Gcd;
    UPy(Gcd::gcd(&a.0, &b.0))
}
#[pyfunction]
pub fn gcd_ext(a: &UPy, b: &UPy) -> (UPy, IPy, IPy) {
    use dashu_base::ring::ExtendedGcd;
    let (g, s, t) = ExtendedGcd::gcd_ext(&a.0, &b.0);
    (UPy(g), IPy(s), IPy(t))
}
#[pyfunction]
pub fn lcm(a: &UPy, b: &UPy) -> UPy {
    use dashu_base::ring::Gcd;
    UPy((&a.0 * &b.0) / Gcd::gcd(&a.0, &b.0))
}
```

Also add corresponding methods directly on FPy/DPy in `float.rs`:

```rust
// In #[pymethods] impl FPy:
fn sin(&self) -> Self { FPy(self.0.sin()) }
fn cos(&self) -> Self { FPy(self.0.cos()) }
fn tan(&self) -> Self { FPy(self.0.tan()) }
fn asin(&self) -> Self { FPy(self.0.asin()) }
fn acos(&self) -> Self { FPy(self.0.acos()) }
fn atan(&self) -> Self { FPy(self.0.atan()) }
fn atan2(&self, x: &Self) -> Self { FPy(self.0.atan2(&x.0)) }
fn sincos(&self) -> (Self, Self) { let (s, c) = self.0.sin_cos(); (FPy(s), FPy(c)) }
fn sinh(&self) -> Self { FPy(self.0.sinh()) }
fn cosh(&self) -> Self { FPy(self.0.cosh()) }
fn tanh(&self) -> Self { FPy(self.0.tanh()) }
fn asinh(&self) -> Self { FPy(self.0.asinh()) }
fn acosh(&self) -> Self { FPy(self.0.acosh()) }
fn atanh(&self) -> Self { FPy(self.0.atanh()) }
fn exp(&self) -> Self { FPy(self.0.exp()) }
fn exp_m1(&self) -> Self { FPy(self.0.exp_m1()) }
fn ln(&self) -> Self { FPy(self.0.ln()) }
fn ln_1p(&self) -> Self { FPy(self.0.ln_1p()) }
fn sqrt(&self) -> Self { FPy(self.0.sqrt()) }
fn cbrt(&self) -> Self { FPy(self.0.cbrt()) }
fn nth_root(&self, n: usize) -> Self { FPy(self.0.nth_root(n)) }
```

Same for DPy.

Register in `lib.rs`:

```rust
mod math;

// In #[pymodule] fn dashu:
m.add_function(wrap_pyfunction!(math::sin, m)?)?;
m.add_function(wrap_pyfunction!(math::cos, m)?)?;
m.add_function(wrap_pyfunction!(math::tan, m)?)?;
m.add_function(wrap_pyfunction!(math::asin, m)?)?;
m.add_function(wrap_pyfunction!(math::acos, m)?)?;
m.add_function(wrap_pyfunction!(math::atan, m)?)?;
m.add_function(wrap_pyfunction!(math::atan2, m)?)?;
m.add_function(wrap_pyfunction!(math::sincos, m)?)?;
m.add_function(wrap_pyfunction!(math::sinh, m)?)?;
m.add_function(wrap_pyfunction!(math::cosh, m)?)?;
m.add_function(wrap_pyfunction!(math::tanh, m)?)?;
m.add_function(wrap_pyfunction!(math::asinh, m)?)?;
m.add_function(wrap_pyfunction!(math::acosh, m)?)?;
m.add_function(wrap_pyfunction!(math::atanh, m)?)?;
m.add_function(wrap_pyfunction!(math::exp, m)?)?;
m.add_function(wrap_pyfunction!(math::exp_m1, m)?)?;
m.add_function(wrap_pyfunction!(math::ln, m)?)?;
m.add_function(wrap_pyfunction!(math::ln_1p, m)?)?;
m.add_function(wrap_pyfunction!(math::sqrt, m)?)?;
m.add_function(wrap_pyfunction!(math::cbrt, m)?)?;
m.add_function(wrap_pyfunction!(math::nth_root, m)?)?;
m.add_function(wrap_pyfunction!(math::gcd, m)?)?;
m.add_function(wrap_pyfunction!(math::gcd_ext, m)?)?;
m.add_function(wrap_pyfunction!(math::lcm, m)?)?;
```

---

## Step 8: Fix `__format__` from `todo!()` to minimal working

**Files: `python/src/int.rs`, `python/src/float.rs`, `python/src/ratio.rs`**

Replace each `fn __format__(&self) { todo!() }` with:

```rust
fn __format__(&self, _format_spec: &str) -> String {
    // For MVP: ignore format_spec, just delegate to Display
    format!("{}", self.0)
}
```

Full Python format mini-language parsing is deferred to Phase 6.

---

## Step 9: Update type stubs

**File: `python/dashu.pyi`**

Complete the stubs to match all newly exposed methods. Key additions:

```python
class FBig:
    def __init__(self, obj: float | str | int): ...
    def unwrap(self) -> tuple[IBig, int]: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
    def __format__(self, format_spec: str) -> str: ...
    def __hash__(self) -> int: ...
    def __float__(self) -> float: ...
    def __int__(self) -> int: ...
    def __bool__(self) -> bool: ...

    # Arithmetic
    @overload
    def __add__(self, other: UBig | IBig | FBig | DBig | RBig) -> UBig | IBig | FBig | DBig | RBig: ...
    @overload
    def __add__(self, other: int) -> FBig: ...
    @overload
    def __radd__(self, other: int) -> FBig: ...
    # ... same for __sub__, __rsub__, __mul__, __rmul__, __truediv__, __rtruediv__, __mod__, __rmod__

    def __neg__(self) -> FBig: ...
    def __pos__(self) -> FBig: ...
    def __abs__(self) -> FBig: ...

    # Comparison
    def __eq__(self, other) -> bool: ...
    def __ne__(self, other) -> bool: ...
    def __lt__(self, other) -> bool: ...
    def __le__(self, other) -> bool: ...
    def __gt__(self, other) -> bool: ...
    def __ge__(self, other) -> bool: ...

    # Predicates
    def is_zero(self) -> bool: ...
    def is_finite(self) -> bool: ...
    def is_infinite(self) -> bool: ...
    def is_nan(self) -> bool: ...
    def sign(self) -> Sign: ...
    def signum(self) -> FBig: ...

    # Rounding
    def trunc(self) -> FBig: ...
    def floor(self) -> FBig: ...
    def ceil(self) -> FBig: ...
    def round(self) -> FBig: ...
    def fract(self) -> FBig: ...

    # Precision
    def precision(self) -> int: ...
    def digits(self) -> int: ...
    def with_precision(self, precision: int) -> FBig: ...

    # Conversion
    def to_int(self) -> IBig: ...
    @staticmethod
    def from_parts(significand: IBig, exponent: int) -> FBig: ...

    # Math
    def sin(self) -> FBig: ...
    def cos(self) -> FBig: ...
    def tan(self) -> FBig: ...
    def exp(self) -> FBig: ...
    def ln(self) -> FBig: ...
    def sqrt(self) -> FBig: ...
    # ... (all math methods)

class DBig:
    # Same as FBig, but constructor accepts decimal.Decimal | str | int

class RBig:
    # Constructor, arithmetic, comparison same pattern
    # Plus:
    @property
    def numerator(self) -> IBig: ...
    @property
    def denominator(self) -> UBig: ...
    def trunc(self) -> IBig: ...
    def floor(self) -> IBig: ...
    def ceil(self) -> IBig: ...
    def round(self) -> IBig: ...
    def fract(self) -> RBig: ...
    def split_at_point(self) -> tuple[IBig, RBig]: ...
    @staticmethod
    def from_parts(numerator: IBig, denominator: UBig) -> RBig: ...
    @staticmethod
    def simplest_from_float(f: FBig) -> RBig | None: ...

class Sign:
    Positive: Sign
    Negative: Sign
```

---

## Step 10: Add tests

**New files to create:**

- `python/tests/test_float_ops.py` — FPy/DPy constructors, arithmetic, comparison, bool, predicates, rounding
- `python/tests/test_ratio_ops.py` — RPy constructors, arithmetic, comparison, bool, properties, rounding
- `python/tests/test_int_math.py` — UBig/IBig sqrt, cbrt, gcd, bit ops, accessors
- `python/tests/test_math.py` — module-level sin/cos/exp/sqrt/gcd/lcm

**Existing files to extend:**

- `python/tests/test_int.py` — add tests for floordiv, divmod, in-place ops

---

## Implementation Order

```
1. Step 0  (PyO3 0.29 upgrade + Cargo.toml deps) — do first
2. Step 1  (fix panics)          — independent
3. Step 2  (conversion helpers: into_*)  — independent
4. Step 2b (broaden __new__ via construct_*) — depends on Step 2
5. Step 2c (global cache module) — independent (cache.rs + FpError mapper)
6. Step 3  (FPy/DPy arithmetic)  — depends on Step 2
7. Step 4  (RPy arithmetic)      — depends on Step 2
8. Step 5  (int methods)         — independent (can parallel with 3-4)
9. Step 6  (float/rational pred + transcendentals via Step 2c) — depends on Step 3-4 + 2c
10. Step 6c (cross-type conversions: to_decimal/to_binary/to_rational/to_float) — depends on Step 6
11. Step 7  (math module)         — depends on Step 6
12. Step 8 (format fix)          — independent
13. Step 9 (stubs)               — after all code changes
14. Step 10 (tests)              — after all code changes
15. Step 11 (complex bindings)   — after Step 6 + 2c (follows the float pattern)
```

Steps 3 and 5 can be done in parallel. Steps 3 and 4 share the same pattern so do them together.

---

## Step 11: Complex number bindings (bare CBig → CPy)

**New dependency:** `dashu-cmplx` (added to `Cargo.toml` in Step 0a).

Per the Architecture decision, `CPy` wraps the **bare** `CBig<mode::Zero, 2>` (not `CachedCBig`). The constant cache is the module-global one from Step 2c, threaded into complex `Context` methods. `CPy` is `Send + Sync` (no `unsendable`).

### 11a: Add `CPy` wrapper type — `types.rs`

Already defined in Step 0d2:
```rust
type CBig2 = dashu_cmplx::CBig<dashu_float::round::mode::Zero, 2>;
#[pyclass(name = "CBig")]
pub struct CPy(pub CBig2);
```
Also add `BComplex(PyRef<'a, CPy>)` variant to `UniInput`.

### 11b: Register in `lib.rs`

```rust
m.add_class::<types::CPy>()?;
```

### 11c: CBig constructor — new file `python/src/complex.rs`

```rust
#[pymethods]
impl CPy {
    #[new]
    fn __new__(ob: &PyAny) -> PyResult<Self> {
        // Python complex → CBig (build from re/im FBig via CBig::from_parts, or FromStr)
        // string "a+bi" → CBig (FromStr; algebraic format)
        // (real, imag) tuple of FPy → CBig::from_parts(re.0.clone(), im.0.clone())
        // int/float/FPy → CBig::from (embed as z + 0i)
    }
}
```

`CBig` provides:
- `FromStr` for algebraic `"a+bi"` format
- `From<FBig>`, `From<UBig>`, `From<IBig>`, `From<primitives>` — embed as `z + 0i`
- `CBig::from_parts(re: FBig, im: FBig)` — note `into_parts()` returns `(FBig, FBig)` on bare CBig

### 11d: Arithmetic operators

Same macro pattern as FPy/DPy/RPy using `into_cpy()`. Bare `CBig` has full `Add`/`Sub`/`Mul`/`Div` (assign + ref/val combos vs itself and `FBig`), `Neg`, `Inverse`, `Sum`, `Product`. (Same infinite-input/`0/0` panic caveat as float arithmetic — address as a follow-up.)

### 11e: Accessors, predicates, complex-specific

```rust
// Accessors (re()/im() return &Repr<B>; project to a bare FBig)
fn real(&self) -> FPy { FPy(FBig::from(self.0.re().clone())) }
fn imag(&self) -> FPy { FPy(FBig::from(self.0.im().clone())) }

// Predicates
fn is_zero(&self) -> bool { self.0.is_zero() }
fn is_finite(&self) -> bool { self.0.is_finite() }
fn is_infinite(&self) -> bool { self.0.is_infinite() }

// Complex-specific (conj/proj are algebraic, no cache; abs/arg/norm return real FBig)
fn conj(&self) -> Self { CPy(self.0.conj()) }
fn proj(&self) -> Self { CPy(self.0.proj()) }
// abs/arg route through the float Context layer for clean errors; norm is algebraic.
fn abs(&self) -> PyResult<FPy> {
    let ctx = self.0.context();
    let res = with_cache(|c| ctx.abs(&self.0, Some(c)));   // returns FpResult<Rounded<FBig>>
    Ok(FPy(unwrap_float(res, ctx)?))
}
fn arg(&self) -> PyResult<FPy> { /* ctx.arg(...) */ }
fn norm(&self) -> FPy { FPy(self.0.norm()) }   // squared modulus, algebraic
```

### 11f: Complex transcendentals — route through the global cache + Context layer

Exactly the float pattern, but using `dashu_cmplx::Context` (obtained via `self.0.context()`) and `unwrap_complex`:

```rust
use crate::cache::{with_cache, unwrap_complex};

#[pymethods]
impl CPy {
    fn sin(&self) -> PyResult<CPy> {
        let ctx = self.0.context();
        let res = with_cache(|c| ctx.sin(&self.0, Some(c)));   // CfpResult<R,B> = Result<CRounded, FpError>
        Ok(CPy(unwrap_complex(res, ctx)?))
    }
    fn cos(&self) -> PyResult<CPy> { /* ctx.cos(...) */ }
    fn tan(&self) -> PyResult<CPy> { /* ctx.tan(...) */ }
    fn asin(&self) -> PyResult<CPy> { /* ctx.asin(...) */ }
    fn acos(&self) -> PyResult<CPy> { /* ctx.acos(...) */ }
    fn atan(&self) -> PyResult<CPy> { /* ctx.atan(...) */ }
    fn exp(&self) -> PyResult<CPy> { /* ctx.exp(...) */ }
    fn ln(&self) -> PyResult<CPy> { /* ctx.ln(...) */ }
    fn powf(&self, w: &Self) -> PyResult<CPy> { /* ctx.powf(...) */ }
    // sqrt/powi take NO cache but still go through Context for clean errors:
    fn sqrt(&self) -> PyResult<CPy> { let ctx = self.0.context(); Ok(CPy(unwrap_complex(ctx.sqrt(&self.0), ctx)?)) }
    fn powi(&self, exp: &IPy) -> PyResult<CPy> { /* ctx.powi(...) */ }
    fn sin_cos(&self) -> PyResult<(CPy, CPy)> { /* ctx.sin_cos(...) */ }
}
```

Domain errors (`ln(0)`, etc.) raise Python `ValueError`; `0/0`-style indeterminate forms raise `ZeroDivisionError` — never a session-crashing panic.

### 11g: Conversion to Python `complex`

```rust
fn __complex__(&self) -> PyResult<PyComplex> {
    let re: f64 = self.0.re().clone().try_into().map_err(conversion_error_to_py)?;
    let im: f64 = self.0.im().clone().try_into().map_err(conversion_error_to_py)?;
    Ok(PyComplex::from_doubles(re, im))
}
```

### 11h: Add to math module

Module-level functions in `math.rs` that delegate to `CPy` methods, matching the pattern for `FPy`.

## Verification

```bash
# Build
cd python && maturin develop

# Smoke test
python -c "
from dashu import FBig, DBig, RBig, UBig, IBig, CBig
a = FBig('1.5')
b = FBig('2.0')
assert a + b == FBig('3.5')
assert a * 3 == FBig('4.5')
assert bool(FBig('0.0')) == False
assert UBig(144).sqrt() == UBig(12)
assert UBig(12).gcd(UBig(8)) == UBig(4)
r = RBig.from_parts(IBig(1), UBig(3))
assert r * 3 == RBig.from_parts(IBig(1), UBig(1))
# transcendental uses the module-global ConstCache (Step 2c)
import math
assert abs(float(FBig('2').ln()) - math.log(2)) < 1e-50  # high precision
print('All smoke tests passed')
"

# Full test suite
python -m pytest python/tests/ -v
```

---

## Notes

- **MSRV**: dashu-python is exempt from the workspace MSRV policy (per AGENTS.md). Uses Rust 1.85 and edition 2024.
- **Feature flags**: `num-modular` is used by `__pow__` but listed as optional — fix to hard dependency or wire as default feature.
- **Complex crate**: `dashu-cmplx` (merged from develop, `6e21a65`) is a new core crate. Python bindings (Step 11) use bare `CBig` + the global cache.
- **Error model**: transcendentals route through the panic-free `Context` layer (`FpResult`) with a module-global `ConstCache` (Step 2c), mapping `Overflow/Underflow → ±∞/0` and `InfiniteInput/OutOfDomain → ValueError`, `Indeterminate → ZeroDivisionError`. See Architecture.
- **Send / free-threaded Python**: all wrappers (`FPy`/`DPy`/`CPy`/`UPy`/`IPy`/`RPy`) are `Send + Sync` — **none use `#[pyclass(unsendable)]`** — because the cache is thread-local, not owned by the value. The binding is free-threaded-Python (no-GIL) compatible; each thread gets its own cache.
- **Outstanding caveat — arithmetic panics**: bare `FBig`/`CBig` arithmetic operators still `panic!` on infinite input (e.g. `exp(1000) + 1`) and on `0/0`. Transcendentals are fully fixed; routing arithmetic through `Context` (or guarding finiteness / the `0/0` case) is a documented follow-up before release.
- **Changelog**: Document all changes in `python/CHANGELOG.md` under `## Unreleased` → `### Add`.
