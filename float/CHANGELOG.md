# Changelog

## Unreleased

### Add
- ×u trigonometric functions `sin_unit`/`cos_unit`/`sin_cos_unit`/`tan_unit` (of `2π·x/u`, the
  argument in units of the full turn divided by `u` — e.g. `u = 360` gives degrees) and their
  inverses `asin_unit`/`acos_unit`/`atan_unit`/`atan2_unit` (of `u·θ/(2π)`), on `Context`,
  `FBig` and `CachedFBig` (the `u` parameter is `usize`; the forward family takes
  `Err(OutOfDomain)` at `u = 0`, the inverse family returns the `u → 0` signed-zero limit).
  Unlike the radian variants, the argument reduces *exactly* mod u in integer arithmetic, so
  the accuracy is independent of the magnitude of the input; arguments where `12x/u` (resp.
  `8x/u` for the tangent) is an integer resolve exactly — quarters (`0`/`±1`), the sine/cosine
  sixths (`±1/2`), the tangent eighths (`±1`) and the tangent poles (`Err(Indeterminate)`,
  where the one-sided limits `+∞`/`−∞` disagree in sign). The inverse family resolves the
  axis/diagonal angles to exact `k·u/8` values (including the finite `y = ±x` diagonals, which
  an exactly-representable one-sided directed-rounding preimage would leave uncertifiable).
- ×π trigonometric functions `sin_pi`/`cos_pi`/`sin_cos_pi`/`tan_pi` (of `x·π`), on `Context`,
  `FBig` and `CachedFBig` — now thin wrappers over the ×u family at `u = 2`.
- ×π hyperbolic functions `sinh_pi`/`cosh_pi`/`sinh_cosh_pi` (of `x·π`), on `Context`, `FBig` and
  `CachedFBig` — the argument ball is built from the shared cached π; a direct series handles
  `|πx| ≤ 1`, the exponential composition the rest.

### Change
- **(internal, `tuning`) the per-attempt Ziv radius trace is now a settable hook** instead of an
  unconditional `eprintln!`: `ziv_set_trace_hook(Some(|guard, radius| …))` (deliberately
  `#[doc(hidden)]` — a profiling hook, not a stable API promise) installs a printer, and the
  default `None` costs one `Cell` read per attempt. The unconditional print cost ~3–4 µs per
  attempt, which silently poisoned any *timing* run built with `tuning` (reading as a 4–5×
  regression on sub-microsecond Ziv cases).
- **`-0` renders with its sign**, as `f64`'s does: `format!("{}", -0)` is now `"-0"` (was `"0"`)
  and `format!("{:e}", -0)` is `"-0e0"` (was `"0e0"`). The parse side moves with it — `"-0"`,
  `"-0.0"` and `"-0e0"` now produce *negative* zero (all three produced `+0`) — so a signed zero
  survives its own `Display`/`FromStr` round-trip. Neither direction could express the sign
  before, which is why the pair is fixed together. Nothing numeric changes (`±0` compare equal)
  and `{:+}` still prints `"+0"` for the positive zero.
- **(internal) the Ziv error radius is now a value-space `Mag` instead of an exact-integer
  ulp count** (`float/src/mag.rs`, `float/src/ball.rs`; both `pub(crate)`). Every `+`/`-`/`*`/`/`
  in a transcendental's algorithm is itself correctly rounded, so the radius composes through
  plain interval algebra over the operation's operands — no ulp-domain shift formulas, no
  per-ball precision state, and no `IBig` error bookkeeping. Results are unchanged for every
  input (validated bit-exact against MPFR/MPC); this is the mechanism the 0.6.0 note below
  described as "an exact-integer error count", replaced.
- **(internal) the operand error of the *input* to a transcendental is now part of the
  certified radius** rather than a per-function hand estimate, so a result that sits close to
  a rounding boundary is retried instead of being rounded the wrong way. See the two `Fix`
  entries below, which are the user-visible symptoms of the old per-function estimates.
- **(internal) faster digit bookkeeping in division**: `digit_len` on a power-of-two base is
  now a plain bit length (`ilog` allocated a `B^log` buffer only to discard it). No perf
  regression from the fixes below: `FBig / FBig` at parity or faster at all benched
  precisions, `DBig / DBig` ~10% faster at 10³–10⁴ digits (+8% at 10 digits, the cost of the
  now-correct wide-quotient rounding), `nth_root` 10–45% faster.

