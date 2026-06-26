# dashu-cmplx — Design & Implementation Plan (v0.5)

Last updated: 2026-06-26
Status: **design (pre-implementation)**. Gated by `TODO-v05.md` Phase 0 (✅ done) and Phase 2
(✅ done, #83). This document specifies the **`CBig`** arbitrary-precision complex type — the
headline new crate for the 0.5 release and the Rust-native alternative to **GNU MPC**.

> **Naming.** The package is `dashu-cmplx` (library `dashu_cmplx`); the source directory and the
> meta-module use the full word `complex` — directory `complex/`, meta-module `dashu::complex` —
> mirroring `dashu-ratio` (package `dashu-ratio`, directory `rational/`, module `dashu::rational`).
> The type alias is `dashu::Complex = CBig`. (`TODO-v05.md` Phase 3 uses `complex/`, which this plan
> follows.)

---

## 0. Purpose & scope

`dashu-cmplx` provides arbitrary-precision **complex** numbers, `CBig`, built on top of
`dashu-float`'s `FBig`. Each `CBig` is a pair of `FBig` parts (`re`, `im`) sharing one precision and
one rounding mode. The crate targets **MPC parity for the "common functionalities"** (field
arithmetic + elementary transcendentals + abs/arg/conj/proj + I/O), with **near-correct rounding**
(guard-digit heuristic, same guarantee class as `dashu-float`'s transcendentals; a guaranteed-correct
Ziv loop is deferred to 0.5.x — see Principle 2), following the **C99 Annex G / Kahan** branch-cut and
signed-zero model that `dashu-float` already implements for reals.

**In scope for 0.5:** construction/conversion, `add/sub/mul/div/neg/sqr`, scalar mul/div by real,
`abs`/`norm`/`arg`/`conj`/`proj`/`mul_i`, `sqrt`/`exp`/`log`/`pow` (complex & integer exponent),
`sin`/`cos`/`tan`/`sin_cos`, `asin`/`acos`/`atan`, `cmp`/`cmp_abs`, `Display`/`Debug`/`FromStr` in
algebraic `a+bi` form, workspace + meta-crate wiring.

**Deferred to 0.5.x** (Section 13): complex hyperbolics & inverse hyperbolics, `fma`, `rootofunity`,
`agm`, `exp2/exp10/log2/log10`, vector ops, `CBig` serde/rkyv, ball arithmetic, independent re/im
rounding, `num_complex` interop, a guaranteed-correct Ziv rounding loop, a
`CachedCBig`/`ComplexFloat` layer.

**Hard constraints (from `AGENTS.md`):** MSRV 1.68 (do not bump), Rust edition 2021, `no_std` +
`alloc` on the default path, `--exclude dashu-python` for workspace commands, every change in
`complex/CHANGELOG.md` `## Unreleased`, no external-library function names in code comments (describe
algorithms in our own terms), algorithm-kernel tests inline as `#[cfg(test)] mod tests`, double-word
first-class citizen.

---

## 1. Design principles

1. **Reuse `FBig`, do not re-derive real math.** Every real transcendental `CBig` needs
   (`exp`, `ln`, `sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `atan2`, `sqrt`, `powi`, `powf`,
   `inv`, `sqr`, `sinh`, `cosh`) already exists on `Context<R>`. `CBig` composes these; it does **not**
   re-implement real arithmetic. (Phase 0 verified these for real inputs — that gate is the whole reason
   `CBig` can lean on them.) These `FBig` primitives use a guard-digit heuristic (see Principle 2), so a
   `CBig` built on them inherits the same *near-correctly-rounded* guarantee class, not a stronger one.

2. **Near-correct rounding via guard digits (mirroring `FBig`).** Each component is evaluated at
   `p + g` guard digits and re-rounded to `p` (`with_precision`) — exactly the technique `FBig`'s own
   transcendentals use today (`Context::<R>::new(p + g)` with a small fixed `g`, e.g. `p + 50` for the
   hyperbolics, `p + 10 + ⌈log2 p⌉` for `powf`). The per-component re-round is delegated to `FBig`, so
   `CBig` needs no bespoke fixed-width rounding code per function. **This is not a Ziv loop**: there is
   no retry when the residual straddles a rounding boundary, so results are *near-correctly-rounded*
   (correct for the overwhelming majority of inputs, at most ~1 ulp off on a rare hard-to-round input) —
   the same guarantee class `FBig`'s transcendentals already carry. A guaranteed-correct Ziv retry loop
   is **deferred to 0.5.x** (§13), and is expected to land in `FBig` first so `CBig` simply inherits it.
   The bounds that size `g` are known and tight regardless: complex `mul` has normwise relative error
   `< √5·u` (Brent–Percival–Zimmermann; `< 2u` with an FMA-style kernel), and complex `div`
   `< (3+√5)·u` naively / `5u` with FMA — so a small fixed guard settles the overwhelming majority of
   inputs today, and the deferred Ziv loop only tightens the hard-to-round tail. (Section 6.1.)

3. **One rounding mode `R` for both parts (uniform), matching `FBig`'s single-context model.**
   `CBig<R, B>` carries a single `R: Round` applied independently to the real and imaginary
   components. This is simpler than MPC's per-axis `mpc_rnd_t` pair and is sufficient for correctness
   (each component is still rounded independently). MPC-parity independent re/im rounding is a deferred
   `CRound`-trait enhancement (Decision 1, with recommendation).

4. **No NaN — reuse the `FpResult`/`FpError` machinery.** `FBig` deliberately has **no NaN** ( infinities
   are terminal, errors are `FpError`). C99 complex arithmetic *would* produce NaN in several cases
   (`0/0`, `∞−∞`, `0·∞`, `log(0)`, etc.). `CBig` maps those to `FpError` variants
   (`Indeterminate` / `OutOfDomain` / `InfiniteInput`) at the context layer, and to panics at the
   convenience layer — exactly mirroring how `FBig` already behaves (Section 4). We do **not** invent
   a complex-only NaN.

5. **C99 Annex G / Kahan branch cuts and signed zeros are a first-class correctness requirement, not
   an edge case.** `FBig`'s `Repr` already encodes `±0` (sentinel exponents `0` / `-1`) and `±∞`
   (`isize::MAX` / `MIN`) and exposes `is_neg_zero()` / `is_infinite()` / `sign()` predicates. `CBig`
   uses these to reproduce the Annex G value tables exactly. Signed zero is *load-bearing* for complex
   continuity across branch cuts (e.g. `sqrt(conj z) == conj(sqrt z)`).

6. **Complex trig via the real–imaginary decomposition, reusing `FBig`'s hyperbolics.** Real
   hyperbolics (`sinh`/`cosh`/`tanh`/`asinh`/`acosh`/`atanh`) now live on `Context<R>` (merged from
   `develop`), so `CBig` evaluates `sin(x+iy) = sin x·cosh y + i·cos x·sinh y` (and the matching `cos`
   form) directly. This avoids the `exp(±iz)` form's exponential blow-up for large `|Im z|` and reuses
   `FBig`'s cancellation-free `sinh`/`cosh` (built on `exp_m1`); the `exp(±iz)` identity is kept only as
   a cross-check in tests. (Section 6.2.)

7. **Encapsulated fields, accessor API.** `re`/`im` are **private** with `re()`/`imag()`/`into_parts()`/
   `from_parts()` accessors, consistent with `FBig`'s private `repr`. This preserves the shared-precision
   invariant. (num-complex uses public fields; we diverge for the same reason `FBig` does — Open
   Decision 3, with recommendation.)

---

## 2. Type & context model

```rust
/// Arbitrary-precision complex number: two `FBig` parts sharing one precision and one rounding mode.
pub struct CBig<R: Round = mode::Zero, const B: Word = 2> {
    pub(crate) re: FBig<R, B>,
    pub(crate) im: FBig<R, B>,
}
```

- **`R: Round`** (default `mode::Zero`, same default as `FBig`) — the rounding mode applied to both
  components. The available modes are `dashu_float::round::mode::{Zero, Away, Up, Down, HalfEven,
  HalfAway}`.
- **`const B: Word`** (default `2`) — the radix, identical to `FBig`'s `B`. Both parts share `B`.
  No decimal alias is defined (there is almost no need for a base-10 `CBig`); callers wanting a decimal
  complex value use `CBig::<mode::HalfAway, 10>` explicitly. (Decision 6.)
- **Precision is uniform.** Both parts always carry the same `Context<R>` (same precision). The
  invariant is enforced by every constructor and operation. (MPC allows different re/im precisions;
  we start uniform — simpler, matches `FBig`'s single-context model. `TODO-v05.md` §3.1.)

- **`CBig` is `Clone`, not `Copy`** — same as `FBig` (the `IBig` significand is heap-allocated).
  Operations take `&self`/`&CBig` and return owned `CBig`, mirroring `FBig`'s convenience methods.

**Context access:**
```rust
impl<R: Round, const B: Word> CBig<R, B> {
    pub fn context(&self) -> Context<R>;          // == self.re.context() == self.im.context()
    pub fn precision(&self) -> usize;             // base-B digit count; 0 = unlimited
}
```

The precision is read from either part (they agree). Operations create result `CBig`s at
`Context::max(lhs.context(), rhs.context())`, again mirroring `FBig`.

---

## 3. Rounding & inexactness model

### 3.1 Rounding
A complex op rounds the **real and imaginary components independently**, each with the single mode
`R`. Correctness of each component is just `FBig`'s already-correct rounding; the complex layer's job
is to *feed each component enough guard precision* that the final `p`-digit round is the true round
(Section 6.1). No new rounding machinery is introduced; `dashu-float`'s `Round` trait and `Context`
are reused as-is.

### 3.2 Inexactness (the `CRounded` dual flag)
MPC returns **two** inexact flags (one per axis). dashu's analog is **not a new type** — it reuses
`dashu_base::Approximation` (the complex twin of `Rounded<T> = Approximation<T, Rounding>`), with a
**tuple of two** `Rounding` flags (one per part) as the error:

```rust
/// Correctly-rounded complex result with per-axis inexactness.
/// `Exact(v)` ⟺ both parts exact; `Inexact(v, (re, im))` carries each part's rounding direction.
pub type CRounded<R, const B: Word> = Approximation<CBig<R, B>, (Rounding, Rounding)>;
```

(Reuses `dashu_float::Rounding { NoOp, AddOne, SubOne }`; `NoOp` ≡ exact. Inherits `.value()` /
`.error()` / `.map()` / `?` (`Try`) for free from `Approximation`, exactly like `Rounded<T>`.)

### 3.3 Result/error type
```rust
pub type CfpResult<R, const B: Word> = Result<CRounded<R, B>, FpError>;
```
Named `CfpResult` to mirror `dashu_float::FpResult` (the `C` prefix is the complex analog). Reuses
`dashu_float::FpError` unchanged (Section 4). The **convenience layer** (`CBig::mul`, etc.) unwraps
via the same policy as `FBig` (`unwrap_fp`: `Overflow(s)→±∞`, `Underflow(s)→±0`, panic on
`Indeterminate`/`OutOfDomain`/`InfiniteInput`) and returns `CBig` directly.

### 3.4 Two-layer API (mirror `FBig`)
| Layer | Where rounding lives | Return type | On error |
|---|---|---|---|
| **Context** (`CBig::mul` is sugar; the real work is a free/kernel `fn`) | mode `R`, precision from the context | `CfpResult` (`Result<CRounded, FpError>`) | `Err(FpError)` |
| **Convenience** (`z.mul(&w)`, operators `z * w`) | same | `CBig` | panics (`unwrap_fp` policy) |

This is the exact split `FBig` uses (`Context::mul → FpResult<FBig>` vs `FBig::mul → FBig`).

---

## 4. Special-value & error model (the no-NaN policy)

`FBig` has `+0`, `-0`, `+∞`, `-∞`, and **no NaN**. C99 complex arithmetic produces NaN in many
situations; `CBig` translates them to `FpError` instead. The full mapping is an explicit, tested table
(Section 8, `tests/special_values.rs`), but the rule set is:

| C99 result | `CBig` behavior | `FpError` / value |
|---|---|---|
| finite, well-defined | normal | `CRounded` with the `CBig` |
| `0/0`, `∞−∞`, `0·∞` (indeterminate) | error | `FpError::Indeterminate` |
| `log(0)` | real `ln(0)` underflows | real part `-∞`; reported via the real op's behavior |
| `sqrt(z)` on the branch cut | defined via signed zero | finite result (Annex G table) |
| result magnitude too large | saturate | `Overflow(Sign)` → `±∞` (convenience) |
| result magnitude too small | flush | `Underflow(Sign)` → `±0` (convenience) |
| infinite operand fed to a non-terminal-tolerant op | error | `FpError::InfiniteInput` |

**Signed zero is preserved and meaningful** (it selects the side of a branch cut). All Annex G
special-value rules for `add`/`sub`/`mul`/`div`/`sqrt`/`log`/`arg`/`proj`/`conj`/`mul_i` are encoded
as a short-circuit **before** the numeric path (Section 6.3), using `FBig`'s `is_neg_zero()` /
`is_infinite()` / `sign()` predicates. **`proj`** maps any part-infinite value to `+∞ + i·0` (the
Riemann point at infinity), preserving the sign of the imaginary zero per Annex G.

---

## 5. Public API surface

### 5.1 Construction & conversion
```rust
CBig::from_parts(re: FBig<R,B>, im: FBig<R,B>) -> Self;   // precision = max; asserts share R/B
CBig::from_real(re: FBig<R,B>) -> Self;                    // im = +0
CBig::from_int(n: IBig) -> Self;                           // re = convert_int(n), im = +0
CBig::I, CBig::ZERO, CBig::ONE;                            // constants (Section 5.7)

FromStr  // "a+bi" algebraic form (Section 5.6)
TryFrom<f32>/<f64>          // (base 2) exact-or-LossOfPrecision
TryFrom<num_complex::Complex<FBig>>   // feature num_complex_v04 (deferred)
```

### 5.2 Field arithmetic
```rust
// context-layer: CfpResult ; convenience-layer (operators): CBig
add(&self, rhs) / sub / neg
mul(&self, rhs) / sqr(&self)               // near-correctly rounded (Section 6.2)
div(&self, rhs)                             // near-correctly rounded (Smith's method + guard re-round)
inv(&self)                                  // 1/z = conj(z)/|z|²
mul_i(&self, negative: bool)               // ×(±i): exact rotation (re,im)→(∓im,±re)
powi(&self, n: IBig) -> Self               // integer power: repeated squaring (cheaper, branch-cut-free)
powf(&self, w: &Self) -> Self              // complex^complex: exp(w·log z)
```
Operator overloads: `Add/Sub/Mul/Div/Neg` for `CBig op CBig` (all four ref/val combinations, via the
existing `forward_all_binop!`-style macro if present, else explicit impls); scalar mul/div by a real
`FBig` is done through **mixed-type operators**, not named methods — `Mul<FBig>`/`Div<FBig>` for `CBig`
(`z * r`, `z / r`) and `Mul<CBig>`/`Div<CBig>` for `FBig` (`r * z`, `r / z`). No standalone
`scale`/`unscale` methods. `AddAssign`-style for the owning variants.

### 5.3 Comparison
```rust
PartialEq/Eq                       // componentwise exact equality (+0 == −0 per component, like FBig)
PartialOrd?                        // complex total order is NOT natural — DO NOT impl Ord
cmp_abs(&self, rhs) -> Ordering    // compare by |z| (uses AbsOrd; match MPC's mpc_cmp_abs)
NumOrd / NumHash                   // numeric comparison/hashing, like FBig/RBig/IBig (dashu_base)
is_zero(&self) / is_infinite(&self) / is_finite(&self)
```
Per `TODO-v05.md` §3.2 and MPC: **no total `Ord`**. Provide `cmp_abs` (modulus comparison) and
equality only. **`NumOrd`/`NumHash`** are implemented too, matching the other numeric types in the
suite: `NumOrd` orders `CBig` lexicographically by `(re, im)` (a stable total order for the
numeric-comparison machinery — not a mathematical order), and `NumHash` is consistent with `PartialEq`.
*(Note: `dashu-base` currently defines `NumOrd` and `NumHash` but has no `NumEq` trait; if a numeric
(tolerant) equality trait is wanted for `CBig`, `NumEq` would need to be added to `dashu-base` first —
tracked as a follow-up, not blocking 0.5.)*

### 5.4 Decomposition & misc
```rust
re(&self) -> &FBig      /  imag(&self) -> &FBig
into_parts(self) -> (FBig, FBig)
conj(&self) -> Self                         // re − i·im : exact (flip sign of im, incl. −0/∞)
proj(&self) -> Self                         // Riemann projection (Section 4)
abs(&self) -> FBig                          // |z| = hypot(re,im) : HARD, near-correctly rounded (Section 6.2)
norm(&self) -> FBig                         // re² + im² : cheap, near-exact (no sqrt)
arg(&self) -> FBig                          // atan2(im, re) ∈ ]−π, π] : branch cut (−∞,0]
```
Naming follows `TODO-v05.md` §3.2: `abs` = modulus (`|z|`), `norm` = **squared** modulus (the cheap
`re²+im²`, matching num-complex's `norm_sqr`). This split matters: `norm` avoids the expensive,
hard-to-round `sqrt`.

### 5.5 Powers & transcendentals
```rust
sqrt(&self) -> Self        // principal; Re ≥ 0; Re=0 ⇒ Im ≥ 0; cut on ]−∞,0]
exp(&self) -> Self         // e^re·(cos im + i sin im)  — reuses FBig exp/sin_cos
log(&self) -> Self         // ln|z| + i·arg(z); cut on ]−∞,0]; Im ∈ ]−π, π]
pow (see 5.2)
sin(&self) / cos(&self) / tan(&self) / sin_cos(&self) -> (Self, Self)   // via real sin/cos + sinh/cosh
asin(&self) / acos(&self) / atan(&self)                                 // via log + sqrt (Kahan)
```
Identities implemented in terms of `FBig` (real hyperbolics now available on `Context<R>`):
- `exp(x+iy) = e^x·(cos y + i sin y)`
- `log z = ln|z| + i·arg z`
- `sin(x+iy) = sin x·cosh y + i·cos x·sinh y`,
  `cos(x+iy) = cos x·cosh y − i·sin x·sinh y`, `tan z = sin z / cos z` (or the direct
  `(sin 2x + i·sinh 2y)/(cos 2x + cosh 2y)` form)
- `asin z = −i·log(iz + sqrt(1−z²))`, `acos z = −i·log(z + i·sqrt(1−z²))` (or `π/2 − asin z`),
  `atan z = (i/2)·(log(1−iz) − log(1+iz))`

### 5.6 I/O (`Display` / `Debug` / `FromStr`)
> **Divergence from MPC.** MPC uses the parenthesized pair `"(re im)"`; `dashu-cmplx` instead uses the
> algebraic `"a+bi"` notation (the `num-complex` idiom) for `Display`/`FromStr`, which is far more
> readable for humans. The parenthesized form is **not** accepted on input.

- **`Display`**: algebraic `"a+bi"` — e.g. `"1+2i"`, `"-3-4i"`, `"5"` (pure real), `"-7i"` (pure
  imaginary), `"i"` (= `0+1i`), `"-i"` (= `0-1i`). Each coefficient uses `FBig`'s native `Display`
  (specials render as `inf`/`-inf`); the imaginary term always carries an explicit sign, a unit
  coefficient is elided (`1i` → `i`), and a zero imaginary is omitted (pure real).
- **`Debug`**: structured `"re:<re> im:<im> (prec: <p>)"` — e.g. `"re:1.5 im:-2.0 (prec: 53)"`, for
  quick inspection (mirrors `FBig`'s `Debug` style).
- **`FromStr`**: parses exactly the `"a+bi"` grammar `Display` emits — an optional real term and an
  optional `"<sign><coeff>i"` imaginary term (at least one required; bare `"i"`/`"-i"` = `±1·i`;
  `"5"` = `5+0i`). Each coefficient parses via `FBig`'s `FromStr` (so `inf`/`-inf`, not MPC's
  `@Inf@`). `ParseError` on anything malformed — **including** the `"(re im)"` parenthesized form.

### 5.7 Constants
```rust
CBig::ZERO  // 0 + 0i
CBig::ONE   // 1 + 0i
CBig::I     // 0 + 1i  (imaginary unit)
```
All built from `FBig::ZERO`/`FBig::ONE`. No `INFINITY` constant in the C99 "two infinities" sense;
complex infinity is the single Riemann point produced by `proj` (`+∞ + i·0`). `π`/`e` are obtained
via `FBig::pi(precision)` / `FBig::ONE.exp()` as needed, not stored on `CBig`.

### 5.8 Operator overloads & traits
`Clone, Debug, Display, Default` (= ZERO), `PartialEq, Eq`, `FromStr`. `Add/Sub/Mul/Div/Neg` and their
`Assign` forms. Standard `core::hash::Hash` is **omitted** (matching `FBig` — hashing arbitrary floats
is ill-defined); the numeric hash is exposed via `NumHash` instead (§5.3). `Sum`/`Product` for iterators
(mirror `FBig`).

---

## 6. Algorithms & rounding strategy

### 6.1 The guard-digit recipe (near-correct rounding, mirroring `FBig`)

Every non-trivial `CBig` op is computed by the same skeleton, parameterized by a *formula* and a
*guard width* `g`:

```
fn complex_op<R,B>(z, w, ctx) -> CfpResult {
    // 1. Short-circuit special values (Annex G table) — see 6.3.
    if let Some(special) = annex_g_shortcut(z, w) { return special; }

    // 2. Evaluate the formula once at p + g guard digits (single pass, no retry).
    let p = ctx.precision();
    let g = guard_for(formula, p);             // a few base-B digits, sized from the error bound
    let gctx = Context::<R>::new(p + g);       // FBig, near-correctly-rounded at p+g
    let (re_hi, im_hi) = formula(z, w, gctx)?; // each component is an FBig at precision p+g

    // 3. Re-round each component to p with mode R (delegated to FBig).
    let re_lo = re_hi.with_precision(p);       // Approximation<FBig, Rounding>
    let im_lo = im_hi.with_precision(p);
    Ok(CRounded::Inexact(CBig::from_parts(re_lo.value(), im_lo.value()),
                         (re_lo.error(), im_lo.error())))  // Exact(v) if both flags are NoOp
}
```

- `formula` is built **only** from `FBig` ops at precision `p+g`.
- `g` is a small **fixed** guard chosen from the formula's per-component error bound (complex `mul`
  ≤ ~√5·u, `div` ≤ ~(3+√5)·u; see Principle 2), sized so the accumulated error is comfortably below
  ½ ulp at `p` for non-cancelling inputs. `FBig` itself uses this same fixed-guard style (`p + 50` for
  the hyperbolics, `p + 10 + ⌈log2 p⌉` for `powf`).
- **There is no retry.** Unlike a Ziv loop, when the `p+g` residual straddles a rounding boundary the
  result is simply re-rounded once — so it is *near-correctly-rounded* (identical guarantee class to
  `FBig`'s transcendentals) and may be 1 ulp off on a rare hard-to-round input. A guaranteed-correct
  Ziv retry loop (re-evaluate at widening `g` until the round is decisive) is **deferred to 0.5.x**
  (§13) and is expected to land in `FBig` first, so `CBig` inherits it by composing correctly-rounded
  parts.

This mirrors `FBig`'s current strategy and makes the "hard" MPC ops (mul/div/abs) tractable **without**
bespoke fixed-width correct-rounding code or a retry loop.

### 6.2 Per-op formulas, FBig reuse, difficulty

| Op | Formula (principal branch) | Reuses | Difficulty | Notes |
|---|---|---|---|---|
| `add`/`sub` | `(x±u) + i(y±v)` | `ctx.add`/`ctx.sub` | **FREE** | componentwise; 1 round/part |
| `neg` | `−x − iy` | `FBig` Neg | **FREE** | exact |
| `conj` | `x − iy` | flip im sign | **FREE** | exact incl. −0/∞ |
| `proj` | Annex G | predicates | **FREE** | ∞→+∞+i0 |
| `mul_i` | `(x,y)→(∓y,±x)` | sign flips | **FREE** | exact rotation |
| `sqr` | `(x²−y²) + i(2xy)` | `ctx.sqr`, `ctx.mul` | **EASY** | 2 mul + guard re-round |
| `mul` | `(xu−yv)+i(xv+yu)` | `ctx.mul` | **EASY** | naive 4-mul or Gauss 3-mul + guard re-round; ≤2 ulp naive |
| `div` | Smith's method (|u|≥|v| branch) | `ctx.mul`/`ctx.add`/`ctx.div` | **MEDIUM** | overflow-safe + guard re-round |
| `inv` | `conj(z)/|z|²` | `ctx.div` | **MEDIUM** | |
| scalar mul/div (`Mul<FBig>`, `Div<FBig>`, etc.) | `x·s`, `y·s` | `ctx.mul`/`ctx.div` | **EASY** | mixed-type operators; no named methods |
| `norm` | `x² + y²` | `ctx.sqr`, `ctx.add` | **EASY** | no sqrt; near-exact |
| `arg` | `atan2(y, x)` | `ctx.atan2` | **EASY** | reuse real atan2 + Annex G table |
| `abs` | `hypot(x,y) = sqrt(x²+y²)` | `ctx.sqrt` | **MEDIUM** | scaled sum-of-squares + guard re-round; the shared kernel |
| `sqrt` | `sqrt((|z|+x)/2) ± i·sgn(y)·sqrt((|z|−x)/2)` | `ctx.sqrt`, `abs` | **MEDIUM** | needs `abs`; cut ]−∞,0] |
| `exp` | `e^x·(cos y + i sin y)` | `ctx.exp`, `ctx.sin_cos` | **EASY** | direct; overflow→∞ |
| `log` | `ln|z| + i·arg z` | `ctx.ln`, `abs`, `arg` | **MEDIUM** | needs `abs`; cut ]−∞,0] |
| `pow` z^w | `exp(w·log z)` | `exp`, `log` | **MEDIUM-HARD** | principal branch |
| `powi` z^n | repeated squaring on `CBig` | `mul`/`sqr` | **MEDIUM** | branch-cut-free; cheaper than `exp(n log z)` |
| `sin`/`cos` | `sin x·cosh y + i·cos x·sinh y` (and `cos` form) | `ctx.sin`, `ctx.cos`, `ctx.sinh`, `ctx.cosh` | **EASY** | real hyperbolics now available; no `exp` blow-up |
| `tan` | `sin z / cos z` | `sin`, `cos` | **EASY** | or direct `(sin 2x + i·sinh 2y)/(cos 2x + cosh 2y)` |
| `asin`/`acos`/`atan` | Kahan log forms | `log`, `sqrt`, `mul` | **MEDIUM-HARD** | branch cuts per Kahan |

(Difficulty ratings assume the §6.1 guard-digit recipe — no Ziv retry — so `mul`/`div`/`abs`/trig are
all MEDIUM-or-easier in 0.5; the only true hard kernel left is `abs`'s overflow-safe scaling.)

**Implementation order respects dependencies:** `abs` (the shared hypot kernel, reused by `sqrt` and
`log`) lands first; then `sqrt`, `log`; then `exp` (free); then trig (needs `sin`/`cos`/`sinh`/`cosh`);
then inverse trig (needs `sqrt`+`log`); then `pow`.

**Per-op implementation notes:**
- **`div`**: Smith's method with the `|u|≥|v|` branch avoids the `|denominator|²` overflow; add the
  **Baudin–Smith robust** refinements (power-of-2 pre/post-scaling; rearrange the product when
  `r = d/c` or `b·r` would underflow to 0). The `0/0`, `∞/∞`, `z/0`, `0/z` cases short-circuit per
  Annex G before the numeric path.
- **`abs`**: scaled sum-of-squares (scale the larger component to O(1), square, add, `sqrt`), rescale
  — no spurious overflow/underflow — then guard re-round. (`FBig` has no `hypot`, so this is a small
  bespoke kernel, reused by `sqrt` and `log`.) Returns a real `FBig` with a single inexact flag
  (mirrors MPC's `mpc_abs`, which returns an `mpfr_t` ternary — not a complex inexact pair).
- **`asin`/`acos`**: `asin z = −i·log(sqrt(1−z²) + i·z)`. Since `sqrt(1−z²) + i·z` always has
  **positive real part**, the inner `log` never crosses the negative-real-axis cut — the branch cut
  comes entirely from the `sqrt`, which simplifies the signed-zero handling.
- **`sin`/`cos`** (real–imaginary form): `sin(x+iy) = sin x·cosh y + i·cos x·sinh y`. This reuses
  `FBig`'s cancellation-free `sinh`/`cosh` and avoids the `exp(±iz)` form's exponential blow-up for
  large `|Im z|`; give large-imaginary inputs a few extra guard digits to absorb cancellation near the
  zeros. The `exp(±iz)` identity is kept as a test cross-check (it need not match exactly on
  hard-to-round inputs, since 0.5 is near-correctly-rounded).
**Guard-digit budget (`g`):** the published bounds (√5·u for `mul`, ~(3+√5)·u for `div` — see
Principle 2) mean a fixed guard of a few base-B digits already brings the total error well under ½ ulp
for non-cancelling inputs. There is **no retry** in 0.5 — the worst-case "hard-to-round" inputs
(catastrophic cancellation in `mul`/`div`, hypot) may land 1 ulp off, exactly as `FBig`'s own
transcendentals can; a guaranteed-correct Ziv retry loop is deferred to 0.5.x (§13).

### 6.3 Special-value short-circuits (C99 Annex G / Kahan)

Before the numeric path, each op consults a predicate-driven table using `FBig`'s
`is_zero()`/`is_neg_zero()`/`is_infinite()`/`sign()`. Representative rules:

- **`sqrt`**: `sqrt(conj z) == conj(sqrt z)`; `x<0 ∧ y=±0 ⇒ ±i·sqrt(|x|)`; `+∞`/`−∞` quadrants per
  Annex G; `sqrt(±0) = ±0`.
- **`log`**: `log(−r ± i0) = ln r ± iπ`; `log(0) = −∞`; `log(∞) = +∞`; cut on `]−∞,0]`.
- **`arg`**: `atan2(y,x)` table — `(±0,−0)→±π`, `(±0,+0)→±0`, `(±0,x<0)→±π`, `y=±∞`→`±π/2`, etc.; range
  `]−π,π]`.
- **`proj`**: any `±∞` part ⇒ `+∞ + i·0` (imaginary-zero sign per Annex G); finite ⇒ unchanged.
- **`mul`/`div`**: `0·∞`/`0/0`/`∞/∞` ⇒ `Indeterminate`; `z/0` (z≠0) ⇒ `±∞`-quadrant; signed-zero
  products per Annex G.

The full tables are codified as exact, deterministic tests (`tests/special_values.rs`, Section 8) — no
proptest, just the reference vectors.

---

## 7. Crate structure & workspace integration

### 7.1 Directory & `Cargo.toml`
New directory `complex/`. `complex/Cargo.toml` follows the `rational/Cargo.toml` template verbatim
(edition 2021, `rust-version = "1.68"`, dual MIT/Apache, same author/repo/docs.rs metadata,
`[lib] bench = false`, `[package.metadata.docs.rs] all-features = true`):

```toml
[package]
name = "dashu-cmplx"
version = "0.5.0"                       # Decision 5: aligned with the 0.5 release
edition = "2021"
rust-version = "1.68"
# … (description/keywords/categories/license/repository mirror rational/Cargo.toml)

