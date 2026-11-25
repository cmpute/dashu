# F-float-exp — dashu-float exp / log / pow / root (run F)

Reports: F10, F14, F20, F22, F43, F56, F86 (7 reports, 7 clusters: no two share a root cause).

Method: every reproducer was run from a scratch crate against this tree (before and after the
fix) and against the read-only checkout of `a0fc535` (debug and release builds for both). "Base
behaviour" below is what was observed there unless it says otherwise.

## Clusters

### nth-root-extreme-exponents — cbrt / nth_root next to the exponent limits

Reports: F10

Verdict: real, fixed

Root cause: `Context::nth_root` aligned the significand with plain `isize` arithmetic —
`x.exponent + digits` (`float/src/root.rs:147`) and `x.exponent - shift` (`:161`) — which leaves
the `isize` range for an exponent within about `n·precision` of `isize::MIN`/`isize::MAX`. Debug
builds panic ("attempt to add/subtract with overflow"); release builds wrap and return a root
with the wrong exponent (`cbrt(8·10^(isize::MIN+2)) = 2·10^3074457345618258603`) or the wrong
significand (`cbrt(27·10^(isize::MAX-1)) = 139247665·10^3074457345618258594`).

Base behaviour: identical — the same two panics in debug (`root.rs:146`, `:160` there) and the
same two wrong values in release.

Provenance: **pre-existing**. The arithmetic is base code, untouched by `a0fc535..8137b6e`. The
earlier triage fixed the same pattern in `sqrt` only (cluster `sqrt-single-step`, commit
`8ec3e01`, test `test_sqrt_extreme_exponents`); no earlier report named `cbrt`/`nth_root`, so
this is the unreported sibling rather than an incomplete fix of a reported case.

Test: `float/tests/root.rs::test_nth_root_extreme_exponents` — the two reported calls, plus an
exponent-invariance sweep (the root of `m·B^e` is the root of `m·B^(e mod n)` with the exponent
moved by `(e − e mod n)/n`) for `n ∈ {3, 4, 5, 7}`, every exponent within `2n+1` of either
limit, bases 2/3/10, exact and inexact roots, negative operands, 20/50/100/500 bits. Passes in
debug and release; fails (panic) on the unfixed source.

Fix: `float/src/root.rs` — the alignment (`r`, `shift`) and the result exponent are computed in
`i128`, as `sqrt` already does. The result exponent (about `exponent / n`) always fits.

### nth-root-negative-zero — an even root of -0 is out of domain

Reports: F20

Verdict: real, fixed

Root cause: `Context::nth_root` ran the domain check (`sign == Negative && n % 2 == 0`,
`float/src/root.rs:117`) before the zero shortcut, and `Repr::sign` of `-0` is negative. So
`nth_root(2k, -0)` was `Err(OutOfDomain)` — a panic through `FBig::nth_root` — while
`sqrt(-0) = -0` and `nth_root(odd, -0) = -0`.

Base behaviour: `Context::nth_root(2, -0)` at a limited precision returns `Err(OutOfDomain)`,
as now. The report's own calls (`FBig::try_from(-0.0).nth_root(2)`, `(-DBig::ZERO).nth_root(4)`,
both precision 0) panic at the base with "precision cannot be 0 (unlimited)" instead — as does
`nth_root(3)` of the same value.

Provenance: **pre-existing** for the defect itself (the check order is base code and the
`Context`-level call fails the same way at the base). For the report's precision-0 inputs it was
**masked** by the `zero-root-precision` defect (E123/E141, fixed in `5d637b0`, which moved the
zero shortcut above the precision assertion but left it below the domain check and documented
"an even root of -0 already errored above").

Test: `float/tests/root.rs::test_nth_root_of_negative_zero` — `nth_root(n, ±0)` for `n = 1..=6`,
bases 2 and 10, precision 0/20/50/100/500, agreement with `sqrt`, the reported calls, and a
negative nonzero operand still being out of domain. Fails on the unfixed source.

Fix: `float/src/root.rs` — the zero shortcut runs first, returning the zero with its sign.

Note: the result is `-0`, matching `sqrt(-0) = -0` in this crate (and the existing
`test_nth_root_equals_sqrt` property that `nth_root(2, x)` equals `sqrt(x)`), which is also what
the report asks for. IEEE 754 `rootn(-0, n)` for even `n > 0` is `+0` (while its `sqrt(-0)` is
`-0`); if the maintainer prefers that convention the one-line change is in the shortcut.

### powi-huge-exponent-negative-base — directed rounding mirrored for an odd exponent above 2^64

