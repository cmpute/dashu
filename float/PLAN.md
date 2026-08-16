# `dashu-float` — Mag-radius error propagation

Design record for migrating the internal Ziv error-tracking type from an IBig ulp-count ball
to a fixed-width value-space radius (`Mag`).

> **Status.** Design stage — nothing implemented. Branch `float-mag-ball` (off `master`
> `12875d8`). The parallel branch `trig_unit` (×u/×π trig families, exact integer reduction)
> is ignored during this work and rebased on top afterwards.

---

## 1. Background

The Ziv driver (`src/ziv.rs`) and its contract — `approx(guard) -> (value, radius)` certified
against the rounding preimage — are **unchanged** by this design. What changes is the internal
type that produces the radius: `src/ball.rs`'s

```rust
pub(crate) struct Ball<const B: Word> {
    mid: FBig<mode::HalfEven, B>,
    n: IBig,   // |mid − true| ≤ n · ulp(mid)
}
```

Three structural problems of the ulp-count representation motivate the migration:

1. **Bookkeeping cost.** Per-op error propagation is IBig arithmetic with allocations
   (`mul_error`'s `n × significand` products, `ceil_shift`s). Measured: the Ball rewrite
   regressed `ln` 2–3×, engineered back to parity at large precision and ~1.4× at 10³ bits
   (commit `494394f`).
2. **Ulp-domain shift algebra.** Every propagation formula derives an exponent shift over
   `lead_exp`/`precision`/raw-exponent terms. Three under-bounding bugs of this class were
   fixed in `326b6ef` (exp_ball `sig_r`, sqrt `e_r` vs `lead_r`, ln_1p precision-difference).
3. **Dynamic range.** The count is not a small integer: the powi chain compounds to
   ~`2^nlen · ulp` (`MAX_POWI_CHAIN_BITS = 64`), and `ln_compute`'s s<0 path multiplies `n`
   by `2^p` via `rescale_precision` (`src/log.rs:331`). No fixed-width counter fits (u64
   breaks at nlen ≥ ~61 / p ≥ ~60; u128 at p ≥ ~124); the IBig is load-bearing. A
   significand×exponent radius with round-up — `Mag` — has no such cliff, and the
   `rescale_precision` mechanism (and its bug class, `756e0fb`) disappears entirely.

## 2. Fact base (repo analysis)

### 2.1 dashu-int needs no change
`UBig::as_words() -> &[Word]` (`integer/src/ubig.rs:89`) is public, O(1), zero-allocation
(little-endian; `words[len-1]` is the most-significant word); `IBig::as_sign_words()`
(`integer/src/ibig.rs:82`) likewise. `Mag::from_repr` is therefore O(1) today:

```rust
let (sign, words) = repr.significand().as_sign_words();
let bits = Word::BITS as usize;
let bit_len = words.len() * bits - words.last().unwrap().leading_zeros() as usize;
// the top Word::BITS bits always lie within the top two words:
let top = (((hi as DoubleWord) << bits) | next as DoubleWord) >> (bit_len - bits);
```

Round up unconditionally (`+1`, ≤1 Mag-ulp loose, sound). Not usable: `clear_high_bits`
(keeps *low* bits, allocates), `shr` (allocates a new result per call).

### 2.2 Kernel surface (midpoint ops on bare `Repr`)
- `Context::addsub_vv/vr/rv/rr(&Repr, &Repr, Sign) -> Rounded<Repr<B>>` (`src/add.rs:426–524`)
  — bare-Repr add/sub kernels, ownership-aware, exactness via `Approximation`.
- `Context::repr_div(Repr, Repr) -> FpResult<Repr<B>>` (`src/div.rs:298`).
- `Context::repr_round` / `repr_round_ref` (`src/repr.rs:722/743`) — `pub(crate)`, usable.
- `Context::mul(&Repr, &Repr) -> FpResult<FBig<R,B>>` (`src/mul.rs:155`) — no bare-Repr
  variant; unwrap via `?.value().into_repr()` (FBig wrapping is zero-cost; `Context` is
  `Copy`). No new kernel added unless a benchmark says otherwise.
- `sqrt`: keep the existing (Ziv-correctly-rounded) FBig-returning path for midpoints.
- Errors: `mul`/`div` can return `Overflow`/`Underflow` (powi chains depend on propagating
  them); `addsub_*` panic on infinite inputs — Ball midpoints are finite by construction
  (±∞ pre-checked at the public layer, as today).

### 2.3 Consumer surface
- `mid.precision()` is read ~38× — the dominant pattern; at every site the working precision
  is already in scope (work `Context` / `self.precision` / Ziv-closure guard arithmetic), so
  explicit threading is mechanical.
- Series-break idiom `term.mid.abs_cmp(&sum.mid.ulp_lb()).is_le()` at 8 sites
  (exp/log/trig/atan) — needs a helper (§3.8).
- `src/math/hyper.rs` consumes only Ball methods (zero direct `.mid` access) — structure
  unchanged, but every op line gains `?` + `prec` (mechanical, not near-zero). `src/root.rs`
  hypot simplifies (`*_tracking` deleted, §3.5). `src/convert.rs:796` becomes cleaner.
- `FpResult<T> = Result<Rounded<T>, FpError>` (`src/error.rs:87`) — every kernel carries the
  Exact/Inexact flag, so the ε term can be **conditional** (§3.4). This one semantic replaces
  the entire `*_tracking` family, including `ln_compute`'s `scale_int_tracking` exactness
  requirement and powi/hypot's exact-chain certification.
- Two awkward sites, both resolved by wrapping the mid in a temporary
  `FBig::new(repr.clone(), ctx)`: `src/exp.rs:159` (`div_rem_euclid`, reduction quotient
  `s = ⌊x/ln B⌋`) and `src/trig.rs:83` (mid division by π/2).
- `ConstCache` returns `FBig`/`Rounded<FBig>` — wrapped into balls at call sites via
  `with_error(repr, k·ulp_mag)` (the "8 ulps" constants).

## 3. Design

### 3.1 Types

```rust
// src/mag.rs (pub(crate) — permanently internal; a public Mag API is ruled out by design,
// and dashu-ball keeps its own u64-width Mag — no sharing is planned)
// man: Word, NOT a fixed u64 — with the public-API endgame dropped and no f64 constructors
// in the internal op subset, the arguments for u64 vanish. Word matches the currency of
// as_words()/DoubleWord (products are native-width on every target, incl. thumbv7em) and
// the usize-typed scalars of the call sites. Accepted cost: mantissa width (and radius
// slop) is platform-dependent — 32-bit targets get 2^-32-per-op radius rounding. Sound at
// any width (slop ≪ the 1-ulp ε term); worst effect is a Ziv retry-count difference on
// adversarially near-tie inputs on 32-bit, never a wrong result.
pub(crate) struct Mag { man: Word, exp: isize } // sentinels 0 / +∞, Word::BITS normalized,
                                                // every operation rounds up
// op subset only — core-only (float is no_std-capable), no f64 anywhere, no transcendental
// bounds except one private `exp_upper` (§3.7 exp fold):
// ZERO/INFINITY, from_pow2, from_word(usize), from_repr/from_repr_lower (O(1), as_words —
// NOT a port of extras' from_int_rounded), add, sub (floors at 0), mul (0×∞=0), div (÷0→∞),
// mul_pow2, cmp (Ord), is_zero/is_infinite, mul_down + sub_down (down-twins, for the div
// denominator and mag_lower), exp_upper (new: halve-then-pow, e^v ≤ (1+2v·2⁻ʲ)^2ʲ via
// Mag::pow; v ≥ 2^62 → INFINITY), to_repr (§3.9)

// src/ball.rs (rewritten)
pub(crate) struct Ball<const B: Word> {
    mid: Repr<B>,  // bare midpoint — precision is a property of operations, not of the ball
    rad: Mag,      // absolute radius; rad == 0 ⟺ exact chain
}
```

The struct stores **no** precision: a stored precision is exactly the state that
`rescale_precision` existed to maintain (and mis-maintain).

### 3.2 Precision convention
Per-operation, explicit, **last argument**: `x.mul(&y, prec)`. Same shape as `dashu-ball`'s
public API (future kernel unification) and Arb's `prec` parameter.

### 3.3 Form of the mechanical propagation

| Form | Decision | Reason |
|---|---|---|
| **Inherent methods on `Ball` + `prec`** | ✅ | Precision visible at every call (auditable); no trait machinery; matches `dashu-ball`/Arb; fully visible to rustdoc/IDE |
| Operator overloading | ❌ | Operators cannot take the extra `prec`; builder forms (`x.at(prec) * y`) hide it |
| Macros | ❌ | Series loops differ materially (alternating vs same-sign, 1–2 accumulators, break variants) → many arms; hides calls from rustdoc; loop bodies are only 3–4 lines after migration |
| `Context` methods | ⚠️ algorithm layer only | Two-layer split: propagation operators on `Ball`; algorithm helpers (`sinh_series`, `atan_compute`, …) stay on `Context` as today (they bundle work context + `ConstCache`) |
| Free functions | ❌ | Reads poorly in chains |

### 3.4 Operation surface

```rust
impl<const B: Word> Ball<B> {
    // construction
    fn exact(mid: Repr<B>) -> Self;                              // rad = 0
    fn from_rounded(mid: Rounded<Repr<B>>, prec: usize) -> Self; // inexact → fold ε = 1 ulp
    fn exact_int(k: IBig, prec: usize) -> Self;                  // prec last, like every op
    fn with_error(mid: Repr<B>, rad: Mag) -> Self;               // cached constants' bounds

    // propagation — ops calling Context kernels return FpResult
    // (Overflow/Underflow from powi chains must propagate; Ziv closures are already Result).
    // ε is conditional: FpResult = Result<Rounded<_>, _>, and an Exact kernel result with
    // exact operands leaves rad = 0 — this replaces the whole *_tracking family (§3.5).
    fn add(&self, rhs: &Self, prec: usize) -> FpResult<Self>;
    fn sub(&self, rhs: &Self, prec: usize) -> FpResult<Self>;
    fn mul(&self, rhs: &Self, prec: usize) -> FpResult<Self>;
    fn div(&self, rhs: &Self, prec: usize) -> FpResult<Self>;
    fn div_int(&self, k: usize, prec: usize) -> FpResult<Self>;  // series hot path
    fn scale_int(&self, k: &IBig, prec: usize) -> FpResult<Self>;
    fn sqrt(&self, prec: usize) -> FpResult<Self>;               // zero-mid special case kept
    fn pow(&self, k: &UBig, prec: usize) -> FpResult<Self>;      // binary powering via mul;
                                                                 // replaces pow_exact (exp's Bⁿ chain
                                                                 // + powi); exact chain ⟺ rad == 0

    // exact operations — no prec, no Result
    fn neg(&self) -> Self;
    fn shift(&self, s: isize) -> Self;                           // rad unchanged

    // radius-side
    fn add_error(&mut self, err: Mag);                           // hand tail bounds land here
    fn mag(&self) -> Mag;                                        // |mid|↑ + rad
    fn mag_lower(&self) -> Mag;                                  // from_repr_lower(mid).sub_down(rad)
    fn is_exact(&self) -> bool;                                  // rad.is_zero()

    // Ziv boundary (contract unchanged — ziv.rs untouched)
    fn to_value_radius<R: Round>(&self, ctx: &Context<R>) -> (FBig<R, B>, FBig<R, B>);
}
// free helpers in ball.rs: ulp_mag(mid: &Repr<B>, prec) -> Mag (B^(lead_ub − prec), source of
// ε), ulps(mid, prec, k: usize) -> Mag (k·ulp — the inflate/constants currency),
// mid_le_ulp_lb(term, sum, prec) — the series-break comparison (§3.8).
}
```

### 3.5 Exactness
`rad == Mag::ZERO` ⟺ exact chain. The `*_tracking` family (`mul_tracking`,
`add_tracking`, `sqrt_tracking`, `scale_int_tracking`) and its `&mut bool` plumbing are
**deleted**; callers (hypot's exact chain, powi's exactly-representable directed case) test
`ball.rad.is_zero()`. Exact operands × exact midpoint op → rad stays 0; any rounding →
ε = 1 ulp > 0. Same information, one representation.

### 3.6 Error semantics
Mirror the kernel surface: ops routed through Context kernels return `FpResult<Ball>`;
exact ops (`neg`, `shift`) are infallible.

### 3.7 Radius propagation rules
(`‖·‖` = `Mag::from_repr` upper bound, `↓` = lower bound; ε = `ulp_mag(mid_r, prec)` when the
midpoint op rounded, else 0. These are the rules already validated in `dashu-ball` against
python-flint.)

| op | radius |
|---|---|
| `add`/`sub` | `rad_a + rad_b + ε` |
| `mul` | `‖a.mid‖·rad_b + ‖b.mid‖·rad_a + rad_a·rad_b + ε` |
| `div` | `(‖a.mid‖·rad_b + ‖b.mid‖·rad_a) / denom + ε` where `denom = from_repr_lower(b.mid) ·_down b.mag_lower()` — the product of the two lower bounds (≈ LB(|b|)²), as validated in dashu-ball's `div.rs`; either factor ZERO → rad = `Mag::INFINITY` (sound whole-line; ziv retries — callers' guards keep it unreachable in practice). Sound at any accuracy — no `(1+2⁻¹⁶)` fast path, and the old zero-numerator special case is subsumed by the symmetric numerator |
| `div_int(k)` | `rad_a / k` (up) `+ ε` |
| `scale_int(k)` | `|k|·rad_a + ε` |
| `sqrt` | `rad_a / (2·‖mid_r‖↓) + ε`; zero-mid special case as documented today. The ε term covers the `fl(√a)` vs `√a` denominator gap exactly as the old `+1` did (valid for `rad_a ≤ |mid_a|`, which every caller guarantees) |
| `pow(k)` | mul-chain compounding, mechanical |
| `neg`/`shift` | unchanged |