[features]
default = ["std"]
std = ["dashu-float/std"]
rand = ["rand_v09"]                     # for UniformCBig (tests/benches) + public random gen
rand_v09 = ["dep:rand_v09", "dashu-float/rand_v09"]
# Deferred to 0.5.x (scaffold only):
serde       = ["dep:serde", "dashu-float/serde"]
num-traits  = ["num-traits_v02"]
num-traits_v02 = ["dep:num-traits_v02", "dashu-float/num-traits_v02"]
num-complex = ["num-complex_v04"]
num-complex_v04 = ["dep:num-complex_v04"]

[dependencies]
dashu-base  = { version = "0.5.0", default-features = false, path = "../base" }
dashu-float = { version = "0.5.0", default-features = false, path = "../float" }   # mandatory
dashu-int   = { version = "0.5.0", default-features = false, path = "../integer" }
# optional: serde, num-traits_v02 (package="num-traits"), num-complex_v04 (package="num-complex"), rand_v09 (package="rand")

[dev-dependencies]
proptest  = "~1.7"                      # matches Phase 0 pinning; MSRV-safe
criterion = { version = "0.5.1", features = ["html_reports"] }
rand_v09  = { version = "0.9", package = "rand" }

[[bench]]
name = "arith"
harness = false
required-features = ["rand"]
[[bench]]
name = "transcendental"
harness = false
required-features = ["rand"]
[[bench]]
name = "io"
harness = false
required-features = ["rand"]
```

### 7.2 `lib.rs` module layout
```rust
#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;
// (crate-level //! doc with # Examples, mirroring rational/src/lib.rs)

