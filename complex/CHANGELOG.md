# Changelog

## Unreleased

### Change
- Use sign multiplication for the signed-zero square case and compare magnitudes without
  constructing redundant absolute-value temporaries in tests.
- `Context::mul`, `sqr`, `fma`, `norm`, `div` and `inv` (and the `*`, `/` operators) are now correctly rounded per component: each part is the exact result rounded once. Division falls back to the previous near-correctly-rounded Smith's method only when the exponents of the operand parts are astronomically far apart or `|w|²` leaves the exponent range.
- A result part that leaves the exponent range now saturates on its own (signed infinity / signed zero) instead of discarding the other, representable part; the whole result still collapses to `∞ + 0i` / `0 + 0i` when both parts do.
- `Display` in bases above 18 (where `i` is a digit) writes the imaginary term with an explicit `*i` suffix and an explicit coefficient (`2i*i`, `1*i`), and `FromStr` requires that suffix there.
- `Display` with the `+` flag signs only the leading term (`+1+2i`), and width/fill apply to the real coefficient only.
- Human-readable serde writes the struct form `{re, im, precision}` when the `a+bi` string cannot imply the precision (unlimited precision, or a part with more digits than the precision), and reads either form.
- `CBig::from_parts` rounds an unlimited-precision part to the shared precision when the other part's precision is limited.
- `Context::neg`, `conj`, `proj` and `mul_i` round to the context precision (and report `Inexact`) when it is narrower than the operand's.
- The saturated `tan` / `tanh` endpoint reports the `Rounding` adjustment relative to the
  truncated significand (inherited from dashu-float's `near_one_endpoint`).

### Fix
- `mul`/`sqr`/`fma` returned a wrong real part (and `Exact`) when the products nearly cancel; `fma` could lose the whole result.
- Complex results whose parts were truncated (`Inexact(_, NoOp)`) were tagged `Exact` (`mul`, `mul_real`, `sqr`, `fma`, `norm`, and transcendentals).
- `sqr` and `div` could round the wrong way under the directed modes.
- The sign of a zero part of a product depended on the operand order (`(-4+0i)*(1+0i)` had imaginary part `-0`).
- `z / z` returned a tiny nonzero imaginary part instead of exactly `1`.
- `0 / w` returned a zero with unlimited precision.
- `Context::div` reported `Indeterminate` instead of `InfiniteInput` for an infinite numerator over zero.
- `CBig::inv` returned `0` or `∞` for a representable reciprocal when `|z|²` leaves the exponent range.
- `CBig::from_parts` with a limited and an unlimited part could hold more digits than its precision (`into_parts` panicked in debug builds).
- `CBig::from_str` rejected a signed exponent in the imaginary coefficient (`1+2e-3i`) and the `inf` coefficients that `Display` writes (`1+infi`).
- `Display` with a precision rounded a negative imaginary part in the wrong direction under `mode::Down` / `mode::Up`.
- Human-readable serde raised the precision of values with letter digits (bases above 10) and turned unlimited precision into the digit count.
- The product of an empty `CachedCBig` iterator was `0` instead of `1`.
- Docs: `Context::max` treats unlimited precision as the lowest; `CBig::from(IBig/UBig)` has the integer's digit count as its precision, as `FBig::from` does.
- Complex results are reported `Exact` only when both parts are exact, and each part's rounding flag is the flag of the true value (a truncated part was reported `Exact`; `sqrt` of a perfect square, `powi(z, -1)` and real-axis `tan_pi` had wrong flags).
- `asin` / `acos` return for real arguments (`asin(0.5 + 0i)` did not), honour the sign of a zero part on their branch cuts and at zero (C99 Annex G), no longer fail with `OutOfDomain` for a large imaginary part or `Indeterminate` next to `±1`, and are correctly rounded for tiny arguments. `asinh` / `acosh` inherit the fixes (`acosh(1.5 + 0i)` was negative).
- `atan(±0 + iy)` for `|y| > 1` takes the sign of its real part `±π/2` from the real zero; `atanh` likewise on its cuts.
- The hyperbolic functions (`sinh`, `sinh_cosh`, `tanh`, `asinh`, `acosh`, `atanh`) round in the right direction under `mode::Down` / `mode::Up`.
- `tan` / `tanh` with a large imaginary (real) part round to the correct side of `±1` when the exact value lies above 1 in magnitude, and `tan_pi` with a large imaginary part no longer takes minutes under the directed modes.
- `tan_pi` next to a real-axis pole with a tiny imaginary part no longer returns `Indeterminate`.
- `log(-0 - 0i)` rounds its imaginary part `-π` in the mode's direction under `mode::Down` / `mode::Up`; `log` is correctly rounded when `|z|` is very close to 1.
- `sqrt` is correctly rounded when the root lies just below a power of the base, and `sqrt(-x - 0i)` has a `+0` real part.
- `exp` of an argument with more digits than the context precision no longer exhausts memory; `exp(±0 ± 0i)`, `powi(z, 0)` and `powf(z, 0)` return `1` at the operand's precision instead of unlimited precision, and `exp(+0 - 0i)` is `1 - 0i`.
- `powi` works on unlimited-precision values for non-negative exponents (`CBig::I.powi(2)`), returns for bases of the form `t·(1 − i)` with a negative exponent, and reports the sign of the result when a power of a negative real base overflows or underflows.
- `powf(0, w)` is `0` for `Re w > 0` (it was `OutOfDomain`); `powf` with an integer or dyadic-rational real exponent returns exact results exactly (`powf(i, 1)`, `powf(-1, 0.5)`) and no longer hangs when the result is exactly representable or a rounding tie.
- serde: the struct form of `CBig` can be deserialized from formats that provide owned or
  transient field names (e.g. CBOR).

## 0.6.2

### Change
- Migrated the ×2 radius folds (`sqrt`/`log`'s `2·rad/(m−rad)` brackets, the `sin`/`cos`
  `cosh·e^r·r` folds) from `Mag::mul_pow2(1)` to the new specialized `Mag::mul2()`. Pure
  call-site migration against float's shared internal type; no observable behavior change.

### Fix
- `tan` (and `tanh` through `−i·tan(i·z)`) no longer stalls on a large `|Im z|` (resp. large
  real part for `tanh`): the imaginary part `sinh 2y/(cos 2x + cosh 2y)` saturates at
  `sign(y)·(1 − δ)` with a sub-half-ulp residual, so the componentwise division's mid collapses
  onto exactly `±1` — the boundary of a one-sided directed-rounding preimage — which the Ziv
  containment can never certify (it climbed toward `~2|y|/ln B` working digits: an effective
  hang under directed modes at `|y| = 10⁵`, e.g. `CBig::tanh(±1755648)`). Beyond
  `|Im z| > (p·ln B)/2 + 1.5·ln 2` the saturated component is pinned to the mode-aware `±1`
  endpoint (nearest/outward keep `±1`; toward-zero modes step one ulp in) while the tiny other
  component — a genuine representable value, dashu has no underflow — keeps its own
  single-part Ziv loop on the same composition.

## 0.6.1

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
- **(breaking) `powi(z, ±1)` now respects the context precision** — it rounds the (exact)
  result to the context like every other precision-taking op, instead of returning the input
  at its own precision; `powi` on an unlimited-precision input with `|n| = 1` therefore
  panics like the rest of the family (a call that previously returned `Ok`).
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
  `(1+i)ⁿ` lattice (a certified real `tⁿ` scaled by the power of *two* `2^((n−r)/2)`, which is
  a bare exponent shift only in the binary base — the zero component of an even power is
  exactly zero), and the axis arguments of `log` through the fold below.
- **`atan` dispatches on the axes**, so its exactly-zero component certifies under the directed
  modes: `atan(x ± i·0) = atan(x) ± i·0` through the real `atan` kernel, and
  `atan(±0 + i·y) = ±0 + i·atanh(y)` for `|y| < 1` through the real `atanh`. Both are
  correctly rounded and exact in the zero component, which the two-log composition can never
  certify (its cancel-to-zero midpoint keeps a nonzero seed-rounding radius, and no interval
  fits inside the one-sided preimage of `+0`). The `±i` branch points, where `atan` diverges in
  every direction, stay with the general path (`Err(OutOfDomain)`, as before the dispatch).
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