The two hand-derived derivative folds (previously ulp-domain shift algebra over `ceil_shift`):

- **exp** (`exp_ball`): `result.rad += Mag::exp_upper(‖x.mid + rad_repr‖↑) · x.rad`, where `x.mid + rad_repr`
  is the ball's **upper endpoint** via exact `Repr` addition (signed — using `|x|+rad` instead would blow the
  radius up for negative x and stall ziv). Soundness: `|e^{x+θ} − e^x| ≤ rad_x·e^{x+rad_x} ≤ rad_x·exp_upper(u)`,
  unconditional. (`result.mag()·x.rad` — the old code's shape — under-covers by the `e^{rad_x}` factor and is
  kept sound in the old code only by reachability margins; do not resurrect it.)
- **ln_1p** (`ln_1p_ball`): `adjust = arg.rad / from_repr_lower((1 + arg.mid) − rad_repr)`, endpoints via exact
  `Repr` ops — the log1p rule validated in dashu-ball, tighter than the old `×2` hand formula.

### 3.8 Helpers
- `ulp_mag(mid, prec)`: `B^(lead_exp_ub(mid) − prec)` as a `Mag`
  (`lead_exp = exponent + digits_ub` — over-estimate only loosens); `ulps(mid, prec, k)` =
  `k·ulp_mag` (the inflate/constants currency).
