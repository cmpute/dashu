# Changelog

## Unreleased

### Fix
- `ConstCache::pi` (and `Context::pi`) now compute one extra Chudnovsky series term,
  fixing an off-by-1-ulp mis-rounding at certain precisions (previously the term count
  provided only the ceiling — no accuracy headroom — so the series truncation error
  could push the result to the wrong side of a rounding boundary).
- `FBig::with_precision` (and `CachedFBig::with_precision`) now rounds an *unlimited*-
  precision source down to a finite target precision. Previously the shrink guard
  compared context precisions (`0 > N` is always false), so an unlimited value kept its
  full significand and the result violated the precision invariant.

## 0.6.0

### Change
- **(breaking) Ziv-backed transcendentals now require `R: ErrorBounds` (not `R: Round`)**: `exp`/
  `exp_m1`/`ln`/`ln_1p`/`log2`/`log10`, `powf`/`powi`, `hypot`, the trig and hyperbolic families
  (incl. inverses), and `FBig::sqrt`. All six built-in modes satisfy `ErrorBounds`; only custom
  non-`ErrorBounds` modes are affected.
- **(breaking) new `FpError::ZivRetryLimitExceeded`** — a Ziv loop that exhausts its retry budget
  (only possible if a radius-bound estimate is wrong) now returns `Err` instead of silently returning
  a possibly-1-ULP-wrong best-effort value.
- **(internal) every transcendental's error radius is now derived mechanically by Ball arithmetic**
  (`float/src/ball.rs`, an exact-integer error count composed through the series/reduction/composition,
  precision-aware) instead of the hand-derived `·ulp` formulas and outward intervals. Behavior
  unchanged, validated bit-exact against MPFR.
- **(internal) `Context::exp` of an astronomically large negative value now returns `Err(Underflow)`**
  (consistent with overflow; the convenience layer is unchanged).
- Removed the `rustversion` dependency (MSRV is 1.68); the unversioned `rand` / `rkyv` feature aliases
  now select the newest versions (`rand_v010` / `rkyv_v08`).

### Add
- **Correctly rounded transcendentals via a Ziv retry loop** — `exp`/`exp_m1`/`ln`/`ln_1p`, `log2`,
  `log10`, `powf`/`powi`, `hypot`, and the trig/hyperbolic families (incl. inverses) certify their
  rounding against the `ErrorBounds` preimage, retrying with more guard digits; exact results report
  radius 0 (MPFR-style `exact` tracking) so directed modes terminate.
- **Base-10 `log10`** (mirrors `log2`, with an exact power-of-ten shortcut).
- **Exact `Add`/`Sub`/`Mul` operators for `Repr`** (new `repr_ops` module) — lossless exact
  intermediates for the Ziv containment test, the correctly-rounded `Sum`, and the `FBig` multiply
  path.
- **`FBig::sqrt` is now correctly rounded** — a `p+1`-digit integer root was previously double-rounded
  (off by 1 ulp at digit boundaries); it now rounds in a single step from the round digit + `sqrtrem`
  remainder (MPFR-style). Power-of-two bases use a fast path; other bases certify via Ziv.
- **`CachedFBig` mirrors the rest of `FBig`'s value surface** (`round`/`trunc`/…/`quantize`,
  `hypot`/`nth_root`, `signum`/`ulp_lb`, base conversions), completing the drop-in mandate.
- `tuning` feature + `ziv_retries()`/`ziv_retries_reset()` for profiling retry counts;
  `Repr::zero_with_sign` public; rkyv 0.7 support (`rkyv_v07`).

### Fix
- **Mode-aware overflow/underflow saturation** — `Err(Overflow)`/`Err(Underflow)` and the
  `FBig → f32`/`f64` conversions now saturate to the directed endpoint per rounding mode (`±∞` /
  largest finite / smallest / `±0`) instead of mode-blind `±∞`/`0`; the `TryFrom` error variant is
  mode-independent.
- **`exp` range reduction for large `|x|`** — `ln B` now carries `⌈log_B|x|⌉+2` extra digits so the
  reduction quotient `s` is pinned (was off by ~`|x|`); the 32-bit `isize` threshold is fixed.
- **Directed `ln`/`log2` near `x ∈ [1, B)`** — the radius now also covers the pre-cancellation `sum`
  scale and the over-delivered `ln_base` context.
