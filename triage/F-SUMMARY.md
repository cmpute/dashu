# Run F: summary and provenance

Run F (`20261125-083220-dashu-ty8dga`) is the Hegel re-run against `8137b6e`, the tree
with the fixes for runs A–E applied. It produced 96 reports. Per-area notes (root cause,
base behaviour, provenance, test and fix for every cluster) are in the `F-<area>.md`
files next to this one.

"Base" below means `a0fc535`, the commit runs A–E were made against. Every reproducer
was run against both the base and `8137b6e`.

## Verdicts

| Verdict | Reports |
|---|---|
| real, fixed | 91 |
| real, not fixed | 1 (F86) |
| not a bug | 2 (F3, F42) |
| docs | 2 (F93; F83, whose related `pow` overflow is fixed) |

- **F86 — not fixed.** `powi(2, isize::MAX)` overflows while the larger base
  `2 + 2^-61` returns a finite value. It is the top-of-range mirror of E145 and needs
  the same exponent-range rework. Test: `float/tests/exp.rs`
  `pow_range::test_powi_monotone_at_exponent_ceiling` (`#[ignore]`d).
- **F3 — not a bug.** `num_integer::Integer::extended_gcd` on `UBig` panics when a
  Bézout coefficient is negative; the trait's coefficients are unsigned and
  num-integer's own `u32` impl panics the same way. The panic now names `gcd_ext`.
- **F42 — not a bug.** The guard digit of a subtraction is rounded away by the next
  operation, including `x + 0`. Documented and pinned.
- **F25 — half not a bug.** A nonzero significand at exponent `isize::MAX` is a value
  the library itself produces (the largest finite number), so the struct form accepts
  it; the missing digit-count check was real and is fixed.

## Provenance

| Provenance | Reports | Count |
|---|---|---|
| Already at the base, same failure (missed by A–E) | all not listed below | 71 |
| Regression: the base was right, an A–E fix broke it | F15, F38, F39, F51, F54, F66, F72, F81, F88 | 9 |
| Fault in code the A–E fixes added | F25, F27 | 2 |
| Incomplete A–E fix (same defect, case not covered) | F9, F12, F19, F21, F28, F30, F34, F37, F75, F76, F94 | 11 |
| Mixture | F7, F13, F71 | 3 |

So 22 reports (plus parts of 2 of the mixtures) trace to the A–E work, and 71 (plus
the rest of the mixtures) were already in the base.

Masking: four of the base-era bugs could not be observed at the base because another
bug failed first on the same input — F7 for base 10 (out of memory, D39/E22), F13 in
debug builds (overflow panic, E193), F20's precision-0 inputs (E123/E141), and F86's
comparison (the `powi` range defects E258/E167/E162/E170).

### Regressions (base correct, broken by an A–E fix)

| Report | What broke | Introduced by |
|---|---|---|
| F15 | `{:.0}` of `-0` prints `-1` under `Up`/`Away` | `c90d537` (E263, tiny-value formatting) |
| F38 | `UniformCBig::new` rejects a box with a zero-width side | `1812267` (uniform sampling fixes) |
| F39 | Human-readable serde of a `CBig` with a large exponent is slow (34 ms → 2.7 s) | `dcae487` (serde precision) |
| F51, F72 | Odd bases: results next to a midpoint never certify, so the call does not return | `f2b0479` (E59, odd-base half-ulp bound) |
| F54 | Python `format(RBig, '.2f')` loses the sign of a negative value that rounds to zero | `740f086` (E100) |
| F66 | `{:#012x}` of a rational puts the zero padding before the `0x` prefix | `da6a507` (E111) |
| F81 | `UniformFBig` cost grows with the exponent gap of the bounds | `1812267` |
| F88 | `<< 0` / `>> 0` changes a value at exponent `isize::MAX` / `isize::MIN` | `3bfafe1` (E207) |
| F71 (part) | `UniformFBig::new_inclusive(0, 0)` returns `-0` | `1812267` |

F72 is itself mixed: the base returned `asinh(3^-100)` at 255 digits one ulp low
(its over-wide bound certified the wrong neighbour); `f2b0479` turned that wrong
value into a hang.

Two further regressions were found while fixing, not reported by Hegel, and are fixed:

- With the `tuning` feature and `DASHU_THRESHOLD_NTT_MUL` below
  `DASHU_THRESHOLD_TOOM4_MUL`, multiplication panicked with "not enough memory
  allocated" (`68b0e88`, the scratch-memory rewrite).
- In bases above 23, the finite float whose digits spell `inf` serialized as `"inf"`
  and read back as infinity (`fbc65cf`).

### Faults in code added by the A–E fixes

| Report | Fault | Added by |
|---|---|---|
| F25 | serde struct form accepts more digits than the precision allows | `622dbbf`, `fbc65cf` |
| F27 | serde writes a value at exponent `isize::MAX` as a literal it cannot read | `fbc65cf`, `59a79d4` |

### Incomplete A–E fixes

| Report | What was left | Earlier report |
|---|---|---|
| F9 | `Repr::new` itself still built `-0` / `inf` from a zero significand; only callers were fixed | B1 |
| F12 | `sinh_pi` at the bottom of the exponent range returns `±inf` | E165 |
| F19 | Results next to a representable value still flagged `Exact` (flag pursuit was bounded) | E251 |
| F21 | `atanh(±1)` panics at precision 0 (listed as a known leftover) | E223 |
| F30 | Exact rows of the `_unit` trig functions panic at precision 0 | E223 |
| F28 | serde: a `CBig` with an infinite part reads back finite in bases 24–36 | E97 |
| F34 | Complex division at large exponent spreads: wrong `Exact`, wrong direction | E83/E94/E118, E217/E246 |
| F37 | Parsing literals whose trailing fractional zeros push the exponent below `isize::MIN` | E211/E261 |
| F75 | `Sum` does not saturate on overflow the way `+` now does | E146 |
| F76 | Complex add/sub: one overflowing part discards the other | E199 |
| F94 | Remainder of a zero dividend with a huge divisor exponent runs out of memory | E149/E150 |

