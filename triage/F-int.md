# F-int — dashu-int and dashu-base (run F, against `8137b6e`)

Reports: F1, F2, F3, F4, F5, F6, F8, F93 (8 reports, 7 clusters).

Every reproducer was run against the current tree and against the base checkout
(`a0fc535`) from throwaway crates with path dependencies. All seven clusters fail the
same way at the base: nothing in this group was introduced by the earlier triage fixes.

## Clusters

### tuning-thresholds — `DASHU_THRESHOLD_*` values outside a kernel's domain panic
Reports: F1, F4
Verdict: real, fixed
Root cause: the threshold accessors of the `tuning` feature returned the environment value
unchecked (`integer/src/mul/mod.rs:66`, `integer/src/sqr/mod.rs:57`,
`integer/src/high/mul.rs:48`, `integer/src/high/sqr.rs:39`), so a kernel could be entered
below its minimum length: Toom-4x2 below 32 words (`TOOM42_MIN=10`), Toom-3 below 16
(`SIMPLE_MUL=4 KARATSUBA_MUL=8`), and the short-product split on windows of 1 to 3 words
(`MULHIGH_SIMPLE` / `SQRHIGH_SIMPLE` = 0, 1, 2). Only the division thresholds were clamped.
Base behaviour: identical panics (run: `TOOM42_MIN=10` -> `toom_4_2.rs:92` assertion,
`SIMPLE_MUL=4,KARATSUBA_MUL=8` -> `toom_3.rs:73` assertion, `MULHIGH_SIMPLE=0` ->
`high/mul.rs:173`, `SQRHIGH_SIMPLE=0` -> `high/sqr.rs:91`). The accessors are unchanged
between `a0fc535` and `8137b6e`.
Provenance: pre-existing.
Test: `integer/tests/tuning.rs::test_threshold_overrides_never_fail` (new file, gated on
`feature = "tuning"`; one test function because it changes process-wide environment
variables). Verified to fail without the fix.
Fix: each accessor clamps its value to the range the gated kernel accepts, following the
existing `div::threshold` pattern: `SIMPLE_MUL` to `[karatsuba::MIN_LEN - 1,
simple::MAX_SMALLER_LEN]`, `KARATSUBA_MUL >= toom_3::MIN_LEN - 1`, `TOOM4_MUL >=
toom_4::MIN_LEN - 1`, `TOOM42_MIN >= toom_4_2::MIN_LEN`, `NTT_MUL >= TOOM4_MUL`, the same
for the `_SQR` variables (`SIMPLE_SQR >= 3`), and `MULHIGH_SIMPLE` / `SQRHIGH_SIMPLE >= 8`
(the bound the existing `const_assert` already imposed on the default).
Notes: to check the fix I swept all 16 threshold variables over 22 values each (one at a
time, all at once, and a few pairs; 381 configurations, products up to 4100 x 4100 words,
squares, `mul_high` / `sqr_high` / `div_high` contracts, `div_rem`, `is_multiple_of`, in a
release build with debug assertions). The sweep found further failures that the reports did
not mention, all fixed in the same commit:
- `SIMPLE_MUL=0/1` and `SIMPLE_SQR=0/1/2` (Karatsuba below its minimum; for squaring the real
  minimum is 3, one more than the old `const_assert` allowed for). Pre-existing.
- `SIMPLE_MUL >= 1025` failed `debug_assert!(b.len() <= MAX_SMALLER_LEN)`. Pre-existing.
- `SIMPLE_MUL` above the Toom-4x2 minimum: the schoolbook kernel splits a long operand into
  1024-word chunks and re-dispatches the tail, which can then land in the Toom-4x2 band and
  ran out of scratch ("not enough memory allocated", e.g. 1924 x 500 words).
  `memory_chain_budget` now budgets that tail (`integer/src/mul/mod.rs`, simple branch).
  Unreachable with the default thresholds.
- `NTT_MUL` below `TOOM4_MUL` (e.g. `NTT_MUL=100`, 300 x 200 words): "not enough memory
  allocated". This one is a **regression** in an unreported configuration: the base passes
  it, and `68b0e88` (scratch for short residues) made the transform branch of
  `memory_chain_budget` skip the chunk tail for shapes that are not chunked by the transform
  kernel, which is wrong when Toom-3/4 take them. Fixed by keeping `NTT_MUL >= TOOM4_MUL`
  (the `const_assert` on the defaults already requires it).
