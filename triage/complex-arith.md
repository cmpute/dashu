# complex-arith

27 reports, 20 clusters. All reproduced at `a0fc535` (the new regression tests were also run
against an unmodified checkout of the base commit: every one fails there, except the test that
pins intended behaviour for E191). All tests live in `complex/tests/`.

## Clusters

### mul/sqr real part wrong under cancellation
Reports: D24, E92
Verdict: real, fixed
Root cause: `Context::mul`/`sqr` (`complex/src/mul.rs:18-64` at the base commit) formed the
products at `precision + 10` guard digits and re-rounded, so a real part `xu − yv` that cancels
more than 10 digits lost everything (0 for an exact 1), and came back tagged `Exact`.
Test: `complex/tests/mul.rs::mul_and_sqr_real_part_cancellation_binary`,
`::mul_and_sqr_real_part_cancellation_decimal`, plus the exact-oracle properties
`::mul_family_is_correctly_rounded` and `::mul_cancelling_real_part_is_correctly_rounded`
(20/50/100/500 bits, five rounding modes).
Fix: new `sum_of_products` in `complex/src/mul.rs`: each result part is the exact sum of its
products, rounded once. `mul`, `sqr`, `fma` and `norm` all use it, so they are now correctly
rounded per component. An addend entirely below the rounding window of the larger ones is
replaced by a sticky unit, so a huge exponent gap between addends costs nothing (covered by
`::astronomically_separated_addends_round_like_the_exact_sum`).

### fma loses a fully cancelled result
Reports: E106
Verdict: real, fixed
Root cause: `Context::fma` (`complex/src/mul.rs:84-111`) rounded `z1·z2` to `precision + 10` digits
before adding `z3`, so it was not fused: a cancellation deeper than the guard returned 0.
Test: `complex/tests/mul.rs::fma_keeps_a_fully_cancelled_result` (+ the `fma` leg of
`::mul_family_is_correctly_rounded`).
Fix: `fma` is the three-term `sum_of_products` (`a·c − b·d ± e`), rounded once.

### Rounded results tagged Exact
Reports: E83, E94, E118
Verdict: real, fixed
Root cause: `combine_parts` (`complex/src/repr.rs:143-161`) decided exactness from the
adjustments (`NoOp` on both parts ⇒ `Exact`), but `Inexact(_, NoOp)` is a *truncated* part. The
`norm` variant (E94) additionally lost the flag in the guard-precision intermediate.
Test: `complex/tests/mul.rs::rounded_results_are_tagged_inexact` (mul, mul_real, sqr, norm, fma).
Fix: `combine_parts` now returns `Exact` only for two `Exact` variants; `norm` is a
`sum_of_products`. Note: this also changes the tag of transcendental results whose parts are
`Inexact(_, NoOp)` (complex-trans area) — the unit test
`math::trig::tests::tan_tanh_large_imaginary_saturation` had pinned the old collapse as a known
quirk and was updated.

