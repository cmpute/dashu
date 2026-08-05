# Changelog

## Unreleased

### Change
- **(internal) `log2` error radius now derived by Ball arithmetic instead of a directed interval.**
  `ln_compute` and `log2_internal` are rewritten around a new `Ball` type (`float/src/ball.rs`): a
  midpoint plus an exact integer relative-error count (`|mid − true| ≤ n·ulp(mid)`). The radius is
  composed mechanically through the atanh series, the `s·ln(B)` reconstruction, and the
  `ln(x)/ln(2)` quotient — the hand-derived `(4·terms + 12)·ulp` formula, the outward-rounded
  `[lo, hi]` interval, and its `INTERVAL_GUARD` constant are gone. The propagation rules are
  precision-aware (an operand that over-delivers its context, e.g. the uncached `ln(2)` constant,
  has its error converted correctly to the result's ulp). Public behavior is unchanged and
  validated against a high-precision oracle and against `rug`/MPFR (bit-exact).
  - **Performance parity is preserved.** The Ball overhead is kept near the legacy cost: `lead_exp`
    uses the cheap `digits_ub`/`digits_lb` bounds (never the exact digit count), every `·B^exp`
    power computation goes through the shared `shl_digits` primitive and every `⌈x/B^k⌉` through
    the new `shr_digits_ceil` (bit shifts for power-of-two bases, the `(x·5^k)<<k` radix-factor
    trick for base 10 — no `B^k` materialization or `O(p²)` division), the multiply
    avoids cloning significands, and
    division by any exact integer (`div_exact`, used for the series hot path `div_int` and the
    `x/2^s` reduction) shrinks the error by `k` directly instead of running the general rational
    division. The general `div` is retained only where the divisor is itself approximate (the
    `(x−1)/(x+1)` series reduction and the `ln(x)/ln(2)` quotient), each O(1) per call.
    Benchmarked `ln`/`exp` (`cargo bench -p dashu-float --bench exp`) are at parity with the
    pre-Ball implementation at large precisions (10⁴ bits) and within ~1.4× at 10³ bits; the
    residual gap is the fixed per-operation overhead at small precisions.
- **(internal) `exp`/`exp_m1`, all hyperbolic, all trigonometric, `powi`, and `hypot` radius
  derivations migrated to the same `Ball` engine.** `exp_compute`, the trig Maclaurin/Euler
  series (`sin`/`cos`/`sin_cos`/`atan`), `powi`'s squaring chain, and `hypot`'s
  `sqrt(large²+small²)` now propagate error mechanically through the `Ball` ops (new `mul`,
  `div`, `div_int`, `scale_int`, `shift`, `pow_exact`, `sqrt`, and exact-tracking
  `mul_tracking`/`add_tracking`/`sqrt_tracking` for the exactly-representable directed-rounding
  cases). The composed functions (`sinh`/`cosh`/`tanh`/`asinh`/`acosh`/`atanh`,
  `asin`/`acos`/`atan`/`atan2`) build on the `Ball` primitives directly with a single outer Ziv
  certification; the trig quadrant-reduction error and π's radius are folded in mechanically.
  The `result.ulp()·{12,14,16}` constants, `reduce_to_quadrant`'s `reduction_err`, `powi`'s
  `<<(nlen+1)`, and `hypot`'s `ulp·8` are gone. `pow_exp_log` (the `powf`/`powi` fallback path) is
  also composed from `ln_compute` + `mul` + `exp_ball`, with the input-ball error folded into the
  `exp` radius. Public behavior is unchanged and validated against the same oracle/fuzz nets.

### Add
- **`tuning` feature + `ziv_retries()`/`ziv_retries_reset()`.** Exposes the per-Ziv-loop retry
  counter (extra attempts beyond the first) for profiling how tight each transcendental's
  error-radius bound is at a given target precision. Implies `std` (the counter is a
  `thread_local`). Named `tuning` to match dashu-int's existing `tuning` feature (the umbrella
  `dashu` crate's `tuning` enables both sub-crates). Used by the dashu-python
  `ziv_retries`/`ziv_retries_reset` bindings and the `python/scripts/ziv_profile.py` script.

### Fix
- **`powf`/`powi` no longer return wrongly-rounded results for large `|y·ln x|`.** `exp_ball`'s
  input-error inflation omitted the result-significand factor `sig_r` (`|exp|/ulp(exp) ≈ sig_r`),
  so the radius under-bounded the propagated input error by up to `B^(p−1)` and the Ziv containment
  test could certify an interval that did not contain the true value. The inflate term now
  multiplies the error count by the result's significand.
- **`sqrt`-based radius under-bound in `asin`/`asinh`/`acosh`/`hypot`.** `Ball::sqrt` (and
  `sqrt_tracking`) computed the error shift with the leading position `E_r` where the raw
  significand exponent `e_r` belongs, dropping the digit count from the denominator — the radius
  no longer satisfied `|mid − true| ≤ n·ulp(mid)` once the input error grew past a couple of ulps.
  Both use the raw exponent now.
- **`atanh(x)` for `x < 0` (near the pole) returned wrongly-rounded values.** `ln_1p_ball`'s
  input-error adjust dropped the precision-difference term (`−p_arg+p_ln`); `ln_compute`'s s<0 path
  doubles the work precision, so the adjust under-bounded by `B^precision` and the Ziv loop
  mis-certified (up to ~2^13 ulps off near `x = −1`). The adjust is now precision-aware.
- **`cargo test -p dashu-float --no-default-features` compiles again.** The ziv test module's
  `#[cfg(test)]` gate referenced the `std`-only `LAST_ZIV_RETRIES` counter; the module is gated on
  `all(test, std)` again.
- **`powf` of a base `< 1` no longer hangs in the Ziv loop.** `ln_compute`'s s<0 reduction
  (`x·2^|s|` scaling before the cancelled `ln(x_scaled) + s·ln(B)` reconstruction) inflated the
  `Ball` error count by a fixed `B^precision` factor even for exactly-representable inputs —
  `scale_int` folded a spurious `+1` that `rescale_precision` then amplified, leaving the radius
  constant across Ziv retries (the composed `exp(y·ln x)` chain never converged). The reduction
  now uses the exact-tracking `scale_int_tracking` (n stays 0 for an exact scaling), so the radius
  shrinks with the guard and `powf` certifies in 1-2 attempts. This also re-enables the
  Ball-composed `pow_exp_log` (`ln_compute` + `exp_ball`) that replaces the hand-derived radius.

## 0.6.0-rc.2

### Change
- **(internal) Unified `f32`/`f64` range detection.** `into_f32_internal`/`into_f64_internal` now
  return `FpResult` (`Ok` in range, `Err(Overflow)`/`Err(Underflow)` at the extremes), so the
  range decision lives in one place — `convert_to_f32`/`convert_to_f64` — shared by the infallible
  `to_f32`/`to_f64` (which saturate the `FpError` to the directed endpoint) and the fallible
  `TryFrom` (which maps it to `OutOfBounds`/`LossOfPrecision`). The four directed-endpoint blocks
  collapse into one helper per float type. No observable behavior change.
- **`Context::exp` of an astronomically large negative value now returns `Err(Underflow)`.** When
  the reduction quotient `s = floor(x/ln B)` overflows `isize` (astronomical `|x|`, ≳2⁶¹), `exp` of
  a negative `x` is a positive value below the smallest representable; it now reports `Err(Underflow)`
  (was `Ok` of the directed endpoint value). `FBig::exp` is unchanged (`unwrap_fp` maps it to the
  same endpoint: `+0` under nearest/inward, smallest-positive under `Up`/`Away`), and `exp_m1` of the
  same input is unchanged (≈−1, a value). This makes `exp`'s underflow an error, consistent with its
  overflow.
- **(internal) `ziv`/`ziv_pair` now take fallible closures; hoisted overflow probes removed.** The
  Ziv driver closures can now return `Err(FpError)` (propagated on the first overflowing attempt), so
  `exp_internal`/`pow_exp_log`/`sinh`/`cosh`/`sinh_cosh` detect overflow *inside* the loop and
  propagate it (mapping the directed sign at the call site) instead of carrying hoisted
  `exp_overflows` probes and `.unwrap()`s. The `exp_overflows` helper is deleted; `ziv_fallible` is
  merged into `ziv`. `ln_internal`/`log2_internal` now return `FpResult`. No observable change beyond
  the `exp` contract note above.

### Fix
- **Directed overflow/underflow of FBig results (`Context::unwrap_fp`).** `Err(Overflow)` and
  `Err(Underflow)` now saturate to the **mode-aware** endpoint instead of a mode-blind `±∞`/signed
  zero. Overflow: outward modes (and nearest) reach `±∞`; inward modes (toward-zero,
  opposite-infinity) saturate to the largest finite `(Bᵖ−1) × B^{isize::MAX}` — the all-`(B−1)`
  significand at the max exponent, mirroring MPFR's `mpfr_setmax` (the significand is `p` digits,
  the output precision, not the value's magnitude). Underflow: outward modes reach the smallest
  representable `B^{isize::MIN}`; inward/nearest reach signed zero. So `Up ≥ Down` now holds on
  both ends and across `exp`/`pow`/`powf`/`mul`/`div`/`sinh`/`cosh` (e.g. `Up(pow(x,y))` agrees
  with `Up(exp(y·ln x))`). The `mul`/`div` *operators* now route through `Context::mul`/`div` +
  `unwrap_fp` (the `Mul`/`Div` trait impls previously bypassed it and saturated at the `Repr`
  kernel, mode-blind). The two endpoints live in shared helpers
  (`overflow_repr_endpoint`/`underflow_repr_endpoint`). Overflow at unlimited precision **panics**
  (the largest finite is undefined there).
- **Directed overflow/underflow of `FBig → f32`/`f64` (range detection).** The directed-endpoint
  branches were gated on the *least*-significant-bit exponent, which is not the overflow/underflow
  threshold: a value whose significand straddles `f32::MAX` (e.g. `3·2¹²⁷`, lsb exponent 127 < 128)
  still overflows but fell through to `encode`, which saturates to `±∞` mode-blindly — so under
  toward-zero it returned `+∞` instead of `f32::MAX`. Symmetrically, `2⁻¹⁶⁰` (lsb exponent −160,
  above the `−173` underflow gate) reached `encode` and returned `±0` mode-blindly — under `Up` a
  positive value below the smallest subnormal must reach `2⁻¹⁴⁹`. Range detection is now delegated
  to `encode` (which tests the *most*-significant bit), and only the saturation endpoint
  (`±MAX` vs `±∞`, `±0` vs `±smallest-subnormal`) is chosen per rounding mode.
- **`TryFrom<FBig> for f32`/`f64` error variant is now mode-independent.** A finite value beyond
  `±MAX` previously returned `Err(OutOfBounds)` under nearest modes (which round to `±∞`) but
  `Err(LossOfPrecision)` under directed modes (which saturate to `±MAX`), because the variant was
  derived from the *result's* infiniteness. It is now classified by the *input* magnitude, so
  `Err(OutOfBounds)` reliably means "beyond the finite range" under every rounding mode. (`to_f32`/
  `to_f64`, which return the mode-aware `Rounded<f32>`/`Rounded<f64>`, are unchanged.)
- **Directed `ln`/`log2` of `x ∈ [1, B)` and of `x` just above 1.** `ln_compute`'s error radius was
  unsound in two ways for the unit binade: (1) it used `result.ulp()`, but `result` inherits
  `ln_base`'s over-delivered context (~`work + guard` digits) even when the `s·ln(B)` term is zero
  (`s = 0`), so the radius under-estimated the work-precision-scale error by ~`B^guard`; the radius
  is now widened by the context inflation. (2) For `x` just above 1, `log2_bounds` can classify
  `s = −1`, so `result = 2·sum + s·ln(B)` cancels — the absolute error then stays at `sum`'s
  magnitude while `result`'s collapses, and `result.ulp()` vastly under-estimates it; the radius now
  also covers the pre-cancellation (`sum`) scale. Both let Ziv certify the wrong 1-ULP neighbor for
  ~1–5% of directed `ln`/`log2` inputs in `[1, B)` at low precision.
- **`exp`/`exp_m1` extreme-negative endpoint carried precision 0.** The mode-aware saturation
  returned the precision-0 constants `FBig::ZERO` / `−FBig::ONE`, so a downstream op on the result
  (`e.sqrt()`, …) panicked via `assert_limited_precision(0)`. The endpoint is now built with the
  input context.
- **`exp` overflow probe could disagree with the computation.** The hoisted probe computed the
  reduction quotient `s = floor(x/ln B)` at a fixed `p + 64` bits, while `exp_compute` inflates
  `ln B` by `⌈log_B|x|⌉ + 2` extra digits — so for huge `|x|` the two could disagree on whether `s`
  fits `isize`, and `exp_compute`'s `s.try_into()` could then panic. The probe now applies the same
  inflation, so its verdict matches the computation's. Separately, the probe's fast-skip threshold
  was the 64-bit literal `61`, so on 32-bit `isize` (`log2(isize::MAX) ≈ 31`) inputs like
  `exp(-2⁵⁰)` — whose `s ≈ -1.8e15` overflows 32-bit `isize` — skipped the probe and panicked in
  `exp_compute`. The threshold is now `isize::BITS − 3` (61 on 64-bit, 29 on 32-bit).
- **`ulp()`/`ulp_lb()` panic on an extreme exponent.** The ulp exponent `e + digits − precision`
  was computed with wrapping `isize` arithmetic, so a value near the representable exponent floor
  (e.g. a `powi` result just above the smallest representable) underflowed and panicked inside the
  Ziv containment test. The arithmetic is now saturating; an extreme exponent yields a saturated
  (smallest-representable) ulp.
- **`exp_m1` of large negative input.** For `x` negative enough that `exp(x)` is below the result's
  precision, `exp_m1(x)` is `−1` plus a sub-ulp residual whose rounding is fully determined, but
  the Ziv loop could not certify it: the working-precision value collapses to exactly `−1` and a
  directed rounding preimage is one-sided, so the containment test never resolved and the loop ran
  to its retry cap (pathologically slow in debug builds). Such inputs now short-circuit to the same
  mode-aware endpoint as the underflowed case.
- **`exp` range reduction for large `|x|`.** The reduction quotient `s = floor(x/ln B)` amplifies a
  1-ulp error in `ln B` by `|x|/ln B`, so for large `|x|` the work-precision `ln B` left `s` (and
  hence the result exponent) off by ~`|x|`, and `Up(exp)` could fall below `Down(exp)`.
  `exp_compute` now computes `ln B` with `⌈log_B|x|⌉ + 2` extra digits, so the reduction contributes
  well under one work-ulp.
- **`FBig → f32`/`f64` subnormal mis-round for wide decimal significands.** The round-to-odd base
  conversion was computed straight at the target width, so the near-correct `ln`/`exp` series (a
  few-ulp error at the work precision) could land a value whose true result sits within ~`2^{-2w}`
  of a `w`-bit midpoint on the wrong side, rounding to the neighbor 1 ULP off. `convert_base_odd`
  now converts at `width + 24` bits and round-to-odd's down to `width`, pushing the residual well
  inside one `w`-bit ulp.
- **`FBig → f32`/`f64` nearest-even underflow of a catastrophically tiny source.** For a value far
  below half the smallest subnormal (e.g. a wide-significand decimal at a hugely negative exponent),
  the base conversion's internal `exp(remainder)` itself underflows, so the converted `odd` carried a
  wildly wrong (too large) magnitude and `encode` never flagged the underflow — `to_f64` returned a
  spurious finite subnormal (≈2⁻⁵⁹³) instead of the directed underflow endpoint. There is now a
  source-`log2_bounds` short-circuit (`|x| < 2⁻¹⁰⁷⁵` for f64, `< 2⁻¹⁵⁰` for f32 — the `½·MIN_SUBNORMAL`
  cutoff) that returns `Err(Underflow)` before the conversion can corrupt the magnitude; the existing
  directed endpoint then gives `±0` (nearest) or the smallest subnormal (outward).
- **`powi` exhausted memory / hung on an extreme integer exponent.** The magnitude guard used exact
  `log2_bounds(base)` but scaled by an f64 `e` and an `isize::MAX as f64` threshold, so (a) a result
  near the finite-range boundary could be misclassified as overflow (a representable value spuriously
  saturated to `±∞`), and (b) an exponent past `i64` with `|base| ≈ 1` slipped through to the squaring
  chain, whose working precision grows with `n.bit_len()` — a single Ziv attempt then allocated
  unboundedly. The guard now uses a margin so only a *certified* extreme result short-circuits (the
  gray zone falls through to the chain, whose `checked_mul` catches a genuine overflow), and
  exponents past `i64` route to an `exp(y·ln x)` fallback (shared with `powf`) whose working
  precision does not scale with the exponent's bit length, so it cannot exhaust memory. This retires
  the `powi` range-handling TODO.
- **`Repr::cmp` overflow near the exponent ceiling.** `repr_cmp_same_base` computed
  `exponent + digits` in plain `isize` for its magnitude shortcuts; for a result near `isize::MAX`
  (e.g. `powi(2, n)` with `n` just below `isize::MAX`, which is representable so the range guard does
  not short-circuit it) that add overflowed and aborted inside the Ziv containment test. The
  shortcuts now use saturating arithmetic (an overflow forgoes the shortcut, falling through to the
  exact comparison), so a near-ceiling power of two compares correctly.
- **`powi` panicked at the exact exponent ceiling.** When `base^n`'s magnitude reaches the finite-
  range ceiling/floor — the squaring chain's exponent arithmetic hits the `±isize::MAX` sentinel —
  `powi_chain` saturated the step to an infinity `FBig`, which the Ziv closure then fed to
  `res.ulp().with_precision(0)`, panicking (`assert_finite`) because the closure's signature cannot
  return `Err`. So `powi(2, isize::MAX)` (and any `powi(base, n)` with `n.bit_len() ≤ 64` whose
  result genuinely overflows) aborted instead of returning the directed endpoint. The chain now
  propagates the `Overflow`/`Underflow`, a new fallible `ziv_fallible` carries it out of the loop,
  and `powi` maps it to the mode-aware endpoint with the correct result sign (base sign × exponent
  parity). The negative-exponent reciprocal path (extreme-base `1/base` underflow) is covered by the
  same propagation.

## 0.6.0-rc.1

### Add
- **Exact `Add`/`Sub`/`Mul` operators for `Repr`** (new `repr_ops` module). A `Repr` carries no
  precision limit, so add/sub/mul on it are lossless — these are the shared primitives the crate uses
  for exact intermediates (the Ziv containment test, the correctly-rounded `Sum`, and the `FBig`
  multiply path). `Mul` saturates exponent overflow/underflow to the signed infinity/zero sentinels,
  so the operator is infallible; the internal `make_mul_repr`/`unwrap_mul_repr!` helpers are removed
  in favor of the operator, and the `Neg` impl moved here from `sign.rs`. No behavior change to
  `FBig` arithmetic.
- **Guaranteed-correct rounding for `exp`, `exp_m1`, `ln`, `ln_1p`** via a Ziv retry loop. A generic
  `Context::ziv` driver rounds the working-precision approximation to the target precision and
  verifies, against the `ErrorBounds` rounding preimage, that the approximation's provable error
  interval lies entirely inside one rounding bin; if not, it retries with more guard digits. The loop
  provably terminates and preserves the `Exact`/`Inexact` flag. Series evaluation is factored into
  near-correct `ln_compute`/`exp_compute` cores (`R: Round`) that the Ziv wrapper certifies.
- **Tightened `exp` guard digits** (now a performance knob, since Ziv — not the guard count —
  guarantees correctness): the `Bⁿ`-powering guard is halved from `2n` to `n` and the series guard
  drops its conservative `+ 2`.
- **Guaranteed-correct rounding for the remaining transcendentals** via the Ziv loop: the
  trigonometric family (`sin`, `cos`, `sin_cos`, `tan`, `asin`, `acos`, `atan`, `atan2`) and the
  hyperbolic family (`sinh`, `cosh`, `sinh_cosh`, `tanh`, `asinh`, `acosh`, `atanh`). The trig series
  (`sin`/`cos`/`atan`) are factored into near-correct `_compute` cores like `exp_compute`; the
  composition-based functions treat the now-Ziv-correct `exp`/`ln`/`atan` as black boxes and count
  only their arithmetic. The trig argument reduction folds a `|k|·ulp(π/2)` reduction-error term into
  the radius so the containment test stays sound for huge `|x|`. `ziv_pair` certifies both halves of
  `sin_cos`/`sinh_cosh`.
- **`hypot` is now correctly rounded** with MPFR-style exactness tracking: the closure computes
  `sqrt(large² + small²)` (operands scaled so `large²` can't overflow the exponent) and OR's each
  step's `Exact`/`Inexact` flag, mirroring MPFR's `exact` flag — an all-exact chain yields the exact
  true value, reported to `ziv` with radius 0, which it accepts without the containment test. This is
  what lets an exact Pythagorean-triple result (`hypot(3,4)=5`, `hypot(5,12)=13`) terminate under a
  directed rounding mode, where the one-sided preimage would otherwise make the containment test
  infinite-retry.
- **Guaranteed-correct rounding for `powf` (non-integer exponent)** via the Ziv loop. `x^y =
  exp(y·ln x)`, and `exp` amplifies the rounding of `y·ln x` by the result magnitude — so the radius
  is `result.ulp()·(|y·ln x|+1)·(B+8)` taken at the *working* precision: it shrinks as `B^{-guard}`,
  so the containment test converges (a radius computed at unlimited precision would be constant
  across retries and never settle for a value near a rounding boundary). An integer-valued exponent
  delegates to `powi` (binary exponentiation), which also admits a negative base — its sign fixed by
  the exponent's parity — so `powf(-x, n)` is in domain for integer `n`.
- **Guaranteed-correct rounding for `powi` (integer exponent)** via the Ziv loop. Binary
  exponentiation (repeated squaring) compounds the relative error — it roughly doubles per squaring
  — so after `bit_len(n)` squarings the Ziv radius reflects it (`ulp_w << (nlen + 1)`). A negative
  exponent computes `(1/base)^|n|` directly, so sign-dependent overflow/underflow falls out
  naturally. When the squaring chain rounds `Exact` (the result is exactly representable, e.g. an
  integer power that fits), the true error is zero and the radius is reported as zero — required
  under the *directed* rounding modes (`Zero`/`Down`/`Up`/`Away`, the `FBig` default), where an
  exactly-representable result lies on a one-sided rounding boundary that no nonzero radius can fit
  inside.
- `Repr::is_int` is now `const` (it only inspects the exponent and the infinity sentinel).

### Change
- **(breaking, bound)** The Ziv-backed transcendentals now require `R: ErrorBounds` rather than
  `R: Round`: `exp`/`exp_m1`/`ln`/`ln_1p`, `powf`, `powi`, `hypot`, the trigonometric family
  (`sin`/`cos`/`sin_cos`/`tan`/`asin`/`acos`/`atan`/`atan2`), and the hyperbolic family — the Ziv
  containment test needs the rounding preimage that `ErrorBounds` provides. All six built-in modes
  satisfy `ErrorBounds`; only custom non-`ErrorBounds` `Round` modes are affected (custom modes are
  already discouraged), and the `num_traits::Pow<IBig>`/`Pow<&FBig>` impls for `FBig` likewise
  tighten to `R: ErrorBounds` (they route through `powi`). Base conversion deliberately stays
  `R: Round` by routing its `ln`/`exp` calls through the near-correct `ln_compute`/`exp_compute`
  cores; arithmetic (`add`/`sub`/`mul`/`div`/`sqr`/`cubic`), `sqrt`/`cbrt`/`nth_root`, and the `e()`
  constant are unchanged.
- `FBig::log2` is now correctly rounded via the Ziv loop (it was near-correct in 0.5.1, evaluated as
  `ln(x)/ln(2)` at elevated precision). Each `ln_compute` reports its provable radius and the two
  radii are carried through the division as an outward-rounded interval `[lo, hi]` the Ziv driver
  certifies; an exact power of two short-circuits to the integer `log2(x)` (required under directed
  modes, where the exactly-representable integer sits on a one-sided boundary no positive radius can
  fit inside, so `log2(2^-159)` under `Up` is the exact `-159`, not `-159 + 1 ulp`).
- **Internal dedup** (no behavior change): the `⌈log_B(precision)⌉` base-guard formula shared by
  every transcendental Ziv loop is now `Context::base_guard_digits::<B>()` (12 call sites in
  `hyper`/`exp`/`log`), and the `ulp·(4·terms + 12)` series-truncation radius is now
  `series_radius(value, terms)` (shared by the `sin`/`cos`/`sin_cos`/`atan`/`ln` series cores). The
  two textually-identical `CachedFBig` forwarding macros (`forward_to_context!` /
  `forward_to_context_unwrap!`) are merged.

### Fix
- **Directed-rounding hang on exact results** (`acos(1)`, `acos(-1)`, `asin(0)`, `hypot` of a
  Pythagorean triple): under a directed mode (Down/Up/Zero) a function whose true value is exactly
  representable carries a positive radius that can't be certified against the value's one-sided
  rounding preimage, so the Ziv containment test infinite-retried (and the retry eventually tripped a
  `dashu-int` NTT assertion, now fixed). `acos` short-circuits `|x| = 1` to the exact `0` / `π`,
  `asin` short-circuits `x = 0` to `±0` (matching the existing special cases in the rest of the
  inverse trig/hyperbolic family), and `hypot` reports radius 0 when its computation chain is exact.
- **`tan` no longer has a hoisted pole check.** The check (which tested `cos` with `is_pos_zero`,
  missing `-0`, and computed a `±∞` pole sign) re-evaluated the sin/cos series a second time on top of
  the first Ziv attempt — a ~2× cost on every `tan` call. It's removed: near a pole (an odd multiple
  of π/2) the value is large but finite, and dashu's wide exponent range holds it as a finite number
  whose sign is carried by the arithmetic (`s/−|c|` is negative), so no `±∞` special-case is needed.
  The unreachable exact-pole case (cos cancelling to a zero significand — impossible for finite-
  precision input, since a `p`-digit rational can't sit closer than ~`B⁻ᵖ` to the irrational pole) is
  handled by the Ziv closure's `significand.is_zero()` retry guard.
- **`no_std` build of the test-only Ziv retry counter.** The `LAST_ZIV_RETRIES` `thread_local!`
  (used by the retry-count tests) requires `std`, so the crate failed to compile under
  `--no-default-features` (the `thread_local!` macro isn't in scope). It's now gated behind
  `feature = "std"` along with its uses and the counter-reading tests, so the Ziv driver itself is
  `no_std`-clean; the retry-count tests run under `std` as before.

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