- Series break (8 sites): `Ball::mid_le_ulp_lb(&self, sum: &Self, prec)` — compare
  `|self.mid|` against `Repr::new(IBig::ONE, lead_lb(sum.mid) − prec − 1)` via exact
  `Repr` comparison (`repr_cmp_same_base::<B, true>`, the ABS-generics pub(crate) helper in
  `cmp.rs`; the `digits_lb − 1` slack replicates `FBig::ulp_lb`, `fbig.rs:348` — the break
  feeds the tail-bound soundness; must not be approximate).

### 3.9 Ziv boundary
`to_value_radius` re-tags `mid` to `R` at the working precision and converts `rad` to a
`Repr` (`rad == 0` → `Repr::zero()`, avoiding the −∞ sentinel pitfall noted at
`ball.rs:494`). The containment test needs only a **sound upper bound** on the radius, so
for B = 10 round it outward to a power of ten: value `= man·2^e < 2^bits` with
`bits = bit_len(man) + e` (when `bits > 0`), and `10^k ≥ 2^bits` for `k = ceil(bits·28/93)`
(`28/93 = 0.30107… > log₁₀2 = 0.30103`, valid for either sign of `bits`) →
`Repr::<10>::new(ONE, k)`; ≤ 10× radius slack, O(1). Do **not** build the exact decimal —
`man·5^|exp|` is O(|exp|) IBig work per ziv attempt. For B = 2, `man·2^exp` is directly
`Repr::<2>::new(man, exp − BITS)` — exact.
Closure signature and `ziv.rs` are untouched. This is one of the only three places base
awareness exists (with `from_repr` and `ulp_mag`) — all arithmetic between `Mag`s is
base-free, because a radius is a real magnitude, not a base-B quantity (the old ulp count
was base-anchored by construction, which is why `ceil_shift::<B>` existed at all).

