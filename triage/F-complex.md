# F-complex

15 reports, 13 clusters (plus one float defect found on the way). Every reproducer was run at
`b0705ed` (the tree the F run was against, `8137b6e`, plus the notes) and against the base
checkout `a0fc535`; all 15 reproduce. All verdicts are **real, fixed** — no report in this group
was wrong, though three of them (F38, F57, F46) are about behaviour that someone had written
down as intended.

File and line references under "Root cause" are to the sources before the fix (`b0705ed`).

The new regression tests for the wide-sum clusters (F33, F34, F68, F44, F79) were also run
against the unfixed sources: every one fails there. For the other clusters the failure before
the fix is the report's reproducer, run before and after.

## Clusters

### UniformCBig rejects a zero-width side
Reports: F38
Verdict: real, fixed
Root cause: `UniformCBig::new` (`complex/src/third_party/rand.rs:39`) builds each side with
`UniformFBig::new`, which since `1812267` panics on `low == high` (an empty half-open interval).
The documented panic condition (`low.part > high.part`) does not hold, and a box that
degenerates to a segment (real-only samples) is a natural request.
Base behaviour: no panic; the degenerate part is the bound (verified by running).
Provenance: **regression** — `1812267` ("float: keep uniform samples inside their range, and
reject empty ranges and zero-digit open intervals"; D25/E176 in float-convert). The float
behaviour change was deliberate and is listed under "Changes that need a maintainer's
decision"; its effect on the complex sampler was not considered.
Test: `complex/tests/random.rs::uniform_cbig_degenerate_side`,
`::uniform_cbig_reversed_side_panics`.
Fix: a side of zero width uses the closed `[x, x]` sampler, so that part of every sample is the
bound; a reversed side still panics. Docs updated (zero width allowed, infinite bound panics).
Note: the report offered "change the docs to `>=`" as the alternative. I restored the base
behaviour instead, because it is useful and the docs already promised it.

### Human-readable serde: infinite part in bases 24–36
Reports: F28
Verdict: real, fixed
Root cause: `Serialize for CBig` (`complex/src/third_party/serde.rs:37-39`) skipped the
read-back check when a part is infinite and always wrote the `a+bi` string. In bases above 23
the letters of `inf` are digits, so `CBig::from_str` reads it as the finite 24171 (base 36).
Base behaviour: the string `"inf"` was written in every base and failed to deserialize in every
base (`invalid syntax`) — an error, not a wrong value (verified by running).
Provenance: **incomplete-fix** of E97 ("FromStr rejects infinite parts": the parser was taught
`inf` for bases ≤ 23 only, by design) combined with `dcae487`, whose `faithful` check exempts
infinite parts. The failure mode got worse for bases 24–36: an error at the base, a silently
wrong finite value now.
Test: `complex/tests/serde.rs::cbig_serde_infinite_parts` (bases 2, 10, 16, 23, 24, 36; five
precisions; value and precision), `::cbig_serde_finite_part_spelled_inf`.
Fix: the string is used only when it reads back as the same value at the same precision, with
no exemption. A finite part next to an infinite one is padded as usual, so the precision now
round-trips for such values (`"inf+2.000i"`); otherwise the struct form is written
(`{"re":"inf","im":…,"precision":…}`), which `Repr`'s deserializer reads in every base.
Note: behaviour change — values with an infinite part and a precision the string cannot imply
are now written in the struct form (both forms were already accepted on input).

### Human-readable serde: time grows faster than the exponent
Reports: F39
Verdict: real, fixed
Root cause: the same `faithful` check rendered the padded `a+bi` string and parsed it back
before deciding. For a part with a large exponent that string has as many digits as the
exponent is large, and parsing it is superlinear; the result was then discarded for the struct
form.
Base behaviour: 34 ms at E = 10⁷ (the base wrote the string without checking); 2.7 s at this
commit (verified by running, release).
Provenance: **regression** — `dcae487` ("complex: human-readable serde keeps the precision",
E130/E190).
Test: `complex/tests/serde.rs::cbig_serde_large_exponent_parts` (struct form chosen, value and
precision round-trip) and the unit test `third_party::serde::tests::positional_fit` (the
pre-check that selects the struct form without formatting).
Fix: `fits_positional` decides from the digit count and the exponent whether the positional
rendering of a part can have at most `precision` digits; if not, the struct form is written
directly. After the fix the CBig takes as long as its two `Repr` fields (74 ms vs 73 ms at
E = 10⁷).
Note: the size of the struct form itself (`Repr` writes a large exponent positionally, 20 MB
here) is the separate `Repr` serde issue the report mentions; not touched.

### Sum overflow discards the other part
Reports: F76
Verdict: real, fixed
Root cause: `Context::add`/`sub` (`complex/src/add.rs:18-30`) propagated the first part's
`Overflow` with `?`, so the operators collapsed to `inf + 0i`.
Base behaviour: identical (`Err(Overflow)`, `inf + 0i`); at the base `mul_real` did the same.
Provenance: **incomplete-fix** of E199 ("One out-of-range part discards the other", `d7c3d32`):
the componentwise policy (`combine_results`) was applied to mul/div/mul_real/div_real but not
to add/sub.
Test: `complex/tests/add.rs::part_out_of_range_keeps_the_other_part` (new file; 10/20/50/100/500
bits, context layer and operators).
Fix: add/sub go through `combine_results`.

### One out-of-range product erases a part of mul/sqr/fma
Reports: F33
Verdict: real, fixed
Root cause: `sum_of_products` (`complex/src/mul.rs:69`) returned the error of the first product
that leaves the exponent range, although the sum can be an ordinary number (the product vanishes
below the rounding window, or cancels against another one).
Base behaviour: also wrong — `0 + 0i` for the reported input (the base lost the imaginary part
too); verified by running.
Provenance: **pre-existing**. The code is new (`d7c3d32` rewrote mul as `sum_of_products`), but
it gives the base's result for the real part on this input; the rewrite fixed the imaginary
part (E199) and kept this limitation.
Test: `complex/tests/mul.rs::product_out_of_range_inside_a_part` (20/50/100/500 bits, five
modes: underflowing product, overflowing products that cancel, value and flags), unit tests in
`complex/src/wide.rs` (the wide path against the in-range path on in-range inputs).
Fix: new module `complex/src/wide.rs`. When a product leaves the range (and the precision is
limited) the sum is formed with the exponent in an `i128`, using the same sticky-unit idea as
the in-range path; only a *result* outside the range is an error. The in-range path is
untouched. At unlimited precision the product's error is still returned (nothing may be
dropped there).