mod cbig;          // the CBig type, constants, predicates, accessors
mod context;       // CRounded, CfpResult, the guard-digit rounding helper
mod add; mod sub; mod mul; mod div; mod root;   // field arithmetic + sqrt
mod exp; mod log; mod power; mod trig;          // transcendentals (trig uses FBig sinh/cosh)
mod misc;          // conj, proj, mul_i, abs(hypot), norm, arg
mod cmp; mod convert; mod parse; mod fmt;
mod third_party;
pub use third_party::*;
pub use cbig::CBig;
#[doc(hidden)] pub use dashu_int::Word;   // re-exported for the cbig! literal macro
```
Kernel routines (the guard-digit helper, `abs`'s scaled sum-of-squares, Smith's division) carry
**inline** `#[cfg(test)] mod tests` (per `AGENTS.md`); cross-cutting/public-API tests live in
`complex/tests/`.

### 7.3 Feature flags
Follow the `xxx_vYY` + unversioned-alias convention (`AGENTS.md`). `rand` aliases `rand_v09` (matches
`dashu-float`'s current rand default). `serde`/`num-traits`/`num-complex` are **scaffolded but
content deferred** to 0.5.x (Section 13) — declare the features so the meta-crate forwarding compiles,
with minimal/`#[cfg(...)]` stub impls or fully omitted until implemented (decide per Decision 10).

### 7.4 Meta-crate wiring
1. Root `Cargo.toml`: add `"complex"` to `[workspace] members` and `default-members`; add
   `dashu-cmplx = { version = "0.5.0", default-features = false, path = "./complex" }` to the
   meta-crate's `[dependencies]`; add `dashu-cmplx/<feat>` to each shared feature-forwarding line
   (`std`, `serde`, `rand`/`rand_v09`, `num-traits`/`num-traits_v02`, `zeroize`, etc.).
2. `src/lib.rs`: add `pub mod complex { pub use dashu_cmplx::*; }` and the type alias
   `pub type Complex = dashu_cmplx::CBig;` alongside `Real`/`Decimal`/`Rational`.
3. `README.md`: add `dashu-cmplx` to the crate table and feature lists.

### 7.5 Literal macro `cbig!` (in scope for 0.5)
Included in 0.5 (Decision 8). Follow the `dashu-macros` pattern: add `cbig`/`static_cbig`/
`cbig_embedded`/`static_cbig_embedded` in `macros/src/lib.rs`, a parser in `macros/src/parse/cbig.rs`
accepting `cbig!(re + im i)` / `cbig!(re, im)`, `macros/Cargo.toml` dep on `dashu-cmplx`, and
`src/macro-docs/cbig.md` + a `macro_rules!` wrapper in the meta-crate. The parser reuses the existing
`FBig` literal parser for each coefficient.

---

## 8. Test plan

Three layers, mirroring Phase 0's structure (`AGENTS.md`: inline kernel tests + `tests/` integration).

**(a) Inline kernel tests** (`#[cfg(test)] mod tests` in the source files): the guard-digit rounding
helper (the `p+g` → `p` re-round), `abs` scaled sum-of-squares (overflow/underflow scaling
invariants), Smith's division branch selection, Annex G predicate dispatch.

**(b) Integration property tests** (`complex/tests/*.rs`, run by the existing CI `test` job across the
`force_bits` matrix; pure-Rust, no GMP):
- `arith_prop.rs` — identities (tolerance ∝ a few ulp via `FBig::ulp()`):
  - add/sub commutativity & associativity & `z − z = 0`, `z + 0 = z`;
  - mul: `z·1 = z`, `z·conj(z) = norm(z)` (purely real), `(zw)·conj(w) = z·norm(w)`,
    commutativity, `z·0 = 0`;
  - div: `z / z = 1` (z≠0), `(z/w)·w = z`;
  - `mul_i`⁴ = identity; `conj(conj z) = z`; `proj` idempotent on finite values.
- `transcendental_prop.rs`:
  - `exp(log z) ≈ z`, `log(exp z) ≈ z` (mod `2πi`);
  - `sqrt(z)² ≈ z`, `sqrt(conj z) = conj(sqrt z)`;
  - `sin²z + cos²z ≈ 1`, de Moivre `(cos z + i sin z)^n ≈ cos(nz) + i sin(nz)`;
  - `log z = ln|z| + i·arg z` (imag part in `]−π,π]`);
  - `exp` periodicity `exp(z + 2πi) = exp z`.
- `rounding_prop.rs` — **correct-rounding self-oracle**: compute each op at precision `p`, recompute
  at `2p` and re-round to `p`, assert ≤ 1 ulp/part agreement (the Phase 0 pattern; applies to
  `mul`/`div`/`abs`/`sqrt`/`exp`/`log`/`sin`/`cos`/`pow`).
- `special_values.rs` — **exact, deterministic** Annex G table vectors (no proptest): `sqrt`/`log`/
  `arg`/`proj`/`conj`/`mul`/`div` with `±0`/`±∞` operands → exact expected `CBig`/`FpError`.

**(c) Oracle / fuzz (manual, not CI — per the maintainer's Phase 0 decision):** in the excluded
`fuzz/` crate, add `rug::Complex` (MPC backend) differential targets at random precision. Run via
`cargo test --manifest-path fuzz/Cargo.toml -- --ignored` before tagging 0.5. Property: dashu result
matches rug to ≤1 ulp/part (or exactly, where both are correctly rounded). Add a
`proptest-regressions/complex/` dir for pinned cases.

---

## 9. Benchmark plan

Compile-guarded by the existing clippy `--all-targets` job; **not run in CI** (criterion, manual).
`harness = false`, `required-features = ["rand"]`, `StdRng::seed_from_u64(1)`, log-scale sizing — all
matching the Phase 0 bench idioms. A `random_cbig(precision, rng)` helper mirrors `random_fbig`.

- `benches/arith.rs` — `mul`/`div`/`sqr` at base-2 precisions `{53, 113, 256, 1024}`; include a
  guard-width sweep (vary `g`) to confirm the default guard keeps the per-op error under ½ ulp for
  non-cancelling inputs and to inform the default `g`. (A naive-vs-Ziv "correct-rounding overhead"
  comparison is deferred with the Ziv loop itself, to 0.5.x.)
- `benches/transcendental.rs` — `exp`/`log`/`sin`/`cos`/`sqrt`/`abs`/`arg`/`pow` at the same
  precisions; include `sin_cos` (shared reduction). Compare `complex exp` cost vs `2 × real exp` to
  confirm the `exp/sin_cos` reuse is cheap.
- `benches/io.rs` — `FromStr`/`Display` of `"a+bi"` at several precisions.

Record a baseline (the only open Phase 0 item) so 0.5.x perf changes are measurable.

---

## 10. Implementation milestones (M1–M6)

Each milestone ends with `cargo check --all-features --tests`, `cargo clippy … -D warnings`,
`cargo fmt --check`, and a `CHANGELOG.md` `## Unreleased` entry.

- **M1 — Skeleton & easy ops.** Crate dir + `Cargo.toml`; `CBig` type, constants, predicates,
  `from_parts`/`from_real`/`re`/`imag`/`into_parts`; `add`/`sub`/`neg`/`conj`/`proj`/`mul_i`/
  `norm`/`arg`/`cmp`/`cmp_abs` + `NumOrd`/`NumHash`; `Display`/`Debug`/`FromStr`; workspace +
  meta-crate wiring. (All **FREE/EASY** ops; single-pass rounding only.) ✅ builds, fmt, clippy clean.
- **M2 — Rounding infra + `mul`/`div`/`sqr`/`inv`.** The `context` module: `CRounded`, `CfpResult`,
  the guard-digit helper, error-bound bookkeeping. Implement `abs` (the shared hypot kernel) here
  since `mul`/`div`'s tests and later ops depend on it. Near-correctly-rounded `mul`/`div`/`sqr`/`inv`.
  Includes `arith_prop.rs` + `rounding_prop.rs` (mul/div) + `special_values.rs` (arith).
- **M3 — `sqrt`/`exp`/`log`/`pow`.** `sqrt` (needs `abs`), `exp` (free via `sin_cos`), `log`,
  `powf`/`powi` (`powf(0,0)` returns `ONE`, matching `FBig::powf`). Extend
  `transcendental_prop.rs` + `special_values.rs`.
- **M4 — Trig & inverse trig.** `sin`/`cos`/`tan`/`sin_cos` (via real `sin`/`cos` + `sinh`/`cosh`),
  `asin`/`acos`/`atan`. Finish `transcendental_prop.rs`.
- **M5 — Hardening.** Full `special_values.rs` Annex G coverage; `rug::Complex` oracle in `fuzz/`
  (manual); benches M1–M4 (`benches/*`); baseline capture.
- **M6 — Polish & macro.** Meta-crate `dashu::complex`/`dashu::Complex`; README + guide chapter
  (`guide/src/` CBig section per `TODO-v05.md` Phase 4); the `cbig!` literal macro (§7.5); version
  sync (Phase 5).

Gating: M1–M6 sit on the **completed** Phase 0 (real transcendentals verified) — the signed-zero trig
bug fixed during Phase 0 is exactly the class of regression this gate prevents.

---

## 11. Decisions (resolved in PR review)

1. **Rounding model — uniform single `R` on both parts** ✅ confirmed. Simpler than MPC's `(R,R)` pair,
   sufficient for correctness, matches `FBig`. The per-axis `(R,R)` / `CRound` trait is deferred.
2. **Inexactness reporting — dual-flag `CRounded`** ✅ confirmed. `CRounded<R,B> =
   Approximation<CBig<R,B>, (Rounding, Rounding)>`; the context-layer result is `CfpResult<R,B> =
   Result<CRounded<R,B>, FpError>` (the `C`-prefixed mirror of `FpResult`). Cheap and faithful to MPC.
3. **Field privacy — private `re`/`im` + accessor methods** ✅ confirmed. Preserves the
   shared-precision invariant, consistent with `FBig`.
4. **No-NaN policy — map C99 NaN-producing cases to `FpError`** ✅ confirmed. No complex-only NaN.
   (Documented Section 4.)
5. **Crate version — `0.5.0`** ✅ confirmed. Aligns with the release.
6. **Naming — `complex` (dir/module) + `dashu-cmplx` (package/lib) + `dashu::Complex = CBig`; no
   `CDBig`** ✅ confirmed. Directory/module use the full word `complex` (mirroring
   `dashu-ratio`/`rational/`); the package is `dashu-cmplx`. No `CDBig` alias — there is almost no need
   for a decimal complex type (§2).
7. **Complex trig — real–imaginary form using `FBig`'s hyperbolics** ✅ confirmed (changed). Real
   hyperbolics (`sinh`/`cosh`/…) were merged from `develop`, so `CBig` uses
   `sin(x+iy) = sin x·cosh y + i·cos x·sinh y` directly (Principle 6, §5.5, §6.2). The `exp(±iz)` form
   is kept only as a test cross-check.
8. **`cbig!` literal macro — include in 0.5** ✅ confirmed (changed). Ship in 0.5 per §7.5 (M6).
9. **Scalar mul/div via mixed-type operators (no named methods)** ✅ confirmed (changed). Scalar
   mul/div by a real `FBig` is exposed only through operators — `Mul<FBig>`/`Div<FBig>` for `CBig`
   (`z * r`, `z / r`) and `Mul<CBig>`/`Div<CBig>` for `FBig` (`r * z`, `r / z`). No standalone
   `scale`/`unscale` methods — they'd just duplicate `*` and `/`. (Mixed-type operators therefore move
   *into* 0.5, replacing the earlier "defer to 0.5.x" plan; §5.2.)
10. **Third-party features — scaffold feature flags, defer impls** ✅ confirmed. Scaffold
    `serde`/`num-traits`/`num-complex` flags; defer impls to 0.5.x (matches `TODO-v05.md` §3.4).
11. **`abs` rounding — near-correctly-rounded in 0.5 (Ziv deferred)** ✅ confirmed (changed). Ship the
    scaled-hypot `abs` with the §6.1 guard-digit recipe in 0.5 (near-correctly-rounded, same class as
    `FBig`'s transcendentals); guaranteed-correct `abs` via Ziv is deferred with the Ziv loop itself
    (decision 13). The original "Ziv makes correctly-rounded `abs` low-risk" framing no longer applies
    to 0.5.
12. **`powf(0, 0)` policy — return `CBig::ONE`** ✅ confirmed. This follows how `FBig` defines it:
    `Context::powf` short-circuits on a zero exponent (`exp.is_zero() ⇒ Exact(FBig::ONE)`) before the
    base-zero branch, so `FBig::powf(0, 0) = 1`. `CBig::powf(0, 0)` returns `ONE` for the same reason
    (also matching the real `0⁰ = 1` convention and our no-NaN policy; C23 left `cpow(0,0)`
    implementation-defined). Documented at the `powf` API.
13. **Rounding strategy — guard-digit heuristic in 0.5; Ziv deferred to 0.5.x** ✅ confirmed
    (maintainer). `FBig` does not use a Ziv loop today (its transcendentals use fixed guard digits), so
    `CBig` mirrors that: compute each component at `p + g` and re-round once (§6.1) —
    near-correctly-rounded, not guaranteed correct. A guaranteed-correct Ziv retry loop is deferred to
    0.5.x and is expected to land in `FBig` first so `CBig` inherits it. (This supersedes the earlier
    "Ziv is dashu's structural advantage" framing in Principles 1–2.)

---

## 12. Risk register

| Risk | Mitigation |
|---|---|
| `mul`/`div`/`abs` are MPC's hardest-to-round ops | Guard-digit re-round (§6.1) keeps them tractable in 0.5; `rug::Complex` oracle + self-oracle tests (§8). Guaranteed correct rounding (Ziv) is a 0.5.x follow-up (§11.13) |
| C99 Annex G special-value combinatorics (±0, ±∞ on every op) | Explicit deterministic table tests (`special_values.rs`); reuse `FBig`'s signed-zero predicates |
| Near-correct rounding ≠ guaranteed correct | Document the guarantee level honestly (matches `FBig`); the `rounding_prop.rs` self-oracle bounds the error to ≤1 ulp/part; Ziv closes the gap in 0.5.x |
| Correctness depends on `FBig` transcendentals | Phase 0 gate (✅ done); the signed-zero trig bug fixed there is the canonical example |
| Scope creep (complex hyperbolics/fma/ball-arith) | Explicit deferred list (§13) |
| Name/version churn after code is written | Settled in §11 (decisions confirmed in review), before any code |
| Complex trig cancellation near zeros | Real–imaginary form (`sin x·cosh y + …`) reuses cancellation-free `sinh`/`cosh`; extra guard digits absorb residual cancellation near zeros |
| `no NaN` surprises users expecting C99 `NaN` | Document the `FpError` mapping prominently in the guide + rustdoc |

---

## 13. Out of scope (deferred to 0.5.x)

- A guaranteed-correct **Ziv retry loop** (0.5 ships near-correct guard-digit rounding, matching
  `FBig`; see §6.1 / §11.13). Expected to land in `FBig` first.
- **Complex** hyperbolic & inverse-hyperbolic family (`CBig::sinh`/`cosh`/`tanh`/`asinh`/`acosh`/
  `atanh`). (Real hyperbolics already exist on `Context<R>` and are *used* by `CBig` trig in 0.5; the
  complex-valued functions themselves are deferred.)
- `fma` (complex fused multiply-add — hard to round correctly).
- `rootofunity` (`e^{2πi/n}`), complex `agm`, `exp2`/`exp10`/`log2`/`log10`.
- Vector ops (`sum`/`dot`/mean).
- `CBig` `serde`/`rkyv`/`zeroize`.
- Ball arithmetic (the `mpcb_t` analogue — interval/uncertainty complex).
- Independent re/im rounding (`CRound` trait; MPC `mpc_rnd_t` parity).
- `num_complex::Complex<FBig>` interop (feature `num-complex_v04`).
- A `ComplexFloat`-style trait unifying `FBig` and `CBig` (sealed, for generic real/complex code).
- A `CachedCBig` variant threading `ConstCache` through transcendental calls (CBig v0.5 passes
  `cache = None`, matching `FBig` convenience behavior).

---

## Appendix — References

- **GNU MPC** — the parity/correctness target (type model, `(re,im)` string format, inexact-flag
  semantics). Referenced as the API/correctness contract; algorithms are re-expressed in our own terms.
- **C99 Annex G** / **W. Kahan, "Branch Cuts for Complex Elementary Functions" (1987)** — the
  authoritative branch-cut + signed-zero model (slit on `]−∞,0]` for `sqrt`/`log`/`pow`;
  `sqrt(conj z)=conj(sqrt z)`; the `csqrt`/`carg`/`hypot` algorithms).
- **C. Percival, "Efficiently rounded complex multiplication"** — correct rounding of complex `mul`
  (the fixed-width difficulty that the guard-digit recipe handles in 0.5 and a full Ziv loop resolves
  in 0.5.x).
- **C. F. Borges, "An Improved Algorithm for hypot(a,b)" (2019)** — overflow-safe, correctly-rounded
  `abs`/hypot (the `abs` kernel in §6.2).
- **Smith's method** (complex division, overflow-safe) and the **Gauss/Karatsuba 3-mul** form —
  standard named algorithms used as implementation baselines.
- **`num-complex`** — Rust API idioms surveyed: `norm_sqr` vs `norm` (→ our `norm` vs `abs`),
  `arg = atan2(im,re)`, tiered method availability, `no_std`. (Its `to_polar`/`from_polar`/`cis`
  polar helpers are deliberately not adopted — see §5.1/§5.4.)
- **`TODO-v05.md` Phase 3** — the original `CBig` spec this plan expands (§3.1–3.4); superseded on
  naming (`complex` dir/module + `dashu-cmplx` package) and elaborated on
  rounding/inexactness/algorithms/tests/benchmarks.
- **Brent, Percival, Zimmermann, “Error Bounds on Complex Floating-Point Multiplication”
  (Math. Comp. 76, 2007)** — the √5·u normwise error bound for complex `mul` (sizes the `mul` guard;
  worst-case inputs catalogued).
- **Jeannerod, Kornerup, Louvet, Muller, “…complex floating-point multiplication with an FMA”
  (Math. Comp. 86, 2017)** — the 2u FMA-kernel bound (optimal kernel if FMA is added later).
- **Baudin & Smith, “A Robust Complex Division in Scilab” (arXiv:1210.4539, 2012)** — overflow/
  underflow-safe complex division; basis for the `div` kernel (Smith + power-of-2 scaling).
- **NIST DLMF §4.23 / §4.37** — principal-value formulas and branch-cut tables for the inverse trig
  and inverse hyperbolic functions.
- **Fousse, Hanrot, Lefèvre, Pélissier, Zimmermann, “MPFR: A Multiple-Precision Binary Floating-Point
  Library With Correct Rounding” (ACM TOMS, 2007)** — the Ziv/correct-rounding strategy `FBig` targets
  (guard-digit heuristic today; full Ziv loop a 0.5.x goal) and `CBig` reuses at one remove.
- **“Accuracy of Complex Mathematical Operations and Functions in C23” (HAL hal-04714173, 2024)** —
  recent survey testing C23 complex `libm` against MPC; useful Annex-G special-value reference.
