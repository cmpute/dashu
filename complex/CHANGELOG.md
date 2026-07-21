# Changelog

## Unreleased

### Add
- `CBig::from_parts_const`: a `const`-evaluable constructor taking `(sign, significand, exponent)`
  parts for each of the real/imaginary components (built on `Repr::new_const`). The `cbig!` literal
  macro now works in `const` position for coefficients that fit in a `DoubleWord`; larger
  coefficients fall back to the runtime heap path.
- **Correct rounding for the complex transcendentals** via a Ziv retry loop (`complex/src/ziv.rs`).
  `exp`, `log`, `powf`, `sin`/`cos`/`tan`/`sin_cos`, `asin`/`acos`/`atan`, and `sqrt` now certify
  *both* the real and imaginary parts — rounding each to the target precision and retrying with more
  guard digits while either part's error interval straddles a rounding boundary — matching
  `dashu-float`'s real transcendentals. Each transcendental reports a provable per-part error radius
  (`result.ulp() × C`, plus an amplification term where the composition magnifies error: `log`'s
  `ln|z|` near `|z| = 1`, `powf`'s result magnitude). `tan` uses the cancellation-free double-angle
  form `(sin 2x + i·sinh 2y)/(cos 2x + cosh 2y)` (the naive `sin z/cos z` cancels in the real part for
  large `|Im z|`), so it is accurate for all finite `|Im z|`. `abs` delegates directly to
  `dashu-float`'s already-correctly-rounded `hypot`, dropping a double-rounding re-round. The
  well-conditioned regime is guaranteed-correctly rounded; `asin`/`acos`/`atan` lose accuracy only
  very near their singularities (`z = ±1`/`±i`), a known limitation of the underlying formulas.

### Change
- **(breaking, bound)** The complex transcendentals (`exp`, `ln`, `powf`, `sin`/`cos`/`tan`/
  `sin_cos`, `asin`/`acos`/`atan`) and their `CachedCBig` forwarders now require `R: ErrorBounds`,
  inherited from `dashu-float`'s Ziv-backed real `exp`/`ln`/`sinh_cosh` primitives. `powi` and the
  field arithmetic (`add`/`sub`/`mul`/`div`/`sqr`/`inv`) remain `R: Round`.

### Fix
- **Unlimited-precision handling**, centralized in the Ziv driver: it asserts a limited context up
  front, so every transcendental panics on precision 0 as documented (the exact special-value
  shortcuts — `exp(0)=1`, `log(0)=-∞`, `powf(z,0)=1`, `sqrt(±0)`, `sin/cos(0)`, etc. — still bypass
  it). The prior per-function `guard()`/`assert_limited()` scaffolding is removed (`guard()` itself
  is gone, now unused).
- **Arithmetic at unlimited precision is correct.** `mul`/`sqr`/`norm` use `Context::work_context`,
  which is the exact `self.float()` (precision 0) at unlimited, so they are exact there. `div`/`inv`
  use the same path and panic at unlimited via `dashu-float`'s `div` (a quotient isn't exactly
  representable in general). `abs` asserts limited before delegating to the float `hypot`.
- **`no_std` build of the test-only Ziv retry counter.** The `LAST_ZIV_RETRIES` `thread_local!`
  requires `std`, so the crate failed to compile under `--no-default-features`. It's now gated behind
  `feature = "std"` (with its uses and the counter-reading tests), so the Ziv driver is `no_std`-clean.

## 0.5.0 (Initial release)

`dashu-cmplx` provides [`CBig`], an arbitrary-precision complex number type built on top of
[`dashu-float`]'s `FBig`. Each `CBig` stores a real and an imaginary part over a single shared
precision and rounding mode, mirroring `FBig`'s `Repr`+`Context` layout.

- **Two-layer API** mirroring `FBig`: context-layer operations on [`Context`] return a `CfpResult`
  carrying per-axis inexactness, while the convenience layer (`CBig::add`, operators) unwraps to a
  plain `CBig` (panicking on domain errors, saturating `Overflow`/`Underflow`).
- **Field arithmetic**: `add`/`sub`/`neg`/`sqr`/`mul`/`div`/`inv` plus scalar `mul`/`div` by a real
  `FBig`. `mul`/`div`/`sqr`/`inv` are near-correctly rounded via the guard-digit recipe.
- **Power**: integer `powi` (repeated squaring) and complex `powf` (`exp(w·log z)`).
- **Decomposition & misc**: `re`/`im`/`into_parts`/`from_parts`, `conj`/`proj`/`mul_i`, `abs`
  (`hypot`), `norm` (squared modulus), `arg` (`atan2`).
- **Transcendentals**: `sqrt`, `exp`, `ln`, `sin`/`cos`/`tan`/`sin_cos`, `asin`/`acos`/`atan`.
- **Comparison surface** mirroring `FBig`: lexicographic `Ord`/`PartialOrd`, `AbsOrd`, and
  `NumOrd`/`NumHash` (behind the `num-order` feature).
- **Formatting**: algebraic `"a+bi"` `Display`/`FromStr`, structured `Debug`, and the `I`/`ZERO`/
  `ONE`/`NEG_ONE` constants.
- **`CachedCBig`** — a cache-backed variant of `CBig` mirroring `CachedFBig`, threading a shared
  `ConstCache` through the transcendentals so real constants (π, ln2, ln10, …) are reused across a
  computation chain. `!Send + !Sync`. The meta-crate gains a `dashu::FastComplex` alias.
- **`cbig!`/`static_cbig!` literal macros** (in `dashu-macros`), exposed as `dashu::cbig!`.
- **Random generation** via the `rand` feature (aliasing `rand_v08`, with `rand_v09`/`rand_v010`
  opt-in): `UniformCBig` samples the box `[low, high)`, and the builtin distributions sample the unit
  square.
- **`num-complex` interop** — `TryFrom` conversions between `CBig` and `num-complex`'s
  `Complex<f32>`/`Complex<f64>`.
- **No-NaN policy**: C99 NaN-producing cases are mapped to `FpError` at the context layer (panics at
  the convenience layer), consistent with `FBig`. Signed zero and the C99 Annex G / Kahan branch-cut
  model are first-class.