### Division: wrong flags, wrong direction and spurious range errors at large spreads
Reports: F34, F68
Verdict: real, fixed
Root cause: beyond a moderate exponent spread (`exact_sum_is_cheap`, about `8·digits + 4096`
digits) or when `|w|²` leaves the exponent range, `quotient` fell back to `div_smith`
(`complex/src/div.rs:167-215`): guard-precision arithmetic re-rounded to the target. That
(a) tags a result `Exact` whenever the guard-precision value happens to fit (F34), (b) rounds
the wrong way under directed modes (F34), and (c) reports an in-range quotient as
overflow/underflow when `r·v`, `u + r·v` or `v/u` leaves the range (F68).
Base behaviour: wrong in the same way and at smaller spreads too — at the base
`inv(1 + 2^100 i)` is already tagged `Exact` and rounded up under `Down`; `inv(2^e + 2^e i)`
overflows for e = MAX−3 … MAX−1 (now only MAX−1); `inv(2^E + 2^-E i)` at E = 2^62 gives
`Overflow` (now `Underflow`). Verified by running.
Provenance: **incomplete-fix** of E83/E94/E118 ("Rounded results tagged Exact") and E217/E246
("Directed rounding goes the wrong way (sqr, div)"): `d7c3d32` made division exact up to the
"cheap" threshold and documented the Smith fallback beyond it as "near-correctly rounded". F68
is the same fallback; for those inputs the base failed too (F68a the same way, F68b with the
opposite error).
Test: `complex/tests/div.rs::wide_exponent_spread_is_correctly_rounded` (spread 2^20000 against
the exact real division, 20/50/100/500 bits, five modes, flags, exact multiples),
`::quotient_in_range_with_intermediates_out_of_range` (both F68 shapes, exponents up to the
last one, astronomically wide divisors, parts that underflow on their own), unit test
`wide::tests::wide_ratio_matches_the_exact_division` (the new path against the exact division
on ~39k small cases, six modes).
Fix: `ratio_of_sums` in `complex/src/wide.rs` replaces Smith's method. An approximate quotient
(8 guard digits, from the two sums rounded with sticky units) pins the truncated significand
down to two neighbours; exact sign tests of `|N| − m·u·D` and `2|N| − (2m+1)·u·D` decide between
them and place the remainder relative to ½, and `Round::round_ratio` does the rest. Every
quotient is now correctly rounded with honest flags; a part outside the range saturates on its
own. `div_smith`, `DIV_GUARD` and `Context::work_context` are gone; the module docs, the
`div`/`inv` docs and the guide (`guide/src/ops/basic.md`, `guide-zh/src/ops/basic.md`) no
longer mention the exception. The cheap exact path is untouched.