### Mixtures

- **F7** — `to_f32`/`to_f64` panic when the base-2 exponent overflows `isize`.
  Pre-existing for base 16 (same panic at the base); masked for base 10, where the
  base ran out of memory first.
- **F13** — `quantize` carries onto the reserved exponent. Pre-existing in release;
  masked in debug by an overflow panic that the E193 fix removed.
- **F71** — `UniformFBig` returns `-0`. Pre-existing for a range straddling zero;
  a regression for `[0, 0]`.

### Already at the base (71)

- **int** (8): F1, F2, F3, F4, F5, F6, F8, F93.
- **rational, macros** (9): F29, F31, F32, F35, F91, F83, F40, F59, F87.
- **float I/O** (10): F11, F17, F18, F23, F24, F49, F50, F77, F90, F92.
- **float arithmetic** (5): F16, F36, F41, F42, F47.
- **float exp/pow/root** (7): F10, F14, F20, F22, F43, F56, F86.
- **float trig** (4): F26, F48, F82, F89.
- **complex** (10): F33, F44, F45, F46, F57, F58, F68, F69, F78, F79.
- **python** (18): F52, F53, F55, F60, F61, F62, F63, F64, F65, F67, F70, F73, F74,
  F80, F84, F85, F95, F96.

Some of these were listed in the A–E notes as noticed and out of scope (F21's sibling
cases, F52, F53, F59, F80), or sit next to cases an A–E fix closed on the side (F29,
F62, F64); they fail the same way at the base.

## Duplicates within the run

96 reports in 85 clusters. Reports that share a root cause: F1/F4, F35/F91,
F55/F60/F73, F85/F96, F51/F72, F82/F89 (with F12 in the same cluster), F34/F68 and
F44/F79. The pipeline's own cross-references ("the fault of bug N is here too") were
right about the shared function in every case checked; F9 was correctly identified as
a re-report of B1.

## Changes that need a maintainer's decision

1. **`powf(-0, odd)`** is `-0` / `-inf` (IEEE 754, and what `powi` gives). The old
   `+0` / `+inf` was marked deliberate in a source comment that cited CPython, which
   in fact returns `-0.0`.
2. **Even `nth_root` of `-0`** is `+0` (IEEE 754 rootn); `sqrt(-0)` stays `-0`
   (IEEE 754 squareRoot).
3. **`Repr::new(0, e)`** is `+0` for every exponent; it no longer rebuilds `-0` or an
   infinity from `into_parts()`.
4. **Unlimited-precision float ÷ integer** panics like any unlimited-precision
   division (after A–E it rounded to one digit; at the base it panicked).
5. **`Context::rem(x, 0)`** returns `Err(Indeterminate)`; `%` still panics.
6. **`FBig::atan2` / `atan2_unit`** and **`CBig::fma`** use the highest operand
   precision instead of the receiver's.
7. **Flag pursuit is unbounded for the trig and hyperbolic functions** (item 11 of
   the A–E summary no longer applies to them; it still applies to `exp`, `ln`, powers,
   roots and the complex functions).
8. **Complex**: division no longer falls back to Smith's method; `log` is
   restructured; `arg` of a zero follows the C Annex G table instead of
   `OutOfDomain`; `cos(+0 + 0i) = 1 - 0i`.
9. **`fbig!(0)` / `dbig!(0.000)`** have the precision of their digits (1 and 4)
   instead of unlimited; negative zero literals still have unlimited precision.
10. **`UBig::try_from(-0.5)`** is `Ok(0)`.
11. **`Uniform01`** (rational) panics at construction for a limit that leaves nothing
    to sample.
12. **serde / postgres**: `Repr` with a large exponent is written as `"1@1000"`;
    values with an infinite complex part may use the struct form; postgres rejects
    more than 16383 fractional digits (was 65535).
13. **Python**: integer `/` returns an exact `RBig`; `UBig`/`IBig` have no in-place
    operators; `%` on `RBig`/`FBig`/`DBig` is floored; arithmetic and rounding on an
    infinity raise `ValueError` (the Rust operators panic on infinities, so there is
    no IEEE-style result to forward); rounding methods keep the precision.

## Known leftovers

- F86 (above).
- The complex transcendentals other than `exp` and `log` are still slow for tiny
  arguments under directed modes (0.3–1.4 s); pinned by the `#[ignore]`d
  `complex/tests/trig.rs::tiny_arguments_under_directed_modes`.
- `(x >> 1) << 1 != x` for `x = 1·10^isize::MAX`: a nonzero shift onto a reserved
  exponent is an overflow by design.
- `from_str_expanded("1e999999999999")` still tries to build a 10^12-digit integer;
  documented, no cap added.
- `Context::powf` converts an integer-valued float exponent with `to_int()` before
  `powi` can reject it on range, so an exponent like `2^(10^12)` would materialise a
  huge integer (read from the code, not reproduced).
- Negative zero float literals (`dbig!(-0.000)`) still have unlimited precision.
- The derived rkyv impls for `RBig` / `Relaxed` were not checked for zero
  denominators.

## Verification on the merged tree

Run and passing: workspace tests with all features (debug and release); `dashu-base`
and `dashu-int` with `force_bits="32"`; `cargo fmt --check`; clippy with
`-D warnings`; no_std checks; rustdoc with `-D warnings`; the Python bindings
(140 tests). Not run: MSRV 1.68, the Dylint lints, `mdbook build`, tests needing a
live PostgreSQL server.