Reports: F14

Verdict: real, fixed

Root cause: for an exponent beyond the squaring chain (`MAX_POWI_CHAIN_BITS = 64`),
`powi_via_exp_log` (`float/src/exp.rs:951`) computed `|base|^n` through `pow_exp_log`, let the
Ziv loop round the *positive* magnitude in the caller's mode, and negated the rounded result
(`rounded.map(|v| v * result_sign)`, `:970`). Under `Up`/`Down` a negative result was therefore
rounded in the mirrored direction and kept the magnitude's flag (`Down` returned the value
nearer zero with `NoOp`, `Up` the one further away with `AddOne`).

Base behaviour: identical output for every line of the reproducer (the 64-bit exponent correct,
the 65-bit one swapped).

Provenance: **pre-existing** (`powi_via_exp_log` and its negate-after-rounding are base code;
`fef9130` touched the function's callee but not this logic).

Test: `float/tests/exp.rs::pow_range::test_powi_huge_odd_exponent_negative_base_directed` — the
reported call with its exact expected values and flags under all five relevant modes
(`-e^(1/64)` lies between `-532545·2^-19` and `-8321·2^-13`), plus a sweep checking
`(-a)^n = -(a^n rounded in the mirrored mode)` with the mirrored flag for odd `n`, and `= a^n`
for even `n`, on both sides of the 64-bit limit, positive and negative exponents, six modes,
20/50/100/500 bits. Fails on the unfixed source.

Fix: `float/src/exp.rs` — `pow_exp_log` takes the result sign and negates the working ball
inside the Ziv closure, before it is rounded; the sign of a range error is set in the same
place. `powi_via_exp_log` just passes the sign.

### powf-zero-base-sign — powf(-0, odd integer) drops the sign

Reports: F22

Verdict: real, fixed

Root cause: the zero-base shortcut of `Context::powf` (`float/src/exp.rs:757`) returned `+0`
(or `+inf` for a negative exponent) whatever the exponent, before the integer-exponent
delegation to `powi`. Its comment called this deliberate ("CPython: `(-0.0) ** y == 0.0`"), but
that premise is wrong (`(-0.0) ** 3.0 == -0.0` in CPython, as in IEEE 754 `pow` and MPFR), and
it contradicts `powf`'s own documentation, which says an integer-valued exponent delegates to
`powi` with the sign fixed by the exponent's parity. In the `y < 0` case the sign of an
infinity was wrong, not only that of a zero.

Base behaviour: identical (`powf(-0, 3) = +0`, `powf(-0, -3) = +inf`; `powi` gives `-0`/`-inf`).

Provenance: **pre-existing** (the shortcut and its comment are base code; `7772745` only changed
the precision the result carries).

Test: `float/tests/exp.rs::pow_range::test_powf_zero_base_sign` — `powf(±0, k)` against the
IEEE rule and against `powi` for integer `k` (including multiples of the base, whose `Repr` has
a positive exponent, and an odd base), non-integer exponents (always `+0`/`+inf`), bases
2/3/10, 20/50/100/500 bits, and the `FBig` API. Fails on the unfixed source.

Fix: `float/src/exp.rs` — the shortcut keeps the base's sign when the exponent is an odd
integer. The parity is read off the exponent's representation (new private `is_odd_int`), so no
integer is materialized for a huge exponent. Docs of `Context::powf` state the rule; the comment
of the unit test `exp::tests::test_powf_zero_base` (whose assertions were already consistent
with the rule) was corrected.

Note: this is a behaviour change to a case the source called deliberate. `CachedFBig::powf` and
the Python `powf` go through `Context::powf` and change with it. `CBig::powf` has its own
zero-base rule (complex-trans cluster 14) and is not affected.

### powf-unlimited-precision-shortcuts — 0^0 on precision-0 zeros panics

Reports: F43

Verdict: real, fixed

Root cause: `Context::powf` asserted a limited precision (`float/src/exp.rs:746`) before its
exact shortcuts. `FBig::powf` uses `Context::max` of the two operands, which is precision 0 when
both come from `FBig::try_from(0.0)` (or `DBig::ZERO`, `FBig::ONE`), so `0^0`, `1^1`, `0^1` …
panicked with "precision cannot be 0 (unlimited)", while `FBig::from(0u8)` (precision 1) got 1.

Base behaviour: identical panic for `try_from(0.0).powf(&try_from(0.0))`, `ONE.powf(&ONE)` and
`try_from(0.0).powf(&ONE)`; `from(0u8).powf(&from(0u8))` returns 1.

Provenance: **pre-existing** (the assertion order is base code). The earlier triage moved the
exact shortcuts above the assertion for `cbrt`/`nth_root`/`hypot` (E123/E141) and the base
already did so for `exp`/`exp_m1`/`sqrt`/`ln`; `powf` was not reported then.

Test: `float/tests/exp.rs::test_powf_exact_shortcuts_unlimited_precision` (the report's cases
plus `x^1`, `0^y`, `1^y` at precision 0, binary and decimal). The existing
`test_pow_unlimited_precision` (`#[should_panic]`) still pins that a non-trivial power needs a
precision. Fails on the unfixed source.

Fix: `float/src/exp.rs` — the assertion moved below the exact shortcuts (zero or unit exponent,
zero or unit base); "Panics" doc of `Context::powf` updated.

### log10-exact-past-isize — log10 of 10^m with |m| > isize::MAX does not return

Reports: F56

Verdict: real, fixed

Root cause: `exact_pow10_log` (`float/src/log.rs:863`) discarded an exact logarithm that does not
fit `isize` (`isize::try_from(m).ok()`). In a base that is a higher power of ten the logarithm
is a multiple of the exponent — `log10(100^e) = 2e` — so for `|e| > isize::MAX/2` the shortcut
returned `None` and the Ziv loop was entered on an exactly representable result, which it can
never certify under a directed mode (`HalfEven` returned the right value with `Exact`).

Base behaviour: identical — `Context::<Up>::new(20).log10::<100>(1·100^(isize::MAX/2+1))` does
not return within 20 s (debug and release), `HalfEven` and `e = isize::MAX/2` return at once.
At the base the limit was a `checked_mul`/`checked_add` in `isize`.

Provenance: **pre-existing**. The earlier `log-exact-power` fix (E82, `3ebc1bf`) rewrote the
shortcut on prime valuations (in `i128`) but kept the base's `isize` restriction for `log10`
(`log2` already returned the `i128`).

Test: `float/tests/log.rs::test_log_exact_integer_past_isize` — `log10(100^e)` and
`log10(10·100^e)` for `e` on both sides of `±isize::MAX/2` and next to the limits, six modes,
20/50/100/500 digits, the `log2(16^e)` analogue, and the directed rounding of an integer wider
than the precision. On the unfixed source the test does not return (killed after 60 s).
Unit test `log::tests::test_exact_pow10_log_overflow` extended.

Fix: `float/src/log.rs` — `exact_pow10_log` returns the `i128`; `convert_int` rounds it.

### powi-exponent-ceiling — 2^isize::MAX overflows, the larger (2+2^-61)^isize::MAX does not

Reports: F86

Verdict: real, not fixed

Root cause: the crate's range convention (`Repr::check_finite_exponent`, `Context::scale_rounded`,
`float/src/exp.rs:468`) is that a finite result has its *normalized* exponent strictly between
`isize::MIN` and `isize::MAX`. `2^isize::MAX` normalizes to significand 1 at exponent
`isize::MAX` and is an overflow — `inf` under the nearest modes, and under `Down`/`Zero` the
saturation endpoint `(2^p − 1)·2^isize::MAX` (`overflow_repr_endpoint`), which is *above* the
exact value. `(2 + 2^-61)^isize::MAX ≈ 7.39·2^isize::MAX` has a full 64-bit significand, hence
exponent `isize::MAX − 60`, and is returned. The set of representable values near the top
therefore depends on the significand's trailing zeros, and `powi` is not monotone across it.
Each call follows the convention; the convention is the defect.

