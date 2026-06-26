# dashu-cplx — Design & Implementation Plan (v0.5)

Last updated: 2026-06-26
Status: **design (pre-implementation)**. Gated by `TODO-v05.md` Phase 0 (✅ done) and Phase 2
(✅ done, #83). This document specifies the **`CBig`** arbitrary-precision complex type — the
headline new crate for the 0.5 release and the Rust-native alternative to **GNU MPC**.

> **Naming.** The maintainer prefers `dashu-cplx` over `dashu-complex`. This plan uses **`cplx`**
> throughout: directory `cplx/`, package `dashu-cplx`, library `dashu_cplx`, meta-module
> `dashu::cplx`, type alias `dashu::Complex = CBig`. (`TODO-v05.md` Phase 3 still says `complex/`;
> that path is superseded by this document.)

---

## 0. Purpose & scope

`dashu-cplx` provides arbitrary-precision **complex** numbers, `CBig`, built on top of
`dashu-float`'s `FBig`. Each `CBig` is a pair of `FBig` parts (`re`, `im`) sharing one precision and
one rounding mode. The crate targets **MPC parity for the "common functionalities"** (field
arithmetic + elementary transcendentals + abs/arg/conj/proj + I/O), with **correct rounding** for
every operation, following the **C99 Annex G / Kahan** branch-cut and signed-zero model that
`dashu-float` already implements for reals.

**In scope for 0.5:** construction/conversion, `add/sub/mul/div/neg/sqr`, scalar mul/div by real,
`abs`/`norm`/`arg`/`conj`/`proj`/`mul_i`, `sqrt`/`exp`/`log`/`pow` (complex & integer exponent),
`sin`/`cos`/`tan`/`sin_cos`, `asin`/`acos`/`atan`, `cmp`/`cmp_abs`, `Display`/`Debug`/`FromStr` in
MPC's `(re im)` form, workspace + meta-crate wiring.

**Deferred to 0.5.x** (Section 13): hyperbolics & inverse hyperbolics, `fma`, `rootofunity`, `agm`,
`exp2/exp10/log2/log10`, vector ops, `CBig` serde/rkyv, ball arithmetic, independent re/im rounding,
`num_complex` interop, the `cplx!` literal macro, a `CachedCBig`/`ComplexFloat` layer.

**Hard constraints (from `AGENTS.md`):** MSRV 1.68 (do not bump), Rust edition 2021, `no_std` +
`alloc` on the default path, `--exclude dashu-python` for workspace commands, every change in
`cplx/CHANGELOG.md` `## Unreleased`, no external-library function names in code comments (describe
algorithms in our own terms), algorithm-kernel tests inline as `#[cfg(test)] mod tests`, double-word
first-class citizen.

---

## 1. Design principles

1. **Reuse `FBig`, do not re-derive real math.** Every real transcendental `CBig` needs
   (`exp`, `ln`, `sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `atan2`, `sqrt`, `powi`, `powf`,
   `inv`, `sqr`) already exists on `Context<R>` and is correctly rounded. `CBig` composes these; it
   does **not** re-implement real arithmetic. (Phase 0 verified these for real inputs — that gate is
   the whole reason `CBig` can lean on them.)

2. **Correct rounding via dynamic precision (Ziv's strategy).** This is exactly the technique MPC
   itself uses — the difference is that `CBig` gets it almost for free. Because each component is a
   `FBig` (already correctly rounded at *any* requested precision, via its own internal Ziv loop),
   `CBig` only has to evaluate the complex formula at `p + g` guard digits and re-round each component
   to `p` (`with_precision`); **if the proven error bound straddles a rounding boundary, raise `g` and
   retry**. The per-component re-round is delegated to `FBig`, so `CBig` needs no bespoke fixed-width
   correct-rounding code per function. The bounds that size `g` are known and tight: complex `mul` has
   normwise relative error `< √5·u` (Brent–Percival–Zimmermann; `< 2u` with an FMA-style kernel), and
   complex `div` `< (3+√5)·u` naively / `5u` with FMA — so a small fixed guard plus a Ziv fallback
   settles the overwhelming majority of inputs. (Section 6.1.)

3. **One rounding mode `R` for both parts (uniform), matching `FBig`'s single-context model.**
   `CBig<R, B>` carries a single `R: Round` applied independently to the real and imaginary
   components. This is simpler than MPC's per-axis `mpc_rnd_t` pair and is sufficient for correctness
   (each component is still rounded independently). MPC-parity independent re/im rounding is a deferred
   `CRound`-trait enhancement (Open Decision 1, with recommendation).

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

6. **Complex trig via `exp(±iz)`, not real hyperbolics.** `sin z = (e^{iz} − e^{−iz})/(2i)`,
   `cos z = (e^{iz} + e^{−iz})/2` need only `FBig::exp`, which exists. This deliberately avoids a
   dependency on real `sinh`/`cosh` (not in `dashu-float`'s 0.5 scope). Switching to the
   `sin x·cosh y + i·cos x·sinh y` form is a cheap future optimization once real hyperbolics land
   (Section 6.2).

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
  A decimal alias mirrors `DBig`: `pub type CDBig = CBig<mode::HalfAway, 10>;` (naming TBD).
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
MPC returns **two** inexact flags (one per axis). dashu's analog is a small struct carrying one
`Rounded` per part:

```rust
/// Correctly-rounded complex result with per-axis inexactness.
pub struct CRounded<R: Round, const B: Word> {
    pub(crate) value: CBig<R, B>,
    pub(crate) re: Rounding,   // NoOp | AddOne | SubOne  (== Exact if NoOp)
    pub(crate) im: Rounding,
}
```

(Reuses `dashu_float::Rounding { NoOp, AddOne, SubOne }`; `NoOp` ≡ exact. `CRounded` is the complex
twin of `Rounded<T> = Approximation<T, Rounding>`.)

### 3.3 Result/error type
```rust
pub type CpResult<R, const B> = Result<CRounded<R, B>, FpError>;
```
Reuses `dashu_float::FpError` unchanged (Section 4). The **convenience layer** (`CBig::mul`, etc.)
unwraps via the same policy as `FBig` (`unwrap_fp`: `Overflow(s)→±∞`, `Underflow(s)→±0`, panic on
`Indeterminate`/`OutOfDomain`/`InfiniteInput`) and returns `CBig` directly.

### 3.4 Two-layer API (mirror `FBig`)
| Layer | Where rounding lives | Return type | On error |
|---|---|---|---|
| **Context** (`CBig::mul` is sugar; the real work is a free/kernel `fn`) | mode `R`, precision from the context | `CpResult` (`Result<CRounded, FpError>`) | `Err(FpError)` |
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
CBig::from_polar(r: &FBig, theta: &FBig) -> Self;          // r·(cos θ + i sin θ)
CBig::cis(phase: &FBig) -> Self;                           // exp(i·phase) == from_polar(1, phase)

FromStr  // "(re im)" — MPC format (Section 5.6)
TryFrom<f32>/<f64>          // (base 2) exact-or-LossOfPrecision
TryFrom<num_complex::Complex<FBig>>   // feature num_complex_v04 (deferred)
```

### 5.2 Field arithmetic
```rust
// context-layer: CpResult ; convenience-layer (operators): CBig
add(&self, rhs) / sub / neg
mul(&self, rhs) / sqr(&self)               // correctly rounded (Section 6.2)
div(&self, rhs)                             // correctly rounded (Smith's method + Ziv)
inv(&self)                                  // 1/z = conj(z)/|z|²
scale(&self, s: &FBig) / unscale            // scalar mul/div by a real FBig (num-complex idiom)
mul_i(&self, negative: bool)               // ×(±i): exact rotation (re,im)→(∓im,±re)
powi(&self, n: IBig) -> Self               // integer power: repeated squaring (cheaper, branch-cut-free)
powf(&self, w: &Self) -> Self              // complex^complex: exp(w·log z)
powc == powf; pow_real(&self, r: &FBig)    // complex^real: exp(r·log z)
```
Operator overloads: `Add/Sub/Mul/Div/Neg` for `CBig op CBig` (all four ref/val combinations, via the
existing `forward_all_binop!`-style macro if present, else explicit impls); `Mul<FBig>`/`Div<FBig>`
for scalar ops (Open Decision 9). `AddAssign`-style for the owning variants.

### 5.3 Comparison
```rust
PartialEq/Eq                       // componentwise exact equality (+0 == −0 per component, like FBig)
PartialOrd?                        // complex total order is NOT natural — DO NOT impl Ord
cmp_abs(&self, rhs) -> Ordering    // compare by |z| (uses AbsOrd; match MPC's mpc_cmp_abs)
is_zero(&self) / is_infinite(&self) / is_finite(&self)
```
Per `TODO-v05.md` §3.2 and MPC: **no total `Ord`**. Provide `cmp_abs` (modulus comparison) and
equality only.

### 5.4 Decomposition & misc
```rust
re(&self) -> &FBig      /  imag(&self) -> &FBig
into_parts(self) -> (FBig, FBig)
conj(&self) -> Self                         // re − i·im : exact (flip sign of im, incl. −0/∞)
proj(&self) -> Self                         // Riemann projection (Section 4)
abs(&self) -> FBig                          // |z| = hypot(re,im) : HARD, correctly rounded (Section 6.2)
norm(&self) -> FBig                         // re² + im² : cheap, near-exact (no sqrt)
arg(&self) -> FBig                          // atan2(im, re) ∈ ]−π, π] : branch cut (−∞,0]
to_polar(self) -> (FBig, FBig)              // (abs, arg)
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
sin(&self) / cos(&self) / tan(&self) / sin_cos(&self) -> (Self, Self)   // via exp(±iz)
asin(&self) / acos(&self) / atan(&self)                                 // via log + sqrt (Kahan)
```
Identities implemented in terms of `FBig` (no real hyperbolics needed in 0.5):
- `exp(x+iy) = e^x·(cos y + i sin y)`
- `log z = ln|z| + i·arg z`
- `sin z = (e^{iz} − e^{−iz})/(2i)`, `cos z = (e^{iz} + e^{−iz})/2`, `tan z = sin z / cos z`
- `asin z = −i·log(iz + sqrt(1−z²))`, `acos z = −i·log(z + i·sqrt(1−z²))` (or `π/2 − asin z`),
  `atan z = (i/2)·(log(1−iz) − log(1+iz))`

### 5.6 I/O (`Display` / `Debug` / `FromStr`)
- **`Display`**: MPC's parenthesized form `"(re im)"` — e.g. `"(1.5 -2.0)"`, `"(inf 0)"`,
  `"(0 -0)"`. Output is always parenthesized with a single space separator. Each part uses `FBig`'s
  native `Display`, so special values render as dashu's `inf`/`-inf` (not MPC's `@Inf@`).
- **`Debug`**: `"<re> + <im>i (prec: <p>)"` or a structured form with `#?` (mirrors `FBig`'s Debug).
- **`FromStr`**: mirrors MPC's string grammar — a bare real (`"3"`, imaginary defaults to `+0`)
  or a parenthesized pair `"(re im)"` separated by whitespace (not a comma); parentheses are required
  for a pair and optional for a bare real. Each part parses via `FBig`'s `FromStr` (so `inf`/`-inf`,
  not MPC's `@Inf@`); `ParseError` on anything malformed.

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
`Assign` forms. `Hash` **only when** both parts are finite integers (or omit `Hash`; recommend omit —
matches the difficulty of hashing `FBig`). `Sum`/`Product` for iterators (mirror `FBig`).

---

## 6. Algorithms & correct-rounding strategy

### 6.1 The dynamic-precision (Ziv) recipe — dashu's structural advantage

Every non-trivial `CBig` op is computed by the same skeleton, parameterized by a *formula* and an
*error bound*:

```
fn complex_op<R,B>(z, w, ctx) -> CpResult {
    // 1. Short-circuit special values (Annex G table) — see 6.3.
    if let Some(special) = annex_g_shortcut(z, w) { return special; }

    // 2. Ziv loop: evaluate the formula at p + g guard digits.
    let p = ctx.precision();
    let mut g = INITIAL_GUARD;          // a few base-B digits (e.g. 2–4)
    loop {
        let gctx = Context::<R>::new(p + g);          // FBig, correctly rounded at p+g
        let (re_hi, im_hi) = formula(z, w, gctx)?;     // each component is an FBig at precision p+g
        let re_lo = re_hi.clone().with_precision(p);   // re-round to p with mode R
        let im_lo = im_hi.clone().with_precision(p);

        // 3. Error bound: |re_hi - re_lo| ≤ err_bound(formula, g, z, w).
        if re_round_is_decisive(&re_hi, &re_lo, err_re, R) &&
           re_round_is_decisive(&im_hi, &im_lo, err_im, R) {
            return Ok(CRounded { value: CBig::from_parts(re_lo.value(), im_lo.value()), re: re_lo_rounding, im: im_lo_rounding });
        }
        g *= 2;                          // ambiguous: raise guard and retry
    }
}
```

- `formula` is built **only** from `FBig` ops at precision `p+g` (each already correctly rounded).
- `err_bound` is a rigorous, per-formula bound on `|computed − true|` in ulps of the `p+g` result
  (derived from the formula's rounding-error count; e.g. naive mul accumulates ≤ ~2 ulp/component, so
  `g` is chosen so the bound is `< 0.5 ulp_at_p` for the *decisive* check).
- `re_round_is_decisive` checks the `p+g` result's residual against the rounding boundary; if the bound
  is too loose to decide, raise `g`.
- Termination is guaranteed: at sufficiently large `g` the computation is effectively exact.

This is the same idea `FBig` uses internally; `CBig` just orchestrates two real re-rounds. It makes
the "hard" MPC ops (mul/div/abs) tractable **without** bespoke fixed-width correct-rounding code.

### 6.2 Per-op formulas, FBig reuse, difficulty

| Op | Formula (principal branch) | Reuses | Difficulty | Notes |
|---|---|---|---|---|
| `add`/`sub` | `(x±u) + i(y±v)` | `ctx.add`/`ctx.sub` | **FREE** | componentwise; 1 round/part |
| `neg` | `−x − iy` | `FBig` Neg | **FREE** | exact |
| `conj` | `x − iy` | flip im sign | **FREE** | exact incl. −0/∞ |
| `proj` | Annex G | predicates | **FREE** | ∞→+∞+i0 |
| `mul_i` | `(x,y)→(∓y,±x)` | sign flips | **FREE** | exact rotation |
| `sqr` | `(x²−y²) + i(2xy)` | `ctx.sqr`, `ctx.mul` | **EASY** | 2 mul + Ziv |
| `mul` | `(xu−yv)+i(xv+yu)` | `ctx.mul` | **MEDIUM** | naive 4-mul or Gauss 3-mul + Ziv; ≤2 ulp naive |
| `div` | Smith's method (|u|≥|v| branch) | `ctx.mul`/`ctx.add`/`ctx.div` | **MEDIUM** | overflow-safe + Ziv |
| `inv` | `conj(z)/|z|²` | `ctx.div` | **MEDIUM** | |
| scalar `scale`/`unscale` | `x·s`, `y·s` | `ctx.mul`/`ctx.div` | **EASY** | |
| `norm` | `x² + y²` | `ctx.sqr`, `ctx.add` | **EASY** | no sqrt; near-exact |
| `arg` | `atan2(y, x)` | `ctx.atan2` | **EASY** | reuse real atan2 + Annex G table |
| `abs` | `hypot(x,y) = sqrt(x²+y²)` | `ctx.sqrt` | **HARD** | scaled sum-of-squares + Ziv; the shared hard kernel |
| `sqrt` | `sqrt((|z|+x)/2) ± i·sgn(y)·sqrt((|z|−x)/2)` | `ctx.sqrt`, `abs` | **MEDIUM** | needs `abs`; cut ]−∞,0] |
| `exp` | `e^x·(cos y + i sin y)` | `ctx.exp`, `ctx.sin_cos` | **EASY** | direct; overflow→∞ |
| `log` | `ln|z| + i·arg z` | `ctx.ln`, `abs`, `arg` | **MEDIUM** | needs `abs`; cut ]−∞,0] |
| `pow` z^w | `exp(w·log z)` | `exp`, `log` | **MEDIUM-HARD** | principal branch |
| `powi` z^n | repeated squaring on `CBig` | `mul`/`sqr` | **MEDIUM** | branch-cut-free; cheaper than `exp(n log z)` |
| `sin`/`cos` | `(e^{±iz}∓...)/(2i)` | `ctx.exp` | **MEDIUM** | cancellation near zeros → Ziv with extra guard |
| `tan` | `sin z / cos z` | `sin`, `cos` | **MEDIUM** | or direct form |
| `asin`/`acos`/`atan` | Kahan log forms | `log`, `sqrt`, `mul` | **MEDIUM-HARD** | branch cuts per Kahan |

