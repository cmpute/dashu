# Hegel bug-report triage: summary

339 reports against `a0fc535` were triaged in 11 areas. Per-area notes (root cause,
test location and fix for every cluster, plus a per-report table) are in the
`<area>.md` files next to this one; `index.tsv` maps every report ID to its area.

## Headline numbers

- **339 reports → 213 clusters** as triaged per area. A handful of clusters in
  different areas share a root cause or are the same defect pattern repeated in
  several crates (see [Cross-area duplicates](#cross-area-duplicates)), so the number
  of independent root causes is a little over 200.
- **330 reports (204 clusters) are real bugs and are fixed**, each with a regression test.
- **1 report (1 cluster) is real and not fixed**: E145.
- **2 reports are not bugs**: E58, E173 (plus one part of E249).
- **6 reports are documentation errors** (the code was right): E191, E213, E215,
  E219, E248, E265.
- No report was unreproducible.

| Area | Reports | Clusters | Real, fixed | Real, not fixed | Not a bug | Docs |
|---|---|---|---|---|---|---|
| `int-kernels` | 28 | 11 | 11 | 0 | 0 | 0 |
| `int-misc` | 40 | 21 | 21 | 0 | 0 | 0 |
| `rational` | 38 | 27 | 25 | 0 | 0 | 2 |
| `float-fmt` | 29 | 17 | 17 | 0 | 0 | 0 |
| `float-convert` | 35 | 23 | 23 | 0 | 0 | 0 |
| `float-arith` | 34 | 26 | 22 | 1 | 2 | 1 |
| `float-exp` | 33 | 15 | 15 | 0 | 0 | 0 |
| `float-trig` | 15 | 11 | 11 | 0 | 0 | 0 |
| `complex-arith` | 27 | 20 | 18 | 0 | 0 | 2 |
| `complex-trans` | 31 | 17 | 17 | 0 | 0 | 0 |
| `python-macros` | 29 | 25 | 24 | 0 | 0 | 1 |
| **Total** | **339** | **213** | **204** | **1** | **2** | **6** |

(Counts are clusters; a cluster's verdict is that of its primary report.)

## Not fixed, not bugs, and partly-wrong reports

- **E145 — real, not fixed.** Under `mode::Up`, a result at exponent `isize::MIN`
  (`3·2^isize::MIN`) rounds to the saturation endpoint `1·2^isize::MIN`, below the
  exact value. A sound fix needs gradual underflow or a different exponent-range
  model across all float kernels. Test: `float/tests/mul.rs`
  `triage::test_directed_rounding_at_minimum_exponent` (`#[ignore]`d).
- **E58 — not a bug.** `rem_euclid` returning `|rhs|` is the exact result rounded to
  the operand precision. Documented and pinned.
- **E173 — not a bug.** The extra digit after a cancelling subtraction is the
  intentional add/sub guard digit (see `AGENTS.md`). Documented and pinned.
- **E249 — partly not a bug.** Its `sinh`/`cosh`/`exp` examples whose result has a
  multi-digit significand at exponent `isize::MAX` correctly report `Overflow` (that
  exponent is reserved); its other `exp` examples are real and fixed.
- **E121 — real, but the report's expected value is wrong.**
  `simplest_from_f64(2^54 + 8)` is `2^54 + 6` (an included midpoint), not `2^54 + 7`.
- **E178 — fixed for zero literals only.** A nonzero literal with an exponent near
  `isize::MIN` still exceeds the integer capacity.
- **E257 — fixed for equal bases only.** Comparing floats of different bases with
  huge, nearly equal magnitudes can still allocate enormously.
- **E159 / E234 / E79** are debug-assertion-only panics, as reported.

## Cross-area duplicates

Clusters that were triaged in different areas but are one root cause, or one defect
pattern copied across crates:

- **`ErrorBounds::error_bounds` next to a power of the base** (`float/src/round.rs`):
  float-exp "ziv-certification" (E45, E229, E174, E69, D29, D40, E225), float-trig
  E239, complex-trans E157/E216, and float-convert E59/E166 (odd bases, unlimited
  precision) all land in this one function. Four areas fixed it independently; the
  merged code is float-convert's version plus the power-of-base tie rule from
  float-exp. It is also the root of rational E14.
- **Wrong `Exact`/`Inexact` flag from the Ziv loop** (flag taken from the
  approximation, not the true value): float-exp D40/E69, float-trig E251, and the
  complex flag clusters (E63, E172, E228; E83/E94/E118 via `combine_parts`).
- **Exact special cases returning precision 0**: float-exp D17/E61/E221,
  float-trig E223, complex-trans E103/E264 (same pattern, separate code in each).
- **Zero built at a nonzero exponent** (`Repr::new(0, e)` gives `-0` for `e = -1`
  and `inf` for `e = isize::MAX`): float-arith B1/D18/E88/E204 and float-fmt
  A3/D14/E10.
- **`num_traits::Signed::abs_sub` returning `|a - b|`**: int-misc A8/E57, rational
  E101, float-convert E72 (one copy per crate; A8 reports all three).
- **`NumOrd`: zero ranked above small positive floats**: int-misc D15/E37, float
  D27/E55, rational D43/E32 (one copy per crate).
- **`FloatEncoding::encode`** (`base/`): int-misc D32/D42/E15/E33/E78/E137; E15's
  `f32::try_from(RBig)` symptom is the same bug.
- **A second sign accepted after the radix point**: float-fmt E153 and rational
  E126 (separate parsers).
- **Human-readable serde loses the precision**: float-fmt A6/D20/E42/E91/E135 and
  complex-arith E130/E190 (separate serializers).
- **`mul_high` sticky bit**: int-kernels B5/C4/D4/E12; D4's `DBig` product one ulp
  off is this bug seen through `dashu-float`.

## Changes that need a maintainer's decision

These fixes change public behaviour beyond "the wrong answer is now right":

1. **(breaking) `IBig % uN`, `IBig::div_rem(uN)` remainder and `uN / IBig` return
   `IBig`** instead of the unsigned primitive (they panicked on a negative result).
2. **Serde, human-readable formats:** values whose precision a literal cannot
   express (unlimited precision, guard-digit values) are written as a struct/object
   for `FBig`/`DBig` and `CBig`; some strings changed (`10` at precision 1 is
   `"1@1"`). Both old and new forms are read.
3. **`CBig` in bases above 18:** the imaginary unit is written and parsed as `*i`
   (a bare `i` is a digit there).
4. **`static_fbig!`/`static_dbig!`** are uniformly unlimited precision, as
   documented; `cbig!` decimal coefficients gain up to one bit of precision.
5. **`RBig::in_expanded` with `{:#.N}`** prints a terminating expansion in full,
   ignoring the precision (as it already did for repeating ones).
6. **Rounding flags:** 11 expectations in `float/tests/add.rs` changed (E136), the
   saturation endpoints of `tanh`/`exp_m1` report the truncation-based flag (E224),
   and two `float/tests/high.rs` tests that asserted over/underflow for
   representable results were rewritten.
7. **New panics / errors in place of nonsense:** `rem_euclid`/`div_euclid` on
   infinities, `UniformFBig::new(x, x, p)`, `Uniform01::new_open(0)`, `RBig::inv`
   of zero, `ilog` of zero for every base; `RBig::from_str("1/0")` and
   out-of-range float exponents are parse errors (reusing existing `ParseError`
   variants).
8. **Float `+`, `-`, `<<`, `>>` saturate** to `±inf` on exponent overflow.
9. **Python:** an underflowing `UBig - UBig` returns an `IBig`; `n -= m` rebinds;
   `format()`/`round()` break ties away from zero (Python uses half-even).
10. **Scratch-memory model** (`integer/`): the multiplication budget envelope was
    rewritten as a monotone upper bound; division and GCD reserve somewhat more at
    mid sizes. Correctness was tested; performance was not benchmarked.
11. **Ziv flag certification is bounded:** the float loop pursues the side of the
    true value up to about twice the target precision, the complex driver up to
    `8·(precision + guard) + 2048` digits, then returns the (correctly rounded)
    value with the approximation's flag. Flags for extremely tiny arguments in
    complex functions can therefore still be wrong (e.g. `exp(2^-10000·(1+i))`
    tagged `Exact`).

## Known leftovers found during the work (not in any report)

- `atanh(±1)` at precision 0 still panics.
- `exp(2^(2^40))` under some paths and `ln_1p` of a huge argument can still be slow
  or memory-hungry.
- A possible under-budget for chunked NTT products `(a, b)` with `a ≥ 3b` was probed
  (50 shapes, no panic) and left alone.
- `complex` `powf(0, w)` with `Re w < 0` returns `Err(Indeterminate)` where the real
  `powf(0, -1)` returns `+inf`.
- `python/src/cache.rs` has two pre-existing unreachable-pattern clippy errors.

## Clusters by area

### int-kernels (28 reports, 11 clusters)

Details: [`int-kernels.md`](int-kernels.md)

| Cluster | Reports | Verdict |
|---|---|---|
| modular-mul-memory | A4, A7, B8, D9, E23, E107, E250 | real, fixed |
| gcd-ext-memory | A9, B11, E7 | real, fixed |
| reducer-check | B4, D5, E2 | real, fixed |
| mul-high-sticky | B5, C4, D4, E12 | real, fixed |
| ntt-carry | D3, D19, E18 | real, fixed |
| gcd-ext-cofactor (wrong cofactors) + gcd-ext-memory (panic) | D12 | real, fixed |
| modular-self-mul-memory | D13, E50 | real, fixed |
| nth-root-slow | D28, E21 | real, fixed |
| pow-modulus-one | E3 | real, fixed |
| const-divisor-dword | E104 | real, fixed |
| div-high-overflow | E218 | real, fixed |

### int-misc (40 reports, 21 clusters)

Details: [`int-misc.md`](int-misc.md)

| Cluster | Reports | Verdict |
|---|---|---|
| num-integer-div-floor | A1, B3, D7, E39 | real, fixed |
| abs-sub | A8, E57 | real, fixed — real, fixed (IBig part; RBig/FBig parts belong to E101/E72) |
| trailing-ones-large | B6, C5, D6, E4 | real, fixed |
| ibig-from-float-floor | B7, D10, E6 | real, fixed |
| ibig-rem-unsigned | C2, D2, E1 | real, fixed |
| cbrt-negative | D8, E5 | real, fixed |
| num-order-float | D15, E34, E37 | real, fixed |
| encode-rounding | D32, D42, E15, E33, E78, E137 | real, fixed |
| from-chunks-alloc | E8 | real, fixed |
| to-f32-exact-32bit | E9 | real, fixed |
| ilog-zero | E11 | real, fixed |
| num-integer-lcm-sign | E65 | real, fixed |
| log2-est-u128 | E139 | real, fixed |
| num-integer-gcd-zero | E151 | real, fixed |
| abs-cmp-min | E160 | real, fixed |
| num-integer-feature | E184 | real, fixed |
| parse-underscore-only | E189 | real, fixed |
| parse-default-radix | E198 | real, fixed |
| abs-neg-zero | E209 | real, fixed |
| from-chunks-zero | E214 | real, fixed |
| log2-bounds-nostd | E243 | real, fixed |

### rational (38 reports, 27 clusters)

Details: [`rational.md`](rational.md)

| Cluster | Reports | Verdict |
|---|---|---|
| expanded-alternate-long-terminating | A2, D11, E24 | real, fixed |
| expanded-odd-radix-rounding | A10, B9, E138 | real, fixed |
| parse-zero-denominator | A11, D41, E76 | real, fixed |
| to-float-double-rounding | A12, D1, E35 | real, fixed |
| float-try-from-large-integer | D22, E16 | real, fixed |
| numord-zero-vs-float | D43, E32 | real, fixed |
| simplest-from-float-power-of-base | E14 | real, fixed |
| expanded-zero-exponent-marker | E17 | real, fixed |
| inv-zero | E28 | real, fixed |
| simplicity-order | E47 | real, fixed |
| farey-linear-time | E64 | real, fixed |
| relaxed-to-integer | E68 | real, fixed |
| simplest-in-zero-endpoint | E74 | real, fixed |
| farey-limit-one-assertion | E79 | real, fixed — real, fixed (debug builds only) |
| num-traits-abs-sub | E101 | real, fixed |
| display-flags | E111 | real, fixed |
| zeroize | E116 | real, fixed |
| simplest-from-f64-interval | E121 | real, fixed — real, fixed (expected value in the report is off by one) |
| expanded-parse-inner-sign | E126 | real, fixed |
| to-float-fast-accuracy | E143, E233 | real, fixed |
| expanded-parse-zero-huge-exponent | E178 | real, fixed — real, fixed (zero literals; huge nonzero values still exceed capacity) |
| num-traits-pow-usize | E208 | real, fixed |
| nearest-docs | E213 | docs |
| next-up-down-docs | E219 | docs |
| expanded-repetend-budget-edge | E240 | real, fixed |
| log2-bounds | E260 | real, fixed |
| expanded-parse-empty-repetend | E262 | real, fixed |

### float-fmt (29 reports, 17 clusters)

Details: [`float-fmt.md`](float-fmt.md)

| Cluster | Reports | Verdict |
|---|---|---|
| parse: zero with a scale | A3, D14, E10 | real, fixed |
| postgres: negative zero panics | A5, D35, E25 | real, fixed |
| serde: human-readable precision does not round trip | A6, D20, E91, E135 | real, fixed |
| fmt: width one short for integers | B10, E109 | real, fixed |
| fmt: extra digit when rounding carries | B12, C1, E13 | real, fixed |
| postgres: dscale drops the fraction | D21, E44 | real, fixed |
| rkyv: infinities and large exponents truncated | E20 | real, fixed |
| serde: unlimited precision becomes precision 1 | E42 | real, fixed |
| serde: serialized infinity cannot be read back | E73 | real, fixed |
| serde: struct form unreadable with owned keys | E102 | real, fixed |
| parse: second sign accepted | E153 | real, fixed |
| postgres: malformed input panics in from_sql | E186 | real, fixed |
| postgres: 1e32768 refused, precision above 32767 overflows | E192 | real, fixed |
| fmt: zero flag ignored with an alignment | E194 | real, fixed |
| parse: "0x." accepted | E210 | real, fixed |
| parse: exponent at or beyond the isize limits | E211, E261 | real, fixed |
| fmt: Display with a precision runs out of memory | E263 | real, fixed |

### float-convert (35 reports, 23 clusters)

Details: [`float-convert.md`](float-convert.md)

| Cluster | Reports | Verdict |
|---|---|---|
| to_f32/to_f64 just below the smallest subnormal | A13, D16, E19 | real, fixed |
| to_f32/to_f64 between MAX and MAX + ½ulp | E77 | real, fixed |
| Negative zero in integer conversions and is_int | B2, E87, E134 | real, fixed |
| with_base precision | E26, E93, D36 | real, fixed |
| with_base precision (subnormals) + Zeros in base conversion (-0.0) | C3 | real, fixed |
| Zeros in base conversion | E38, E122 | real, fixed |
| with_base_and_precision to the same base does not round | E30 | real, fixed |
| Base conversion wrong for \|exponent\| > 38 | D39, E22 | real, fixed |
| abs_cmp against UBig/IBig uses the signed order | D31, E31 | real, fixed |
| NumOrd: zero ranks above small positive floats | D27, E55 | real, fixed |
| NumOrd against floats overflows for extreme exponents | E175 | real, fixed |
| Same-base NumOrd allocates the full power of the base | E257 | real, fixed |
| log2_bounds invalid for exponents above 2^24 | E255 | real, fixed |
| num-traits Signed::abs_sub | E72 | real, fixed |
| num-traits integer ToPrimitive panics on infinity | E242 | real, fixed |
| Uniform samples outside the range | D25, E85 | real, fixed |
| Uniform over unlimited-precision bounds collapses | E80 | real, fixed |
| rand 0.9/0.10 Uniform::new on empty or reversed ranges | E176 | real, fixed |
| Uniform01::new_open(0) | E200 | real, fixed |
| from_parts_const precision | E128, E222 | real, fixed |
| Repr::new_const exponent overflow | E235 | real, fixed |
| error_bounds too wide in odd bases | E59 | real, fixed |
| error_bounds panics on unlimited precision under Down/Up | E166 | real, fixed |

### float-arith (34 reports, 26 clusters)

Details: [`float-arith.md`](float-arith.md)

| Cluster | Reports | Verdict |
|---|---|---|
| Exact cancellation builds its zero at the operands' exponent | B1, D18, E88, E204 | real, fixed |
| `signum` precision | D30 | real, fixed |
| `/` with a dividend wider than the result precision | E40 | real, fixed |
| Empty `Product` of `CachedFBig` | E54 | real, fixed |
| `rem_euclid` can return `\|rhs\|` | E58 | not a bug |
| `quantize`: sign of a zero result, extreme quantum exponents | E60, E193 | real, fixed |
| Zero remainder sign | E90 | real, fixed |
| `fma` with a zero product | E98 | real, fixed |
| Exact product tagged `Inexact(NoOp)` | E115 | real, fixed |
| Rounding flag after a partial cancellation | E136 | real, fixed |
| Directed underflow endpoint below the exact value | E145 | real, not fixed |
| `+` / `-` leave an overflowed exponent | E146 | real, fixed |
| Remainder cost grows with the exponent gap | E149, E150 | real, fixed |
| Exponent range judged before rounding | E158, E237 | real, fixed |
| Round-to-integer operations drop the sign of a zero result | E168, E197, E206 | real, fixed |
| `Sum` cost grows with the exponent gap | E169 | real, fixed |
| Guard digit after a subtraction | E173 | not a bug |
| `CachedFBig::ulp` reimplements `FBig::ulp` | E179 | real, fixed |
| `Sum`: sign of an all-`+0` sum under `Down` | E182 | real, fixed |
| `fract` changes the precision of values below one | E187 | real, fixed |
| `Repr` `+` / `-` zero signs | E196 | real, fixed |
| Exponent gap beyond `isize::MAX` in add/sub | E205 | real, fixed |
| Shifts to a sentinel exponent | E207 | real, fixed |
| `split_at_point` on `+inf` | E212 | real, fixed |
| `ulp` documentation | E248 | docs |
| `ulp` of `-0` | E253 | real, fixed |

### float-exp (33 reports, 15 clusters)

Details: [`float-exp.md`](float-exp.md)

| Cluster | Reports | Verdict |
|---|---|---|
| exact-shortcut-precision | D17, E61, E221 | real, fixed |
| powf-exact | D23, E48 | real, fixed |
| ziv-certification | D29, E45, E69, E174, E229 | real, fixed |
| ziv-certification (sqrt half: sqrt-single-step) | D40 | real, fixed |
| sqrt-single-step | E29, E75, E203 | real, fixed |
| log-exact-power | E82 | real, fixed |
| zero-root-precision | E123, E141 | real, fixed |
| hypot-range | E131, E185, E195 | real, fixed |
| exp-range-edges | E144, E230, E249, E252 | real, fixed — real, fixed for `exp`; the `sinh`/`cosh`/`exp` examples at exponent `isize::MAX` are not a bug (sentinel exponent) |
| powi-range | E162, E167, E170, E258 | real, fixed |
| cache-ln-base | E188 | real, fixed |
| ziv-certification (via sqrt-single-step) | E225 | real, fixed |
| exp-input-rounding | E227 | real, fixed |
| ln-min-exponent | E241 | real, fixed |
| exp-huge-argument | E266 | real, fixed |

### float-trig (15 reports, 11 clusters)

Details: [`float-trig.md`](float-trig.md)

| Cluster | Reports | Verdict |
|---|---|---|
| atan / atan2 special values rounded twice | E27 | real, fixed |
| Results that hug an exact value | E86, E165, E251 | real, fixed |
| atan2_unit diagonals swapped for negative y | E99 | real, fixed |
| sin returns 0 next to a multiple of π for an over-precise input | E108 | real, fixed |
| asin_unit / acos_unit mistake other arguments for 1/2 in odd bases | E155 | real, fixed |
| Ball::sqrt does not cover its input ball, and asserts rad <= \|mid\| | E159, E181, E234 | real, fixed — real, fixed (debug-only panic) |
| sinh_pi / cosh_pi out of memory at a large argument and low precision | E163 | real, fixed |
| Trig/hyperbolic functions panic on a precision-0 zero | E223 | real, fixed |
| tanh flag at the saturation shortcut | E224 | real, fixed |
| error_bounds is symmetric at a power of the base (value); Results that hug an exact value (flag) | E239 | real, fixed |
| asinh / acosh out of memory at a huge argument | E259 | real, fixed |

### complex-arith (27 reports, 20 clusters)

Details: [`complex-arith.md`](complex-arith.md)

| Cluster | Reports | Verdict |
|---|---|---|
| mul/sqr real part wrong under cancellation | D24, E92 | real, fixed |
| z / z is not exactly 1 | D33, E52 | real, fixed |
| FromStr rejects a signed exponent in the imaginary coefficient | D34, E110 | real, fixed |
| Signed zero of a product depends on the operand order | E46 | real, fixed |
| Rounded results tagged Exact | E83, E94, E118 | real, fixed |
| FromStr rejects infinite parts | E97 | real, fixed |
| 0 / w has unlimited precision | E105 | real, fixed |
| fma loses a fully cancelled result | E106 | real, fixed |
| from_parts with a limited and an unlimited part | E112 | real, fixed |
| Bases where `i` is a digit | E125 | real, fixed |
| Human-readable serde loses the precision | E130, E190 | real, fixed |
| Context::neg/conj/proj/mul_i ignore a narrower context | E142 | real, fixed |
| `{:+}` signs every part | E154 | real, fixed |
| Display precision rounds a negative imaginary part the wrong way | E180 | real, fixed |
| From<IBig>/From<UBig> precision | E191 | docs |
| One out-of-range part discards the other | E199 | real, fixed |
| Directed rounding goes the wrong way (sqr, div) | E217, E246 | real, fixed |
| CachedCBig empty product | E244 | real, fixed |
| inf / 0 reports Indeterminate | E256 | real, fixed |
| Context::max docs | E265 | docs |

### complex-trans (31 reports, 17 clusters)

Details: [`complex-trans.md`](complex-trans.md)

| Cluster | Reports | Verdict |
|---|---|---|
| 4. asin/acos on the axes and branch cuts | D38, E53, E133, E152, E156, E201, E245 | real, fixed |
| 6. hyperbolic functions under Down/Up | E41 | real, fixed |
| 14. powf: zero base, exact and tie results | E43, E62, E231 | real, fixed |
| 12. powi of an anti-diagonal base | E56 | real, fixed |
| 1. Exact/Inexact flags | E63, E172, E228 | real, fixed |
| 15. tan_pi slow for a large imaginary part | E89 | real, fixed |
| 9. wide arguments rounded first | E96 | real, fixed |
| 10. precision-0 results / exp zero sign | E103, E236, E264 | real, fixed |
| 11. powi at unlimited precision | E113 | real, fixed |
| 7. log(−0 − 0i) | E114 | real, fixed |
| 3. wrongly-Exact float kernels | E117, E232, E238 | real, fixed — real, fixed (float side: D40 family) |
| 8. saturated tan/tanh side | E127 | real, fixed |
| 16. sqrt(−x − 0i) real part −0 | E147 | real, fixed |
| 13. powi range-error sign | E148 | real, fixed |
| 2. ErrorBounds at a power of the base (value); 1. flags | E157 | real, fixed |
| 5. atan on the imaginary-axis cuts | E161 | real, fixed |
| 2. ErrorBounds at a power of the base | E216 | real, fixed |

### python-macros (29 reports, 25 clusters)

Details: [`python-macros.md`](python-macros.md)

| Cluster | Reports | Verdict |
|---|---|---|
| cbig-decimal-integer-truncated | D26, E36 | real, fixed |
| py-ubig-del-bits | D37, E49 | real, fixed |
| py-fbig-decimal-format-digits | E51 | real, fixed |
| py-int-dbig-rounds | E66 | real, fixed |
| py-auto-float-precision | E67 | real, fixed |
| py-gcd-lcm-of-zeros | E70 | real, fixed |
| py-hash-disagrees-with-equality | E71 | real, fixed |
| py-zero-division-panics | E81, E124 | real, fixed |
| py-rbig-math-rounding-protocol | E84 | real, fixed |
| float-literal-negative-zero | E95 | real, fixed |
| py-rbig-format-digits | E100 | real, fixed |
| py-ubig-subtraction-underflow | E119 | real, fixed |
| py-general-format-never-scientific | E120 | real, fixed |
| rbig-denominator-without-slash | E129, E254 | real, fixed |
| py-inplace-operators-reject-operands | E132 | real, fixed |
| py-ubig-empty-slice-panics | E140 | real, fixed |
| py-missing-reflected-operators | E164 | real, fixed |
| py-cbig-two-argument-constructor | E171 | real, fixed |
| py-cbig-equality-raises | E177 | real, fixed |
| py-hex-format-sign-and-padding | E183 | real, fixed |
| py-decimal-format-layout | E202 | real, fixed |
| guide-unlimited-precision-example | E215 | docs |
| py-index-beyond-isize | E220 | real, fixed |
| ibig-repeated-signs | E226 | real, fixed |
| static-float-precision-depends-on-size | E247 | real, fixed |
