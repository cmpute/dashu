# Changelog

## Unreleased

### Change
### Change
- **`CBig`'s `Debug` renders its parts as raw `Repr`s** (`significand * base ^ exponent`,
  one trailing context `prec`) instead of `FBig`'s `Display`: a base-2 part's `Display` is
  native binary positional (`0.1` = one half), which reads as garbage when glanced at as
  decimal (`re:0.001011…` for one half).
- **The transcendental radius estimates are now mechanically propagated** instead of
  hand-written `ulp·k` constants: `exp`, `log`, `sqrt`, `sin`/`cos`, `tan`, `sin_pi`/`cos_pi`,
  `tan_pi`, `powi` and `powf` compose through a complex ball (`CBall` over float's `Ball`/`Mag`,
  shared `#[doc(hidden)]` in lockstep), whose radius tracks every rounding of the composition
  and grows exactly where it is amplified (`y/(2a)` in `sqrt`, `‖z‖` near 1 in `log`, the
  kernel input folds for over-precise inputs, `‖w·log z‖` in `powf`). An exactly-representable
  result certifies through a zero radius, fixing `ZivRetryLimitExceeded` on inputs like
  `√4`/`√(3+4i)`/`acos(1)` under the outward rounding modes. The directed-rounding fuzz
  differentials assert **bit-exact** per-component agreement with MPC across
  `Up`/`Down`/`Zero`/`HalfEven` for every family except the ×π family (no MPC entry — a
  premultiplied-π reference tests a different function) and `powf` (MPC's `pow` is not
  guaranteed correctly rounded).
- **`powi(z, ±1)` now respects the context precision** — it rounds the (exact) result to the
  context like every other precision-taking op, instead of returning the input at its own
  precision; `powi` on an unlimited-precision input with `|n| = 1` therefore panics like the
  rest of the family.
- **A `-0` component of a complex value renders with its sign**, following `dashu-float`: the
  components are formatted by `FBig`, whose `Display`/`LowerExp` now print `-0` rather than `0`
  (and `FromStr` parses it back as negative zero). Nothing numeric changes.

### Add
- ×π trigonometric functions `sin_pi`/`cos_pi`/`sin_cos_pi`/`tan_pi` (of `z·π`), on `Context`,
  `CBig` and `CachedCBig`. Pure-real arguments reduce exactly through the real ×π kernels
  (quarter-integer exact cases included); the imaginary part composes the real
  `sinh_cosh_pi`. `tan_pi` uses the same cancellation-free double-angle identity as `tan`, and
  reports `Err(Indeterminate)` at the real-axis poles (`y = 0`, x an odd multiple of `1/2`).

### Fix
- **`powi` of a base whose parts round onto the diagonal at the working precision certified a
  wrong real part**: `powi((−2.826, −2.826'), −10)` at 20 bits returned `re = 0` where the true
  value is `7.6·10⁻²²` (~2⁶⁹ ulps off). The `a² − b²` cancellation collapsed onto exact zero and
  the driver's ±ulp preimage of ±0 — an f64-style artifact (subnormals bound the exponent range
  there; at unbounded exponents no nonzero real rounds to zero) — admitted the honest-but-nonzero
  radius. The Ziv driver now carries `dashu-float`'s zero-candidate guard: a zero candidate is
  certifiable only by a zero radius, and the structural zeros the closures own are exact —
  `powi` of a base with *identical* parts `t·(1+i)` computes through the exact
  `(1+i)ⁿ` lattice (a certified real `tⁿ` scaled by a power of two, the zero component of an
  even power exactly zero), and the axis arguments of `log` through the fold below.
- **`log`'s argument fold is the componentwise gradient bound**
  `(|y|·rad_x + |x|·rad_y)/‖z‖²` (was the joint 1-Lipschitz `(rad_x + rad_y)/‖z‖`): it vanishes
  exactly on the axes — the angle is invariant along the real error direction when `y ≡ 0`,
  which is what makes `asin(±i)`'s exactly-zero real part certifiable — and is tighter than the
  joint bound off-axis by the component ratio.
- **`tan`/`tan_pi` near the real-axis poles no longer error or panic on finite,
  correctly-roundable inputs.** The double-angle denominator `cos(2x) + cosh(2y)` cancels
  into its addends' rounding noise near the poles: the composed ball division previously
  surfaced a terminal `Err(Indeterminate)` (`0/0`) or an infinity that panicked the part
  arithmetic (`x/0`) — e.g. `tan_pi(0.5 + 1e-40·i)` (true value ≈ `3.18e39·i`) errored, and
  `tan_pi((0.5 + 1e-60) + 1e-40·i)` panicked. A collapsed — or deeply cancelled — denominator
  now exports the whole-line ball (an unbounded radius that no Ziv attempt can certify), so
  the loop retries at a higher guard, where the strictly positive true denominator
  (`D = cos 2x + cosh 2y > 0` for `y ≠ 0`) re-emerges above its addends' rounding noise; the
  genuine 0/0 at an exact pole still reports `Err(Indeterminate)`.
- **An exactly-zero result part carries a zero error radius**, so it certifies under the
  directed modes: the one-sided rounding preimage of `+0` (`[0, ulp)` under `Down`) fits no
  nonzero symmetric interval. In the ball composition the property is mechanical — an exact
  factor times anything is an exact product, and the kernel's exact values (e.g.
  `cos_pi(0.5) = 0`) seed radius-0 balls — verified by the directed ×π sweep on
  `sin_pi(0.5 + 1e-8·i)`, which previously could not certify under directed modes.

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