### Fix
- **32-bit `Word` targets returned a wrong `log₂ BASE` from the fixed-point walk** (the
  compile-time bracket behind the generic-base radius rules): the normalization shifted with
  `Word::leading_zeros`, so on `Word = u32` (e.g. the `i686` CI target) `log2(10)` came out
  ≈ 35 instead of ≈ 3.32 — radii then inflated by orders of magnitude, saturated to infinity
  inside the `atan` reduction and panicked the Ziv containment test (`atan2` of any
  non-unit-scaled pair). The walk is now pinned to explicit widths; a value-pinning test
  guards it.
- **The MSRV (1.68) workspace check failed on a std-only `f32::abs`** in `exp`'s
  overflow probe (that check compiles `dashu-float` without `std`; the inherent method is
  core-only from 1.85) — it now goes through the `Abs` trait, which is no_std-safe.
- **`no_std` test builds failed to compile**: the Ziv retry-count regression tests read the
  `thread_local` counter that only exists under `std`; those tests are now
  `#[cfg(feature = "std")]`, matching the `ziv.rs` test module's own gate.
- **Performance regressions of the Mag/Ball migration**: the
  two-stage `ln` reduction fired its cancellation double-precision on inputs that reduce to
  *exactly* a power of two (`ln(1e100)` in base 10 was 9× slower than the pre-migration
  code at 600 digits) — the cancellation condition now reads off the rounded input directly
  (one exact comparison) instead of the split exponents, and the power-two floor is settled
  by an exact comparison at the boundary. The generic-base radius rules (`scale_by_base_pow`
  and `to_repr`'s base-power export) no longer build `BASE^|e|` bigints per call — a
  compile-time fixed-point `log₂ BASE` bracket replaces them (one-bit tight, sound on either
  side), which was worth 1.5–3× on base-3 `exp` at high precision. `asinh` squares through
  the dedicated `sqr` kernel again, and the base-10 radius export keeps its significand
  instead of collapsing to a bare power of ten (≤ ~1× slack instead of ≤ 10×).
- **`exp`-family results near a rounding boundary paid one systematic Ziv retry** (~2.25×)
  on non-power-of-two bases: the `Bⁿ` powering chain compounds the per-op radius slack past
  the first attempt's preimage. The chain length is now charged to the initial guard
  (`pow_chain_guard`: `n` for `B ≠ 2` on `exp`/`exp_m1` and the hyperbolic family, `2n`
  unconditionally on `powf` — the base-2 `powf` margins are thin enough to need it). All 20
  measured regressions across the sweep are eliminated (18 of the 20 now at or below
  master's own timings; results bit-identical — correct rounding is unique), at the cost of
  a few guard digits on the affected functions.
- **`ln` of a large base power paid one Ziv retry near a rounding boundary**: the
  reconstruction constants (`s2·ln2`, `e_base·lnB`) evaluated at the bare work precision, so
  their 8-ulp radii were amplified by the scale factor (`ln(1e100)` @6 digits carried ~1/3
  of the target half-ulp before the candidate margin). The constants now evaluate with
  `⌈log_B|scale|⌉ + 1` extra digits — the same construction `exp` already uses for its
  `ln(B)` — keeping the amplified radius sub-ulp at a cost confined to inputs with large
  scale factors.
- **`sqrt` under directed modes** (#99): a sticky remainder was rounded *down* under
  `Up`/`Away` when the integer root carries `precision + 1` digits (`Up(sqrt(6))² < 6` at
  p53) — the final rounding now consults the rounding mode instead of hardcoding half-up.
- **`div` no longer pre-rounds an over-wide dividend** (#100): the kernel bounds it by an
  exact digit split instead, keeping the dropped digits as sticky rounding information.
  The old pre-round could report a false `Exact` (`5/1` @ p1), invert the direction
  (`7/-1` under `Up` gave `-8` instead of `-4`), or land on a false midpoint (`31/4`
  base 3 `HalfEven` gave `6` instead of `9`).
- **`div` now rounds exact and `precision+1`-digit quotients in a single step** (#100):
  an exactly-dividing quotient was returned unreduced (`15/3` @ p2 gave the 3-digit `5`
  instead of `4`), and an over-wide quotient was rounded on the integer instead of the
  precision grid (`3/5` @ p2 gave the unrepresentable `0.625` instead of `0.5`).
- **`div` with a negative divisor and an over-wide dividend** carried the sticky low part
  with the wrong sign (the divisor-sign normalization negates both operands), and the
  `precision+1`-digit quotient's half comparison sign-flipped `B − 2·ql` where only `|ql|`
  enters — the latter mis-rounded negative dividends in non-binary bases under nearest
  modes (`-21/2` base 10 @ p1 `HalfEven` gave `-20` instead of `-10`).
- **`nth_root` no longer double-rounds a `precision+1`-digit root** (#100): the exponent
  alignment now takes the truncating shift when the padding one would grow the root past
  the precision, so one rounding decides (`nth_root(2, 1.75)` @ p2 `HalfEven` gave `1.0`
  instead of `1.5`).
- **`with_base` rounds its exact-conversion shortcuts** (#100): the power-of-base shortcuts
  and small-exponent path returned significands wider than the target precision, and the
  division path pre-rounded its dividend, so the `Exact` flag could be false (`2.1` →
  base 2 reported `Exact`).
- `exp` — and every transcendental built on it, e.g. `sinh`/`cosh` — of an argument whose
  exponent is in the 10^14 range no longer dies on an out-of-memory allocation. The base-aware
  radius export converted a binary exponent to a decimal one (and back) through a small
  rational bound (`28/93` and `30102/100000` for log₁₀2, `3322/1000` and `33218/10000` for
  log₂10). Those err by ~5·10⁻⁵ *relative*, and that error multiplies the exponent: at 10^14 it
  overshot the outward power of ten by `10^1.2e10`, leaving the Ziv error radius astronomically
  larger than the value it bounded — the containment test then tried to align that gap and ran
  out of memory. The conversions now use 64- and 62-bit fixed-point bounds, so the outward
  slack stays under one digit for every `isize` exponent.
- Additions/subtractions of operands with an astronomically large exponent gap (~10⁹+ digits,
  e.g. the two exponentials composing `sinh(1e14)`) no longer die on an out-of-memory
  allocation: the sticky low part of the aligned sum is collapsed to a bounded position when
  it sits entirely below the rounding window (`repr_round_sum`), and the hyperbolic
  `sinh`/`cosh`/`sinh_cosh` compositions drop an exponential that sits below the other's ulp
  window instead of aligning the gap.
- `sqrt` of a perfect square in a non-power-of-two base never returned under the directed
  rounding modes (`Down`/`Up`/`Zero`): it kept doubling its working precision until the retry
  budget was exhausted — `sqrt(4)` in base 10 escalated past 10^8 digits instead of returning
  `2`. `sqrt` was the one transcendental still deriving its Ziv radius by hand (a blanket
  `value.ulp()`), and no nonzero radius fits a one-sided directed preimage (`Down`'s
  `[y, y+ulp)` cannot contain `[y−r, y+r]`), so an exactly-representable root could not be
  certified at all. It now wraps its kernel result as a `Ball`, so an exact root carries
  radius 0 and certifies immediately; an inexact root keeps the same one-ulp bound, and the
  base-2 fast path is untouched.
- `log2`/`ln` of a value a hair above 1 at low precision could return exactly `0`: the Ziv
  containment test accepted a zero candidate whose radius was nonzero, but no nonzero real
  rounds to exactly zero (the documented ±ulp preimage of ±0 is a special case). A zero
  candidate is now certified only with a zero radius. Scoped to this crate's driver — the
  public `ErrorBounds` semantics are unchanged.
- `x + (-0)`, `x - (-0)` and their mirrors returned `0` instead of `x` whenever the exponent
  gap to `-0`'s sentinel exponent (`-1`) reached past the end of `x`'s significand — e.g.
  `1e-16 + (-0)` was `0` at precision 12. The `addsub_*` kernels short-circuited only on
  `+0`, so `-0` fell through to the alignment path, where its sentinel exponent makes it look
  like the *larger* operand and `x`'s significand is shifted out entirely.
- A sum or difference of two *zero* operands now follows IEEE 754 §6.3: `(-0) + (-0)` and
  `(-0) - (+0)` are `-0` (`x + x` retains `x`'s sign, even when `x` is zero); a zero of mixed
  signs is `+0`, or `-0` under roundTowardNegative, as the exact zero of a cancellation
  already was.
- `powf` (and the `powi` fallback for exponents past the squaring chain) no longer stalls
  when the exponentiation drives the `exp` argument far negative — e.g.
  `powf(7.03e71, -84.91)`, whose `y·ln x ≈ −14035` makes the result ≈ `1e-6096`. The
  internal `exp` input-error fold omitted the result's magnitude for a negative argument,
  over-estimating the error radius by `e^{−x}` (hundreds of digits); the Ziv loop could only
  certify by growing its working precision past `|x|/log_B e`, burning 9 retries and ~6000
  digits: 1.5 s at 16 digits instead of 60 µs (and 6.5 s at 151). The fold now scales by the
  result's magnitude, so such calls certify on the first attempt.
- `ln` (and `log2`/`log10`/`ln_1p`, which share its core) of an argument with a huge
  exponent (e.g. `ln(1e1000000000)`) previously never returned, holding gigabytes: the
  argument reduction materialized a power of two spanning the whole exponent *gap*. The
  reduction now splits the magnitude by an exact base-exponent re-tag plus a bounded
  power-of-two finish, so no power of the gap is ever materialized, and the reduction
  arithmetic is exact integer work end to end (the previous single f32 `log2` estimate
  loses hundreds of bits of accuracy once `|log2 x|` approaches 10^9). (issue #103)
- The hyperbolic functions (`sinh`, `cosh`, `sinh_cosh`, `tanh`, `asinh`, `acosh`) now
  fold the *input's own rounding* into the certified error radius. Previously the input
  was rounded to the working precision and the rounding error dropped, which understated
  the radius by the ulp of the *original* magnitude — fatal for `acosh` near 1, where the
  `x−1` cancellation shrinks the value (and its ulp) while the inherited error does not:
  `acosh` could certify the wrong neighbour of a decimal tie (issue #102). All six now
  thread the input ball (or pass the raw repr into `exp_compute`/`ln_compute`, which fold
  it themselves).
- `ConstCache::pi` (and `Context::pi`) now compute one extra Chudnovsky series term,
  fixing an off-by-1-ulp mis-rounding at certain precisions (previously the term count
  provided only the ceiling — no accuracy headroom — so the series truncation error
  could push the result to the wrong side of a rounding boundary).
- `FBig::with_precision` (and `CachedFBig::with_precision`) now rounds an *unlimited*-
  precision source down to a finite target precision. Previously the shrink guard
  compared context precisions (`0 > N` is always false), so an unlimited value kept its
  full significand and the result violated the precision invariant.
- **(internal) `Ball`'s square rule folded only one `|mid|·rad` cross term** where
  `(m ± r)² = m² ± 2mr + r²` needs two — an under-bound by exactly `‖m‖·r` (the doc
  comment already stated the `2·`; the code did not). Only `asinh` squares a
  nonzero-radius ball through the dedicated `sqr` kernel, so the blast radius was its
  input-error term; the corner-coverage regression pins both `(m ± r)²` extremes.
- `hypot` of two operands past the extreme-exponent scale-down threshold returned
  `Err(Overflow)` whenever the result was not exactly representable (e.g.
  `hypot(7·2^e, 4·2^e) = √65·2^e` at `e ≈ isize::MAX/2`). The rescale factor `k` was
  derived from the *larger* operand's raw exponent only, but base-B normalization can
  leave the smaller value with the larger raw exponent (`4·2^e` normalizes to `1·2^(e+2)`)
  — the smaller operand's square then collided with the infinity sentinel. `k` now scales
  by the larger of the two raw exponents.
- `tan_pi` (and `tan_unit` at any `u`) returned `-0` at the *positive* odd multiples of
  `u/2` unconditionally, breaking oddness (`tan_pi(-1) == tan_pi(1) == -0`) and diverging
  from MPFR (`+0`). Every integer zero of the tangent now carries the sign of the input,
  like the even-multiple row already did.
- `asin_unit(x, 0)`/`acos_unit(x, 0)` returned the `u → 0` limit `±0` for out-of-domain
  `|x| > 1` instead of `Err(OutOfDomain)`. The domain error now outranks the limit (the
  limit of a function undefined at that `x` for every `u > 0` is not `+0`).
- `Sum` of `FBig` lost the sign of an all-`-0` sum: `[-x.zero].sum()` and
  `[-0, -0].sum()` returned `+0` under the nearest modes, while the chained `(-0) + (-0)`
  returns `-0`. The exact accumulator's zero sign now follows the same IEEE 754 §6.3
  left-fold rule as the chained operators (`-0` under roundTowardNegative or when every
  addend is `-0`).
- **(internal) the ×u reduction's f32 magnitude guards are now one-sided-conservative**:
  the bound products `u_lb + s·b_lb` round in f32, and an *upward* rounding of the
  `k = 0` fast-path test could in principle classify a `k ≥ 1` argument as first-quadrant
  (a wrong result; reachable only from gigabit-scale inputs, but a wrongness, not a
  slowdown). The shared bound is shaved by its own f32 slack (`log2_u_bs_lb`), so any
  error falls towards the always-exact general path. The sticky-collapse guard in
  `repr_round_sum` likewise used the upper bound of `log₂B` where the derivation needs
  the lower one, and now carries an explicit margin for the f32 products.
- **(internal) `CachedFBig` mirrors the full ×u family** (`sin_unit`/`cos_unit`/
  `tan_unit`/`asin_unit`/`acos_unit`/`atan_unit`), restoring the drop-in rule that code
  compiling against `FBig` compiles unchanged against `CachedFBig` (only the fused
  `sin_cos_unit`/`atan2_unit` had been mirrored; the forwarding macro cannot pass the
  `u` argument, so these are hand-written wrappers).

## 0.6.0

### Change
- **(breaking) Ziv-backed transcendentals now require `R: ErrorBounds` (not `R: Round`)**: `exp`/
  `exp_m1`/`ln`/`ln_1p`/`log2`/`log10`, `powf`/`powi`, `hypot`, the trig and hyperbolic families
  (incl. inverses), and `FBig::sqrt`. All six built-in modes satisfy `ErrorBounds`; only custom
  non-`ErrorBounds` modes are affected.
- **(breaking) new `FpError::ZivRetryLimitExceeded`** — a Ziv loop that exhausts its retry budget
  (only possible if a radius-bound estimate is wrong) now returns `Err` instead of silently returning
  a possibly-1-ULP-wrong best-effort value.
- **(internal) every transcendental's error radius is now derived mechanically by Ball arithmetic**
  (`float/src/ball.rs`, an exact-integer error count composed through the series/reduction/composition,
  precision-aware) instead of the hand-derived `·ulp` formulas and outward intervals. Behavior
  unchanged, validated bit-exact against MPFR.
- **(internal) `Context::exp` of an astronomically large negative value now returns `Err(Underflow)`**
  (consistent with overflow; the convenience layer is unchanged).
- Removed the `rustversion` dependency (MSRV is 1.68); the unversioned `rand` / `rkyv` feature aliases
  now select the newest versions (`rand_v010` / `rkyv_v08`).

### Add
- **Correctly rounded transcendentals via a Ziv retry loop** — `exp`/`exp_m1`/`ln`/`ln_1p`, `log2`,
  `log10`, `powf`/`powi`, `hypot`, and the trig/hyperbolic families (incl. inverses) certify their
  rounding against the `ErrorBounds` preimage, retrying with more guard digits; exact results report
  radius 0 (MPFR-style `exact` tracking) so directed modes terminate.
- **Base-10 `log10`** (mirrors `log2`, with an exact power-of-ten shortcut).
- **Exact `Add`/`Sub`/`Mul` operators for `Repr`** (new `repr_ops` module) — lossless exact
  intermediates for the Ziv containment test, the correctly-rounded `Sum`, and the `FBig` multiply
  path.
- **`FBig::sqrt` is now correctly rounded** — a `p+1`-digit integer root was previously double-rounded
  (off by 1 ulp at digit boundaries); it now rounds in a single step from the round digit + `sqrtrem`
  remainder (MPFR-style). Power-of-two bases use a fast path; other bases certify via Ziv.
- **`CachedFBig` mirrors the rest of `FBig`'s value surface** (`round`/`trunc`/…/`quantize`,
  `hypot`/`nth_root`, `signum`/`ulp_lb`, base conversions), completing the drop-in mandate.
- `tuning` feature + `ziv_retries()`/`ziv_retries_reset()` for profiling retry counts;
  `Repr::zero_with_sign` public; rkyv 0.7 support (`rkyv_v07`).

### Fix
- **Mode-aware overflow/underflow saturation** — `Err(Overflow)`/`Err(Underflow)` and the
  `FBig → f32`/`f64` conversions now saturate to the directed endpoint per rounding mode (`±∞` /
  largest finite / smallest / `±0`) instead of mode-blind `±∞`/`0`; the `TryFrom` error variant is
  mode-independent.
- **`exp` range reduction for large `|x|`** — `ln B` now carries `⌈log_B|x|⌉+2` extra digits so the
  reduction quotient `s` is pinned (was off by ~`|x|`); the 32-bit `isize` threshold is fixed.
- **Directed `ln`/`log2` near `x ∈ [1, B)`** — the radius now also covers the pre-cancellation `sum`
  scale and the over-delivered `ln_base` context.
- **`FBig → f32`/`f64` subnormal/underflow** — round-to-odd at `width + 24` bits; a
  source-`log2_bounds` short-circuit for catastrophically tiny values.
- `ulp()`/`Repr::cmp` near the exponent ceiling (saturating arithmetic); the `no_std` build (the
  test-only Ziv counter is gated on `std`); `tan` pole check removed (~2× faster).

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