### Directed rounding goes the wrong way (sqr, div)
Reports: E217, E246
Verdict: real, fixed
Root cause: double rounding — the guard-precision intermediate was rounded with the directed
mode and then re-rounded (`mul.rs:33-35`, and Smith's method in `div.rs:38-85`), which can land
on the wrong side of the exact value.
Test: `complex/tests/mul.rs::sqr_respects_directed_rounding`,
`complex/tests/div.rs::div_respects_directed_rounding`, and the directed legs of the properties.
Fix: single rounding of the exact value (see the mul and div clusters).

### Signed zero of a product depends on the operand order
Reports: E46
Verdict: real, fixed
Root cause: the imaginary part was `fma(y, u, x·v)` (`complex/src/mul.rs:58-62`); when both
products are zero the result took the sign of one product instead of the IEEE sum of the two
signed zeros (inferred from the code and the reported outputs; the float FMA was not traced).
Test: `complex/tests/mul.rs::mul_zero_parts_follow_ieee_zero_sums`.
Fix: `sum_of_products` applies IEEE 754 §6.3 to zero sums (like-signed zeros keep the sign,
otherwise `+0`, or `−0` under `Down`). Consequence: `sqr` of a signed zero has real part `−0`
under `mode::Down` (it is `(+0) − (+0)`); the existing `HalfEven`/`HalfAway` tests are unchanged.

### z / z is not exactly 1
Reports: D33, E52
Verdict: real, fixed
Root cause: Smith's method (`complex/src/div.rs:38-85`) rounds the ratio `r = v/u` first, so the
numerator `y − r·x` does not cancel exactly; the tiny residue was returned (sometimes `Exact`).
Test: `complex/tests/div.rs::self_division_is_exactly_one`, `::exact_multiples_divide_exactly`,
`::div_is_correctly_rounded` (exact error-bound oracle, 20/50/100/500 bits, five modes).
Fix: `div`/`inv` form the numerators `xu + yv`, `yu − xv` and the denominator `u² + v²` exactly
and do one correctly-rounded real division per part. Smith's method is kept only as a fallback
when the exact intermediates would be impractically wide (exponent spread between the parts
beyond `8·(precision + operand digits) + 4096` digits) or `|w|²` leaves the exponent range; in
that fallback the result is near-correctly rounded as before. This is the one place where the
result is not guaranteed correctly rounded (documented on `Context::div`).

### 0 / w has unlimited precision
Reports: E105
Verdict: real, fixed
Root cause: `div_special` (`complex/src/div.rs:123`) returned `FBig::ZERO` parts (precision 0)
instead of zeros at the context precision.
Test: `complex/tests/div.rs::zero_numerator_keeps_the_context_precision`.
Fix: the zero quotient is built from the (signed-zero) numerators at the context's precision.
The `z/0` infinity now also uses the operation's context rather than `max` of the operands'.

### inf / 0 reports Indeterminate
Reports: E256
Verdict: real, fixed
Root cause: `div_special` skipped infinite operands and the general path then hit the float
`0/0` check of Smith's ratio before any infinity check.
Test: `complex/tests/div.rs::infinite_operand_is_rejected_before_the_zero_checks`.
Fix: `Context::div` and `Context::inv` reject an infinite operand up front (`InfiniteInput`).

### One out-of-range part discards the other
Reports: E199
Verdict: real, fixed
Root cause: a per-part `FpError::Overflow`/`Underflow` was propagated with `?` for the whole
complex result, and `unwrap_cfp` then collapsed it to `∞ + 0i` / `0 + 0i`
(`complex/src/repr.rs:118-119`); `inv` also formed `|z|²`, which overflows although `1/z` is
representable.
Test: `complex/tests/mul.rs::part_out_of_range_keeps_the_other_part`,
`complex/tests/div.rs::reciprocal_near_the_exponent_range_limits`.
Fix: new `combine_results` (`complex/src/repr.rs`): when exactly one part over/underflows it
saturates alone (signed infinity / signed zero, flagged inexact) and the other part is kept;
when both fail the error is returned as before. Used by mul, sqr, mul_real, fma, div, inv,
div_real. `inv` shares `div`'s code path, so it falls back to Smith's method when `|z|²` is out
of range. Only reachable with exponents near ±2^62.

### Context::neg/conj/proj/mul_i ignore a narrower context
Reports: E142
Verdict: real, fixed
Root cause: `complex/src/misc.rs:62-108` wrapped the operand's parts in the target context without
rounding, producing over-wide parts (debug assertion in `FBig::from_repr`) tagged `Exact`.
Test: `complex/tests/misc.rs::exact_ops_round_under_a_narrower_context`,
`::exact_ops_stay_exact_under_an_equal_or_wider_context`.
Fix: each part is carried over with `FBig::with_precision`, which rounds only when the operand's
context is wider than the target (so `-z`, `z.conj()` etc. on the operand's own context still
return the parts untouched, guard digit included).

### from_parts with a limited and an unlimited part
Reports: E112
Verdict: real, fixed
Root cause: `CBig::from_parts` (`complex/src/cbig.rs:95-102`) takes the limited precision
(`Context::max` treats unlimited as lowest) but kept the unlimited part's digits, violating the
`FBig` digit invariant (`into_parts` panics in debug, yields an invalid `FBig` in release).
Test: `complex/tests/convert.rs::from_parts_fits_an_unlimited_part_to_the_shared_precision`.
Fix: an unlimited part is rounded to the shared limited precision (as `FBig` arithmetic mixing
the two precisions does); docs updated.

### From<IBig>/From<UBig> precision
Reports: E191
Verdict: docs
Root cause: the doc comments (`complex/src/convert.rs:27,36`) claimed "exact, unlimited
precision"; the conversion is `FBig::from(int).into()`, whose precision is the digit count —
the documented `FBig` behaviour, and the same as the primitive-integer conversions.
Test: `complex/tests/convert.rs::integer_embeddings_follow_fbig` (pins the intended behaviour).
Fix: doc comments corrected. (If the float group changes `FBig::from(IBig)`, CBig follows
automatically and this test needs the same update.)

### Context::max docs
Reports: E265
Verdict: docs
Root cause: doc comment said "unlimited `0` dominates"; the code (and the existing unit test
`context_delegates_to_float`, and `dashu_float::Context::max`) treat unlimited as the lowest.
Test: existing `repr::tests::context_delegates_to_float`.
Fix: doc comment in `complex/src/repr.rs` corrected.

### FromStr rejects a signed exponent in the imaginary coefficient
Reports: D34, E110
Verdict: real, fixed
Root cause: `complex/src/parse.rs:49` split at the last `+`/`-`, which is the exponent's sign in
`1+2e-3i`.
Test: `complex/tests/io.rs::imaginary_coefficient_with_signed_exponent`.
Fix: the parser takes the rightmost sign at which both sides parse as numbers (an exponent sign
never qualifies, the text before it ends in the scale marker). `1+-2i`-style double signs are
rejected explicitly.

### FromStr rejects infinite parts
Reports: E97
Verdict: real, fixed
Root cause: `parse.rs:29-32` rejected any string with more than one `i` (`inf` has one), and
`FBig::from_str` intentionally does not accept `inf`.
Test: `complex/tests/io.rs::infinite_parts_round_trip`,
`complex/tests/serde.rs::cbig_serde_infinite_parts`.
Fix: the complex parser accepts `inf` / `+inf` / `-inf` as a coefficient itself (bases ≤ 23; above
that the letters are digits). This also makes real-part infinities (`inf+2i`) read back, which
the report listed as a separate known float limitation — it is handled inside the complex
parser, `FBig::from_str` is unchanged.

### `{:+}` signs every part
Reports: E154
Verdict: real, fixed
Root cause: `Display` (`complex/src/fmt.rs:25-56`) forwarded the formatter, flags included, to
both coefficients.
Test: `complex/tests/io.rs::sign_flag_applies_once`.
Fix: the imaginary coefficient is formatted with only the precision/alternate flags; the sign
flag applies once to the leading term (`+1+2i`, `+2i`, `-2i`). Width/fill now apply to the real
coefficient only.

### Display precision rounds a negative imaginary part the wrong way
Reports: E180
Verdict: real, fixed
Root cause: `Display` printed `-` followed by `|im|` rounded by the mode, mirroring the direction
under `Down`/`Up`.
Test: `complex/tests/io.rs::precision_flag_rounds_the_signed_coefficient`.
Fix: the signed coefficient is formatted directly (its own `-` is the separator).

### Bases where `i` is a digit
Reports: E125
Verdict: real, fixed
Root cause: in bases 19–36 the digit 18 is written `i`, so the trailing imaginary marker is
ambiguous (`1i` is both the real 38 and `0+1i`) and `2ii` was rejected.
Test: `complex/tests/io.rs::bases_where_i_is_a_digit_round_trip`.
Fix: a format decision — in bases above 18 `Display` writes the imaginary term with an explicit
`*i` suffix and never elides the coefficient (`2i*i`, `1*i`), and `FromStr` requires `*i` there
(a bare trailing `i` is a digit). Bases ≤ 18 are unchanged. Guide updated (both languages).
Worth a maintainer look, since it defines new syntax.

### Human-readable serde loses the precision
Reports: E130, E190
Verdict: real, fixed
Root cause: the `a+bi` string implies the precision by digit count. `write_fbig_padded`
(`complex/src/third_party/serde.rs:72`) counted only ASCII decimal digits, so letter digits made
it over-pad (E130); and an unlimited precision cannot be implied by any digit count (E190).
Test: `complex/tests/serde.rs::cbig_serde_letter_digits_keep_the_precision`,
`::cbig_serde_unlimited_precision`, `::cbig_serde_over_precision_value`.
Fix: letter digits are counted. The serializer then checks that the string reads back as the
same value at the same precision; if not (unlimited precision, or a part with more digits than
the precision, e.g. `0.1` at precision 1 or a guard digit) it writes the struct form
`{"re": …, "im": …, "precision": …}` in human-readable formats too, and the human-readable
deserializer accepts either form (`deserialize_any`). The existing test that pinned the
over-precision limitation now asserts the precision round-trips. A format decision worth a
maintainer look. Infinite parts still use the plain string (precision not recoverable).

### CachedCBig empty product
Reports: E244
Verdict: real, fixed
Root cause: `complex/src/cbig_cached_ops.rs:315,330` returned `Self::default()` (zero) for an
empty iterator in `Product`.
Test: `complex/tests/iter.rs::empty_sum_and_product_are_the_identities`,
`::cached_product_matches_cbig`.
Fix: the empty product is `CachedCBig::from(CBig::ONE)`.

## Per-report table

| Report | Cluster | Verdict |
|---|---|---|
| D24 | mul/sqr real part wrong under cancellation | real, fixed |
| D33 | z / z is not exactly 1 | real, fixed |
| D34 | FromStr rejects a signed exponent in the imaginary coefficient | real, fixed |
| E46 | Signed zero of a product depends on the operand order | real, fixed |
| E52 | z / z is not exactly 1 | duplicate of D33 (real, fixed) |
| E83 | Rounded results tagged Exact | real, fixed |
| E92 | mul/sqr real part wrong under cancellation | duplicate of D24 (real, fixed) |
| E94 | Rounded results tagged Exact | duplicate of E83 (real, fixed) |
| E97 | FromStr rejects infinite parts | real, fixed |
| E105 | 0 / w has unlimited precision | real, fixed |
| E106 | fma loses a fully cancelled result | real, fixed |
| E110 | FromStr rejects a signed exponent in the imaginary coefficient | duplicate of D34 (real, fixed) |
| E112 | from_parts with a limited and an unlimited part | real, fixed |
| E118 | Rounded results tagged Exact | duplicate of E83 (real, fixed) |
| E125 | Bases where `i` is a digit | real, fixed |
| E130 | Human-readable serde loses the precision | real, fixed |
| E142 | Context::neg/conj/proj/mul_i ignore a narrower context | real, fixed |
| E154 | `{:+}` signs every part | real, fixed |
| E180 | Display precision rounds a negative imaginary part the wrong way | real, fixed |
| E190 | Human-readable serde loses the precision | real, fixed |
| E191 | From<IBig>/From<UBig> precision | docs |
| E199 | One out-of-range part discards the other | real, fixed |
| E217 | Directed rounding goes the wrong way (sqr, div) | real, fixed |
| E244 | CachedCBig empty product | real, fixed |
| E246 | Directed rounding goes the wrong way (sqr, div) | real, fixed |
| E256 | inf / 0 reports Indeterminate | real, fixed |
| E265 | Context::max docs | docs |

## Changelog entries

`complex/CHANGELOG.md`, under `## Unreleased`:

```markdown
### Change
- `Context::mul`, `sqr`, `fma`, `norm`, `div` and `inv` (and the `*`, `/` operators) are now correctly rounded per component: each part is the exact result rounded once. Division falls back to the previous near-correctly-rounded Smith's method only when the exponents of the operand parts are astronomically far apart or `|w|²` leaves the exponent range.
- A result part that leaves the exponent range now saturates on its own (signed infinity / signed zero) instead of discarding the other, representable part; the whole result still collapses to `∞ + 0i` / `0 + 0i` when both parts do.
- `Display` in bases above 18 (where `i` is a digit) writes the imaginary term with an explicit `*i` suffix and an explicit coefficient (`2i*i`, `1*i`), and `FromStr` requires that suffix there.
- `Display` with the `+` flag signs only the leading term (`+1+2i`), and width/fill apply to the real coefficient only.
- Human-readable serde writes the struct form `{re, im, precision}` when the `a+bi` string cannot imply the precision (unlimited precision, or a part with more digits than the precision), and reads either form.
- `CBig::from_parts` rounds an unlimited-precision part to the shared precision when the other part's precision is limited.
- `Context::neg`, `conj`, `proj` and `mul_i` round to the context precision (and report `Inexact`) when it is narrower than the operand's.

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
```

## Cross-group notes

- No file under `float/` was touched; every fix is in `complex/` (plus the guide pages
  `guide/src/ops/basic.md`, `guide/src/io/print.md`, `guide/src/io/parse.md` and their `guide-zh`
  mirrors).
- **complex-trans**: `combine_parts` (shared by the transcendentals) no longer collapses
  `Inexact(_, NoOp)` parts to `Exact`. I updated the one unit test that pinned the old behaviour,
  `complex/src/math/trig.rs` `tan_tanh_large_imaginary_saturation`. `Context::neg`/`conj`/`mul_i`
  are unchanged for operands at the context's own or a narrower precision.
- **float-fmt** (A6, D20, E42, E73, E91, E135 are the `FBig` serde analogues of E130/E190/E97):
  the complex human-readable serializer still relies on `FBig::from_str` inferring the precision
  from the digit count, but it now verifies the round trip and falls back to the struct form, so
  it stays correct if that inference changes. The struct form serializes the parts through
  `Repr`'s serde; `Repr`'s human-readable form cannot read `inf` back (E73), which is why
  infinite parts keep the string form here.
- **float-arith**: the complex code relies on `dashu_float::Context::{mul, add, div}` being exact
  at unlimited precision / correctly rounded for operands wider than the precision. E40 and E115
  (float-arith) concern those paths; the complex tests here use exact oracles and pass at this
  commit.
- E191 depends on `FBig::from(IBig)` semantics (float-convert); if that changes, the pinned test
  and the doc comment need the same change.
- **Fuzz differentials (MPC via `rug`, run in release with `--ignored`)**: `fuzz/tests/cmplx_random.rs`
  (`cbig_mul_div_sqr_fuzz`) passes; `fuzz/tests/cmplx_transcendental.rs` passes 21 of 22.
  `cbig_tan_fuzz` fails under `mode::Down` (e.g. `tan[down] zre=5.0575 zim=61.408 prec=50`, the
  imaginary part saturating just below 1) — it fails the same way on the unmodified base commit,
  so it is a pre-existing complex-trans issue, not caused by these changes.