### abs_cmp: slow for parts far apart, panics for large exponents
Reports: F44, F79
Verdict: real, fixed
Root cause: `AbsOrd::abs_cmp` (`complex/src/cmp.rs:57-67`) formed both `|z|²` exactly at
unlimited precision: that aligns `re²` and `im²` across their whole exponent gap (F44, time
linear in the gap) and fails when a square leaves the exponent range, which `unwrap_fp` turns
into the unrelated "precision cannot be 0" panic (F79).
Base behaviour: identical timings and the same panic (verified by running).
Provenance: **pre-existing** (both).
Test: `complex/tests/cmp.rs` (new file): `abs_cmp_matches_the_exact_squared_moduli`,
`abs_cmp_with_parts_far_apart`, `abs_cmp_with_squares_beyond_the_exponent_range`,
`abs_cmp_with_infinite_parts`; unit test `wide::tests::sign_across_the_exponent_range`.
Fix: `abs_cmp` is the exact sign of `re₁² + im₁² − re₂² − im₂²` (`wide::sign_of_sum`), which
accumulates from the top and stops as soon as the partial sum exceeds what is left. Infinite
values are handled first (larger than any finite value, equal to each other — as before, but a
finite operand with an out-of-range square no longer panics next to one).

### CBig::fma rounds at the receiver's precision
Reports: F58
Verdict: real, fixed
Root cause: `CBig::fma` (`complex/src/mul.rs:232`) used `self.context()` alone.
Base behaviour: identical (verified by running).
Provenance: **pre-existing**.
Test: `complex/tests/mul.rs::fma_rounds_at_the_largest_operand_precision`.
Fix: the context is the maximum of the three operands' (as for the binary operators and
`FBig::fma`). `CachedCBig` had no `fma` at all — added, delegating to the inner value.

### arg of a zero is OutOfDomain
Reports: F57
Verdict: real, fixed
Root cause: `Context::arg` (`complex/src/misc.rs:153`) forwards to the real `atan2`, which
deliberately rejects two zero arguments; the complex docs promise the C99 `atan2` table and
`Context::log` already returns `arg(±0 ± 0i)` as its imaginary part.
Base behaviour: identical (verified by running).
Provenance: **pre-existing**.
Test: `complex/tests/special_values.rs::arg_of_signed_zeros` (four zeros, 20/50/100/500 bits,
four modes, against `log` and `f64::atan2`),
`::arg_of_negative_zero_at_unlimited_precision_panics`.
Fix: `arg(+0 ± 0i) = ±0` exactly, `arg(−0 ± 0i) = ±π` rounded once under the mode. The real
`atan2(0, 0)` is unchanged (it is tested as `OutOfDomain` in dashu-float). Compliance table
updated in both guides.
Note: the report's suggested test is partly wrong. It builds the zeros with `try_from(0.0)`,
which have unlimited precision, and expects `CBig::arg(−0 ± 0i)` to be a 53-bit `±π`. There is
no precision to round `π` to there, so that call panics with the library's usual
"precision cannot be 0 (unlimited)" (pinned by the second test), as every other inexact `arg`
at unlimited precision does; `Context::arg` under a limited context returns `±π`, and
`arg(+0 ± 0i) = ±0` is exact at any precision.

