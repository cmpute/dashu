# Changelog

## Unreleased

### Add
- **serde support for `CBig`** (`serde` feature): `Serialize`/`Deserialize` round-trips the real and
  imaginary parts (via the float `Repr` serde) and the shared precision; the rounding mode is a
  type parameter and is not serialized.
- **zeroize support for `CBig`** (`zeroize` feature): `Zeroize` on `CBig` and the complex
  `Context` (delegating to the wrapped float types).
- **rkyv 0.7 support for `CBig`** (`rkyv` feature, versioned `rkyv_v07`): archive via derive,
  delegating the parts to the float/int archives.
- **rkyv 0.8 support for `CBig`** (`rkyv_v08` feature): the same derive-based archive, via rkyv 0.8's
  `Place`-based derive. Requires Rust ≥ 1.81 (rkyv 0.8's MSRV); excluded from the 1.68 MSRV build.

## 0.6.0-rc.1

### Add
- **Correct rounding for the complex transcendentals** via a Ziv retry loop (`complex/src/ziv.rs`).
  `exp`, `ln`, `powf`, `sin`/`cos`/`tan`/`sin_cos`, `asin`/`acos`/`atan`, and `sqrt` now certify
  *both* the real and imaginary parts — rounding each to the target precision and retrying with more
  guard digits while either part's error interval straddles a rounding boundary — matching
  `dashu-float`'s real transcendentals. Each transcendental reports a provable per-part error radius
  (`result.ulp() × C`, plus an amplification term where the composition magnifies error: `log`'s
  `ln|z|` near `|z| = 1`, `powf`'s result magnitude). `tan` uses the cancellation-free double-angle
  form `(sin 2x + i·sinh 2y)/(cos 2x + cosh 2y)` (the naive `sin z/cos z` cancels in the real part
  for large `|Im z|`), so it is accurate for all finite `|Im z|`. `abs` delegates directly to
  `dashu-float`'s already-correctly-rounded `hypot`, dropping a double-rounding re-round. The
  well-conditioned regime is guaranteed-correctly rounded; `asin`/`acos` compute the inner `1-z²` in
  the factored form `(1-z)(1+z)` (Sterbenz-exact near the singularities `z = ±1`, where the direct
  `1 - z²` would cancel against the `sqr` rounding error), so they stay accurate right up to `z = ±1`.
  (`atan` was already sound near its singularities `z = ±i`: its `1 ± iz` is built from an exact
  rotation and an exact-significand add/sub, so there is no cancellation to factor out.)

### Change
- **(breaking, bound)** The complex transcendentals (`exp`, `ln`, `powf`, `powi`,
  `sin`/`cos`/`tan`/`sin_cos`, `asin`/`acos`/`atan`), `abs`, `arg`, and their `CachedCBig` forwarders
  now require `R: ErrorBounds`, inherited from `dashu-float`'s Ziv-backed real `exp`/`ln`/`hypot`/
  `sinh_cosh`/trig primitives. `powi` is now correctly rounded via its own Ziv loop (repeated
  squaring, with a per-part radius that scales with the squaring-compounding error and drops to zero
  when the chain is exact — required under the directed rounding modes, `CBig`'s default). The field
  arithmetic (`add`/`sub`/`mul`/`div`/`sqr`/`inv`) remains `R: Round`.
- `CBig`'s `FromStr` now returns `ParseError::InvalidSyntax` (new in `dashu-base`) for structurally
  malformed input — the MPC `(re im)` parenthesized form, more than one `i`, or a non-trailing `i`
  — instead of `ParseError::InvalidDigit`.

### Fix
- **`Sum` for `CBig` is now correctly rounded.** It exact-accumulates the real and imaginary parts
  independently via `FBig`'s exact-accumulating `Sum` (lossless `Repr` accumulation, a single final
  rounding per axis at the `Context::max` target), instead of the previous componentwise fold whose
  per-step rounding could lose low-order terms (e.g. `1e20 + 1 - 1e20` folded to `0` at low
  precision; it now correctly yields `1`). `Product` stays a fold, matching `FBig: Product`.
- **Unlimited-precision handling**, centralized in the Ziv driver: it asserts a limited context up
  front, so every transcendental panics on precision 0 as documented (the exact special-value
  shortcuts — `exp(0)=1`, `log(0)=-∞`, `powf(z,0)=1`, `sqrt(±0)`, `sin/cos(0)`, etc. — still bypass
  it). The prior per-function `guard()`/`assert_limited()` scaffolding is consolidated (`guard()` is
  renamed `work_context()`).
- **Arithmetic at unlimited precision is correct.** `mul`/`sqr`/`norm` use `Context::work_context`,
  which is the exact `self.float()` (precision 0) at unlimited, so they are exact there. `div`/`inv`
  use the same path and panic at unlimited via `dashu-float`'s `div` (a quotient isn't exactly
  representable in general). `abs` asserts limited before delegating to the float `hypot`.
- **`no_std` build of the test-only Ziv retry counter.** The `LAST_ZIV_RETRIES` `thread_local!`
  requires `std`, so the crate failed to compile under `--no-default-features`. It's now gated behind
  `feature = "std"` (with its uses and the counter-reading tests), so the Ziv driver is `no_std`-clean.

## 0.5.2

### Add
- `CBig::fma` / `Context::fma`: fused complex multiply–add `z1·z2 + sign·z3`,
  computed as chained real FMA per component (sign scales `z3`).

### Change
- Complex `mul` and `div` (Smith's method) now use real FMA to fuse each cross
  product with its add/subtract (one rounding instead of mul-then-add/sub's two),
  preserving the cancellation structure of `xu − yv` and the division numerators.
  `sqr` and `norm` (`x² ± y²`) keep the dedicated `sqr()` kernel, which is faster
  than the general product path inside `fma`.
- `Context::fma`'s final `± z3` now routes through `dashu-float`'s new
  `Context::addsub_vr` kernel, collapsing the former `match sign { add, sub }`
  into a single signed call and consuming the `z1·z2` component (no clone).

## 0.5.1

### Add
- `CBig::from_parts_const`: a `const`-evaluable constructor taking `(sign, significand, exponent)`
  parts for each of the real/imaginary components (built on `Repr::new_const`). The `cbig!` literal
  macro now works in `const` position for coefficients that fit in a `DoubleWord`; larger
  coefficients fall back to the runtime heap path.

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