### 3.10 Base 10
`Mag` is base-agnostic (absolute value); only the boundary helpers are base-aware, and
they need bound-level conversion only — one-directional, slack-tolerant, always sound
(the precision-critical kind of base conversion lives in the public `with_base` layer,
whose exp/log machinery the Ball merely rides). `Mag::from_repr::<10>`: exact when
`|exp|` is small (`10^e` fits a `Word`/`u128`), otherwise a power-of-two upper bound
`10^e ≤ 2^⌈3.322·e⌉` (`3.322 > log₂ 10 = 3.321928`, valid for both exponent signs,
slack < a factor 2 for any realistic exponent). The `ceil_shift::<B>`/`shl_digits::<B>`/
`shr_digits_ceil` base-10 machinery in the error terms is deleted.

## 4. Series loop after migration (ln's atanh)

```rust
let mut pow = z.clone();
let mut sum = z;
let mut k = 3usize;
loop {
    pow = pow.mul(&z2, wp)?;
    let increase = pow.div_int(k, wp)?;
    if increase.mid_le_ulp_lb(&sum, wp) { break; }
    sum = sum.add(&increase, wp)?;
    k += 2;
}
sum.add_error(ulps(&sum.mid, wp, B as usize)); // hand tail bound — one line, as before
```

The only call-surface delta vs today: one extra `wp` argument per propagation call — the
same coin as deleting the 38 implicit `mid.precision()` reads.

