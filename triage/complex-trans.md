# complex-trans — dashu-cmplx transcendental functions

31 reports, 16 clusters. Every report was reproduced at `a0fc535` (for E238, E232, E216 and
the wrong `Exact` tags the root cause was traced before fixing). All 31 are **real, fixed**;
nothing is left `#[ignore]`d.

Several clusters share root causes in the complex Ziv driver / ball layer, so the fixes are
not separable per cluster: they landed as one `float/` commit (`ErrorBounds`, the one
float-side root cause fixed here) and two `complex/` commits (everything else with the tests,
then the bound on the driver's flag proof — the second depends on the first, and the complex
tests for E157/E216 depend on the float commit).
See *Cross-group notes*.

All tests named below were run and pass; `cargo test -p dashu-float -p dashu-cmplx
-p dashu-ratio -p dashu -p dashu-macros --all-features`, `cargo fmt --all -- --check` and the
workspace clippy line from `AGENTS.md` are clean. The `fuzz/` MPFR/MPC differentials were
**not** run (MPFR/MPC are not installed here).

## Clusters

### 1. Exact/Inexact flags of complex results

Reports: E63, E157 (flag part), E172, E228
Verdict: real, fixed
Root cause: two independent defects. (a) `combine_parts` (`complex/src/repr.rs:143`)
reported `Exact` whenever both per-part flags were `NoOp`, but an `Inexact(_, NoOp)` part is a
*truncated* part, not an exact one — so `powi(16+i, -1)` at 1 bit, `√(8186i)` at 12 bits,
`exp` under `mode::Zero`, … came back `Exact`. (b) The Ziv driver
(`complex/src/ziv.rs`, `contained`) took each part's flag from rounding the
*approximation*; when the candidate lay inside the error interval (in particular when the
working value already equalled it) the flag described the approximation, not the true value —
`√(w²)` for a 13-digit `w` at 13 digits came back `Inexact(AddOne)`. Separately
(c) `tan_pi` on the real axis wrapped the real kernel's result in `exact(...)`, dropping its
flag (`complex/src/math/trig.rs`, old line 365).
Test: `complex/tests/root.rs::sqrt_of_perfect_square_is_exact`,
`::sqrt_of_non_square_is_inexact`; `complex/tests/exp.rs::powi_minus_one_flags_match_inv`;
`complex/tests/trig.rs::tan_pi_on_the_real_axis_keeps_the_inexact_flag`.
Fix: `combine_parts` is `Exact` iff both parts are `Approximation::Exact`. The driver now
accepts a part only if its radius is zero or the candidate lies strictly outside
`[value − radius, value + radius]` (otherwise it retries), so the flag is proven. That proof
is pursued up to a budget of `8·(precision + guard) + 2048` extra digits; a value still
indistinguishable from a representable number there is returned (nearest modes) with the
flag of its approximation, as before — see the notes. `tan_pi` keeps the kernel's flag via
`combine_parts`.
Notes: an in-tree unit test (`tan_tanh_large_imaginary_saturation`) asserted the old
collapsing behaviour, with a comment calling it intended ("the flag channel carries the
adjustment, not exactness"). That contradicts the documented contract of `CRounded`
("`Exact(v)` ⟺ both parts are exact") which the reports quote; the test was updated. The same
`combine_parts` defect is behind complex-arith's E83/E94/E118/D24 (see Cross-group notes).

### 2. Misrounding next to a power of the base; `HalfEven` tie parity (`ErrorBounds`)

Reports: E157 (value part), E216
Verdict: real, fixed (root cause in `float/`)
Root cause: `ErrorBounds::error_bounds` (`float/src/round.rs`) returned a symmetric
(half-)ulp on both sides of every value. Below a power of the base the neighbour is `B` times
closer, so the claimed preimage of `64` at 12 bits reached down to `63.984…` instead of the
midpoint `63.9921875`: `√(8190i)` (true root `63.99218702…·(1+i)`) was certified as `64+64i`,
and `Im acos(i·255·2⁻¹⁹)` at 7 bits as `−2⁻¹¹`. Also, the `HalfEven` inclusiveness test read
bit 0 of the *normalized* significand (always 1 in base 2), i.e. the parity test was inverted
for full-width significands — in base 10 an even full-width candidate that *is* the image of
an exact tie could never be certified.
Test: `float/tests/round.rs::test_error_bounds_are_the_rounding_preimage` (checks the defining
property on both endpoints for all six modes, bases 2 and 10, precisions 1…500);
`complex/tests/root.rs::sqrt_just_below_midpoint_under_power_of_base`,
`::sqrt_matches_oracle_in_all_modes`;
`complex/tests/trig.rs::asin_acos_on_the_imaginary_axis_and_for_large_imaginary_parts`.
Fix: `error_bounds` uses the finer gap on the toward-zero side of a power of the base (all
modes), and the `HalfEven` inclusive flags follow the parity of the full-precision
significand (with the power-of-base side decided by the lower neighbour `B^p − 1`).

### 3. Unsound results from wrongly-`Exact` float kernels (`ln|z|` near 1, tiny `asin`, `tan_pi` near a pole)

Reports: E238, E232, E117
Verdict: real, fixed in the complex layer (the float defect itself is float-exp **D40** and
the float-trig "Exact for a rounded result" family, left to those groups)
Root cause: the float kernels can return `Exact` for a rounded result (when their internal
working approximation lands exactly on the returned value — D40). The complex ball layer
trusted that flag (`Ball::from_rounded`: `Exact` ⇒ radius 0), so its radius became unsound.
E238: at working precision 60 `hypot(1, y)` returned `Exact(1 + 3·2⁻⁵⁹)`, and `ln` of it was
then certified with ~20 good bits. E117: `cosh_pi(2⁻⁴³)` returned `Exact(1)`, so the
denominator `cos_pi(1) + cosh` became an *exact* zero and `0/0` surfaced as `Indeterminate`.
E232: `hypot`/`sqrt` `Exact` flags inside `asin`.
Test: `complex/tests/log.rs::log_near_unit_circle_is_correctly_rounded`;
`complex/tests/trig.rs::asin_of_small_decimal_argument`, `::tan_pi_next_to_a_pole`.
Fix: `kernel_ball` (`complex/src/ball.rs`) replaces `Ball::from_rounded` at every kernel call
in the crate: a kernel's `Exact` is believed only where the result *can* be exact, by a
cheap mathematical criterion at the call site — `exp`/`sin`/`cos`/`sinh`/`cosh` only at 0,
`ln` only at 1, `atan2` only on the positive real axis, `acos` only at 1, the ×π pair only
for `6x ∈ ℤ` (Niven), and `hypot` / `sqrt` / real `powi` by exact verification of the claimed
value (`mid² = a² + b²`, …). Otherwise the result gets the one-ulp radius of a rounded value.
Notes: with this the complex layer is sound whether or not the float flags are fixed. (I
first fixed the float driver instead — accept only when the flag is proven — and reverted
it: it makes `FBig::powf(4, 0.5)` under the *default* mode hang until float-exp's E48 fix
exists, and tiny arguments slow under the nearest modes. See Cross-group notes.)

### 4. `asin` / `acos` on the axes and branch cuts

Reports: D38, E53, E133, E156, E245, E152, E201
Verdict: real, fixed
Root cause: `asin`/`acos` (`complex/src/math/trig.rs`) had no axis dispatch and used the Kahan
log form everywhere. (i) For a real argument in `[−1, 1]` the imaginary part is exactly zero,
which a nonzero radius can never certify → endless retry (D38, E53). (ii) The log form cannot
see the sign of a zero part, so both sides of each cut, and all four signed zeros, gave the
same result (E133, E245, E156 — `acosh(1.5+0i)` negative). (iii) For `Im z > 0` large,
`iz + √(1−z²)` cancels to an exact zero and `ln 0` surfaced as `Err(OutOfDomain)` (E201).
(iv) An argument wider than the working precision was rounded onto the branch point 1, and
`sqrt_real` (`complex/src/ball.rs`) turned a zero *midpoint with a nonzero radius* into an
exact zero, giving `0/0 = Indeterminate` (E152).
Test: `complex/tests/trig.rs::asin_acos_of_real_argument_inside_the_cut`,
`::asin_acos_of_signed_zero`, `::asin_acos_on_the_branch_cuts`,
`::asin_acos_on_the_imaginary_axis_and_for_large_imaginary_parts`,
`::asin_acos_of_wide_argument_next_to_one`; `complex/tests/hyper.rs::asinh_on_the_imaginary_axis`,
`::acosh_of_real_argument`.
Fix: axis dispatch to the real kernels with exact signed zeros (C99 Annex G: `casin(x±0i) =
asin x ± 0i` / `±π/2 ± i·acosh|x|`, `casin(±0+iy) = ±0 + i·asinh y`, `cacos` likewise with the
conjugate convention). Off the axes the log form is evaluated in the half plane where its sum
cannot cancel (`asin(−z) = −asin z`, `acos(conj z) = conj(acos z)`, applied to the
working-precision ball). Inputs are seeded exactly (cluster 9); `sqrt_real` exports an
infinite radius for an inexact zero; `CBall::log` exports an infinite radius (retry) when
both midpoints cancelled to zero with a nonzero radius instead of calling `ln 0`.

### 5. `atan` on the imaginary-axis cuts

Reports: E161
Verdict: real, fixed
Root cause: for `±0 + iy`, `|y| > 1`, `atan` fell through to the two-log form, whose real
part `±π/2` takes its sign from `y`, not from the real zero.
Test: `complex/tests/trig.rs::atan_on_the_imaginary_axis_cuts`;
`complex/tests/hyper.rs::atanh_on_the_real_axis_cuts`.
Fix: axis branch `atan(±0 + iy) = ±π/2 + i·atanh(1/y)` (real part with the sign of the zero,
correctly rounded `π/2`; `atanh(1/y)` through a single-part Ziv loop with the reciprocal's
rounding folded in).

### 6. Hyperbolic functions round the wrong way under `Down`/`Up`

Reports: E41
Verdict: real, fixed
Root cause: `complex/src/math/hyper.rs` built `sinh z = −i·sin(iz)` etc. by negating one part
of an *already rounded* result (`rotate_neg_i`); under `Down`/`Up` a negated part is rounded
in the opposite direction (and its flag was then flipped to match the wrong value). Affected
`sinh`, `sinh_cosh`, `tanh`, `asinh`, `acosh`, `atanh`.
Test: `complex/tests/hyper.rs::sinh_of_imaginary_unit_rounds_in_the_mode_direction`,
`::hyperbolic_functions_directed_match_oracle`,
`::hyperbolic_functions_are_rotations_of_the_circular_ones`.
Fix: the odd functions are now `S∘f∘S` with `S(x+iy) = y+ix` (a pure part swap, no negation:
`sinh(x+iy)` is `sin(y+ix)` with its parts swapped). Where a negation is unavoidable
(`sinh_cosh`'s shared evaluation, `acosh` below the real axis) it is applied to the
working-precision value before the final rounding (`sin_cos_parts(.., negate)`,
`acos_parts(.., negate_re)`). Guide text updated in `guide/` and `guide-zh/`.

### 7. `log(−0 − 0i)`: `−π` rounded in the wrong direction

Reports: E114
Verdict: real, fixed
Root cause: `complex/src/log.rs` computed `+π` under the mode and negated it (flipping the
flag), which reverses the direction under `Down`/`Up`.
Test: `complex/tests/log.rs::log_of_negative_zero_rounds_pi_in_the_mode_direction`.
Fix: `Context::round_pi_multiple` (in `complex/src/ziv.rs`) rounds `±π` / `±π/2` once, after
applying the sign. Also used by the `asin`/`acos`/`atan` cut branches.

### 8. Saturated `tan` / `tanh` on the wrong side of ±1

Reports: E127
Verdict: real, fixed
Root cause: the saturation gate in `tan` assumed `|Im tan z| < 1` and pinned the endpoint with
float's `near_one_endpoint`. But `|Im tan z| − 1` has the sign of `−(cos 2x + e^(−2|y|))`: for
`cos 2x < 0` the value is just *above* 1.
Test: `complex/tests/trig.rs::tan_saturated_imaginary_part_lies_on_the_right_side_of_one`.
Fix: `tan_saturated_side` decides the sign of `cos 2x·e^t + 1` on balls (retrying until the
ball excludes zero), and `near_unit_endpoint` rounds a representative `±(1 ± B^−(p+2))` under
the mode — which also yields the consistent inexactness flag (the old endpoint reported
`NoOp` where `AddOne` was meant; cf. float-trig E224).

### 9. Arguments wider than the working precision were rounded first (`exp` out of memory)

Reports: E96 (and part of E63, E152)
Verdict: real, fixed
Root cause: `CBall::from_parts` (`complex/src/ball.rs`) re-rounded each input part to the
working precision and carried the rounding as an input radius. For `x = 278130·2³²` at 15
working bits that radius is ~2³⁵, and `exp`'s fold turns it into an error bound
`e^(2³⁵)` times the result; the containment test then aligned the two bit for bit (34 GB /
34 s here, "out of memory" in the report). It also made exactly-representable results of wide
inputs uncertifiable until the guard outgrew the input, and pushed inputs next to a branch
point onto it.
Test: `complex/tests/exp.rs::exp_of_wide_large_real_at_low_precision`;
`complex/src/ball.rs::tests::from_parts_is_exact_at_any_width`.
Fix: inputs are seeded exactly (the operations round their own results). As a backstop,
`contained` rejects a radius whose exponent is above the candidate's leading digit before
doing any exact arithmetic.

### 10. `exp(0)`, `powi(z, 0)`, `powf(z, 0)` return precision 0; `exp(+0 − 0i)` loses the zero sign

Reports: E103, E236, E264
Verdict: real, fixed (same shape as float-exp D17/E61/E221, but the complex code is separate)
Root cause: `complex/src/exp.rs` returned the unlimited-precision constants `CBig::ONE` /
`FBig::ONE`, `FBig::ZERO` (precision 0, `+0`).
Test: `complex/tests/exp.rs::exp_of_zero_keeps_precision_and_zero_sign`,
`::zeroth_power_keeps_precision`.
Fix: the results are built at the context's precision; `exp(±0 ± 0i) = 1 ± 0i` keeps the sign
of the imaginary zero (`exp(conj z) = conj(exp z)`). `exp(−∞)`'s zero got the context
precision too.

### 11. `powi` on an unlimited-precision value panics

Reports: E113
Verdict: real, fixed
Root cause: every nonzero exponent went through the Ziv driver, which rejects precision 0.
Test: `complex/tests/exp.rs::powi_at_unlimited_precision_is_exact`,
`::powi_negative_at_unlimited_precision_panics`.
Fix: a positive exponent at unlimited precision runs the squaring chain on exact products
(like `z * z`, and like `FBig::powi`); a negative exponent still panics (documented).

### 12. `powi` of an anti-diagonal base never returns

Reports: E56
Verdict: real, fixed
Root cause: the exact-lattice shortcut for `t·(1+i)` only tested `re == im`. For `t·(1−i)`
one part of an even power is also exactly zero, which the generic chain cannot certify once
it is inexact (`(3−3i)⁻²`).
Test: `complex/tests/exp.rs::powi_of_anti_diagonal_base`.
Fix: the shortcut also takes `re == −im`, using the conjugate lattice point.

### 13. `powi` range-error sign for a negative real base

Reports: E148
Verdict: real, fixed
Root cause: the overflow/underflow error propagated from whichever intermediate square left
the exponent range, so it was always `Positive`.
Test: `complex/tests/exp.rs::powi_real_base_range_error_sign`.
Fix: for a real base the error is re-signed with the sign of the true result.

### 14. `powf`: zero base, exact and tie results

Reports: E62, E43, E231
Verdict: real, fixed
Root cause: `powf` always evaluated `exp(w·log z)`. `log 0` errors (E62). A result with an
exactly-zero part (`i¹`, `(−1)^½`) or one that is exactly representable or a tie
(`(28+64i)⁵` at 20 bits) can never be certified through a transcendental composition with a
nonzero radius → wrong tiny real part under the old lax zero preimage / endless retry (E43,
E231).
Test: `complex/tests/exp.rs::powf_of_zero_base`, `::powf_with_integer_exponent_is_powi`,
`::powf_with_dyadic_exponent`, `::powf_general_matches_oracle`.
Fix: `0^w = 0` for `Re w > 0`, `Err(Indeterminate)` for a zero base otherwise (as
`powi(0, −1)`). A real exponent `m/2ᵏ` (`k ≤ 64`, `m` ≤ 256 bits; recognized in any base,
e.g. decimal `0.5`) is evaluated as `(z^(1/2ᵏ))^m` — `powi` for `k = 0`, otherwise `k` ball
square roots plus the integer chain — which is exact whenever the result is.
Notes: a result that is exact for a *non-dyadic* real exponent (only possible in a non-binary
base, e.g. `32^0.2 = 2` in base 10) is not recognized as exact: under the nearest modes it is
returned correctly after the driver's flag-proof budget (pinned by
`complex/tests/exp.rs::powf_exact_non_dyadic_result_returns`), under the directed modes it
still does not return — as before, and as for the real `powf` (float-exp D23/E48 family).

### 15. `tan_pi` slow for a large imaginary part under directed modes

Reports: E89
Verdict: real, fixed
Root cause: `tan_pi` had no saturation gate (only the radian `tan` did), so under a directed
mode the Ziv loop climbed to ~2π|y|/ln 2 working bits.
Test: `complex/tests/trig.rs::tan_pi_with_large_imaginary_part` (|y| = 10⁵ at 20/50/100/500
bits under four modes; the whole test file runs in ~2 s in a debug build).
Fix: `tan` and `tan_pi` share one body (`tan_parts`), including the gate (threshold scaled
by π) and the side decision of cluster 8.

### 16. `sqrt(−x − 0i)` has a `−0` real part

Reports: E147
Verdict: real, fixed
Root cause: on the `x < 0` branch `CBall::sqrt` (`complex/src/ball.rs`) computed the real part
as `y/(2b)` with `b` carrying the sign of `y`; for `y = −0` the zero numerator kept its sign.
Test: `complex/tests/root.rs::sqrt_of_negative_real_with_signed_zero_imaginary`;
`complex/src/ball.rs::tests::sqrt_negative_real_with_negative_zero_imaginary`.
Fix: the real part is `|y|/(2|b|)`, never negative (also as a zero).

## Per-report table

| Report | Cluster | Verdict |
|---|---|---|
| D38 | 4. asin/acos on the axes and branch cuts | real, fixed |
| E41 | 6. hyperbolic functions under Down/Up | real, fixed |
| E43 | 14. powf: zero base, exact and tie results | real, fixed |
| E53 | 4. asin/acos on the axes and branch cuts | duplicate of D38 (real, fixed) |
| E56 | 12. powi of an anti-diagonal base | real, fixed |
| E62 | 14. powf: zero base, exact and tie results | real, fixed |
| E63 | 1. Exact/Inexact flags | real, fixed |
| E89 | 15. tan_pi slow for a large imaginary part | real, fixed |
| E96 | 9. wide arguments rounded first | real, fixed |
| E103 | 10. precision-0 results / exp zero sign | real, fixed |
| E113 | 11. powi at unlimited precision | real, fixed |
| E114 | 7. log(−0 − 0i) | real, fixed |
| E117 | 3. wrongly-Exact float kernels | real, fixed (float side: D40 family) |
| E127 | 8. saturated tan/tanh side | real, fixed |
| E133 | 4. asin/acos on the axes and branch cuts | real, fixed |
| E147 | 16. sqrt(−x − 0i) real part −0 | real, fixed |
| E148 | 13. powi range-error sign | real, fixed |
| E152 | 4. asin/acos on the axes and branch cuts | real, fixed |
| E156 | 4. asin/acos on the axes and branch cuts | real, fixed |
| E157 | 2. ErrorBounds at a power of the base (value); 1. flags | real, fixed |
| E161 | 5. atan on the imaginary-axis cuts | real, fixed |
| E172 | 1. Exact/Inexact flags | real, fixed |
| E201 | 4. asin/acos on the axes and branch cuts | real, fixed |
| E216 | 2. ErrorBounds at a power of the base | real, fixed |
| E228 | 1. Exact/Inexact flags | real, fixed |
| E231 | 14. powf: zero base, exact and tie results | real, fixed |
| E232 | 3. wrongly-Exact float kernels | real, fixed (float side: D40 family) |
| E236 | 10. precision-0 results / exp zero sign | real, fixed |
| E238 | 3. wrongly-Exact float kernels | real, fixed (float side: D40 family) |
| E245 | 4. asin/acos on the axes and branch cuts | real, fixed |
| E264 | 10. precision-0 results / exp zero sign | real, fixed |

## Changelog entries

`complex/CHANGELOG.md`, under `## Unreleased`:

```markdown
### Fix
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
```

`float/CHANGELOG.md`, under `## Unreleased` (a root cause fixed here on behalf of this group —
reconcile with the float-* groups' entries for E45 / E29 / D29 / E59):

```markdown
### Fix
- `ErrorBounds::error_bounds` returns the true rounding preimage next to a power of the base (the gap on the side facing zero is `B` times narrower), and the `mode::HalfEven` bounds are inclusive exactly when a tie rounds to the value; results just below a power of the base were mis-rounded by the transcendental functions, and an exact tie could not be certified in base 10.
```

## Cross-group notes

Files touched outside `complex/`:

- `float/src/round.rs` — `ErrorBounds::error_bounds` for all six modes (power-of-base
  asymmetry) and the `HalfEven` inclusive flags (tie parity). Needed for E157/E216. Overlaps
  with float-exp **E45** ("returns 1 instead of 0.999 … just below 0.9995" — the same
  power-of-base defect) and **E29**/**D29** (HalfEven tie certification in base 10 — the
  parity defect). It also touches the function float-convert **E59** (odd bases: the
  half-ulp is rounded up; **not** addressed here) and **E166** (`Down`/`Up` panic at
  unlimited precision; **not** addressed here) are about. Expect a textual conflict in this
  function at merge; `float/tests/round.rs::test_error_bounds_are_the_rounding_preimage`
  states the property any resolution must keep (it covers bases 2 and 10, not odd bases).
- `float/tests/round.rs` — the test for the change above.
- `guide/src/ops/trig_n_hyper.md`, `guide-zh/src/ops/trig_n_hyper.md` — one sentence each
  (the hyperbolic functions are evaluated as a part swap).

Not touched, but relevant to the float-* groups:

- `float/src/ziv.rs` has the same flag defect as the complex driver had (cluster 1b): the
  flag is read off the working approximation. I tried the same fix there (accept only if the
  radius is zero or the candidate is strictly outside the error interval): it fixes D40-style
  flags and the float suite stays green, **but** `Context::<HalfEven>::powf(4, 0.5)` then no
  longer returns (an exact result whose closure reports a nonzero radius — today that hangs
  only under the directed modes, float-exp D23/E48), and `exp(2⁻ⁿ)`-style tiny arguments need
  ~n working bits under the nearest modes as they already do under the directed ones
  (float-trig E86). So that rule needs the exactness detection of E48/E82/E75 and the
  tiny-argument shortcuts of E86 to land with it — or a bounded proof like the one the
  complex driver ended up with. I reverted it and made the complex layer independent of the
  float flags instead (cluster 3).
- The complex axis shortcuts return float kernel results directly (`asin x`, `acos x`,
  `asinh y`, `atanh y`, `atan x`, real-axis `tan_pi`), so a wrong *flag* from those kernels
  shows through unchanged (the value is unaffected).

Shared with complex-arith:

- `complex/src/repr.rs::combine_parts` (cluster 1a) is the root cause of complex-arith's
  **E83 / E94 / E118 / D24** as far as they are "all-`NoOp` collapsed to `Exact`". Fixed here
  because E172/E157/E228 need it; expect the same or a conflicting edit from that group.
- `complex/src/ziv.rs` and `complex/src/ball.rs` are shared infrastructure. `CBall::from_parts`
  no longer rounds its input (cluster 9); the complex Ziv driver's acceptance rule is
  stricter (cluster 1b, bounded by the flag-proof budget). Arithmetic ops that go through the driver should be re-run after the
  merge (the complex suite, including `arith_prop`, passes here).

Reports in this group that are another group's root cause surfacing: E238, E232, E117 (float
`Exact` flags: float-exp D40 family — defended against here, not fixed there); the value part
of E157 and E216 (float `error_bounds`: float-exp E45 family — fixed here). E103/E264 have
the same *shape* as float-exp D17/E61/E221 but separate code.

## Unsure

- Flag proof budget. A part that agrees with a representable value to n digits needs ~n
  working digits before its rounding direction is known. Proving that without bound made
  `powf(32, 0.2)` (base 10, exact, non-dyadic) hang under `HalfEven` and tiny arguments take
  seconds (`exp(2⁻¹⁰⁶·(1+i))`: 4.6 s), where the old code returned at once — so the driver
  stops proving after `8·(precision + guard) + 2048` extra digits and returns the (correctly
  rounded) value with the approximation's flag. Consequence: the flag is proven for every
  case in the reports and for arguments down to about `2⁻²⁰⁰⁰` at 53 bits, but
  `exp(2⁻¹⁰⁰⁰⁰·(1+i))` is still tagged `Exact` (as before), and `asin` of `|z| < ~2⁻⁸⁰⁰` can
  carry an unproven direction. Release-build timings at 53 bits, `HalfEven`:
  `exp(2⁻¹⁰⁰⁰⁰·(1+i))` 0.6 ms (15 µs before), `asin(2⁻¹⁰⁰⁰⁰·(1+i))` 0.17 ms (230 ms before);
  a typical mix (`exp`, `asin`, `tan`, `sqrt`, `log`, `powf`) 288 ms per 1000 rounds (339 ms
  before). Under the directed modes there is no such exit (unchanged behaviour): tiny
  arguments and unprovable exact results cost ~n digits / run to the retry cap.
- `combine_parts`: I followed the documented `CRounded` contract over an in-tree test comment
  that called the collapsing intentional.
- `powf(0, w)` for `Re w < 0` returns `Err(Indeterminate)` (consistent with complex
  `powi(0, −1)`), whereas the real `FBig::powf(0, −1)` returns `+∞`. The report only asked for
  `Re w > 0`.
- Not run: the `fuzz/` MPC differentials (no MPFR/MPC here). Correct rounding was checked
  against exact expectations from the reports, the real kernels on the axes, and
  higher-precision self-evaluation in all quadrants under `Down`/`Up`/`Zero`/`HalfEven`.
