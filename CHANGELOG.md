# Changelog

This changelog aggregates the releases of every dashu crate — `dashu`, `dashu-base`,
`dashu-int`, `dashu-float`, `dashu-ratio`, `dashu-cmplx`, `dashu-macros`, and the
`dashu-python` binding. Each entry is tagged with the crate and minor version it belongs
to; see the per-crate `CHANGELOG.md` in each subdirectory (`base/`, `integer/`, …) for
full detail.

## Unreleased

## 0.6.0-rc.3 — coordinated release

### Add
- **dashu-base**: `Approximation::value_with_exact()` — the value together with an `is_exact` flag
  (feeds dashu-float's exact-tracking Ball ops and `hypot`).
- **dashu-int**: rkyv 0.7 (`rkyv_v07`) and rkyv 0.8 (`rkyv_v08`) support — `UBig`/`IBig` archive
  as their native word vectors (zero-copy).
- **dashu-float**: `tuning` feature + `ziv_retries()`/`ziv_retries_reset()` — per-Ziv-loop retry
  counter for profiling how tight each transcendental's error radius is.
- **dashu-float**: rkyv 0.7/0.8 support for `Repr`/`Context`/`FBig`; `Repr::zero_with_sign` is
  now public.
- **dashu-ratio**: rkyv 0.7/0.8 support for `RBig`/`Relaxed`.
- **dashu-cmplx**: `serde` and `zeroize` support for `CBig`; rkyv 0.7/0.8 support; signed-zero
  preservation on exact-zero `sin_cos`/`sqr`/`log`.
- **dashu-python**: CI wheel/sdist build workflow (`.github/workflows/python-wheels.yml`).

### Change
- **dashu-int** / **dashu-float** / **dashu-ratio** / **dashu-cmplx**: unversioned `rand`/`rkyv`
  feature aliases now select rand 0.10 / rkyv 0.8 (pin `rand_v08`/`rkyv_v07` to keep the old).
- **dashu-float**: `log2`'s error radius now derived by Ball arithmetic instead of directed
  intervals (precision-aware propagation; public behavior unchanged, performance at parity).
- **dashu-float**: `exp`/`exp_m1`, the hyperbolic and trigonometric families, `powi`, and `hypot`
  migrated to the same Ball error-propagation engine.
- **dashu-ratio**: **(breaking)** `RBig::pow`/`Relaxed::pow` take `isize` (was `usize`); negative
  exponents reciprocate.
- **dashu-cmplx**: **(breaking)** complex infinity is now a terminal value — no arithmetic accepts
  an infinite operand anymore (rejects with `FpError::InfiniteInput`).
- **dashu-macros**: version-aligned to rc.3 (no functional changes; skipped rc.2).

### Fix
- **dashu-float**: `powf`/`powi` wrongly rounded for large `|y·ln x|` (input-error inflation
  omitted the result-significand factor).
- **dashu-float**: `sqrt`-based radius under-bound in `asin`/`asinh`/`acosh`/`hypot`.
- **dashu-float**: `atanh(x)` for `x < 0` near the pole returned wrongly-rounded values.
- **dashu-float**: `powf` of a base `< 1` no longer hangs in the Ziv loop (`scale_int` →
  exact-tracking `scale_int_tracking`).
- **dashu-float**: `cargo test --no-default-features` compiles again (Ziv counter gated on `std`).
- **dashu-cmplx**: dangling intra-doc link in the `fma` docs.

## 0.6.0-rc.2 — dashu-float only

### Change
- Unified `f32`/`f64` range detection: `into_f32_internal`/`into_f64_internal` now return
  `FpResult`, so the range decision lives in one place.
- `Context::exp` of an astronomically large negative value now returns `Err(Underflow)`.
- Ziv driver takes fallible closures; hoisted overflow probes removed.

### Fix
- Directed overflow/underflow of `FBig` results now saturate to the **mode-aware** endpoint
  (`Context::unwrap_fp`), so `Up ≥ Down` holds on both ends.
- Directed overflow/underflow of `FBig → f32`/`f64` (range detection delegated to `encode`).
- `TryFrom<FBig> for f32`/`f64` error variant is now mode-independent.
- Directed `ln`/`log2` of `x ∈ [1, B)` and of `x` just above 1 (radius under-estimates).
- `exp`/`exp_m1` extreme-negative endpoint carried precision 0; `ulp()`/`ulp_lb()` panic on an
  extreme exponent (saturating arithmetic); `exp_m1` of large negative input short-circuits.
- `exp` overflow probe could disagree with the computation; `exp` range reduction for large `|x|`.
- `FBig → f32`/`f64` subnormal mis-round (convert at `width + 24` bits); underflow of a
  catastrophically tiny source short-circuits before conversion.
- `powi` memory exhaustion / hang on extreme exponents (margined guard + `exp(y·ln x)` fallback);
  panic at the exact exponent ceiling; `Repr::cmp` overflow near the exponent ceiling.

## 0.6.0-rc.1 — coordinated release

### Add
- **dashu-base**: `ParseError::InvalidSyntax` variant for structurally malformed input (breaking
  for exhaustive `ParseError` matches).
- **dashu-float**: exact `Add`/`Sub`/`Mul` operators for `Repr` (lossless intermediates).
- **dashu-float**: guaranteed-correct rounding for `exp`/`exp_m1`/`ln`/`ln_1p`, the trig family,
  and the hyperbolic family via a Ziv retry loop.
- **dashu-float**: `hypot`, `powf`, and `powi` now correctly rounded via Ziv (with exactness
  tracking for `hypot`); `Repr::is_int` is now `const`.
- **dashu-cmplx**: correct rounding for the complex transcendentals (`exp`, `ln`, `powf`, `powi`,
  trig family, `abs`, `arg`, `sqrt`) via a complex Ziv driver.

### Change
- **dashu-float**: **(breaking, bound)** Ziv-backed transcendentals now require `R: ErrorBounds`
  instead of `R: Round`.
- **dashu-float**: `FBig::log2` now correctly rounded via Ziv (was near-correct).
- **dashu-cmplx**: **(breaking, bound)** complex transcendentals now require `R: ErrorBounds`;
  `powi` correctly rounded via its own Ziv loop.
- **dashu-cmplx**: `FromStr` returns `ParseError::InvalidSyntax` for malformed input.
- **dashu-ratio**: `from_str_radix`/`from_str_with_radix_prefix` reject multiple `/` separators
  with `ParseError::InvalidSyntax`.
- **dashu-macros**: literal macros surface the new `ParseError::InvalidSyntax` with a dedicated
  panic message.

### Fix
- **dashu-int**: NTT squaring/multiplication of an all-zero operand no longer panics (the crash
  behind dashu-float's `hypot(3,4)` under directed rounding).
- **dashu-float**: directed-rounding hang on exactly-representable results (`acos(±1)`, `asin(0)`,
  exact `hypot`); `tan` pole check removed (~2× cost); `no_std` build of the Ziv retry counter.
- **dashu-cmplx**: `Sum` for `CBig` now correctly rounded; unlimited-precision arithmetic correct;
  `no_std` build of the Ziv retry counter.

## 0.5.x

*crates: dashu-base, dashu-int, dashu-float, dashu-ratio, dashu-cmplx, dashu-macros, dashu-python*

- **dashu-float** (0.5.2): Add `FBig::e` (Euler's constant, exact binary splitting), `FBig::fma`
  (fused multiply–add, one rounding), `FBig::ulp_lb`, `Context::addsub_vv/vr/rv/rr` kernels;
  `x ± 0` now rounds to the context precision; `mul`/`sqr`/`cubic` strictly correctly rounded.
- **dashu-base** (0.5.1): Fix `BitTest::bit` at the sign-bit position; `FloatEncoding::encode` no
  longer returns `NaN` for huge exponents.
- **dashu-int** (0.5.1): Fix `to_f64` exactness at `DoubleWord::MAX`; GCD of large similarly-sized
  integers no longer aborts.
- **dashu-float** (0.5.1): Add `Repr::new_const`, correctly-rounded `FBig::log2`; fix spurious
  `powi` overflow for base ≈ 1; single-rounding `to_f64`/`to_f32`.
- **dashu-ratio** (0.5.1): Add `from_str_expanded`/`from_str_decimal` (repeating notation),
  `num_traits::Pow<isize>`; fix GCD memory abort, `UBig::try_from(RBig)` denominator check.
- **dashu-cmplx** (0.5.1): Add `CBig::from_parts_const` (const `cbig!`).
- **dashu-macros** (0.5.1): `cbig!` in `const` position for `DoubleWord`-fitting coefficients.
- **dashu-base** (0.5.0): Remove `AbsEq` (use `AbsOrd`); full docs + `#![deny(missing_docs)]`.
- **dashu-int** (0.5.0): Add `to_digits`/`from_digits`; breaking serde format → two's-complement
  LE bytes, `in_radix` radix → `u8`; fix `nth_root(0)`; full docs.
- **dashu-float** (0.5.0): IEEE-754 signed zero; `FpError`/`FpResult` error model;
  `ConstCache`/`CachedFBig`; hyperbolic functions; `hypot`; `sinh_cosh`; breaking `is_zero` →
  `is_pos_zero`, correctly-rounded `Sum`, serde precision padding, `Context` returns `FpResult`;
  remove public `from_str_native`.
- **dashu-ratio** (0.5.0): breaking `From<RBig/Repr> for FBig` → `TryFrom` (exactness in the
  target base); `in_radix` radix → `u8`; fix `UBig::try_from(RBig)`.
- **dashu-cmplx** (0.5.0): **initial release** — two-layer `Context`/`CBig` API, field arithmetic,
  `powi`/`powf`, transcendentals, `CachedCBig`, `cbig!`/`static_cbig!`, rand, `num-complex` interop.
- **dashu-macros** (0.5.0): `cbig!`/`static_cbig!`; `#![deny(missing_docs)]`.
- **dashu-python** (0.5.0): **initial release of dashu-rs on PyPI** — six types, panic-free
  transcendentals, module `math` API, optional serde/rand/rkyv/zeroize.

## 0.4.x

*crates: dashu-base, dashu-int, dashu-float, dashu-ratio, dashu-macros*

- **dashu-float** (0.4.5): Add `quantize`, `cbrt`/`nth_root`, trig functions + π (Chudnovsky,
  binary splitting), `FpResult`, `rand_v09`/`rand_v010`; fix `to_f32`/`to_f64` rounding,
  add/sub rounding bugs.
- **dashu-float** (0.4.4): Bump MSRV to 1.68.
- **dashu-ratio** (0.4.4): Fix `to_f32`/`to_f64` double rounding.
- **dashu-base** (0.4.3): Add `Sign::as_sign_str`; fast integer `sqrt` via native `f64::sqrt`.
- **dashu-int** (0.4.3): Add NTT multiplication, asymmetric NTT, `monty` module, specialized
  squaring, const `from_u64`/`from_i64`, `rand_v09`/`rand_v010`, version-agnostic rand module;
  double-word mul kernels, runtime threshold tuning (`DASHU_THRESHOLD_*`); assorted fixes.
- **dashu-float** (0.4.3): Deprecate `from_str_native`; `TryFrom` for primitive ints/floats;
  formatting trait impls.
- **dashu-ratio** (0.4.3): Add `Binary`/`Octal`/`LowerHex`/`UpperHex`, `in_radix`/`in_expanded`,
  `rand_v09`/`rand_v010`, version-agnostic rand module.
- **dashu-base** (0.4.2): `log2_bounds` strictly enclosing; MSRV bump.
- **dashu-int** (0.4.2): Add `ones`/`as_ubig`/`from_chunks`/`to_chunks`, `TryFrom<f32/f64>`,
  `IBig` byte conversions, mixed bit ops; fixes; MSRV bump.
- **dashu-float** (0.4.2): `Repr::from_static_words`; `FBig::from_repr_const`; `NumOrd`/`AbsOrd`
  with ints; `Debug` no longer shows the rounding mode.
- **dashu-ratio** (0.4.2): `Div<RBig>`/`Div<Relaxed>` for `UBig`/`IBig`; division bug fix;
  MSRV bump.
- **dashu-macros** (0.4.2): Replace `paste` with `pastey`; MSRV bump.
- **dashu-base** (0.4.1): Deprecate `AbsEq`; re-implement `next_up`/`next_down`.
- **dashu-int** (0.4.1): `AbsEq`/`AbsOrd`; `from_static_words` (static macros); `is_multiple_of`;
  const `trailing_zeros`/`trailing_ones`; `trailing_ones` fix.
- **dashu-float** (0.4.1): Fix `ln`/`exp` series termination; `powf` no longer panics on base 0.
- **dashu-ratio** (0.4.1): `AbsOrd`/`NumOrd` with ints/floats; `as_relaxed`; `to_float` enforces
  precision on zero.
- **dashu-macros** (0.4.1): `static_ubig!`/`static_ibig!`/`static_fbig!`/`static_rbig!`.
- **dashu-base** (0.4.0): `is_positive`/`is_negative`; trait moves (`SquareRoot`/`CubicRoot` →
  `math`, `AbsCmp` → `AbsOrd`, `square` → `sqr`).
- **dashu-int** (0.4.0): `ConstDivisor`; `as_ibig`; `NumOrd`; serde format → LE byte sequence;
  `IntoRing` refactor; `Modulo` → `Reduced`; remove `PartialOrd`/`PartialEq` between
  `UBig`/`IBig`.
- **dashu-float** (0.4.0): `NumOrd`/`NumHash`; `ErrorBounds`; feature defaults (`num-order` on);
  `Repr::BASE` → `UBig`; `sqrt` via the `SquareRoot` trait.
- **dashu-ratio** (0.4.0): `is_int`; `NumOrd`/`NumHash`; `simplest_from_float`; feature defaults;
  remove `PartialOrd` between `RBig`/`Relaxed`.
- **dashu-macros** (0.4.0): Remove the `embedded` feature.

## 0.3.x

*crates: dashu-base, dashu-int, dashu-float, dashu-ratio, dashu-macros*

- **dashu-float** (0.3.2): Default precision from the actual digits of integer conversions.
- **dashu-ratio** (0.3.2): Multiplication bug fix.
- **dashu-base** (0.3.1): `Inverse` trait; `AbsCmp`/`AbsEq` for primitives.
- **dashu-int** (0.3.1): `UniformBits`; `count_ones`/`count_zeros`; `cubic`; `rand_v08`/
  `num-traits_v02` feature flags.
- **dashu-float** (0.3.1): `num_traits`/rand/serde/diesel/postgres impls; `Uniform01`/
  `UniformFBig`; `to_f32`/`to_f64`; `round`; `rand_v08`/`num-traits_v02`.
- **dashu-ratio** (0.3.1): `Sum`/`Product`/`Rem`/`num_traits`/rand/serde impls; `cubic`/`pow`/
  `round`; `rand_v08`/`num-traits_v02`.
- **dashu-macros** (0.3.1): Fix `ibig`/`rbig` macros using incorrect crate names.
- **dashu-base** (0.3.0): `AbsCmp`/`AbsEq`/`FloatEncoding`/`Signed`/`SquareRoot`/`CubicRoot`/
  `EstimatedLog2`; error types moved from int; `BitTest` rework.
- **dashu-int** (0.3.0): `gcd`/`gcd_ext`/`div_rem` between `UBig`/`IBig`; `BitTest` for `IBig`;
  `TryFrom<f32/f64>`; trait-only exposure; `to_le_bytes` → `Box<[u8]>`.
- **dashu-float** (0.3.0): Subnormal `f32`/`f64` conversion; `split_at_point`.
- **dashu-ratio** (0.3.0): **initial release** — basic arithmetic, numeric conversion,
  Diophantine approximations.
- **dashu-macros** (0.3.0): Const context limited to `u32`-fitting literals; `embedded` feature.

## 0.2.x

*crates: dashu-base, dashu-int, dashu-float, dashu-macros*

- **dashu-base** (0.2.1): `RootRem`/`Root` for unsigned primitives.
- **dashu-int** (0.2.1): `sqrt`/`sqrt_rem`/`nth_root`; `Sum`/`Product`; `num_traits`/
  `num_integer`/`num_order`/`zeroize` impls; serde `is_human_readable`.
- **dashu-float** (0.2.1): `Sum`/`Product`; `powf`; `sqrt`.
- **dashu-base** (0.2.0): `Approximation`, `Sign`, `EstimatedLog2`.
- **dashu-int** (0.2.0): Expose `Sign`/`DoubleWord`/`as_words`; const constructors;
  `split_bits`/`clear_high_bits`; `ilog`/`log2_bounds`; macros moved to a separate `dashu-macro`
  crate; `ZERO`/`ONE`/`NEG_ONE`; `pow` speed-up.
- **dashu-float** (0.2.0): **initial release** — basic arithmetic (`add`/`sub`/`mul`/`div`/
  `exp`/`ln`) and base conversion.
- **dashu-macros** (0.2.0): **initial release** — integer and float literal macros.

## 0.1.x

*crates: dashu-base, dashu-int*

- **dashu-base** (0.1.1): GCD algorithm bug fix.
- **dashu-int** (0.1.1): Modular inverse; `gcd`/`extended_gcd`.
- **dashu-base** (0.1.0): **initial release** — common trait definitions.
- **dashu-int** (0.1.0): **initial release** — ported from `ibig @ 0.3.5` with modifications
  (see `integer/NOTICE.md`).