## 5. Deleted

`term_in_ulps`, `ceil_shift`, `rel_err`, `mul_error`, `Ball::lead_exp`, `div_exact`
(log.rs:316 becomes `div` by an exact ball — the div rule with `rad_b = 0` yields
`rad_a/|b|`; the O(p²) the old method avoided was the IBig rational error arithmetic, which
no longer exists), `rescale_precision` (+ its single call site, `src/log.rs:331–332` — the
s<0 double work precision itself **stays**, moved ahead of ball construction; only the
`n` re-tag goes), `shr_digits_ceil`'s radix tricks, the four `*_tracking` methods
(`pow_exact` and `inflate` are replaced by `pow` and `add_error`, not deleted in name only).
`src/ball.rs` rewrites from ~700 to ~300 lines.

## 6. Migration steps

1. `src/mag.rs`: port the arithmetic subset from `dashu-ball` (normalization cascade
   included); `from_repr` via `as_sign_words` (O(1)); port the round-up invariant unit
   tests. (~+300 lines.)
2. Rewrite `src/ball.rs` per §3.4; radius rules per §3.7; port the `assert_invariant`
   fixtures (assert `|mid − true| ≤ rad` via exact `Repr` arithmetic).
3. Call sites (~110 references, mostly mechanical): `exp.rs`, `log.rs`, `math/trig.rs`,
   `math/hyper.rs` (structure unchanged — zero `.mid` access — but every op line gains
   `?` + `prec`), `root.rs` (simplifies), `convert.rs:796` (cleaner). One atomic commit with
   step 2 — ball.rs and its consumers cannot switch piecemeal.
4. Validation: unit tests + the `ziv_few_retries_for_typical_inputs` sentinel + the
   force_bits width matrix (`RUSTFLAGS='--cfg force_bits="16|32"'`) + no_std check
   (`cargo check -p dashu-float --no-default-features`) + fuzz at `~/dashu/fuzz` (standalone
   workspace, proptest vs **rug**: `cargo test --manifest-path fuzz/Cargo.toml --test
   float_transcendental --test float_trig_random` — final results must be bit-identical;
   only first-attempt guards may differ) + benchmark (`benches/exp.rs`, criterion,
   `cargo bench -p dashu-float --features rand --bench exp`, baseline captured before the
   migration; success criterion: recover the 1.4× at 10³ bits to ~1.0×).
5. Later: rebase `trig_unit` (its `pi_scaled_ball`/`unit_argument_ball` glue collapses to
   one-liners on the new substrate). The `mag` module stays `pub(crate)` permanently — a
   public Mag, and any sharing with `dashu-ball`'s u64-width Mag, is ruled out by design.
   `dashu-complex` is unaffected by this migration: it depends only on float's public API
   (its Ziv driver deliberately mirrors float's through the public `FBig` surface), and its
   per-function hand radii (`ulp()·k` value-space formulas over float's certified
   primitives) have no use for Mag. If complex ever grows mechanical propagation (a
   `CBall`), it carries its own private Mag copy — or a `#[doc(hidden)]` lockstep-shared
   surface, never a public one.

## 7. Resolved decisions

1. `prec` argument position: **last** (`x.mul(&y, prec)`, matches `dashu-ball`); `exact_int`
   follows (`exact_int(k, prec)`).
2. `add_error` form: **`&mut self`** in place (mirrors the old `inflate`'s `&mut`).
3. `mul` unwrap path: **interim `ctx.mul(...)?.value().into_repr()`** — no new kernel
   (`FBig` wrapping is zero-cost; revisit only if a benchmark says otherwise).