- `NTT_ASYM_RATIO=usize::MAX`: overflow in `ratio * b` (now `saturating_mul`). Pre-existing.
- With 16-bit words, any `NTT_MUL` / `NTT_SQR` value reached `unreachable!("NTT unavailable
  on 16-bit targets")`; the override is no longer read there. Pre-existing.
After the fix all 381 configurations pass, at both operand-size ranges. At the base 44 of
them fail at the small sizes (up to 300 words); the tree before this fix failed 38 of the
364 configurations of a slightly smaller version of the same sweep. `NTT_ASYM_MIN=0..10`
failed at the base and was already fixed by the earlier triage.
The report says `DASHU_THRESHOLD_SIMPLE_DIV` and `DIVHIGH_SIMPLE` already clamp: correct.

### num-integer-multiple-of-zero — `Integer::is_multiple_of(&x, &0)` panics
Reports: F2
Verdict: real, fixed
Root cause: `integer/src/third_party/num_integer.rs:25` (UBig) and `:122` (IBig) forwarded
to the inherent `is_multiple_of`, which documents a panic for a zero divisor, whereas
num-integer defines `x.is_multiple_of(&0) == x.is_zero()` (its primitive impls and tests).
`divides` forwarded the same way.
Base behaviour: same panic (`div_exact.rs:650: division by zero`), run.
Provenance: pre-existing (the earlier num-integer cluster, `ffa44c4`, fixed `div_floor`,
`lcm` and `gcd(0, 0)` but did not look at divisibility).
Test: `integer/tests/num_integer.rs::is_multiple_of_zero_divisor`
Fix: the trait methods return `self.is_zero()` for a zero divisor; `divides` goes through
the trait method. The inherent methods are unchanged (their panic is documented).

### num-integer-ubig-extended-gcd — `Integer::extended_gcd` on `UBig` panics for coprime inputs
Reports: F3
Verdict: not a bug (unrepresentable result); panic message and docs improved
Root cause: `num_integer::ExtendedGcd<UBig>` has `UBig` coefficients, and
`x*a + y*b == gcd` has a solution without a negative coefficient only when the gcd is one
of the operands. For every other input there is no value to return. The impl
(`integer/src/third_party/num_integer.rs:63`) converted the `IBig` coefficients with a bare
`unwrap()`, hence the unhelpful `OutOfBounds` message. num-integer's own impl for the
primitive unsigned integers fails the same way: `Integer::extended_gcd(&3u32, &5u32)` panics
with "attempt to subtract with overflow" in a debug build (run) and wraps in release.
Base behaviour: same panic (`num_integer.rs:54: ... OutOfBounds`), run.
Provenance: pre-existing.
Test: `integer/tests/num_integer.rs::ubig_extended_gcd_with_representable_coefficients`
(the inputs that do have a result, with the Bezout identity checked) and
`::ubig_extended_gcd_with_negative_coefficient` (`#[should_panic]`, pins the message).
Fix: no behaviour change. The panic now says "the Bézout coefficients of these operands are
not representable as UBig, use dashu_base::ExtendedGcd::gcd_ext instead", and the method
has a `# Panics` section.
Notes: the report is right that it panics and that the trait documents no panic; it is
wrong to expect a result with `gcd == 1` and unspecified coefficients, since a caller of
`extended_gcd` uses the coefficients. A maintainer could alternatively decide to return
reduced non-negative residues (`x mod b`, `y mod a`), but that would silently break the
identity the trait documents.

