# Changelog

## Unreleased

## 0.6.0

### Change
- **(breaking, bound) the complex transcendentals now require `R: ErrorBounds`** — `exp`/`ln`/
  `powf`/`powi`, the trig and hyperbolic families (incl. inverses), `abs`/`arg`, and their
  `CachedCBig` forwarders, inherited from the float Ziv layer; field arithmetic stays `R: Round`.
- **(breaking) the complex Ziv driver reports `FpError::ZivRetryLimitExceeded`** when the retry budget
  is exhausted (was a silently possibly-1-ULP-wrong best-effort result).
- **(breaking) complex infinity is now a terminal value** — the single Riemann point `+∞ + i·0`,
  produced by finite blow-ups (`1/0`, `exp(+∞ + i·0)`, `log(0)`, overflow) but never accepted as an
  operand (`∞·z`, `z/∞`, `∞−∞`, `inv(∞)`, `log(∞)`, `sqrt(∞)` now reject with `InfiniteInput` instead
  of folding to `(+∞, +0)`); only `exp(±∞ + i·0)` and `proj` special-case it.
- Removed the `rustversion` dependency; the unversioned `rand` / `rkyv` feature aliases now select the
  newest versions (`rand_v010` / `rkyv_v08`).

### Add
- **Correct rounding for the complex transcendentals** via a Ziv retry loop (`complex/src/ziv.rs`) —
  `exp`, `ln`, `powf`, `powi`, trig + inverses, `sqrt` certify both parts against the float preimage.
  `tan` uses the cancellation-free double-angle form; `asin`/`acos` use the factored `(1-z)(1+z)`
  (Sterbenz-exact near `z = ±1`).
- **Hyperbolic & inverse-hyperbolic family** (`sinh`/`cosh`/`sinh_cosh`/`tanh`/`asinh`/`acosh`/
  `atanh`) via the rotation identities, with Annex-G signed-zero shortcuts.
- **Signed-zero preservation on exact-zero inputs** (`sin_cos`, `sqr`, `log`).
- serde / zeroize / rkyv 0.7 / rkyv 0.8 support for `CBig`.

### Fix
- **`Sum` for `CBig` is now correctly rounded** (exact-accumulates per axis instead of a componentwise
  fold).
- `no_std` build (the test-only Ziv counter is gated on `std`).
- `FromStr` returns `ParseError::InvalidSyntax` for structurally malformed input.

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