### cos(±0 ± 0i) has the wrong imaginary zero sign
Reports: F46
Verdict: real, fixed
Root cause: `sin_cos_of_zero` (`complex/src/math/trig.rs:83-87`) used the sign of the product
`x·y` for the imaginary zero of `cos` — the `cosh` table. For `cos` it is `−sin x·sinh y`
(`ccos(z) = ccosh(iz)`), the opposite sign. The code comment, the unit test
`sin_cos_signed_zero` and the fuzz test `cbig_signed_zero_fuzz` all pinned the wrong table; the
nonzero neighbours `cos(±ε ± 0i)` already returned the right sign.
Base behaviour: identical (verified by running).
Provenance: **pre-existing**.
Test: `complex/tests/trig.rs::cos_of_signed_zero` (`cos`, `sin_cos`, `cos_pi`, `sin_cos_pi`;
against `cosh(iz)` and the nonzero neighbours; 20/50/100/500 bits); unit test and fuzz test
corrected.
Fix: the sign is `−(sign x · sign y)`.

### tan/tanh overflow for |Im z| ≥ 2^62
Reports: F45
Verdict: real, fixed
Root cause: `tan_parts` (`complex/src/math/trig.rs:262-274`) evaluates `sinh 2y`, `cosh 2y`
even in its saturated branch; for `2|y| ≥ 2^63·ln 2` they overflow and the error was returned
for the whole result.
Base behaviour: identical (verified by running).
Provenance: **pre-existing**.
Test: `complex/tests/trig.rs::tan_with_imaginary_part_beyond_the_exponent_range` (`tan`,
`tanh`, `tan_pi`; y up to 2^(isize::MAX−10); both sides of ±1; four modes; 20/50/100/500 bits).
Fix: when the kernels overflow in the saturated branch, `tan_beyond_range` takes the side of ±1
from the sign of `cos 2x` (`e^{−t}` is below every positive representable number there) and
lets the real part underflow to a zero with the sign of `sin 2x`. A real part that alone
underflows (with `e^{2|y|}` still in range) now saturates the same way instead of failing the
whole result.

### exp of a tiny argument does not return under directed modes
Reports: F69
Verdict: real, fixed
Root cause: `exp(x + iy)` for tiny `x`, `y` is `1 + iy` displaced by far less than the
precision shows. The Ziv composition (`complex/src/exp.rs:322-329`) works on symmetric balls,
which cannot tell which side of the closed form the value is on until the working precision
reaches about twice the exponent of the argument.
Base behaviour: identical — no result within 10 s at k = 10⁷ (verified by running). At the base
the real `FBig::exp(2^-k)` hung the same way; the float layer got its tiny-argument shortcuts
during the earlier triage, the complex layer did not.
Provenance: **pre-existing**.
Test: `complex/tests/exp.rs::exp_of_tiny_argument` (16 shapes against a high-precision oracle
at k = p + 40, including the degenerate `y² = 2x` and `y² = 6x`; arguments down to 2^-(2^61)
against the closed forms; five modes; 20/50/100/500 bits).
Fix: on the axes each part is a single real kernel (`exp(x ± 0i) = e^x ± 0i`,
`exp(±0 + iy) = cos y + i sin y`), correctly rounded by the float layer. Off the axes
`exp_tiny` rounds `1` and `y` with a sticky part whose side is the exact sign of `2x − y²`
(resp. `6x − y²`), when that dominates the remainder (else the general path runs).
Note: the same weakness exists for tiny arguments of the other complex transcendentals — see
"Known leftovers".