- **`FBig → f32`/`f64` subnormal/underflow** — round-to-odd at `width + 24` bits; a
  source-`log2_bounds` short-circuit for catastrophically tiny values.
- `ulp()`/`Repr::cmp` near the exponent ceiling (saturating arithmetic); the `no_std` build (the
  test-only Ziv counter is gated on `std`); `tan` pole check removed (~2× faster).

## 0.5.2

### Add
- `FBig::e` / `Context::e` / `CachedFBig::e`: Euler's number *e*, computed by
  exact-integer binary splitting on `e = Σ 1/k!` (leaf `(1, k, 1)`, reusing the
  universal `(P, Q, T)` merge). Unlike π, *e* is self-contained — it depends on no
  other cached constant and is itself reused by no operation — so it is **not**
  stored in `ConstCache` and `Context::e` takes no cache parameter. The factorial
  series is the optimal algorithm for *e* (asymptotically `O(M(n) log n)` under
  FFT multiplication, i.e. faster than π), and it avoids both the `ln`-based
  argument reduction and the `√p`-fold powering that `exp(1)` would pay for.
- `FBig::fma` / `Context::fma` / `CachedFBig::fma`: fused multiply–add
  `c + sign·(a·b)` with a single rounding. `sign` (`Sign::Positive`/`Sign::Negative`)
  selects add vs subtract, mirroring integer's `add_signed_mul`. It assembles the
  existing exact-product (`make_mul_repr`) and add/round kernels, inheriting their
  severe-cancellation and sticky-tail handling (and the guard digit an effective
  subtraction may leave). The trig quadrant reduction `x − k·(π/2)` now uses it,
  removing one rounding from range reduction. (`sqr` keeps its dedicated, faster
  kernel — don't write `x²` as `fma(x, x, …)`.)

- `FBig::ulp_lb`: a cheap lower bound on `ulp`, guaranteed strictly smaller than it
  (computed from the approximated `digits_lb` rather than the exact digit count). Public
  successor to the internal `sub_ulp`; useful as a conservative negligibility threshold
  (e.g. iterative-method termination). For a rigorous error/radius bound, use `ulp`.
- `Context::addsub_vv` / `addsub_vr` / `addsub_rv` / `addsub_rr`: low-level
  ownership-aware add/subtract kernels computing `lhs + rhs_sign· rhs` directly on
  `Repr` (no `FBig` wrapping, no `Result`) — `Sign::Positive` adds, `Negative`
  subtracts. The four variants cover every ownership combination of the two operands
  (`v` = by-value, `r` = by-ref); each reuses the owned operand's significand buffer
  where it can, avoiding a clone versus the `Context::add`/`sub` path. Intended for
  downstream crates (e.g. `dashu-ball`) that want by-value `Repr` arithmetic on a
  fixed context. The `+`/`-` operators and `Context::add`/`sub` now route through
  `addsub_*`, making it the single source of truth for add/subtract routing.

### Change
- `x ± 0` now rounds `x` to the context precision. The add/subtract path previously
  short-circuited on a zero operand and returned the other operand verbatim, which
  could leave a guard digit (up to `precision + 1` digits) in the result. All other
  add/subtract results are unchanged.

### Fix
- `test_e_known_decimal_prefix` failed to compile under `no_std` (`ToString` is not in
  the prelude without `std`): import `alloc::string::ToString` in the cache tests.
- `Context::mul`, `Context::sqr`, and `Context::cubic` are now strictly correctly rounded. They
  previously shrank the operand(s) to `2*precision` (mul/sqr) or `3*precision` (cubic) before
  multiplying — a speed optimization. That operand pre-rounding — though each operand is rounded
  correctly — perturbs the result by the accumulated rounding error, so the final value could land
  1 ulp off the exact-product-rounded value when it sat near a rounding boundary. The exact product
  / square / cube of the full operand(s) is now computed and rounded (still via the dedicated
  `sqr`/`cubic` kernels), which is always correctly rounded, at the cost of operating on operands
  far larger than the target precision (uncommon).
- `Display`/`LowerExp`/`UpperExp` (and the per-base `{:b}`/`{:o}`/`{:h}`/`{:x}` formatters) now read
  `Repr::sign` (not the bare significand, which is always `+` for zero) and clamp a zero
  significand's display exponent to `0`, so the `-0` sentinel exponent no longer leaks into the
  output. By default `-0` and `+0` both render as `"0"` (signed zero is treated as an internal
  detail); the formatter's `+` flag reveals the sign (`-0` → `"-0"`, `+0` → `"+0"`) — use it for
  string round-trips (e.g. into MPFR) that must preserve the signed-zero sign.

## 0.5.1

### Add
- `Repr::new_const`: a `const`-evaluable, normalized `Repr` constructor from a `DoubleWord`
  significand (the `const` counterpart of `Repr::new`). `FBig::from_parts_const` now delegates to
  it, and the complex literal macro uses it.
- `FBig::log2` / `Context::log2` / `CachedFBig::log2`: base-2 logarithm, correctly rounded via
  `ln(x)/ln(2)` evaluated at an elevated working precision. Previously only the f32-precision
  `log2_bounds` magnitude estimate was available, so directed `log2` was wrong by many ULPs.

### Fix
- `powi`'s overflow guard no longer reports a spurious overflow when the base is very close to 1
  (a large significand with a large negative exponent). It estimated `log2(base)` with `log2_est`,
  which for such a base is the difference of two ~1e3-magnitude terms and catastrophically cancels
  to ~1e-4 of `f32` noise; scaled by a large exponent that noise crossed the overflow threshold
  — which on 32-bit targets is only `isize::MAX·log2(B) ≈ 7e9` — and returned a spurious ±inf. This
  made high-precision base conversion (`FBig::with_base`) panic ("arithmetic operations with the
  infinity are not allowed!") on 32-bit targets (wasm32, i686). The guard now uses the
  bit-length-based `log2_bounds`, which does not cancel (#95).
- `to_f64`/`to_f32` now round the source once, directly to the target's precision at its own
  magnitude (fewer than 53/24 bits for subnormals), instead of through a fixed 53/24-bit
  intermediate that re-rounds into the subnormal grid. This removes a 1-ULP double-rounding error
  on subnormal values that sit just past a subnormal halfway.
- `to_f64`/`to_f32` no longer panic in debug builds (nor silently double-round in release) on
  high-precision inputs: the base-changing conversion now pre-shrinks the source significand before
  dividing, upholding `repr_div`'s dividend-width contract (as `Context::div` already does) instead
  of feeding an oversized dividend into the division.
- `exp`, `exp_m1`, `sqrt`, `ln`, and `ln_1p` no longer panic on exact zero/one inputs that carry
  unlimited precision (precision 0), such as `FBig::try_from(0.0)` and the `FBig::ONE`/`ZERO`
  constants. The exact-result shortcuts now run before the limited-precision assertion.
- The `round_fract` debug assertion no longer materializes `B^precision`, which for a sparse sticky
  tail (where `precision` is the exponent gap — e.g. `exp_m1` of a large-magnitude input) could
  exhaust memory in debug builds. It now checks the precondition with `log2` bounds.

## 0.5.0

### Add
- **IEEE-754 signed zero (`-0`)**: operations now produce the sign of zero mandated by the standard
  (e.g. `1 / -inf = -0`, `sqrt(-0) = -0`, `ceil(-0) = -0`, cancellation under round-toward-negative).
  `+0` and `-0` compare equal; `-0.0` round-trips through `f32`/`f64`.
- **New error model**: `FpError` (`InfiniteInput`, `OutOfDomain`, `Indeterminate`, and new
  `Overflow(Sign)`/`Underflow(Sign)`) with `FpResult<T> = Result<Rounded<T>, FpError>`. Infinite
  *outputs* are values inside `Ok` (`1/0 → +inf`, `ln(0) → -inf`, `exp(huge) → +inf`); infinite
  *inputs* are `Err(InfiniteInput)` (structurally avoiding NaN-producing indeterminate forms); domain
  errors (`0/0`, `sqrt(-x)`, `ln(-x)`, `asin(|x|>1)`) are `Err`. The `FBig`/`CachedFBig` convenience
  layers panic on error and saturate `Overflow`/`Underflow` to signed infinity/zero.
- **`ConstCache` + `CachedFBig`**: `ConstCache` caches exact binary-splitting tree state for constants
  (π, ln2, ln10, ln(B) — including the base-free `√10005` isqrt that feeds π) so repeated calls at
  increasing precision *extend* prior work instead of recomputing. `CachedFBig` is an `FBig` carrying a
  shared `Rc<RefCell<ConstCache>>` handle; its transcendentals (`ln`, `exp`, `sin`/`cos`/…, `pi`, base
  conversion) thread that handle through `Context`. `Context`/`FBig` stay `Copy` + `Send` + `Sync` +
  `no_std`; only `CachedFBig` is `!Send + !Sync`. `CachedFBig::cache()`/`clear_cache()` and
  `ConstCache::total_terms()`/`total_words()` inspect/free cached memory.
- **Hyperbolic functions** `sinh`/`cosh`/`tanh`/`asinh`/`acosh`/`atanh` on `Context`/`FBig`/`CachedFBig`,
  built from cancellation-free `exp_m1`/`ln_1p` formulas with IEEE special-value handling.
- **`FBig::hypot`** / `Context::hypot`: overflow/underflow-safe `sqrt(a² + b²)` via the scaled
  sum-of-squares (the larger operand is never squared).
- **`FBig::sinh_cosh`** / `Context::sinh_cosh`: combined `sinh`+`cosh` sharing the `exp_m1(±x)` work.
- `exp`/`exp_m1` now accept infinite input (`exp(+inf) = +inf`, `exp(-inf) = +0`, `exp_m1(-inf) = -1`).
- `CachedFBig` now mirrors `FBig`'s full trait surface — formatting, ordering, conversions, shift and
  root/euclid ops, `Sum`/`Product` — plus the reference-operand variants of its binary operators and
  mixed ops with `FBig` and the integer primitives. Third-party traits (serde/num-traits/num-order/
  rand/zeroize/postgres) are reached via `.as_fbig()`.
- `Repr::num_hash_residue` (behind `num-order`), exposed so composite types can combine their parts'
  residues algebraically.

### Change
- **(breaking)** `Repr::is_zero` is renamed to `Repr::is_pos_zero` (it tests only `+0`); use
  `significand().is_zero()` to detect either signed zero. `num_traits::Zero::is_zero` for `FBig` now
  returns `true` for either.
- **(breaking)** `Sum` for `FBig` is now correctly-rounded: addends are accumulated exactly and the
  total rounded once (MPFR `mpfr_sum` semantics). The generic `Sum<T>`/`Product<T>` impls are replaced
  by concrete `Sum`/`Sum<&FBig>`/`Product`/`Product<&FBig>`; cross-type sums (e.g. `Sum<u8>`) require
  converting the elements first.
- **(breaking)** `FBig` human-readable serde now pads the serialized string to the context precision's
  digit count so precision round-trips (the binary format already preserved it).
- **(breaking, encoding)** infinities are re-encoded with sentinel exponents `isize::MAX`/`isize::MIN`
  and `-0` at exponent `-1`; `normalize()` preserves these, and `Repr`'s `PartialEq`/`Eq` are manual
  so `+0 == -0`.
- **(breaking, result model)** `Context` arithmetic/transcendental/trig methods now return
  `FpResult<FBig<R, B>>` instead of `Rounded<FBig<R, B>>` (arithmetic) / the old trig `FpResult` enum.
  `FBig::tan`/`asin`/`acos`/`atan2` now return `Self` (panic on error), matching the other trig methods.
- **(breaking, low-level)** `Context` constant-source methods take an additional
  `cache: Option<&mut ConstCache>` parameter; the high-level `FBig` API is unchanged (passes `None`).
- `atan2(±finite, +inf)` returns the signed zero of `y`; `powf(±0, y)` returns the *positive* result
  (`+0` for `y > 0`, `+inf` for `y < 0`) — use `powi` for the sign-correct `pow(-0, odd) = -0`.

### Remove
- Public `Repr::from_str_native` / `FBig::from_str_native` (now crate-private — use `s.parse()`).
- The old `FpResult` enum and the `MathCache` type (subsumed by the public `ConstCache`).
- The `panic_overflow`/`panic_underflow`/`panic_infinite`/`panic_power_negative_base`/`panic_root_negative`
  helpers (their conditions are now `FpError`s).

### Fix
- Signed-zero correctness: `exp_m1(-0) = -0`; `powf(base, -0) = 1`; `quantize(-0)` preserves the sign;
  `+`/`-` produce `-0` on exact cancellation under `Down`; `Sum` cancellation to zero yields `+0` (or
  `-0` only under roundTowardNegative); `IBig`/`UBig::try_from(FBig)` accept `-0`.
- `NumOrd` against a primitive `0.0`/`-0.0` now reports either signed zero as `Equal`.
- `error_bounds` honors the `ErrorBounds` contract for unlimited precision (`Away` returns
  `(0, 0, true, true)`), and `HalfEven` gives `-0` the one-sided preimage (matching `Zero`/`HalfAway`).
- `Context::asin`/`acos` no longer panic on `±1` under `Down` (the `1 - x²` → `-0` → `sqrt(-0)` path).
- `exp`/`exp_m1`/`powi` return `±inf`/`0` on astronomically large results instead of panicking.
- `exp`/`exp_m1` at high precision (≳ a few thousand digits) were wrong in the low bits — the series
  working precision now carries `≈ √p` extra guard digits to absorb the `Bⁿ` final-powering error
  amplification (cf. MPFR's `K ≈ √precy`).
- `ShrAssign` (`>>=`) previously subtracted the shift twice.
- Trig functions no longer panic on tiny negative inputs (`sin(-1e-30)`): the signed-zero encoding no
  longer trips argument reduction.
- Broken intra-doc links surfaced by `cargo doc -D warnings`; `f64::ceil` in `ConstCache` replaced
  with a `no_std`-safe integer ceiling.
- `FBig::from_repr`'s debug assertion now accepts the documented single guard digit.

### Improve
- Documented the `math::trig` module and enabled `#![deny(missing_docs)]` together with
  `clippy::dbg_macro`, `clippy::undocumented_unsafe_blocks`, and `clippy::let_underscore_must_use`
  as crate-level denies.
- Migrated the verbose `FBig` type prose out of the rustdoc and into the user guide, leaving a concise
  summary with guide links; the runnable `# Examples` are kept verbatim.
- (internal) `Context::iacoth` and the `ConstCache` π path use binary splitting; the PostgreSQL
  `NUMERIC` conversion and trig argument reduction use `UBig::to_digits` / `IBig::try_from`.

## 0.4.5

### Add
- Add `FBig::quantize(exp)` to round to the nearest multiple of `BASE^exp` (the dashu analog of Python's `Decimal.quantize()`), returning `Rounded<Self>` with the result precision set so that `ulp()` equals `BASE^exp`.
- Implement the cubic root (`CubicRoot` for `FBig`, `Context::cbrt`) and the general nth root (`FBig::nth_root`, `Context::nth_root`) with correct rounding, built on top of `UBig::nth_root`.
- Implement trigonometric functions (`sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `atan2`, `sin_cos`) for `FBig` and `Context<R>` ([#60](https://github.com/cmpute/dashu/pull/60)).
- Add π constant computation (`FBig::pi()` and `Context::pi()`) using the Chudnovsky algorithm with binary splitting ([#60](https://github.com/cmpute/dashu/pull/60)).
- Add `FpResult` enum to handle non-finite math operation results (NaN, Infinite, Overflow, Underflow) without panicking ([#60](https://github.com/cmpute/dashu/pull/60)).
- Add `panic_nan`, `panic_overflow`, `panic_underflow`, and `panic_infinite` helpers to the `error` module.
- Optional `rand_v09` (rand 0.9, MSRV 1.63) and `rand_v010` (rand 0.10, MSRV 1.85) features mirroring `rand_v08`. The default `rand` feature still maps to `rand_v08`.
- The random-float distributions (`Uniform01`, `UniformFBig`) and their sampling now live once in the version-agnostic `dashu_float::rand` module. The per-version modules are now private trait bindings.

### Fix
- Fix rounding issues in `to_f32()` and `to_f64()` ([#53](https://github.com/cmpute/dashu/issues/53), [#56](https://github.com/cmpute/dashu/issues/56)).
- Fix several rounding bugs in `FBig`/`Context` addition and subtraction: severe-cancellation collapse, spurious-ULP errors from negligible operands, the window-edge boundary, and `Context::sub` with a zero left operand under directed rounding modes.
- Fix `FBig::fract()` inflating context precision and `split_at_point_internal` using an incorrect fractional scale for values smaller than one.

## 0.4.4

- Bump MSRV from 1.61 to 1.68.

## 0.4.3

- Mark `FBig::from_str_native` as deprecated.
- Implement `TryFrom<Repr>` and `TryFrom<FBig>` for primitive integers.
- Implement `TryFrom<Repr<2>>` and `TryFrom<FBig<_, 2>>` for primitive floats.
- Implement `From<UBig>` and `From<IBig>` for `Repr`.
- Implement `core::fmt::{Binary, Oct, LowerExp, UpperExp, LowerHex, UpperHex}` for `Repr`, `FBig` (some are limited to certain bases).

## 0.4.2

- Add `Repr::from_static_words` to support the `static_fbig!` and `static_dbig!` macros.
- Add `FBig::from_repr_const` to support create an `FBig` instance from repr in const context.
- Add conversion from `f32`/`f64` to `Repr<2>`.
- Implement `NumOrd` between `FBig` and primitive integers / floats. 
- Implement `AbsOrd` between `FBig` and `UBig`/`IBig`.
- Now the `Debug` output of `FBig` values will not contains the rounding mode information (when alternative flag is not set).

## 0.4.1

- Fix the termination criteria for `ln` and `exp` series ([#44](https://github.com/cmpute/dashu/issues/44)).
- Fix `powf` panicking when base is 0.

## 0.4.0

### Add

- Implement `num-order::NumOrd` between `FBig` and `UBig`/`IBig` and between `FBig` with different bases.
- Implement `num-order::NumHash` for `FBig` and `Repr`.
- Add `ErrorBounds` trait that calculate the rounding range for a floating point number.

### Change

- Now feature `num-traits` and `rand` are not enabled by default, feature `num-order` is enabled instead.
- The type of `Repr::BASE` is changed from `IBig` to `UBig`
- `UBig::square` and `IBig::square` are renamed to `sqr`.
- The implementation of square root is now implemented by the `dashu_base::SquareRoot` trait instead of a standalone method of `FBig`.
- The rounding behaviors of `FBig::to_decimal` and `FBig::to_binary` are changed for better ergonomics.
- The rounding behaviors of `FBig::to_f32` and `FBig::to_f64` now follow the mode specified by the type argument.

## 0.3.2

- The default precision for float numbers from `from_parts`/`From<UBig>`/`From<IBig>` are now based on the actual digits on the integers, rather than the digits after simplification. (#28)

## 0.3.1

- Implement `num_traits::{Zero, One, FromPrimitive, ToPrimitive, Num, Signed, Euclid, Pow}` for `FBig` (#19)
- Implement `rand::distributions::uniform::UniformSampler` for `FBig` through `crate::rand::UniformFBig`
- Implement `rand::distributions::{Open01, OpenClosed01, Standard}` for `FBig`
- Implement `dashu_base::Inverse` for `FBig`
- Implement `rand::distributions::uniform::SampleUniform` for `FBig`.
- Implement `serde::{Serialize, Deserialize}` for `FBig` and `Repr`
- Implement `Rem` trait for `FBig`
- Add support of random floating point numbers generation through `crate::rand::Uniform01` and `crate::rand::UniformFBig`.
- Add support for serialization from/to PostgreSQL arguments through `diesel::{deserialize::FromSql, serialize::ToSql}` and `postgres_types::{FromSql, ToSql}`.
- Add `from_str_native()` for `Repr`
- Add `to_f32()`, `to_f64()` for `Repr`, and these two methods supports all bases for both `Repr` and `FBig`.
- Add `to_int()` for `Repr`, which is equivalent to `FBig::trunc()`
- Add `TryFrom<FBig>` for `UBig` and `IBig`
- Add `round()` for `FBig`
- Add `rand_v08` and `num-traits_v02` feature flags to prevent breaking changes due to dependency updates in future 
- Re-export operation traits through the `ops` module.

## 0.3.0

### Add

- Conversion from FBig to `f32`/`f64` support subnormal values now.
- Add a `split_at_point()` function to `FBig`

## 0.2.1

- Implement `core::iter::{Sum, Product}` for `FBig`
- Implement `powf`, `sqrt` for `FBig`

## 0.2.0 (Initial release)

- Support basic arithmetic operations (`add`/`sub`/`mul`/`div`/`exp`/`ln`) and base conversion.

# Todo

## Roadmap to next version
- Support generating base math constants (E, Pi, SQRT2, etc.)
- Support operations with inf
- Create operations benchmark
- Benchmark against crates: rug, twofloat, num-bigfloat, rust_decimal, bigdecimal, scientific
- Implement more formatting traits
- Other math functions: sin/cos/tan/etc.