### ntt-capacity-32 — products beyond the largest transform hit `unreachable!` (32-bit words)
Reports: F5
Verdict: real, fixed
Root cause: with 32-bit words the transform primes support at most `2^26` points
(`MAX_LOG_N`, `integer/src/arch/generic_32_bit/ntt.rs:18`), i.e. operands of at most
`2^26 + 1` words in total at one word per coefficient. `select_params`
(`integer/src/mul/ntt/mod.rs:42`) skipped every candidate beyond that and fell into its
`unreachable!`; nothing in the dispatcher bounded the operand size.
Base behaviour: same panic for `sqr` and `mul` of `2^25 + 1` words; `2^25` words works
(run with `--cfg force_bits="32"`, release).
Provenance: pre-existing.
Test: `integer/src/mul/mod.rs::threshold_tests::test_mul_in_pieces` (the splitting and its
scratch budget, with short pieces) and `::test_transform_capacity` (the capacity predicate;
with 32-bit words on a 64-bit host also that the report's shape is budgeted piecewise
instead of panicking). The report's sizes are in
`integer/tests/mul.rs::test_mul_beyond_largest_transform`, which is `#[ignore]`d **for its
cost, not because it fails**: it needs ~4 GiB and only does anything with 32-bit words on a
64-bit host. It passes (see below).
Fix: `ntt::fits(a, b)` tells whether the convolution the transform kernel would run fits
the largest transform; when it does not, the dispatcher (`add_signed_mul`,
`add_signed_mul_same_len`, and `sqr::sqr`) multiplies piecewise with
`add_signed_mul_in_pieces`: the operands are cut into pieces of `ntt::MAX_OPERAND_LEN`
(`2^25` words with 32-bit words) and each piece product goes back through the dispatcher.
`memory_chain_budget` / `sqr::memory_requirement_exact` model it with `pieces_budget`.
Verified with `--cfg force_bits="32"` in release: the ignored test (70 s), and from a
scratch program the report's two cases, a dense square of `2^25 + 2` words (34 s, 3.9 GB
resident), a dense `2^25 + 1002` x `2^25 + 3` word product (39 s, 4.0 GB), and an
unbalanced one (`3.5 * 2^25` x `0.75 * 2^25` words, 158 s, 4.6 GB).
Notes: this is only reachable with 32-bit words on a 64-bit host (`force_bits="32"`, as in
CI): on a real 32-bit target the transform scratch for operands of that size (several GiB)
cannot be allocated in the first place. With 64-bit words the limit is `2^56` words.

### reducer-check-unshifted — `Reducer<UBig>::check` accepts unshifted small values
Reports: F6
Verdict: real, fixed
Root cause: `integer/src/modular/reducer.rs:105`: for a modulus of three or more words and
a value of at most two words, `check` returned `true` unconditionally. Valid values are
multiples of `2^shift` (the normalization shift); the arm for longer values tested that, the
one for short values did not.
Base behaviour: same (`check(1) == true` for `2^150 + 1`), run.
Provenance: pre-existing. The earlier cluster `reducer-check` (B4, D5, E2; commit `7babb63`)
changed the neighbouring arm (`is_le` -> `is_lt`, rejecting the modulus itself); it is a
different invalid class, and the arm at fault here is untouched base code.
Test: `integer/tests/reducer.rs::test_reducer_check_rejects_unshifted_values`
Fix: the short-value arm checks the low `shift` bits too. (`reduce_once` never relied on
that arm.)
Notes: the report says the defect needs a top word without its top bit set; `2^191 + 1`
(no shift, with 64-bit words) indeed has `transform(1) == 1` and `check(1)` rightly `true`.

### ubig-from-float-negative-fraction — `UBig::try_from(-0.5)` is an error
Reports: F8
Verdict: real, fixed
Root cause: `integer/src/convert.rs:848` converted the signed mantissa to `UBig` before
shifting the fraction out, so every negative float was rejected, including those in
`(-1, 0)` that truncate to zero. `num_traits::FromPrimitive::from_f32/from_f64` forward to
it and so disagreed with `u64::from_f64(-0.5) == Some(0)`, and with `UBig` itself for
`0.5` and `-0.0`.
Base behaviour: same (`None` / `Err(OutOfBounds)`), run.
Provenance: pre-existing. The sibling `ibig-from-float-floor` (B7, D10, E6; `f834a33`) made
`IBig` truncate towards zero and documented truncation as the rule; `UBig` was not
re-examined.
Test: `integer/tests/convert.rs::test_ubig_from_float_truncates`,
`integer/tests/num_traits.rs::test_ubig_from_primitive_float_truncates` (compares with
`u64` / `u32`).
Fix: truncate the magnitude first, then reject a negative value only if the result is
nonzero. Values at or below -1, NaN and infinities are still `OutOfBounds`.
Notes: this changes the inherent `TryFrom<f32/f64> for UBig` (not only the num-traits impl),
on purpose: it is the same truncation `IBig` and the positive branch already apply.