### log next to 1 does not return
Reports: F78
Verdict: real, fixed
Root cause: `ln|z|` was `ln(hypot(x, y))` (`complex/src/ball.rs`, `CBall::log`). Next to the
unit circle the modulus rounds to 1 at any working precision below twice the exponent of the
offset, so the Ziv loop had to climb there (and the imaginary part `atan2(2^-e, 1)` has the
same closed-form-plus-invisible-tail shape under directed modes).
Base behaviour: identical — 3 s at e = 750000, no result at e = 800000 (verified by running).
Provenance: **pre-existing**.
Test: `complex/tests/log.rs::log_next_to_one` (10 shapes against a high-precision oracle in
base 2, plus bases 10 and 3; offsets down to 2^-(2^61) against the closed forms; `|z| = 1`
exactly; an underflowing real part; five modes; 20/50/100/500 bits),
`::log_next_to_one_odd_base_nearest_shallow`.
Fix: the two parts are rounded independently. `arg z` is one real `atan2`, correctly rounded by
the float layer. For a modulus near 1 (`−1/B < s < 1`), `ln|z| = ½·ln(1 + s)` with
`s = x² + y² − 1` formed exactly and rounded once (`sum_of_products`); a small `s` is summed
through its series as a ball (`ln_1p_ball`, at most 16 terms, else the float `ln_1p`), and when
`s` is exact and tiny the result is `s/2` rounded with a negative sticky part (even bases).
Elsewhere `ln(hypot)` as before (`CBall::ln_abs`, split out of `CBall::log`). A real part that
underflows now saturates on its own (`combine_results`).
Note: in an odd base under the round-to-nearest modes a deep case still does not return —
`log(1 + i·3^-510)` at 500 digits. That is not this cluster's defect but a float one (see
"Cross-group notes": odd-base `ErrorBounds`); it fails the same way at `b0705ed`, and returns
at the base. Pinned by the `#[ignore]`d `complex/tests/log.rs::log_next_to_one_odd_base_nearest`.