Base behaviour: `powi(2, isize::MAX)` is identical (`inf` under `HalfEven`,
`18446744073709551615·2^isize::MAX` under `Down` — pinned at the base by the unit test
`exp::tests::test_powi_exact_ceiling_overflow_directed`). `powi(2 + 2^-61, isize::MAX)` did not
return within 30–60 s (debug and release) under either mode.

Provenance: **pre-existing** design limitation, whose observable symptom here (the comparison of
the two powers) was **masked** at the base by the `powi-range` defects fixed in `fef9130`
(E258/E167/E162/E170): only since then does the larger base return a value to compare against.

Test: `float/tests/exp.rs::pow_range::test_powi_monotone_at_exponent_ceiling`, `#[ignore]`d
(it fails: `inf > 8519001675203524397·2^9223372036854775747`).

Fix: none. This is the top-of-range mirror of E145 (`real, not fixed`: `Up` at exponent
`isize::MIN` saturating below the exact value) and the open question "the top of the exponent
range" in `triage/float-exp.md`: a sound fix needs a crate-wide decision on the exponent-range
model (a magnitude-based range, or endpoints at `isize::MAX − 1`), touching every float kernel
and the saturation endpoints. The reported `(2+2^-61)^isize::MAX` value itself is correct.

## Per-report table