**Implementation order respects dependencies:** `abs` (the HARD kernel, reused by `sqrt` and `log`)
lands first; then `sqrt`, `log`; then `exp` (free); then trig (needs `exp`); then inverse trig (needs
`sqrt`+`log`); then `pow`.

**Per-op implementation notes:**
- **`div`**: Smith's method with the `|u|≥|v|` branch avoids the `|denominator|² overflow; add the
  **Baudin–Smith robust** refinements (power-of-2 pre/post-scaling; rearrange the product when
  `r = d/c` or `b·r` would underflow to 0). The `0/0`, `∞/∞`, `z/0`, `0/z` cases short-circuit per
  Annex G before the numeric path.
- **`abs`**: scaled sum-of-squares (scale the larger component to O(1), square, add, `sqrt`), rescale
  — no spurious overflow/underflow — then Ziv re-round. (`FBig` has no `hypot`, so this is a small
  bespoke kernel, reused by `sqrt` and `log`.) Returns a real `FBig` with a single inexact flag
  (mirrors MPC's `mpc_abs`, which returns an `mpfr_t` ternary — not a complex inexact pair).
- **`asin`/`acos`**: `asin z = −i·log(sqrt(1−z²) + i·z)`. Since `sqrt(1−z²) + i·z` always has
  **positive real part**, the inner `log` never crosses the negative-real-axis cut — the branch cut
  comes entirely from the `sqrt`, which simplifies the signed-zero handling.
- **`sin`/`cos`** (exp(±iz) form): for large `|Im z|` the intermediate `e^|y|` grows exponentially —
  give large-imaginary inputs extra guard digits, and let Ziv absorb cancellation near the zeros. The
  `sin x·cosh y + i·cos x·sinh y` form is a cheap future win once real hyperbolics land (post-0.5).
**Guard-digit budget (initial `g`):** the published bounds (√5·u for `mul`, ~(3+√5)·u for `div` — see
Principle 2) mean a guard of a few base-B digits already brings the total error well under ½ ulp for
non-cancelling inputs; the Ziv loop doubles `g` on ambiguity. The worst-case "hard-to-round" inputs
(catastrophic cancellation in `mul`/`div`, hypot) are exactly what the retry exists for and are rare;
the "correct-rounding overhead" benchmark (Section 9) measures the retry rate to confirm it.

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
New directory `cplx/`. `cplx/Cargo.toml` follows the `rational/Cargo.toml` template verbatim
(edition 2021, `rust-version = "1.68"`, dual MIT/Apache, same author/repo/docs.rs metadata,
`[lib] bench = false`, `[package.metadata.docs.rs] all-features = true`):

```toml
[package]
name = "dashu-cplx"
version = "0.5.0"                       # Open Decision 5: align-with-release (rec) vs 0.1.0
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
mod context;       // CRounded, CpResult, the Ziv rounding helper
mod add; mod sub; mod mul; mod div; mod root;   // field arithmetic + sqrt
mod exp; mod log; mod power; mod trig;          // transcendentals
mod misc;          // conj, proj, mul_i, abs(hypot), norm, arg, polar, scale
mod cmp; mod convert; mod parse; mod fmt;
mod third_party;
pub use third_party::*;
pub use cbig::CBig;
#[doc(hidden)] pub use dashu_int::Word;   // for macros, if a cplx! macro is added
```
Kernel routines (the Ziv helper, `abs`'s scaled sum-of-squares, Smith's division) carry **inline**
`#[cfg(test)] mod tests` (per `AGENTS.md`); cross-cutting/public-API tests live in `cplx/tests/`.

### 7.3 Feature flags
Follow the `xxx_vYY` + unversioned-alias convention (`AGENTS.md`). `rand` aliases `rand_v09` (matches
`dashu-float`'s current rand default). `serde`/`num-traits`/`num-complex` are **scaffolded but
content deferred** to 0.5.x (Section 13) — declare the features so the meta-crate forwarding compiles,
with minimal/`#[cfg(...)]` stub impls or fully omitted until implemented (decide per Open Decision 10).

### 7.4 Meta-crate wiring
1. Root `Cargo.toml`: add `"cplx"` to `[workspace] members` and `default-members`; add
   `dashu-cplx = { version = "0.5.0", default-features = false, path = "./cplx" }` to the meta-crate's
   `[dependencies]`; add `dashu-cplx/<feat>` to each shared feature-forwarding line (`std`, `serde`,
   `rand`/`rand_v09`, `num-traits`/`num-traits_v02`, `zeroize`, etc.).
2. `src/lib.rs`: add `pub mod cplx { pub use dashu_cplx::*; }` and the type alias
   `pub type Complex = dashu_cplx::CBig;` alongside `Real`/`Decimal`/`Rational`.
3. `README.md`: add `dashu-cplx` to the crate table and feature lists.

### 7.5 Literal macro `cplx!` (optional / deferred)
If shipped in 0.5, follow the `dashu-macros` pattern: add `cplx`/`static_cplx`/`cplx_embedded`/
`static_cplx_embedded` in `macros/src/lib.rs`, a parser in `macros/src/parse/cplx.rs` accepting
`cplx!(re + im i)` / `cplx!(re, im)`, `macros/Cargo.toml` dep on `dashu-cplx`, and
`src/macro-docs/cplx.md` + a `macro_rules!` wrapper in the meta-crate. **Recommendation: defer to
0.5.x** to keep 0.5 scope tight (Open Decision 8).

---

## 8. Test plan

Three layers, mirroring Phase 0's structure (`AGENTS.md`: inline kernel tests + `tests/` integration).

**(a) Inline kernel tests** (`#[cfg(test)] mod tests` in the source files): the Ziv rounding helper
(decisive/ambiguous transitions), `abs` scaled sum-of-squares (overflow/underflow scaling invariants),
Smith's division branch selection, Annex G predicate dispatch.

**(b) Integration property tests** (`cplx/tests/*.rs`, run by the existing CI `test` job across the
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
`proptest-regressions/cplx/` dir for pinned cases.

---

## 9. Benchmark plan

Compile-guarded by the existing clippy `--all-targets` job; **not run in CI** (criterion, manual).
`harness = false`, `required-features = ["rand"]`, `StdRng::seed_from_u64(1)`, log-scale sizing — all
matching the Phase 0 bench idioms. A `random_cbig(precision, rng)` helper mirrors `random_fbig`.

- `benches/arith.rs` — `mul`/`div`/`sqr` at base-2 precisions `{53, 113, 256, 1024}`; **plus a
  "correct-rounding overhead" group** comparing a deliberately-naive `mul` (no Ziv) against the
  correctly-rounded `mul`, to quantify the guard-digit/retry cost (informs the default `g`).
- `benches/transcendental.rs` — `exp`/`log`/`sin`/`cos`/`sqrt`/`abs`/`arg`/`pow` at the same
  precisions; include `sin_cos` (shared reduction). Compare `complex exp` cost vs `2 × real exp` to
  confirm the `exp/sin_cos` reuse is cheap.
- `benches/io.rs` — `FromStr`/`Display` of `"(re im)"` at several precisions.

Record a baseline (the only open Phase 0 item) so 0.5.x perf changes are measurable.

---

## 10. Implementation milestones (M1–M6)

Each milestone ends with `cargo check --all-features --tests`, `cargo clippy … -D warnings`,
`cargo fmt --check`, and a `CHANGELOG.md` `## Unreleased` entry.

- **M1 — Skeleton & easy ops.** Crate dir + `Cargo.toml`; `CBig` type, constants, predicates,
  `from_parts`/`from_real`/`re`/`imag`/`into_parts`; `add`/`sub`/`neg`/`conj`/`proj`/`mul_i`/
  `norm`/`arg`/`scale`/`cmp`/`cmp_abs`; `Display`/`Debug`/`FromStr`; workspace + meta-crate wiring.
  (All **FREE/EASY** ops; no Ziv yet.) ✅ builds, fmt, clippy clean.
- **M2 — Correct-rounding infra + `mul`/`div`/`sqr`/`inv`.** The `context` module: `CRounded`,
  `CpResult`, the Ziv helper, error-bound bookkeeping. Implement `abs` (the HARD hypot kernel) here
  since `mul`/`div`'s tests and later ops depend on it. Correctly-rounded `mul`/`div`/`sqr`/`inv`.
  Includes `arith_prop.rs` + `rounding_prop.rs` (mul/div) + `special_values.rs` (arith).
- **M3 — `sqrt`/`exp`/`log`/`pow`.** `sqrt` (needs `abs`), `exp` (free via `sin_cos`), `log`,
  `powf`/`powi`/`pow_real`. Extend `transcendental_prop.rs` + `special_values.rs`.
- **M4 — Trig & inverse trig.** `sin`/`cos`/`tan`/`sin_cos` (via `exp(±iz)`), `asin`/`acos`/`atan`.
  Finish `transcendental_prop.rs`.
- **M5 — Hardening.** Full `special_values.rs` Annex G coverage; `rug::Complex` oracle in `fuzz/`
  (manual); benches M1–M4 (`benches/*`); baseline capture.
- **M6 — Polish.** Meta-crate `dashu::cplx`/`dashu::Complex`; README + guide chapter (`guide/src/`
  CBig section per `TODO-v05.md` Phase 4); version sync (Phase 5); `cplx!` macro if kept in scope.

Gating: M1–M6 sit on the **completed** Phase 0 (real transcendentals verified) — the signed-zero trig
bug fixed during Phase 0 is exactly the class of regression this gate prevents.

---

## 11. Open decisions (recommendations marked **(rec)**)

1. **Rounding model.** Uniform single `R` on both parts **(rec)** vs MPC-style `(R,R)` pair via a
   `CRound` trait. Uniform is simpler, sufficient for correctness, and matches `FBig`; defer the pair.
2. **Inexactness reporting.** Dual-flag `CRounded { re, im }` **(rec)** vs simple `Result<CBig,
   FpError>`. The dual flag is cheap and faithful to MPC.
3. **Field privacy.** Private `re`/`im` + accessor methods **(rec)** vs `pub` fields (num-complex
   style). Private preserves the shared-precision invariant, consistent with `FBig`.
4. **No-NaN policy.** Map C99 NaN-producing cases to `FpError` (`Indeterminate`/`OutOfDomain`) **(rec)**;
   do **not** introduce a complex-only NaN. (Documented Section 4.)
5. **Crate version.** `0.5.0` to align with the release **(rec)** vs `0.1.0` (new-crate convention).
6. **Name confirmation.** `cplx` (dir/crate/module) + `dashu::Complex = CBig` **(rec, per maintainer
   preference)**. `CDBig` decimal alias name TBD.
7. **Complex trig via `exp(±iz)`** (avoids needing real `sinh`/`cosh`) **(rec)** vs adding real
   hyperbolics to `dashu-float` first. Defer the hyperbolic-form switch to 0.5.x.
8. **`cplx!` literal macro.** Defer to 0.5.x **(rec)** vs include in 0.5.
9. **Scalar operator surface.** Provide `Mul<FBig>`/`Div<FBig>` + `scale`/`unscale` methods **(rec)**
   vs CBig-only operators (num-complex deliberately omits mixed-type operators).
10. **Third-party features in 0.5.** Scaffold `serde`/`num-traits`/`num-complex` feature flags but
    defer impls to 0.5.x **(rec)** vs implement now. (Matches `TODO-v05.md` §3.4 deferral of `CBig`
    serde/rkyv.)
11. **`abs` correct rounding in 0.5.** Ship correctly-rounded `abs` (HARD but tractable via Ziv) in
    0.5 **(rec)** vs a 1-ulp `abs` now and refine later. Ziv makes this low-risk.
12. **`pow(0, 0)` policy.** C23 made `cpow(0,0)` implementation-defined (Annex G was removed) and the
    raw `exp(0·log 0)` path is indeterminate. Return `CBig::ONE` (matching the real `0⁰ = 1` convention
    and our no-NaN policy) **(rec)** vs raise `FpError::Indeterminate`. Document whichever is chosen.

---

## 12. Risk register

| Risk | Mitigation |
|---|---|
| Correct rounding of `mul`/`div`/`abs` is MPC's hardest problem | Ziv dynamic precision (§6.1) makes it mechanical; `rug::Complex` oracle + self-oracle tests (§8) |
| C99 Annex G special-value combinatorics (±0, ±∞ on every op) | Explicit deterministic table tests (`special_values.rs`); reuse `FBig`'s signed-zero predicates |
| Ziv retry overhead hurts `mul` perf | "correct-rounding overhead" bench group (§9); most inputs decide at `g₀`; tune default `g` |
| Correctness depends on `FBig` transcendentals | Phase 0 gate (✅ done); the signed-zero trig bug fixed there is the canonical example |
| Scope creep (hyperbolics/fma/ball-arith) | Explicit deferred list (§13) |
| Name/version churn after code is written | Settle §11.5/§11.6 at M1, before any code |
| Complex trig cancellation near zeros (`exp(±iz)`) | Extra guard digits in the trig Ziv loop; switch to hyperbolic form post-0.5 |
| `no NaN` surprises users expecting C99 `NaN` | Document the `FpError` mapping prominently in the guide + rustdoc |

---

## 13. Out of scope (deferred to 0.5.x)

- Hyperbolic & inverse-hyperbolic family (`sinh`/`cosh`/`tanh`/`asinh`/`acosh`/`atanh`) — needs real
  hyperbolics or the `exp` forms.
- `fma` (complex fused multiply-add — hard to round correctly).
- `rootofunity` (`e^{2πi/n}`), complex `agm`, `exp2`/`exp10`/`log2`/`log10`.
- Vector ops (`sum`/`dot`/mean).
- `CBig` `serde`/`rkyv`/`zeroize`.
- Ball arithmetic (the `mpcb_t` analogue — interval/uncertainty complex).
- Independent re/im rounding (`CRound` trait; MPC `mpc_rnd_t` parity).
- `num_complex::Complex<FBig>` interop (feature `num-complex_v04`).
- `cplx!`/`static_cplx!` literal macros.
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
  (the fixed-width difficulty that Ziv sidesteps here).
- **C. F. Borges, "An Improved Algorithm for hypot(a,b)" (2019)** — overflow-safe, correctly-rounded
  `abs`/hypot (the `abs` kernel in §6.2).
- **Smith's method** (complex division, overflow-safe) and the **Gauss/Karatsuba 3-mul** form —
  standard named algorithms used as implementation baselines.
- **`num-complex`** — Rust API idioms surveyed: `norm_sqr` vs `norm` (→ our `norm` vs `abs`),
  `arg = atan2(im,re)`, `to_polar`/`from_polar`/`cis`, tiered method availability, `no_std`.
- **`TODO-v05.md` Phase 3** — the original `CBig` spec this plan expands (§3.1–3.4); superseded on
  naming (`cplx`) and elaborated on rounding/inexactness/algorithms/tests/benchmarks.
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
  Library With Correct Rounding” (ACM TOMS, 2007)** — the Ziv/correct-rounding strategy `FBig`
  inherits and `CBig` reuses.
- **“Accuracy of Complex Mathematical Operations and Functions in C23” (HAL hal-04714173, 2024)** —
  recent survey testing C23 complex `libm` against MPC; useful Annex-G special-value reference.