### float: the finite number spelled `inf` serializes as an infinity (found with F28)
Reports: none (found while fixing F28; the complex struct form goes through `Repr`'s serde)
Verdict: real, fixed
Root cause: `fbc65cf` made the human-readable deserializers of `Repr`/`FBig` read the strings
`inf`/`-inf` as infinities in every base, but in bases above 23 the finite integer with the
digits `i`, `n`, `f` (24171 in base 36) is written as exactly that string
(`float/src/third_party/serde.rs`).
Base behaviour: `"inf"` read back as the finite number (and a real infinity did not round-trip).
Provenance: **regression** — `fbc65cf` ("float: serde keeps the precision exactly, reads back
infinities and owned field names").
Test: `float/tests/serde.rs::test_serde_finite_number_spelled_inf`.
Fix: that one finite value is written with an explicit zero scale (`inf@0`, `-inf@0`), which
the parser already reads, for `Repr` and for `FBig`.

## Known leftovers

### Tiny arguments of the other complex transcendentals under directed modes
Reports: none in this group (F69 and F78 are the `exp` and `log` instances)
Verdict: real, not fixed
Root cause: the same as F69/F78 — next to a closed form (`sin z = z − z³/6 + …`) the Ziv
composition on symmetric balls certifies a directed rounding only at a working precision
proportional to the exponent of the argument. Measured at this commit under `Down`, 53 bits,
release, argument `2^-300000·(1 + i)`: `sin`, `cos`, `tan`, `sinh`, `cosh`, `tanh`, `asin`,
`acos`, `acosh`, `sin_pi`, `cos_pi` take 0.3–0.8 s each (growing faster than linearly: they
would take minutes at 2^-10⁷); `atan` takes 1.4 s already at `2^-4000·(1 + i)` and, like
`atanh`, more than 30 s at 2^-20000; `asinh(1 + i·2^-300000)` does not return within 12 s.
`sqrt` and `tan_pi` are fine.
Base behaviour: the same (`atan(2^-1000·(1+i))`: 81 ms at both commits).
Provenance: **pre-existing**.
Test: `complex/tests/trig.rs::tiny_arguments_under_directed_modes`, `#[ignore]`d (its closed
forms were checked at 2^-300, where the calls return).
Why not fixed: each function needs its own closed form and side analysis, as done for `exp`
(`exp_tiny`) and `log` here — about fourteen of them — or the balls need to carry a one-sided
"strictly above/below" bit through every operation. Either is a project of its own.

## Per-report table

| Report | Cluster | Verdict | Provenance |
|---|---|---|---|
| F28 | Human-readable serde: infinite part in bases 24–36 | real, fixed | incomplete-fix (E97; `dcae487`) |
| F39 | Human-readable serde: time grows faster than the exponent | real, fixed | regression (`dcae487`) |
| F33 | One out-of-range product erases a part of mul/sqr/fma | real, fixed | pre-existing |
| F34 | Division: wrong flags, wrong direction and spurious range errors at large spreads | real, fixed | incomplete-fix (E83/E94/E118, E217/E246) |
| F68 | Division: wrong flags, wrong direction and spurious range errors at large spreads | real, fixed (same fallback as F34, different symptom) | pre-existing |
| F38 | UniformCBig rejects a zero-width side | real, fixed | regression (`1812267`) |
| F44 | abs_cmp: slow for parts far apart, panics for large exponents | real, fixed | pre-existing |
| F79 | abs_cmp: slow for parts far apart, panics for large exponents | real, fixed (same function as F44, different symptom) | pre-existing |
| F45 | tan/tanh overflow for \|Im z\| ≥ 2^62 | real, fixed | pre-existing |
| F46 | cos(±0 ± 0i) has the wrong imaginary zero sign | real, fixed | pre-existing |
| F57 | arg of a zero is OutOfDomain | real, fixed | pre-existing |
| F58 | CBig::fma rounds at the receiver's precision | real, fixed | pre-existing |
| F69 | exp of a tiny argument does not return under directed modes | real, fixed | pre-existing |
| F76 | Sum overflow discards the other part | real, fixed | incomplete-fix (E199) |
| F78 | log next to 1 does not return | real, fixed | pre-existing |

Counts: 15 real, fixed (13 clusters). Provenance by report: 9 pre-existing, 4 incomplete-fix,
2 regression. Found on the way and not from a report: the float serde `inf` spelling
(regression, fixed), the tiny-argument leftover (pre-existing, not fixed) and the odd-base
`ErrorBounds` hang in dashu-float (regression, not fixed, flagged).

## Changelog entries

`complex/CHANGELOG.md`, under `## Unreleased`:

### Add
- `CachedCBig::fma`, mirroring `CBig::fma`.

### Change
- Complex division (`/`, `Context::div`, `Context::inv`, `Inverse`) is correctly rounded per component for every finite operand: the fallback to Smith's method for operands whose parts are far apart (or whose squared modulus leaves the exponent range) is gone.
- `UniformCBig::new` accepts a box with a zero-width side (`low.re == high.re` or `low.im == high.im`): that part of every sample is the bound.
- Human-readable serde writes the struct form for a value with an infinite part whenever the `a+bi` string would not read back as the same value at the same precision (bases above 23, or two infinite parts at a limited precision); a finite part next to an infinite one is now padded, so the precision round-trips.
- `Context::add` / `Context::sub` (and the `+`/`-` operators) saturate a part that overflows on its own and keep the other part, like multiplication and division; `Err` is returned only when both parts overflow.
- `CBig::fma` rounds at the largest precision of its three operands instead of the receiver's.
- `Context::arg` / `CBig::arg` of a zero return `±0` (`+0 ± 0i`) or `±π` (`−0 ± 0i`) instead of `OutOfDomain` / a panic.

### Fix
- `mul`, `sqr` and `fma`: a single product leaving the exponent range no longer erases the part it belongs to (the real part of `(1 + s·i)²` was `0` for `s²` below the range); only a result outside the range over- or underflows.
- `div` / `inv` with a large exponent spread between the operand parts tagged inexact results `Exact`, rounded the wrong way under directed modes, and returned overflow/underflow for quotients that are in range.
- `AbsOrd::abs_cmp` took time proportional to the exponent gap between the parts of a value, and panicked when a squared part left the exponent range; it is now exact and prompt for all finite values.
- `cos`, `cos_pi`, `sin_cos` and `sin_cos_pi` of a signed zero return the imaginary zero with the sign of `−sin x·sinh y` (`cos(+0 + 0i) = 1 − 0i`), as for the nonzero neighbours; it had the sign of the `cosh` table.
- `tan`, `tanh` and `tan_pi` return `±1` (rounded from the correct side) instead of an overflow when `e^(2|Im z|)` is beyond the exponent range (`|Im z| ≥ 2^62` in base 2); a real part that underflows on its own no longer fails the whole result.
- `exp` of a tiny argument (and of a purely real or purely imaginary one) returns promptly under the directed rounding modes; it used to need a working precision of about twice the exponent of the argument.
- `log` of an argument next to the unit circle (`1 + i·2^-k`) returns promptly; `ln|z|` is computed from `x² + y² − 1` there, and the imaginary part is a single correctly rounded `atan2`.
- Human-readable serde wrote a value with an infinite part as a string that reads back as a finite number in bases 24 to 36.
- Human-readable serialization of a value with a large exponent no longer renders and re-parses a string as long as the exponent is large before choosing the struct form.

`float/CHANGELOG.md`, under `## Unreleased`:

### Fix
- serde: in bases above 23 the finite number whose digits spell `inf` (24171 in base 36) is written as `inf@0` by the human-readable serializers of `Repr` and `FBig`; it was written as `inf`, which reads back as an infinity.

## Cross-group notes

- **float (round / Ziv), not fixed — a regression that other groups should know about**: in an
  odd base the round-to-nearest `ErrorBounds` are cut off about 129 digits (base 3) inside half
  an ulp (`half_ulp`, `float/src/round.rs:215-227`, from `f2b0479`). A value closer to a tie
  than that is never certified, so every Ziv loop (float and complex) climbs to its retry cap
  at an exploding working precision — an effective hang. Ties are natural in odd bases
  (`1/2 = 0.111…₃`): `FBig::<HalfEven, 3>` `ln_1p(3^-220)` at 228 digits does not return at
  this commit and returns in 0.2 ms at `a0fc535`; complex `log(1 + i·3^-510)` at 500 digits
  likewise (both verified by running, also at `b0705ed`). It is outside this group's reports
  and needs an exact comparison against half an ulp (doubling both sides) in the two
  certification routines rather than a truncated bound.

- **float (serde)**: `float/src/third_party/serde.rs` and `float/tests/serde.rs` — the `inf@0`
  fix above (commit "float: do not serialize the finite number spelled inf as an infinity").
  Small and self-contained; it may collide textually with another agent's serde changes.
- **fuzz**: `fuzz/tests/cmplx_transcendental.rs` (`cbig_signed_zero_fuzz`) pinned the wrong
  `cos` zero table; corrected with F46. The MPC differentials build here and pass at the end
  of this work: `cmplx_random` (1 test) and `cmplx_transcendental` (22 tests), release,
  `--ignored`, `FUZZ_CASES=200`.
- **MSRV**: no 1.68 toolchain is installed here; the changed crates were checked with 1.85
  (the oldest installed toolchain whose cargo reads the workspace) and with
  `--no-default-features`. The new code uses nothing newer than `usize::BITS` (1.53).
- **guides**: `guide/src/compliance.md`, `guide/src/ops/basic.md` and their `guide-zh` mirrors
  (arg of a zero; division no longer has an exception).