| Report | Cluster | Verdict | Provenance |
|---|---|---|---|
| F10 | nth-root-extreme-exponents | real, fixed | pre-existing |
| F14 | powi-huge-exponent-negative-base | real, fixed | pre-existing |
| F20 | nth-root-negative-zero | real, fixed | pre-existing (the report's precision-0 calls were masked by E123/E141) |
| F22 | powf-zero-base-sign | real, fixed | pre-existing |
| F43 | powf-unlimited-precision-shortcuts | real, fixed | pre-existing |
| F56 | log10-exact-past-isize | real, fixed | pre-existing |
| F86 | powi-exponent-ceiling | real, not fixed | pre-existing (the comparison was masked by E258/E167/E162/E170) |

The pipeline's cross-references ("the fault of bug 10 is in `nth_root` too" on F20, "the fault
of bug 22 is in `powf` too" on F43) are right about the function and right that the defects are
different: each pair is two independent fixes in the same function.

## Changelog entries

`float/CHANGELOG.md`, under `## Unreleased`:

### Fix
- Fix `cbrt`/`nth_root` of a value with an exponent next to `isize::MIN`/`isize::MAX` (debug overflow panic, wrong exponent or significand in release builds).
- Fix an even `nth_root` of `-0` reporting an out-of-domain error (a panic through `FBig::nth_root`); it is `-0`, as for `sqrt`.
- Fix the rounding direction and flag of `powi` under `Up`/`Down` for a negative base with an odd exponent above `2^64`.
- Fix `powf` panicking with "precision cannot be 0" for a trivially exact power (zero or unit exponent, zero or unit base) of unlimited-precision operands such as `FBig::try_from(0.0)`.
- Fix `log10` not terminating under directed rounding for a power of ten whose logarithm exceeds `isize` (`log10(100^e)` with `|e| > isize::MAX / 2`).

### Change
- `powf` on a zero base now keeps the base's sign for an odd integer exponent, as `powi` and IEEE 754 `pow` do: `powf(-0, 3) = -0` and `powf(-0, -3) = -inf` (previously `+0` and `+inf`).

## Cross-group notes

- Files touched: `float/src/root.rs`, `float/src/exp.rs`, `float/src/log.rs` and their test files
  `float/tests/{root,exp,log}.rs` — all this group's own area. Nothing in `repr.rs`, `round.rs`,
  `ziv.rs` or `ball.rs` was changed.
- F86 belongs with float-arith's E145 and the "top of the exponent range" open question; any
  crate-wide change to the range model should un-ignore
  `pow_range::test_powi_monotone_at_exponent_ceiling`.
- F22 changes `Context::powf`, which `CachedFBig::powf` and `dashu-python`'s `powf` call; no test
  or doc in `python/`, `guide/` or `complex/` pinned the old sign (checked by grep), and the
  Python suite was not run.
- Seen while here, not changed: `Context::powf` converts an integer-valued exponent with
  `exp.to_int()` before delegating to `powi`, which materializes the integer even when its
  exponent is huge (`powf(x, 1·2^(10^12))`); `powi`'s own range pre-check would reject it, but only
  after the allocation. This is inferred from reading the code — it was not run (the allocation
  would be tens of gigabytes) — and it is not part of any F report in this group.

## Verification

Run:
- every reproducer against this tree before/after the fixes and against `a0fc535`, debug and
  release (the `a0fc535` hangs were cut off at 20–60 s);
- the six new non-ignored tests against the unfixed `float/src/{root,exp,log}.rs` of `b0705ed`:
  five fail on a panic/assertion, the `log10` one does not return;
- at the head of this branch: `cargo test -p dashu-float`; `cargo test -p dashu-float -p
  dashu-cmplx -p dashu-ratio --all-features`; `cargo test -p dashu-float --release --test root
  --test exp --test log`; `cargo fmt --all -- --check`; `cargo clippy --all-features
  --all-targets --workspace --exclude dashu-python -- -D warnings`; `cargo doc -p dashu-float
  --no-deps --all-features --document-private-items` with `-D warnings`; `cargo check -p
  dashu-float --no-default-features`.

Not run: the MSRV toolchain, a 32-bit target, the `fuzz/` MPFR differentials, the dylint house
lints, and the Python test suite.