### cbrt-rem-doc — `CubicRootRem::cbrt_rem` documents a strict bound
Reports: F93
Verdict: docs
Root cause: `base/src/ring/mod.rs:161` said `0 <= rem < 3*root*root + 3*root`; the remainder
equals that bound right below every cube (`7.cbrt_rem() == (1, 6)`). The code and the unit
test in `base/src/ring/root.rs` are right.
Base behaviour: same text, same (correct) results, run.
Provenance: pre-existing.
Test: `integer/tests/root.rs::test_cbrt_rem_largest_remainder` (pins the largest remainder
for primitives and `UBig`; it does not parse the doc text, unlike the report's suggestion).
Fix: `<=` in the doc comment.

## Per-report table

| Report | Cluster | Verdict | Provenance |
|---|---|---|---|
| F1 | tuning-thresholds | real, fixed | pre-existing |
| F2 | num-integer-multiple-of-zero | real, fixed | pre-existing |
| F3 | num-integer-ubig-extended-gcd | not a bug (message and docs improved) | pre-existing |
| F4 | tuning-thresholds | real, fixed (duplicate root cause of F1) | pre-existing |
| F5 | ntt-capacity-32 | real, fixed | pre-existing |
| F6 | reducer-check-unshifted | real, fixed | pre-existing |
| F8 | ubig-from-float-negative-fraction | real, fixed | pre-existing |
| F93 | cbrt-rem-doc | docs | pre-existing |

Counts: 7 clusters. Verdicts: 5 real, fixed; 1 not a bug; 1 docs. Provenance: 7
pre-existing (0 regression, 0 new-code, 0 incomplete-fix, 0 masked). One unreported
regression (from `68b0e88`) was found and fixed inside tuning-thresholds, see there.

## Changelog entries

`integer/CHANGELOG.md`, under `## Unreleased`:

```markdown
### Fix
- With the `tuning` feature, the `DASHU_THRESHOLD_*` overrides of the multiplication, squaring and short product (`high::mul_high`, `high::sqr_high`) thresholds are clamped to the range their algorithms support, as the division thresholds already were. Values that were too small (or, for `DASHU_THRESHOLD_SIMPLE_MUL`, too large) made products panic.
- Fix a panic ("entered unreachable code") when multiplying or squaring operands beyond the largest transform size, i.e. more than `2^26 + 1` words in total with 32-bit words: such products are now split into pieces.
- `num_integer::Integer::is_multiple_of` and `divides` for `UBig` and `IBig` return `self.is_zero()` for a zero divisor, as the trait defines, instead of panicking.
- `<ConstDivisor as num_modular::Reducer<UBig>>::check` rejects values of one or two words that are not multiples of the normalization shift for moduli of three or more words (it accepted every such value).
- `UBig::try_from(f32/f64)` (and `num_traits::FromPrimitive::from_f32/from_f64` for `UBig`) truncates a negative value in `(-1, 0)` to zero, like `IBig` and the primitive unsigned integers, instead of failing.

### Change
- `num_integer::Integer::extended_gcd` for `UBig` documents that it panics when a Bézout coefficient is negative (the trait's coefficients are unsigned), with a message pointing at `gcd_ext`.
```

`base/CHANGELOG.md`, under `## Unreleased`:

```markdown
### Fix
- Fix the documented bound of the remainder of `CubicRootRem::cbrt_rem`: `rem <= 3*root*root + 3*root` (it was stated as strict).
```

## Cross-group notes

- All source changes are in `integer/` and `base/`; no other crate was touched.
- `UBig::try_from(f32/f64)` now succeeds for negative fractions in `(-1, 0)`. I found no
  caller in the other crates that relies on the old error (grep for `UBig::try_from(` with
  a float argument in `float/`, `rational/`, `complex/`, `python/`), but a group whose
  reports go through float-to-`UBig` conversion should re-test on top of `da36634`.
- `integer/src/mul/mod.rs` (`memory_chain_budget`, the dispatcher) is shared with whichever
  group handles other multiplication or scratch-memory reports; my changes there are the
  threshold clamps, the schoolbook chunk-tail budget and the oversized-product path.
- No report in this group belongs to another group's root cause, and I am not aware of
  duplicates of these in other groups.
